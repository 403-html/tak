# TAK Pack

<sub>For coding agents: <a href="AGENTS.md">project instructions</a>.</sub>

TAK is an experimental, lossless compression project written in Rust. It explores classical and **tensor-network-based** compression methods, with a focus on compression ratio, speed, and memory efficiency.

TAK beats gzip -9 on Calgary and enwik9 and beats Zstandard-19 on Calgary.

**Status**: _Experimental_. The best-performing model is not yet integrated into the main CLI. Formats are unstable and should not be used for long-term storage.

## Benchmarks

### Calgary (independent files)

| Compressor | Compressed bytes |
|---|---:|
| xz -9 | 885,860 |
| **TAK (RFC 0032)** | **918,436** |
| Zstandard -19 | 921,073 |
| gzip -9 | 1,059,335 |

RFC 0032 is 32,576 bytes above xz. Its single-run encode screen measured
0.815 MB/s and 95,476 KiB peak RSS. The RFC 0033 decoder reached a median
1.559 MB/s at 44,060 KiB peak RSS over five repetitions, with exact
restoration on all files.
[Codec results](research/experiments/2026-11-25-variable-order.md) ·
[Decoder results](research/experiments/2026-11-25-tk32-boundary-search.md)

### enwik9 (1 GB)

| Compressor | Compressed size |
|---|---:|
| xz -9 | 215.67 MB |
| Zstandard -19 | 235.33 MB |
| **TAK (RFC 0032)** | **246.64 MB** |
| gzip -9 | 322.59 MB |

RFC 0032 encoded at 1.25 MB/s with 238,692 KiB peak RSS. The RFC 0033
decoder restored and compared every byte at 2.56 MB/s with 113,952 KiB
peak RSS. The artifact is 30,967,820 bytes above xz and 49,642,016 above M2.
Encode and decode are separate single-run screens.

[Codec results](research/experiments/2026-11-25-variable-order.md) ·
[Decoder results](research/experiments/2026-11-25-tk32-boundary-search.md)

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

## Research

- [Experiment reports](research/experiments/) cover results, limitations, and reproduction steps.
- [Project RFCs](docs/rfcs/) record proposals and decisions.
- [Benchmark methodology](benchmarks/methodology/phase-1.md) and [raw results](benchmarks/reports/raw/) document how measurements were made.

## License

Licensed under Apache-2.0.

Copyright 2026 Tymoteusz "403-html" Stępień.

