# RFC 0047: Keep a shorter match from the chain

- Status: Rejected at complete-artifact gate
- Date: 2026-11-25
- Scope: Classical encoder search
- Format change: None

RFC 0046 keeps only the longest match recovered by the bounded
chain. An earlier length record in the same traversal is shorter
and closer. It can have a cheaper distance or leave a better
next match. Test whether retaining it improves complete files.

Keep the 1 MiB window, the 16-position traversal limit, and all
prices and forced-match decisions from RFC 0046. Whenever a
chain candidate strictly improves the best length beyond the
ordinary candidates, save the old record before replacing it.
Submit both the final record and its predecessor to the solver,
in that order, after the ordinary candidates. Omit an empty
predecessor. The predecessor cannot change the forced decision.
Use the same TK32 bitstream and decoder.

Compare RFC 0034, RFC 0046, and the candidate on all 18 Calgary
files and the ten independent Canterbury files. Exclude ptt5
because it duplicates Calgary pic. Measure complete file size,
wall time, throughput, and peak RSS. Require exact restoration
for every trial and reproduce the recorded Calgary control
and chain hashes. Run Canterbury even if Calgary misses the gate.

Advance only if the candidate saves at least 0.05% against
RFC 0046 and 0.25% against RFC 0034 on both corpora with peak RSS
below 1 GiB. Then test the RFC 0038 transforms and full enwik9.
Initial single-run timings are screening measurements.

## Result

Retaining the shorter match reduces the chain aggregate by eight
bytes on Calgary and one byte on independent Canterbury. The
incremental gate requires 452 and 295 bytes, respectively.
Every stream restores exactly. Stop before byte shuffle and enwik9.

[Experiment report](../../research/experiments/2026-11-25-chain-shorter-match.md)
