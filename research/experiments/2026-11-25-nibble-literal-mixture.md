# Bit-group literal prediction

RFC 0035 tested separate upper and lower four-bit predictors on
2,300,874 literals from the frozen RFC 0034 Calgary parse. The
alternate predictor was stronger on geo. TK32 was stronger on
the other 17 files, including obj2 and pic.

| Scope | TK32 ideal bits | Bit-group ideal bits | Mixture ideal bits |
|---|---:|---:|---:|
| Calgary | 6,286,188.962 | 8,356,021.653 | 6,218,065.899 |
| geo | 530,372.274 | 462,231.211 | 462,232.211 |
| obj2 | 524,714.986 | 713,932.610 | 524,715.986 |
| pic | 371,476.760 | 411,270.333 | 371,477.760 |

The equal-prior mixture saves 68,123.064 ideal bits across Calgary,
or 0.020951 per input byte. The gate required 0.03. The three target
files save 0.079007 bits per input byte in aggregate, passing their
0.02 gate solely because of geo. The screened model offers a lead
for geo-like data. Moving it into a general literal model would
require an independent corpus with similar structure.

The diagnostic stops before Canterbury, artifact development, and
enwik9. Candidate compressed size, exact decoding, and decode
resources remain unmeasured. Peak diagnostic RSS was 107,420 KiB.

The retained RFC 0034 Calgary artifact is 906,449 bytes, 20,589
bytes above xz. Its enwik9 artifact is 243,396,020 bytes,
27,721,824 bytes above xz and 46,396,020 bytes above nominal M2.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-match-distance-literal
python3 benchmarks/runners/measure.py --timing timing.tsv --stdout row.tsv \
  target/release/tak-match-distance-literal --nibble-mixture INPUT
```

Run once per Calgary file. The runner reports wall time and peak RSS.
The raw file records input digests and all 18 results. Source is
`4336f2c` plus RFC 0035. Linux 6.18.57, Intel Xeon Platinum 8573C,
x86-64, three visible logical CPUs, and 10,207,480 KiB visible RAM.
Rust 1.85.0 release build and Python 3.12.14 runner. One thread and
no experiment-specific process memory cap.

[Raw Calgary rows](../../benchmarks/reports/raw/2026-11-25-nibble-literal-mixture.tsv)
SHA-256 `a3eb7255508f8b10ad01a32a6a3c1eaa451000938875377709cf21fb80190f0f`.
