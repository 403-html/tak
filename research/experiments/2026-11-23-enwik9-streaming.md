# Completed screen: byte-identical enwik9 streaming

## Status

RFC 0007 is accepted. Preserve this result as a non-publishable single-sample
engineering screen. It validates bounded-memory operation, not a compression
improvement.

## Hypothesis and control

The streaming implementation can preserve the complete RFC 0006 strength-4
`TKN2` artifact byte for byte while reducing peak process RSS below one
gigabyte. The buffered RFC 0006 result is the control. Its complete artifact
was 368,643,002 bytes. Its all-memory runner peaked at 3,665,840 KiB while it
held multiple complete inputs, artifacts, and outputs.

The stop condition was any artifact-size change, round-trip failure, peak RSS
of at least one gigabyte, or model reset at an I/O boundary.

## Dataset and environment

- Input: enwik9, exactly 1,000,000,000 bytes.
- Input SHA-256: `159b85351e5f76e60cbe32e04c677847a9ecba3adc79addab6f4c6c7aa3744bc`.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Model: RFC 0006 normalized order-2, fixed strength 4.
- Accounting: complete `TKN2` artifact, including its 28-byte header.
- Date: 2026-11-23 Europe/Warsaw.
- Samples: one measured run without a warm-up.
- Source commit: `d1e384a8ec16505e54d6c4fa1bf125d60666f09a` plus the RFC 0007 working tree.

The raw measurements are in
`benchmarks/reports/raw/2026-11-23-enwik9-streaming.tsv`.

## Result

| Operation | Complete bytes | Wall time | Throughput | Peak RSS | Exact restoration |
|---|---:|---:|---:|---:|---:|
| Encode | 368,643,002 | 119.869 s | 7.96 MiB/s | 13,216 KiB | not applicable |
| Decode | 368,643,002 | 217.710 s | 4.38 MiB/s | 13,216 KiB | yes |

The streaming artifact SHA-256 is
`224b355de5ad1d5b25be848602c1de87c2c706b6ba4fee2fbbaa91b50c7cadea`.
Its size exactly matches the buffered control. Unit tests also compare the
complete bytes produced by both encoders for empty, short, structured, binary,
and arithmetic-rescaling inputs.

The restored enwik9 SHA-256 exactly matches the input digest. The decoder also
verified the artifact checksum. The complete file comparison succeeded.

Peak RSS fell from 3,665,840 KiB for the multi-artifact RFC 0006 runner to
13,216 KiB for each isolated streaming process. These are different process
scopes, so the ratio does not isolate only the buffering change. The new
measurement nevertheless demonstrates that the actual encoder and decoder
remain far below the one-gigabyte requirement on enwik9.

The operation-reported encode and decode times were 119.853 and 217.701
seconds. The table uses outer process wall time, including file I/O and command
startup. Compare these values with the earlier in-process times only with that
accounting warning.

## Goal distance

Streaming changes resource use, not compressed size. The artifact still
exceeds M1 by 46,643,002 bytes and must remove 12.65% of its current bytes to
reach 322,000,000 bytes. It exceeds M2 by 171,643,002 bytes and must remove
46.56% to reach 197,000,000 bytes.

The decoder reaches only 4.38 MiB/s. It would need about 45.7 times this
throughput to reach the aspirational 200 MiB/s decoder target on this hardware
and accounting method. The encoder would need about 6.3 times its measured
7.96 MiB/s to reach 50 MiB/s.

## Decision

Accept RFC 0007. Keep the streaming path as the bounded-memory research path.
Do not claim a compression improvement, because the artifact is intentionally
identical.

The next compression experiment must address dependency remaining after a
mature order-2 model. A tensor attempt still requires a new RFC that identifies
a specific longer-context dependency that the rejected bigram factorization
cannot represent. Increasing tensor rank alone remains unjustified.

## Reproduction

```console
python3 benchmarks/runners/measure.py \
  --timing encode-memory.tsv --stdout encode-output.tsv \
  target/release/tak-order2-normalized-stream encode \
  /tmp/enwik9 /tmp/enwik9.tkn2 --strength 4

python3 benchmarks/runners/measure.py \
  --timing decode-memory.tsv --stdout decode-output.tsv \
  target/release/tak-order2-normalized-stream decode \
  /tmp/enwik9.tkn2 /tmp/enwik9.restored --max-output 1000000000

cmp /tmp/enwik9 /tmp/enwik9.restored
```
