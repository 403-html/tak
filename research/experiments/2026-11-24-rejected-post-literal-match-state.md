# Rejected post-literal match state

## Decision

Reject RFC 0020. Splitting match-length and distance-bucket distributions by
whether the match follows a literal or another match increases ideal aggregate
cost on the Silesia development tails. Do not add previous-length,
previous-distance, or multi-block contexts on these corpora.

No compressed artifact was implemented. Calgary and enwik9 were not run.

## Frozen method

The diagnostic uses the RFC 0019 parse without changes. It maintains two
adaptive match-length bucket distributions and two adaptive distance-bucket
distributions. The preceding block type selects the distribution. All four
distributions start with unit frequencies. Raw remainder bits are unchanged
and fully counted.

The control uses RFC 0019's one global match-length distribution and one
global distance distribution. The candidate sends no state metadata because
the decoder already knows the preceding block type.

## Results

| File | Length change, saved bits | Distance change, saved bits | Combined saved bits | Saved bits/input byte |
|---|---:|---:|---:|---:|
| dickens | 222.110 | -510.373 | -288.263 | -0.000283 |
| mozilla | -894.914 | -2,560.762 | -3,455.676 | -0.000675 |
| ooffice | -45.607 | 65.384 | 19.777 | 0.000032 |
| samba | -403.099 | -770.105 | -1,173.204 | -0.000543 |
| xml | -49.789 | -45.943 | -95.732 | -0.000179 |

Only OOffice improves. Across 9,451,681 input bytes, the candidate adds
4,993.098 ideal bits, or 0.000528 bits per input byte. The frozen gate requires
all five files to improve and at least 0.01 saved bits per input byte.

The extra distributions fragment evidence more than the state dependency
helps. An oracle that selects components after seeing each file is prohibited.

Raw rows are in
`benchmarks/reports/raw/2026-11-24-post-literal-match-state.tsv`, SHA-256
`bdc6c9f38e92c3b7a9dbc44efc993707be1e7e5769394f12d1dd6d7bd442d136`.

## Tensor scope

This negative result does not authorize a tensor. The tested state has only
two values and direct classical tables already represent it. A tensor would
add complexity without a measured dependency or useful parameter-sharing
problem.

Any later tensor RFC still needs a different specific longer-context
dependency that the rejected bigram factorization cannot represent. Merely
increasing rank remains prohibited.

## Distance to goals

RFC 0019 remains the best measured TAK enwik9 artifact at 249,762,881 bytes.
It remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz still requires a 13.65% reduction.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `b4bec81` plus the RFC 0020 working tree.
- Command: `tak-lz-priced-profile INPUT --tail-bytes N`.
