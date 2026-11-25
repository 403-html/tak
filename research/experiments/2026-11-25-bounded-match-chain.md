# Bounded chain of exact match keys

RFC 0046 follows earlier occurrences of each exact eight-byte key
within the preceding 1 MiB. The chain contributes an extra candidate
only when it finds a longer match than the two ordinary sources.
The control and candidate emit complete TK32 streams. All 56 streams
restored byte for byte.

| Corpus and profile | Complete bytes | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encoder KiB | Peak decoder KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| Calgary control | 906,449 | 5.528 | 2.150 | 0.588 | 1.512 | 114,428 | 43,936 |
| Calgary match chain | 903,055 | 6.091 | 2.177 | 0.534 | 1.494 | 118,624 | 43,936 |
| Canterbury control | 590,818 | 3.558 | 1.493 | 0.646 | 1.539 | 106,644 | 38,456 |
| Canterbury match chain | 589,827 | 4.139 | 1.503 | 0.555 | 1.529 | 110,748 | 38,432 |

Calgary saves 3,394 bytes (0.374%). Fifteen files shrink and
three keep the same size. The largest gain is 1,248 bytes on
book2. The candidate is 1,009 bytes below the RFC 0045 combined
search aggregate on Calgary.

Independent Canterbury saves 991 bytes (0.168%). Eight files
shrink and two keep the same size. Lcet10.txt accounts for 753
bytes of the gain. The chain is 187 bytes below the RFC 0045
combined search aggregate. The 0.25% transfer gate requires
1,478 bytes, so the result stops before byte-shuffle selection
and enwik9. The table uses single-run screening timings.
Times sum individual runs, peak RSS is the maximum over files,
and exact comparisons follow timed decoding. A final-build rerun
after extracting a helper reproduced all 56 artifact hashes and
exact restorations. Both control sets match the RFC 0045 hashes.

The unwrapped candidate is 17,195 bytes above xz -9 on Calgary.
The retained Calgary selection with byte shuffle is 893,989 bytes,
8,129 above xz -9. The retained enwik9 artifact is 243,396,020
bytes, 27,721,824 above xz -9 and 46,396,020 above nominal M2.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/measure.py --timing enc.tsv --stdout enc.out \
  target/release/tak-lz-stream encode-variable-order-repriced INPUT ARTIFACT
python3 benchmarks/runners/measure.py --timing dec.tsv --stdout dec.out \
  target/release/tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES
cmp INPUT RESTORED
```

Run both profiles independently on each input. Use
`encode-variable-order-match-chain` for the candidate.
Exclude ptt5 after comparing it byte for byte with Calgary pic.
Record input and complete artifact size and SHA-256.

Source is `f518185` plus RFC 0046. Linux 6.18.57, Intel Xeon
Platinum 8573C, x86-64, three visible logical CPUs, 10,207,480
KiB visible RAM, and an 8 GiB cgroup limit. Rust 1.85.0 release
build and Python 3.12.14. One codec thread and one repetition.

- [Raw Calgary rows](../../benchmarks/reports/raw/2026-11-25-bounded-match-chain-calgary.tsv)
- [Raw Canterbury rows](../../benchmarks/reports/raw/2026-11-25-bounded-match-chain-canterbury.tsv)
- [Raw final-build recheck](../../benchmarks/reports/raw/2026-11-25-bounded-match-chain-recheck.tsv)
