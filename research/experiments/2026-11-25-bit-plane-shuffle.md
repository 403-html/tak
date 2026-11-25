# Bit-plane shuffle before TK32

RFC 0041 rearranges the bits of each 64 KiB block into eight planes.
The screen compares complete files including the nine wrapper bytes.
All 90 Calgary artifacts restored byte for byte. Every RFC 0038
control reproduced its artifact SHA-256 and byte length.

| Calgary profile | Complete bytes |
|---|---:|
| RFC 0038 selection | 893,989 |
| Selection with bit planes available | 893,989 |
| Bit planes on every file | 2,276,267 |

The bit-plane stream is larger than the existing selection on every
file. Geo grows from 54,098 to 60,972 bytes, obj2 from 76,959 to
200,870 bytes, and pic from 51,986 to 84,890 bytes. No profile
change meets the 2,235-byte aggregate gate. The experiment stops
before Canterbury and enwik9.

| Calgary operation | Wall s | MB/s | Peak RSS KiB |
|---|---:|---:|---:|
| Encode selected profiles only | 7.128 | 0.456 | 121,460 |
| Decode selected profiles | 2.802 | 1.161 | 48,544 |
| Encode all RFC 0038 profiles | 28.822 | 0.113 | 122,748 |
| Encode all RFC 0041 profiles | 36.881 | 0.088 | 169,388 |

Times sum per-file results. Search throughput divides original bytes
by the total time of all encode trials. These single-run results
include process startup and temporary file I/O. Exact comparison
occurs outside timed decoding.

The retained Calgary selection is 8,129 bytes above xz -9. The
retained enwik9 artifact is 243,396,020 bytes, 27,721,824 above
xz -9 and 46,396,020 above the nominal 197 MB M2 ceiling.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/byte_shuffle.py screen target/release/tak-lz-stream CALGARY --bit-planes
```

The raw width column uses 1 for bit-plane shuffle and preserves the
RFC 0038 meanings for other values. SHA-256 verifies the input
files against the RFC 0038 Calgary records.

Source is `a0e182a` plus RFC 0041. Linux 6.18.57, Intel Xeon Platinum
8573C, x86-64, three visible logical CPUs, 10,207,480 KiB visible RAM,
and an 8 GiB cgroup limit. Rust 1.85.0 release build, Python 3.12.14,
and one codec thread. Each row is a single run.

[Raw complete-artifact trials](../../benchmarks/reports/raw/2026-11-25-bit-plane-shuffle-calgary.tsv)
SHA-256 `9593acc61333662ec97458d3de5dac08f1b7b0355d7eb6fe272377bba1975a2a`.
