# Shorter match candidate from the chain

RFC 0047 gives the parser the last two length records from the
existing bounded chain. The complete TK32 syntax remains the same.
All 84 control and candidate streams restored byte for byte.

| Corpus and profile | Complete bytes | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encoder KiB | Peak decoder KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| Calgary RFC 0034 | 906,449 | 6.134 | 2.533 | 0.530 | 1.284 | 114,452 | 43,936 |
| Calgary RFC 0046 | 903,055 | 7.029 | 2.536 | 0.463 | 1.282 | 118,564 | 44,068 |
| Calgary RFC 0047 | 903,047 | 6.908 | 2.339 | 0.471 | 1.390 | 118,452 | 43,936 |
| Canterbury RFC 0034 | 590,818 | 4.173 | 1.841 | 0.551 | 1.248 | 106,620 | 38,432 |
| Canterbury RFC 0046 | 589,827 | 4.738 | 1.656 | 0.485 | 1.387 | 110,632 | 38,436 |
| Canterbury RFC 0047 | 589,826 | 4.562 | 1.476 | 0.504 | 1.556 | 110,720 | 38,560 |

The candidate saves eight bytes over RFC 0046 on Calgary. Book2
saves eight and pic saves one. Paper2 grows by one. On independent
Canterbury, alice29.txt and plrabn12.txt each save one byte, and
lcet10.txt grows by one. The one-byte Canterbury gain falls below
the 295-byte incremental threshold. Calgary falls below its
452-byte threshold. The experiment stops before byte shuffle and
enwik9.

Times sum per-file runs with one codec thread. Peak RSS is the
largest per-file measurement. The timings include startup and file
output, with exact comparisons outside the timed command. These
single-run screens do not establish a speed difference. The final
build reproduces every size and SHA-256 from the initial screen.
Both control profiles also reproduce the archived RFC 0046 hashes.

The unwrapped candidate is 17,187 bytes above xz -9 on Calgary.
The retained byte-shuffle selection is 893,989 bytes, 8,129 bytes
above xz -9. The retained enwik9 artifact is 243,396,020 bytes,
27,721,824 above xz -9 and 46,396,020 above the nominal 197 MB M2
threshold.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/measure.py --timing enc.tsv --stdout enc.out \
  target/release/tak-lz-stream encode-variable-order-chain-shorter INPUT ARTIFACT
python3 benchmarks/runners/measure.py --timing dec.tsv --stdout dec.out \
  target/release/tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES
cmp INPUT RESTORED
```

Run each file independently. Repeat with
`encode-variable-order-repriced` and `encode-variable-order-match-chain`
as controls. Record the size and SHA-256 of every artifact. Verify
input digests against RFC 0046. Exclude Canterbury ptt5 after
confirming byte identity with Calgary pic.

Source is `e3ab106` plus RFC 0047. Linux 6.18.57 on Intel Xeon
Platinum 8573C, x86-64, three visible logical CPUs, 10,207,480 KiB
visible RAM, and an 8 GiB cgroup limit. Rust 1.85.0 release build
and Python 3.12.14 benchmark runner. The initial screen preceded a
parser refactor. The table uses the final-build runs.

- [Final Calgary rows](../../benchmarks/reports/raw/2026-11-25-chain-shorter-match-calgary.tsv)
- [Final Canterbury rows](../../benchmarks/reports/raw/2026-11-25-chain-shorter-match-canterbury.tsv)
- [Initial screen](../../benchmarks/reports/raw/2026-11-25-chain-shorter-match-initial.tsv)
