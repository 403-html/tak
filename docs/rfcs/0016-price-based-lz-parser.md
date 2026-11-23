# RFC 0016: Price-based LZ parser gate

- Status: Calgary validated after decoder speed-up; enwik9 unmeasured
- Date: 2026-11-23
- Scope: Classical encoder parser only
- Format change: None

## 1. Problem statement

RFC 0015 requires a profile of token costs and parser misses before the next
ratio experiment. The `tak-lz-profile` diagnostic does this profile. It parses
the input with the frozen RFC 0015 two-candidate parser. Then it divides the
ideal code length into its parts. Its total is equal to the RFC 0015 Silesia
candidate bits.

The profile gives these results on the five Silesia development groups:

| Part | Ideal bits | Bits per input byte |
|---|---:|---:|
| Literal bytes | 13,176,882 | 1.394 |
| Distance remainder bits | 4,848,112 | 0.513 |
| Distance buckets | 1,659,908 | 0.176 |
| Match lengths | 1,659,356 | 0.176 |
| Literal lengths | 903,701 | 0.096 |
| Block types | 572,106 | 0.061 |

Two other mechanisms have small upper limits:

- Repeat-distance coding. Some matches use one of the last four distances.
  These matches use 509,264 distance bits in total. That is 0.054 bits per
  byte. A repeat alphabet cannot save more than this value.
- Literal exclusion. The first literal after a match cannot be equal to the
  byte that stopped the match. If the model removes that byte, it saves
  102,269 bits. That is 0.011 bits per byte.

Distance remainder bits are the largest metadata cost. The bucket already
holds most of their predictable magnitude, so the remainder bits are almost
uniform. This RFC does not test a new distance model.

The largest parser miss is that the parser does not compare costs. The greedy
parser accepts every match of eight bytes or more. For 145,471 of 416,990
matches, the type, length, and distance bits are more than the RFC 0009
byte-model bits of the same bytes. The excess is 658,219 bits, or 0.070 bits
per byte. A one-step lazy check also finds 46,144 matches where the next
position has a longer match.

The RFC 0009 byte model updates with every input byte. A literal byte and a
match byte update it in the same way. Thus the model cost of each byte does not
change with the parse. An encoder can calculate the exact cost of each literal.
It can then select the blocks with the lowest total cost. The decoder does not
change.

## 2. Hypothesis

A segment-wise lowest-cost parser makes the complete `TKLZ` artifact smaller
than the RFC 0015 artifact. The decrease is at least 0.05 bits per input byte
in aggregate on the five Silesia development groups. At least three of the
five groups MUST decrease.

The control is the complete RFC 0015 artifact from the same input. Both
artifacts MUST decode with the unchanged RFC 0014 decoder. Both MUST restore
the input exactly.

## 3. Method

The parser uses the RFC 0015 table, hash, window, minimum length, maximum
length, and two-candidate replacement rule. It inserts each input position in
input order, as the greedy parser does. Thus the candidates at each position
do not change with the parse.

At each position, the parser uses the newest table candidate. It also uses the
older candidate if that candidate is longer.

The parser divides the input into segments of 65,536 positions or fewer. For
each segment, it finds the lowest-cost path through two states. One state ends
with a literal block. The other state ends with a match block or at the start
of the segment.

- The cost of a literal byte is its exact ideal RFC 0009 model bits.
- The start of a literal block also costs the literal type price and the
  price of literal-length bucket 0.
- The cost of a match is the sum of three parts: the match type price, the
  match-length bucket price and its remainder bits, and the distance bucket
  price and its remainder bits.
- The parser calculates the cost of each match length from 8 to 71 bytes. It
  also calculates the cost of the full available length. A match stops at the
  end of the segment.

Each metadata price is `log2(total / count)`. The counts come from the
adaptive distributions of the encoder at the start of the segment. The parser
updates these distributions with each block, in encoder order. The update
includes the halving at 16,384.

A candidate of 1,024 bytes or more stops the current segment. The parser emits
it at full length immediately. Then it inserts the skipped positions into the
table and the price model. This rule limits the work on long repeats.

