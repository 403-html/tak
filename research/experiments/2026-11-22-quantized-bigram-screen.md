# Preliminary experiment: quantized bigram factorization

## Status

Preliminary negative result on one development corpus. RFC 0001 remains
experimental because the required held-out dataset groups have not been run.

## Question

Can a two-core, nonnegative factorization of byte-bigram counts beat the
adaptive order-1 control after the quantized cores are included in the artifact?

## Method

- Input: `enwik8`, 100,000,000 bytes.
- SHA-256:
  `2b49720ec4d78c3c9fabaee6e4179a5e997302b3a70029f30f2d582218c024a8`.
- Training partition: bytes 0 through 89,999,999.
- Test partition: bytes 90,000,000 through 99,999,999.
- Factorization updates: 20.
- Ranks: 1, 2, 4, and 8.
- Quantization: unsigned 16-bit cores, as specified by RFC 0001.
- Entropy coder: the same integer arithmetic coder used by the controls.
- CPU: Intel Xeon Platinum 8573C, x86-64.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.

This is development data. It is not a held-out multi-domain result. Each rank
was measured once after the implementation tests. The times below are useful
for screening, not for performance claims.

## Results

| Model | Model bytes | Complete bytes | Train (s) | Encode (s) | Decode (s) | Round trip |
|---|---:|---:|---:|---:|---:|---|
| Tensor rank 1 | 1,024 | 6,432,914 | 0.073 | 1.006 | 1.225 | Pass |
| Tensor rank 2 | 2,048 | 6,259,004 | 0.063 | 0.921 | 1.147 | Pass |
| Tensor rank 4 | 4,096 | 5,996,886 | 0.092 | 0.983 | 1.136 | Pass |
| Tensor rank 8 | 8,192 | 5,638,609 | 0.092 | 0.881 | 1.109 | Pass |
| Adaptive order-1 control | 0 | 4,853,625 | n/a | 1.000 | 1.142 | Pass |

All tensor sizes are physical `TKRX` artifact sizes. They include the 28-byte
header, both quantized cores, and the arithmetic-coded payload. The control is
a complete TAK-C frame.

The separately measured rank-8 experiment process peaked at 123,884 KiB. That
number includes the complete 100 MB source held by the experiment harness and
is not a decoder-only memory measurement. The control compression and
decompression processes peaked at 20,384 KiB and 15,648 KiB, respectively.
These RSS values are not directly comparable until the harness isolates model
training from decoding.

## Loss attribution

A follow-up diagnostic measured held-out cross-entropy at three stages. The
calculation aggregates the 65,536 possible byte bigrams and uses the same
add-one smoothing mass as the integer model.

| Rank | Full-precision factorization (bpb) | Quantized cores (bpb) | Integer frequencies (bpb) |
|---:|---:|---:|---:|
| 1 | 5.158928 | 5.143281 | 5.145489 |
| 2 | 5.014667 | 4.999019 | 5.005542 |
| 4 | 4.804840 | 4.789923 | 4.794209 |
| 8 | 4.518284 | 4.502694 | 4.504311 |

At rank 8, 16-bit core quantization improves held-out cross-entropy by 0.015590
bits per byte. This small improvement is consistent with regularization or
rounding noise. Integer frequency rounding then costs 0.001617 bits per byte.
Neither stage explains the gap to the adaptive order-1 control.

The physical rank-8 artifact uses 4.510887 bits per input byte. The difference
from the 4.504311-bit integer model is 0.006576 bits per byte and includes the
model, header, and arithmetic-coder finalization. The control's physical stream
uses 3.882900 bits per byte. The dominant deficit is therefore the predictive
model, not quantization, metadata, or entropy-coder overhead.

## Finding

Higher tested ranks monotonically improved compressed size, and rank 8 decoded
at approximately the same speed as the adaptive order-1 control. However, the
best tensor artifact was 784,984 bytes (16.2%) larger than the control. Model
metadata was only 8,192 bytes, so metadata is not the primary problem. The
two-core factorization does not predict the held-out byte distribution well
enough.

This result does not support adding a tensor method to TAK-C. It also does not
test longer-context TT or MPS models.

## Next experiment

Run the same fixed ranks on source, JSON, logs, documents, and binary held-out
data. Do not spend effort on wider core values: 16-bit quantization is not the
limiting factor in this result. Do not attempt a higher-order TT unless at least
one domain shows a credible gain over its classical control or a longer-context
hypothesis states a specific mechanism that the bigram factorization cannot
represent.

