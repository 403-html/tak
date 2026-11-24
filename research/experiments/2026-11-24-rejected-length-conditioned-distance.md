# Rejected length-conditioned distance buckets

## Decision

Reject RFC 0025. Selecting distance-bucket distributions from the current
match length increases ideal cost on every Silesia development tail. Do not
tune length thresholds or add previous-length contexts on these files.

Calgary, a distinct artifact, and enwik9 were not run.

## Frozen method

The control uses RFC 0019's one adaptive 32-symbol distance-bucket
distribution. The candidate uses four independent distributions selected by
`min(match_length - 8, 3)`. The classes therefore contain lengths 8, 9, 10,
and 11 or more.

All distributions start with unit frequencies and use the existing update
and halving rules. The RFC 0019 parse, distance buckets, and raw remainder bits
are unchanged. The decoder would derive the class from the current match
length, so the opportunity model transmits no selector or fitted parameter.

## Results

| File | Input bytes | Matches | Control bits | Candidate bits | Saved bits | Saved bits/input byte |
|---|---:|---:|---:|---:|---:|---:|
| dickens | 1,019,245 | 35,605 | 613,835.005 | 614,323.607 | -488.602 | -0.000479 |
| mozilla | 5,122,048 | 131,331 | 1,593,626.360 | 1,596,289.745 | -2,663.384 | -0.000520 |
| ooffice | 615,220 | 11,490 | 128,818.389 | 128,894.342 | -75.952 | -0.000123 |
| samba | 2,160,640 | 63,192 | 882,632.055 | 883,029.144 | -397.089 | -0.000184 |
| xml | 534,528 | 18,125 | 254,202.717 | 254,402.349 | -199.632 | -0.000373 |
| **Total** | **9,451,681** | **259,743** | **3,473,114.527** | **3,476,939.186** | **-3,824.659** | **-0.000405** |

Every file fails the frozen requirement. The candidate adds 3,824.659 ideal
bits in aggregate. The small length-8, length-9, and length-10 tables fragment
adaptive counts without enough distribution shift to compensate.

The raw timing values are single-run screening measurements. They are not
publishable performance results.

Raw rows, including exact input digests, are in
`benchmarks/reports/raw/2026-11-24-length-conditioned-distance.tsv`, SHA-256
`4cdc2a0aacd02bb079b734aa10a60ca6b63c0aee9c15898de3aa400bd05b0fe1`.

## Tensor scope

This result does not authorize a tensor. Four direct 32-symbol tables already
represent the tested dependency and lose to the global control. Parameter
sharing would need a different pre-registered dependency and evidence; merely
factorizing these failed tables is not justified.

## Distance to goals

RFC 0019 remains the best measured TAK enwik9 artifact at 249,762,881 bytes.
It remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz still requires a 13.65% reduction.

RFC 0025 has no artifact or enwik9 result. Do not extrapolate ideal costs.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `c44073d` plus the RFC 0025 working tree.
- Command: `tak-length-distance INPUT --tail-bytes N`.
- Repetitions: one screening run per file.
