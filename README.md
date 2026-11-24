# TAK Pack

<sub>For coding agents: <a href="AGENTS.md">project instructions</a>.</sub>

TAK is an experimental, lossless compression project written in Rust. It explores classical and **tensor-network-based** compression methods, with a focus on compression ratio, speed, and memory efficiency.

The current classical research model beats gzip -9 on Calgary and enwik9, but remains behind xz and Zstandard-19.

**Status**: _Experimental_. The best-performing model is not yet integrated into the main CLI. Formats are unstable and should not be used for long-term storage.

## Benchmarks

### enwik9 (1 GB)

| Compressor | Compressed size |
|---|---:|
| xz -9 | 215.67 MB |
| Zstandard -19 | 235.33 MB |
| **TAK (RFC 0019)** | **249.76 MB** |
| gzip -9 | 322.59 MB |

**TAK performance:** 1.59 MB/s compression with 220,912 KiB peak RSS and
8.94 MB/s decompression with 96,024 KiB peak RSS. Exact SHA-256-verified
restoration. This bounded reference path prioritizes evidence over speed.

[Full benchmark results](research/experiments/2026-11-24-structural-lz-block-types.md)

## Build

```console
cargo build --release
cargo test --workspace
```

## Use

```console
cargo run --release -p tak-cli -- compress input.txt -o input.takc
cargo run --release -p tak-cli -- decompress input.takc -o restored.txt
cargo run --release -p tak-cli -- verify input.takc
cargo run --release -p tak-cli -- benchmark input.txt
```

`compress` tests each implemented method and stores the smallest valid frame.
Use `--method raw`, `--method order-0`, or `--method order-1` to run a controlled
experiment.

## Scope

This is a research foundation, not a stable codec or archive application.
The `.takp` archive container, production tensor model, FFI, and stable format
specification will be added only after measurements justify them.

## Tensor screening experiment

The `tak-tensor` crate preserves rejected RFC 0001 as a reproducible research
artifact. It is not a TAK-C codec. Run one quantized bigram-factorization rank
with:

```console
cargo run --release -p tak-tensor --bin tak-tensor-experiment -- \
  input.bin --rank 4 --iterations 20 --output result.tkrx
```

By default, the first 90% of the input trains the model and the last 10% is the
held-out compression input. Use `--train-bytes` to record an exact split. The
reported artifact size includes the header, quantized cores, and payload.
The output also separates full-precision factorization loss, core quantization
loss, and integer-frequency rounding loss in bits per byte.

RFC 0002 passed its classical gate: adaptive two-byte context improves complete
size on the current development corpus. Its three-core tensor train was then
implemented and rejected because it lost to both classical controls. Neither
research method has a TAK-C identifier.

RFC 0003 tested a specific third-order dependency with a sparse adaptive
classical control. It was rejected because it produced larger artifacts and
slower decoding than order-2 on every tested group. It does not authorize
another tensor implementation. A new tensor structure still requires a new
dependency mechanism and a stop condition that address RFC 0002's measured
factorization loss.

RFC 0004 recovered a small size benefit by using order-3 only for mature
contexts. Threshold 64 beat order-2 by 2.2% to 3.7% on three development
groups. An exact sparse gap decoder reduced median slowdown to 1.30 to 1.78
times, so the classical development gate passed. Independent Canterbury
validation then failed: backoff was larger than order-2 in all three held-out
groups. The model remains a research artifact, not a TAK-C method or
authorization for another tensor model.

RFC 0005 tested whether an adaptive order-1 parent could smooth cold order-2
contexts. The raw-count blend improved Canterbury prose and source/markup, but
lost on all five Silesia groups and was 4.9% larger in aggregate. It is
rejected. A future smoothing experiment must normalize the parent prior instead
of testing more raw-count weights. This result does not authorize tensor work.

RFC 0006 applies that normalization with a fixed prior mass. Strength 4 is
13.7% smaller than order-2 in aggregate and passes the size condition on all
eight development groups. Dense local rows reduced controlled decoder slowdown
to 1.26–1.82 times with 54,040 KiB peak process memory, so the development gate
passes. At that stage, independent validation was still required before any
codec claim or TAK-C method could be considered.

Fixed strength 4 subsequently passed an untouched Calgary validation: it was
19.6% to 38.8% smaller than order-2 across all three groups, decoded 1.48 to
1.73 times slower, and used 9,216 KiB peak process memory. Its complete size
was still 4.5% larger than gzip, 20.2% larger than Zstandard level 19, and
25.0% larger than xz under matching independent-file accounting. The model
remains research-only.

