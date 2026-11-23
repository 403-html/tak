# Calgary-validated artifact: price-based LZ parser

## Decision

RFC 0016 passes its size gate on Silesia development data and on frozen
Calgary validation. The first controlled timing missed the pre-registered
decode bound: the median decode ratio was 1.102 against a bound of 1.10. A
decoder-only follow-up then reduced literal cost without changing any artifact
byte. Two repeated controlled runs gave decode ratios of 1.053 and 1.050, so
the bound now passes.

RFC 0016 is the smallest measured TAK Calgary result. RFC 0015 remains the
README profile because the RFC 0016 enwik9 result is unmeasured.

The parser changes only encoder choices. The `TKLZ` syntax and decoder are
unchanged, and every artifact restores exactly through the existing decoder.

## Token-cost profile

`tak-lz-profile` replays the frozen RFC 0015 parse. Its candidate bits
reproduce the RFC 0015 Silesia and Calgary opportunity rows exactly.

| Component | Silesia bits per byte | Calgary bits per byte |
|---|---:|---:|
| Literal bytes | 1.394 | 1.247 |
| Distance remainder bits | 0.513 | 0.649 |
| Distance buckets | 0.176 | 0.180 |
| Match lengths | 0.176 | 0.175 |
| Literal lengths | 0.096 | 0.127 |
| Block types | 0.061 | 0.074 |

Parser misses and mechanism bounds on Silesia development data:

| Measurement | Count | Bits per byte |
|---|---:|---:|
| Matches costing more than their byte-model bits | 145,471 of 416,990 | 0.0696 excess |
| Distance bits on last-four repeat distances | 49,233 matches | 0.0539 upper bound |
| Literal exclusion after a mismatching match | 209,214 literals | 0.0108 |
| One-step lazy positions with a longer match | 46,144 matches | not priced |

Repeat-distance coding and literal exclusion are bounded below 0.06 bits per
byte. Cost-blind match acceptance is the largest measured miss. RFC 0016
therefore tests a price-based parser. Calgary shows the same pattern: 57,116
unprofitable matches with 0.0665 excess bits per byte.

## Development gate

| Group | RFC 0015 bytes | RFC 0016 bytes | Change |
|---|---:|---:|---:|
| Dickens | 339,732 | 316,625 | -6.80% |
| Samba | 404,081 | 371,039 | -8.18% |
| XML | 117,194 | 108,714 | -7.24% |
| Mozilla | 1,767,879 | 1,695,206 | -4.11% |
| OOffice | 216,220 | 210,780 | -2.52% |
| **Aggregate** | **2,845,106** | **2,702,364** | **-5.02%** |

The aggregate saving is 0.1208 bits per input byte. The gate required 0.05
and at least three improved groups. All five groups improve. These are
development-corpus results and are preliminary.

## Calgary validation

| Metric | RFC 0015 | RFC 0016 | Change |
|---|---:|---:|---:|
| Calgary bytes | 996,359 | 950,768 | -45,591 (-4.58%) |
| Median encode time | 1.679 s | 1.933 s | 1.15x |
| Median decode time | 0.820 s | 0.903 s | 1.10x (1.102) |
| Exact round trips | 18/18 | 18/18 | — |

All 18 files improve. `geo` and `obj1` improve by fewer than 70 bytes each.
Peak process RSS is 55,812 KiB. Timings use one warm-up and five measured
aggregate repetitions through `tak-lz-stream`.

The priced parse codes 1,895,245 Calgary bytes as literals instead of 936,030.
It emits 79,345 matches instead of 165,917. Literals require an
arithmetic-coded byte-model step in the decoder. This explains the direction of
the decode-time change. The block counts come from a separate instrumented run
that counted the blocks passed to the encoder. That run was not timed.

## Decoder follow-up

The follow-up keeps the 1.10 bound and changes decoder implementation only.
It adds 16-symbol group sums to dense byte distributions and normalized
order-2 increments, serves payload bits from a buffered 64-bit register, and
applies range-coder renormalization in two bulk steps. A randomized test checks
the bulk renormalization against the single-bit loop on 20,000 states.

| Decoder | RFC 0015 median decode | RFC 0016 median decode | Ratio |
|---|---:|---:|---:|
| Original | 0.820 s | 0.903 s | 1.102 |
| Group sums and buffered reader only | 0.646 s | 0.727 s | 1.124 |
| Complete follow-up | 0.659 s | 0.694 s | 1.053 |
| Complete follow-up, repeated | 0.660 s | 0.694 s | 1.050 |

