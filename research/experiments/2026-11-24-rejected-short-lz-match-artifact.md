# Rejected short LZ match artifact

## Decision

Reject the RFC 0018 short-match artifact. The complete artifact increases the
Silesia development aggregate. Do not tune its table size, length limits,
selection rule, or token syntax on these corpora.

The encoder and decoder restore every screened input exactly. The rejection is
for compressed size, not correctness.

## Frozen implementation

The candidate applies the RFC 0018 opportunity selection without changing its
parameters. It adds one block type for exact matches of three through seven
bytes. It codes short length with a five-symbol adaptive distribution. Short
and ordinary matches share the existing adaptive distance distribution. The
artifact uses the distinct experimental magic value `TKS3` and includes the
standard 28-byte header and CRC-32.

The implementation also fixes a pre-measurement boundary defect. The first
attempt allowed a three-byte candidate when fewer than three bytes remained in
a literal block. That attempt panicked before producing an artifact result.
The fixed code rejects such candidates. A regression test covers the boundary.

## Development screen

This is one non-publishable screening run per file. It is sufficient to apply
the frozen size stop condition. It is not a controlled timing result.

| File | Input bytes | RFC 0017 bytes | RFC 0018 bytes | Saved bytes | Saved bits/input byte | Decode ratio | Exact |
|---|---:|---:|---:|---:|---:|---:|:---:|
| dickens | 1,019,245 | 315,921 | 338,784 | -22,863 | -0.17945 | 0.96x | Yes |
| mozilla | 5,122,048 | 1,693,006 | 1,686,020 | 6,986 | 0.01091 | 0.86x | Yes |
| ooffice | 615,220 | 210,614 | 208,276 | 2,338 | 0.03040 | 0.74x | Yes |
| samba | 2,160,640 | 369,464 | 375,848 | -6,384 | -0.02364 | 0.89x | Yes |
| xml | 534,528 | 107,541 | 110,666 | -3,125 | -0.04677 | 1.01x | Yes |

Only two of five files improve. The gate requires four. Across 9,451,681 input
bytes, RFC 0017 occupies 2,696,546 bytes and RFC 0018 occupies 2,719,594 bytes.
The candidate grows by 23,048 bytes, equivalent to -0.01951 saved bits per
input byte. The gate requires at least +0.08.

The single-run decode ratios are below 1.25, but size already rejects the
candidate. Peak RSS was not measured after the size failure. Calgary and
enwik9 were not run, as required by the RFC stop condition.

Raw rows are in
`benchmarks/reports/raw/2026-11-24-short-match-artifact-screen.tsv`. Its
SHA-256 is `5c407dbd4b2d06bb3d1273c84d7bda75f0ce0fa8adce838239560f7ca5a52c9f`.

## Explanation and limitation

The opportunity estimate excluded block-type adaptation, short-length coding,
literal-block splitting, and the resulting resets of literal phase. Complete
accounting reverses the estimated gain on three files. The estimate was useful
as an upper bound but did not predict artifact size.

This result rejects this fixed explicit short-match mechanism. It does not show
that all short-match parsers are ineffective. The no-tuning rule prevents a
post-result search on the same development corpora.

## Distance to goals

RFC 0017 remains the best measured TAK enwik9 artifact at 251,698,380 bytes.
It remains 36,024,184 bytes above xz and needs a further 14.31% reduction to
match it. RFC 0018 does not change the README benchmark leader or any enwik9
distance because it failed before Calgary and enwik9 validation.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: branch `research/rfc0018-short-match-opportunity` working tree.
- Command: `tak-lz-short-gate INPUT --tail-bytes N`.
