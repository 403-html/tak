# RFC 0048: Select a chain candidate by distance price

- Status: Rejected at complete-artifact gate
- Date: 2026-11-25
- Scope: Classical encoder search
- Format change: None

## 1. Hypothesis

RFC 0046's bounded chain reduced complete TK32 size on both
corpora. RFC 0047 retained the previous length record and saved
very little. A different chain source can encode the same length
at a lower distance price, even when it never breaks a length
record. Select an extra source using this price difference.

## 2. Selection rule

Keep the RFC 0046 chain window, probe count, longest candidate,
and forced-match decisions. For each visited extra source of
length L, use the comparison length l = min(L, 71). Every
reference that covers l can offer it in the dense length search
unless the segment boundary clips that length. The selector
uses l before segment clipping and the solver enforces its
existing segment limits.

Distance price is -log2(count / total) plus the number of
remainder bits. Freeze the same distance distribution as the
segment solver. For each source take the minimum distance price
among the two ordinary candidates and longest chain candidate
that can cover l. Subtract the source's distance price from
that minimum. Choose the source with the largest strictly
positive difference. On ties prefer greater L, then the first
position visited. Submit at most one extra source. Length and
type prices cancel in this comparison because the match starts
and ends at the same positions. A positive score measures one
edge's modeled improvement. It does not guarantee a smaller
complete artifact or a better path.

## 3. Measurement

Compare complete RFC 0034, RFC 0046, and RFC 0048 artifacts on
independent Calgary and Canterbury files. Exclude ptt5 after
checking it against Calgary pic. Run both corpora regardless
of the first result. Verify exact restoration, record complete
artifact sizes, hashes, wall time, throughput, and peak RSS.
The candidate advances only if both corpora save at least 0.05%
against RFC 0046 and 0.25% against RFC 0034 with peak RSS below
1 GiB. A passing result permits a byte-shuffle comparison and
full enwik9 validation. Single-run timing is a screen.

## 4. Result

The candidate saves five bytes over the chain on Calgary and 20
on independent Canterbury. All 84 streams restore exactly. The
incremental size gates require savings of 452 and 295 bytes.
Stop before wrapped selection and enwik9.

[Experiment report](../../research/experiments/2026-11-25-chain-distance-price.md)
