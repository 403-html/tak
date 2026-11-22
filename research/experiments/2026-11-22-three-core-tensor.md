# Rejected experiment: three-core second-order tensor train

## Status

RFC 0002 Phase B is rejected. The adaptive order-2 Phase A control remains a
successful classical result.

## Hypothesis

A three-core tensor train might retain the useful dependency
`P(next | previous two bytes)` while sharing parameters across the full
256-by-256 context space.

The cores are:

```text
G0[previous-2, i] * G1[i, previous-1, j] * G2[j, next]
```

The encoder trains them with two sequential nonnegative matrix factorizations.
The decoder uses serialized unsigned 16-bit cores and integer contractions.

## Method

- Base commit: `65ebd72`, plus the Phase B implementation.
- Training: first 90% of each input.
- Test: final 10% after integer truncation.
- Factorization updates: 20.
- Symmetric ranks: 1, 2, 4, and 8.
- Startup: uniform distributions for the first two bytes.
- Size: complete physical `TKR3`, TAK-C order-1, and `TKO2` artifacts.
- CPU: Intel Xeon Platinum 8573C, x86-64.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Samples: one screening run per rank and input.
- Verification: exact byte-for-byte restoration for every artifact.

Input sizes, digests, and Silesia mirror provenance are in the earlier research
reports. These are screening measurements, not publication-quality timings.

## enwik8 results

The held-out input is 10,000,000 bytes. Order-1 is 4,853,625 bytes. Order-2 is
4,069,511 bytes. The `TKR3` header is 28 bytes and has no fallback parameters.

| Rank | Core bytes | Payload bytes | Total bytes | Full precision (bpb) | Quantized (bpb) | Integer (bpb) | Train (s) | Encode (s) | Decode (s) |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 1,536 | 7,000,518 | 7,002,082 | 5.613883 | 5.586547 | 5.600414 | 1.606 | 1.000 | 1.285 |
| 2 | 4,096 | 6,381,434 | 6,385,558 | 5.124302 | 5.097509 | 5.105147 | 1.770 | 0.984 | 1.250 |
| 4 | 12,288 | 5,993,138 | 6,005,454 | 4.806992 | 4.782993 | 4.794510 | 1.662 | 1.054 | 1.377 |
| 8 | 40,960 | 5,695,043 | 5,736,031 | 4.572163 | 4.547996 | 4.556034 | 2.115 | 0.927 | 1.345 |

A rank-8 run with 100 updates produced 5,701,463 bytes. Additional updates do
not close the gap. The 20-update rank-8 experiment process peaked at 180,308
KiB, including the complete source, training matrices, diagnostics, controls,
and decoded outputs.

## Multi-domain results

| Input | Rank | Tensor bytes | Order-1 bytes | Order-2 bytes | Full precision (bpb) | Quantized (bpb) | Integer (bpb) | Train (s) | Encode (s) | Decode (s) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| dickens | 1 | 663,321 | 452,558 | 412,203 | 5.287884 | 5.191086 | 5.194094 | 1.073 | 0.129 | 0.169 |
| dickens | 2 | 569,838 | 452,558 | 412,203 | 4.524110 | 4.439552 | 4.440254 | 1.034 | 0.135 | 0.167 |
| dickens | 4 | 548,139 | 452,558 | 412,203 | 4.282102 | 4.205008 | 4.205641 | 1.214 | 0.118 | 0.188 |
| dickens | 8 | 518,028 | 452,558 | 412,203 | 3.692743 | 3.744477 | 3.744258 | 1.425 | 0.100 | 0.231 |
| samba | 1 | 2,534,773 | 942,729 | 789,852 | 11.953020 | 9.317061 | 9.379476 | 2.494 | 0.296 | 0.378 |
| samba | 2 | 2,532,794 | 942,729 | 789,852 | 10.844051 | 9.275121 | 9.362667 | 2.603 | 0.276 | 0.371 |
| samba | 4 | 1,588,817 | 942,729 | 789,852 | 5.840608 | 5.828419 | 5.837161 | 2.646 | 0.230 | 0.339 |
| samba | 8 | 1,320,039 | 942,729 | 789,852 | 4.737978 | 4.731721 | 4.735819 | 3.596 | 0.216 | 0.365 |
| xml | 1 | 607,502 | 250,174 | 220,755 | 9.369819 | 9.018791 | 9.068750 | 1.045 | 0.086 | 0.129 |
| xml | 2 | 523,208 | 250,174 | 220,755 | 7.825096 | 7.729706 | 7.768853 | 1.084 | 0.094 | 0.185 |
| xml | 4 | 462,116 | 250,174 | 220,755 | 6.701368 | 6.704382 | 6.731913 | 1.236 | 0.071 | 0.131 |
| xml | 8 | 452,867 | 250,174 | 220,755 | 6.132668 | 6.134348 | 6.164368 | 1.591 | 0.081 | 0.180 |
| mozilla | 1 | 4,657,025 | 2,689,171 | 2,393,950 | 7.231802 | 7.232137 | 7.271248 | 4.418 | 0.609 | 0.843 |
| mozilla | 2 | 4,370,873 | 2,689,171 | 2,393,950 | 6.796508 | 6.793476 | 6.820316 | 5.011 | 0.634 | 0.898 |
| mozilla | 4 | 4,548,691 | 2,689,171 | 2,393,950 | 7.047716 | 7.048372 | 7.085251 | 5.256 | 0.578 | 0.806 |
| mozilla | 8 | 4,607,963 | 2,689,171 | 2,393,950 | 7.097444 | 7.089737 | 7.133044 | 7.360 | 0.644 | 1.032 |
| ooffice | 1 | 666,829 | 379,177 | 355,001 | 8.525911 | 8.524849 | 8.650751 | 1.968 | 0.100 | 0.191 |
| ooffice | 2 | 670,957 | 379,177 | 355,001 | 8.543120 | 8.541465 | 8.671135 | 2.001 | 0.104 | 0.193 |
| ooffice | 4 | 684,763 | 379,177 | 355,001 | 8.609231 | 8.610163 | 8.744139 | 2.245 | 0.099 | 0.218 |
| ooffice | 8 | 689,181 | 379,177 | 355,001 | 8.319905 | 8.324810 | 8.428759 | 2.930 | 0.094 | 0.264 |

## Best complete result by input

| Input | Best rank | Excess over order-1 | Excess over order-2 |
|---|---:|---:|---:|
| enwik8 | 8 | 18.2% | 41.0% |
| dickens | 8 | 14.5% | 25.7% |
| samba | 8 | 40.0% | 67.1% |
| xml | 8 | 81.0% | 105.1% |
| mozilla | 2 | 62.5% | 82.6% |
| ooffice | 1 | 75.9% | 87.8% |

## Decision

Reject this Phase B implementation. No rank beats either classical control.
The factorization or its sequential nonnegative training objective is the main
loss. Quantization and integer rounding are secondary. More iterations do not
recover the gap, and higher rank is not consistently better on binary data.

Do not increase rank. Preserve `TKR3` and its decoder for reproducibility only.
A future tensor attempt needs a new optimizer or structure, a new RFC or RFC
amendment, and a stop condition that distinguishes representational capacity
from training failure.

