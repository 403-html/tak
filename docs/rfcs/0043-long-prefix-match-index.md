# RFC 0043: Rescue longer matches with a sixteen-byte index

- Status: Rejected at Calgary complete-artifact gate
- Date: 2026-11-25
- Scope: Classical encoder search
- Format change: None

## 1. Hypothesis

The RFC 0034 search remembers two positions for an eight-byte prefix.
The latest two occurrences can have different continuations from an
older occurrence. A separate sixteen-byte index can recover some of
the longer repeats. RFC 0028 tested collisions between distinct
eight-byte keys. This experiment keeps that table unchanged and
searches for a different exact prefix.

## 2. Method

Maintain the latest position for each retained sixteen-byte key in a
direct-mapped table of 262,144 slots. Read the first and second eight
bytes as big-endian integers, XOR the first with the second rotated
left by 23, multiply modulo 2^64 by 0x9e3779b97f4a7c15, and use the
top 18 bits as the index. Compare both integers before using an entry.
Replace the selected slot after lookup. Insert all input positions with
sixteen available bytes, including bytes covered by forced matches.

Keep both RFC 0034 candidates. Add a third only if the sixteen-byte
source yields a strictly longer match than both. In the segment solver
evaluate that source after the two normal candidates using the same
lengths and tie rules. For the existing forced-match threshold choose
the longest candidate. Use the same distance window and match length
limits as RFC 0034, including for the new candidate. Literal probability
tables remain unchanged. Only the encoder allocates the extra table.

## 3. Gates

Run the candidate and RFC 0034 control on all 18 Calgary files
independently as complete TK32 streams. Require exact restoration on
every trial, at least 0.25% aggregate savings, and peak memory below
1 GiB. Record artifact lengths and hashes, wall time, throughput, and
peak RSS. The initial screen uses one run per file and profile.

If Calgary passes, run the fixed profile on independent Canterbury,
excluding ptt5. Require exact restoration and at least 0.25% aggregate
savings. Then measure the interaction with the RFC 0038 selected
transforms and run a full enwik9 comparison. Stop at the first failed
gate. The existing TK32 decoder interprets both sets of streams.

## 4. Result

Calgary shrinks from 906,449 to 904,698 bytes. Sixteen files
shrink, two are byte-identical, and every stream restores exactly.
The saving of 1,751 bytes is 0.193%, below the required 0.25%
(2,267 bytes). Stop before Canterbury, wrapped selection, and
enwik9.

[Experiment report](../../research/experiments/2026-11-25-long-prefix-match-index.md)
