#!/usr/bin/env python3

"""
VFIO Setup Script for PCI Devices

This script sets up VFIO for safe userspace PCI device access for any specified device.
Generated by AI as part of the pci_rust_example project.
"""

import os
import sys
import subprocess
import time
import pathlib
import logging
import argparse
import re
from typing import Optional, List, Tuple


class VFIOSetupError(Exception):
    """Custom exception for VFIO setup errors"""

    pass


# Configure logging
def setup_logging():
    """Setup logging"""
    logger = logging.getLogger()
    logger.setLevel(logging.INFO)

    # Clear existing handlers
    logger.handlers.clear()

    # Create console handler
    console_handler = logging.StreamHandler()
    console_handler.setLevel(logging.INFO)

    # Create formatter
    formatter = logging.Formatter("[%(levelname)s] %(message)s")
    console_handler.setFormatter(formatter)

    # Add handler to logger
    logger.addHandler(console_handler)

    return logger


class VFIOSetup:
    """Main class for VFIO setup operations"""

    def __init__(self, pci_device: str):
        self.pci_device = pci_device
        # Handle both short format (BB:DD.F) and full format (SSSS:BB:DD.F)
        if ":" in pci_device and pci_device.count(":") == 1:
            # Short format, add segment
            self.full_pci_path = f"0000:{pci_device}"
        else:
            # Assume full format or already has segment
            self.full_pci_path = pci_device

        self.vfio_pci_driver = pathlib.Path("/sys/bus/pci/drivers/vfio-pci")
        self.device_path = pathlib.Path(f"/sys/bus/pci/devices/{self.full_pci_path}")
        self.vendor_device_id: Optional[str] = None

    def run_command(
        self, command: List[str], check: bool = True
    ) -> subprocess.CompletedProcess:
        """Run a system command with error handling"""
        try:
            result = subprocess.run(
                command, capture_output=True, text=True, check=check
            )
            return result
        except subprocess.CalledProcessError as e:
            logging.error(f"Command failed: {' '.join(command)}")
            logging.error(f"Error: {e.stderr}")
            raise VFIOSetupError(f"Command execution failed: {e}")

    def check_root(self):
        """Check if running as root"""
        if os.geteuid() != 0:
            logging.error("This script must be run as root (use sudo)")
            raise VFIOSetupError("Root privileges required")

    def check_iommu(self):
        """Check if IOMMU is enabled"""
        logging.info("Checking IOMMU support...")

        iommu_groups_path = pathlib.Path("/sys/kernel/iommu_groups")
        if not iommu_groups_path.exists():
            logging.error("IOMMU is not enabled!")
            logging.error("Please enable IOMMU in BIOS/UEFI and add kernel parameters:")
            logging.error("  Intel: intel_iommu=on iommu=pt")
            logging.error("  AMD: amd_iommu=on iommu=pt")
            raise VFIOSetupError("IOMMU not enabled")

        # Count IOMMU groups
        try:
            groups = list(iommu_groups_path.iterdir())
            group_count = len([g for g in groups if g.is_dir()])

            if group_count > 0:
                logging.info(f"IOMMU is enabled with {group_count} groups")
            else:
                logging.warning("IOMMU directory exists but no groups found")
        except Exception as e:
            logging.error(f"Failed to check IOMMU groups: {e}")
            raise VFIOSetupError("IOMMU check failed")

    def check_device_exists(self):
        """Check if the PCI device exists"""
        logging.info(f"Checking if PCI device {self.pci_device} exists...")

        if not self.device_path.exists():
            logging.error(f"PCI device {self.pci_device} not found!")
            logging.error("Available devices:")

            # Show available devices
            try:
                result = self.run_command(["lspci"], check=False)
                lines = result.stdout.strip().split("\n")[:10]
                for line in lines:
                    print(f"  {line}")
            except Exception:
                pass

            raise VFIOSetupError(f"Device {self.pci_device} not found")

        # Show device info
        try:
            result = self.run_command(["lspci", "-v", "-s", self.pci_device])
            device_info = "\n".join(result.stdout.strip().split("\n")[:3])
            logging.info("Device found:")
            print(device_info)
        except Exception as e:
            logging.warning(f"Could not get device info: {e}")

    def get_vendor_device_id(self) -> str:
        """Get vendor:device ID for the PCI device"""
        logging.info("Getting vendor:device ID...")

        try:
            vendor_path = self.device_path / "vendor"
            device_path = self.device_path / "device"

            if not vendor_path.exists() or not device_path.exists():
                raise VFIOSetupError("Vendor/device ID files not found")

            vendor_id = vendor_path.read_text().strip()
            device_id = device_path.read_text().strip()

            # Remove 0x prefix
            vendor_id = vendor_id.replace("0x", "")
            device_id = device_id.replace("0x", "")

            self.vendor_device_id = f"{vendor_id} {device_id}"
            logging.info(f"Vendor:Device ID: {self.vendor_device_id}")
            return self.vendor_device_id

        except Exception as e:
            logging.error(f"Failed to read vendor/device ID: {e}")
            raise VFIOSetupError("Could not get device ID")

    def load_vfio_modules(self):
        """Load required VFIO kernel modules"""
        logging.info("Loading VFIO kernel modules...")

        modules = ["vfio", "vfio_iommu_type1", "vfio_pci"]

        for module in modules:
            try:
                # Check if module is already loaded
                result = self.run_command(["lsmod"], check=False)
                if module in result.stdout:
                    logging.info(f"Module {module} already loaded")
                    continue

                # Load the module
                logging.info(f"Loading module {module}...")
                self.run_command(["modprobe", module])

                # Verify loading
                result = self.run_command(["lsmod"], check=False)
                if module in result.stdout:
                    logging.info(f"Module {module} loaded successfully")
                else:
                    raise VFIOSetupError(f"Module {module} not loaded after modprobe")

            except Exception as e:
                logging.error(f"Failed to load module {module}: {e}")
                raise VFIOSetupError(f"Module loading failed: {module}")

    def unbind_current_driver(self):
        """Unbind device from current driver"""
        logging.info("Checking current driver binding...")

        driver_path = self.device_path / "driver"
        if driver_path.is_symlink():
            current_driver = driver_path.readlink().name
            logging.info(f"Device currently bound to driver: {current_driver}")

            if current_driver == "vfio-pci":
                logging.info("Device already bound to vfio-pci")
                return

            logging.info(f"Unbinding from {current_driver}...")
            try:
                unbind_path = driver_path / "unbind"
                unbind_path.write_text(self.full_pci_path)
                time.sleep(1)

                # Verify unbind
                if driver_path.is_symlink():
                    raise VFIOSetupError(f"Failed to unbind from {current_driver}")

                logging.info(f"Successfully unbound from {current_driver}")

            except Exception as e:
                logging.error(f"Failed to unbind from {current_driver}: {e}")
                raise VFIOSetupError("Driver unbinding failed")
        else:
            logging.info("Device not bound to any driver")

    def bind_to_vfio(self):
        """Bind device to vfio-pci driver"""
        logging.info("Binding device to vfio-pci driver...")

        if not self.vendor_device_id:
            raise VFIOSetupError("Vendor/device ID not available")

        try:
            # Add device ID to vfio-pci driver
            new_id_path = self.vfio_pci_driver / "new_id"
            if new_id_path.exists():
                try:
                    new_id_path.write_text(self.vendor_device_id)
                    logging.info("Added device ID to vfio-pci driver")
                except Exception:
                    # This might fail if ID is already added, which is OK
                    logging.info("Device ID may already be registered with vfio-pci")
            else:
                raise VFIOSetupError("Cannot access vfio-pci driver new_id file")

            # Bind the specific device
            bind_path = self.vfio_pci_driver / "bind"
            if bind_path.exists():
                bind_path.write_text(self.full_pci_path)
                time.sleep(1)

                # Verify binding
                driver_path = self.device_path / "driver"
                if driver_path.is_symlink():
                    bound_driver = driver_path.readlink().name
                    if bound_driver == "vfio-pci":
                        logging.info("Device successfully bound to vfio-pci")
                    else:
                        raise VFIOSetupError(
                            f"Device bound to wrong driver: {bound_driver}"
                        )
                else:
                    raise VFIOSetupError("Device binding failed")
            else:
                raise VFIOSetupError("Cannot access vfio-pci driver bind file")

        except Exception as e:
            logging.error(f"Failed to bind to vfio-pci: {e}")
            raise VFIOSetupError("VFIO binding failed")

    def setup_permissions(self):
        """Set up VFIO device permissions"""
        logging.info("Setting up VFIO permissions...")

        # Find IOMMU group
        iommu_group_path = self.device_path / "iommu_group"
        if not iommu_group_path.is_symlink():
            raise VFIOSetupError("Cannot determine IOMMU group")

        iommu_group = iommu_group_path.readlink().name
        logging.info(f"Device is in IOMMU group: {iommu_group}")

        # Set permissions on VFIO group device
        vfio_group_path = pathlib.Path(f"/dev/vfio/{iommu_group}")
        if vfio_group_path.exists():
            try:
                os.chmod(vfio_group_path, 0o666)
                logging.info(f"Set permissions on {vfio_group_path}")
            except Exception as e:
                logging.error(f"Failed to set permissions on {vfio_group_path}: {e}")
                raise VFIOSetupError("Permission setup failed")
        else:
            raise VFIOSetupError(f"VFIO group device {vfio_group_path} not found")

        # Set permissions on VFIO container
        vfio_container_path = pathlib.Path("/dev/vfio/vfio")
        if vfio_container_path.exists():
            try:
                os.chmod(vfio_container_path, 0o666)
                logging.info(f"Set permissions on {vfio_container_path}")
            except Exception as e:
                logging.error(
                    f"Failed to set permissions on {vfio_container_path}: {e}"
                )
                raise VFIOSetupError("Container permission setup failed")
        else:
            raise VFIOSetupError(
                f"VFIO container device {vfio_container_path} not found"
            )

    def verify_setup(self):
        """Verify VFIO setup"""
        logging.info("Verifying VFIO setup...")

        # Check driver binding
        driver_path = self.device_path / "driver"
        if driver_path.is_symlink():
            driver = driver_path.readlink().name
            if driver == "vfio-pci":
                logging.info("✓ Device bound to vfio-pci driver")
            else:
                logging.error(f"✗ Device bound to wrong driver: {driver}")
                raise VFIOSetupError("Driver verification failed")
        else:
            logging.error("✗ Device not bound to any driver")
            raise VFIOSetupError("No driver bound")

        # Check IOMMU group
        iommu_group_path = self.device_path / "iommu_group"
        if iommu_group_path.is_symlink():
            group = iommu_group_path.readlink().name
            logging.info(f"✓ Device in IOMMU group: {group}")

            # Check group device file
            vfio_group_path = pathlib.Path(f"/dev/vfio/{group}")
            if vfio_group_path.exists():
                logging.info(f"✓ VFIO group device exists: {vfio_group_path}")
            else:
                logging.error(f"✗ VFIO group device missing: {vfio_group_path}")
                raise VFIOSetupError("VFIO group device missing")
        else:
            logging.error("✗ Device not in any IOMMU group")
            raise VFIOSetupError("IOMMU group verification failed")

        # Check permissions
        group = iommu_group_path.readlink().name
        vfio_group_path = pathlib.Path(f"/dev/vfio/{group}")
        try:
            stat_info = vfio_group_path.stat()
            perms = oct(stat_info.st_mode)[-3:]
            if perms == "666":
                logging.info(f"✓ VFIO group permissions: {perms}")
            else:
                logging.warning(f"! VFIO group permissions: {perms} (expected: 666)")
        except Exception as e:
            logging.warning(f"Could not check permissions: {e}")

        logging.info("VFIO setup verification completed successfully!")

    def show_device_info(self):
        """Show device information"""
        logging.info("Device Information:")
        print("=" * 50)

        try:
            result = self.run_command(["lspci", "-v", "-s", self.pci_device])
            print(result.stdout)
        except Exception as e:
            logging.warning(f"Could not get device info: {e}")

        logging.info("VFIO Status:")
        print("=" * 30)
        print(f"Device Path: {self.device_path}")

        # Current driver
        driver_path = self.device_path / "driver"
        if driver_path.is_symlink():
            driver = driver_path.readlink().name
            print(f"Driver: {driver}")
        else:
            print("Driver: None")

        # IOMMU group
        iommu_group_path = self.device_path / "iommu_group"
        if iommu_group_path.is_symlink():
            group = iommu_group_path.readlink().name
            print(f"IOMMU Group: {group}")
        else:
            print("IOMMU Group: None")

        print(f"Vendor:Device ID: {self.vendor_device_id}")

    def run_setup(self):
        """Run the complete VFIO setup process"""
        print("=" * 64)
        print(f"VFIO Setup Script for PCI Device {self.pci_device}")
        print("=" * 64)
        print()

        try:
            self.check_root()
            self.check_iommu()
            self.check_device_exists()
            self.get_vendor_device_id()
            self.load_vfio_modules()
            self.unbind_current_driver()
            self.bind_to_vfio()
            self.setup_permissions()
            self.verify_setup()

            print()
            logging.info("VFIO setup completed successfully!")
            print()
            self.show_device_info()

            print()
            logging.info("Device is now ready for VFIO access!")
            print()
            logging.info("You can now use this device with VFIO-based applications.")
            logging.info("For the Rust example program:")
            print("  sudo ./target/release/pci_rust_example")
            print()
            logging.info("Or without sudo if user permissions are configured:")
            print("  ./target/release/pci_rust_example")

        except VFIOSetupError as e:
            logging.error(f"Setup failed: {e}")
            sys.exit(1)
        except KeyboardInterrupt:
            logging.error("Setup interrupted by user")
            sys.exit(1)
        except Exception as e:
            logging.error(f"Unexpected error: {e}")
            sys.exit(1)


