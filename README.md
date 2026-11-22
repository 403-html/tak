# TAK Pack

<sub>For coding agents: <a href="AGENTS.md">project instructions</a>.</sub>

TAK Pack is an experimental, lossless compression project. The research goal
is to test whether compact tensor-network models can improve practical lossless
compression. The project does not yet claim that they can.

The repository currently contains the Phase 1 classical control experiment:

- a deterministic integer arithmetic coder;
- adaptive order-0 and order-1 byte models;
- a research-only adaptive order-2 control;
- a small, versioned experimental `.takc` stream;
- exact round-trip and malformed-input tests; and
- a benchmark command that reports size and wall-clock throughput.

The format is experimental and has no compatibility guarantee yet. Do not use
it for durable storage.

## Build

```console
cargo build --release
cargo test --workspace
```

## Use

```console
cargo run --release -p tak-cli -- compress input.txt -o input.takc
cargo run --release -p tak-cli -- decompress input.takc -o restored.txt
cargo run --release -p tak-cli -- verify input.takc
cargo run --release -p tak-cli -- benchmark input.txt
```

`compress` tests each implemented method and stores the smallest valid frame.
Use `--method raw`, `--method order-0`, or `--method order-1` to run a controlled
experiment.

## Scope

This is a research foundation, not a stable codec or archive application.
The `.takp` archive container, production tensor model, FFI, and stable format
specification will be added only after measurements justify them.

## Tensor screening experiment

The `tak-tensor` crate preserves rejected RFC 0001 as a reproducible research
artifact. It is not a TAK-C codec. Run one quantized bigram-factorization rank
with:

```console
cargo run --release -p tak-tensor --bin tak-tensor-experiment -- \
  input.bin --rank 4 --iterations 20 --output result.tkrx
```

By default, the first 90% of the input trains the model and the last 10% is the
held-out compression input. Use `--train-bytes` to record an exact split. The
reported artifact size includes the header, quantized cores, and payload.
The output also separates full-precision factorization loss, core quantization
loss, and integer-frequency rounding loss in bits per byte.

RFC 0002 passed its classical gate: adaptive two-byte context improves complete
size on the current development corpus. Its three-core tensor train was then
implemented and rejected because it lost to both classical controls. Neither
research method has a TAK-C identifier.

Reproduce the RFC 0002 tensor artifact with:

```console
cargo run --release -p tak-tensor --bin tak-tensor-order2-experiment -- \
  input.bin --rank 8 --iterations 20 --output result.tkr3
```

## License

Licensed under Apache-2.0.

Copyright 2026 Tymoteusz "403-html" Stępień.

