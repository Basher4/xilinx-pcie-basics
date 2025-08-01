# PCI VFIO BAR0 Access Tools

This Rust project provides safe PCI device memory access using VFIO (Virtual Function I/O) framework. It includes two binaries:

1. **pci_rust_example** - Basic example demonstrating BAR0 access with hex dump and write verification
2. **benchmark** - Comprehensive performance testing tool for BAR0 read/write operations

## Features

### Basic Example (pci_rust_example)
- Safe access to PCI device memory using the `pci-driver` crate
- No unsafe code required
- Hex dump display of memory contents
- 64-bit value read/write operations
- Verification of write operations

### Benchmark Tool (benchmark)
- Performance testing for various block sizes (1 byte to 4KB)
- Separate read, write, and combined read+write benchmarks
- Throughput measurements in MB/s with timing statistics
- Comprehensive performance analysis and summaries
- Support for any PCI device address

## Prerequisites

### 1. VFIO Kernel Module

Ensure VFIO is enabled in your kernel:

```bash
# Check if VFIO modules are loaded
lsmod | grep vfio

# Load VFIO modules if not present
sudo modprobe vfio
sudo modprobe vfio-pci
```

### 2. IOMMU Support

Enable IOMMU in your system:

**For Intel systems (VT-d):**
Add `intel_iommu=on` to your kernel command line parameters.

**For AMD systems (AMD-Vi):**
Add `amd_iommu=on` to your kernel command line parameters.

Edit `/etc/default/grub`:
```bash
GRUB_CMDLINE_LINUX="intel_iommu=on iommu=pt"
```

Update GRUB and reboot:
```bash
sudo update-grub
sudo reboot
```

### 3. Bind Device to VFIO Driver

**Option A: Use the automated setup script (Recommended)**

The easiest way is to use the included Python script:

```bash
# Make sure the script is executable
chmod +x setup_vfio.py

# Run for your specific device (replace 22:00.0 with your device address)
sudo ./setup_vfio.py 22:00.0
```

**Option B: Manual setup**

First, identify your device and its current driver:

```bash
# Find the device
lspci -v | grep -A 10 "22:00.0"

# Note the vendor:device ID (e.g., 10ee:0666)
lspci -n -s 22:00.0
```

Bind the device to VFIO:

```bash
# Replace 10ee:0666 with your actual vendor:device ID
echo "10ee 0666" | sudo tee /sys/bus/pci/drivers/vfio-pci/new_id

# Unbind from current driver (if any)
echo "0000:22:00.0" | sudo tee /sys/bus/pci/devices/0000:22:00.0/driver/unbind

# Bind to VFIO
echo "0000:22:00.0" | sudo tee /sys/bus/pci/drivers/vfio-pci/bind
```

Or use the `vfio-pci` module parameter:
```bash
# Add to kernel parameters
modprobe vfio-pci ids=10ee:0666
```

### 4. Permissions

Ensure your user has access to VFIO devices:

```bash
# Add user to vfio group
sudo usermod -a -G vfio $USER

# Or temporarily change permissions
sudo chmod 666 /dev/vfio/*
```

## Project Structure

This project is explicitly configured to build two binaries with shared common functionality:

```toml
# Cargo.toml defines two binaries
[[bin]]
name = "pci_rust_example"  # Basic example (src/main.rs)
path = "src/main.rs"

[[bin]]
name = "benchmark"         # Performance tool (src/bin/benchmark.rs)
path = "src/bin/benchmark.rs"
```

### Code Architecture

The project uses a modular architecture with shared functionality:

- **`src/lib.rs`** - Library root that exposes the device module
- **`src/device.rs`** - Common PCI device operations module
  - `device::parse_device_args()` - Command-line argument parsing and validation
  - `device::open_device()` - Device opening with error handling and validation
  - `device::print_hex_dump()` - Hex dump formatting utility
  - `device::validate_bar_size()` - BAR size validation
  - `device::bar_supports_write()` - Write permission checking
- **`src/main.rs`** - Basic PCI access example (uses device module)
- **`src/bin/benchmark.rs`** - Performance benchmark tool (uses device module)

## Building and Running

### Build both programs:

```bash
# Build both binaries
cargo build --release

# Or build specific binary
cargo build --release --bin pci_rust_example
cargo build --release --bin benchmark
```

### Set up VFIO for your device:

First, identify your PCI device:
```bash
lspci | grep -i <your_device_type>
```

Then run the VFIO setup script:
```bash
# For device at address 22:00.0 (example)
sudo ./setup_vfio.py 22:00.0

# For a different device
sudo ./setup_vfio.py 01:00.0

# With verbose output
sudo ./setup_vfio.py -v 22:00.0

# Full format with segment
sudo ./setup_vfio.py 0000:22:00.0
```

### Run the basic example:

```bash
# Run with appropriate permissions for device 22:00.0
sudo ./target/release/pci_rust_example 22:00.0

# Run for a different device
sudo ./target/release/pci_rust_example 01:00.0

# Using full format
sudo ./target/release/pci_rust_example 0000:22:00.0

# Or if user permissions are set up correctly
./target/release/pci_rust_example 22:00.0
```

### Run the benchmark tool:

The project includes a comprehensive benchmark tool that measures read/write performance to BAR0:

```bash
# Run benchmark for device 22:00.0
sudo ./target/release/benchmark 22:00.0

# Run benchmark for a different device
sudo ./target/release/benchmark 01:00.0

# Using full format
sudo ./target/release/benchmark 0000:22:00.0
```