A literal run that continues across a segment boundary is one literal block.
The encoder calculates prices with `f64` logarithms. Thus different platforms
MAY emit different valid artifacts. The decoder does not use these prices.

## 4. Gate and stop condition

Make the complete RFC 0015 and RFC 0016 artifacts on the final-10% Silesia
development partitions. The gate passes only if all of these conditions are
true:

- The hypothesis in Section 2 is true.
- Each artifact restores the input exactly.
- The encoder peak RSS is less than one gigabyte.

If the gate fails, reject the parser. Do not tune the segment length, the nice
length, the range of priced lengths, the literal start price, or the candidate
rule on the same development results.

If the gate passes, freeze all parameters and validate on Calgary. The
validation passes only if all of these conditions are true:

- The Calgary aggregate artifact is smaller than RFC 0015.
- Each file restores exactly.
- The decode time is not more than 1.10 times the RFC 0015 decode time.

Report the encode time, but do not use it as a pass condition. Do not claim an
enwik9 result before a bounded encoder runs on enwik9.

## 5. Tensor scope

This RFC does not permit a tensor model. It tests a limit of the classical
parser. A tensor experiment still requires a separate RFC. That RFC must
identify a specific dependency that the rejected models cannot represent.

## 6. Compatibility and security

The `TKLZ` syntax, decoder, limits, and checksum do not change. Each block
from the parser is in the RFC 0014 length and distance limits. All decoder
rejection conditions stay the same.

## 7. Testing strategy

Unit tests MUST show these properties:

- A priced parse covers the input in order.
- Each match copies bytes that are equal to its source bytes.
- Long runs cross the segment boundary and the nice-length boundary.

Artifact tests MUST do a round trip through the existing decoder.

## 8. Results

The Silesia development gate passes. The complete priced artifact is smaller
on all five groups. The aggregate size decreases from 2,845,106 to 2,702,364
bytes. That is a decrease of 142,742 bytes, or 5.02%. It is 0.1208 bits per
input byte, which is more than the 0.05 threshold. Each artifact restores
exactly through the unchanged decoder. The peak RSS of the gate process is
145,056 KiB. This value includes both encoders and both decoders.

| Group | RFC 0015 bytes | RFC 0016 bytes | Saved bits per byte |
|---|---:|---:|---:|
| Dickens | 339,732 | 316,625 | 0.1814 |
| Samba | 404,081 | 371,039 | 0.1223 |
| XML | 117,194 | 108,714 | 0.1269 |
| Mozilla | 1,767,879 | 1,695,206 | 0.1135 |
| OOffice | 216,220 | 210,780 | 0.0707 |

Then the frozen parser makes all 18 Calgary files smaller. The aggregate size
decreases from 996,359 to 950,768 bytes. That is a decrease of 45,591 bytes,
or 4.58%. Each file restores exactly.

The controlled Calgary timing uses one warm-up and five measured
repetitions. The median aggregate decode time is 0.903 seconds. For RFC 0015
it is 0.820 seconds. The ratio is 1.102. This is 0.002 more than the 1.10
limit in Section 4. The median aggregate encode time is 1.933 seconds. For RFC
0015 it is 1.679 seconds, so the ratio is 1.15. The peak process RSS is 55,812
KiB.

## 9. Decision

The size hypothesis passes on the development data and the validation data.
The decode-time condition fails by a small margin. Do not make RFC 0016 the
best-ratio profile. Do not change the limit after this measurement.

The decoder does not change, so the parse causes the slower decode. On
Calgary, the priced parse codes 1,895,245 bytes as literals. RFC 0015 codes
936,030 bytes as literals. The priced parse has 79,345 matches. RFC 0015 has
165,917 matches. The decoder must decode each literal with the arithmetic
coder and the byte model. A match byte only updates the model. These block
counts come from a separate instrumented run, not from the timed run.

A follow-up MUST do one of two things. It can set a new decode-time limit
before a new measurement. Or it can decrease the decoder cost of literals. Do
not tune the parser parameters on these results. An enwik9 result requires a
bounded priced encoder. The current research encoder reads all of the input
into memory, so there is no enwik9 result.

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

## 10. Decoder literal-cost follow-up

