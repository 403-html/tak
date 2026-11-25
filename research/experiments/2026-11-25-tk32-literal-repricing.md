# TK32 literal backoff prices

RFC 0034 replaces Match8 literal prices on the subset where TK32 would
use longer context rows. The encoder retains the existing parse search
and writes complete TK32 streams for the same decoder.

| Corpus | Control bytes | Repriced bytes | Saved bytes | Smaller files |
|---|---:|---:|---:|---:|
| Calgary | 918,436 | 906,449 | 11,987 (1.31%) | 17/18 |
| Independent Canterbury | 597,600 | 590,818 | 6,782 (1.13%) | 9/10 |

Every file restored byte for byte, including the duplicate Canterbury
ptt5, which is excluded from the independent totals. Calgary obj1 grows
by 21 bytes. Canterbury sum grows by 7 bytes. Calgary is 20,589 bytes
above xz (885,860) and 14,624 bytes below Zstandard-19 (921,073).
It takes 20,590 bytes, or 2.27% of the candidate total, to beat xz.

| Corpus and profile | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encode KiB | Peak decode KiB |
|---|---:|---:|---:|---:|---:|---:|
| Calgary TK32 | 3.961 | 2.107 | 0.821 | 1.543 | 95,380 | 44,064 |
| Calgary repriced | 5.619 | 2.311 | 0.579 | 1.407 | 113,432 | 43,932 |
| Canterbury TK32 | 2.473 | 1.317 | 0.929 | 1.745 | 87,120 | 38,488 |
| Canterbury repriced | 3.208 | 1.285 | 0.716 | 1.788 | 105,724 | 38,436 |

The parser carries its own copy of the longer-context rows. This
increases encode time and peak memory. Timings sum the per-file runs
and include process startup and file output. Each row is a one-run
screen. The byte comparison occurs outside the measurement.

## enwik9

| Profile | Bytes | Encode s | Decode and compare s | Encode MB/s | Decode and compare MB/s | Peak encode KiB | Peak decode and compare KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| TK32, recorded RFC 0032 encode and RFC 0033 decode | 246,642,016 | 798.17 | 390.69 | 1.25 | 2.56 | 238,692 | 113,952 |
| TK32, RFC 0034 parse | 243,396,020 | 1,315.45 | 413.07 | 0.76 | 2.42 | 256,844 | 113,952 |

The complete stream saves 3,245,996 bytes (1.32%). Streaming comparison
verified all 1,000,000,000 original bytes. The gap is 8,067,081 bytes to
Zstandard-19, 27,721,824 to xz, and 46,396,020 to nominal M2.
Matching xz requires a further 11.39% reduction.

These are separate single-run screens on the same input. Workspace
checks ran during the early part of the new encode. The timing includes
this overlap. The Calgary and Canterbury runs measured the same binaries
back to back and provide the direct comparison of encoding cost.
[Raw enwik9 row](../../benchmarks/reports/raw/2026-11-25-tk32-literal-repricing-enwik9.tsv)

## Reproduction

Build `cargo build --release -p tak-tensor --bin tak-lz-stream`.
Encode each input with `tak-lz-stream encode-variable-order INPUT ARTIFACT`
and `tak-lz-stream encode-variable-order-repriced INPUT ARTIFACT`.
Use `tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES`
for either artifact. Measure each operation with
`benchmarks/runners/measure.py` and verify output with `cmp`.
The raw rows record input hashes and full artifact sizes.

For the enwik9 decode screen, time the whole pipeline with the same
measurement runner:

```console
bash -o pipefail -c 'target/release/tak-lz-stream decode-variable-order enwik9.tk32 - --max-output 1000000000 | cmp enwik9 -'
```

Source is `092356c` plus RFC 0034. Linux 6.18.57, Intel Xeon Platinum
8573C, x86-64, three visible logical CPUs, and 10,207,480 KiB visible RAM.
Rust 1.85.0 release build, Python 3.12.14 runner, one codec thread,
and no imposed experiment-specific memory cap.

[Raw Calgary and Canterbury rows](../../benchmarks/reports/raw/2026-11-25-tk32-literal-repricing.tsv)
SHA-256 `f841848402a015dee8dd6afb751ada33da23a031e384dcec0d50f2f8f66565d2`.
