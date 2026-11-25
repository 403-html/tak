# Distance prices for alternative chain matches

RFC 0048 selects one extra match by comparing distance prices
at a shared length. For a fixed start and end, the match type,
length cost, and decoded bytes are equal across sources. This
lets the encoder compare sources by distance price alone under
the frozen model. The complete stream still depends on the chosen
parse and later adaptive updates. All 84 streams restored exactly.

| Corpus and profile | Complete bytes | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encoder KiB | Peak decoder KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| Calgary RFC 0034 | 906,449 | 6.638 | 2.063 | 0.490 | 1.576 | 114,440 | 43,972 |
| Calgary RFC 0046 | 903,055 | 7.317 | 2.248 | 0.444 | 1.446 | 118,564 | 43,936 |
| Calgary RFC 0048 | 903,050 | 6.210 | 2.316 | 0.524 | 1.404 | 118,564 | 43,936 |
| Canterbury RFC 0034 | 590,818 | 3.156 | 1.287 | 0.728 | 1.786 | 106,748 | 38,564 |
| Canterbury RFC 0046 | 589,827 | 3.543 | 1.308 | 0.649 | 1.756 | 110,724 | 38,468 |
| Canterbury RFC 0048 | 589,807 | 3.734 | 1.333 | 0.615 | 1.723 | 110,752 | 38,432 |

The candidate saves 5 bytes over RFC 0046 on Calgary and 20 on
independent Canterbury. Book2 saves seven Calgary bytes and news
saves one. Book1, obj2, and paper2 each grow by one byte. On
Canterbury, kennedy.xls saves 20 bytes. Two text files each save
one byte and lcet10.txt grows by two. RFC 0047's length-only
alternative saved eight and one byte on the same corpora.

Both gains miss the incremental 0.05% gate. The experiment stops
before wrapped selection and enwik9. The unwrapped Calgary
candidate is 17,190 bytes above xz -9. The retained Calgary
result with byte shuffle is 893,989 bytes, 8,129 above xz -9.
The retained enwik9 artifact is 243,396,020 bytes, 27,721,824 above
xz -9 and 46,396,020 above the 197 MB M2 threshold.

Times sum one run per file with one codec thread. Peak RSS is
the maximum per-file value. Timing includes startup and file
output. Exact comparisons occur after timed decoding. The first
Canterbury screen overlapped workspace checks. The table uses
the final release build without that overlap. Every artifact
hash and exact restoration reproduced on the final build.
Both control profiles match their archived RFC 0046 digests.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/measure.py --timing enc.tsv --stdout enc.out \
  target/release/tak-lz-stream encode-variable-order-chain-price INPUT ARTIFACT
python3 benchmarks/runners/measure.py --timing dec.tsv --stdout dec.out \
  target/release/tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES
cmp INPUT RESTORED
```

Use both `encode-variable-order-repriced` and
`encode-variable-order-match-chain` as controls. Run each file
independently. Verify input lengths and SHA-256 against RFC 0046.
Exclude Canterbury ptt5 after comparing it against Calgary pic.
Measure complete artifact sizes and SHA-256.

Source is `0dfc8cd` plus RFC 0048. Linux 6.18.57, Intel Xeon
Platinum 8573C, x86-64, three visible logical CPUs, 10,207,480 KiB
visible RAM, and an 8 GiB cgroup limit. Rust 1.85.0 release build
and Python 3.12.14.

- [Final Calgary rows](../../benchmarks/reports/raw/2026-11-25-chain-distance-price-calgary.tsv)
- [Final Canterbury rows](../../benchmarks/reports/raw/2026-11-25-chain-distance-price-canterbury.tsv)
- [Initial screen](../../benchmarks/reports/raw/2026-11-25-chain-distance-price-initial.tsv)
