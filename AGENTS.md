# TAK engineering and research guide

This file applies to the complete repository. Read it before changing code,
formats, experiments, benchmarks, or specifications.

## Project priorities

1. Preserve exact lossless round trips.
2. Report compression size, speed, memory, and reliability together.
3. Prefer measured evidence to architectural ambition.
4. Keep decoding deterministic, bounded, portable, and independent of services.
5. Keep replaceable modules simple. Do not add speculative abstractions.

Tensor networks are a research direction, not a required product feature. Do
not describe a tensor result as an improvement until complete artifacts beat
the relevant classical control under the same method.

## Research rules

- State a falsifiable hypothesis, control, dataset split, accounting rule, and
  stop condition before a substantial experiment.
- Include headers, model parameters, dictionaries, and all decoder metadata in
  compressed size.
- Preserve configurations and raw results. Record negative results in
  `research/experiments/`.
- Separate tuning data from held-out data. Do not optimize only for `enwik9`.
- Never fabricate, extrapolate, or silently discard a measurement.
- Do not report one arbitrary combined score. Keep size, time, throughput,
  peak memory, thread count, and round-trip status visible.
- Use ablations for tensor work. Always include the non-tensor control.
- Reject complexity that does not produce a reproducible practical benefit.

Development-corpus results are preliminary. Say so explicitly.

## Format and decoder rules

- Treat every encoded byte as untrusted input.
- Validate versions, flags, identifiers, lengths, arithmetic, allocations, and
  resource limits before use.
- Avoid unspecified floating-point behavior in decoders. Prefer fixed-width
  integer operations with documented overflow bounds.
- Check corruption, truncation, unsupported features, and output limits.
- Do not assign an experimental method a TAK-C identifier before its RFC is
  accepted. Use a distinct research artifact magic value.
- Never silently reinterpret a published field. Document compatibility impact
  before changing a format.
- A format is not stable until an independent decoder can implement its
  decoder-first specification and pass conformance vectors.

Checksums detect accidental corruption. Do not describe them as authentication.

## Rust rules

- The workspace uses stable Rust 1.85 or newer and Apache-2.0 only.
- Unsafe Rust is forbidden unless a later accepted RFC changes that policy.
- Keep dependencies minimal and justify each new dependency.
- Use explicit errors for invalid input. Do not panic on untrusted data.
- Keep the core decoder free of network, cloud, GPU, and platform-service
  requirements.
- Create crates and directories only when the implementation needs them.

Before committing, run:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

Add round-trip, boundary, malformed-input, and resource-limit tests in
proportion to each change. Cross-platform behavior must remain covered by CI.

## Benchmark rules

- Record the commit, input digest, input size, CPU, OS, compiler, tool versions,
  thread count, memory limit, exact command, and repetition count.
- Use complete physical stream or artifact sizes.
- Run a warm-up and at least five measured repetitions for publishable results.
- Verify every decompressed output byte for byte.
- Use Pareto plots for size versus compression speed, decompression speed, and
  peak memory when enough methods exist.
- Label quick checks and single-sample runs as non-publishable screening data.
- In research handoffs, include a concrete distance-to-goal example. Use
  same-dataset measurements when possible. If an enwik9 milestone has not been
  measured, say that it is unmeasured and do not extrapolate from another
  corpus.

Follow `benchmarks/methodology/phase-1.md` for controlled comparisons.

## Documentation rules

- Write specifications and RFCs in English with short, direct sentences.
- Use one term for one concept. Prefer active voice.
- Use BCP 14 terms only for objective normative requirements.
- Separate requirements from explanation and hypotheses from results.
- Every binary format document must define byte order, field sizes, boundaries,
  limits, decoding operations, integrity coverage, rejection conditions, and
  compatibility behavior.
- Project RFCs belong in `docs/rfcs/`. They are not IETF publications.

Update the relevant RFC, experiment record, README, and tests when behavior or
evidence changes. Do not create empty placeholder documents.