Section 9 permits a follow-up that decreases the decoder cost of literals.
The 1.10 limit does not change. The follow-up changes only the decoder
implementation. It does not change the `TKLZ` syntax, the arithmetic, the
model, or any artifact byte. The RFC 0015 and RFC 0016 artifacts are the same
before and after the change. The Calgary sizes in Section 8 do not change.

Instruction profiles of the bounded decoder show three large costs for
literals: the byte-model symbol search, range-coder renormalization, and
payload reads of one byte at a time. The follow-up has three changes:

1. Dense byte distributions and normalized order-2 increments keep the sum of
   each group of 16 symbols. The cumulative search examines the group sums
   first, and then the symbols in one group. Each frequency change also
   updates its group sum. Halving calculates all group sums again.
2. The bounded bit reader has its own 64 KiB buffer. It gives bits from a
   64-bit register. Before the change, it read one byte through `BufReader`
   for each eight bits.
3. Renormalization does all half-interval steps, and then all quarter steps.
   It uses leading-bit counts. The equal leading bits of `low` and `high` give
   the number of half steps. After these steps the leading bits are
   different, so a half step cannot follow a quarter step. One quarter step is
   equal to a one-bit left shift, followed by inversion of bit 31. A
   randomized test compares the result with the single-bit loop on 20,000
   states.

The controlled Calgary timing from Section 8 was done again with all three
changes. The median decode time is 0.694 seconds for RFC 0016 and 0.659
seconds for RFC 0015. The ratio is 1.053. A second full run of the same
procedure gives 1.050. Both results are in the 1.10 limit. The median encode
ratios are 1.17 and 1.18. The peak process RSS is 56,180 KiB. Each decode is
exact.

An intermediate run with only the first two changes gave a decode ratio of
1.124. These two changes also decrease the cost of work that literals and
matches share. Thus the ratio increased, but both decoders became faster. The
raw file keeps this run.

RFC 0016 now passes all validation conditions in Section 4. It is the smallest
measured TAK Calgary result. It is not the README profile, because there is no
enwik9 result.

Controlled decoder rows are in
`benchmarks/reports/raw/2026-11-23-lz-priced-decoder-controlled.tsv`, SHA-256
`cab8b54f2b82b3863068ba3a3894f6b79759fbf39f7a68900ae47aeb72bc0639`.

## 11. Bounded encoder

The bounded encoder reads a seekable input in sequence into a 128 MiB ring
buffer. The ring holds the 64 MiB match window, one maximum match of
lookahead, and one 1 MiB read. The encoder writes a literal block only after
it knows the block length. If a literal run is older than the ring, the
encoder reads it again from the input in 64 KiB pages. The encoder keeps a
pricing model, a coding model, the match table, and one parser segment. It does
not keep the complete input, the block list, or the arithmetic payload.

The bounded encoder makes the same artifacts as the buffered encoder. Tests
show this on mixed input and on empty and tiny input. A test with a 64 KiB
ring also makes the encoder read old literal bytes again.

On the complete 51,220,480-byte Mozilla file, both encoders made the same
19,299,991-byte artifact. This artifact is 805,664 bytes, or 4.01%, smaller
than the bounded RFC 0015 artifact of the same file. The bounded RFC 0016
encode took 26.92 seconds with 189,476 KiB peak RSS. The bounded decode took
8.07 seconds with 105,444 KiB peak RSS. It restored the exact input. In the
same session, the bounded RFC 0015 encode took 22.18 seconds and its decode
took 7.45 seconds. These are single screening runs. The Mozilla file includes
the development tail, so this result is not validation evidence.

The bounded encoder agrees with the memory rule for enwik9. There is still no
enwik9 result, because the measurement host could not get the enwik9 file. Do
the measurement once, with no changes, with these commands:

```console
target/release/tak-lz-stream encode-priced-bounded enwik9 enwik9.tklz
target/release/tak-lz-stream decode enwik9.tklz enwik9.out \
  --max-output 1000000000
```

Raw rows are in
`benchmarks/reports/raw/2026-11-23-lz-priced-bounded-mozilla.tsv`, SHA-256
`07b9fd18525e987bf9730ff96c302fa9975437003ea9a4fd6b7592150af3ff25`.
