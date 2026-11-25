# Predict match distance from the previous match

RFC 0036 ran the absolute and signed distance predictors on the same
37,763 matches from the RFC 0034 Calgary parse. Of these, 1,963 repeated
the immediately previous distance.

| Scope | Absolute ideal bits | Signed ideal bits | Mixture ideal bits |
|---|---:|---:|---:|
| Calgary | 560,177.882 | 610,224.292 | 556,671.136 |

The mixture saves 3,506.746 ideal bits, or 0.001079 per input byte.
The gate required at least 0.010 and improvement on 12 files. Three
files improved. Pic accounts for 3,351.867 bits of the savings, geo
for 118.356, and progp for 51.523. Each other file costs about one
extra bit under the equal-prior mixture. The signed predictor saves
14,223.681 bits on exact repeats and costs 64,270.091 extra bits on
nonzero differences. The previous match distance is too weak as a
file-wide reference for nonrepeating distances under this code.

The experiment stops at the Calgary diagnostic. Canterbury, enwik9,
complete artifact size, exact candidate restoration, and candidate
decoder resources remain unmeasured. Peak diagnostic RSS was 72,728
KiB. The diagnostic materializes the parse and does not measure
streaming codec resource use.

## Distance to goal

The best verified TAK artifact remains RFC 0034. Calgary totals
906,449 bytes, 20,589 bytes above xz -9. The enwik9 artifact is
243,396,020 bytes, 27,721,824 bytes above xz -9 and 46,396,020 bytes
above the nominal 197 MB M2 ceiling.

[RFC 0034 size, time, RSS, and exact restoration results](2026-11-25-tk32-literal-repricing.md)

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-length-distance
python3 benchmarks/runners/measure.py --timing timing.tsv --stdout row.tsv \
  target/release/tak-length-distance --relative INPUT
```

Run once per file in the [Calgary input manifest](../../benchmarks/reports/raw/2026-11-22-calgary-inputs.tsv).
Verify the byte length and SHA-256 of every input. The raw rows include
both command wall time and peak RSS. This is a single-run screen.

Source is commit `a4a7f46` plus RFC 0036. Linux 6.18.57, Intel Xeon
Platinum 8573C, x86-64, three visible logical CPUs, and 10,207,480 KiB
visible RAM. The cgroup memory cap was 8 GiB. Rust 1.85.0 release
build, Python 3.12.14 runner, and one thread.

[Raw Calgary rows](../../benchmarks/reports/raw/2026-11-25-relative-distance-calgary.tsv)
SHA-256 `6634f61ea0209bbbe0b6d64edf15830b3cbe8b8f851e4d76ae8f6aca2954c57c`.
