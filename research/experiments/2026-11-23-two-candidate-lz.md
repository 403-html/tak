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

## Limitations and next experiment

The result does not establish an enwik9 gain. Run the frozen bounded encoder
once on enwik9 and verify the complete restored SHA-256. Reject further
candidate-count expansion even if enwik9 improves. Later ratio work must
identify another measured mechanism.

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
