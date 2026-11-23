# Accepted Calgary artifact: two-candidate LZ search

## Decision

Accept RFC 0015 on Calgary and permit one bounded enwik9 measurement. Keeping
the two most recent positions for each exact 8-byte key improves every Silesia
development group and every Calgary file. It changes encoder search only. The
`TKLZ` syntax and decoder are unchanged.

Do not add more candidates or tune the parser on these corpora. The measured
Calgary improvement is useful but small.

## Opportunity results

| Corpus | One-candidate ideal bits | Two-candidate ideal bits | Incremental saving | Saving per byte |
|---|---:|---:|---:|---:|
| Silesia development | 23,053,197.854 | 22,820,063.339 | 233,134.515 | 0.02467 bits |
| Calgary validation | 8,040,841.017 | 7,974,778.374 | 66,062.643 | 0.02032 bits |

All five development groups and all 18 validation files improve. The method
passes the RFC 0015 gate without parameter changes.

## Complete artifact

| Metric | RFC 0014 | RFC 0015 | Change |
|---|---:|---:|---:|
| Calgary bytes | 1,004,678 | 996,359 | -8,319 (-0.83%) |
| Median encode time | 1.538 s | 1.751 s | 1.14x |
| Median decode time | 0.743 s | 0.755 s | 1.02x |
| Exact round trips | 18/18 | 18/18 | — |

RFC 0015 is 4,336 bytes smaller than RFC 0010, the prior TAK Calgary record.
It remains 75,286 bytes above Zstandard-19 and 110,499 bytes above xz. Peak
process RSS is 49,952 KiB. Timings use one warm-up and five measured aggregate
repetitions.

## Enwik9 result

At commit `f527fd5`, the bounded two-candidate encoder produces a
276,339,048-byte artifact. It improves RFC 0014 by 5,091,585 bytes, or 1.81%,
and RFC 0009 by 39,239,718 bytes, or 12.43%. The input and restored SHA-256 are
both `159b85351e5f76e60cbe32e04c677847a9ecba3adc79addab6f4c6c7aa3744bc`.
The artifact SHA-256 is
`90228bdd3fd2b54c9bc87493adecf4ef33b88a7495af6b70d24457a826f5c55c`.

Encoding takes 715.42 seconds at 1.40 MB/s with 54,436 KiB peak RSS. Decoding
takes 160.91 seconds at 6.21 MB/s with 94,992 KiB peak RSS. These are one-run
screening results without a warm-up. Both throughput targets fail.

The artifact beats M1 by 45,660,952 bytes. It remains 41,010,109 bytes above
Zstandard-19, 60,664,852 bytes above xz, and 79,339,048 bytes above M2. Matching
xz requires a further 21.95% reduction from the current artifact.

Raw data is in
`benchmarks/reports/raw/2026-11-23-lz-two-candidate-enwik9.tsv`, SHA-256
`08152a74c67cf9c7682d72d8f934afd043b44da105b02534af6a224f81341446`.

## Limitations and next experiment

The result establishes an enwik9 gain but does not approach xz. Reject further
candidate-count expansion. Later ratio work must identify another measured
mechanism. Profile token costs and parser misses before proposing a format
change.

This classical parser result does not authorize tensor work. A new tensor RFC
must still name a specific longer-context dependency that the rejected bigram
factorization cannot represent.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-23 Europe/Warsaw.
- Source: commit `671b672` plus the RFC 0015 working tree.
