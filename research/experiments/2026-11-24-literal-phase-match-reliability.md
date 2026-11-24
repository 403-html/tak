# Accepted literal-phase match reliability

## Decision

Accept RFC 0017 as the smallest measured TAK enwik9 artifact. The candidate
conditions exact-suffix match confidence on the decoder-known offset inside a
literal block. It uses four independent six-expert integer mixtures. It does
not use a tensor network because 24 integer weights represent the measured
dependency directly.

Do not tune phase boundaries or experts on enwik9. A later tensor experiment
still requires a new RFC that identifies an additional longer-context
dependency and beats this complete classical control.

## Complete development and held-out results

The five Silesia development tails decrease from 2,702,364 to 2,696,546
bytes. The candidate saves 5,818 bytes, or 0.00492 bits per input byte. Every
artifact is smaller and restores the exact input. Median candidate/control
decode ratios range from 1.05 to 1.17, below the 1.25 limit.

The largest development or held-out screening peak is 145,024 KiB, below the
one-gigabyte gate. Memory rows are in
`benchmarks/reports/raw/2026-11-24-literal-phase-memory-screen.tsv`, SHA-256
`172be6d8443a2d944ab53a0c195840e2f813c4c2db4c5c3846b54780aee0e86b`.

The frozen method then passed three previously unused final-10% Silesia tails:

| File | Type | RFC 0016 bytes | RFC 0017 bytes | Saved bytes | Median decode ratio |
|---|---|---:|---:|---:|---:|
| Reymont | Text | 183,887 | 182,998 | 889 | 1.10x |
| NCI | Chemical database | 285,532 | 283,679 | 1,853 | 0.95x |
| X-ray | Binary medical data | 444,668 | 444,614 | 54 | 1.08x |

All held-out outputs restore exactly. The held-out aggregate decreases by
2,796 bytes. The controlled raw repetitions are in
`benchmarks/reports/raw/2026-11-23-literal-phase-controlled.tsv`, SHA-256
`16907f56535c48e76d01e549fb6e7182c3d442f6c7c910a8858575aae41388d3`.

## Bounded implementation

The bounded encoder reuses RFC 0016's price-based parser and 128 MiB input
ring. The bounded decoder reuses the 64 MiB match window. A release test proves
that bounded and buffered encoders produce byte-identical `TKP7` artifacts.
The bounded decoder restores the same input.

An attempted replacement of the RFC's 128-bit mixture arithmetic with 64-bit
operations changed artifacts and regressed the development aggregate by 2,278
bytes. It was reverted. The final implementation uses the specified 128-bit
intermediates. A decoder-only cache avoids repeated calculations without
changing artifact bytes.

## enwik9 result

The bounded encoder produces a 251,698,380-byte artifact from the canonical
1,000,000,000-byte enwik9 input. This is 280,109 bytes, or 0.111%, smaller
than RFC 0016. The bounded decoder restores all bytes. The input and restored
SHA-256 are both
`159b85351e5f76e60cbe32e04c677847a9ecba3adc79addab6f4c6c7aa3744bc`.
The artifact SHA-256 is
`2342b9845c52b397f539294d4fe9cf11cc50b9c3c9a57c7971a6b458b25d6415`.

Encoding takes 553.39 seconds at 1.81 MB/s with 220,796 KiB peak RSS.
Decoding takes 107.87 seconds at 9.27 MB/s with 96,064 KiB peak RSS. This is a
single controlled measurement, not five publishable timing repetitions.

RFC 0017 beats the nominal 322,000,000-byte M1 target by 70,301,620 bytes. It
remains 16,369,441 bytes above Zstandard-19, 36,024,184 bytes above xz, and
54,698,380 bytes above M2. Matching xz requires a further 14.31% reduction
from the current TAK artifact. The compression and decompression throughput
targets remain approximately 27.7x and 21.6x away, respectively.

Raw data is in
`benchmarks/reports/raw/2026-11-24-literal-phase-enwik9.tsv`, SHA-256
`ace40ef031dafa52357a7c2f1788a08777337a05b3c7a1a74df1ac8aede7bd2e`.

## Limitations and next experiment

The size gain is real but small. Phase conditioning does not close a material
part of the xz gap. Do not add more phase bins or bonus experts without a new
held-out opportunity result.

The best next step is to profile the remaining literal cost under RFC 0017 and
identify a decoder-known dependency with a larger complete-artifact upper
bound. Tensor work is justified only if that dependency requires parameter
sharing that a small classical table cannot provide.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `f819a84` plus the RFC 0017 working tree.
- Encode command: `tak-lz-stream encode-phase enwik9 enwik9-rfc0017.tkp7`.
- Decode command: `tak-lz-stream decode-phase enwik9-rfc0017.tkp7 enwik9.out --max-output 1000000000`.