The first enwik9 screen is less favorable. Fixed strength 4 produced a
368,643,002-byte artifact, only 0.87% smaller than order-2. It misses the
nominal 322,000,000-byte M1 target by 46,643,002 bytes and must remove 12.65%
of its current bytes to reach M1. Same-input gzip produced 322,591,988 bytes,
xz produced 215,674,196 bytes, and Zstandard level 19 produced 235,328,939
bytes. TAK encoded at 8.10 MiB/s and decoded at 4.43 MiB/s in this single-run
screen. That non-streaming runner peaked at 3,665,840 KiB, which made streaming
and bounded-memory work mandatory before codec promotion.

RFC 0007 now provides a byte-identical bounded-memory path. Its enwik9 screen
produced the same 368,643,002-byte complete size, encoded at 7.96 MiB/s,
decoded at 4.38 MiB/s, and peaked at 13,216 KiB for each isolated process. The
restored output matched all 1,000,000,000 input bytes. This fixes the prior
memory failure but does not improve size or speed: TAK remains 46,643,002
bytes above M1 and must remove 12.65% of its current artifact to reach that
target.

RFC 0008 identifies a different dependency for the next research stage. The
byte following the most recent exact 8-byte suffix reduced ideal cross-entropy
by 0.192 to 0.513 bits per byte across all five Silesia development groups
after conditioning on RFC 0006. This is a diagnostic result, not compressed
size. A bounded classical match artifact must measure its actual benefit before
any tensor implementation or milestone claim.

RFC 0009 implements the required bounded classical control. Its fixed
1,048,576-slot exact-match table reduced complete size by 6.39% to 20.54% on
all five Silesia development groups and by 11.26% in aggregate. Peak complete
process memory was 115,040 KiB, and every artifact round-tripped exactly. This
is development evidence; controlled timing and independent validation remain
required before an enwik9 run or tensor proposal.

The unchanged RFC 0009 model subsequently passed Calgary validation. It was
7.04% smaller than RFC 0006 in aggregate and 2.8% smaller than gzip under
matching independent-file accounting. Median group decode ratios remained
below 1.86x, peak process RSS was 21,720 KiB, and every decode was exact. A
bounded streaming `TKM8` path was required before enwik9 measurement.

The byte-identical streaming path then produced a 315,578,766-byte enwik9
artifact and restored all 1,000,000,000 bytes exactly. This achieves nominal
M1 by 6,421,234 bytes and **beats measured gzip by 7,013,222 bytes**. It remains
80,249,827 bytes larger than Zstandard level 19 and 99,904,570 bytes larger
than xz. Encoding reached 6.54 MiB/s and decoding 4.11 MiB/s with about 29 MiB
peak RSS, so both compression ratio and speed remain major research targets.

RFC 0010 tested one stronger exact-match bonus against the Calgary xz and
Zstandard targets. It reduced RFC 0009 by another 2.80% but stopped at
1,000,695 bytes, still 79,622 bytes above Zstandard-19 and 114,835 above xz.
The experiment is rejected; more bonus tuning is prohibited. A future ratio
experiment must test a new mechanism such as explicit match-run tokens.

RFC 0011 measured that match-run opportunity before defining another format.
Runs of at least four bytes covered 30.45% of Calgary, but a raw marker plus
LEB128 length was 26,921 bits worse than RFC 0009 on those bytes. Length-8 runs
saved only 131,959 gross bits before control costs, far below the xz gap. The
raw-token design is rejected; future work requires entropy-coded tokens or a
genuine LZ parser rather than more match-weight tuning.

RFC 0012 then identified the first tensor-specific opportunity since the
rejected bigram work. Match reliability depends on distance and correct-run
state, and exact 8-byte suffix identity adds another 0.0785 bits per matched
event in weighted development log loss. RFC 0013 proposes a new eleven-core
tensor train over those explicit modes. One shared MPS transition processes
the eight ordered suffix bytes, cutting the smallest dependency-capable core
payload from 16,456 to 2,120 bytes. This is not a rank increase: it must beat
the 24-cell and sparse suffix classical controls after quantized core metadata
is counted before any tensor codec is implemented.

That RFC 0013 tensor has now been rejected. Ranks 1, 2, 4, and 8 failed the
full-precision gate on Dickens, XML, and OOffice, which makes the required
three-of-five result unreachable. Rank 2 was the best dependency-capable model
but still lost to the stronger classical control on every measured group.
Quantization was accurate; representation quality and metadata were the
failures. Higher ranks and more epochs are not justified by this experiment.

RFC 0026 later measured one specific representation limitation rather than
retrying rank. The shared RFC 0013 byte transition cannot specialize its
mapping by suffix position. Position-specific classical tables improve a
shared-position control on all five Silesia tails by 0.023 to 0.141 bits per
matched event. Weighted loss is 0.4887 bits/event, slightly below the sparse
exact-suffix control's 0.4979. This permits a new untied position-aware tensor
design with both classical models as mandatory controls. It is not yet a
tensor or compressed-size improvement.

