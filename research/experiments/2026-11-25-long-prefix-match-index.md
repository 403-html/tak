# Sixteen-byte match index

RFC 0043 adds a second index to the RFC 0034 match search. A
sixteen-byte key can retain an older source when the two current
eight-byte sources have different continuations. The candidate
only adds sources that yield a longer match. Both encoders emit
complete TK32 streams, and the existing decoder restored every
input byte across all 36 trials.

| Calgary profile | Complete bytes | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encoder KiB | Peak decoder KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| RFC 0034 control | 906,449 | 4.801 | 1.983 | 0.677 | 1.640 | 113,840 | 43,932 |
| RFC 0043 long prefix | 904,698 | 5.307 | 1.895 | 0.613 | 1.716 | 120,092 | 43,932 |

Sixteen Calgary files shrink. Paper4 and paper5 are byte-identical.
Book2 accounts for 781 of the 1,751 saved bytes. The saving is
0.193%. The required decrease was 2,267 bytes (0.25%), so the
candidate stops before Canterbury, wrapped selection, and enwik9.

These timings sum per-file wall times and include startup and
file output. Peak RSS is the maximum over the file runs. Times are
single-run screens and do not establish a decode speed change.
Exact comparison follows timed decoding.

The unwrapped candidate is 18,838 bytes above xz -9 on Calgary.
The retained Calgary selection with optional byte shuffle is
893,989 bytes, 8,129 above xz -9. The retained enwik9 artifact is
243,396,020 bytes, 27,721,824 above xz -9 and 46,396,020 above
the nominal 197 MB M2 ceiling.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/measure.py --timing enc.tsv --stdout enc.out \
  target/release/tak-lz-stream encode-variable-order-repriced INPUT ARTIFACT
python3 benchmarks/runners/measure.py --timing dec.tsv --stdout dec.out \
  target/release/tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES
cmp INPUT RESTORED
```

Run each profile independently on all Calgary files. Use
`encode-variable-order-long-prefix` for the candidate. Record the
full length and SHA-256 of every artifact and input. Compare the
input hashes against the RFC 0034 raw records.

Source is `6f777c8` plus RFC 0043. Linux 6.18.57, Intel Xeon Platinum
8573C, x86-64, three visible logical CPUs, 10,207,480 KiB visible
RAM, and an 8 GiB cgroup limit. Rust 1.85.0 release build,
Python 3.12.14, and one codec thread.

[Raw Calgary results](../../benchmarks/reports/raw/2026-11-25-long-prefix-match-index-calgary.tsv)
SHA-256 `2f7d21c685a0ccccce124678483dfc0fe4e1b377e1eeaeba4f65b9adbf82a919`.
