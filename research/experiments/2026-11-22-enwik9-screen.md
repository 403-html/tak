# Completed screen: RFC 0006 on enwik9

## Status

Completed as predeclared. Fixed strength 4 does not reach M1. Preserve this as
a non-publishable single-sample screen.

## Objective

Measure fixed RFC 0006 strength 4 on the project's 1,000,000,000-byte enwik9
reference input. Do not tune the model or strength from this result.

Compare complete `TKN2` and `TKO2` research artifacts. Also measure gzip 1.13
at level 9, XZ Utils 5.8.1 at level 9, and Zstandard 1.5.7 at level 19. Include
all headers and decoder metadata in physical stream size.

## Measurements

Record:

- input and download SHA-256 digests;
- complete compressed bytes and input ratio;
- wall-clock compression and decompression time;
- throughput;
- peak complete-process resident memory;
- thread count and environment; and
- exact byte-for-byte restoration.

Run one warm-up only if its cost is practical. Run one measured sample for this
initial screen. Label the results non-publishable because five repetitions are
not planned at this stage.

The current TAK runner reads the input, artifacts, and restored output into
memory. Its peak process memory is not an isolated decoder measurement. Report
it conservatively and do not claim bounded streaming memory from this run.

## Goal accounting

Compare the complete TAK size directly with these project milestones:

| Milestone | Maximum size |
|---|---:|
| M1 | 322,000,000 bytes |
| M2 | 197,000,000 bytes |
| M3 | 130,000,000 bytes |
| M4 | 110,000,000 bytes |
| M5 | 89,000,000 bytes |

For each missed milestone, report both excess bytes and the percentage of the
current TAK artifact that must be removed. Do not substitute a ratio measured
on another corpus.

## Stop condition

Preserve the result whether positive or negative. Do not promote `TKN2` to
TAK-C from one enwik9 screen. If resource use prevents completion, record the
failure and identify the required streaming change before retrying.

## Dataset and environment

- Source: `https://mattmahoney.net/dc/enwik9.zip`.
- Download SHA-256: `99cdb5ac84392252d3f0912ccedd195bc95bd80cbef3b0cdf2eee4ad9a3b7a51`.
- Input SHA-256: `159b85351e5f76e60cbe32e04c677847a9ecba3adc79addab6f4c6c7aa3744bc`.
- Input size: 1,000,000,000 bytes.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one for TAK and the selected external commands.
- Date: 2026-11-23 Europe/Warsaw; measurement started 2026-11-22 UTC.
- Samples: one measured run; no warm-up because of cost.

Raw result files and SHA-256 digests:

- `benchmarks/reports/raw/2026-11-22-enwik9-input.tsv`:
  `15bc711000863188d6708b90f4c1e2701c0e4606cf851676a00ad01b78bce5f2`;
- `benchmarks/reports/raw/2026-11-22-enwik9-tak.tsv`:
  `81d0fea8a24809d0a9407dd8b661aeb1fc368734f4b052218fa261bbe3880716`;
  and
- `benchmarks/reports/raw/2026-11-22-enwik9-external.tsv`:
  `5f9a06986ea7519189a76e631017eafa0ab7d4bae1005b4b2230da56964efd0a`.

## Size results

| Method | Complete bytes | Input ratio | Difference from TAK |
|---|---:|---:|---:|
| TAK order-2 | 371,890,796 | 37.19% | 3,247,794 larger |
| TAK normalized strength 4 | 368,643,002 | 36.86% | — |
| gzip 1.13 `-9 -n` | 322,591,988 | 32.26% | 46,051,014 smaller |
| XZ Utils 5.8.1 `-9` | 215,674,196 | 21.57% | 152,968,806 smaller |
| Zstandard 1.5.7 `-19` | 235,328,939 | 23.53% | 133,314,063 smaller |

Strength 4 improves the complete order-2 artifact by only 0.87% on enwik9.
This is much less than its development and Calgary gains. On a long stream,
order-2 contexts accumulate enough local evidence that cold-context smoothing
has less opportunity to help.

## Time and memory

| Method | Encode (s) | Encode (MiB/s) | Decode (s) | Decode (MiB/s) | Encode/decode peak RSS |
|---|---:|---:|---:|---:|---:|
| TAK order-2 | 86.581 | 11.01 | 112.163 | 8.50 | see note |
| TAK normalized strength 4 | 117.733 | 8.10 | 215.504 | 4.43 | see note |
| gzip -9 | 76.084 | 12.53 | 10.204 | 93.47 | 9,216 KiB |
| xz -9 | 1,146.152 | 0.83 | 8.843 | 107.84 | 981,120 / 1,224,344 KiB |
| zstd -19 | 899.420 | 1.06 | 4.455 | 214.05 | 225,664 / 12,032 KiB |

The TAK operation times are in-process measurements after input loading. The
external process times include command startup and file I/O. Compare them only
with this accounting warning. All size rows use matching complete-stream
accounting and are directly comparable.

The complete TAK runner took 542.351 seconds and peaked at 3,665,840 KiB while
holding the input, both research artifacts, and restored outputs. This does
not isolate decoder memory. It does show that the current non-streaming path
does not meet the one-gigabyte process target. A streaming benchmark and
decoder are required before claiming bounded memory.

Every method restored the exact 1,000,000,000-byte input.

## Direct milestone distance

| Milestone | Target bytes | TAK excess bytes | TAK bytes that must be removed |
|---|---:|---:|---:|
| M1 | 322,000,000 | 46,643,002 | 12.65% |
| M2 | 197,000,000 | 171,643,002 | 46.56% |
| M3 | 130,000,000 | 238,643,002 | 64.74% |
| M4 | 110,000,000 | 258,643,002 | 70.16% |
| M5 | 89,000,000 | 279,643,002 | 75.86% |

The measured gzip stream is 322,591,988 bytes, slightly above the nominal
322,000,000-byte M1 threshold. This confirms that the brief's reference sizes
are approximate and command/version dependent.

TAK is 14.28% larger than gzip, 70.93% larger than xz, and 56.65% larger than
Zstandard level 19. To equal those measured streams, TAK must remove 12.49%,
41.50%, and 36.16% of its current bytes, respectively.

## Decision

Keep normalized strength 4 as a validated classical research control, but do
not treat it as a path to M1 by itself. Its main gain is cold-context sharing,
which contributes little on one long text stream.

Do not retune strength on enwik9. The next compression experiment must target
a measured dependency that remains after mature order-2 modeling. The next
engineering experiment must introduce streaming encode/decode so memory and
throughput can be measured without full-file buffers.

This result does not authorize tensor work. A tensor proposal still requires
a new RFC naming a specific longer-context dependency and representation.

## Reproduction

```console
target/release/tak-order2-normalized-gate /tmp/enwik9 --strength 4
gzip -9 -n -c /tmp/enwik9
xz -9 -c /tmp/enwik9
zstd -q -19 -c /tmp/enwik9
```

Use `benchmarks/runners/measure.py` to record complete-process wall time and
peak child RSS. Decode every produced stream and compare it byte for byte with
the input.
