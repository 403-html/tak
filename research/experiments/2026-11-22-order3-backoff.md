# Accepted development gate: mature third-order context backoff

## Status

RFC 0004 passes its classical development gate after an exact sparse-decoder
optimization. This result does not authorize tensor work or a TAK-C method.

## Hypothesis and stop condition

Rare order-3 contexts might cause the RFC 0003 loss. A hard-backoff model uses
order-3 only after an exact three-byte context reaches a fixed observation
threshold. Otherwise, it uses order-2. Both models update after every byte.

Test thresholds 2, 4, 8, 16, 32, and 64. One fixed threshold must make complete
artifacts at least 2% smaller on three groups, decode no more than two times
slower than order-2, remain below one gigabyte, and restore every input exactly.

## Method

- Base commit: `36eb29a`, plus the RFC 0004 implementation.
- Dataset: the same final-10% Silesia partitions and SHA-256 digests recorded
  by RFC 0003.
- Accounting: complete `TKO2` and `TKB3` artifacts, including 28-byte headers.
- Arithmetic coder: identical for both controls.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Threshold screen: one non-publishable run for every threshold and input.
- Selected threshold timing: one warm-up and five measured repetitions.
- Time statistic: median wall-clock time.
- Memory statistic: maximum complete-process resident memory across the five
  measured repetitions.
- Verification: exact byte-for-byte restoration for every artifact.

The five raw controlled repetitions are stored in
`benchmarks/reports/raw/2026-11-22-order3-backoff-threshold64.tsv`.
The optimized repetitions are stored in
`benchmarks/reports/raw/2026-11-22-order3-backoff-gap-decoder.tsv`.

The Silesia inputs are development data. Threshold 64 requires later validation
on a separate held-out corpus before any general compression claim.

## Threshold screen

Artifact size does not depend on the in-memory context index. The final
deterministic hash implementation produced these complete sizes:

| Threshold | dickens | samba | xml | mozilla | ooffice |
|---:|---:|---:|---:|---:|---:|
| 2 | 457,395 | 823,537 | 235,405 | 2,500,626 | 377,940 |
| 4 | 453,518 | 813,981 | 232,465 | 2,457,368 | 372,003 |
| 8 | 447,648 | 801,454 | 228,657 | 2,418,833 | 366,935 |
| 16 | 439,473 | 787,027 | 224,208 | 2,382,861 | 361,835 |
| 32 | 429,371 | 772,419 | 219,564 | 2,352,863 | 357,921 |
| 64 | 418,727 | 760,797 | 215,818 | 2,333,655 | 355,611 |
| Order-2 | 412,203 | 789,852 | 220,755 | 2,393,950 | 355,001 |

Threshold 64 is the only tested fixed threshold that beats order-2 by at least
2% on three groups. Do not infer that a larger threshold is better. Values
above 64 were outside the predefined screen.

## Pre-optimization threshold-64 results

| Input | Order-2 bytes | Backoff bytes | Size reduction | Order-2 encode (s) | Backoff encode (s) | Order-2 decode (s) | Backoff decode (s) | Decode slowdown | Backoff encode (MiB/s) | Backoff decode (MiB/s) | Peak RSS (KiB) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| dickens | 412,203 | 418,727 | -1.6% | 0.090 | 0.159 | 0.116 | 0.294 | 2.54x | 6.10 | 3.31 | 8,448 |
| samba | 789,852 | 760,797 | 3.7% | 0.181 | 0.324 | 0.231 | 0.521 | 2.25x | 6.36 | 3.95 | 19,256 |
| xml | 220,755 | 215,818 | 2.2% | 0.049 | 0.082 | 0.061 | 0.135 | 2.23x | 6.20 | 3.77 | 8,448 |
| mozilla | 2,393,950 | 2,333,655 | 2.5% | 0.550 | 1.267 | 0.684 | 1.481 | 2.17x | 3.86 | 3.30 | 130,868 |
| ooffice | 355,001 | 355,611 | -0.2% | 0.077 | 0.142 | 0.085 | 0.172 | 2.02x | 4.13 | 3.41 | 25,308 |

Negative reduction means the backoff artifact is larger. All measured runs
restored the exact input.

## Performance ablations

