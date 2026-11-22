# Rejected experiment: two-core factorization on Silesia

## Status

Negative multi-domain result. This experiment caused RFC 0001 to be rejected.

## Question

Does any tested two-core byte-bigram rank beat the adaptive order-1 control on
data outside the `enwik8` development input?

## Dataset

The official Silesia host returned HTTP 503 on 2026-11-22. The experiment used
the public `MiloszKrajewski/SilesiaCorpus` GitHub mirror without adding corpus
files to this repository:

`https://github.com/MiloszKrajewski/SilesiaCorpus`

| File | Role | Bytes | SHA-256 |
|---|---|---:|---|
| `dickens` | English text | 10,192,446 | `b24c37886142e11d0ee687db6ab06f936207aa7f2ea1fd1d9a36763c7a507e6a` |
| `samba` | Source code | 21,606,400 | `93ba07bc44d8267789c1d911992f40b089ffa2140b4a160fac11ccae9a40e7b2` |
| `xml` | Structured document | 5,345,280 | `0e82e54e695c1938e4193448022543845b33020c8be6bf3bf3ead2224903e08c` |
| `mozilla` | Executable binary | 51,220,480 | `657fc3764b0c75ac9de9623125705831ebbfbe08fed248df73bc2dc66e2a963b` |
| `ooffice` | Office binary | 6,152,192 | `e7ee013880d34dd5208283d0d3d91b07f442e067454276095ded14f322a656eb` |

For each file, the first 90% trained the factorization and the final 10% was
held out for compression. The split uses integer truncation. Corpus files were
not mixed across training and test partitions.

## Environment and method

- Base commit: `3c67f25` working tree, before this report.
- CPU: Intel Xeon Platinum 8573C, x86-64.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Factorization updates: 20.
- Ranks: 1, 2, 4, and 8.
- Samples: one screening run per rank and file.
- Tensor sizes: complete physical `TKRX` artifacts.
- Control sizes: complete physical adaptive order-1 TAK-C frames.
- Verification: every artifact decoded byte for byte.

This is screening evidence, not a publication-quality timing comparison.

## Complete results

| File | Rank | Artifact bytes | Control bytes | Full precision (bpb) | Quantized (bpb) | Integer (bpb) | Encode (s) | Decode (s) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| dickens | 1 | 583,858 | 452,558 | 4.694174 | 4.574921 | 4.574408 | 0.091 | 0.115 |
| dickens | 2 | 565,256 | 452,558 | 4.540341 | 4.421117 | 4.420368 | 0.089 | 0.116 |
| dickens | 4 | 533,126 | 452,558 | 4.271951 | 4.152763 | 4.152101 | 0.087 | 0.112 |
| dickens | 8 | 495,564 | 452,558 | 3.825679 | 3.825650 | 3.825133 | 0.086 | 0.118 |
| samba | 1 | 2,466,371 | 942,729 | 9.072205 | 9.072889 | 9.128102 | 0.287 | 0.288 |
| samba | 2 | 1,353,176 | 942,729 | 5.009756 | 5.009755 | 5.002591 | 0.207 | 0.220 |
| samba | 4 | 1,257,446 | 942,729 | 4.647015 | 4.647150 | 4.640558 | 0.181 | 0.223 |
| samba | 8 | 1,201,422 | 942,729 | 4.424424 | 4.424472 | 4.417957 | 0.176 | 0.211 |
| xml | 1 | 558,617 | 250,174 | 8.316196 | 8.315252 | 8.344770 | 0.061 | 0.072 |
| xml | 2 | 465,410 | 250,174 | 6.911876 | 6.912452 | 6.934472 | 0.054 | 0.066 |
| xml | 4 | 384,440 | 250,174 | 5.686340 | 5.686167 | 5.691988 | 0.049 | 0.059 |
| xml | 8 | 375,944 | 250,174 | 5.495221 | 5.495184 | 5.503528 | 0.049 | 0.061 |
| mozilla | 1 | 4,469,759 | 2,689,171 | 6.950902 | 6.951308 | 6.979562 | 0.524 | 0.592 |
| mozilla | 2 | 3,949,017 | 2,689,171 | 6.157800 | 6.157667 | 6.164629 | 0.476 | 0.553 |
| mozilla | 4 | 4,146,168 | 2,689,171 | 6.453228 | 6.453219 | 6.469354 | 0.481 | 0.601 |
| mozilla | 8 | 3,985,070 | 2,689,171 | 6.199318 | 6.199333 | 6.211343 | 0.494 | 0.558 |
| ooffice | 1 | 641,334 | 379,177 | 8.240879 | 8.241148 | 8.325884 | 0.067 | 0.090 |
| ooffice | 2 | 642,810 | 379,177 | 8.247710 | 8.247191 | 8.331761 | 0.066 | 0.080 |
| ooffice | 4 | 612,940 | 379,177 | 7.861018 | 7.861260 | 7.916716 | 0.067 | 0.079 |
| ooffice | 8 | 607,507 | 379,177 | 7.742580 | 7.742786 | 7.792803 | 0.065 | 0.086 |

## Best result by file

| File | Best rank | Tensor bytes | Control bytes | Tensor excess |
|---|---:|---:|---:|---:|
| dickens | 8 | 495,564 | 452,558 | 9.5% |
| samba | 8 | 1,201,422 | 942,729 | 27.4% |
| xml | 8 | 375,944 | 250,174 | 50.3% |
| mozilla | 2 | 3,949,017 | 2,689,171 | 46.8% |
| ooffice | 8 | 607,507 | 379,177 | 60.2% |

## Decision

Reject RFC 0001. No tested rank beats the adaptive order-1 control. The result
is not close on four of five files. Higher rank is not uniformly better on
binary inputs. Quantization is again too small to explain the deficit.

Preserve the crate and artifact decoder as a reproducible negative experiment.
Do not add this method to TAK-C. A future tensor proposal must use a longer
context or a different structure, explain what new dependency it represents,
and define a new stop condition before implementation.

