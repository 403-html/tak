# Fixed-width byte shuffle

RFC 0038 tested widths 2, 4, and 8 against the RFC 0034 encoder.
The selector used complete artifact lengths, including the nine-byte
header on every nonidentity stream. All 112 Calgary and Canterbury
trials restored exactly. Canterbury excludes ptt5, a byte-for-byte
duplicate of Calgary pic.

| Corpus | Input bytes | Control bytes | Selected bytes | Saved bytes | File with a gain |
|---|---:|---:|---:|---:|---|
| Calgary | 3,251,493 | 906,449 | 893,989 | 12,460 (1.375%) | geo, width 4 |
| Independent Canterbury | 2,297,568 | 590,818 | 584,448 | 6,370 (1.078%) | kennedy.xls, width 2 |

Calgary geo falls from 66,558 to 54,098 bytes. It remains 726 bytes
above xz -9 on that file. Canterbury kennedy.xls falls from 212,808
to 206,438 bytes. Ordinary TK32 is smallest on every other input.
This result supports byte shuffle for some multi-byte data. The
experiment covers only two files with positive gains.

| Corpus and selection | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encoder KiB | Peak decoder KiB |
|---|---:|---:|---:|---:|---:|---:|
| Calgary control | 6.976 | 3.377 | 0.466 | 0.963 | 113,424 | 44,064 |
| Calgary selected | 6.766 | 3.356 | 0.481 | 0.969 | 121,464 | 48,544 |
| Canterbury control | 4.395 | 1.936 | 0.523 | 1.187 | 105,592 | 38,560 |
| Canterbury selected | 4.527 | 2.011 | 0.508 | 1.142 | 166,592 | 69,148 |

The selected rows sum one encode and one decode per input. The full
width search spent 32.393 s on Calgary and 17.472 s on Canterbury,
or 0.100 and 0.131 MB/s over the original inputs. Peak encoder RSS
including all trials was 122,748 and 166,592 KiB. These are
single-run measurements of the initial buffered wrapper. Timing
includes subprocess startup and file output. Exact comparisons
took place outside timing.

## enwik9

Each column totals complete sample artifacts over four fixed 1 MiB
windows chosen before measurement. All 16 runs restored exactly.

| Width | 0 (identity) | 2 | 4 | 8 |
|---|---:|---:|---:|---:|
| Bytes | 1,008,806 | 1,538,033 | 1,955,551 | 2,177,149 |

The sampling policy selects identity. The retained full file is the
243,396,020-byte RFC 0034 artifact. Selective byte shuffle has no
measured compression gain on enwik9 under this policy. The nonzero
widths were measured only on the samples. The original artifact took
1,315.45 s to encode at 0.76 MB/s with 256,844 KiB peak RSS. That measured
encode was reused. A fresh decode and streaming comparison restored
all 1,000,000,000 bytes exactly in 447.10 s, or 2.24 MB/s, with 113,952
KiB peak RSS. Workspace checks overlapped the end of the sample screen
and beginning of the full verification. Some wrapper checks also ran
during verification. These timings do not establish a speed change.

[Original complete encode measurement](2026-11-25-tk32-literal-repricing.md)

Calgary is 8,129 bytes above xz -9. The enwik9 artifact is 8,067,081
bytes above Zstandard -19, 27,721,824 above xz -9, and 46,396,020
above the nominal 197 MB M2 ceiling.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/byte_shuffle.py screen target/release/tak-lz-stream CALGARY
python3 benchmarks/runners/byte_shuffle.py screen target/release/tak-lz-stream CANTERBURY_WITHOUT_PTT5
```

The current runner uses the later file-based wrapper. That
implementation preserves the transform and wire bytes. It copies
64 KiB at a time and uses temporary disk space for the inner stream
and transformed data. It reproduced the SHA-256 of both winning
artifacts. Checks also covered block boundary lengths, a truncated
header, an unknown width, a bad original checksum, and a lower output
limit. Full width search requires up to four encodes
per input, and is currently a research procedure.

To reproduce the sample run, extract 1,048,576 bytes from each
of the four starting offsets 0, 249,954,304, 499,974,144, and
749,993,984, then run `screen` over those four files. Verify the
sample digests in the raw rows. Verify the full input and retained
artifact hashes before comparing decoded output. Measure this pipeline:

```console
bash -o pipefail -c 'target/release/tak-lz-stream decode-variable-order enwik9.tk32 - --max-output 1000000000 | cmp enwik9 -'
```

Source is commit `b9ead50` plus RFC 0038. Linux 6.18.57, Intel
Xeon Platinum 8573C, x86-64, three visible logical CPUs,
10,207,480 KiB visible RAM, and an 8 GiB cgroup limit.
Rust 1.85.0 release build and Python 3.12.14. One codec thread
and one run per file and width. The small screening wrapper and its
bounded successor have no TAK-C method identifier.

- [Raw Calgary rows](../../benchmarks/reports/raw/2026-11-25-byte-shuffle-calgary.tsv)
- [Raw Canterbury rows](../../benchmarks/reports/raw/2026-11-25-byte-shuffle-canterbury.tsv)
- [Raw enwik9 sample rows](../../benchmarks/reports/raw/2026-11-25-byte-shuffle-enwik9-samples.tsv)
- [Raw full enwik9 decode and comparison](../../benchmarks/reports/raw/2026-11-25-byte-shuffle-enwik9.tsv)
