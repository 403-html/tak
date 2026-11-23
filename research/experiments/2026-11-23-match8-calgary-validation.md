# Accepted validation: bounded exact match

## Status

The unchanged RFC 0009 model passes Calgary validation. This validates the
classical exact-match mechanism on a held-out corpus. `TKM8` remains a research
artifact and is not a TAK-C method.

## Fixed model and gate

The model retains its 8-byte suffix, divisor-4 bonus, 1,048,576 direct-mapped
slots, multiplicative hash, and collision replacement. Calgary did not select
or tune these values.

Validation required at least 2% complete-size improvement on two of three
groups, no aggregate regression, no group above two times baseline decoding,
less than one gigabyte peak RSS, and exact restoration.

## Dataset and environment

- Corpus: Calgary, 3,251,493 bytes in 18 independently compressed files.
- Groups: the existing prose/reference, technical/source, and binary groups.
- Input digests: `benchmarks/reports/raw/2026-11-22-calgary-inputs.tsv`.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Date: 2026-11-23 Europe/Warsaw.
- Source: commit `9116e2a` plus the validation working tree.

The size screen is in
`benchmarks/reports/raw/2026-11-23-match8-calgary-screen.tsv`, SHA-256
`4f2978100d36d8da837575e20d07cc5c3fe1d554e783fc489a0013a3d4be1f5a`.
All timing rows are in
`benchmarks/reports/raw/2026-11-23-match8-calgary-controlled.tsv`, SHA-256
`4986dec9e0b36b61dfd28d14a9d60a22452ad96a9982a3159614d58d169485c2`.

## Results

| Group | Input bytes | TKN2 bytes | TKM8 bytes | Reduction | TKN2 decode | TKM8 decode | Ratio |
|---|---:|---:|---:|---:|---:|---:|---:|
| Prose/reference | 1,867,997 | 698,535 | 646,504 | 7.45% | 0.376 s | 0.471 s | 1.25x |
| Technical/source | 499,562 | 177,441 | 162,584 | 8.37% | 0.103 s | 0.191 s | 1.86x |
| Binary | 883,934 | 231,409 | 220,392 | 4.76% | 0.123 s | 0.167 s | 1.36x |
| Total | 3,251,493 | 1,107,385 | 1,029,480 | 7.04% | — | — | — |

Times are medians after one warm-up and five measured repetitions per file,
with file times summed by group for each repetition. All 90 measured decodes
restored their exact inputs. The largest measured process, on `book1`, used
21,720 KiB peak RSS.

For context, the previous matching-accounting Calgary streams were gzip
1,059,335 bytes, xz 885,860 bytes, and Zstandard level 19 at 921,073 bytes.
`TKM8` is now 29,855 bytes or 2.8% smaller than gzip. It remains 16.2% larger
than xz and 11.8% larger than Zstandard. It must remove 13.9% and 10.5% of its
current bytes to equal them, respectively.

## Distance to project goals

This result does not change enwik9. The measured TAK artifact remains
368,643,002 bytes, 46,643,002 bytes above M1. It must remove 12.65% to reach M1
and 46.56% to reach M2. Do not transfer the 7.04% Calgary gain to enwik9.

## Decision

Accept validation. The next engineering requirement is a byte-identical
streaming `TKM8` path and an enwik9 measurement with bounded memory. Do not run
the current all-memory artifact path on enwik9. Tensor work still requires a
new RFC with a match-state representation and comparison against this control.
