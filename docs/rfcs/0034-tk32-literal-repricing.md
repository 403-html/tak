# RFC 0034: Price TK32 literal backoff during parsing

- Status: Calgary and Canterbury gates passed
- Date: 2026-11-25
- Scope: Encoder parse experiment

The TK32 encoder uses RFC 0016's parse. Its literal prices omit the
RFC 0032 variable-order backoff that the coder applies. Test whether
pricing this backoff reduces complete TK32 size.

Change the parser's literal price only when the eight-byte lookup
misses and a longer context row is available. Use the exact TK32
integer probability in that case. For all other bytes retain the
existing Match8 price, including its bonus when the lookup hits.
The backoff rows observe every input byte in order. This makes their
prices independent of candidate parse paths. Keep the match finder,
length and distance prices, and segment limits fixed.

Encode with the existing TK32 syntax and decode with the existing
TK32 decoder. Compare complete independent files on Calgary, then
Canterbury if Calgary passes. Advance if each aggregate shrinks by
at least 0.4% and at least 12 of 18 Calgary files and 6 of 10
independent Canterbury files shrink. Exclude Canterbury ptt5 from the
independent aggregate because it duplicates Calgary pic. Verify every
restoration byte for byte. Run the bounded enwik9 screen if both gates
pass. Preserve size, time, and peak RSS for every measured artifact.
The initial screens use one repetition.

## Results before enwik9

The candidate produces 906,449 bytes on Calgary, saving 11,987 bytes.
Seventeen of eighteen files improve. Independent Canterbury saves
6,782 bytes and improves nine of ten files. Every measured stream
restores exactly. Both results pass the size gate.
[Experiment report](../../research/experiments/2026-11-25-tk32-literal-repricing.md)
