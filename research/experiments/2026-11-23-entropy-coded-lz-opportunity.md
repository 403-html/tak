# Accepted diagnostic: entropy-coded LZ blocks

## Decision

Accept RFC 0014's ideal opportunity gate and permit one complete research
artifact. The fixed parser passed four of five Silesia development groups and
remained positive in Calgary aggregate. The Calgary opportunity is modest and
does not close the measured gap to xz.

## Method

- Match key: exact 8-byte sequence.
- Match table: 1,048,576 direct-mapped entries.
- Window: 67,108,864 bytes.
- Maximum match: 65,535 bytes.
- Parser: greedy, one candidate, minimum match length 8.
- Blocks: literal, match, and end.
- Metadata: adaptive ideal type and logarithmic-bucket distributions.
- Literal model: RFC 0009, updated with every reconstructed byte.
- Samples: one screening run without a warm-up. Timings are not publishable.
- Source: commit `69b45f2` plus the RFC 0014 working tree.

## Development results

| Group | Match coverage | Saved bits/byte | Decision |
|---|---:|---:|---|
| Dickens | 77.91% | -0.0754 | Regress |
| Samba | 89.12% | 0.4261 | Pass |
| XML | 87.92% | 0.2444 | Pass |
| Mozilla | 68.56% | 0.2871 | Pass |
| OOffice | 59.66% | 0.2614 | Pass |
| Aggregate | 74.79% | 0.2757 | Pass |

Aggregate ideal savings were 2,605,801 bits, approximately 325,725 bytes.
This excludes integer entropy-coder rounding and a physical stream header.

## Calgary validation

The frozen parser saved 190,882 ideal bits across 3,251,493 bytes, or 0.0587
bits per byte. Match blocks covered 70.95% of the corpus. Book1 regressed by
0.0956 bits per byte and `pic` by 0.0823. The other files were positive, but
`geo` improved by only 0.0027 bits per byte.

The ideal Calgary saving is approximately 23,860 bytes. RFC 0009 remains
143,620 bytes larger than xz, so this mechanism alone cannot close that gap
under the measured accounting. A complete artifact must establish whether the
small ideal margin survives deterministic integer coding.

## Raw results and environment

Raw development results are in
`benchmarks/reports/raw/2026-11-23-lz-opportunity-silesia.tsv`, SHA-256
`ccb567498d325b2329c975297878e0f94c441f538eec7c9abc1a0589634d7387`.
Raw Calgary results are in
`benchmarks/reports/raw/2026-11-23-lz-opportunity-calgary.tsv`, SHA-256
`b10b347083163322901bdf61081b840c297c41b283490e296864e674f51b224e`.

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-23 Europe/Warsaw.

Peak process RSS was 108,428 KiB on the development inputs and 25,504 KiB on
Calgary. Wall time was below 0.85 seconds per development file and below 0.18
seconds per Calgary file.

## Distance to project goals

This diagnostic does not change the enwik9 artifact. TAK remains at
315,578,766 bytes. M1 remains achieved by 6,421,234 bytes. M2 remains
118,578,766 bytes away and requires a 37.57% reduction. The measured gaps to
Zstandard-19 and xz remain 80,249,827 and 99,904,570 bytes.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-opportunity
python3 benchmarks/runners/measure.py \
  --timing /tmp/lz-samba-memory.tsv \
  --stdout /tmp/lz-samba.tsv \
  target/release/tak-lz-opportunity \
  /tmp/silesia-screen/samba --tail-bytes 2160640
```
