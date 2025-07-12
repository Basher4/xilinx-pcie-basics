use pci_rust_example::device;
use pci_driver::backends::vfio::VfioPciDevice;
use pci_driver::device::PciDevice;
use pci_driver::regions::PciRegion;
use std::error::Error;
use std::time::{Duration, Instant};
use std::fmt;

#[derive(Debug, Clone)]
struct BenchmarkResult {
    operation: String,
    block_size: usize,
    iterations: usize,
    min_time: Duration,
    max_time: Duration,
    avg_time: Duration,
    throughput_mbps: f64,
}

impl BenchmarkResult {
    fn new(operation: String, block_size: usize, iterations: usize, times: &[Duration]) -> Self {
        let total_time = times.iter().sum::<Duration>();
        let min_time = *times.iter().min().unwrap();
        let max_time = *times.iter().max().unwrap();
        let avg_time = total_time / iterations as u32;
        let total_bytes = block_size * iterations;
        let throughput_mbps = (total_bytes as f64 / (1024.0 * 1024.0)) / total_time.as_secs_f64();

        BenchmarkResult {
            operation,
            block_size,
            iterations,
            min_time,
            max_time,
            avg_time,
            throughput_mbps,
        }
    }
}

impl fmt::Display for BenchmarkResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "{} | {:>6} bytes | {:>6} ops | {:>8.2} MB/s | {:>8.2} ms avg | {:>8.2} ms min | {:>8.2} ms max",
            self.operation,
            self.block_size,
            self.iterations,
            self.throughput_mbps,
            self.avg_time.as_secs_f64() * 1000.0,
            self.min_time.as_secs_f64() * 1000.0,
            self.max_time.as_secs_f64() * 1000.0
        )
    }
}

struct VfioBenchmark {
    device: VfioPciDevice,
    target_size: usize,
}

impl VfioBenchmark {
    fn new(device_addr: &str) -> Result<Self, Box<dyn Error>> {
        println!("Initializing VFIO benchmark for device {}...", device_addr);

        // Open device using common utility
        let device = device::open_device(device_addr)?;

        // Validate that BAR0 is large enough for benchmark requirements
        device::validate_bar_size(&device, 16 * 1024)?;

        println!("Device successfully validated for benchmark");

        Ok(VfioBenchmark {
            device,
            target_size: 16 * 1024, // 16KiB
        })
    }

