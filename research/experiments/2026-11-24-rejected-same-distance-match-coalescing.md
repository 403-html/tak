# Rejected same-distance match coalescing

## Decision

Reject RFC 0024. Coalescing adjacent same-distance match blocks is exact and
slightly reduces four artifacts, but the total benefit is only 31 bytes on the
five Silesia development tails. Do not add distance tolerances, bridge literal
blocks, or change parser limits based on this result.

The size gate failed. Controlled timing, peak memory, Calgary, bounded byte
identity, and enwik9 were not run.

## Frozen method

The control is the accepted RFC 0019 `TKT9` artifact. The candidate retains
the frozen parse and applies one deterministic post-processing pass. It joins
each maximal run of adjacent matches with the same distance, then splits the
logical run into valid matches no longer than 65,535 bytes and no shorter than
eight bytes.

The artifact syntax, literal model, block models, arithmetic coder, checksum,
and decoder are unchanged. The candidate uses the existing RFC 0019 decoder.

## Screening results

| File | Input bytes | RFC 0019 bytes | RFC 0024 bytes | Saved bytes | Saved bits/input byte | Exact |
|---|---:|---:|---:|---:|---:|:---:|
| dickens | 1,019,245 | 314,932 | 314,932 | 0 | 0.000000 | yes |
| mozilla | 5,122,048 | 1,682,999 | 1,682,984 | 15 | 0.000023 | yes |
| ooffice | 615,220 | 209,682 | 209,680 | 2 | 0.000026 | yes |
| samba | 2,160,640 | 366,444 | 366,432 | 12 | 0.000044 | yes |
| xml | 534,528 | 106,742 | 106,740 | 2 | 0.000030 | yes |
| **Total** | **9,451,681** | **2,680,799** | **2,680,768** | **31** | **0.000026** | **yes** |

The frozen gate requires every file to become smaller and at least 0.001
saved bits per input byte. Dickens is unchanged. Aggregate savings are only
2.62% of the required threshold.

The times in the raw file are single-sample screening measurements. They are
not controlled or publishable performance results. The size gate failed
before the required warm-up and five measured repetitions.

Raw rows, including exact input digests, are in
`benchmarks/reports/raw/2026-11-24-same-distance-coalescing-screen.tsv`,
SHA-256
`2a23f211368f2453b3ff98d691422ddcfbe95c6426f42d6017e8bc899d086ed6`.

## Interpretation

Consecutive same-distance blocks exist, but their repeated token cost is too
small to matter. The exact post-processing pass does not justify permanent
encoder complexity or a bounded implementation.

This deterministic result provides no tensor opportunity. It has no learned
dependency or parameter-sharing problem.

## Distance to goals

RFC 0019 remains the best measured TAK enwik9 artifact at 249,762,881 bytes.
It remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz still requires a 13.65% reduction.

RFC 0024 has no enwik9 result. Do not extrapolate its Silesia screening gain.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `63afa38` plus the RFC 0024 working tree.
- Command: `tak-lz-coalesced-gate INPUT --tail-bytes N`.
- Repetitions: one screening run per file.
