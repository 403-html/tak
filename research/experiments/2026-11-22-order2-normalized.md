# Preliminary gate: finite-strength context prior

## Status

RFC 0006 passes its development size conditions at fixed strength 4. The
decoder-speed condition is pending. This is not a TAK-C method and does not
authorize tensor work.

## Method

The model normalizes an adaptive order-1 parent to a fixed prior mass. For
local count `L`, parent frequency `P`, parent total `P_total`, and strength
`a`, it codes with `F = L * P_total + a * P`.

- Strengths: 1, 4, 16, 64, 256, and 1024, fixed before measurement.
- Inputs and grouping: identical to RFC 0005.
- Accounting: complete 28-byte artifacts; Canterbury files remain separate.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Samples: one non-publishable screen per strength and input.
- Verification: all 96 decodes restored the exact input.

Raw results are in
`benchmarks/reports/raw/2026-11-22-order2-normalized-screen.tsv`. The file
SHA-256 is
`49182c93c5d6603758671bc8ce7f9d518181c199481ee2143b1f347ea1c2736c`.

## Strength screen

| Strength | Order-2 bytes | Normalized bytes | Size reduction | Encode (s) | Decode (s) | Decode slowdown |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 5,093,784 | 4,518,830 | 11.29% | 1.926 | 3.483 | 2.13x |
| 4 | 5,093,784 | 4,395,511 | 13.71% | 1.876 | 3.448 | 2.10x |
| 16 | 5,093,784 | 4,356,646 | 14.47% | 1.888 | 4.115 | 2.51x |
| 64 | 5,093,784 | 4,434,064 | 12.95% | 2.265 | 3.402 | 2.08x |
| 256 | 5,093,784 | 4,633,532 | 9.04% | 2.213 | 3.744 | 2.28x |
| 1024 | 5,093,784 | 4,919,521 | 3.42% | 1.998 | 3.455 | 2.11x |

Strength 16 has the best aggregate size, but the predeclared gate selects a
single strength that also meets per-group and performance conditions. Strength
4 passes the size condition on all eight groups and is the fixed candidate for
speed work.

## Strength-4 groups

| Group | Order-2 bytes | Normalized bytes | Size reduction | Decode slowdown |
|---|---:|---:|---:|---:|
| Silesia English text | 412,203 | 356,669 | 13.5% | 2.04x |
| Silesia source code | 789,852 | 666,314 | 15.6% | 2.39x |
| Silesia structured document | 220,755 | 166,005 | 24.8% | 2.50x |
| Silesia executable | 2,393,950 | 2,170,670 | 9.3% | 2.25x |
| Silesia office data | 355,001 | 254,900 | 28.2% | 1.45x |
| Canterbury prose | 534,817 | 414,943 | 22.4% | 2.28x |
| Canterbury source/markup | 29,568 | 16,629 | 43.8% | 1.42x |
| Canterbury binary | 357,638 | 349,381 | 2.3% | 1.38x |

The size result is broad, but five groups miss the two-times decoder limit in
this screen. The decoder currently scans symbols linearly while combining the
parent and local distributions. Optimize that measured path without changing
strength or artifact bytes, then run controlled repetitions and peak-memory
measurement.

## Distance to project goals

The enwik9 milestones remain unmeasured. Do not extrapolate development data
to enwik9.

On Canterbury, strength 4 produced 780,953 bytes, or 27.78% of input. Same-data
gzip produced 730,625 bytes, so this candidate is 6.9% larger and must remove
6.4% of its current bytes to equal gzip. It is 58.6% larger than xz and must
remove 37.0% of its bytes to equal xz.

Across all development inputs, the screen measured 6.23 MiB/s encoding and
3.39 MiB/s decoding. These are about 8 and 59 times below the aspirational 50
MiB/s and 200 MiB/s targets. Peak memory has not yet been measured for this
candidate.

## Next step

Keep strength 4 fixed. Optimize exact cumulative-symbol lookup. Reject the
model if controlled decoding remains above two times order-2 or memory exceeds
one gigabyte. If it passes, validate on a corpus unused by RFCs 0001 through
0006.
