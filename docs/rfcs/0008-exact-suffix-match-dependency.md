# RFC 0008: Exact-suffix match dependency diagnostic

- Status: Development diagnostic accepted
- Date: 2026-11-23
- Scope: Research diagnostic only
- Format change: None

## 1. Problem statement

RFC 0006 improves cold order-2 contexts, but its benefit falls to 0.87% on
enwik9 after those contexts mature. RFCs 0003 and 0004 show that adding one
independent preceding byte fragments evidence and does not generalize.

A distant repeated sequence contains a different dependency. If an exact byte
suffix occurred before, the byte that followed its most recent occurrence can
predict the next byte. An order-2 model merges this event with every history
that ends in the same two bytes. The rejected RFC 0001 bigram factorization
cannot represent the position or continuation of a longer match at any rank.

## 2. Hypothesis

The most recent continuation after an exact 8-byte suffix contains useful
information after conditioning on the RFC 0006 strength-4 model.

On at least three of five Silesia development groups, adding one quarter of
the baseline frequency total to the predicted byte will reduce theoretical
cross-entropy by at least 0.02 bits per input byte. It MUST NOT increase total
cross-entropy in aggregate.

This diagnostic tests a dependency. It does not estimate final compressed
size and does not authorize a codec or tensor implementation.

## 3. Diagnostic

Process bytes from left to right. Use the normalized order-2 model with fixed
prior strength 4 as the baseline. Before updating the baseline at position
`t`, calculate its exact integer frequency for the actual byte.

For suffix length `L`, maintain a table from the exact `L` bytes before a
position to the byte that followed their most recent occurrence. At position
`t`:

1. Look up `input[t-L..t]` if `t` is at least `L`.
2. Use the stored continuation as the match prediction, if present.
3. Calculate baseline and augmented ideal code lengths for `input[t]`.
4. Replace the stored continuation with `input[t]`.
5. Update the normalized order-2 baseline with `input[t]`.

The table lookup MUST compare exact suffix bytes. Hash collisions MUST NOT
change predictions.

Let `F(s)` and `T` be the baseline frequency and total. Let `p` be the match
prediction and `D` be a positive bonus divisor. Define:

```text
B = floor(T / D)
F'(s) = F(s) + B, if s = p
F'(s) = F(s),     otherwise
T' = T + B
```

The diagnostic code length is `-log2(F'(s) / T')`. Unmatched positions use the
baseline code length.

## 4. Development screen

Measure suffix lengths 4, 8, and 16 and bonus divisors 1, 4, 16, and 64 on the
same final-10% Silesia partitions used by RFCs 0002 through 0006. These values
are development parameters. Preserve every result.

Record for each configuration:

- input bytes;
- matched positions and match coverage;
- correct match predictions and conditional accuracy;
- baseline bits per byte;
- augmented bits per byte;
- total bit reduction and bits-per-byte reduction;
- wall time and peak process RSS; and
- exact command and environment.

The fixed hypothesis in Section 2 determines acceptance. Other configurations
are diagnostics only and MUST NOT replace it after measurement.

## 5. Stop condition

Reject the dependency if the fixed 8-byte, divisor-4 configuration fails any
size condition in Section 2. Do not implement a match artifact, tune more
lengths or bonuses, or propose a tensor representation from a failed result.

If the gate passes, the next RFC MAY propose a deterministic classical match
model. It must specify bounded table memory, eviction, integer probabilities,
decoder operations, complete artifact accounting, and a held-out validation
before any tensor work.

## 6. Tensor scope

This RFC authorizes no tensor model. A later tensor RFC must explain how its
cores represent the exact-suffix or equivalent long-range state, why that
structure avoids RFC 0002's factorization loss, and how it compares with the
classical match control. Increasing rank alone remains prohibited.

## 7. Security and compatibility

The diagnostic reads ordinary input and writes tabular measurements. It emits
no compressed format and changes no TAK-C identifier.

A later decoder MUST bound match distance, table memory, output length, and
integer operations. This diagnostic makes no decoder-safety claim.

## 8. Decision

Accept the development diagnostic. The fixed 8-byte, divisor-4 configuration
reduced theoretical cross-entropy on all five Silesia groups. Reductions ranged
from 0.192 to 0.513 bits per byte. The aggregate reduction was 0.360 bits per
byte across 9,451,681 input bytes.

Match coverage ranged from 47.1% to 74.9%. Conditional continuation accuracy
ranged from 58.1% to 88.5%. These results demonstrate information beyond the
normalized order-2 baseline on the development partitions.

This is not a measured compression-ratio improvement. The diagnostic uses
ideal code lengths and transmits no table state. It does not include integer
frequency rounding, arithmetic-coder termination, artifact fields, or a
bounded-memory eviction policy. The largest process peaked at 212,432 KiB.

The next RFC may define a bounded classical match control and complete
artifact. It must preserve the fixed 8-byte dependency and divisor-4 starting
point for its first measurement. It must compare complete size, speed, memory,
and exact restoration with RFC 0006. Tensor work remains unauthorized.
