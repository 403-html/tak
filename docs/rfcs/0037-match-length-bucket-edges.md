# RFC 0037: Test match lengths at bucket boundaries

- Status: Rejected at Calgary complete-artifact gate
- Date: 2026-11-25
- Scope: Classical encoder search
- Format change: None

## 1. Hypothesis

RFC 0034's bounded search tries every length from 8 through 71 and the
longest available length. An intermediate length can cost fewer bucket
bits or end at a position with a better next match. Adding the lengths
on either side of each skipped bucket boundary will reduce complete
artifact size.

## 2. Method

Keep the RFC 0034 parser and TK32 encoder as the control. For each match
candidate in the segment solver, keep the existing lengths. If the
segment-clipped maximum exceeds 71, also try lengths of the form
`(8 - 1) + 2^k - 1` and `(8 - 1) + 2^k`, starting at k = 7.
Try added lengths in ascending order, strictly below the clipped
maximum, before the existing maximum. A minimum of 1,024 bytes still
triggers the unchanged forced-match rule. Keep match candidates, table
updates, price models, segment size, and all decoder operations fixed.
The extra work per candidate is logarithmic in the clipped maximum.

## 3. Gates

Run all 18 Calgary inputs independently. Compare full TK32 files and
require byte-for-byte restoration for every output. Advance if the
candidate saves at least 0.15% in aggregate and improves a majority of
the files whose sizes change.

Freeze the implementation before running independent Canterbury files.
Exclude ptt5 because it duplicates Calgary pic. Require at least 0.10%
aggregate savings and exact restoration for all outputs. If these size
gates pass, measure again with a warm-up and five repetitions before
full enwik9 validation. Record size, wall time, throughput, and peak RSS
for both operations. Keep encoder and decoder peak RSS below 1 GiB.
The decoder must retain its existing bounds and TK32 compatibility.

Keep failed screening rows. Stop at the first failed gate. This is an
encoder search experiment and introduces no new model or stream fields.

## 4. Result

The added lengths reduce the Calgary total from 906,449 to 906,445
bytes. Pic saves two bytes. Progp and trans each save one. The other
15 files have the same size as the control. Of these, 11 are
byte-identical. All 36 streams decode
through the current TK32 decoder and restore the exact input.

The four-byte reduction falls below the 0.15% gate, which requires
1,360 bytes at this control size. Stop before Canterbury, repeated
timings, and enwik9.

[Experiment report](../../research/experiments/2026-11-25-match-length-bucket-edges.md)
