# Literal-only order-2 counts

RFC 0029 passed the Silesia development gate and failed the Calgary transfer
gate. The fixed RFC 0019 parse supplied the same literal and match events
to both models. The candidate learned probability counts on literals and
maintained both histories and the exact-suffix table on all bytes.

| Corpus | Input bytes | Improved files | Ideal bits saved | Saved bits/input byte |
|---|---:|---:|---:|---:|
| Silesia development tails | 9,451,681 | 5/5 | 300,222.870 | 0.031764 |
| Calgary validation | 3,251,493 | 10/18 | 26,866.742 | 0.008263 |

Both aggregates exceeded the 0.008 threshold. Calgary required at least
12 improvements. Six paper files, progc, and progp regressed. Mozilla
accounted for 74% of the development savings. The diagnostic isolates one
update rule and shows its sensitivity to corpus composition.

This stage measured integer-probability log loss. It produced no new artifact
or decoder, so compression throughput, restoration, and enwik9 would require
an artifact experiment. The measured Calgary leaders are RFC 0019 at
942,454 bytes and xz at 885,860 bytes. TAK needs to remove 56,595 bytes to
beat xz. On enwik9, RFC 0019 is 249,762,881 bytes, 34,088,685 above xz and
52,762,881 above the nominal 197,000,000-byte M2 threshold.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-literal-phase-order2
python3 benchmarks/runners/measure.py --timing timing.tsv --stdout row.tsv \
  target/release/tak-literal-phase-order2 --literal-only-counts INPUT
```

Append `--tail-bytes N` for Silesia with the selected length from the
raw rows. Run each Calgary file independently. The raw rows include input
hashes, per-phase costs, wall time, and peak RSS.

- Source: `cc29d0c` plus the RFC 0029 diagnostic.
- Environment: Linux 6.18.57, x86-64 Xeon Platinum 8573C, three visible CPUs,
  10,207,480 KiB reported memory, Rust 1.85.0 release build, Python 3.12.14.
- One thread, no experiment-specific memory cap, one screening run per file.
- Maximum process RSS: 170,784 KiB on Silesia and 87,904 KiB on Calgary.
- Raw SHA-256: `e753a9cc157fdedb67368071419d43a44cdbd691e3249081b07990472d91f76d`.
  [Raw rows](../../benchmarks/reports/raw/2026-11-24-literal-only-counts.tsv)
