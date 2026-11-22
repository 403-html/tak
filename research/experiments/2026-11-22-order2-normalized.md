# Accepted development gate: finite-strength context prior

## Status

RFC 0006 passes its complete development gate at fixed strength 4 after an
exact in-memory optimization. This is not a TAK-C method and does not authorize
tensor work. Independent validation is still required.

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

## Decoder optimization

The initial local rows stored only observed symbols. This saved memory but
required binary search and vector insertion during every hot-context update.
A dense 256-entry local row uses the same integer counts and rescaling rule.
It changes no coding probability or artifact byte. Its worst-case frequency
storage is approximately 34 MiB.

Two alternatives were rejected before selecting dense rows:

- A Fenwick-indexed parent plus binary cumulative search increased lookup and
  update overhead. Quick Silesia decoding remained approximately 2.1 to 3.1
  times slower than order-2.
- Jumping between sparse local entries repeated parent-prefix scans and made
  executable decoding substantially slower.

The retained decoder also directly inverts the parent distribution when a
local context has no observations. This is exact because the distribution is
then only the parent multiplied by strength.

The controlled results use one warm-up and five measured runs per file. Group
times sum files for each repetition and report the median. Raw results are in
`benchmarks/reports/raw/2026-11-22-order2-normalized-dense.tsv`; its SHA-256 is
`2a1c282628281a21100fa886ccc12e5e68e2dc9dd4bae80e1ebfd45158a54974`.
The separate memory measurement is in
`benchmarks/reports/raw/2026-11-22-order2-normalized-dense-memory.tsv`.

| Group | Order-2 encode (s) | Normalized encode (s) | Order-2 decode (s) | Normalized decode (s) | Decode slowdown |
|---|---:|---:|---:|---:|---:|
| Silesia English text | 0.091 | 0.102 | 0.105 | 0.190 | 1.81x |
| Silesia source code | 0.172 | 0.202 | 0.210 | 0.383 | 1.82x |
| Silesia structured document | 0.044 | 0.052 | 0.058 | 0.102 | 1.76x |
| Silesia executable | 0.553 | 0.605 | 0.659 | 0.990 | 1.50x |
| Silesia office data | 0.069 | 0.071 | 0.077 | 0.119 | 1.54x |
| Canterbury prose | 0.102 | 0.119 | 0.132 | 0.228 | 1.72x |
| Canterbury source/markup | 0.006 | 0.006 | 0.007 | 0.008 | 1.26x |
| Canterbury binary | 0.078 | 0.083 | 0.087 | 0.126 | 1.44x |
| Aggregate | 1.137 | 1.247 | 1.359 | 2.144 | 1.58x |

Peak complete-process resident memory was 54,040 KiB on the 5,122,048-byte
Mozilla partition. Every controlled decode restored the exact input.

## Distance to project goals

The enwik9 milestones remain unmeasured. Do not extrapolate development data
to enwik9.

On Canterbury, strength 4 produced 780,953 bytes, or 27.78% of input. Same-data
gzip produced 730,625 bytes, so this candidate is 6.9% larger and must remove
6.4% of its current bytes to equal gzip. It is 58.6% larger than xz and must
remove 37.0% of its bytes to equal xz.

Across all development inputs, the optimized model measured 9.38 MiB/s
encoding and 5.46 MiB/s decoding. These are about 5 and 37 times below the
aspirational 50 MiB/s and 200 MiB/s targets. Peak memory is 5.2% of the
one-gigabyte decoder target.

## Next step

Keep strength 4 fixed. Validate on a corpus unused by RFCs 0001 through 0006.
Do not tune the model on that corpus before evaluating the predeclared gate.
