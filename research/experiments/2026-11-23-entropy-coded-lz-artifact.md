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
65,535 bytes. The bounded command path retains neither the complete input nor
the complete restored output.

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

## Bounded encoder screen

The bounded encoder uses four 64 KiB cached input pages. It streams the
arithmetic payload and patches the header after the payload length is known.
It produced byte-identical artifacts in unit tests and for all Calgary files.
The Calgary aggregate remained exactly 1,004,678 bytes with 18 exact round
trips. This single screen used 41,632 KiB maximum encoder RSS.

On Mozilla, bounded encoding produced the same 20,243,862-byte artifact and
artifact SHA-256 as the buffered encoder. It took 24.06 seconds and used 68,768
KiB peak RSS. Bounded decoding took 12.06 seconds and used 101,792 KiB. The
buffered candidate's internal encode time was 7.43 seconds, so bounded encoding
was 3.24 times slower. The buffered gate's 294,192 KiB process peak also
includes RFC 0009 and decoding, so it is not an isolated memory control.

Raw Calgary rows are in
`benchmarks/reports/raw/2026-11-23-lz-bounded-encoder-calgary.tsv`, SHA-256
`822203fb602f5fe975c04c38c0ac8bbdb7b6ed4ef63b4806ec2f019895a76c1d`.
Mozilla rows are in
`benchmarks/reports/raw/2026-11-23-lz-bounded-encoder-mozilla.tsv`, SHA-256
`6bebdf5190e5d8ab92bc4b067f2d6226a829868c2a053c47543fbb6f277e13ea`.

The bounded implementation gate passes for correctness and memory, not speed.
It removes the enwik9 memory blocker. Optimize the page-cache path after the
first enwik9 result establishes whether RFC 0014 has enough ratio value.

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-23 Europe/Warsaw.
- Samples: one measured run per file without a warm-up.
- Source: commit `3e410d9` plus the bounded-encoder working tree.

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

cargo build --release -p tak-tensor --bin tak-lz-stream
target/release/tak-lz-stream encode input.bin output.tklz
target/release/tak-lz-stream decode output.tklz restored.bin \
  --max-output 1000000000
```