RFC 0027 tested that position dependency directly at fixed rank two. Fully
untied cores beat the shared tensor on Dickens, XML, and OOffice, with weighted
loss falling from 0.5908 to 0.5645 bits per event. The position-specific
classical control remained stronger at 0.4674. The compact position-modulated
tensor did not retain the untied gain, and metadata made both candidates much
worse. The exact representations are rejected without testing higher rank or
tuning the optimizer. This does not rule out other tensor topologies or
higher-order interactions.

RFC 0014 returns to a classical entropy-coded LZ mechanism. Its fixed greedy
parser passed four of five Silesia development groups with 0.2757 aggregate
ideal bits per byte saved. Frozen Calgary validation remained positive but
saved only about 23,860 ideal bytes, far below the 143,620-byte RFC 0009-to-xz
gap. One complete artifact is permitted to test whether this modest margin
survives integer coding; no xz or M2 claim follows from the diagnostic.

The complete RFC 0014 `TKLZ` artifact then saved 24,802 bytes, or 2.41%, over
RFC 0009 on Calgary and restored all 18 files exactly. Screening decode time
was 0.56x the RFC 0009 time with 36,756 KiB peak RSS. It remains 3,983 bytes
larger than retained RFC 0010, 83,605 bytes above Zstandard-19, and 118,818
bytes above xz. Keep it as a fast classical research component, not a new
compression-ratio record.

Five controlled Calgary repetitions confirmed median encode and decode ratios
of 0.74x and 0.54x versus RFC 0009.

Bounded RFC 0014 encoding and decoding now produce the same artifact and exact
restoration as the buffered path. On the 51,220,480-byte Mozilla input, bounded
encoding used 68,768 KiB peak RSS and bounded decoding used 101,792 KiB. The
encoder took 24.06 seconds, 3.24x the buffered candidate's internal time. This
is a bounded reference path, not a throughput result.

The bounded path then produced a 281,430,633-byte enwik9 artifact and restored
all 1,000,000,000 bytes exactly. It improves RFC 0009 by 34,148,133 bytes, or
10.82%. It beats M1 by 40,569,367 bytes, but remains 46,101,694 bytes above
Zstandard-19, 65,756,437 bytes above xz, and 84,430,633 bytes above M2.
Encoding reached only 1.65 MB/s and decoding 7.21 MB/s, so ratio and throughput
both remain research blockers.

RFC 0015 tests one additional exact-key occurrence without changing the
decoder. Its frozen parser improves all five Silesia development groups and all
18 Calgary files. The complete Calgary artifact is 996,359 bytes, 8,319 bytes
smaller than RFC 0014 and 4,336 bytes smaller than the prior TAK record. It
remains 75,286 bytes above Zstandard-19 and 110,499 bytes above xz. Controlled
encoding is 1.14x slower; decoding is 1.02x slower.

The bounded RFC 0015 enwik9 artifact is 276,339,048 bytes, 5,091,585 bytes or
1.81% smaller than RFC 0014. It restores all one billion bytes exactly. It
beats M1 by 45,660,952 bytes but remains 41,010,109 bytes above Zstandard-19,
60,664,852 bytes above xz, and 79,339,048 bytes above M2. Do not test more
candidate counts; the next ratio experiment must identify another mechanism.

RFC 0016 measured the RFC 0015 token costs. Approximately 35% of its matches
cost more than the byte model needs for the same bytes. A price-based parser
compares the exact literal cost with the match cost. The `TKLZ` decoder does
not change. The parser made all five Silesia development groups smaller, by
5.02% in aggregate. Then it made all 18 Calgary files smaller. The Calgary
total is 950,768 bytes, 4.58% less than RFC 0015. That is 29,695 bytes more
than Zstandard-19 and 64,908 bytes more than xz.

The first controlled decode-time ratio was 1.102. The limit was 1.10. A
decoder-only change then decreased the ratio to 1.053. It did not change any
artifact byte. Thus RFC 0016 is now the smallest validated TAK Calgary result.
The bounded encoder then produced a 251,978,489-byte enwik9 artifact and
restored all one billion bytes exactly. This is 24,360,559 bytes, or 8.82%,
smaller than RFC 0015. It beats M1 by 70,021,511 bytes, but remains 16,649,550
bytes above Zstandard-19, 36,304,293 bytes above xz, and 54,978,489 bytes
above M2. Encoding reached 1.84 MB/s and decoding reached 8.95 MB/s.

RFC 0017 measures a specific longer-context dependency that the rejected
bigram tensor cannot represent: exact-suffix match reliability changes with
the decoder-known offset inside a literal block. Four six-expert integer
mixtures improve all five Silesia development tails and three previously
unused held-out tails. A tensor is not useful for only 24 integer weights.

