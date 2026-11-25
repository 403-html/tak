# Combined longer-match searches

RFC 0045 tests the RFC 0043 sixteen-byte index, RFC 0044 shifted
lookup, and their combination against the RFC 0034 control.
All 112 complete TK32 streams restore byte for byte. The three
previous Calgary profiles reproduce their original artifact hashes.

| Corpus and profile | Complete bytes | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encoder KiB | Peak decoder KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| Calgary control | 906,449 | 5.351 | 1.969 | 0.608 | 1.652 | 114,332 | 43,940 |
| Calgary sixteen-byte index | 904,698 | 5.950 | 2.032 | 0.546 | 1.600 | 120,600 | 43,932 |
| Calgary shifted lookup | 905,033 | 5.294 | 2.042 | 0.614 | 1.592 | 114,500 | 44,064 |
| Calgary combined | 904,064 | 6.580 | 1.927 | 0.494 | 1.687 | 120,496 | 43,936 |
| Canterbury control | 590,818 | 3.253 | 1.381 | 0.706 | 1.663 | 106,608 | 38,432 |
| Canterbury sixteen-byte index | 590,201 | 3.648 | 1.316 | 0.630 | 1.746 | 112,768 | 38,452 |
| Canterbury shifted lookup | 590,326 | 3.210 | 1.338 | 0.716 | 1.717 | 106,760 | 38,560 |
| Canterbury combined | 590,014 | 3.816 | 1.352 | 0.602 | 1.700 | 112,892 | 38,432 |

The combined Calgary saving is 2,385 bytes (0.263%) against
the control and 634 bytes (0.070%) against the best standalone
aggregate. Both thresholds pass. The aggregate saves 782 fewer
bytes than the sum of the two standalone savings. It also beats
the sum of the smallest standalone result on each file by 467
bytes. Together these comparisons show a benefit from combining
the searches and quantify how much of the separate savings carry
over. They do not count overlapping match events.

Canterbury saves 804 bytes (0.136%) against the control and 187
bytes (0.032%) against the better standalone search. Both fall
below the 0.25% and 0.05% transfer gates. The combination saves
the most bytes on both corpora, with more total encode time than
the control in this screen. Timing uses one run per file and
profile, including process startup and file output. RSS is the
maximum per-file value. The exact comparison runs outside timing.

The unwrapped combination is 18,204 bytes above xz -9 on Calgary.
The retained Calgary profile with optional byte shuffle is 893,989
bytes, 8,129 above xz -9. The combination was not measured with
the byte-shuffle selection or on enwik9. The retained enwik9
artifact is 243,396,020 bytes, 27,721,824 above xz -9 and
46,396,020 above the nominal 197 MB M2 ceiling.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/measure.py --timing enc.tsv --stdout enc.out \
  target/release/tak-lz-stream encode-variable-order-repriced INPUT ARTIFACT
python3 benchmarks/runners/measure.py --timing dec.tsv --stdout dec.out \
  target/release/tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES
cmp INPUT RESTORED
```

For each input also use `encode-variable-order-long-prefix`,
`encode-variable-order-shifted-anchor`, and
`encode-variable-order-combined-search`. Verify complete artifact
sizes and SHA-256. Exclude Canterbury ptt5 after comparing it
byte for byte with Calgary pic. Input lengths and digests match
the recorded RFC 0034 corpora.

Source is `a13a2a2` plus RFC 0045. Linux 6.18.57, Intel Xeon
Platinum 8573C, x86-64, three visible logical CPUs, 10,207,480 KiB
visible RAM, and an 8 GiB cgroup limit. Rust 1.85.0 release build,
Python 3.12.14, and one codec thread.

- [Raw Calgary rows](../../benchmarks/reports/raw/2026-11-25-combined-match-search-calgary.tsv)
- [Raw Canterbury rows](../../benchmarks/reports/raw/2026-11-25-combined-match-search-canterbury.tsv)
