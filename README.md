# TAK Pack

<sub>For coding agents: <a href="AGENTS.md">project instructions</a>.</sub>

TAK Pack is an experimental, lossless compression project. The research goal
is to test whether compact tensor-network models can improve practical lossless
compression. The project does not yet claim that they can.

The repository currently contains the Phase 1 classical control experiment:

- a deterministic integer arithmetic coder;
- adaptive order-0 and order-1 byte models;
- a research-only adaptive order-2 control;
- a small, versioned experimental `.takc` stream;
- exact round-trip and malformed-input tests; and
- a benchmark command that reports size and wall-clock throughput.

The format is experimental and has no compatibility guarantee yet. Do not use
it for durable storage.

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
M1 by 6,421,234 bytes and beats measured gzip by 7,013,222 bytes. It remains
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

