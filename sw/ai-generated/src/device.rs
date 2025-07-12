use pci_driver::backends::vfio::VfioPciDevice;
use pci_driver::device::PciDevice;
use pci_driver::regions::PciRegion;
use std::error::Error;
use std::fmt::Write;

/// Parse and validate command-line arguments for PCI device address
pub fn parse_device_args() -> Result<String, Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <pci_device_address>", args[0]);
        eprintln!("Example: {} 22:00.0", args[0]);
        eprintln!("         {} 0000:22:00.0", args[0]);
        std::process::exit(1);
    }

    let device_addr = args[1].clone();

    // Validate device address format
    if !device_addr.contains(':') || !device_addr.contains('.') {
        eprintln!("Error: Invalid PCI device address format");
        eprintln!("Expected format: BB:DD.F or SSSS:BB:DD.F");
        std::process::exit(1);
    }

    Ok(device_addr)
}

/// Format device address into system device path
pub fn format_device_path(device_addr: &str) -> String {
    if device_addr.contains("0000:") {
        format!("/sys/bus/pci/devices/{}", device_addr)
    } else {
        format!("/sys/bus/pci/devices/0000:{}", device_addr)
    }
}

/// Open PCI device with proper error handling and validation
pub fn open_device(device_addr: &str) -> Result<VfioPciDevice, Box<dyn Error>> {
    let device_path = format_device_path(device_addr);
    println!("Opening device: {} ({})", device_addr, device_path);

    let device = VfioPciDevice::open(&device_path)?;
    println!("Device opened successfully");

    // Validate that we can access BAR0
    let bar0 = device.bar(0).ok_or("Device does not have BAR0")?;
    println!("BAR0 size: {} bytes", bar0.len());
    println!("BAR0 permissions: {:?}", bar0.permissions());

    Ok(device)
}

/// Print hex dump of data with ASCII representation
pub fn print_hex_dump(data: &[u8], base_offset: u64) {
    for (i, chunk) in data.chunks(16).enumerate() {
        let mut hex_part = String::new();
        let mut ascii_part = String::new();

        // Print offset
        print!("{:08x}: ", base_offset + (i * 16) as u64);

        // Print hex bytes
        for (j, &byte) in chunk.iter().enumerate() {
            if j == 8 {
                hex_part.push(' '); // Extra space after 8 bytes
            }
            write!(&mut hex_part, "{:02x} ", byte).unwrap();
        }

        // Pad hex part if needed
        while hex_part.len() < 50 {
            hex_part.push(' ');
        }

        // Print ASCII representation
        for &byte in chunk {
            if byte >= 32 && byte <= 126 {
                ascii_part.push(byte as char);
            } else {
                ascii_part.push('.');
            }
        }

        println!("{} |{}|", hex_part, ascii_part);
    }
}

/// Validate that BAR0 is large enough for the given size requirement
pub fn validate_bar_size(device: &VfioPciDevice, min_size: u64) -> Result<(), Box<dyn Error>> {
    let bar0 = device.bar(0).ok_or("Device does not have BAR0")?;
    if bar0.len() < min_size {
        return Err(format!("BAR0 size ({} bytes) is less than required {} bytes", bar0.len(), min_size).into());
    }
    Ok(())
}

/// Check if BAR0 supports write operations
pub fn bar_supports_write(device: &VfioPciDevice) -> Result<bool, Box<dyn Error>> {
    let bar0 = device.bar(0).ok_or("Device does not have BAR0")?;
    Ok(bar0.permissions().can_write())
}