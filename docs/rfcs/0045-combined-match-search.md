# RFC 0045: Combine both longer-match searches

- Status: Passed Calgary, rejected at Canterbury transfer gate
- Date: 2026-11-25
- Scope: Classical encoder search
- Format change: None

## 1. Hypothesis

RFC 0043 and RFC 0044 both recovered older match sources. The
sixteen-byte index and shifted lookup may recover different sources.
Test whether their combination saves more complete bytes than
either alone and whether that benefit transfers to held-out data.

## 2. Method

Compare the unwrapped RFC 0034 control, RFC 0043 alone, RFC 0044
alone, and both searches together. In the combined profile,
each search independently proposes a match only if it beats both
ordinary candidates in length. Evaluate the ordinary candidates
first, the sixteen-byte source next, and the shifted source last.
Omit a shifted source with the same distance as the sixteen-byte
source. The longest match triggers the existing forced rule, with
ties resolved by the same order. Preserve every other parser and
encoder rule. All profiles emit TK32 and share one decoder.

## 3. Measurement and gates

Run all four profiles on all 18 independent Calgary files, then
all 10 independent Canterbury files. Exclude the duplicate ptt5.
Run Canterbury even if Calgary misses the promotion gate so the
comparison measures transfer. Measure complete artifact size, wall
time, throughput, and peak RSS. Verify every output byte for byte.
Require the control and standalone variants to reproduce their
existing Calgary artifact hashes.

Advance the combination only if it saves at least 0.25% against
the control and at least 0.05% against the smaller standalone
aggregate on both corpora, with peak RSS below 1 GiB for each
operation. If it passes, test with RFC 0038's transforms and
measure the full enwik9 artifact. Keep all raw rows. Initial
single-run timings are screens.

## 4. Result

The combination passes both Calgary size gates. It saves 2,385
bytes (0.263%) against the control and 634 bytes (0.070%) against
the better standalone search. On Canterbury it saves 804 bytes
(0.136%) and 187 bytes (0.032%) against the same two references.
Those gains are below both Canterbury thresholds. All 112 streams
restore exactly. Stop before wrapped selection and full enwik9.

[Experiment report](../../research/experiments/2026-11-25-combined-match-search.md)
