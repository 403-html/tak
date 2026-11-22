# Rejected experiment: Fenwick cumulative-frequency index

## Status

Rejected on 2026-11-22. The implementation was not merged.

## Hypothesis

Replacing each linear scan of 256 symbol frequencies with a Fenwick tree would
reduce arithmetic-model lookup from O(256) to O(log 256) and improve both
compression and decompression throughput.

## Change

Each adaptive distribution retained its 256 16-bit frequencies and added a
257-entry 16-bit Fenwick tree. Model rescaling rebuilt the tree. The arithmetic
coder, update thresholds, stream frame, and compiler settings did not change.

## Method

- Base commit: `e9a92cf64bdd65891b9b7d7d208bc960d393c2c6`.
- Input: `enwik8`, 100,000,000 bytes.
- Input SHA-256:
  `2b49720ec4d78c3c9fabaee6e4179a5e997302b3a70029f30f2d582218c024a8`.
- CPU: Intel Xeon Platinum 8573C, x86-64.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Samples: one before and one after measurement, following untimed builds and
  compatibility runs.
- Time: wall-clock seconds measured by `benchmarks/runners/measure.py`.
- Memory: peak resident set size in KiB reported by the same runner.

One sample is sufficient to reject this change because the primary decoder
metric regressed. It is not sufficient to claim small speed improvements.

## Results

Complete frame sizes were identical before and after the change.

| Model | Operation | Before (s) | Fenwick (s) | Change | Before RSS (KiB) | Fenwick RSS (KiB) |
|---|---|---:|---:|---:|---:|---:|
| order-0 | compress | 10.0567 | 10.1312 | 0.7% slower | 221,248 | 221,336 |
| order-0 | decompress | 11.0224 | 10.9899 | 0.3% faster | 159,936 | 159,776 |
| order-1 | compress | 9.0482 | 8.7758 | 3.0% faster | 193,472 | 193,416 |
| order-1 | decompress | 11.0547 | 11.7712 | 6.5% slower | 146,112 | 146,208 |

- Order-0 frame: 62,760,459 bytes.
- Order-1 frame: 48,480,097 bytes.
- Every decoded output matched the input byte for byte.
- The modified encoder produced byte-identical streams for both models.
- The modified decoder decoded both streams produced by the base commit.

## Decision

Reject the Fenwick index. It increases model state, does not help order-0, and
makes order-1 decompression materially slower. The likely causes are the small
fixed alphabet, optimized contiguous scans, and less cache-friendly indexed
access.

Do not retry this data structure without a profile that identifies cumulative
frequency lookup as a dominant cost on a different model or alphabet.

