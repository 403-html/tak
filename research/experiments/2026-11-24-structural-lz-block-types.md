# Accepted structural LZ block types

## Decision

Accept RFC 0019 as the smallest measured TAK artifact on Calgary and enwik9.
The codec removes block-type symbols that the decoder can infer from parser
state and the declared output length. It uses no tensor network because one
bit of parser state has a complete classical representation.

## Development and validation

The ideal opportunity gate saved 0.01333 bits per input byte on the five
Silesia development tails and 0.01295 on Calgary. Every file improved. The
complete artifact retained almost the complete ideal bound.

| Corpus | RFC 0017 bytes | RFC 0019 bytes | Saved bytes | Files improved |
|---|---:|---:|---:|---:|
| Silesia development | 2,696,546 | 2,680,799 | 15,747 | 5/5 |
| Calgary validation | 947,716 | 942,454 | 5,262 | 18/18 |

All outputs restore exactly. Silesia median candidate/control decode ratios
range from approximately 0.91 to 1.095. Calgary ratios range from
approximately 0.78 to 1.05. The largest Silesia complete-process screen is
145,028 KiB. These results pass the frozen size, speed, and memory gates.

The complete artifact rows are in
`benchmarks/reports/raw/2026-11-24-structural-block-type-artifacts.tsv`,
SHA-256 `95890347356e8b5a844d97878328ec48a2e3fca45585b5cf4f2e07043ccfb43c`.
Opportunity rows are in
`benchmarks/reports/raw/2026-11-24-structural-block-type-opportunity.tsv`,
SHA-256 `d285d95aa63786a9b89bd5ffb633ddc2ce139ddb6357529db6baff12b853fe58`.

## Bounded implementation

The bounded encoder uses RFC 0017's 128 MiB input ring. The bounded decoder
uses the 64 MiB match window. A test proves that buffered and bounded encoders
produce byte-identical `TKT9` artifacts. The bounded decoder restores the same
input and rejects corruption and output-limit violations.

## enwik9 result

The bounded encoder produces a 249,762,881-byte artifact from canonical
enwik9. This is 1,935,499 bytes, or 0.769%, smaller than RFC 0017. The bounded
decoder restores all 1,000,000,000 bytes. The input and restored SHA-256 are
both `159b85351e5f76e60cbe32e04c677847a9ecba3adc79addab6f4c6c7aa3744bc`.
The artifact SHA-256 is
`78142e9c826347168c4bdb433dc111ea1d4c7a1885c6b48109e35e31f15dd921`.

Encoding takes 628.62 seconds at 1.59 MB/s with 220,912 KiB peak RSS.
Decoding takes 111.88 seconds at 8.94 MB/s with 96,024 KiB peak RSS. This is
one controlled measurement, not five publishable timing repetitions.

RFC 0019 beats the nominal 322,000,000-byte M1 target by 72,237,119 bytes. It
remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz requires a further 13.65% reduction
from the current TAK artifact. Compression and decompression throughput remain
approximately 31.4x and 22.4x below their aspirational targets.

Raw enwik9 data is in
`benchmarks/reports/raw/2026-11-24-structural-block-type-enwik9.tsv`, SHA-256
`cc8dd04074113e374b6941a40e6c13b75540bfc8b5601a5fe41717e71ed37ade`.

## Limitations and next experiment

This result removes redundant syntax but does not improve literal or match
probabilities. It closes only 5.37% of RFC 0017's remaining enwik9 gap to xz.
The encoder is slower than RFC 0017 in this single run, while decoding is
similar. Ratio and throughput both remain far from the project targets.

Do not add previous-length, distance, or multi-block type contexts without a
new opportunity gate. A tensor attempt still requires a new RFC that names a
specific longer-context dependency the rejected bigram factorization cannot
represent and that needs meaningful parameter sharing. Increasing rank is not
justified.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: branch `research/lz-block-state-opportunity` working tree.
- Encode command: `tak-lz-stream encode-structural enwik9 enwik9-rfc0019.tkt9`.
- Decode command: `tak-lz-stream decode-structural enwik9-rfc0019.tkt9 enwik9.out --max-output 1000000000`.
