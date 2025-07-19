PCIe on Ultrascale+ FPGA: Hello World
=====================================

A while ago I procured this Xilinx KU5P board from [Aliexpress](https://www.aliexpress.com/item/4001302554837.html)

![Aliexpress Kintex UltraScale+ board](./docs/imgs/aliexpress_ku5p.jpg)

It's pretty barren, with a few LEDs, 2x QSFP+ ports, PCIe 3 x8 interface, and a mandatory SPI chip. Having an on-board
DDR would be nice, but considering it costs only £150 I'm not going to be too picky.

# Hardware

I watched [FPGAZealot's YouTube video](https://www.youtube.com/watch?v=m56rBYnmxME) where he's using the
[XDMA IP (PG195)](https://docs.amd.com/r/en-US/pg195-pcie-dma/Introduction) to interface with his FPGA. It looked like
XDMA is ais a very simple and performant way to talk to the FPGA from a host over PCIe. However, I don't like that
to use XDMA I need to compile and load a [custom driver](https://github.com/Xilinx/dma_ip_drivers/tree/master). For my
first PCIe project I wanted to keep things as simple as possible. Both on the FPGA side and software.

I discovered that the XDMA IP can work in
[AXI Bridge Mode (PG194)](https://docs.amd.com/r/en-US/pg194-axi-bridge-pcie-gen3/Introduction) as well!

![XDMA IP Configured in AXI Bridge Mode](./docs/imgs/xdma-axi-bridge-mode.png)

With this configuration the IP will translate reads and writes to BAR0 of the PCIe device as AXI4 reads and writes.

![XDMA IP BAR0 to AXI offset translateion](./docs/imgs/xdma-axi-bridge-pcie-to-axi-translate.png)

This should make writing software really simple, or so I thought.

## Architecture

The block diagram contains two BRAM memories accessible via the AXI network, an LED to indicate whether the link is up,
and an LED to indicate whether we have clock on the AXI bus.

![Block Diagram](./docs/imgs/block.svg)

## AXI memory map

- 0x0000 .. 0x3FFF (16k) - `blk_mem_gen_0` (only 4k backed by BRAM)
- 0x4000 .. 0x7FFF (16k) - `blk_mem_gen_1` (only 4k backed by BRAM)

The two BRAMs are wired slightly differently. If I configure the Block Memory Generator as a BRAM Controller, which is
designed to work with AXI BRAM Controller, I cannot initialize the block ram from a coefficient file. So I want to try
to wire the AXI BRAM Controller directly to a BRAM, and see if that will work.

# Connecting the FPGA to a computer

Now that the hardware is done, I need to connect it to a computer. The standard way is to plug it into a spare PCIe slot
on a desktop computer, or lately to a SBC like a Raspberry Pi 5 that has a PCIe slot.

Unfortunately, I do not have such an SBC or a spare computer that I'd be willing to let running the whole time.

Another downside of this approach is that every time I change the size of the PCIe BARs (which I didn't know how often I
would do) I would need to restart the computer. Which might reset the FPGA. Without the correct bitstream loaded, the
FPGA will not show up as a PCIe device. That means I would need to program the on-board flash. If possible I'd like to
avoid both of these steps.

## Thunderbolt to the rescue

My laptop is running linux, and it has two USB 4 ports that support Thunderbolt. And Thunderbolt can provide external
access to the PCIe bus. Using a [cheap Thunderbolt 4 NVMe enclosure](https://fideco-it.com/products/fideco-usb-4-40gbps-type-c-nvme-enclosure-with-cooling-fan-mt402)
and an [NVMe to PCIe adapter](https://www.amazon.co.uk/dp/B0CTT7JQ5D) I was able to connect the FPGA to my laptop.

[//]: # (TODO: Add an image of my real setup)

One benefit of this approach is that if I do make changes to the BARs, I don't need to reboot my computert. I can just
unplug and plug in the external NVMe enclosure.

After plugging in the USB-C cable, I see the clock LED flashing, link up LED is lit, and my device has showed up!

```bash
$ lspci
20:00.0 PCI bridge: ASMedia Technology Inc. Device 2463
21:00.0 PCI bridge: ASMedia Technology Inc. Device 2463
22:00.0 Serial controller: Xilinx Corporation Device 9031

$ sudo lspci -s 22:00.0 -vnn
22:00.0 Serial controller [0700]: Xilinx Corporation Device [10ee:9031] (prog-if 01 [16450])
        Subsystem: Xilinx Corporation Device [10ee:0007]
        Flags: fast devsel, IRQ 16, IOMMU group 16
        Memory at b0000000 (32-bit, non-prefetchable) [size=128K]
        Capabilities: [40] Power Management version 3
        Capabilities: [48] MSI: Enable- Count=1/1 Maskable- 64bit+
        Capabilities: [70] Express Endpoint, MSI 00
        Capabilities: [100] Advanced Error Reporting
        Capabilities: [1c0] Secondary PCI Express

$ lspci -s 22:00.0 -vnnt
-[0000:00]---07.0-[20-49]----00.0-[21-22]----00.0-[22]----00.0  Xilinx Corporation Device [10ee:9031]
```

An interesting observation is that the Thunderbolt connects the device through 2 PCI bridges. I wasn't expecting that.
Let's have a look at the two devices:

```bash
$ sudo lspci -s 20:00.0 -v
20:00.0 PCI bridge: ASMedia Technology Inc. Device 2463 (prog-if 00 [Normal decode])
        Subsystem: ASMedia Technology Inc. Device 2463
        Physical Slot: 3
        Flags: bus master, fast devsel, latency 0, IRQ 16, IOMMU group 15
        Bus: primary=20, secondary=21, subordinate=22, sec-latency=0
        I/O behind bridge: 3000-3fff [size=4K] [16-bit]
        Memory behind bridge: b0000000-bc1fffff [size=194M] [32-bit]
        Prefetchable memory behind bridge: 6000000000-601bffffff [size=448M] [32-bit]
        Capabilities: [50] MSI: Enable- Count=1/1 Maskable- 64bit+
        Capabilities: [70] Power Management version 3
        Capabilities: [80] Express Upstream Port, MSI 00
        Capabilities: [c0] Subsystem: ASMedia Technology Inc. Device 2463
        Capabilities: [100] Advanced Error Reporting
        Capabilities: [160] Latency Tolerance Reporting
        Capabilities: [1c0] Secondary PCI Express
        Capabilities: [200] L1 PM Substates
        Capabilities: [220] Data Link Feature <?>
        Capabilities: [240] Physical Layer 16.0 GT/s <?>
        Capabilities: [280] Lane Margining at the Receiver <?>
        Kernel driver in use: pcieport

$ sudo lspci -s 21:00.0 -v
21:00.0 PCI bridge: ASMedia Technology Inc. Device 2463 (prog-if 00 [Normal decode])
        Subsystem: ASMedia Technology Inc. Device 2463
        Flags: bus master, fast devsel, latency 0, IRQ 210, IOMMU group 16
        Bus: primary=21, secondary=22, subordinate=22, sec-latency=0
        I/O behind bridge: 3000-3fff [size=4K] [16-bit]
        Memory behind bridge: b0000000-bc1fffff [size=194M] [32-bit]
        Prefetchable memory behind bridge: 6000000000-601bffffff [size=448M] [32-bit]
        Capabilities: [50] MSI: Enable+ Count=1/1 Maskable- 64bit+
        Capabilities: [70] Power Management version 3
        Capabilities: [80] Express Downstream Port (Slot+), MSI 00
        Capabilities: [c0] Subsystem: ASMedia Technology Inc. Device 2463
        Capabilities: [100] Advanced Error Reporting
        Capabilities: [1c0] Secondary PCI Express
        Capabilities: [200] L1 PM Substates
        Capabilities: [220] Data Link Feature <?>
        Capabilities: [240] Physical Layer 16.0 GT/s <?>
        Capabilities: [280] Lane Margining at the Receiver <?>
        Kernel driver in use: pcieport
```

If you look at capability 0x80, you can see taht 20:00.0 is an upstream port, and 21:00.0 is a downstream port. Today
I learned that PCIe bridges (and possibly switches too?) will show up as multiple functions. One for the upstream port
that connects to the system and one for evey downstream port.

# Talking to the FPGA

Now that the FPGA is programmed and connected to my computer, let's talk to it.

## Reading BAR0

As a very first sanity check, let's read and write to BAR0. On linux that should be really simple because linux provides
a sysfs itnerface for PCI devices. If we look at the sysfs of the device, we see a file `resource0` which corresponds to
BAR0.

```bash
$ ll /sys/bus/pci/devices/0000:22:00.0/resource0
-rw------- 1 root root 128K Jul 19 13:23 /sys/bus/pci/devices/0000:22:00.0/resource0
```

It has the right size - we configured the BAR0 to be 128KiB. Let's read the first 64 bytes.

```bash
$ sudo hexdump -n 64 /sys/bus/pci/devices/0000:22:00.0/resource0
hexdump: /sys/bus/pci/devices/0000:22:00.0/resource0: Input/output error
```

Well... that's not what I was expecting. But after inspecting the the [`pci_create_attr`](https://elixir.bootlin.com/linux/v6.15.6/source/drivers/pci/pci-sysfs.c#L1193)
function that creates the `resource0` file we can see that read/write syscalls are implemetned only for BARs that are in
IO space.

No problem, let's throw together a small C program to open the and MMAP the first few bytes of BAR0:

```c
#include <stdio.h>
#include <fcntl.h>
#include <sys/mman.h>

int main(void) {
    int fd = open("/sys/bus/pci/devices/0000:22:00.0/resource0", O_RDONLY);
    if (fd < 0) {
        perror("open");
        return 1;
    }

    int *addr = (int*)mmap(NULL, 128, PROT_READ, MAP_SHARED, fd, 0);
    if (addr == MAP_FAILED) {
        perror("mmap");
        return 1;
    }

    printf("%x\n", addr[0]);
    return 0;
}
```

Let's compile and run this program as root

```bash
$ gcc bar0.c -o bar0
$ sudo ./bar0
mmap: Operation not permitted
```

And it still fails. I wansn't quite sure what was going on but I had 2 theories:
1. I'm running Ubuntu 24; perhaps AppArmor has rules that forbid mmaping PCIe BARs from userspace
1. Device is connected via Thudnerbolt and there might be extra security in place to prevent attacks from malicious
   Thunderbolt devices

It was simple enough to test whether AppArmor is the issue.

```
$ sudo systemctl stop apparmor

$ sudo ./bar0
mmap: Operation not permitted

$ sudo systemctl start apparmor
```

Likely not, then.

Could the issue be because of thunderbolt

After a bit of googling I discovered that the root cause is something completely different. A user in
[this reddit thread](https://www.reddit.com/r/linux_programming/comments/pg0yy2/strange_mmap_for_accessing_a_pci_bar_over_sysfs/)
had a similar issue. However, his kernel config had `CONFIG_IO_STRICT_DEVMEM` set, which mine doesn't.

```bash
$ grep DEVMEM /boot/config-6.8.0-63-generic
CONFIG_DEVMEM=y
CONFIG_ARCH_HAS_DEVMEM_IS_ALLOWED=y
CONFIG_STRICT_DEVMEM=y
# CONFIG_IO_STRICT_DEVMEM is not set
```

Solution to the redditor's problem was to add `iomem=relaxed` to the kernel command line. However, when I tried this I
still couldn't read the contents of BAR0.

# VFIO

VFIO (Virtual Function I/O) is a linux kernel framework that allows us to write device drivers in userspace. In this
project I use it only to read and write to the PCIe BAR, but the most useful features of VFIO are DMA and interrupt
remapping.

With DMA remapping I can create buffers in my application and VFIO configures the IOMMU to point certain device
virtual addresses to my buffers. With interrupt remapping we can use eventfds in our application.

More info on VFIO and how it works can be found at [docs.kernel.org](https://docs.kernel.org/driver-api/vfio.html).

## VFIO crash course

Using VFIO is surprisingly simple.

### 1. Create a container

Think of a container as a set of devices that will 

```c
int container_fd = open("/dev/vfio/vfio", O_RDWR);
```

## AI take the wheel

- Cursor trial through my employer

## Why so slow?

