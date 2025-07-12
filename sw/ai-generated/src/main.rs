use pci_rust_example::device;
use pci_driver::device::PciDevice;
use pci_driver::regions::PciRegion;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments
    let device_addr = device::parse_device_args()?;

    println!("PCI VFIO BAR0 Access Example");
    println!("============================");

    // Open device using common utility
    let device = device::open_device(&device_addr)?;

    // Get BAR0
    let bar0 = device.bar(0).ok_or("Device does not have BAR0")?;

    // Process offset 0
    println!("\n--- Processing offset 0x0000 ---");
    process_offset(&bar0, 0)?;

    // Process offset 0x4000
    println!("\n--- Processing offset 0x4000 ---");
    process_offset(&bar0, 0x4000)?;

    println!("\nProgram completed successfully!");
    Ok(())
}

fn process_offset(bar0: &impl PciRegion, offset: u64) -> Result<(), Box<dyn Error>> {
    // Check if the offset is within bounds
    if offset + 64 > bar0.len() {
        return Err(format!("Offset 0x{:04x} + 64 bytes exceeds BAR0 size", offset).into());
    }

    println!("Reading 64 bytes from offset 0x{:04x}:", offset);

    // Read and display first 64 bytes
    let mut initial_data = vec![0u8; 64];
    for i in 0..64 {
        initial_data[i] = bar0.read_u8(offset + i as u64)?;
    }

    println!("Initial contents:");
    device::print_hex_dump(&initial_data, offset);

    // Write a 64-bit value (0x1234567890ABCDEF) to offset 0
    let test_value: u64 = 0x1234567890ABCDEF;
    println!(
        "\nWriting 64-bit value 0x{:016x} to offset 0x{:04x}",
        test_value, offset
    );

    // Check if we can write to this region
    if !bar0.permissions().can_write() {
        println!("Warning: BAR0 is read-only, write operation will be skipped");
        return Ok(());
    }

    // Write the 64-bit value in little-endian format (as two 32-bit writes)
    bar0.write_le_u32(offset, (test_value & 0xFFFFFFFF) as u32)?;
    bar0.write_le_u32(offset + 4, (test_value >> 32) as u32)?;

    println!("Write completed");

    // Read and display the contents again
    println!("\nReading 64 bytes again from offset 0x{:04x}:", offset);
    let mut updated_data = vec![0u8; 64];
    for i in 0..64 {
        updated_data[i] = bar0.read_u8(offset + i as u64)?;
    }

    println!("Updated contents:");
    device::print_hex_dump(&updated_data, offset);

    // Compare the first 8 bytes to see if our write took effect
    let low_word = bar0.read_le_u32(offset)? as u64;
    let high_word = bar0.read_le_u32(offset + 4)? as u64;
    let read_back_value = low_word | (high_word << 32);
    if read_back_value == test_value {
        println!(
            "✓ Write verification successful: 0x{:016x}",
            read_back_value
        );
    } else {
        println!("! Write verification failed:");
        println!("  Expected: 0x{:016x}", test_value);
        println!("  Got:      0x{:016x}", read_back_value);
    }

    Ok(())
}


