# Accepted gate: match reliability state

## Status

RFC 0012 passes both classical dependency gates. This permits the new tensor
design in RFC 0013. It does not demonstrate that the tensor will compress
better than the classical controls.

These are single-sample development diagnostics without a warm-up. They are
not publishable performance measurements.

## Dataset and environment

- Inputs: the exact final-10% Silesia partitions from RFC 0008.
- Total selected bytes: 9,451,681.
- Input digests: `benchmarks/reports/raw/2026-11-23-match-dependency-inputs.tsv`.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-23 Europe/Warsaw.
- Samples: one measured run per file without a warm-up.
- Source: commit `97ff314` plus the RFC 0012 working tree.

## Results

| Group | Matched events | Global bpe | Distance/run bpe | Reduction | Suffix bpe | Suffix reduction |
|---|---:|---:|---:|---:|---:|---:|
| Dickens | 489,162 | 0.9812 | 0.9733 | 0.0080 | 0.7788 | 0.1945 |
| Samba | 1,595,254 | 0.5838 | 0.5370 | 0.0468 | 0.5007 | 0.0363 |
| XML | 377,036 | 0.6811 | 0.6552 | 0.0259 | 0.5562 | 0.0990 |
| Mozilla | 2,661,222 | 0.5834 | 0.5279 | 0.0555 | 0.4356 | 0.0923 |
| OOffice | 288,257 | 0.5136 | 0.4658 | 0.0478 | 0.5053 | -0.0395 |

Distance/run state passes on four groups. Exact suffix identity passes on four
groups and regresses on OOffice. Weighted across 5,410,931 events, the suffix
control improves from 0.5764 to 0.4979 bits per event.

Raw Phase A results are in
`benchmarks/reports/raw/2026-11-23-match-reliability-silesia.tsv`, SHA-256
`1aadc2741b4715d9e9be6e3e7e12ccd774a1bfe62ec92e5c1fda6821c430da09`.
Suffix residual results are in
`benchmarks/reports/raw/2026-11-23-match-reliability-suffix.tsv`, SHA-256
`e5aba3d65cf977b73209e090afba287fc7a0f9855365fee8ca136af6daa7fce5`.

Peak process RSS was 75,664 KiB. These are ideal binary log-loss diagnostics,
not compressed artifacts.

## Decision

Proceed only with RFC 0013's match-state tensor. Do not revive the rejected
bigram tensor or increase its rank. The new tensor must beat the 24-cell and
sparse suffix controls after core metadata and integer contraction are counted.

The current measured enwik9 control remains 315,578,766 bytes. M1 is achieved;
M2 remains 118,578,766 bytes away and requires a 37.57% reduction.

## Reproduction

Build the diagnostic with:

```console
cargo build --release -p tak-tensor --bin tak-match-reliability
```

Measure each input with the established runner. For example:

```console
python3 benchmarks/runners/measure.py \
  --timing /tmp/match-reliability-dickens-memory.tsv \
  --stdout /tmp/match-reliability-dickens.tsv \
  target/release/tak-match-reliability \
  /tmp/silesia-screen/dickens --tail-bytes 1019245
```

Replace the input and partition size with the values in the raw tables. The
Phase A table came from the distance/run-only revision. The suffix table came
from the final diagnostic, which reports both controls in one run.
