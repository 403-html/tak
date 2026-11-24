# Rejected experiment: position-aware match-reliability tensor

## Decision

Reject the two RFC 0027 representations under the frozen training method.
The position-modulated and fully untied rank-2 tensors both fail the
full-precision representation gate on Dickens, XML, and OOffice. Three
failures make the required three-of-five condition unreachable. Stop without
running Samba or Mozilla.

Do not implement a compression artifact. This result rejects these exact
representations and training method. It does not show that untied tensor cores
cannot model higher-order interactions.

## Method

- Input: the established final-10% Silesia development tails.
- Split: first 90% of each tail for training and final 10% for evaluation.
- State: independent empty bounded match tables for both partitions.
- Rank: two for every tensor.
- Training: 100 deterministic full-batch exponentiated-gradient epochs.
- Initial learning rate: 1.0, divided by the square root of the epoch number.
- Candidates: a position-modulated MPS and fully untied position cores.
- Controls: shared tensor, distance/run, sparse exact suffix, shared position,
  and position-specific adaptive models.
- Accounting: every unsigned-16-bit core byte amortized over evaluation
  events.
- Samples: one screening run without warm-up. Time and memory are not
  publishable performance results.
- Source: commit `69634c5` plus the RFC 0027 working tree.

## Results

Values are bits per evaluation match event before tensor metadata unless the
column says otherwise.

| File | Events | Sparse suffix | Position-specific | Shared tensor | Modulated tensor | Untied tensor |
|---|---:|---:|---:|---:|---:|---:|
| Dickens | 25,354 | 0.843930 | 0.809721 | 0.944779 | 0.940258 | 0.885115 |
| XML | 31,452 | 0.496021 | 0.436418 | 0.550822 | 0.552386 | 0.528100 |
| OOffice | 39,748 | 0.480767 | 0.273534 | 0.396633 | 0.397635 | 0.388894 |
| **Weighted** | **96,554** | **0.581099** | **0.467389** | **0.590796** | **0.590531** | **0.564542** |

The fully untied tensor improves the frozen shared tensor on all three files.
Its weighted gain is 0.026254 bits per event. Positional specialization
therefore has measurable representation value inside this tensor family.

The stronger classical control remains better on every file. The fully untied
tensor's weighted 0.564542 bits per event is 0.097153 above the
position-specific model. It also fails to beat both required controls on each
file. The compact position-modulated factorization provides almost no
aggregate improvement over the shared tensor.

Quantization and integer contraction add less than 0.000026 bits per event in
every candidate run. Representation quality, not numerical approximation, is
the pre-metadata failure.

Metadata makes the complete diagnostic much worse. The modulated model uses
2,184 bytes. Its weighted integer loss with metadata is 1.133408 bits per
event. The fully untied model uses 16,456 bytes and reaches 4.654953 bits per
event. Peak screened process RSS is 50,376 KiB.

Raw rows are in
`benchmarks/reports/raw/2026-11-24-position-aware-tensor-screen.tsv`. The
SHA-256 digest is
`9f1007be2ff67772e8d9ed886cecdb900ffff1aba38ab14f5fad8d1207a3db9c`.

## Interpretation

RFC 0026 correctly identified a transferable suffix-position dependency.
Untying the tensor cores captures some of it: the untied tensor consistently
beats the shared tensor. It does not capture the dependency as efficiently as
the causal classical control under this experiment.

The failure of the classical residual would not have ruled out untied cores.
Its success also did not guarantee a tensor win. This direct tensor result now
weakens the case for these rank-2 MPS parameterizations more than the
classical diagnostic alone could.

Do not retry higher rank, more epochs, or a different learning rate as the
next tensor attempt. A later tensor RFC needs a different topology or a newly
measured joint dependency, a compact metadata plan, and pre-registered gates.
Classical and hybrid codec research can continue independently.

## Exactness

This was a probability-model diagnostic, not a compressed artifact. It did
not test round-trip restoration. The quantized model uses bounded integer
contraction, but no compression claim follows from log loss.

A future tensor artifact can remain mathematically lossless if its integer
frequencies drive a lossless entropy coder. The entropy coder must encode
every decision exactly; tensor approximation must affect only probability
estimates.

## Distance to goals

RFC 0019 remains the best measured TAK enwik9 artifact at 249,762,881 bytes.
It remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz still requires a 13.65% reduction.

RFC 0027 has no artifact or enwik9 result. Do not extrapolate these
development log-loss results to compressed size.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.

