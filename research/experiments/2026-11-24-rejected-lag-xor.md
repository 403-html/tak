# Reversible lag-XOR screen

RFC 0031 tested whether a fixed-distance XOR residual would improve
compression with the RFC 0019 encoder. The selector tried identity and
nine lags for every file and chose the smallest physical artifact.
Every nonidentity result includes the 10-byte wrapper and the full
nested TKT9 stream.

| Corpus | Input bytes | RFC 0019 bytes | Selected bytes | Savings | Files improved |
|---|---:|---:|---:|---:|---:|
| Calgary | 3,251,493 | 942,454 | 942,239 | 215 (0.0228%) | 1/18 |
| Canterbury | 2,810,784 | 675,273 | 675,273 | 0 | 0/11 |
| Independent Canterbury binary subset | 1,067,984 | 236,375 | 236,375 | 0 | 0/2 |

Lag 4 decreased geo from 65,256 to 65,041 bytes. Obj2 and pic selected
identity. Canterbury's ptt5 and Calgary's pic have identical SHA-256
digests. The independent binary subset therefore contains kennedy.xls and
sum. Calgary is exploratory because its gaps informed the experiment.
Neither aggregate reached the 0.5% threshold.

Every trial restored the source byte for byte. Boundary smoke checks cover
the transform on empty and short input and wrapper rejection for
unsupported lag, checksum mismatch, truncation, trailing bytes, and
output-limit excess.

In Calgary, identity took 3.561 s total to encode and 1.803 s to decode.
The selected trials took 3.575 s and 1.840 s, respectively. Peak RSS
among the selected trials was 78,656 KiB for encoding and 26,756 KiB for
decoding. The full Calgary search spent 43.150 s in the ten encode trials
per file, summed across the corpus. Peak RSS across all trials in both
corpora was 130,836 KiB for encoding and 51,360 KiB for decoding. These
one-repetition measurements include process startup and wrapper work.

The screened Calgary total is still 56,379 bytes above xz (885,860).
It closes 0.38% of the current 56,594-byte gap. RFC 0019 remains the
retained profile. Its enwik9 artifact is 249,762,881 bytes, 34,088,685
above xz and 52,762,881 above nominal M2. Lag-XOR has no enwik9 measurement.

The next distinct hypothesis is that a bounded distribution over multiple
continuations can improve literal prediction where the single-continuation
RFC 0030 fallback was weak. Compare evidence-weighted context depth with
a bounded PPM-style control on the same literal events before framing a
new artifact. The lag screen provides no justification for a wider search
over these fixed transforms.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/lag_xor.py screen target/release/tak-lz-stream CALGARY_DIR
python3 benchmarks/runners/lag_xor.py screen target/release/tak-lz-stream CANTERBURY_DIR
```

Take the minimum artifact bytes per file. The raw rows order identity
first and then lags in ascending order, which defines tie breaking.

Source is `cd5ab66` plus the RFC 0031 runner. Measurements used Rust 1.85.0
in release mode, Python 3.12.14, Linux 6.18.57, and an x86-64 Intel Xeon
Platinum 8573C. The environment exposed three logical CPUs and 10,207,480
KiB reported memory. Runs used one thread and no imposed process memory
limit. The runner caps the input and restored output at 8 MiB.

Raw rows include SHA-256 input hashes, physical bytes, per-operation
timings, RSS, and exact restoration status. The TSV SHA-256 is
`b702288fa8b0c67e272eea45b9a07007bdb8687a65e914ae6f0fc78c4406008a`.
[Raw rows](../../benchmarks/reports/raw/2026-11-24-lag-xor-screen.tsv)
