# Why CPU-initiated MMIO reads crawl while writes cruise

The asymmetry starts at the CPU front-side bus, goes through the uncore, the PCIe root complex and ends in the device.
Below is the full chain, from micro-ops to TLPs.

## 1. Micro-architecture: posted vs. non-posted

CPU **stores** to an uncacheable (UC) mapping are *posted*:
- The core issues a write micro-op.
- It deposits the data into a write buffer (or a write-combining (WC) buffer if the memory type, as set by the
  PageAttribute Table (PAT), is WC).
- As soon as the buffer entry is filled the core retires the instruction; execution continues.
- The uncore turns the entry into a PCIe **Memory Write** TLP whenever the link is free.
- Latency hidden; throughput limited only by buffer size & link bandwidth.

CPU **loads** from UC memory are *non-posted*:
- Core sends a read request into the Request Queue.
- Retirement is **blocked** until the matching data comes back.
- A single read generates **two** TLPs:
  1. Memory Read Request (header only, 16 B)
  2. Completion w/ Data (header + payload, min 20 B header + n × DW)
- Round-trip must traverse: core → LLC → iMC → Root Port → multiple PCIe hops → device → back.
  Typical x86 latency: 200–600 ns.

With a single outstanding request you waste the entire RTT, so the effective bandwidth is

$\text{BW} = \frac{\text{payload}}{\text{RTT}} = \frac{8\text{–}64 B}{250 ns} ≈ 30–250 MiB/s$

Your ILA captured ~3 MiB/s because **only 8 B** (one DW) were requested and the core could not issue the next read until
the previous one retired.

## 2. Outstanding requests & MRRS

Modern CPUs can track dozens of reads, but two things cap the flight depth:

1. **Uncacheable mapping** – the core issues at most one linearly-addressed UC load per thread.
2. **Max Read Request Size (MRRS)** advertised by the device / bridge. Default is 128 B; you measured 8 B.

Result: You were effectively limited to **one 8-byte request in flight**. Writes, by contrast, stream out as 128-byte
bursts and queue up in the root port.

## 3. PCIe protocol overhead

Each TLP carries headers and “dead time”:

Memory Write:   3 DW header + payload (posted)
Memory Read:    3 DW req header **plus** 3 DW cpl header + payload (non-posted)

Example for 8-byte load through x1 Gen-3 (≈ 985 MB/s raw):

```
Read request   :  12 B   (LANE 0→1)
Read completion:  20 B + 8 B payload = 28 B
Encode 8b/10b  :  (Gen-3 is 128/130, ~1.54% OH)
Total symbols  :  ~40 B transferred
Effective eff. : 8/40 = 20 %
```

## 4. Root-complex write buffers & WC

The CPU coalesces consecutive writes into 32/64-byte WC buffers, flushing them as full-line Memory Writes. In your WC
test you’ll see **128-byte** bursts—the maximum payload for non-AT device on a 64-bit BAR. Reads cannot benefit from
buffering.

## 5. Device latency

After the read TLP arrives the XDMA bridge:

• Decodes the address.
• Issues AXI-Lite read (32 bit) → incurs 27-cycle BRAM latency (you measured).
• Waits for data, forms completion.

This adds another ~220 ns and prevents the bridge from accepting a new read until the first one finishes unless you
enable multiple credits.

## 6. Thunderbolt / PCIe bridges

Each ASMedia bridge adds ~50 ns serialization delay and two additional DLLPs (acknowledgements). That is negligible for
posted writes, but proportionally hurts reads.

## 7. Ordering rules

The PCIe spec forces:

• **Non-posted reads** to be globally visible before any subsequent posted writes from the same requester (RC-ordered).
• CPUs therefore insert an implicit *fence* after every UC load, stalling the store queue behind it.
Your benchmark that alternates load/store shows even lower throughput because writes are forced to drain.

# What can be improved

- Map as **WC** and use `_mm_stream_*` to raise write burst size – already easy.
- Increase **MRRS/MPS** to 512 B/256 B and issue block reads with AVX loads; you get one completion per 512 B → ~250 MiB/s read.
- Allow multiple read credits in the AXI-PCIe bridge (enable *Tag* > 1).
- Add DMA so reads are fetched as long (>1 KiB) streamed completions—achieving 400–600 MiB/s on x1 Gen-3.
- Widen AXI to 128/256 bits, reduce BRAM latency, or cache data on-chip.

Until you get DMA or at least deep outstanding queues, the intrinsic posted/non-posted asymmetry keeps
**CPU-initiated reads 10–100 × slower than writes**.

---

Notice from a human: All of the above was generated with OpenAI o3-pro model. Not sure what is correct and what it
hallucinated so reader beware.