# Phase 1 research plan

## Question

Can a tensor-network probability model reduce total compressed size enough to
justify its model metadata, update cost, memory, and decoder complexity?

This repository does not yet answer that question. The current order-0 and
order-1 models are controls. They establish the entropy-coding path and the
measurement contract before a tensor model is introduced.

## Review summary

Established general-purpose lossless compressors combine reversible parsing or
transforms with probability modeling and entropy coding. Important controls
include DEFLATE (LZ77 plus Huffman coding), LZMA2 (dictionary parsing plus range
coding), Zstandard (LZ-family parsing plus FSE/Huffman coding), PPM, and context
mixing. These methods set a high bar because their decoders are mature and
their metadata costs are small relative to large inputs.

Tensor Train (TT) and Matrix Product State (MPS) factorizations can represent a
high-order tensor as a chain of lower-order cores. Published work has applied
MPS models to supervised learning and generative probability models. This
suggests three testable opportunities for lossless sequence modeling:

1. Factor a large table of discrete context probabilities into small cores.
2. Adapt the effective rank by block when the size reduction pays for metadata.
3. Use a tensor prediction only as one input to a classical mixture, so that
   poorly modeled blocks fall back safely.

The hard problems are also clear. Normalizing 256 symbol probabilities can
require many contractions. Floating-point training can make independent
decoders disagree. Core parameters can cost more bytes than they save. Online
updates can also destroy the speed and bounded-memory properties required by a
practical decoder.

Useful starting references include:

- RFC 1951, *DEFLATE Compressed Data Format Specification*.
- RFC 8878, *Zstandard Compression and the application/zstd Media Type*.
- I. V. Oseledets, “Tensor-Train Decomposition,” SIAM Journal on Scientific
  Computing 33(5), 2011.
- E. M. Stoudenmire and D. J. Schwab, “Supervised Learning with Tensor
  Networks,” NeurIPS 2016.
- Z.-Y. Han et al., “Unsupervised Generative Modeling Using Matrix Product
  States,” Physical Review X 8, 031012, 2018.

This is a focused engineering review, not a systematic literature review.

## Smallest tensor experiment

The first tensor experiment SHOULD be offline and block based:

1. Fix a byte-context factorization and rank before running the held-out set.
2. Quantize every transmitted core to a specified integer representation.
3. Serialize all core parameters into the measured stream.
4. Convert contracted scores to integer cumulative frequencies with a fully
   specified normalization rule.
5. Use the existing arithmetic coder without modification.
6. Compare with order-0 and order-1 using identical blocks and runner settings.
7. Report contraction count, model bytes, encode time, decode time, and peak
   resident memory in addition to compressed size.

The first experiment SHOULD use ranks 1, 2, 4, and 8. It MUST include rank 1
and a disabled-tensor run as ablations. Stop the line of work if repeated
held-out tests do not recover model metadata or if decoding is impractical.

## Why the tensor model is not implemented yet

Adding an unvalidated “TT-like” predictor would make the name stronger than the
evidence. The next implementation must first define the factorization,
quantization, normalization, and metadata accounting in a project RFC. The
classical frame in this repository provides the controlled path needed for
that work.

