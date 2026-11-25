# RFC 0032: Bounded variable-order literal prediction

- Status: Complete artifact measured on Calgary, Canterbury, and enwik9
- Date: 2026-11-25
- Scope: Classical probability diagnostic

RFC 0030 uses the most recent continuation of an exact three-byte suffix.
A suffix can have multiple valid continuations. Test whether bounded
continuation counts over suffix orders three through six improve literal
prediction on the frozen RFC 0019 parse.

Each order has 8,192 direct-mapped rows. Use the full suffix as the key,
hashed by multiplying it by `0x9e3779b97f4a7c15` modulo 2^64 and taking
the top 13 bits. On a collision, replace the row on update. Update counts
on all source bytes, including match copies. When a row total reaches
4,096, halve all its counts rounding down before the next increment.
Maintain its total N, distinct count D, and singleton count S.

Start both predictions at the RFC 0019 literal probability P. Where
the exact eight-byte predictor has no prediction, visit matching rows
in increasing order. Replace P with `(count(byte) + A * P) / (N + A)`.
The PPM-style interpolated control uses A = D. The candidate uses
A = 4 + 2S, giving the shorter context more weight when many observations
are singletons. Both score the same events and share all row counts.
This PPM-style control uses interpolation and no exclusion.

Run Calgary first, independently per file. Advance only if either
model reduces aggregate ideal cost by at least 0.03 bits per input byte
and improves at least 12 of 18 files. Use the lower-loss qualifying
model for the next step, with ties favoring the PPM-style control.
An advance requires a separate integer artifact specification and a
complete-artifact comparison on Calgary and independent validation
before enwik9. The coding-cost screen uses floating-point arithmetic.
It creates no artifact and provides no decoder performance result.

## Integer artifact

The Calgary probability gate authorizes testing the singleton-weighted
model. Use magic TK32 with the same 28-byte header, extent checks, CRC,
block syntax, and encoder parse as RFC 0019. Use no transmitted model
state. The only change to the payload is the literal distribution when
one or more longer contexts are usable and the eight-byte lookup misses.

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | ASCII TK32 |
| 4 | 4 | Reserved, all zero |
| 8 | 8 | Restored byte count, little-endian u64 |
| 16 | 8 | Arithmetic payload byte count, little-endian u64 |
| 24 | 4 | CRC-32/ISO-HDLC of restored bytes, little-endian u32 |
| 28 | declared count | RFC 0019 arithmetic payload, with the literal change below |

Use the RFC 0019 arithmetic bit order and termination. The decoder
rejects nonzero reserved bytes, unexpected magic, an inconsistent
physical extent, output exceeding the caller's limit, invalid block or
match extents, invalid arithmetic intervals, and a checksum mismatch.
The stream ends when the declared output byte count is restored and
the checksum passes. The 64 MiB match window and the 32,768 fixed
continuation rows bound model storage independently of input length.
TK32 is a research identifier and has no TAK-C registry entry.

Let Q = 2^20. For each byte boundary b in 0 through 256, start with
`C = floor(Q * RFC0019_cumulative(b) / RFC0019_total)`. At each usable
row in increasing order, replace C with
`floor((Q * prefix_count(b) + (4 + 2S) * C) / (N + 4 + 2S))`.
The prefix counts sum observations of bytes strictly below b. Use u64
intermediates. Add b to the final C, reserving at least one count for
each byte and giving total Q + 256. The decoder finds the interval
containing the scaled value, then updates both models with the decoded
byte. All other ranges and updates follow RFC 0019 and the rules above.

Screen complete artifacts on Calgary and Canterbury. Exclude the
duplicate ptt5 from independent Canterbury totals. Advance to enwik9
only if both aggregates shrink by at least 0.5%, at least 12 Calgary
files and 6 independent Canterbury files improve, and every decode is
exact. Record separate encode and decode times and peak resident memory.
Preserve all results if either corpus fails.

## Results before enwik9

The singleton-weighted model saves 180,836.692 ideal bits on Calgary,
or 0.055617 per input byte. It improves 16 of 18 files. The PPM-style
control adds 80,268.045 bits and improves six files.

The complete stream saves 24,018 bytes on Calgary and 26,211 bytes on
the independent Canterbury files. It restores every file exactly.
See [the experiment report](../../research/experiments/2026-11-25-variable-order.md).

## enwik9 result

TK32 compresses enwik9 to 246,642,016 bytes, 3,120,865 bytes below RFC 0019.
The decoder restores all bytes exactly. The screen uses 238,692 KiB
peak encode RSS and 113,824 KiB peak decode-and-compare RSS. Encoding
takes 798.17 seconds. Decoding and comparison take 451.45 seconds.
The artifact remains 30,967,820 bytes above xz.
