# RFC 0015: Two-candidate LZ opportunity gate

- Status: Bounded enwik9 result accepted
- Date: 2026-11-23
- Scope: Classical parser diagnostic only
- Format change: None

## 1. Problem statement

RFC 0014 stores one recent position for each exact 8-byte key. Repeated keys
can have different continuations. The most recent occurrence can produce a
shorter match than the preceding occurrence. The parser cannot select that
older, longer match.

RFC 0014 improves enwik9 by 10.82% relative to RFC 0009, but remains 65,756,437
bytes larger than xz. Measure this parser limitation before changing the
artifact or adding a more complex model.

## 2. Hypothesis

Keeping the two most recent positions for each exact key reduces ideal RFC 0014
code length by at least 0.02 bits per input byte in aggregate on the five
established Silesia development groups. It MUST improve at least three groups.

The diagnostic MUST use the same literal model, metadata accounting, window,
maximum match length, minimum match length, and greedy parser as RFC 0014.

## 3. Method

Each direct-mapped table entry stores one exact key and its two most recent
positions. A different key replaces the complete entry. At each input
position, compare both valid candidates. Select the longer match. Select the
most recent candidate when lengths are equal.

Run one-candidate and two-candidate parsing on identical inputs. Report complete
ideal bits, incremental bits per byte, match coverage, block counts, wall time,
and peak RSS. Use final-10% Silesia partitions for development. Freeze the
method before Calgary validation.

## 4. Stop condition

Reject the mechanism if it misses either size condition in Section 2. Do not
add more candidates or change the hash table on the same development results.

If the development gate passes, validate the frozen mechanism on Calgary. A
complete artifact experiment MAY proceed only if Calgary remains smaller in
aggregate and encoder memory remains below one gigabyte. Decoder complexity and
memory MUST remain unchanged.

## 5. Tensor scope

This RFC does not authorize a tensor model. It tests a classical search
limitation. A tensor attempt still requires a separate RFC that identifies a
specific longer-context dependency that RFC 0013 and this parser cannot
represent. Increasing tensor rank alone remains prohibited.

## 6. Compatibility and security

The diagnostic does not change the `TKLZ` artifact syntax. Different valid
encoders can already select different block sequences. Existing bounds on
length, distance, output, arithmetic state, and checksum coverage remain
unchanged.

## 7. Testing strategy

The one-candidate mode MUST reproduce the RFC 0014 diagnostic. Unit tests MUST
show a repeated key for which the older candidate is longer. Every reported
block sequence MUST cover the complete input without overlap or omission.

## 8. Results

The Silesia development gate passes. Two candidates reduce ideal code length
on all five groups. Aggregate incremental savings are 233,134.515 bits across
9,451,681 bytes, or 0.02467 bits per byte. The frozen Calgary validation also
improves all 18 files. Aggregate incremental savings are 66,062.643 bits across
3,251,493 bytes, or 0.02032 bits per byte.

The complete Calgary artifact is 996,359 bytes. It is 8,319 bytes, or 0.83%,
smaller than RFC 0014. It is also 4,336 bytes smaller than RFC 0010, the prior
best TAK result on Calgary. Every file restores exactly. The artifact remains
75,286 bytes larger than Zstandard-19 and 110,499 bytes larger than xz.

Five controlled repetitions after one warm-up give a median aggregate encode
time of 1.751 seconds, 1.14 times the one-candidate path. Median decode time is
0.755 seconds, 1.02 times the control. Peak process RSS is 49,952 KiB. Retain
the mechanism as the best-ratio TAK parser. Do not claim competitive
compression.

## 9. Decision

Accept the Calgary artifact. The mechanism adds no decoder state and produces
valid `TKLZ` artifacts. Permit one bounded enwik9 measurement with the frozen
two-candidate parser. Do not add a third candidate or tune parser parameters.

Raw Silesia rows are in
`benchmarks/reports/raw/2026-11-23-lz-two-candidate-silesia.tsv`, SHA-256
`be58210fb933f4800c7a7f4561d778e3df65b177fa6ba0709c0258b1b1d42844`.
Raw Calgary opportunity rows are in
`benchmarks/reports/raw/2026-11-23-lz-two-candidate-calgary.tsv`, SHA-256
`532112eff1afa22c7f3ad526a2955e2458831eeb811495f60042f83f9eae327c`.
Complete artifact rows are in
`benchmarks/reports/raw/2026-11-23-lz-two-candidate-artifact-calgary.tsv`,
SHA-256
`c152db88edd84258f9a5a0979c48171630853d2c94285d585604a320340a7b63`.
Controlled aggregate rows are in
`benchmarks/reports/raw/2026-11-23-lz-two-candidate-controlled.tsv`, SHA-256
`d097ccf7a0b3c3f3f291e9d84c174813322c6ba272d06847f60b5831829fada9`.

## 10. Enwik9 result

The bounded two-candidate encoder produces a 276,339,048-byte enwik9 artifact.
It is 5,091,585 bytes, or 1.81%, smaller than RFC 0014 and 39,239,718 bytes, or
12.43%, smaller than RFC 0009. The decoder restores the exact one-billion-byte
input and SHA-256.

Encoding takes 715.42 seconds, or 1.40 MB/s, with 54,436 KiB peak RSS. Decoding
takes 160.91 seconds, or 6.21 MB/s, with 94,992 KiB peak RSS. These are
single-run measurements. Memory remains bounded, but both throughput targets
fail by large margins.

RFC 0015 beats M1 by 45,660,952 bytes. It remains 41,010,109 bytes larger than
Zstandard-19, 60,664,852 bytes larger than xz, and 79,339,048 bytes larger than
M2. It must remove 21.95% of its current bytes to match xz and 28.71% to reach
M2.

Accept RFC 0015 as the best measured TAK ratio profile. Stop candidate-count
tuning. A later ratio experiment MUST identify and gate a different mechanism.
The tensor restriction in Section 5 remains in force.

Raw enwik9 data is in
`benchmarks/reports/raw/2026-11-23-lz-two-candidate-enwik9.tsv`, SHA-256
`08152a74c67cf9c7682d72d8f934afd043b44da105b02534af6a224f81341446`.