The benchmark tool will:
- Test various block sizes (1 byte to 4KB)
- Measure read, write, and combined read+write performance
- Display throughput in MB/s and timing statistics
- Show performance summary with best configurations

## Expected Output

The program will:

1. Open the specified PCIe device (e.g., `22:00.0`)
2. Access BAR0 and display its size and permissions
3. For offset `0x0000`:
   - Display the first 64 bytes in hex dump format
   - Write a test value (`0x1234567890ABCDEF`) to the first 8 bytes
   - Display the updated contents
   - Verify the write operation
4. Repeat the same process for offset `0x4000`

Example output for `sudo ./target/release/pci_rust_example 22:00.0`:
```
PCI VFIO BAR0 Access Example
============================
Opening device: 22:00.0 (/sys/bus/pci/devices/0000:22:00.0)
Device opened successfully
BAR0 size: 65536 bytes
BAR0 permissions: ReadWrite

--- Processing offset 0x0000 ---
Reading 64 bytes from offset 0x0000:
Initial contents:
00000000: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
00000010: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
00000020: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
00000030: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|

Writing 64-bit value 0x1234567890abcdef to offset 0x0000
Write completed

Reading 64 bytes again from offset 0x0000:
Updated contents:
00000000: ef cd ab 90 78 56 34 12 00 00 00 00 00 00 00 00  |....xV4.........|
00000010: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
00000020: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|
00000030: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  |................|

✓ Write verification successful: 0x1234567890abcdef

--- Processing offset 0x4000 ---
[... similar output for offset 0x4000 ...]

Program completed successfully!
```

## Benchmark Tool Output

The benchmark tool provides comprehensive performance analysis:

```
VFIO BAR0 Benchmark Tool
========================

Initializing VFIO benchmark for device 22:00.0...
Successfully opened device 22:00.0 (BAR0 size: 65536 bytes)

=== VFIO BAR0 Performance Benchmark ===
Target region: First 16KiB of BAR0
Device: Connected via VFIO

Testing 1-byte blocks:
  Running 10000 read operations with 1-byte blocks...
  Running 10000 write operations with 1-byte blocks...
  Running 5000 read+write operations with 1-byte blocks...

[... similar output for other block sizes ...]

=== BENCHMARK RESULTS ===
OP    | Block Size | Ops    | Throughput |  Avg Time |  Min Time |  Max Time
------|------------|--------|------------|-----------|-----------|----------
READ  |      1 bytes |  10000 |     2.34 MB/s |     0.42 ms |     0.38 ms |     1.23 ms
WRITE |      1 bytes |  10000 |     1.89 MB/s |     0.53 ms |     0.45 ms |     1.87 ms
R+W   |      1 bytes |   5000 |     1.12 MB/s |     0.89 ms |     0.78 ms |     2.34 ms
[... results for other block sizes ...]

=== PERFORMANCE SUMMARY ===
Best READ performance:  15.23 MB/s with 1024-byte blocks
Best WRITE performance: 12.87 MB/s with 512-byte blocks
Best R+W performance:   8.45 MB/s with 256-byte blocks

Average READ throughput:  8.45 MB/s
Average WRITE throughput: 7.12 MB/s
Average R+W throughput:   4.78 MB/s

Benchmark completed successfully!
```

## Important Notes

- **Generic Device Support**: Both tools now accept any PCI device address as a command-line argument. Use the format `BB:DD.F` (e.g., `22:00.0`) or `SSSS:BB:DD.F` (e.g., `0000:22:00.0`).

- **BAR0 Size**: The basic example assumes BAR0 is large enough to accommodate reads/writes at offset `0x4000`. If your device has a smaller BAR0, the program will report an error. The benchmark tool tests the first 16KiB of BAR0.

- **Write Permissions**: Some devices may have read-only BARs. Both programs will detect this and skip write operations.

- **VFIO Setup**: Proper VFIO setup is crucial. The device must be unbound from its native driver and bound to the VFIO driver.

- **Root Privileges**: Depending on your system configuration, you may need to run the program as root or ensure proper user permissions for VFIO device access.

## Troubleshooting

### "No such file or directory" error:
- Verify the device exists: `ls /sys/bus/pci/devices/0000:22:00.0`
- Check if the device is bound to VFIO: `lspci -v -s 22:00.0`

### "Permission denied" error:
- Check VFIO device permissions: `ls -la /dev/vfio/`
- Ensure your user is in the `vfio` group: `groups $USER`

### "Device does not have BAR0" error:
- Check the device's BAR configuration: `lspci -v -s 22:00.0`
- Some devices may not have BAR0 or it may be disabled

## Dependencies

- `pci-driver` crate with VFIO backend support
- Linux kernel with VFIO and IOMMU support
- Rust 1.56 or later

## Project Metadata

This project is configured with the following metadata in `Cargo.toml`:

- **Description**: "Safe PCI device access using VFIO with Rust - includes basic example and performance benchmark tools"
- **License**: MIT
- **Authors**: AI Generated
- **Version**: 0.1.0

## License

MIT License - This project is provided as-is for educational purposes. Use responsibly and ensure you have proper permissions to access the hardware device.

## Binary Targets

The project explicitly defines two binary targets:

1. **pci_rust_example** (`src/main.rs`) - Basic PCI device access demonstration
2. **benchmark** (`src/bin/benchmark.rs`) - Performance benchmark tool

Both can be built individually using `cargo build --bin <name>` or together using `cargo build --bins`.