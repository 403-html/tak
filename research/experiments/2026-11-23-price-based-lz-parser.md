# Calgary-validated artifact: price-based LZ parser

## Decision

RFC 0016 passes its size gate on the Silesia development data. It also passes
the frozen Calgary validation. The first controlled timing did not pass the
decode limit. The median decode ratio was 1.102, and the limit was 1.10. Then
a decoder-only follow-up decreased the cost of literals. It did not change any
artifact byte. Two controlled runs then gave decode ratios of 1.053 and 1.050.
Thus the decode limit now passes.

RFC 0016 is the smallest measured TAK Calgary result. RFC 0015 stays the
README profile, because there is no RFC 0016 enwik9 result.

The parser changes only the choices of the encoder. The `TKLZ` syntax and the
decoder do not change. Each artifact restores exactly through the existing
decoder.

## Token-cost profile

`tak-lz-profile` parses the input with the frozen RFC 0015 parser. Its
candidate bits are equal to the RFC 0015 Silesia and Calgary opportunity rows.

| Part | Silesia bits per byte | Calgary bits per byte |
|---|---:|---:|
| Literal bytes | 1.394 | 1.247 |
| Distance remainder bits | 0.513 | 0.649 |
| Distance buckets | 0.176 | 0.180 |
| Match lengths | 0.176 | 0.175 |
| Literal lengths | 0.096 | 0.127 |
| Block types | 0.061 | 0.074 |

Parser misses and mechanism limits on the Silesia development data:

| Measurement | Count | Bits per byte |
|---|---:|---:|
| Matches that cost more than their byte-model bits | 145,471 of 416,990 | 0.0696 excess |
| Distance bits on the last four repeat distances | 49,233 matches | 0.0539 upper limit |
| Literal exclusion after a match stops | 209,214 literals | 0.0108 |
| One-step lazy positions with a longer match | 46,144 matches | no price |

Repeat-distance coding and literal exclusion can save less than 0.06 bits per
byte. The largest measured miss is that the parser accepts matches without a
cost comparison. Thus RFC 0016 tests a price-based parser. Calgary shows the
same pattern: 57,116 matches cost too much, with 0.0665 excess bits per byte.

## Development gate

| Group | RFC 0015 bytes | RFC 0016 bytes | Change |
|---|---:|---:|---:|
| Dickens | 339,732 | 316,625 | -6.80% |
| Samba | 404,081 | 371,039 | -8.18% |
| XML | 117,194 | 108,714 | -7.24% |
| Mozilla | 1,767,879 | 1,695,206 | -4.11% |
| OOffice | 216,220 | 210,780 | -2.52% |
| **Aggregate** | **2,845,106** | **2,702,364** | **-5.02%** |

The aggregate decrease is 0.1208 bits per input byte. The gate required 0.05
bits per byte and at least three smaller groups. All five groups are smaller.
These are development-corpus results. They are preliminary.

## Calgary validation

| Metric | RFC 0015 | RFC 0016 | Change |
|---|---:|---:|---:|
| Calgary bytes | 996,359 | 950,768 | -45,591 (-4.58%) |
| Median encode time | 1.679 s | 1.933 s | 1.15x |
| Median decode time | 0.820 s | 0.903 s | 1.10x (1.102) |
| Exact round trips | 18/18 | 18/18 | — |

All 18 files are smaller. The `geo` and `obj1` files each decrease by fewer
than 70 bytes. The peak process RSS is 55,812 KiB. The timing uses one warm-up
and five measured aggregate repetitions through `tak-lz-stream`.

The priced parse codes 1,895,245 Calgary bytes as literals. RFC 0015 codes
936,030 bytes as literals. The priced parse has 79,345 matches. RFC 0015 has
165,917 matches. The decoder must decode each literal with the arithmetic
coder and the byte model. This is why the decode time increases. A separate
instrumented run counted the blocks that went to the encoder. That run had no
timing.

## Decoder follow-up

The follow-up keeps the 1.10 limit. It changes only the decoder
implementation. It has three changes:

- Dense byte distributions and normalized order-2 increments keep the sum of
  each group of 16 symbols.
- The bit reader gives payload bits from a buffered 64-bit register.
- The range coder does renormalization in two bulk steps.

A randomized test compares the bulk renormalization with the single-bit loop
on 20,000 states.