The first implementation used the standard keyed hash and asked the entropy
model for its total twice per byte. Removing the duplicate query and using a
deterministic one-to-one hash for 32-bit context keys reduced decoding time
without changing artifact bytes.

A two-level direct table was then tested as a speed-memory tradeoff. It lazily
allocated 32 KiB pointer pages for the 24-bit context space. The table was
dominated: its median threshold-64 decoder slowdowns were 2.40x to 2.81x on
all five inputs, and peak process memory reached 231,852 KiB. Sparse binary
data touched many pages, so page allocation and memory traffic outweighed the
saved hashing. The code was reverted. This negative ablation is retained here
rather than in the artifact format.

Code-path inspection then found that sparse decoding still scanned all 256
symbols. An input profile confirmed that 76.1% to 95.5% of observed contexts
had at most four distinct continuations. Only 1,318 to 7,618 contexts per input
reached threshold 64.

| Input | Observed contexts | Threshold-64 contexts | One continuation | At most four continuations |
|---|---:|---:|---:|---:|
| dickens | 10,965 | 2,281 | 4,677 | 8,345 (76.1%) |
| samba | 44,391 | 4,259 | 26,693 | 38,947 (87.7%) |
| xml | 14,952 | 1,499 | 9,043 | 13,343 (89.2%) |
| mozilla | 574,890 | 7,618 | 464,528 | 549,031 (95.5%) |
| ooffice | 109,875 | 1,318 | 92,398 | 103,140 (93.9%) |

An exact gap decoder iterates the sorted stored entries. It computes runs of
implicit unit-frequency symbols arithmetically. The dense-equivalence test now
checks every cumulative value after rescaling. Artifact sizes remained exactly
the same.

Two additional optimizations were rejected before the gap decoder:

- A context-to-row index cache reduced hash calls but added a random vector
  access. Median slowdowns still exceeded two times on four domains.
- Four inline continuation entries avoided many heap allocations, but enlarged
  every map row. Mozilla peak memory increased from about 128 MiB to 135 MiB,
  and the speed gate still failed.

## Optimized threshold-64 results

The following table reports medians after one warm-up and five new measured
repetitions. Size reductions are unchanged from the threshold screen.

| Input | Order-2 bytes | Backoff bytes | Size reduction | Order-2 encode (s) | Backoff encode (s) | Order-2 decode (s) | Backoff decode (s) | Decode slowdown | Backoff encode (MiB/s) | Backoff decode (MiB/s) | Peak RSS (KiB) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| dickens | 412,203 | 418,727 | -1.6% | 0.094 | 0.156 | 0.114 | 0.162 | 1.43x | 6.22 | 5.99 | 8,448 |
| samba | 789,852 | 760,797 | 3.7% | 0.180 | 0.330 | 0.232 | 0.352 | 1.52x | 6.24 | 5.86 | 19,268 |
| xml | 220,755 | 215,818 | 2.2% | 0.049 | 0.081 | 0.065 | 0.085 | 1.30x | 6.32 | 6.03 | 8,448 |
| mozilla | 2,393,950 | 2,333,655 | 2.5% | 0.567 | 1.288 | 0.675 | 1.205 | 1.78x | 3.79 | 4.05 | 130,872 |
| ooffice | 355,001 | 355,611 | -0.2% | 0.074 | 0.134 | 0.085 | 0.146 | 1.72x | 4.37 | 4.02 | 25,340 |

## Decision

Accept the classical development gate. Mature order-3 contexts improve
complete size on source, structured-document, and executable development data.
The optimized decoder stays below the two-times limit on every input, remains
below one gigabyte, and restores every byte exactly.

Do not propose a tensor representation from this result alone. RFC 0002 showed
that a useful dependency does not imply that the tested tensor training
objective can represent it efficiently. Validate threshold 64 on a separate
held-out corpus before deciding whether this classical model deserves further
codec work. Any tensor attempt still requires a new RFC with a specific
parameter-sharing and training mechanism.

## Reproduction

Screen every predefined threshold:

```console
cargo run --release -p tak-tensor --bin tak-order3-backoff-gate -- input.bin
```

Measure the selected threshold:

```console
cargo run --release -p tak-tensor --bin tak-order3-backoff-gate -- \
  input.bin --threshold 64
```
