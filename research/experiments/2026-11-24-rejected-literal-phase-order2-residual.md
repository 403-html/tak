# Rejected literal-phase order-2 residual

## Decision

Reject RFC 0022. A phase-specific order-2 expert finds a large opportunity on
Mozilla but no opportunity on the other four Silesia development tails. The
result fails both frozen development conditions. Do not tune the mixture,
phase boundaries, or prior strength on these files.

Calgary, deterministic artifact, enwik9, and tensor work were not run.

## Frozen method

The control is the complete literal probability from the accepted RFC 0019
path. It includes RFC 0017's normalized order-2 baseline, exact eight-byte
match prediction, and four integer phase mixtures.

The candidate adds four normalized order-2 models. Each model learns only
literals in one decoder-known phase but observes every byte to keep its
two-byte history synchronized. One causal two-expert Bayesian mixture per
phase combines the control probability with the matching phase model. Both
weights start at one. The diagnostic transmits no fitted parameters.

The parse is unchanged from RFC 0019. The diagnostic counts literal ideal
bits only and reports savings per complete input byte. It creates no artifact
and makes no compressed-size claim.

## Results

| File | Input bytes | Literal bytes | Control bits | Candidate bits | Saved bits | Saved bits/input byte |
|---|---:|---:|---:|---:|---:|---:|
| dickens | 1,019,245 | 635,240 | 1,678,574.684 | 1,678,578.684 | -4.000 | -0.000004 |
| mozilla | 5,122,048 | 2,472,077 | 10,864,616.230 | 10,788,881.839 | 75,734.392 | 0.014786 |
| ooffice | 615,220 | 313,122 | 1,443,135.579 | 1,443,139.579 | -4.000 | -0.000007 |
| samba | 2,160,640 | 601,611 | 1,519,973.069 | 1,519,977.069 | -4.000 | -0.000002 |
| xml | 534,528 | 169,991 | 444,511.729 | 444,515.729 | -4.000 | -0.000007 |
| **Total** | **9,451,681** | **4,192,041** | **15,950,811.291** | **15,875,092.899** | **75,718.392** | **0.008011** |

Only Mozilla improves. Its phase-three expert saves 75,737.392 bits after the
other three phases each pay a one-bit initialization cost. On every other
file, each phase mixture selects the control and loses exactly its one-bit
initialization cost.

The result misses the 0.01 bits-per-input-byte threshold and the requirement
that all five files improve. A per-file or phase-three-only selector chosen
after seeing these results is prohibited.

The raw timing values are single-run screening measurements. They are not
publishable performance results.

Raw rows, including exact input digests, are in
`benchmarks/reports/raw/2026-11-24-literal-phase-order2-screen.tsv`, SHA-256
`c48de959c832e8b2146f69bd97731f28a52edfba7917ad9fba08adc471850717`.

## Interpretation

The named four-mode dependency exists strongly in Mozilla phase three, but it
does not transfer across text, office, source/archive, and XML data under this
method. The Bayesian mixture's bounded four-bit loss shows that the negative
files are not hiding a useful phase model behind catastrophic overfitting;
they consistently prefer the accepted control.

This mixed result does not justify a format change. It also does not justify
a tensor. Tensor parameter sharing cannot turn one corpus-specific win into
the required cross-domain evidence without a new dependency and gate.

## Distance to goals

RFC 0019 remains the best measured TAK enwik9 artifact at 249,762,881 bytes.
It remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz still requires a 13.65% reduction.

RFC 0022 has no complete artifact or enwik9 result. Do not extrapolate its
ideal Mozilla result.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `f63bb34` plus the RFC 0022 working tree.
- Command: `tak-literal-phase-order2 INPUT --tail-bytes N`.
- Repetitions: one screening run per file.