The intermediate row rose because those two changes also sped up work shared by
literals and matches. Each row uses one warm-up and five measured aggregate
repetitions over all 18 files. Artifact sizes are unchanged: 996,359 bytes for
RFC 0015 and 950,768 for RFC 0016. Every decode is exact. Peak process RSS is
56,212 KiB or less. Median encode ratios in these runs are 1.17 to 1.18.

The complete follow-up also makes RFC 0016 decode about 15% faster than the
original decoder took on RFC 0015.

## Competitive position

On the same independent-file Calgary accounting, RFC 0016 is:

- 108,567 bytes smaller than gzip;
- 29,695 bytes larger than Zstandard level 19;
- 64,908 bytes larger than xz; and
- 49,927 bytes smaller than RFC 0010.

It closes 41.3% of the RFC 0015-to-xz gap on Calgary. It must remove a further
6.83% of its bytes to match xz.

The enwik9 result is unmeasured. The research encoder reads the complete input,
so it does not meet the bounded-memory rule for a one-billion-byte input. Do not
extrapolate the Calgary reduction to enwik9. The current enwik9 record remains
RFC 0015 at 276,339,048 bytes, 60,664,852 bytes above xz.

## Limitations and next experiment

The next required step is a bounded priced encoder and one enwik9 measurement.
Keep the 1.10 decode bound for that measurement. Do not tune segment length, nice length, priced length range, literal start
price, or the candidate rule on these results.

This classical parser result does not authorize tensor work.

## Raw results

| File | SHA-256 |
|---|---|
| `benchmarks/reports/raw/2026-11-23-lz-profile-silesia.tsv` | `62c9821be0bd5d94520f34e6b3044d01d73e4eb1a5ab4ea7db332b7c4f228045` |
| `benchmarks/reports/raw/2026-11-23-lz-profile-calgary.tsv` | `d392298cf47c24357ba049cf4a9ec226903954ed1220a537a240f1d7a879eec1` |
| `benchmarks/reports/raw/2026-11-23-lz-priced-silesia.tsv` | `832f442044ecda5def2e7d8627da651355489ace61d56891f94048a7595e98ee` |
| `benchmarks/reports/raw/2026-11-23-lz-priced-calgary.tsv` | `bf9ae772150319a2156f0f72e7b74d798bf3d5f3e5a388366f936f9a2d1063f7` |
| `benchmarks/reports/raw/2026-11-23-lz-priced-controlled.tsv` | `48ea8d9845689c424fac7f2e710f82cf898bfd1dd48b7ce147ab787f88b23d49` |
| `benchmarks/reports/raw/2026-11-23-lz-priced-blocks-calgary.tsv` | `16281a7c20872ba0ee0762cc3f4ef12611c3cb23c07ed051752a5acb33fa7c21` |
| `benchmarks/reports/raw/2026-11-23-lz-priced-decoder-controlled.tsv` | `cab8b54f2b82b3863068ba3a3894f6b79759fbf39f7a68900ae47aeb72bc0639` |

Input digests match `benchmarks/reports/raw/2026-11-22-calgary-inputs.tsv` and
`benchmarks/reports/raw/2026-11-23-match-dependency-inputs.tsv`. The gate and
profile rows are single screening runs without a warm-up. Their timings are not
publishable. Sizes and round trips are exact.

## Reproduction

```console
cargo build --release -p tak-tensor
target/release/tak-lz-profile silesia/dickens --tail-bytes 1019245
target/release/tak-lz-priced-gate silesia/dickens --tail-bytes 1019245
target/release/tak-lz-priced-gate calgary/book1
target/release/tak-lz-stream encode-priced calgary/book1 book1.tklz
target/release/tak-lz-stream decode book1.tklz book1.out \
  --max-output 1000000000
```

Silesia tail lengths are Dickens 1,019,245, Samba 2,160,640, XML 534,528,
Mozilla 5,122,048, and OOffice 615,220 bytes. Each gate and profile row was
run through `benchmarks/runners/measure.py`.

## Environment

- CPU: Intel Xeon Processor at 2.10 GHz, x86-64, four visible logical CPUs.
- OS: Linux 6.18.44.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-23 Europe/Warsaw.
- Source: commit `c58cc95` plus the `encode-priced` stream mode. The decoder
  follow-up was measured on commit `2f9cb71` plus the decoder changes.
