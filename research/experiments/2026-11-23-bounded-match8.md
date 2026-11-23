# Accepted development gate: bounded exact match

## Status

RFC 0009 passes its single-sample development gate. The fixed bounded model
improves complete artifact size on every Silesia group. It remains a research
artifact and requires controlled timing and independent validation.

## Method

The model combines RFC 0006 strength 4 with the RFC 0008 exact 8-byte suffix
prediction and divisor-4 bonus. A deterministic direct-mapped table contains
1,048,576 slots. Collisions replace the indexed entry. The artifact transmits
no table because the decoder reconstructs it from prior output.

The control is the complete strength-4 `TKN2` artifact. The candidate is the
complete `TKM8` artifact. Both include the 28-byte header, payload, length, and
checksum. Both use the same arithmetic coder.

## Dataset and environment

- Inputs: the exact final-10% Silesia partitions from RFC 0008.
- Total input: 9,451,681 bytes.
- Input digests: `benchmarks/reports/raw/2026-11-23-match-dependency-inputs.tsv`.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Date: 2026-11-23 Europe/Warsaw.
- Samples: one measured run without a warm-up.
- Source: commit `90f99d7` plus the RFC 0009 working tree.

Raw sizes and operation times are in
`benchmarks/reports/raw/2026-11-23-match8-screen.tsv`, SHA-256
`523abc9431d7730b37a76143f970643e7f64165bd05dfd29c94452108b9b3910`.
Process measurements are in
`benchmarks/reports/raw/2026-11-23-match8-memory.tsv`, SHA-256
`84f4a30fa2a5b9909bb023f0df3dbefbf6ba6a81beb5600ae282abde7bf0657c`.

## Results

| Group | Input bytes | TKN2 bytes | TKM8 bytes | Reduction | Decode ratio | Peak RSS |
|---|---:|---:|---:|---:|---:|---:|
| Dickens | 1,019,245 | 356,669 | 333,890 | 6.39% | 1.71x | 31,424 KiB |
| Samba | 2,160,640 | 666,314 | 529,423 | 20.54% | 0.75x | 50,228 KiB |
| XML | 534,528 | 166,005 | 137,035 | 17.45% | 0.91x | 26,172 KiB |
| Mozilla | 5,122,048 | 2,170,670 | 1,969,186 | 9.28% | 1.07x | 115,040 KiB |
| OOffice | 615,220 | 254,900 | 237,985 | 6.64% | 1.19x | 38,084 KiB |
| Aggregate | 9,451,681 | 3,614,558 | 3,207,519 | 11.26% | 1.06x | — |

All ten artifacts restored their exact inputs. Aggregate in-process throughput
was 6.49 MiB/s encoding and 4.25 MiB/s decoding for `TKM8`, versus 6.64 and
4.49 MiB/s for `TKN2`. These are single-sample development measurements, not
publishable timing results.

The direct-mapped table retains enough of the diagnostic benefit despite
collisions. The measured complete-size reduction is smaller than the ideal
RFC 0008 prediction, as expected, but it remains broad and substantial.

## Distance to project goals

This screen does not measure enwik9. The last measured enwik9 artifact remains
368,643,002 bytes. TAK is still 46,643,002 bytes above M1 and must remove
12.65% of its current bytes to reach M1. It must remove 46.56% to reach M2.

Do not apply the 11.26% development reduction to enwik9. If it generalized
perfectly, it would still not quite close the measured M1 gap, but such an
extrapolation is not evidence. A bounded streaming enwik9 measurement is
required after held-out validation.

## Decision

Accept the RFC 0009 development gate. Keep every model parameter fixed. Next,
run one warm-up and five controlled repetitions, then validate the same model
on an independent corpus. Do not implement a tensor representation yet.

## Reproduction

```console
python3 benchmarks/runners/measure.py \
  --timing /tmp/match8-mozilla-memory.tsv \
  --stdout /tmp/match8-mozilla.tsv \
  target/release/tak-match8-gate \
  /tmp/silesia-screen/mozilla --tail-bytes 5122048
```

Replace the input and partition size with the exact values in the raw table.
