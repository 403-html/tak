# RFC 0016 residual mechanism profile

## Decision

Do not create a repeat-distance or literal-exclusion format RFC. Their
combined zero-overhead upper bound is 0.05312 bits per input byte on the five
Silesia development tails and 0.01883 bits per input byte on Calgary. The
Calgary bound is only 7,654 bytes. A real code must spend bits to select a
repeat slot and to signal exclusion, so it cannot reach this bound.

The accepted RFC 0016 artifact is 29,695 bytes larger than Zstandard level 19
and 64,908 bytes larger than xz on Calgary. Even the unattainable upper bound
can remove only 25.8% of the Zstandard difference and 11.8% of the xz
difference. This mechanism does not justify a format change.

## Method

`tak-lz-priced-profile` runs the frozen RFC 0016 parser. It then accounts for
two mechanisms that RFC 0016 identified but did not test:

- A repeat-distance upper bound removes the complete current distance cost
  whenever a match uses one of the four most recent distances. It charges no
  repeat symbol or slot. Thus it is an upper bound, not a proposed code.
- A literal-exclusion upper bound removes the source continuation byte from
  the first literal distribution after a match. It counts the event only when
  the source byte differs from the actual literal. This condition matters
  because the price-based parser can end a match before a mismatch.

The diagnostic uses the adaptive distance distribution and byte model in
encoder order. It includes distribution halving. It does not create an
artifact and makes no compression claim.

## Results

| Corpus | Input bytes | Match blocks | Distance bits/byte | Repeat-4 upper bound | Exclusion upper bound | Combined upper bound |
|---|---:|---:|---:|---:|---:|---:|
| Silesia development | 9,451,681 | 259,743 | 0.36746 | 0.04849 | 0.00463 | 0.05312 |
| Calgary validation | 3,251,493 | 79,345 | 0.36939 | 0.01251 | 0.00633 | 0.01883 |

The repeat opportunity does not transfer uniformly. On Silesia, Mozilla
contributes 379,808 of the 458,307 repeat-distance bits. On Calgary, the
combined upper bound ranges from 0.00540 bits per byte on `paper3` to 0.06775
on `obj2`. The aggregate result is too small to justify implementation.

RFC 0016 codes 1,895,245 Calgary bytes as literals. This is 58.29% of the
input. Literal probability modeling is therefore a larger research surface
than the two rejected metadata mechanisms. A later experiment must measure a
specific dependency in those literals before it proposes a model or a tensor
network.

## Older exact-suffix source screen

The profiler also tests one specific longer-context dependency. The current
RFC 0009 byte model predicts from the latest prior occurrence of the previous
eight bytes. The RFC 0015 match table keeps one older occurrence. At RFC 0016
literal positions, the diagnostic adds one fixed bonus of `T / 5` to the
older continuation when it differs from the latest continuation. This equals
one quarter of the underlying order-2 total after the current latest-source
bonus. No bonus value was selected from the results.

This dependency uses two remote eight-byte source histories. The rejected
bigram tensor in RFC 0001 cannot represent it. It is nevertheless not a useful
tensor target:

| Corpus | Literal bytes | Source disagreements | Latest hits | Older hits | Candidate change |
|---|---:|---:|---:|---:|---:|
| Silesia development | 4,192,041 | 324,580 | 67,502 | 59,902 | +8,497 bits |
| Calgary validation | 1,895,245 | 144,516 | 33,864 | 30,986 | -97 bits |

The older source is less reliable than the latest source on both corpora. The
fixed candidate regresses by 0.00090 bits per input byte on Silesia. Its total
Calgary saving is 97 ideal bits, or approximately 12 bytes before any format,
table, or decoder cost. Reject this mechanism. Do not create RFC 0017 for it,
and do not increase tensor rank to fit it.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-priced-profile
target/release/tak-lz-priced-profile silesia/dickens.test
target/release/tak-lz-priced-profile calgary/book1
```

The Silesia rows use the final 10% files from the RFC 0016 development split.
The Calgary rows use all 18 files independently. Each row is one diagnostic
run. Timing is screening data.

Raw rows are in
`benchmarks/reports/raw/2026-11-23-lz-priced-residual.tsv`, SHA-256
`b36ceb8b0d30c801a29cb213ca90450723d03a538c9dd0939d632b785619aee1`.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-23 Europe/Warsaw.
- Source: commit `036e043` plus the residual profiler working tree.
