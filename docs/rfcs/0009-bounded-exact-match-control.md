# RFC 0009: Bounded exact-match classical control

- Status: Independent validation accepted; enwik9 M1 achieved
- Date: 2026-11-23
- Scope: Classical research artifact only
- TAK-C method identifier: None

## 1. Problem statement

RFC 0008 found that the most recent continuation after an exact 8-byte suffix
reduces ideal cross-entropy after normalized order-2 conditioning. It did not
measure a complete compressed artifact and used an unbounded hash table.

This RFC tests the same dependency with deterministic bounded state and the
existing arithmetic coder. It is the required classical control before any
tensor proposal for long-range dependencies.

## 2. Hypothesis

A fixed-capacity exact-match model will produce a complete artifact at least
2% smaller than RFC 0006 strength 4 on at least three of five Silesia
development groups. It MUST NOT be larger in aggregate.

The model MUST decode no more than two times slower than RFC 0006 and MUST use
less than one gigabyte peak process RSS. Every artifact MUST restore the exact
input.

## 3. Model

Maintain the RFC 0006 normalized order-2 model at strength 4. Also maintain a
direct-mapped table of 1,048,576 slots. Each occupied slot stores an exact
8-byte suffix key and the byte that followed its most recent mapped occurrence.

Interpret the suffix bytes as one unsigned 64-bit big-endian integer `K`.
Calculate the table index as:

```text
index = high_20_bits(K * 0x9E3779B97F4A7C15 modulo 2^64)
```

An occupied slot predicts only if its stored key equals `K`. A different key
at the same index is a collision and MUST NOT predict. After coding the actual
byte, replace the indexed slot with `K` and that byte.

For a valid prediction, use the RFC 0008 divisor-4 rule:

```text
B = floor(T / 4)
F'(s) = F(s) + B, if s is the predicted byte
F'(s) = F(s),     otherwise
T' = T + B
```

Here `F` and `T` are the exact RFC 0006 integer frequency and total. The first
eight bytes have no match prediction. All operations used by the decoder are
fixed-width integer operations.

## 4. Research artifact

The complete artifact uses magic `TKM8`. It sends no match table because both
sides derive the table from decoded bytes. All multi-byte integers use
little-endian byte order.

| Offset | Size | Field | Required value |
|---:|---:|---|---|
| 0 | 4 | Magic | ASCII `TKM8` |
| 4 | 1 | Version | 0 |
| 5 | 1 | Flags | 0 |
| 6 | 2 | Reserved | 0 |
| 8 | 8 | Uncompressed length | 0 to 2^64 - 1 |
| 16 | 8 | Payload length | 0 to 2^64 - 1 |
| 24 | 4 | Checksum | CRC-32/ISO-HDLC of restored bytes |
| 28 | variable | Payload | Arithmetic-coded bytes |

The decoder MUST validate fixed fields and physical extent. It MUST enforce
the caller's output limit before allocation. It MUST verify the checksum.

## 5. Gate and stop condition

Use the exact RFC 0008 Silesia partitions. Compare complete `TKM8` artifacts
with complete strength-4 `TKN2` artifacts. Run one screening sample first.

Reject this model if it fails any hypothesis condition. Do not tune capacity,
hashing, suffix length, or bonus after measurement. Preserve a failed result.

If the screen passes, optimize only measured implementation bottlenecks without
changing artifact bytes. Then run controlled timing repetitions and validate
the fixed model on a corpus not used to select it.

## 6. Tensor scope

This RFC authorizes no tensor implementation. A later tensor RFC must specify
how its state represents the measured exact-match dependency and must compare
against this classical control with complete accounting. Increasing rank in a
model that lacks match state remains unjustified.

## 7. Security and compatibility

`TKM8` is a research artifact, not TAK-C. Table capacity is fixed. The output
limit bounds decode work. Checksums detect accidental corruption and do not
authenticate data.

## 8. Testing strategy

Test exact round trips, histories shorter than eight bytes, correct and
incorrect predictions, deterministic collision replacement, arithmetic
rescaling, truncation, corruption, output limits, and unsupported fields.

## 9. Development decision

Accept the development gate. `TKM8` was smaller than strength-4 `TKN2` on all
five Silesia groups. Reductions were 6.39% on Dickens, 20.54% on Samba, 17.45%
on XML, 9.28% on Mozilla, and 6.64% on OOffice. Aggregate complete size fell
from 3,614,558 to 3,207,519 bytes, an 11.26% reduction.

Every artifact restored the exact input. Match decoding ranged from 0.75 to
1.71 times the baseline time in this single-sample screen. The largest complete
process used 115,040 KiB peak RSS. The screen therefore passes every gate
condition.

This result validates a classical development control, not a stable codec.
Run controlled timing repetitions and independent validation before promotion.
Do not tune the table, hash, suffix, or bonus. Do not implement a tensor model
from development evidence alone.

## 10. Validation plan

Validate the unchanged model on the Calgary corpus, compressed as independent
files. Use the existing prose/reference, technical/source, and binary groups.
This corpus did not select the RFC 0009 suffix, bonus, table, or hash.

Validation passes only if `TKM8` is at least 2% smaller than strength-4 `TKN2`
on two of three groups, is not larger in aggregate, stays below two times the
baseline decode time in every group, uses less than one gigabyte peak process
RSS, and restores every file exactly.

Run one size-and-correctness screen first. If it passes, run one warm-up and
five measured repetitions per file. Sum file times by group and repetition,
then report group medians. Do not tune the fixed model on Calgary.

## 11. Validation decision

Accept independent Calgary validation. `TKM8` reduced complete size by 7.45%
on prose/reference, 8.37% on technical/source, and 4.76% on binary data.
Aggregate size fell from 1,107,385 to 1,029,480 bytes, a 7.04% reduction.

After one warm-up and five measured repetitions per file, median group decode
ratios were 1.25, 1.86, and 1.36 times the strength-4 baseline. The largest
measured process used 21,720 KiB peak RSS. Every artifact restored its input.

The classical mechanism now has development and held-out evidence. It remains
research-only. Its speed is far below project targets, and enwik9 behavior is
unmeasured. A future tensor RFC must still specify a concrete representation
of match state and a reason it can beat this direct table. Rank alone is not a
mechanism.

## 12. enwik9 validation

The byte-identical streaming implementation produced a 315,578,766-byte
complete artifact. This is 53,064,236 bytes or 14.39% smaller than RFC 0006.
It is 6,421,234 bytes below the nominal M1 target.

Encoding took 145.729 seconds at 6.54 MiB/s and peaked at 29,596 KiB RSS.
Decoding took 232.001 seconds at 4.11 MiB/s and peaked at 29,488 KiB RSS. The
restored billion-byte output matched the canonical enwik9 SHA-256 exactly.

Accept M1 as a measured single-sample result. The artifact remains 118,578,766
bytes above M2 and must remove 37.57% of its current bytes to reach it. It is
80,249,827 bytes larger than measured Zstandard level 19 and 99,904,570 bytes
larger than measured xz. Decode speed remains a major blocker.

This result does not authorize a tensor implementation. Any tensor RFC must
represent the measured long-range match state and compare against `TKM8`.
