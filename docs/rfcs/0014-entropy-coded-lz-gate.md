# RFC 0014: Entropy-coded LZ opportunity gate

- Status: Development artifact and bounded implementation accepted
- Date: 2026-11-23
- Scope: Classical research diagnostic only
- Format change: None

## 1. Problem statement

RFC 0011 found frequent correct match runs but rejected a raw marker and
LEB128 length. Its best length-8 row saved only 131,959 gross bits on Calgary
before control costs. A block parser can amortize control cost across literal
runs and entropy-code match length and distance.

## 2. Hypothesis

A greedy bounded LZ parser with entropy-coded block metadata reduces ideal
code length relative to RFC 0009 by at least 0.05 bits per input byte on three
of five Silesia development groups and does not regress in aggregate.

This is an opportunity diagnostic. It is not compressed size and assigns no
TAK-C method identifier.

## 3. Parser

Use a direct-mapped table with 1,048,576 entries and the RFC 0009 multiplicative
hash. Each entry stores an exact 8-byte key and the most recent input position.
Reject candidates farther than 67,108,864 bytes back.

At each input position, find the longest match from the indexed candidate.
Limit a match to 65,535 bytes. Emit a match only when its length is at least
eight bytes. Otherwise append the byte to the current literal block. Use greedy
parsing. Update the table for every consumed position, including positions
inside a match. Do not search additional candidates or tune the minimum length.

Process each file independently. Do not join blocks across file boundaries.

## 4. Ideal accounting

Charge every emitted block and one end block. Use these adaptive distributions
with unit pseudocounts:

1. one three-symbol distribution for literal, match, and end block types;
2. one 32-symbol literal-length bucket distribution;
3. one 32-symbol match-length bucket distribution; and
4. one 32-symbol match-distance bucket distribution.

For an unsigned value `v`, set `bucket = floor(log2(v + 1))`. Charge the
adaptive ideal log loss of the bucket plus `bucket` raw remainder bits. Encode
literal length as `length - 1`, match length as `length - 8`, and match
distance as `distance - 1`.

Charge RFC 0009 ideal byte-model bits for literal bytes. Update the RFC 0009
model with every reconstructed byte, including match bytes. The baseline is
the RFC 0009 ideal byte-model cost for every input byte.

Account separately for type bits, length bits, distance bits, literal bits,
block counts, covered match bytes, wall time, and peak RSS.

## 5. Gate and stop condition

Run the fixed diagnostic on the established final-10% Silesia development
partitions. Proceed to a complete research artifact only if the hypothesis
passes. A later artifact MUST specify exact integer entropy coding, headers,
decoder state, corruption handling, output limits, window storage, and exact
round-trip behavior.

If the gate fails, reject this parser. Do not tune the minimum match length,
window, table capacity, or bucket mapping on the same development results.

If the gate passes, freeze every parser parameter before validation. Do not
claim progress toward M2 until a complete enwik9 artifact is measured.

## 6. Tensor scope

This RFC does not authorize a tensor model. RFC 0013 remains rejected. A later
tensor attempt still requires a new RFC that identifies a dependency or
representation limitation not addressed by the rejected models.

## 7. Results and decision

The Silesia development gate passes on four groups. Savings were 0.4261 bits
per byte on Samba, 0.2444 on XML, 0.2871 on Mozilla, and 0.2614 on OOffice.
Dickens regressed by 0.0754 bits per byte. Aggregate ideal savings were
2,605,801 bits, or 0.2757 bits per byte.

Freeze the parser and validate it without parameter changes. On Calgary, the
same diagnostic saved 190,882 ideal bits in aggregate, or 0.0587 bits per
byte. Book1 and `pic` regressed. The ideal saving is approximately 23,860
bytes, far below the 143,620-byte RFC 0009-to-xz artifact gap.

Permit one complete research artifact because the fixed mechanism passes its
development gate and remains positive on Calgary. The artifact MUST beat RFC
0009 complete size by at least 1% in Calgary aggregate, restore every file
exactly, stay below two times RFC 0009 decode time, and use less than one
gigabyte peak RSS. Reject it if any condition fails. Do not change parser
parameters or claim that this opportunity can beat xz before measurement.

## 8. Experimental artifact format