    fn benchmark_read(&self, block_size: usize, iterations: usize) -> Result<BenchmarkResult, Box<dyn Error>> {
        let mut times = Vec::with_capacity(iterations);
        let mut buffer = vec![0u8; block_size];
        let max_offset = self.target_size - block_size;
        let bar0 = self.device.bar(0).ok_or("Device does not have BAR0")?;

        println!("  Running {} read operations with {}-byte blocks...", iterations, block_size);

        for i in 0..iterations {
            let offset = (i * block_size) % max_offset;
            let start = Instant::now();

            // Read data from BAR0
            for j in 0..block_size {
                buffer[j] = bar0.read_u8((offset + j) as u64)?;
            }

            let elapsed = start.elapsed();
            times.push(elapsed);

            // Progress indicator for longer tests
            if iterations > 1000 && i % (iterations / 10) == 0 {
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
        }

        if iterations > 1000 {
            println!(); // New line after progress dots
        }

        Ok(BenchmarkResult::new("READ ".to_string(), block_size, iterations, &times))
    }

    fn benchmark_write(&self, block_size: usize, iterations: usize) -> Result<BenchmarkResult, Box<dyn Error>> {
        let mut times = Vec::with_capacity(iterations);
        let test_pattern = 0xAA; // Alternating bit pattern
        let max_offset = self.target_size - block_size;
        let bar0 = self.device.bar(0).ok_or("Device does not have BAR0")?;

        println!("  Running {} write operations with {}-byte blocks...", iterations, block_size);

        for i in 0..iterations {
            let offset = (i * block_size) % max_offset;
            let start = Instant::now();

            // Write test pattern to BAR0
            for j in 0..block_size {
                bar0.write_u8((offset + j) as u64, test_pattern)?;
            }

            let elapsed = start.elapsed();
            times.push(elapsed);

            // Progress indicator for longer tests
            if iterations > 1000 && i % (iterations / 10) == 0 {
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
        }

        if iterations > 1000 {
            println!(); // New line after progress dots
        }

        Ok(BenchmarkResult::new("WRITE".to_string(), block_size, iterations, &times))
    }

    fn benchmark_read_write(&self, block_size: usize, iterations: usize) -> Result<BenchmarkResult, Box<dyn Error>> {
        let mut times = Vec::with_capacity(iterations);
        let test_pattern = 0x55; // Alternating bit pattern
        let max_offset = self.target_size - block_size;
        let bar0 = self.device.bar(0).ok_or("Device does not have BAR0")?;

        println!("  Running {} read+write operations with {}-byte blocks...", iterations, block_size);

        for i in 0..iterations {
            let offset = (i * block_size) % max_offset;
            let start = Instant::now();

            // Write then read back
            for j in 0..block_size {
                bar0.write_u8((offset + j) as u64, test_pattern)?;
                let _read_back = bar0.read_u8((offset + j) as u64)?;
            }

            let elapsed = start.elapsed();
            times.push(elapsed);

            // Progress indicator for longer tests
            if iterations > 1000 && i % (iterations / 10) == 0 {
                print!(".");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
            }
        }

        if iterations > 1000 {
            println!(); // New line after progress dots
        }

        Ok(BenchmarkResult::new("R+W  ".to_string(), block_size, iterations, &times))
    }

    fn run_comprehensive_benchmark(&self) -> Result<(), Box<dyn Error>> {
        println!("\n=== VFIO BAR0 Performance Benchmark ===");
        println!("Target region: First 16KiB of BAR0");
        println!("Device: Connected via VFIO");
        println!();

        // Test different block sizes
        let block_sizes = vec![1, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096];
        let mut results = Vec::new();

        for &block_size in &block_sizes {
            // Adjust iterations based on block size to keep test duration reasonable
            let iterations = match block_size {
                1..=16 => 10000,
                17..=64 => 5000,
                65..=256 => 2000,
                257..=1024 => 1000,
                _ => 500,
            };

            println!("\nTesting {}-byte blocks:", block_size);

            // Run read benchmark
            match self.benchmark_read(block_size, iterations) {
                Ok(result) => results.push(result),
                Err(e) => println!("  Read benchmark failed: {}", e),
            }

            // Run write benchmark
            match self.benchmark_write(block_size, iterations) {
                Ok(result) => results.push(result),
                Err(e) => println!("  Write benchmark failed: {}", e),
            }

            // Run read+write benchmark
            match self.benchmark_read_write(block_size, iterations / 2) {
                Ok(result) => results.push(result),
                Err(e) => println!("  Read+Write benchmark failed: {}", e),
            }
        }

        // Display results table
        println!("\n=== BENCHMARK RESULTS ===");
        println!("OP    | Block Size | Ops    | Throughput |  Avg Time |  Min Time |  Max Time");
        println!("------|------------|--------|------------|-----------|-----------|----------");

        for result in &results {
            println!("{}", result);
        }

        // Find best performing configurations
        println!("\n=== PERFORMANCE SUMMARY ===");

        let read_results: Vec<_> = results.iter().filter(|r| r.operation == "READ ").collect();
        let write_results: Vec<_> = results.iter().filter(|r| r.operation == "WRITE").collect();
        let rw_results: Vec<_> = results.iter().filter(|r| r.operation == "R+W  ").collect();

        if let Some(best_read) = read_results.iter().max_by(|a, b| a.throughput_mbps.partial_cmp(&b.throughput_mbps).unwrap()) {
            println!("Best READ performance:  {:.2} MB/s with {}-byte blocks", best_read.throughput_mbps, best_read.block_size);
        }

        if let Some(best_write) = write_results.iter().max_by(|a, b| a.throughput_mbps.partial_cmp(&b.throughput_mbps).unwrap()) {
            println!("Best WRITE performance: {:.2} MB/s with {}-byte blocks", best_write.throughput_mbps, best_write.block_size);
        }

        if let Some(best_rw) = rw_results.iter().max_by(|a, b| a.throughput_mbps.partial_cmp(&b.throughput_mbps).unwrap()) {
            println!("Best R+W performance:   {:.2} MB/s with {}-byte blocks", best_rw.throughput_mbps, best_rw.block_size);
        }

        // Calculate averages
        let avg_read_throughput: f64 = read_results.iter().map(|r| r.throughput_mbps).sum::<f64>() / read_results.len() as f64;
        let avg_write_throughput: f64 = write_results.iter().map(|r| r.throughput_mbps).sum::<f64>() / write_results.len() as f64;
        let avg_rw_throughput: f64 = rw_results.iter().map(|r| r.throughput_mbps).sum::<f64>() / rw_results.len() as f64;

        println!("\nAverage READ throughput:  {:.2} MB/s", avg_read_throughput);
        println!("Average WRITE throughput: {:.2} MB/s", avg_write_throughput);
        println!("Average R+W throughput:   {:.2} MB/s", avg_rw_throughput);

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments using common utility
    let device_addr = device::parse_device_args()?;

    println!("VFIO BAR0 Benchmark Tool");
    println!("========================");
    println!();

    // Initialize benchmark
    let benchmark = VfioBenchmark::new(&device_addr)?;

    // Run comprehensive benchmark
    benchmark.run_comprehensive_benchmark()?;

    println!("\nBenchmark completed successfully!");

    Ok(())
}