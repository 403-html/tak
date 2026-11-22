# TAK Pack

TAK Pack is an experimental, lossless compression project. The research goal
is to test whether compact tensor-network models can improve practical lossless
compression. The project does not yet claim that they can.

The repository currently contains the Phase 1 classical control experiment:

- a deterministic integer arithmetic coder;
- adaptive order-0 and order-1 byte models;
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
The `.takp` archive container, tensor model, FFI, and stable format specification
will be added only after measurements justify them.

## License

Licensed under Apache-2.0.

