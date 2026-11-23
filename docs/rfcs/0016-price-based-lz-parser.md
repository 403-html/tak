# RFC 0016: Price-based LZ parser gate

- Status: Draft
- Date: 2026-11-23
- Scope: Classical encoder parser only
- Format change: None

## 1. Problem statement

RFC 0015 requires the next ratio experiment to profile token costs and parser
misses before it proposes a mechanism. The `tak-lz-profile` diagnostic
replays the frozen RFC 0015 two-candidate parse and splits its ideal code
length. It reproduces the RFC 0015 Silesia candidate bits exactly.

On the five Silesia development groups, the profile shows:

| Component | Ideal bits | Bits per input byte |
|---|---:|---:|
| Literal bytes | 13,176,882 | 1.394 |
| Distance remainder bits | 4,848,112 | 0.513 |
| Distance buckets | 1,659,908 | 0.176 |
| Match lengths | 1,659,356 | 0.176 |
| Literal lengths | 903,701 | 0.096 |
| Block types | 572,106 | 0.061 |

Three candidate mechanisms are small:

- Repeat-distance coding: matches that reuse one of the last four distances
  spend 509,264 distance bits in total. That is 0.054 bits per byte before any
  repeat-symbol cost. A screening replay with a five-symbol repeat alphabet
  saved only 254,877 bits, or 0.027 bits per byte.
- Literal exclusion: removing the known mismatching byte from the first
  literal after each match saves 102,269 bits, or 0.011 bits per byte.
- Finer distance-slot or alignment coding saved 15,666 to 220,001 bits in
  the same screening replay. It does not help text.

The largest measured parser miss is cost-blind match acceptance. The greedy
parser takes every match of at least eight bytes. For 145,471 of 416,990
matches, the block type, length, and distance bits exceed the RFC 0009
byte-model bits of the covered bytes. The excess is 658,220 bits, or 0.070
bits per byte. A one-step lazy check also finds 46,144 matches for which the
next position offers a longer match.

The RFC 0009 byte model is updated with every input byte, whether a literal or
match produced it. Its per-byte code length is therefore independent of the
parse. An encoder can price every literal exactly and choose blocks by minimum
total price without any decoder change.

## 2. Hypothesis

A segment-wise minimum-price parser reduces complete `TKLZ` artifact size
relative to the RFC 0015 artifact by at least 0.05 bits per input byte in
aggregate on the five established Silesia development groups. It MUST reduce
at least three of the five groups.

The control is the complete RFC 0015 artifact produced from the same input.
Both artifacts MUST decode with the unchanged RFC 0014 decoder and restore the
input exactly.

## 3. Method

The parser uses the RFC 0015 table, hash, window, minimum length, maximum
length, and two-candidate replacement rule. It inserts every input position in
input order, as the greedy parser does. Candidates at each position are
therefore independent of the chosen parse.

At each position, the parser considers the newest table candidate and the
older candidate when the older candidate is strictly longer.

The input is parsed in segments of at most 65,536 positions. Each segment
solves a two-state shortest path. One state ends in a literal block. The other
ends in a match block or at the segment start.

- A literal byte costs its exact ideal RFC 0009 model bits.
- Starting a literal block also costs the literal type and literal-length
  bucket 0 prices.
- A match costs the match type price, the match-length bucket price plus its
  remainder bits, and the distance bucket price plus its remainder bits.
- Every match length from 8 to 71 bytes is priced, plus the full available
  length. A match is truncated at the segment end.

Metadata prices are `log2(total / count)` from the encoder's adaptive
distributions at segment start. The parser updates those distributions with
every emitted block in encoder order, including halving at 16,384.

A candidate of at least 1,024 bytes ends the current segment and is emitted
immediately at full length. The skipped positions are inserted into the table
and the price model. This bounds work on long repeats.

Literal runs that cross a segment boundary are emitted as one literal block.
Prices use `f64` logarithms in the encoder only. Different platforms MAY emit
different valid artifacts. The decoder is unaffected.

## 4. Gate and stop condition

Run the complete RFC 0015 and RFC 0016 artifacts on the final-10% Silesia
development partitions. Pass only if the hypothesis in Section 2 holds, every
artifact restores exactly, and encoder peak RSS stays below one gigabyte.

If the gate fails, reject the parser. Do not tune the segment length, nice
length, priced length range, literal start price, or candidate rule on the
same development results.

If the gate passes, freeze every parameter and validate on Calgary. The
validation passes only if the Calgary aggregate artifact is smaller than RFC
0015, every file restores exactly, and decode time stays within 1.10 times RFC
0015. Report encode time without a pass threshold. Do not claim an enwik9
result until a bounded encoder is measured on enwik9.

## 5. Tensor scope

This RFC does not authorize a tensor model. It tests a classical parser
limitation. A tensor attempt still requires a separate RFC that identifies a
specific dependency that the rejected models cannot represent.

## 6. Compatibility and security

The `TKLZ` syntax, decoder, limits, and checksum are unchanged. Every emitted
block satisfies the RFC 0014 length and distance limits. Existing decoder
rejection conditions apply unchanged.

## 7. Testing strategy

Unit tests MUST show that a priced parse covers the input in order, that each
match copies bytes equal to its source, and that long runs cross both the
segment and nice-length boundaries. Artifact tests MUST round-trip through the
existing decoder.
