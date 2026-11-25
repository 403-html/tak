# RFC 0033: Reuse TK32 decode boundaries

- Status: Calgary and enwik9 verified
- Date: 2026-11-25
- Scope: Decoder implementation

The RFC 0032 decoder finds literals by binary search. It recomputes
both boundaries for each probe and looks up the same suffix rows for
every boundary. Test whether collecting the eligible rows once and
preserving the two known search boundaries reduces decode time.

The first search bracket is [0, 256] in byte boundaries. Its range
is [0, 2^20 + 256). Evaluate one midpoint boundary per iteration.
The comparison with the scaled arithmetic value selects a half.
Keep the evaluated boundary as the retained half's new endpoint.
When the bracket spans one byte, return the two recorded cumulative
values. Use the RFC 0032 formulas and arithmetic exactly.

Compare old and new decoders on the same complete TK32 artifacts for
all 18 Calgary files. Both must restore every input byte. Verify the
new encoder produces identical artifacts. Run one warm-up and five
measured repetitions per decoder. Advance if the median aggregate
decode time falls by at least 20% and peak RSS grows by at most 2 MiB.
Then decode the existing complete enwik9 artifact and verify every
restored byte. Compare the single timed screening run with RFC 0032's
recorded decode-and-compare run on the same artifact. Treat this historical
timing comparison as screening evidence.
No serialization or model changes are part of this experiment.

## Result

Calgary median aggregate decode time decreases from 2.820146 to
2.085782 seconds with exact restoration and byte-identical artifacts.
Peak measured RSS is 44,060 KiB, compared with 44,124 KiB in the control.
The existing enwik9 stream restores exactly in 390.689 seconds with
113,952 KiB peak decode-and-compare RSS. The recorded RFC 0032
decode-and-compare time is 451.449 seconds.
[Experiment report](../../research/experiments/2026-11-25-tk32-boundary-search.md)
