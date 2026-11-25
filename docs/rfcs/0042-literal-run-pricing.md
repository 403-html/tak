# RFC 0042: Price literal run lengths during parsing

- Status: Rejected at Calgary complete-artifact gate
- Date: 2026-11-25
- Scope: Classical encoder search
- Format change: None

## 1. Hypothesis

RFC 0034 prices a new literal run as if it contains one byte.
Extending the run pays only the next literal's probability.
TK32 pays a length bucket and remainder for the complete run.
Charging the change in length price on each extension can improve
the parser's decisions around match and literal boundaries.

## 2. Method

Use the RFC 0034 search, prices, models, and segment boundaries.
Keep the run length on the best path ending in a literal at the
current position. When the path starts a run, pay the existing
length-one and type prices. When the path extends a run of length
n to n+1, add the difference between their length prices. A
length price includes the frozen adaptive bucket price and its
remainder width. At a segment boundary, charge only the change
from any carried run length. A carried run can extend or end
in a match. It cannot restart at the segment boundary. Keep the
existing tie rules.
The solver still keeps one path per literal and match state at
each position. It approximates the search because different paths
can reach the same position with different run lengths.

This affects only choices in the encoder. The existing TK32 decoder
accepts either parse and retains its limits and validation.

## 3. Gates

Compare complete identity TK32 streams on the 18 independent Calgary
files. Require exact restoration, at least 0.25% aggregate savings,
and at least 10 smaller files. Record time, throughput, and peak RSS.
Stop at the first failed gate.

If Calgary passes, require at least 0.25% aggregate savings and exact
restoration on independent Canterbury, excluding ptt5. Then rerun the
RFC 0038 profile selection on Calgary with the new parser to measure
the remaining gap to xz. Validate complete enwik9 streams with the same
accounting, measured resources, and exact restoration.

## 4. Result

The complete Calgary total grows from 906,449 to 907,499 bytes.
Five files shrink, 12 grow, and one retains the same size.
All 36 streams restore exactly. Both Calgary size requirements
fail. Stop before Canterbury, wrapped selection, and enwik9.

[Experiment report](../../research/experiments/2026-11-25-literal-run-pricing.md)
