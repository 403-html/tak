# RFC 0046: Follow a bounded chain of exact match keys

- Status: Passed Calgary, rejected at Canterbury transfer gate
- Date: 2026-11-25
- Scope: Classical encoder search
- Format change: None

## 1. Hypothesis

RFC 0045 found that distinct searches can recover useful older
sources. It still indexes a fixed number of positions for any
given key. A bounded chain of older exact occurrences may recover
longer matches whose first eight bytes are common.

## 2. Method

Use the RFC 0034 parser as the control. In the candidate, maintain
a ring of 1,048,576 links keyed by source position modulo the ring
length. Each link is the previous position found for the same
exact eight-byte key in the existing table. A missing link uses
a sentinel. Store links in source order, including for bytes
covered by forced matches. The ring adds 4 MiB of encoder state.

Keep the two ordinary candidates. Start from the table's latest
position and skip the first two positions in the chain. Inspect at
most 14 further positions and stop when a link leaves the last
1,048,576 bytes. Keep the first strictly longest extra candidate.
Stop searching once a candidate reaches 1,024 bytes. If an ordinary
candidate already reaches that threshold, use the ordinary forced
selection without following the chain. All match lengths and
distances use the existing limits and exact byte comparisons.
Preserve the parser prices, tie order, TK32 syntax, and decoder.

## 3. Gates

Run the control and candidate on all 18 independent Calgary
files. Require exact restoration, at least 0.25% aggregate
savings over RFC 0034, and peak RSS below 1 GiB. Record complete
artifact size, digest, wall time, throughput, and peak RSS.
These one-run trials are screening measurements.

If Calgary passes, repeat unchanged on independent Canterbury,
excluding ptt5, with the same requirements. Compare with the
recorded RFC 0045 aggregates for context. Only a passing held-out
candidate advances to the RFC 0038 transform combination and full
enwik9 validation. Stop at the first failed gate.

## 4. Result

Calgary saves 3,394 bytes (0.374%) and passes. Independent Canterbury
saves 991 bytes (0.168%) and falls below the 0.25% threshold. All
56 streams restore exactly. Peak RSS is below 1 GiB for every
operation. Stop before wrapped selection and enwik9.

[Experiment report](../../research/experiments/2026-11-25-bounded-match-chain.md)
