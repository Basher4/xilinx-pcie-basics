use pci_driver::backends::vfio::VfioPciDevice;
use pci_driver::device::PciDevice;
use pci_driver::regions::PciRegion;

const DEVICE_PATH: &str = "/sys/bus/pci/devices/0000:22:00.0";
const TOTAL_READ_BYTES: usize = 16 << 20;

const READ_SIZE: &[usize] = &[
    // 1 << 0,  // 1 byte
    // 1 << 3,  // 8 bytes
    // 1 << 6,  // 64 bytes
    1 << 7,  // 128 bytes
    // 1 << 9,  // 512 bytes
    // 1 << 10, // 1 KiB
    // 1 << 12, // 4 KiB
];

fn mbps(size: usize, duration: &std::time::Duration) -> f64 {
    (size as f64) / 1024.0 / 1024.0 / duration.as_secs_f64()
}

fn main() {
    println!("Starting the benchmark...");

    let device = VfioPciDevice::open(DEVICE_PATH)
        .expect(format!("Can open VFIO device {DEVICE_PATH}").as_str());
    let bar0 = device.bar(0).expect("Can get BAR0 of the device");

    for size in READ_SIZE.iter().copied() {
        // println!("Transaction size: {size}");

        let mut buf = vec![0u8; size];
        let iters = TOTAL_READ_BYTES / size;
        let mut samples = Vec::with_capacity(iters);

        for _ in 0..iters {
            let start = std::time::Instant::now();
            bar0.read_bytes(0, buf.as_mut_slice())
                .expect(format!("Can read {size} bytes from BAR0").as_str());
            let elapsed = start.elapsed();
            samples.push(elapsed);
        }

        let mean_time = samples.iter().sum::<std::time::Duration>() / samples.len() as u32;
        let min_time = samples.iter().min().unwrap();
        let max_time = samples.iter().max().unwrap();

        println!(
            "Read size: {:4} bytes, Mean: {:.3} MiB/s, Min: {:.3} MiB/s, Max: {:.3} MiB/s",
            size, mbps(size, &mean_time), mbps(size, &min_time), mbps(size, &max_time)
        );
    }


    let start = std::time::Instant::now();
    for _ in 0..(TOTAL_READ_BYTES / 4) {
        bar0.write_le_u32(0x0, 0xCAFE).expect("Can write");
    }
    let elapsed = start.elapsed();

    println!("Write bandwidth: {:4.3} MiB/s", mbps(TOTAL_READ_BYTES, &elapsed));
}