def validate_pci_device(device: str) -> str:
    """Validate PCI device format"""
    # Pattern for short format: BB:DD.F (e.g., 22:00.0)
    short_pattern = r"^[0-9a-fA-F]{2}:[0-9a-fA-F]{2}\.[0-9a-fA-F]$"
    # Pattern for full format: SSSS:BB:DD.F (e.g., 0000:22:00.0)
    full_pattern = r"^[0-9a-fA-F]{4}:[0-9a-fA-F]{2}:[0-9a-fA-F]{2}\.[0-9a-fA-F]$"

    if re.match(short_pattern, device) or re.match(full_pattern, device):
        return device
    else:
        raise argparse.ArgumentTypeError(
            f"Invalid PCI device format: {device}. "
            "Expected format: BB:DD.F (e.g., 22:00.0) or SSSS:BB:DD.F (e.g., 0000:22:00.0)"
        )


def parse_arguments():
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(
        description="VFIO Setup Script for PCI Devices",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s 22:00.0                    # Short format (segment assumed to be 0000)
  %(prog)s 0000:22:00.0               # Full format with segment
  %(prog)s 01:00.0                    # Different bus

Note: This script must be run as root (use sudo).
""",
    )

    parser.add_argument(
        "device",
        type=validate_pci_device,
        help="PCI device address in format BB:DD.F or SSSS:BB:DD.F",
    )

    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Enable verbose logging",
    )

    return parser.parse_args()


def main():
    """Main entry point"""
    args = parse_arguments()

    # Setup logging
    setup_logging()
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    # Create and run VFIO setup
    setup = VFIOSetup(args.device)
    setup.run_setup()


if __name__ == "__main__":
    main()
