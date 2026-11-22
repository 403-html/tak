# Baseline runner smoke test: 2026-11-22

## Status

This is a runner validation, not a compression benchmark. The 1,437-byte
project README is too small and too close to development data to support a
compression claim. Process startup dominates all timings.

## Environment

- Commit under test: working tree based on `04d5fa0`.
- OS: Linux 6.18.57, x86-64.
- CPU: Intel Xeon Platinum 8573C, three logical CPUs available.
- Threads used by each codec: one.
- Rust: 1.85.0.
- Comparison tools: gzip 1.13, XZ Utils 5.8.1, Zstandard 1.5.7.
- Repetitions: one warm-up and one measured run.

## Complete stream sizes

| Method | Bytes | Round trip |
|---|---:|---|
| TAK raw | 1,465 | Pass |
| TAK order-0 | 961 | Pass |
| TAK order-1 | 1,180 | Pass |
| gzip -9 | 721 | Pass |
| xz -9 | 828 | Pass |
| zstd -19 | 719 | Pass |

The runner also captured wall time and peak resident memory for both operations.
Those measurements are intentionally not summarized here because the input is
too small for useful throughput or memory comparisons.

## Findings

The frame, checksum, decoder limit, external baselines, measurements, and exact
round-trip checks operate end to end. The current TAK models do not beat the
classical tools on this smoke input. Order-1 also performs worse than order-0,
which confirms that more context does not automatically improve total size.

## Next experiment

Run the documented five-repetition method on a versioned held-out corpus with
text, source, JSON, documents, and binary data. Add `enwik9` only after its
source and SHA-256 digest are recorded. Optimize the classical model lookup
before adding a tensor model; the current linear scan over 256 frequencies is
an intentional reference implementation, not a speed target.

