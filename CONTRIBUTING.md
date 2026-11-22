# Contributing to TAK Pack

TAK is in the research phase. Open an issue before implementing a new codec,
binary-format feature, or tensor model. State the hypothesis, control, dataset,
metadata accounting, and stop condition.

All changes must preserve exact round-trip behavior. Run:

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Compression changes must include raw measurements for size, compression time,
decompression time, and peak memory. Keep unsuccessful experiment records.
Do not submit estimated benchmark results as measurements.

By contributing, you agree that your contribution is licensed under the
Apache License 2.0.

