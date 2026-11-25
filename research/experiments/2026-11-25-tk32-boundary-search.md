# TK32 decode boundary reuse

RFC 0033 collects the active context rows once per literal search and
carries forward both known cumulative endpoints. The search evaluates only
one new endpoint at each step. Each endpoint still uses the RFC 0032
integer formula.

## Calgary

| Metric | RFC 0032 decoder | Boundary reuse |
|---|---:|---:|
| Complete input bytes | 3,251,493 | 3,251,493 |
| Complete artifact bytes | 918,436 | 918,436 |
| Median aggregate decode time (s) | 2.820146 | 2.085782 |
| Aggregate decode throughput (MB/s) | 1.153 | 1.559 |
| Maximum measured decode RSS (KiB) | 44,124 | 44,060 |
| Exact decodes, including warm-up | 108/108 | 108/108 |

The new encoder produced byte-identical artifacts for all 18 files.
The two decoders read the same artifacts. Encode time and encode RSS
come from the RFC 0032 screen. No encoder measurements were taken here.
Median decode time fell by 26.04%. This passes the 20% time reduction gate
and the 2 MiB RSS ceiling for additional memory.

Each file had one warm-up and five timed repetitions per decoder.
Within each repetition the control ran first on even numbers and the
candidate ran first on odd numbers. Sum the file times by repetition,
then take the median. Peak RSS is the maximum across the measured rows.
The decode timing includes process startup and file output. The byte
comparison runs after the timed process.

## enwik9

Both observations use the same 246,642,016-byte TK32 artifact. The
optimized decoder restored all 1,000,000,000 bytes exactly.

| Decoder | Decode and compare (s) | MB/s | Peak RSS (KiB) |
|---|---:|---:|---:|
| RFC 0032, recorded | 451.449 | 2.215 | 113,824 |
| RFC 0033 | 390.689 | 2.560 | 113,952 |

The end-to-end screening time falls by 13.46%. These are single
observations taken in separate runs. Both measure the decode-and-compare
pipeline. RFC 0032's measured encode remains 1.25 MB/s at 238,692 KiB peak
RSS. RFC 0019 still has higher measured decode throughput at 8.94 MB/s.

The retained artifact sizes put Calgary 32,576 bytes above xz.
On enwik9 TAK is 30,967,820 bytes above xz and 49,642,016 above M2.
Matching xz on enwik9 requires a further 12.56% size reduction.

[New enwik9 row](../../benchmarks/reports/raw/2026-11-25-tk32-boundary-enwik9.tsv) ·
[Historical control row](../../benchmarks/reports/raw/2026-11-25-variable-order-enwik9.tsv)

## Reproduction

Use `c2580c0` as the control and build both versions with
`cargo build --release -p tak-tensor --bin tak-lz-stream`.
For each Calgary file, run `tak-lz-stream encode-variable-order INPUT ARTIFACT`
with both encoders and compare their artifacts with `cmp`. Measure both
decoders using `benchmarks/runners/measure.py` and the command
`tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES`.
Compare each restoration with `cmp` before the next trial.

For enwik9, wrap the following command with the same measurement runner:

```console
bash -o pipefail -c 'target/release/tak-lz-stream decode-variable-order enwik9.tk32 - --max-output 1000000000 | cmp enwik9 -'
```

Linux 6.18.57, Intel Xeon Platinum 8573C, x86-64, three visible logical
CPUs, and 10,207,480 KiB visible memory. Rust 1.85.0 release build.
Python 3.12.14 runner. Single-threaded codec with no imposed memory cap.

[Raw Calgary repetitions](../../benchmarks/reports/raw/2026-11-25-tk32-boundary-calgary.tsv)
SHA-256 `617810437ca47972e80495564092858310486199e1bfdcd9f7e318104627d3b6`.
