# Accepted diagnostic: exact-suffix continuation

## Status

RFC 0008 passes its development diagnostic. This is evidence for a specific
long-range dependency. It is not a codec result, a compressed-size claim, or
authorization for a tensor implementation.

## Hypothesis and control

The fixed hypothesis adds one quarter of the normalized order-2 frequency
total to the byte that followed the most recent exact 8-byte suffix. It passes
only if ideal cross-entropy falls by at least 0.02 bits per byte on three of
five Silesia groups and does not regress in aggregate.

The control is the RFC 0006 normalized order-2 model at fixed strength 4. The
diagnostic also preserves results for suffix lengths 4 and 16 and bonus
divisors 1, 16, and 64. These alternatives do not replace the fixed gate.

## Dataset and environment

- Inputs: final 10% of Dickens, Samba, XML, Mozilla, and OOffice from Silesia.
- Total selected bytes: 9,451,681.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Date: 2026-11-23 Europe/Warsaw.
- Samples: one measured run per file without a warm-up.
- Source: commit `750ff8a` plus the RFC 0008 working tree.

All configurations are in
`benchmarks/reports/raw/2026-11-23-match-dependency.tsv`. Process measurements
are in `benchmarks/reports/raw/2026-11-23-match-dependency-memory.tsv`. Input
digests are in `benchmarks/reports/raw/2026-11-23-match-dependency-inputs.tsv`.

Raw file SHA-256 digests:

- results: `907f852714ecfda6b448f8782b8960cab52231687485336f60cd23938e3eeb43`;
- inputs: `b2f3379b9b63331b1f410d6e456f9526af90f00546814fc50caafc3148202841`;
  and
- memory: `c7094f6fa74a56682178a4e76ec19f62469d46b917b8097b21eed17fb1462fc3`.

## Fixed-gate result

| Group | Match coverage | Match accuracy | Baseline bpb | Augmented bpb | Reduction bpb | Peak RSS |
|---|---:|---:|---:|---:|---:|---:|
| Dickens | 51.42% | 58.13% | 2.799 | 2.607 | 0.192 | 85,976 KiB |
| Samba | 74.89% | 85.76% | 2.467 | 1.954 | 0.513 | 99,676 KiB |
| XML | 71.12% | 81.89% | 2.484 | 2.047 | 0.437 | 26,180 KiB |
| Mozilla | 54.29% | 85.68% | 3.390 | 3.053 | 0.337 | 212,432 KiB |
| OOffice | 47.06% | 88.51% | 3.314 | 3.093 | 0.221 | 30,304 KiB |
| Aggregate | — | — | 3.059 | 2.700 | 0.360 | — |

The gate passes on all five groups. The aggregate ideal reduction is
3,399,632 bits, or approximately 424,954 bytes before any implementation and
format costs. Do not subtract that number from a measured artifact: the
diagnostic is not an entropy-coded stream.

The measured process wall times ranged from 0.376 to 5.350 seconds. The raw
table reports per-suffix scan time. The process reads each complete source file
before selecting its final partition, so peak RSS includes the full source
buffer and three sequential diagnostic tables. It is not a decoder-memory
measurement.

## Interpretation

The previous exact suffix provides information that remains after normalized
order-2 conditioning. The dependency is broad across text, source, structured
documents, executable data, and office data. An order-2 model cannot identify
which distant occurrence supplied the continuation. The rejected bigram
factorization cannot add that state by increasing rank.

The result does not show that a tensor network is a suitable representation.
A direct bounded match table is the required classical control. It will also
expose the actual costs of hashing, eviction, probability mixing, and exact
decoding.

## Distance to project goals

RFC 0008 does not change the measured enwik9 artifact. TAK remains at
368,643,002 bytes, 46,643,002 bytes above M1. It must remove 12.65% of its
current bytes to reach M1 and 46.56% to reach M2.

The 0.360-bit development reduction is not extrapolated to enwik9. Even on the
development inputs, it is an ideal probability-model result rather than a
complete-size result. The next experiment must measure a complete artifact
before changing any milestone distance.

## Decision

Accept the dependency diagnostic. Write a new RFC before implementing the
bounded classical match control. Keep the first control fixed at an 8-byte
suffix and divisor 4. Define deterministic capacity and eviction before
measurement. Compare against the byte-identical streaming RFC 0006 path.

Do not implement a tensor model yet. A later tensor RFC must name a structure
that represents long-range match state and must beat the classical match
control under identical accounting.

## Reproduction

```console
python3 benchmarks/runners/measure.py \
  --timing /tmp/match-dickens-memory.tsv \
  --stdout /tmp/match-dickens.tsv \
  target/release/tak-match-dependency \
  /tmp/silesia-screen/dickens --tail-bytes 1019245

python3 benchmarks/runners/measure.py \
  --timing /tmp/match-samba-memory.tsv \
  --stdout /tmp/match-samba.tsv \
  target/release/tak-match-dependency \
  /tmp/silesia-screen/samba --tail-bytes 2160640

python3 benchmarks/runners/measure.py \
  --timing /tmp/match-xml-memory.tsv \
  --stdout /tmp/match-xml.tsv \
  target/release/tak-match-dependency \
  /tmp/silesia-screen/xml --tail-bytes 534528

python3 benchmarks/runners/measure.py \
  --timing /tmp/match-mozilla-memory.tsv \
  --stdout /tmp/match-mozilla.tsv \
  target/release/tak-match-dependency \
  /tmp/silesia-screen/mozilla --tail-bytes 5122048

python3 benchmarks/runners/measure.py \
  --timing /tmp/match-ooffice-memory.tsv \
  --stdout /tmp/match-ooffice.tsv \
  target/release/tak-match-dependency \
  /tmp/silesia-screen/ooffice --tail-bytes 615220
```
