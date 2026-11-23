# RFC 0014: Entropy-coded LZ opportunity gate

- Status: Diagnostic accepted; artifact permitted
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
