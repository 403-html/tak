# Accepted suffix-position reliability residual

## Decision

Accept RFC 0026's classical residual gate. Suffix position changes the
relationship between a suffix byte and exact-match reliability after distance
and correct-run state are known. The dependency transfers to all five Silesia
development tails.

Permit a new RFC for an untied, position-aware tensor representation. That
RFC must compare against both the position-specific classical model and the
sparse exact-suffix control. This result is not a tensor or compression win.

## Frozen method

Every exact eight-byte suffix prediction event has a distance bucket, run
bucket, eight suffix bytes, and a correct or incorrect outcome.

The shared-position control uses adaptive Bernoulli tables keyed by distance,
run, and byte. All suffix positions share those tables. The candidate adds
suffix position to the key. Each model combines its eight position
probabilities with an independent causal eight-expert Bayesian mixture.
Tables start with one correct and one incorrect count. Mixture weights start
at one.

Both models update after each event and transmit no fitted parameters. The
diagnostic measures binary log loss. It creates no artifact.

## Results

| File | Events | Feature b/e | Sparse suffix b/e | Shared-position b/e | Position-specific b/e | Saved vs shared b/e |
|---|---:|---:|---:|---:|---:|---:|
| dickens | 489,162 | 0.973273 | 0.778752 | 0.935011 | 0.794182 | 0.140829 |
| mozilla | 2,661,222 | 0.527881 | 0.435575 | 0.499975 | 0.442018 | 0.057956 |
| ooffice | 288,257 | 0.465839 | 0.505318 | 0.436671 | 0.413970 | 0.022701 |
| samba | 1,595,254 | 0.537002 | 0.500703 | 0.513776 | 0.471394 | 0.042382 |
| xml | 377,036 | 0.655199 | 0.556181 | 0.639294 | 0.551842 | 0.087452 |
| **Weighted** | **5,410,931** | **0.576401** | **0.497920** | **0.549707** | **0.488673** | **0.061034** |

Every file exceeds the required 0.01-bit reduction against the shared model.
The candidate also beats the 24-cell feature control on every file. In
aggregate it improves the sparse exact-suffix control by 0.009246 bits per
event, but the sparse control remains stronger on Dickens and Mozilla.

Peak screened process RSS is 76,220 KiB. Wall times are single screening
measurements without a warm-up and are not publishable performance results.

Raw rows, including exact input digests, are in
`benchmarks/reports/raw/2026-11-24-suffix-position-residual.tsv`.
Its SHA-256 digest is
`918d6bbc0279076ea0ed9fa01f06285f90d2c8f3aad338ec6b3c31f096c6d8a7`.

## Meaning for tensor networks

The result identifies a representation limitation in RFC 0013's shared byte
transition. Untied position-aware cores can address that limitation. The
direct classical candidate contains 49,152 adaptive Bernoulli cells and is a
mandatory control.

The result does not show that a tensor will win. An untied tensor must capture
joint interactions that the marginal tables miss, generalize to held-out
events, and repay serialized core metadata. A negative marginal result would
only have weakened this justification; it would not have disproved possible
higher-order tensor interactions.

Approximate tensor training remains compatible with exact decompression only
when the transmitted integer model deterministically supplies probabilities
to a lossless entropy coder. The coder, not the model approximation, preserves
every source decision.

## Distance to goals

RFC 0019 remains the best measured TAK enwik9 artifact at 249,762,881 bytes.
It remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz still requires a 13.65% reduction.

RFC 0026 has no artifact or enwik9 result. Do not extrapolate log-loss gains
to compressed bytes.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `cc8dceb` plus the RFC 0026 working tree.
- Command: `tak-match-reliability INPUT --tail-bytes N`.
- Repetitions: one screening run per file.
