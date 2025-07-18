PCIe on Ultrascale+ FPGA: Hello World
=====================================

A simple demo to read and write to a BRAM via PCIe, using a cheap Xilinx KU5P board from [Aliexpress](https://www.aliexpress.com/item/4001302554837.html)

![Aliexpress Kintex UltraScale+ board](./docs/imgs/aliexpress_ku5p.jpg)

# Hardware

## Architecture

Since this is a hello world project, the design is very simple.

I am using XDMA IP in AXI Bridge Mode ([PG194](https://docs.amd.com/r/en-US/pg194-axi-bridge-pcie-gen3/Introduction)) to avoid the Xiling XDMA driver.
I can access some memory backed by BRAM (the board has off-chip memory) and I use the few provided LEDs to show whether the link is up and whether we have clock on the AXI bus.

An ILA connected to the AXI Master bus will let us investigate what is happening when we do PCIe transactions.

![Block Diagram](./docs/imgs/block.svg)

There are two memories because if I use Memory Generator in BRAM Controller mode I cannot initialize the block ram from a coefficient file. I want to see if I can wire the AXI BRAM Controller directly to a BRAM, since that configuration does allow me 

## Memory map
- 0x0000 .. 0x3FFF (16k) - `blk_mem_gen_0` (only 4k backed by BRAM)
- 0x4000 .. 0x7FFF (16k) - `blk_mem_gen_1` (only 4k backed by BRAM)

## Building

According to [this Xilinx article](https://adaptivesupport.amd.com/s/article/Revision-Control-with-a-Vivado-Project?language=en_US) the recommended way to version control vivado projects is to keep the project in the git repo. So that's what I'm doing.

Project was created using Vivado 2025.1. To generate the bitstream, it should be enough to open the `./hw/vivado/00_blank_cpie.xpr` project and build itvivado.

# Software

Software to interact with the FPGA is using [VFIO](https://docs.kernel.org/driver-api/vfio.html). All modern linux distros ship with VFIO driver.

## Building

To build the C examples, simply run

```bash
gcc <file.c> -o <file>
```

Rust code builds with cargo. If you don't have rust installed, follow instructions at [https://rustup.rs/](https://rustup.rs/).

```bash
cd ./sw/ai-generated
cargo build --release
```

## Running the examples

1. Bind the device to VFIO driver (you can use the AI-generated `setup_vfio.py` script)
1. Run the example as root
    - If you're running the rust example, you might need to directly run the resulting binary, for example `sudo ./target/release/pci_rust_example 22:00.0`
