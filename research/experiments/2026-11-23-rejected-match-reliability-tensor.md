# Rejected experiment: shared-transition match-reliability tensor

## Decision

Reject RFC 0013. The tensor failed the full-precision gate on Dickens, XML,
and OOffice at every tested rank. The three-of-five condition is unreachable,
so Samba and Mozilla were not run. No compression artifact was implemented.

This result rejects the specified shared-transition model and training method.
It does not show that all tensor-network context models are ineffective.

## Method

- Input: the established final-10% Silesia development partitions.
- Input digests: `benchmarks/reports/raw/2026-11-23-match-dependency-inputs.tsv`.
- Split: first 90% of each partition for training and final 10% for evaluation.
- State: independent empty bounded match tables for training and evaluation.
- Ranks: 1, 2, 4, and 8. Rank 1 is a negative control.
- Training: 100 deterministic full-batch exponentiated-gradient epochs.
- Initial learning rate: 1.0, divided by the square root of the epoch number.
- Initialization: deterministic distance spread and near-identity recurrent
  transitions.
- Accounting: every unsigned-16-bit core byte is amortized over evaluation
  match events.
- Samples: one screening run without a warm-up. Timings are not publishable.
- Source: commit `32c1e63` plus the RFC 0013 working tree.

Dickens was used to diagnose a state-washout problem in the initial optimizer.
The near-identity initialization and 100-epoch configuration were fixed before
the XML and OOffice screens. All results below use that final configuration.

## Results

| Group | Best dependency rank | Feature control | Suffix control | Tensor float | Integer + metadata | Gate |
|---|---:|---:|---:|---:|---:|---|
| Dickens | 2 | 0.9680 | 0.8439 | 0.9448 | 1.6137 | Fail suffix |
| XML | 2 | 0.4937 | 0.4960 | 0.5508 | 1.0901 | Fail both |
| OOffice | 2 | 0.2857 | 0.4808 | 0.3966 | 0.8233 | Fail feature |

Values are bits per evaluation match event. Ranks 4 and 8 did not improve the
full-precision result. Quantization and deterministic integer contraction
changed loss by less than 0.00002 bits per event in these rank-2 runs. The
dominant complete-model failure was core metadata: 0.4267 to 0.6689 bits per
event for rank 2.

Peak process RSS was 49,976 KiB. Rank-8 wall time reached 78.62 seconds on
Dickens. The raw table contains every rank, operation time, wall time, and RSS:
`benchmarks/reports/raw/2026-11-23-match-reliability-tensor-screen.tsv`,
SHA-256
`74a5cea5e735c586046ff3835b4106b8580196d7ac4cd881de161e1d02c25100`.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: none imposed.
- Date: 2026-11-23 Europe/Warsaw.

## Distance to project goals

This rejected diagnostic does not change the measured enwik9 artifact.
TAK remains at 315,578,766 bytes. M1 remains achieved by 6,421,234 bytes. M2
is 118,578,766 bytes away and requires a 37.57% reduction. The gaps to the
measured Zstandard-19 and xz results remain 80,249,827 and 99,904,570 bytes.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-match-reliability-tensor
python3 benchmarks/runners/measure.py \
  --timing /tmp/tensor-dickens-r2-memory.tsv \
  --stdout /tmp/tensor-dickens-r2.tsv \
  target/release/tak-match-reliability-tensor \
  /tmp/silesia-screen/dickens --tail-bytes 1019245 \
  --rank 2 --epochs 100 --learning-rate 1.0
```