The bounded RFC 0017 enwik9 artifact is 251,698,380 bytes, 280,109 bytes or
0.111% smaller than RFC 0016. It restores all one billion bytes exactly. It
beats M1 by 70,301,620 bytes but remains 16,369,441 bytes above Zstandard-19,
36,024,184 bytes above xz, and 54,698,380 bytes above M2. Matching xz requires
a further 14.31% reduction from the current artifact. Encoding reached 1.81
MB/s and decoding reached 9.27 MB/s. Do not add phase bins or experts without
a new held-out opportunity result.

The next diagnostic tested adaptive coding of LZ distance-remainder bits by
bit position and distance bucket. It saved 0.01753 bits per input byte on the
Silesia development tails but only 0.00436 on Calgary, with improvements on
just 3 of 18 Calgary files. Even a zero-overhead per-file oracle can remove
only about 2.1 KB, or 3.3% of the Calgary xz gap. The mechanism is rejected;
do not add mixtures, more contexts, or a tensor for distance remainders.

RFC 0018 tested exact 3-to-7-byte matches inside RFC 0017 literal blocks. Its
optimistic opportunity model improved all five Silesia tails and all 18 Calgary
files, but complete accounting reversed the result. The artifact improved only
two of five development files and grew by 23,048 bytes in aggregate. Exact
restoration passed. The method is rejected without tuning, and Calgary and
enwik9 were not run. RFC 0017 remained the benchmark leader after this test.

RFC 0019 removes LZ block-type symbols that are already implied by the
preceding block and declared output length. The frozen complete artifact
improves all five Silesia development files and all 18 Calgary files. The
bounded enwik9 artifact is 249,762,881 bytes, 1,935,499 bytes or 0.769% smaller
than RFC 0017, and restores all one billion bytes exactly. It beats M1 by
72,237,119 bytes but remains 14,433,942 bytes above Zstandard-19, 34,088,685
bytes above xz, and 52,762,881 bytes above M2. Matching xz requires a further
13.65% reduction. Encoding reached 1.59 MB/s and decoding reached 8.94 MB/s.

RFC 0020 tested whether match-length and distance buckets depend usefully on
whether a match follows a literal or another match. The frozen two-state model
improved only one of five Silesia development files and added about 4,993 ideal
bits in aggregate. It is rejected before artifact, Calgary, or enwik9 work.
Do not add previous-length, previous-distance, or multi-block state contexts
on these corpora without a different measured dependency.

RFC 0028 tested a fixed-memory two-way parser table while retaining exactly
two occurrences per equal key and the existing `TKT9` decoder. It saved 1,798
bytes and improved all five Silesia development files. On Calgary it saved
791 bytes but improved only 10 of 18 files; eight were byte-identical. This
fails the frozen 15-file transfer gate. Associativity and replacement tuning
are prohibited, and enwik9 was not run.

RFC 0029 tested learning the order-2 counts from literals while keeping
match history. It lowered ideal literal cost on all five Silesia tails
and on 10 of 18 Calgary files. The Calgary gate required 12 improvements.
The experiment ended before artifact implementation.

RFC 0030 added a three-byte suffix predictor for literals without an
eight-byte prediction. Four of five Silesia tails improved. The aggregate
saved 0.012735 ideal bits per input byte, below the 0.016 development gate.
The experiment ended before Calgary.

Reproduce the RFC 0002 tensor artifact with:

```console
cargo run --release -p tak-tensor --bin tak-tensor-order2-experiment -- \
  input.bin --rank 8 --iterations 20 --output result.tkr3
```

Run the RFC 0003 classical gate with:

```console
cargo run --release -p tak-tensor --bin tak-order3-gate -- input.bin
```

Reproduce the RFC 0004 threshold screen with:

```console
cargo run --release -p tak-tensor --bin tak-order3-backoff-gate -- input.bin
```

Reproduce the rejected RFC 0005 smoothing screen with:

```console
cargo run --release -p tak-tensor --bin tak-order2-smoothing-gate -- input.bin
```

Run the preliminary RFC 0006 screen with:

```console
cargo run --release -p tak-tensor --bin tak-order2-normalized-gate -- input.bin
```

Run its RFC 0007 bounded-memory artifact path with:

```console
cargo run --release -p tak-tensor --bin tak-order2-normalized-stream -- \
  encode input.bin output.tkn2 --strength 4
cargo run --release -p tak-tensor --bin tak-order2-normalized-stream -- \
  decode output.tkn2 restored.bin --max-output 1000000000
```

Run the RFC 0008 dependency diagnostic with:

```console
cargo run --release -p tak-tensor --bin tak-match-dependency -- \
  input.bin --tail-bytes 1000000
```

Run the RFC 0009 bounded classical gate with:

```console
cargo run --release -p tak-tensor --bin tak-match8-gate -- \
  input.bin --tail-bytes 1000000
```

## License

Licensed under Apache-2.0.

Copyright 2026 Tymoteusz "403-html" Stępień.

