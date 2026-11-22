# Rejected gate: hierarchical order-2 smoothing

## Status

RFC 0005 is rejected. A raw adaptive order-1 parent prior does not improve the
retained order-2 control across domains. No tensor implementation is
authorized.

## Hypothesis and stop condition

For parent weight `w`, parent frequency `P(s)`, and local two-byte-context
increment `L(s)`, the experiment used `F(s) = w * P(s) + L(s)`.

One fixed weight from 1, 2, 4, 8, 16, 32, and 64 had to make complete artifacts
at least 2% smaller than order-2 on five of eight development groups. At least
two passing groups had to come from Canterbury. Aggregate size could not
increase. Decoding had to remain within two times order-2, peak resident memory
had to stay below one gigabyte, and every artifact had to restore exactly.

## Method

- Dataset: the five final-10% Silesia partitions from RFC 0003 and all eleven
  Canterbury files grouped as in RFC 0004.
- Data role: development. Canterbury is no longer held out after RFC 0004.
- Accounting: complete 28-byte `TKO2` and `TKS2` artifacts. Every Canterbury
  file is a separate artifact.
- Arithmetic: deterministic integer model operations and the same arithmetic
  coder for both methods.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Samples: one non-publishable screening run per weight and input.
- Verification: every decoder output was compared byte for byte.

The 112 per-file weight results are stored in
`benchmarks/reports/raw/2026-11-22-order2-smoothing-screen.tsv`. Its SHA-256 is
`61d8988f7a754b8f6e68f9567b0f6ef6d312b5d97613ee2812e3f6adae2ad975`.
The separate memory measurement is stored in
`benchmarks/reports/raw/2026-11-22-order2-smoothing-memory.tsv`.

## Group results for the best aggregate weight

Negative reduction means that smoothing made the artifact larger.

| Group | Input bytes | Order-2 bytes | Smoothed bytes | Size reduction | Decode slowdown |
|---|---:|---:|---:|---:|---:|
| Silesia English text | 1,019,245 | 412,203 | 425,438 | -3.2% | 2.26x |
| Silesia source code | 2,160,640 | 789,852 | 865,273 | -9.5% | 2.14x |
| Silesia structured document | 534,528 | 220,755 | 232,869 | -5.5% | 2.29x |
| Silesia executable | 5,122,048 | 2,393,950 | 2,554,339 | -6.7% | 2.20x |
| Silesia office data | 615,220 | 355,001 | 360,474 | -1.5% | 1.94x |
| Canterbury prose | 1,185,883 | 534,817 | 512,358 | 4.2% | 2.27x |
| Canterbury source/markup | 43,701 | 29,568 | 24,450 | 17.3% | 1.69x |
| Canterbury binary | 1,581,200 | 357,638 | 368,298 | -3.0% | 1.63x |

Weight 1 passed the 2% size condition on only two of eight groups. All seven
weights lost on every Silesia group, so no weight could reach five passing
groups.

## Aggregate weight screen

| Parent weight | Order-2 bytes | Smoothed bytes | Size reduction | Encode (s) | Decode (s) | Decode slowdown |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 5,093,784 | 5,343,499 | -4.90% | 1.858 | 3.116 | 2.15x |
| 2 | 5,093,784 | 5,475,598 | -7.50% | 1.719 | 3.173 | 2.19x |
| 4 | 5,093,784 | 5,570,914 | -9.37% | 1.798 | 3.141 | 2.17x |
| 8 | 5,093,784 | 5,633,426 | -10.59% | 1.809 | 3.114 | 2.15x |
| 16 | 5,093,784 | 5,671,238 | -11.34% | 1.830 | 3.099 | 2.14x |
| 32 | 5,093,784 | 5,692,695 | -11.76% | 1.870 | 3.089 | 2.13x |
| 64 | 5,093,784 | 5,704,327 | -11.99% | 1.729 | 2.933 | 2.03x |

The aggregate input is 12,262,465 bytes. Weight 1 encoded at 6.29 MiB/s and
decoded at 3.75 MiB/s. A separate complete-process measurement on the largest
input used 46,708 KiB peak resident memory. All 112 decodes restored the exact
input.

These single-run times are screening data. The failed size gate made repeated
timing unnecessary.

## Interpretation

The parent contribution used raw accumulated adaptive counts. The local row
started at zero and added one per context observation. Parent counts therefore
swamped local evidence even at weight 1. Increasing the parent weight made
aggregate size monotonically worse.

The Canterbury text and source improvements show that shorter-context sharing
can help cold contexts. The binary and Silesia losses show that this raw-count
blend cannot adapt local influence reliably across domains.

Do not test more weights for this formula. A normalized parent distribution
with a fixed prior mass is a different mechanism and needs a new RFC. It must
retain the order-2 control and use a later untouched validation corpus if it
passes development screening.

## Distance to project goals

The enwik9 milestones remain unmeasured. Do not extrapolate these development
results to enwik9.

On Canterbury alone, weight 1 produced 905,106 bytes, or 32.20% of input. This
numerically matches the M1 enwik9 ratio, but same-data gzip produced 730,625
bytes, or 25.99%. The smoothed artifact is 23.9% larger than gzip and must
remove 19.3% of its current bytes to equal gzip on Canterbury. It is 83.8%
larger than xz and must remove 45.6% of its bytes to equal xz.

Across all development inputs, weight 1 reached 6.29 MiB/s encoding and 3.75
MiB/s decoding. Those rates are about 8 times and 53 times below the
aspirational 50 MiB/s and 200 MiB/s targets. Peak memory is already well below
the one-gigabyte target.

## Decision

Reject RFC 0005. Preserve the implementation and artifact as a reproducible
negative result, but do not promote it to TAK-C.

The next justified classical experiment is a finite-strength normalized prior,
not another raw-count weight and not a tensor rank increase.

## Reproduction

```console
cargo run --release -p tak-tensor --bin tak-order2-smoothing-gate -- input.bin
```
