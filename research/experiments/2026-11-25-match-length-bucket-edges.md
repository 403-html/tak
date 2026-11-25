# Match lengths at bucket boundaries

RFC 0037 tests intermediate LZ lengths just before and after the
bucket boundaries beyond the parser's dense search. Both profiles
write complete TK32 streams. The existing decoder restores all
18 Calgary inputs byte for byte under both profiles.

| Profile | Artifact bytes | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encoder KiB | Peak decoder KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| RFC 0034 control | 906,449 | 6.493 | 2.548 | 0.501 | 1.276 | 113,432 | 43,916 |
| RFC 0037 bucket edges | 906,445 | 6.121 | 2.515 | 0.531 | 1.293 | 113,324 | 44,064 |

Pic saves two bytes. Progp and trans each save one. Four files
change artifact SHA-256 at the same byte size. The other 11
artifacts are byte-identical to their controls.
The aggregate reduction is 0.000441%, below the 0.15% gate.
The experiment stops before Canterbury and enwik9. The timings
sum individual file runs, including startup and file output.
Peak RSS is the largest per-file result. This single-run screen
does not establish a speed difference.

The candidate is 20,585 bytes above xz -9 on Calgary. The retained
RFC 0034 control is 20,589 bytes above xz -9. Its verified enwik9
stream is 243,396,020 bytes, 8,067,081 bytes above Zstandard -19,
27,721,824 above xz -9, and 46,396,020 above the nominal 197 MB
M2 ceiling.

## Reproduction

Run both profiles on each independent file in the
[Calgary manifest](../../benchmarks/reports/raw/2026-11-22-calgary-inputs.tsv).
Use a fresh artifact and output path for each run.

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/measure.py --timing encode.tsv --stdout encode.out \
  target/release/tak-lz-stream encode-variable-order-repriced INPUT ARTIFACT
python3 benchmarks/runners/measure.py --timing decode.tsv --stdout decode.out \
  target/release/tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES
cmp INPUT RESTORED
```

For the candidate, replace the encoder command with
`encode-variable-order-bucket-edges`. Record the complete file size
and SHA-256 after encoding. Every screened input matches the published
input size and digest. The control reproduces the recorded artifact size
on every file.

Source is commit `0eb3155` plus RFC 0037. Linux 6.18.57, Intel Xeon
Platinum 8573C, x86-64, three visible logical CPUs, and 10,207,480 KiB
visible RAM. The cgroup limit was 8 GiB. Rust 1.85.0 release build,
Python 3.12.14 runner, one codec thread, one run per file and profile.

[Raw rows](../../benchmarks/reports/raw/2026-11-25-match-length-bucket-edges.tsv)
SHA-256 `56947d083501f5f03d17d7975324591d4b4e809a71ed70534677af2402fd533f`.
