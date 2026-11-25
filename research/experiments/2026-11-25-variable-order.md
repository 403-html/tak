# Bounded variable-order literal prediction

On Calgary, the singleton-weighted model saved 180,836.692 ideal bits
across 1,895,245 literals. It used longer rows on 917,400 literals.
The interpolated distinct-count control added 80,268.045 bits. Both
models use identical counts and the same parse. This isolates the
backoff weight. The diagnostic's peak RSS was 88,236 KiB.

## Complete artifacts

| Corpus | RFC 0019 bytes | TK32 bytes | Savings | Files improved |
|---|---:|---:|---:|---:|
| Calgary | 942,454 | 918,436 | 24,018 (2.55%) | 16/18 |
| Independent Canterbury | 623,811 | 597,600 | 26,211 (4.20%) | 10/10 |

Every decode matched its original byte for byte. The independently
compressed Calgary comparison is 32,576 bytes above xz at 885,860.
It is 2,637 bytes below the recorded Zstandard-19 total of 921,073.
The remaining reduction needed to beat xz is 3.55% of the TK32 total.
Calgary's geo and pic regressed. Canterbury's ptt5 is identical to
Calgary's pic and is excluded from the independent aggregate.
TK32 beats xz on five Calgary files. The three largest remaining
deficits are obj2 (16,097 bytes), geo (13,192), and pic (10,066).

| Corpus and model | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encode KiB | Peak decode KiB |
|---|---:|---:|---:|---:|---:|---:|
| Calgary RFC 0019 | 2.494 | 0.611 | 1.303 | 5.324 | 77,560 | 26,016 |
| Calgary TK32 | 3.987 | 2.849 | 0.815 | 1.141 | 95,476 | 44,044 |
| Canterbury RFC 0019 | 1.326 | 0.437 | 1.733 | 5.263 | 69,072 | 20,512 |
| Canterbury TK32 | 1.999 | 1.661 | 1.149 | 1.383 | 87,100 | 38,540 |

Times sum independent one-repetition screens and include process
startup. The binary search over composed cumulative ranges increases
decode work. These measurements show a size-speed tradeoff.

## enwik9

| Model | Artifact bytes | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encode KiB | Peak decode KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| RFC 0019, recorded | 249,762,881 | 628.62 | 111.88 | 1.59 | 8.94 | 220,912 | 96,024 |
| TK32 | 246,642,016 | 798.17 | 451.45 | 1.25 | 2.22 | 238,692 | 113,824 |

The complete TK32 stream saves 3,120,865 bytes (1.25%). It restores
all 1,000,000,000 bytes exactly. The TK32 decode measurement includes
the streaming comparison process. These are separate single-run screens.
The original input digest matches the recorded RFC 0019 input.

The remaining enwik9 gaps are 11,313,077 bytes to Zstandard-19,
30,967,820 bytes to xz, and 49,642,016 bytes to nominal M2. Matching
xz requires a 12.56% reduction from TK32. The older RFC 0019 result
retains higher measured throughput.

To reproduce the enwik9 measurement, use the same encode command above
on enwik9. Run the decode comparison with pipeline failure detection
so both commands must succeed:

```console
bash -o pipefail -c 'target/release/tak-lz-stream decode-variable-order enwik9.tk32 - --max-output 1000000000 | cmp enwik9 -'
```

[Raw enwik9 row](../../benchmarks/reports/raw/2026-11-25-variable-order-enwik9.tsv)

## Reproduction

Build both binaries with
`cargo build --release -p tak-tensor --bin tak-match-distance-literal --bin tak-lz-stream`.
For the opportunity screen run
`tak-match-distance-literal --variable-order INPUT` per Calgary file.
For the artifact screen run both
`tak-lz-stream encode-structural INPUT ARTIFACT` and
`tak-lz-stream encode-variable-order INPUT ARTIFACT` on each file.
Use the corresponding decode mode with `--max-output INPUT_BYTES`.
Measure each operation using `benchmarks/runners/measure.py`, then
compare restored files with `cmp`. The row table lists all input hashes.

Source is `287ad58` plus RFC 0032. The opportunity run preceded the
addition of the grouped prefix cache, which changes the row storage
but leaves the diagnostic probabilities intact. Linux 6.18.57 on
an x86-64 Intel Xeon Platinum 8573C with three visible logical CPUs
and 10,207,480 KiB visible RAM. Rust 1.85.0 release build. Python 3.12.14
runner. One thread and no experiment-specific memory cap.

- [Opportunity rows](../../benchmarks/reports/raw/2026-11-25-variable-order-opportunity.tsv)
  SHA-256 `5ad6b50b6f6fc23c00d615a4e917cc224062264a2223f691b878a738c960463f`
- [Artifact rows](../../benchmarks/reports/raw/2026-11-25-variable-order-artifacts.tsv)
  SHA-256 `604f29fb5275022927ecb632087d98cb628c851687ade81c7bacecc0e737d948`