The complete artifact uses the four-byte magic `TKLZ`. It has no TAK-C method
identifier. All multibyte integers use little-endian byte order.

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | Magic `TKLZ` |
| 4 | 4 | Version and flags, all zero |
| 8 | 8 | Restored byte length |
| 16 | 8 | Payload byte length |
| 24 | 4 | CRC-32 of all restored bytes |
| 28 | variable | Arithmetic-coded payload |

The payload uses one 32-bit arithmetic-coder state. It interleaves block type,
bucket, remainder-bit, and literal-byte ranges. Adaptive metadata counts start
at one and halve with upward rounding when their total reaches 16,384. Literal
bytes use the exact RFC 0009 model. Encoder and decoder update that model with
every literal byte and reconstructed match byte.

Payload bits use most-significant-bit-first order within each byte. Initialize
`low` to 0, `high` to `0xffffffff`, and `code` from the first 32 payload bits.
Treat absent bits after the payload extent as zero. For a distribution total
`T`, calculate:

```text
width  = high - low + 1
scaled = floor(((code - low + 1) * T - 1) / width)
```

Select the symbol whose cumulative interval is `[C_low, C_high)`. Update:

```text
high = low + floor(width * C_high / T) - 1
low  = low + floor(width * C_low  / T)
```

Renormalize while `high < 0x80000000`, `low >= 0x80000000`, or
`low >= 0x40000000 && high < 0xc0000000`. Subtract the applicable half or
quarter offset, shift `low` and `high` left, set the new low bit of `high`, and
shift one payload bit into `code`. All state arithmetic is unsigned 64-bit
arithmetic bounded to the 32-bit coding interval.

Bucket remainder bits use uniform binary ranges and most-significant-bit-first
order. Metadata symbols use increasing numeric order. CRC-32 uses reflected
polynomial `0xedb88320`, initial state `0xffffffff`, and final complement.

The decoder reads blocks until the end symbol. A literal length is the decoded
bucket value plus one. A match length is the decoded value plus eight. A match
distance is the decoded value plus one. The decoder copies overlapping matches
one byte at a time from `output_length - distance`.

The decoder MUST reject a nonzero version or flag, inconsistent payload extent,
declared output above the caller limit, arithmetic failure, bucket above 31,
match longer than 65,535 bytes, distance above 67,108,864 bytes, distance beyond
the restored prefix, block beyond the declared output, premature end symbol,
or checksum mismatch. CRC-32 detects accidental corruption only.

## 9. Artifact result

The fixed artifact produced 1,004,678 bytes across Calgary, compared with
1,029,480 for RFC 0009. It saves 24,802 bytes, or 2.41%, and passes the 1% size
gate. All 18 files restored exactly. Single-sample aggregate decode time was
0.56 times RFC 0009 and peak process RSS was 36,756 KiB.

The artifact remains 83,605 bytes larger than Zstandard-19 and 118,818 bytes
larger than xz. RFC 0010 is also 3,983 bytes smaller. Retain RFC 0014 as a
research component, not the best-ratio profile.

Controlled Calgary timing used one warm-up and five measured repetitions.
Median encode time was 0.74 times RFC 0009. Median decode time was 0.54 times
RFC 0009. The timing gate passes.

## 10. Bounded decoder

The bounded decoder reads the arithmetic payload through a 64 KiB input
buffer. It writes restored bytes through a 64 KiB output buffer and retains at
most the last 67,108,864 restored bytes for match copies. It produces the same
bytes and checksum result as the buffered decoder.

On the complete 51,220,480-byte Mozilla file, the bounded decoder restored the
exact SHA-256 input, used 101,788 KiB peak RSS, and completed in 13.59 seconds.
The artifact was 20,243,862 bytes. This is one screening run, not publishable
timing.

## 11. Bounded encoder

The bounded encoder reads a seekable input through four 64 KiB cached pages.
It retains the fixed match table and probability models. It does not retain the
complete input, parsed block list, or arithmetic payload. It writes the payload
incrementally and patches the 28-byte header after the payload length is known.

On the complete Mozilla file, the bounded encoder produced the byte-identical
20,243,862-byte artifact. It used 68,768 KiB peak RSS and completed in 24.06
seconds. The buffered candidate's internal encode measurement was 7.43 seconds,
so the bounded implementation was 3.24 times slower. The buffered gate's
294,192 KiB process peak includes RFC 0009 and both RFC 0014 operations; it is
not an isolated encoder-memory measurement.

The bounded implementation gate passes for correctness and memory. It does not
pass the aspirational throughput target. Enwik9 evaluation is now permitted,
but this reference encoder requires optimization before product use.
