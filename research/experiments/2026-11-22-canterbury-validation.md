# Rejected held-out validation: mature third-order backoff

## Status

Threshold 64 fails independent validation on the Canterbury corpus. Preserve
the RFC 0004 development result, but do not promote the model to TAK-C or use
it to justify a tensor implementation.

## Predeclared hypothesis and gate

The threshold was fixed at 64 before this experiment. Canterbury was not used
to design the model or choose the threshold.

The corpus files were assigned to three groups before measurement:

- prose: `alice29.txt`, `asyoulik.txt`, `lcet10.txt`, `plrabn12.txt`;
- source/markup: `cp.html`, `fields.c`, `grammar.lsp`, `xargs.1`; and
- binary: `kennedy.xls`, `ptt5`, `sum`.

Each file is a separate complete artifact. Group size is the sum of those
artifacts, including every 28-byte header. Validation requires backoff to beat
order-2 by at least 2% on two of three groups, decode no more than two times
slower, use less than one gigabyte, and restore every file exactly.

## Dataset and environment

- Source: `https://corpus.canterbury.ac.nz/resources/cantrbry.zip`.
- Archive SHA-256: `c44b686dfc137e74aba4db0540e5d6568cb09e270ba8f8411d2f9df24f39a1a6`.
- License: the downloaded archive contains no license file. The corpus is used
  only as an external benchmark and is not redistributed by TAK.
- TAK commit: `7dacc32`.
- Date: 2026-11-22 UTC.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Memory: 10,207,480 KiB available; no lower external limit imposed.
- Compiler: Rust 1.85.0.
- Threads: one.
- Repetitions: one warm-up and five measured runs per file.
- Time statistic: median of the summed per-file times for each group.
- Memory statistic: maximum complete-process resident memory.
- Verification: every decoder output compared byte for byte.

| Input | Group | Bytes | SHA-256 |
|---|---|---:|---|
| alice29.txt | prose | 152,089 | `7467306ee0feed4971260f3c87421154a05be571d944e9cb021a5713700c38f0` |
| asyoulik.txt | prose | 125,179 | `eaa3526fe53859f34ecdf255712f9ecf0b2c903451d4755b2edaa2e2599cb0fc` |
| lcet10.txt | prose | 426,754 | `5314ba1dbb03f471df88bec6cd120a938ef60d0fd3511c5c1dce61bf7463245f` |
| plrabn12.txt | prose | 481,861 | `07e2e0b461af78c7c647cb53dab39de560198e16f799b4516eccf0fbd69f764c` |
| cp.html | source/markup | 24,603 | `e0cd21cef5b6c4069461e949be100080c3ce887de6f1dd8626c480528efaaf61` |
| fields.c | source/markup | 11,150 | `85d73e354cc50cec76cb5a50537cf8dc035f8cbb8480f9e1cbe2f7d6c23393c7` |
| grammar.lsp | source/markup | 3,721 | `1b0805dfc0ae706b35aac2bb4e15f02485efd24dda5dbd29de7b2f84d1a88c15` |
| xargs.1 | source/markup | 4,227 | `c58aeb5d2d1e12751d47e7412b45784405fc30a5671b03d480fa05776e183619` |
| kennedy.xls | binary | 1,029,744 | `9af47239ca29dfe20e633f80bbbb9a4cc9783d0803d7b2b5626f42e4c3790420` |
| ptt5 | binary | 513,216 | `0ec3a75089bb52342813496b17e51377bc9eba3cb519a444d67025354841d650` |
| sum | binary | 38,240 | `ee5733cd76ecc2f9d8ff156adc3c02a7a851051dcf43a2d56ff4ee4ff606bdb3` |

Raw repetitions are stored in:

- `benchmarks/reports/raw/2026-11-22-canterbury-tak.tsv`; and
- `benchmarks/reports/raw/2026-11-22-canterbury-external.tsv`.

The measured commands were:

