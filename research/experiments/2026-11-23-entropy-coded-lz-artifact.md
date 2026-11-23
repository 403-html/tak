# Accepted development artifact: entropy-coded LZ blocks

## Decision

Accept the RFC 0014 complete-artifact development gate. The fixed `TKLZ`
artifact is 2.41% smaller than RFC 0009 in Calgary aggregate, restores every
file exactly, decodes faster in the screening run, and stays far below the
one-gigabyte memory limit.

Retain it as a research component. Do not promote it as TAK's best-ratio
profile: RFC 0010 remains 3,983 bytes smaller on the same corpus.

## Results

| Metric | RFC 0009 | RFC 0014 | Change |
|---|---:|---:|---:|
| Calgary complete bytes | 1,029,480 | 1,004,678 | -24,802 (-2.41%) |
| Aggregate encode time | 0.674 s | 0.509 s | 0.76x |
| Aggregate decode time | 0.759 s | 0.425 s | 0.56x |
| Exact round trips | 18/18 | 18/18 | — |

Book1 grew by 8,529 bytes and `pic` grew by 5,281 bytes. The fixed aggregate
gate passes despite those regressions. Peak process RSS was 36,756 KiB.

These are single-sample screening times without a warm-up. They are not
publishable performance results. Artifact sizes and round trips are exact.

Controlled timing then used one warm-up and five measured repetitions over all
18 files. Median aggregate encode time was 0.423 seconds for RFC 0014 and 0.570
seconds for RFC 0009, a 0.74x ratio. Median aggregate decode time was 0.411
seconds versus 0.767, a 0.54x ratio. Every repetition produced identical sizes
and exact round trips. Peak RSS ranged from 36,760 to 36,800 KiB in measured
runs.

## Competitive position

On the same independent-file Calgary accounting, `TKLZ` is:

- 54,657 bytes smaller than gzip;
- 83,605 bytes larger than Zstandard level 19;
- 118,818 bytes larger than xz; and
- 3,983 bytes larger than retained RFC 0010.

The implementation confirms that the ideal diagnostic's small opportunity
survives integer coding. It does not reach the prior objective of beating
Zstandard-19 or xz on Calgary.

## Format and safety

The research artifact uses magic `TKLZ`, a 28-byte little-endian header, one
arithmetic-coded payload, an explicit end block, and CRC-32 over the restored
bytes. The decoder rejects invalid versions, extents, arithmetic intervals,
lengths, distances, premature end blocks, output-limit violations, checksum
failures, and truncated artifacts.

The match window is limited to 67,108,864 bytes. Match length is limited to
65,535 bytes. This implementation retains the complete output in memory and
is not yet the bounded streaming enwik9 path.

## Raw results and environment

Raw rows are in
`benchmarks/reports/raw/2026-11-23-lz-artifact-calgary.tsv`, SHA-256
`427ff03a0898f66c526e1b97aaffc91e0cef9ab7d969197a60e39f2cbf7bd438`.
Controlled aggregate repetitions are in
`benchmarks/reports/raw/2026-11-23-lz-artifact-controlled.tsv`, SHA-256
`842e62beb130ac3afb29fa0cdd8d3fa0eb8d071dd6a014be658c05d9fc0a3d2c`.

## Bounded decoder screen

The bounded decoder retains a 64 MiB match window and 64 KiB input and output
buffers instead of the complete output. On the full 51,220,480-byte Mozilla
file it restored the exact input SHA-256, used 101,788 KiB peak RSS, and took
13.59 seconds. The `TKLZ` artifact was 20,243,862 bytes.

Raw data and digests are in
`benchmarks/reports/raw/2026-11-23-lz-streaming-decoder.tsv`, SHA-256
`62ac91cc403212817b887e2234b92d75721f4567e72f1dbc8d291be8e8397c00`.

This passes the bounded decoder gate. It does not authorize enwik9 yet because
the encoder still retains the complete input, parsed block list, and payload.

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-23 Europe/Warsaw.
- Samples: one measured run per file without a warm-up.
- Source: commit `fdc929d` plus the RFC 0014 artifact working tree.

## Distance to project goals

The artifact has not been measured on enwik9. The last measured enwik9 result
remains RFC 0009 at 315,578,766 bytes. M1 remains achieved by 6,421,234 bytes.
M2 remains 118,578,766 bytes away and requires a 37.57% reduction. Do not
extrapolate Calgary's 2.41% artifact reduction to enwik9.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-gate
python3 benchmarks/runners/measure.py \
  --timing /tmp/lz-book2-memory.tsv \
  --stdout /tmp/lz-book2.tsv \
  target/release/tak-lz-gate /tmp/tak-calgary/book2
```
