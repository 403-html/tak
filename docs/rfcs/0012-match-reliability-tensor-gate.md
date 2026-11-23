# RFC 0012: Match-reliability tensor gate

- Status: Classical gates accepted; tensor design permitted
- Date: 2026-11-23
- Scope: Dependency diagnostic before tensor implementation
- Format change: None

## 1. Problem statement

RFC 0009 proves that exact 8-byte match state improves compression. RFC 0010
shows that one global confidence change is insufficient. Match reliability may
depend on how far back the matched continuation occurred and how many preceding
match predictions were correct.

The rejected RFC 0001 bigram factorization cannot represent the exact 8-byte
key, match distance, or run state at any rank.

## 2. Classical dependency gate

For every RFC 0009 table hit, predict whether its continuation is correct.
Compare two adaptive Bernoulli controls with initial counts `(correct=1,
incorrect=1)`:

1. one global reliability distribution; and
2. separate distributions selected by match-distance and correct-run buckets.

Distance buckets are `0..255`, `256..4095`, `4096..65535`, and `65536+` bytes.
Run buckets are 0, 1, 2–3, 4–7, 8–15, and 16+ preceding correct predictions.
Both controls update after each matched event. Report ideal binary log loss;
do not report it as compressed size.

The gate passes only if the feature control improves by at least 0.01 bits per
matched event on three of five Silesia groups and does not regress in aggregate.

## 3. Tensor scope

Passing Phase A does not authorize a tensor implementation. The 24-cell
classical table is too small to require factorization.

A Phase B amendment must first show residual reliability dependence on the
eight suffix bytes after conditioning on distance and run state. It must define
a tensor with explicit modes for those features, integer quantization,
transmitted metadata, contraction bounds, a classical sparse/hashed control,
and a stop condition. Increasing an old tensor rank is prohibited.

## 4. Stop condition

Reject distance/run conditioning if the classical gate fails. If it passes,
retain the 24-cell control and measure suffix-conditioned residuals before
designing tensor cores. No compression format changes in this RFC.

## 5. Accounting

Use the established Silesia final-10% development partitions. Preserve matched
events, correct events, both log losses, reduction per matched event, wall
time, and peak RSS. Use one screening sample.

## 6. Phase A decision

Accept the distance/run dependency gate. The feature table reduced log loss by
0.0468, 0.0259, 0.0555, and 0.0478 bits per matched event on Samba, XML,
Mozilla, and OOffice. Dickens improved by 0.0080 and missed the per-group
threshold. Four of five groups pass, and none regresses.

This does not authorize tensor cores. Proceed to the predeclared suffix
residual control below.

## 7. Suffix residual control

Add one adaptive Bernoulli distribution to each occupied exact-suffix table
entry. Preserve its counts while the same key remains in that slot and reset
them on collision replacement. Compare its ideal log loss with the accepted
distance/run table.

The suffix residual passes only if exact-suffix reliability improves by at
least 0.01 bits per matched event on three Silesia groups and does not regress
in aggregate. Passing permits a tensor-design RFC with the classical suffix
table as its control. Failure rejects suffix-byte tensor modes for this
reliability mechanism.

## 8. Suffix residual decision

Accept the suffix residual gate. Exact-suffix reliability improved over the
distance/run control by 0.1945 bits per matched event on Dickens, 0.0363 on
Samba, 0.0990 on XML, and 0.0923 on Mozilla. OOffice regressed by 0.0395.

Across 5,410,931 matched events, weighted log loss fell from 0.5764 to 0.4979
bits per event, a 0.0785-bit reduction. Four groups pass and aggregate loss
improves. The suffix-conditioned dependency is real but not universal.

This result permits a new tensor-design RFC. The tensor MUST model suffix,
distance, and run state and MUST compare against both classical controls. It
does not authorize reuse of the rejected bigram factorization.
