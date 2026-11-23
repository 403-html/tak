# Retained candidate, rejected target: high-confidence exact match

## Status

Retain the RFC 0010 artifact and its 2.80% gain as a high-compression research
candidate. Reject only the explicit target of beating both Zstandard-19 and xz
on Calgary. The candidate needs held-out validation before it can replace RFC
0009 as the control.

## Result

| Group | RFC 0009 bytes | Divisor-1 bytes | Reduction |
|---|---:|---:|---:|
| Prose/reference | 646,504 | 629,660 | 2.61% |
| Technical/source | 162,584 | 154,813 | 4.78% |
| Binary | 220,392 | 216,222 | 1.89% |
| Total | 1,029,480 | 1,000,695 | 2.80% |

All 18 files restored exactly. Raw single-sample results are in
`benchmarks/reports/raw/2026-11-23-match8-strong-calgary.tsv`, SHA-256
`0eec940133d0c256ebbca291741a7d2fe1eecc5cb3c75a84844295d63ef29edd`.

The candidate is 79,622 bytes larger than the measured 921,073-byte
Zstandard-19 result and 114,835 bytes larger than the measured 885,860-byte xz
result. It must remove 7.96% and 11.48% of its current bytes to equal them.

## Decision

Stop confidence tuning. The fixed target required beating both external
streams, and the candidate did neither. Preserve both the improvement and the
failed target.

The next ratio experiment should test explicit match-run tokenization. That
mechanism represents long repeated spans directly and is materially different
from changing the probability assigned to one predicted byte.