```console
target/release/tak-order3-backoff-gate INPUT --threshold 64
gzip -9 -n -c INPUT
gzip -d -c STREAM
xz -9 -c INPUT
xz -d -c STREAM
zstd -q -19 -c INPUT
zstd -q -d -c STREAM
```

## TAK validation results

Negative reduction means that backoff is larger than order-2.

| Group | Input bytes | Order-2 bytes | Backoff bytes | Size reduction | Order-2 encode (s) | Backoff encode (s) | Order-2 decode (s) | Backoff decode (s) | Decode slowdown | Peak RSS (KiB) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| prose | 1,185,883 | 534,817 | 551,651 | -3.1% | 0.108 | 0.187 | 0.133 | 0.192 | 1.44x | 8,448 |
| source/markup | 43,701 | 29,568 | 29,600 | -0.1% | 0.006 | 0.009 | 0.007 | 0.009 | 1.35x | 8,448 |
| binary | 1,581,200 | 357,638 | 363,906 | -1.8% | 0.079 | 0.181 | 0.088 | 0.197 | 2.25x | 16,808 |
| total | 2,810,784 | 922,023 | 945,157 | -2.5% | 0.193 | 0.381 | 0.229 | 0.413 | 1.80x | 16,808 |

Aggregate backoff throughput was 7.03 MiB/s for encoding and 6.49 MiB/s for
decoding. Order-2 achieved 13.87 MiB/s and 11.70 MiB/s. All 110 TAK decodes
restored their file exactly.

## External size and performance context

Each external method also compressed every file independently. Versions were
gzip 1.13, XZ Utils 5.8.1, and Zstandard 1.5.7.

| Method | Complete bytes | Ratio | Compress (s) | Decompress (s) | Compress (MiB/s) | Decompress (MiB/s) | Peak RSS (KiB) |
|---|---:|---:|---:|---:|---:|---:|---:|
| TAK order-2 | 922,023 | 32.80% | 0.193 | 0.229 | 13.87 | 11.70 | 16,808 |
| TAK backoff-64 | 945,157 | 33.63% | 0.381 | 0.413 | 7.03 | 6.49 | 16,808 |
| gzip -9 | 730,625 | 25.99% | 0.830 | 0.047 | 3.23 | 56.93 | 9,216 |
| xz -9 | 492,364 | 17.52% | 1.462 | 0.066 | 1.83 | 40.92 | 44,672 |
| zstd -19 | 516,401 | 18.37% | 1.419 | 0.043 | 1.89 | 61.75 | 20,480 |

TAK times are the in-process model operation times reported by the research
runner. External times include command startup for 11 independent streams.
Therefore, compare the timing rows only with this accounting warning. Stream
sizes use matching independent-file accounting and are directly comparable.

All 165 external decodes restored their file exactly.

## Distance to project goals

The enwik9 milestones remain unmeasured. Do not extrapolate a Canterbury result
to enwik9.

As a concrete scale example, the held-out TAK backoff ratio is 33.63%. The M1
enwik9 target corresponds to 32.2%. Those numbers look only 1.43 percentage
points apart, but gzip reaches 25.99% on this same held-out corpus. TAK backoff
is 214,532 bytes, or 29.4%, larger than gzip under matching size accounting.

TAK backoff is 92.0% larger than xz on the same inputs. To equal gzip here, it
would need to remove 22.7% of its current compressed bytes. To equal xz, it
would need to remove 47.9%.

This is not evidence that M1 is close. It is evidence that a raw ratio from a
different corpus can be misleading without same-data baselines.

## Decision

Reject threshold 64 as a general model. It fails the predeclared size gate in
all three held-out groups and the binary speed gate. The Silesia improvement
does not generalize.

Do not retune the threshold on Canterbury and call that validation. A later
proposal must state a new mechanism and use another untouched validation
corpus. The simplest current classical control remains adaptive order-2.
