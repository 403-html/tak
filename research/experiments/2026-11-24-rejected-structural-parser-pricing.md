# Rejected structural parser pricing

## Decision

Reject RFC 0021. Pricing the parser with RFC 0019's structural block-type
syntax does not produce a consistent or material complete-artifact benefit on
the Silesia development tails. Do not tune the prices or parser limits on
these files.

The size gate failed. Controlled timing, peak memory, Calgary, bounded byte
identity, and enwik9 were not run.

## Frozen method

The control uses RFC 0019's accepted `TKT9` artifact and RFC 0016 parser. The
candidate changes only encoder parser accounting:

1. It uses the binary literal-or-match type distribution.
2. It charges no match-type bits after a literal.
3. It performs no type update for that implicit match.
4. It charges no end-symbol price.

The match finder, candidate count, segment length, length choices, literal
model, artifact syntax, and decoder are unchanged. The candidate uses the
existing RFC 0019 decoder.

## Screening results

| File | Input bytes | RFC 0019 bytes | RFC 0021 bytes | Saved bytes | Saved bits/input byte | Exact |
|---|---:|---:|---:|---:|---:|:---:|
| dickens | 1,019,245 | 314,932 | 314,428 | 504 | 0.003956 | yes |
| mozilla | 5,122,048 | 1,682,999 | 1,683,312 | -313 | -0.000489 | yes |
| ooffice | 615,220 | 209,682 | 209,583 | 99 | 0.001287 | yes |
| samba | 2,160,640 | 366,444 | 366,312 | 132 | 0.000489 | yes |
| xml | 534,528 | 106,742 | 106,670 | 72 | 0.001078 | yes |
| **Total** | **9,451,681** | **2,680,799** | **2,680,305** | **494** | **0.000418** | **yes** |

Mozilla grows by 313 bytes. This fails the requirement that every artifact be
smaller. Aggregate savings are 0.000418 bits per input byte, which is less
than one twelfth of the required 0.005 bits per input byte.

The times in the raw file are single-sample screening measurements. They are
not controlled or publishable performance results. The pre-registered method
requires one warm-up and five measured repetitions only after the size gate.

Raw rows, including exact input digests and screening times, are in
`benchmarks/reports/raw/2026-11-24-structural-parser-pricing-screen.tsv`,
SHA-256
`9b746856b1b7509220adeb18f78030e7d7f1195ee7581d05b1eff552e4ec5709`.

## Interpretation

The mismatch between parser type prices and physical block-type syntax is
real, but correcting it changes too few parse decisions to be useful. On
Mozilla, the changed decisions make the complete artifact larger. Correcter
local prices do not guarantee a smaller adaptive arithmetic-coded stream.

This result rejects this exact encoder change. It does not change RFC 0019's
accepted format or decoder.

## Tensor scope

This negative result does not authorize a tensor. Structural type state is a
small deterministic state with a direct classical representation. It is not
a longer-context dependency or parameter-sharing problem.

A later tensor RFC still requires a specific measured longer-context
dependency that the rejected bigram factorization cannot represent. Merely
increasing tensor rank remains prohibited.

## Distance to goals

RFC 0019 remains the best measured TAK enwik9 artifact at 249,762,881 bytes.
It remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz still requires a 13.65% reduction.

RFC 0021 has no enwik9 result. Do not extrapolate its Silesia screening result
to enwik9.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `32b465f` plus the RFC 0021 working tree.
- Command: `tak-lz-repriced-gate INPUT --tail-bytes N`.
- Repetitions: one screening run per file.
