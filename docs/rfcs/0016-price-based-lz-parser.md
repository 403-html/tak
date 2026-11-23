# RFC 0016: Price-based LZ parser gate

- Status: Calgary size validated; decode-time condition failed
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

Two other candidate mechanisms are small upper bounds:

- Repeat-distance coding: matches that reuse one of the last four distances
  spend 509,264 distance bits in total. That is 0.054 bits per byte before any
  repeat-symbol cost, so a repeat alphabet cannot save more.
- Literal exclusion: removing the known mismatching byte from the first
  literal after each match saves 102,269 bits, or 0.011 bits per byte.

Distance remainder bits are the largest metadata cost, but they are stored
uniformly because the bucket already captures most of their predictable
magnitude. This RFC does not test distance remodeling.

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

## 8. Results

The Silesia development gate passes. The complete priced artifact is smaller
on all five groups. Aggregate size falls from 2,845,106 to 2,702,364 bytes, a
142,742-byte or 5.02% reduction. That is 0.1208 bits per input byte, above the
0.05 threshold. Every artifact restores exactly through the unchanged decoder.
Peak gate-process RSS, covering both encoders and decoders, is 145,056 KiB.

| Group | RFC 0015 bytes | RFC 0016 bytes | Saved bits per byte |
|---|---:|---:|---:|
| Dickens | 339,732 | 316,625 | 0.1814 |
| Samba | 404,081 | 371,039 | 0.1223 |
| XML | 117,194 | 108,714 | 0.1269 |
| Mozilla | 1,767,879 | 1,695,206 | 0.1135 |
| OOffice | 216,220 | 210,780 | 0.0707 |

The frozen parser then improves all 18 Calgary files. Aggregate size falls
from 996,359 to 950,768 bytes, a 45,591-byte or 4.58% reduction. Every file
restores exactly.

Five controlled Calgary repetitions after one warm-up give a median aggregate
decode time of 0.903 seconds versus 0.820 seconds for RFC 0015. The ratio is
1.102. It misses the 1.10 bound in Section 4 by 0.002. Median aggregate encode
time is 1.933 seconds versus 1.679, or 1.15 times. Peak process RSS is 55,812
KiB.

## 9. Decision

The size hypothesis passes on development and validation data. The
pre-registered decode-time condition fails narrowly. Do not promote RFC 0016
to the best-ratio profile, and do not relax the bound on the same
measurement.

The decoder is unchanged. The extra decode time follows from the chosen
blocks. On Calgary, the priced parse codes 1,895,245 bytes as literals instead
of 936,030, and emits 79,345 matches instead of 165,917. Each literal costs an
arithmetic-coded byte-model step, while match bytes only update the model.
This block count comes from a separate instrumented run, not the timed run.

A follow-up MUST either state a new decode-time budget before any new
measurement, or reduce decoder literal cost. Do not tune the parser parameters
on these results. An enwik9 result requires a bounded priced encoder. The
current research encoder reads the complete input, so enwik9 is unmeasured.

Raw profile rows are in
`benchmarks/reports/raw/2026-11-23-lz-profile-silesia.tsv`, SHA-256
`62c9821be0bd5d94520f34e6b3044d01d73e4eb1a5ab4ea7db332b7c4f228045`, and
`benchmarks/reports/raw/2026-11-23-lz-profile-calgary.tsv`, SHA-256
`d392298cf47c24357ba049cf4a9ec226903954ed1220a537a240f1d7a879eec1`.
Development artifact rows are in
`benchmarks/reports/raw/2026-11-23-lz-priced-silesia.tsv`, SHA-256
`832f442044ecda5def2e7d8627da651355489ace61d56891f94048a7595e98ee`.
Calgary artifact rows are in
`benchmarks/reports/raw/2026-11-23-lz-priced-calgary.tsv`, SHA-256
`bf9ae772150319a2156f0f72e7b74d798bf3d5f3e5a388366f936f9a2d1063f7`.
Controlled Calgary rows are in
`benchmarks/reports/raw/2026-11-23-lz-priced-controlled.tsv`, SHA-256
`48ea8d9845689c424fac7f2e710f82cf898bfd1dd48b7ce147ab787f88b23d49`.
Calgary block counts are in
`benchmarks/reports/raw/2026-11-23-lz-priced-blocks-calgary.tsv`, SHA-256
`16281a7c20872ba0ee0762cc3f4ef12611c3cb23c07ed051752a5acb33fa7c21`.
