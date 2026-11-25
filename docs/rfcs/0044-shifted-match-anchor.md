# RFC 0044: Search for matches with a shifted anchor

- Status: Rejected at Calgary complete-artifact gate
- Date: 2026-11-25
- Scope: Classical encoder search
- Format change: None

## 1. Hypothesis

RFC 0043 found that a sixteen-byte key recovered useful older
sources at the cost of a second table. Bytes 4 through 11 also
distinguish some continuations that share bytes 0 through 7.
Looking up this shifted key in the existing table may recover
longer matches without allocating another index.

## 2. Method

Keep the RFC 0034 parser and the ordinary candidates. At each
position with at least 12 remaining bytes, look up the exact
eight-byte key that starts four bytes ahead. Inspect the latest
entry for that key. Subtract four from its stored position with
checked arithmetic. Reject the source if the subtraction fails,
if the source is outside the match window, or if its initial
eight-byte key differs from the current initial key.

Measure the surviving match with the existing comparison and limits.
Add it as the third candidate only if it is strictly longer than
both normal candidates. Use the existing tie order and forced
match rule. Keep the existing index insertion order, candidate
lengths, parser prices, TK32 syntax, and decoder. This profile
uses no separate sixteen-byte index.

## 3. Gates

Compare RFC 0034 and the shifted profile on complete independent
Calgary files. Require exact restoration, at least 0.25% aggregate
savings, and encoder and decoder peak RSS below 1 GiB. Record
physical sizes, hashes, wall time, throughput, and peak RSS.

If Calgary passes, freeze the profile and require exact restoration
and at least 0.25% aggregate savings on independent Canterbury,
excluding ptt5. Then measure the interaction with the RFC 0038
transforms and validate the full enwik9 artifact. Stop at the
first failed gate. Measurements with one repetition are screens.

## 4. Result

Calgary shrinks from 906,449 to 905,033 bytes. Sixteen files
shrink and two have the same size. All 36 streams restore exactly.
The saving of 1,416 bytes (0.156%) falls below the required
2,267 bytes. Stop before Canterbury, wrapped selection, and enwik9.

[Experiment report](../../research/experiments/2026-11-25-shifted-match-anchor.md)
