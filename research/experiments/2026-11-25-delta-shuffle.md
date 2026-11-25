# Modular delta before byte shuffle

RFC 0040 tests a modular byte difference between neighboring records
before shuffling each block. The three candidate widths are 2, 4,
and 8. All 126 complete artifacts restore exactly. Every RFC 0038
control reproduces its recorded size and SHA-256.

| Fixed profile across Calgary | Complete bytes |
|---|---:|
| Identity | 906,449 |
| Shuffle width 2 | 1,293,314 |
| Shuffle width 4 | 1,584,431 |
| Shuffle width 8 | 1,740,362 |
| Delta shuffle width 2 | 1,546,327 |
| Delta shuffle width 4 | 1,920,373 |
| Delta shuffle width 8 | 2,109,384 |

None of the 54 delta trials improves its file's RFC 0038 selection.
Both searches select 893,989 bytes. For geo, the smallest delta shuffle
is 54,867 bytes, 769 bytes above the plain width-4 shuffle. For obj2,
the smallest delta shuffle is 109,581 bytes, 32,622 above ordinary TK32.
The complete streams give no size benefit from adding the subtraction
step to this encoder.

| Calgary operation | Wall s | MB/s | Peak RSS KiB |
|---|---:|---:|---:|
| Encode selected profiles only | 6.666 | 0.488 | 121,480 |
| Decode selected profiles | 3.466 | 0.938 | 48,544 |
| Encode all RFC 0038 profiles | 30.472 | 0.107 | 122,732 |
| Encode all RFC 0040 profiles | 53.813 | 0.060 | 143,072 |

Selected times sum one result per file. Search throughput uses original
corpus bytes divided by the total time for all encode trials. These
single-run timings include process startup and temporary file I/O.
The exact comparison is outside the timed decode.

The gain is zero against a threshold of 2,235 bytes. Both Canterbury
and enwik9 remain unmeasured for the delta profiles. The retained
Calgary result is 8,129 bytes above xz -9. The retained enwik9 result
is 243,396,020 bytes, 27,721,824 above xz -9 and 46,396,020 above
the nominal 197 MB M2 ceiling.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/byte_shuffle.py screen target/release/tak-lz-stream CALGARY --delta-shuffle
```

The raw width column uses 102, 104, and 108 to distinguish delta
widths from shuffle widths. The artifact header stores the actual
width. Input hashes match the RFC 0038 Calgary rows. The measurements
include the nine-byte wrapper for each transformed stream.

Source is `d72c90c` plus RFC 0040. Linux 6.18.57, Intel Xeon Platinum
8573C, x86-64, three visible logical CPUs, 10,207,480 KiB visible RAM,
and an 8 GiB cgroup limit. Rust 1.85.0 release build, Python 3.12.14,
one codec thread, and one run per file and profile.

[Raw complete-artifact trials](../../benchmarks/reports/raw/2026-11-25-delta-shuffle-calgary.tsv)
SHA-256 `c8da82252156496fbbc9fd294faf0c7877a5a1e56ef94678a2d560c381cc9a95`.
