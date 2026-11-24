# RFC 0030: Short-suffix literal backoff

- Status: Rejected at Silesia opportunity gate
- Date: 2026-11-24
- Scope: Classical literal probability diagnostic

RFC 0019 falls back to normalized order-2 when its exact eight-byte
suffix table has no prediction. Test whether the most recent continuation
of the exact three-byte suffix can improve those literal predictions.

Keep the RFC 0019 parse and probability model fixed. Maintain a direct
table of 262,144 entries for three-byte suffixes. Form each key in
oldest-byte-first order. Multiply the key by `0x9e3779b1` modulo
2^32 and take the upper 18 bits as the table index. Compare the entire
key on lookup. After each byte, including a match copy, replace the
entry for the preceding suffix with that key and its new continuation.
The first three bytes cannot supply a prediction.

Use the candidate only on literals with a matching three-byte entry and
no eight-byte prediction. Add a bonus to the short-suffix continuation
using one RFC 0017 six-expert integer mixture, initialized with unit
weights. Update those weights only on eligible literals, using the
baseline range and short-suffix hit outcome. All other literal
probabilities equal the control. The control model updates as usual.

Sum ideal literal bits from the integer ranges for both models and
report savings per complete input byte. The candidate derives its state
from restored bytes. The diagnostic produces no artifact.

Require at least 0.016 ideal bits saved per input byte across the five
established Silesia tails and improvements on at least three. If it
passes, validate the frozen candidate independently on all 18 Calgary
files, requiring at least 0.012 bits saved per input byte and at least
12 improvements. End at the first failed gate. Passing both gates
permits a separate complete-artifact experiment with exact restoration,
bounded memory, and decoder timings before enwik9.

## Result

Four of five files improved. The aggregate saved 120,369.746 ideal bits,
or 0.012735 bits per input byte. This falls short of 0.016. OOffice
regressed by 3,621.358 ideal bits. The experiment ended at the development
gate. See [the report](../../research/experiments/2026-11-24-rejected-short-suffix-backoff.md).