| Decoder | RFC 0015 median decode | RFC 0016 median decode | Ratio |
|---|---:|---:|---:|
| Original | 0.820 s | 0.903 s | 1.102 |
| Group sums and buffered reader only | 0.646 s | 0.727 s | 1.124 |
| Complete follow-up | 0.659 s | 0.694 s | 1.053 |
| Complete follow-up, repeated | 0.660 s | 0.694 s | 1.050 |

The intermediate ratio increased. Its two changes also decreased the cost of
work that literals and matches share. Each row uses one warm-up and five
measured aggregate repetitions on all 18 files. The artifact sizes do not
change: 996,359 bytes for RFC 0015 and 950,768 bytes for RFC 0016. Each decode
is exact. The peak process RSS is 56,212 KiB or less. The median encode ratios
in these runs are 1.17 to 1.18.

With the complete follow-up, RFC 0016 decodes approximately 15% faster than
RFC 0015 decoded with the original decoder.

## Competitive position

With the same independent-file Calgary accounting, RFC 0016 is:

- 108,567 bytes smaller than gzip;
- 29,695 bytes larger than Zstandard level 19;
- 64,908 bytes larger than xz; and
- 49,927 bytes smaller than RFC 0010.

It removes 41.3% of the difference between RFC 0015 and xz on Calgary. To
equal xz, it must remove 6.83% more of its bytes.

There is no enwik9 result. A bounded encoder is now available, but the
measurement host could not get the enwik9 file. Do not apply the Calgary
decrease to enwik9. The enwik9 record is still RFC 0015 at 276,339,048 bytes.
That is 60,664,852 bytes more than xz.

## Bounded encoder screen

The bounded encoder (`tak-lz-stream encode-priced-bounded`) keeps a 128 MiB
input ring. It reads old literal runs again in 64 KiB pages. It makes the same
artifacts as the buffered encoder.

| Full Mozilla, 51,220,480 bytes | RFC 0015 bounded | RFC 0016 bounded |
|---|---:|---:|
| Artifact bytes | 20,105,655 | 19,299,991 (-4.01%) |
| Encode time | 22.18 s | 26.92 s |
| Encode peak RSS | 80,156 KiB | 189,476 KiB |
| Decode time | 7.45 s | 8.07 s |
| Decode peak RSS | 105,148 KiB | 105,444 KiB |
| Exact restore | yes | yes |

These are single screening runs. The file contains development data. The
results show bounded memory. They are not publishable throughput results.

## Limitations and next experiment

The next required step is one enwik9 measurement with the bounded encoder.
Keep the 1.10 decode limit for that measurement. Do not tune the segment
length, the nice length, the range of priced lengths, the literal start price,
or the candidate rule on these results.

This classical parser result does not permit tensor work.

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
| `benchmarks/reports/raw/2026-11-23-lz-priced-bounded-mozilla.tsv` | `07b9fd18525e987bf9730ff96c302fa9975437003ea9a4fd6b7592150af3ff25` |

The input digests are equal to the digests in
`benchmarks/reports/raw/2026-11-22-calgary-inputs.tsv` and
`benchmarks/reports/raw/2026-11-23-match-dependency-inputs.tsv`. The gate and
profile rows are single screening runs without a warm-up. Their timings are not
publishable. The sizes and round trips are exact.

## Reproduction

```console
cargo build --release -p tak-tensor
target/release/tak-lz-profile silesia/dickens --tail-bytes 1019245
target/release/tak-lz-priced-gate silesia/dickens --tail-bytes 1019245
target/release/tak-lz-priced-gate calgary/book1
target/release/tak-lz-stream encode-priced calgary/book1 book1.tklz
target/release/tak-lz-stream encode-priced-bounded calgary/book1 book1b.tklz
target/release/tak-lz-stream decode book1.tklz book1.out \
  --max-output 1000000000
```

The Silesia tail lengths are: Dickens 1,019,245, Samba 2,160,640, XML
534,528, Mozilla 5,122,048, and OOffice 615,220 bytes. Each gate and profile
row ran through `benchmarks/runners/measure.py`.

## Environment

- CPU: Intel Xeon Processor at 2.10 GHz, x86-64, four visible logical CPUs.
- OS: Linux 6.18.44.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none.
- Date: 2026-11-23 Europe/Warsaw.
- Source: commit `c58cc95` plus the `encode-priced` stream mode. The decoder
  follow-up used commit `2f9cb71` plus the decoder changes. The bounded
  encoder screen used commit `00268ea` plus the bounded encoder.
