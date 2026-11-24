# Rejected match-distance literal continuation

## Decision

Reject RFC 0023. The advancing byte at the previous match distance is a
strong literal predictor on Mozilla and OOffice, but it provides no measured
benefit on Dickens, Samba, or XML. The candidate fails the frozen requirement
that every Silesia development file improve.

Do not add a corpus selector or tune mixtures by file type, phase, distance,
or source agreement on these results. Calgary, deterministic artifact,
enwik9, and tensor work were not run.

## Frozen method

The control is the complete accepted RFC 0019 literal probability. The parse,
normalized order-2 baseline, exact eight-byte predictor, and integer phase
mixture are unchanged.

After a match with distance `D`, the candidate predicts each following
literal at output position `P` from the already decoded byte at `P - D`. One
causal Bayesian mixture combines six source-byte bonus experts with bonuses
`T`, `T/2`, `T/4`, `T/8`, `T/16`, and zero. Every weight starts at one. The
mixture transmits no fitted parameter.

The diagnostic counts ideal literal bits and reports savings per complete
input byte. It creates no artifact and makes no compression claim.

## Results

| File | Input bytes | Eligible literals | Source hit rate | Control bits | Candidate bits | Saved bits | Saved bits/input byte |
|---|---:|---:|---:|---:|---:|---:|---:|
| dickens | 1,019,245 | 635,100 | 7.59% | 1,678,574.684 | 1,678,577.269 | -2.585 | -0.000003 |
| mozilla | 5,122,048 | 2,471,877 | 32.71% | 10,864,616.230 | 10,546,053.118 | 318,563.112 | 0.062194 |
| ooffice | 615,220 | 312,806 | 26.91% | 1,443,135.579 | 1,388,188.307 | 54,947.272 | 0.089313 |
| samba | 2,160,640 | 601,562 | 15.47% | 1,519,973.069 | 1,519,975.654 | -2.585 | -0.000001 |
| xml | 534,528 | 169,949 | 8.22% | 444,511.729 | 444,514.314 | -2.585 | -0.000005 |
| **Total** | **9,451,681** | **4,191,294** | **25.00%** | **15,950,811.291** | **15,577,308.661** | **373,502.630** | **0.039517** |

The aggregate ideal saving is approximately 46,688 bytes before integer
quantization, artifact interactions, or decoder cost. It exceeds the
aggregate threshold. Transfer fails because only two of five files improve.

Each negative file loses exactly `log2(6) = 2.584963` bits. The Bayesian
mixture selects the zero-bonus control expert and pays only its initial model
uncertainty. This is evidence that the direct source dependency is absent or
redundant on those files under the tested model.

The raw timing values are single-run screening measurements. They are not
publishable performance results.

Raw rows, including exact input digests, are in
`benchmarks/reports/raw/2026-11-24-match-distance-literal-screen.tsv`, SHA-256
`fa16be420dc48f447e3162dd1553b4de041f47d2ba56a6d2647281e2f6f522d2`.

## Interpretation

The dependency is real but domain-specific. Binary and office container data
retain useful alignment with the previous match source. Text, source/archive,
and XML data prefer the existing context model. Selecting the mechanism by
file after observing these results would be an oracle and is prohibited.

The result does not justify a tensor. A tensor would add parameter and decoder
cost without evidence that parameter sharing makes the dependency transfer
across domains.

## Distance to goals

RFC 0019 remains the best measured TAK enwik9 artifact at 249,762,881 bytes.
It remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz still requires a 13.65% reduction.

RFC 0023 has no artifact or enwik9 result. Do not extrapolate its aggregate
ideal Silesia saving.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `8f9623b` plus the RFC 0023 working tree.
- Command: `tak-match-distance-literal INPUT --tail-bytes N`.
- Repetitions: one screening run per file.
