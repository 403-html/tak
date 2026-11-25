# Shifted match anchor

RFC 0044 searches the existing eight-byte index using the key
four bytes ahead. It verifies the full initial prefix before
measuring an extra match. Both encoders write ordinary TK32.
The decoder restored every byte in all 36 Calgary trials.

| Calgary profile | Complete bytes | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encoder KiB | Peak decoder KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| RFC 0034 control | 906,449 | 4.597 | 1.818 | 0.707 | 1.788 | 113,912 | 43,936 |
| RFC 0044 shifted anchor | 905,033 | 5.210 | 1.806 | 0.624 | 1.800 | 113,944 | 43,936 |

Sixteen files get smaller. Book1 saves 358 bytes and book2 saves
611 bytes. The 1,416-byte aggregate decrease falls below the
2,267-byte gate. The result is 335 bytes larger than the recorded
RFC 0043 sixteen-byte-index aggregate. RFC 0044 uses the existing
index, so only its candidate discovery and parser decisions change.
The screen stops before Canterbury, wrapped selection, and enwik9.

Times sum independent per-file runs, including startup and output.
Peak RSS is the maximum per-file value. Each pair is one screening
run. Exact byte comparison follows timed decoding.

The unwrapped candidate is 19,173 bytes above xz -9 on Calgary.
The retained Calgary selection is 893,989 bytes, 8,129 bytes above
xz -9. The retained enwik9 artifact is 243,396,020 bytes,
27,721,824 above xz -9 and 46,396,020 above nominal M2.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/measure.py --timing enc.tsv --stdout enc.out \
  target/release/tak-lz-stream encode-variable-order-repriced INPUT ARTIFACT
python3 benchmarks/runners/measure.py --timing dec.tsv --stdout dec.out \
  target/release/tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES
cmp INPUT RESTORED
```

Run both profiles on each Calgary input independently. Use
`encode-variable-order-shifted-anchor` for the candidate. Record
the full artifact length and SHA-256, and verify the input digest
against the RFC 0034 measurements.

Source is `25cc40b` plus RFC 0044. Linux 6.18.57, Intel Xeon Platinum
8573C, x86-64, three visible logical CPUs, 10,207,480 KiB visible
RAM, and an 8 GiB cgroup limit. Rust 1.85.0 release build,
Python 3.12.14, and one codec thread.

[Raw Calgary results](../../benchmarks/reports/raw/2026-11-25-shifted-match-anchor-calgary.tsv)
SHA-256 `b0a46bde27611633cf53cd127107a5ece435178a9acd3a360c55ae5461715368`.
