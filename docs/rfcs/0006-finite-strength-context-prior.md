# RFC 0006: Finite-strength context prior

- Status: Development size gate accepted; speed gate pending
- Date: 2026-11-22
- Scope: Classical research artifact only
- TAK-C method identifier: None

## 1. Problem statement

RFC 0005 showed that an order-1 parent helps cold order-2 contexts on
Canterbury prose and source data. It failed across domains because raw parent
counts overwhelmed local evidence. The effective prior strength grew with the
parent observation total.

This RFC tests a normalized parent distribution with a fixed prior strength.

## 2. Hypothesis

One fixed prior strength will make complete artifacts at least 2% smaller than
order-2 on at least five of the eight RFC 0005 development groups. At least
two passing groups MUST come from Canterbury.

Test strengths 1, 4, 16, 64, 256, and 1024. Select one strength across all
files. These values and both corpora are development data.

## 3. Model

Let `P(s)` be the adaptive order-1 parent frequency, `P_total` its total,
`L(s)` the local two-byte-context count, `L_total` its total, and `a` the fixed
prior strength. The exact integer coding frequency is:

```text
F(s) = L(s) * P_total + a * P(s)
F_total = (L_total + a) * P_total
```

This is proportional to `L(s) + a * P(s) / P_total`. The parent contributes a
fixed mass of `a` local observations regardless of its own observation count.

The first two bytes use only the applicable order-1 parent. After coding, both
sides update the parent and the applicable local row with the exact byte. The
parent and local rescaling rules are the same as RFC 0005.

The decoder MUST use integer arithmetic. The largest screened total is less
than `(16128 + 1024) * 16384`, which fits in 32 bits. The entropy coder's
intermediate product fits in 64 bits.

## 4. Research artifact

The complete artifact uses magic `TKN2`. Its 28-byte header has the same
layout as RFC 0005. Offset 6 contains a little-endian prior strength. It MUST
be 1, 4, 16, 64, 256, or 1024. All other rejection and integrity rules from
RFC 0005 apply.

## 5. Gate and stop condition

Use the exact RFC 0005 files, groups, and complete-artifact accounting. The
gate passes only if one fixed strength:

1. is at least 2% smaller than `TKO2` on five of eight groups;
2. passes that condition on two Canterbury groups;
3. is not larger than `TKO2` in aggregate;
4. decodes no more than two times slower than `TKO2`;
5. uses less than one gigabyte peak decoder resident memory; and
6. restores every input and passes malformed-input tests on all CI platforms.

Reject the mechanism if no strength passes. Do not add more strengths after
measurement. A pass is development evidence and requires later validation on
an untouched corpus.

## 6. Tensor scope

This is a classical smoothing control. It does not authorize a tensor model.
A tensor attempt still requires a new RFC that identifies a longer-context
dependency and representation that the rejected bigram factorization cannot
express.

## 7. Preliminary decision

Strength 4 passed the size conditions on all eight development groups. It was
13.7% smaller than order-2 in aggregate. Group reductions ranged from 2.3% on
Canterbury binary data to 43.8% on Canterbury source/markup.

Do not accept the complete gate yet. Single-run decoding exceeded two times
order-2 on four Silesia groups and Canterbury prose. Preserve strength 4 as
the fixed candidate while investigating the measured symbol-lookup cost. Do
not retune the strength during decoder optimization.
