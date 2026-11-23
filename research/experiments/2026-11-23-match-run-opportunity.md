# Rejected diagnostic: raw match-run tokens

## Status

Reject RFC 0011's raw marker and LEB128 run representation. Consecutive match
runs are common, but this token accounting cannot beat the existing arithmetic
probabilities and is far too small to close the xz gap.

## Result

| Minimum run | Runs | Covered bytes | Coverage | Gross saved bits | Gross saved bytes |
|---:|---:|---:|---:|---:|---:|
| 2 | 153,010 | 1,168,761 | 35.95% | -508,021 | -63,503 |
| 4 | 78,439 | 990,179 | 30.45% | -26,921 | -3,365 |
| 8 | 32,522 | 757,095 | 23.28% | 131,959 | 16,495 |
| 16 | 11,071 | 538,979 | 16.58% | 113,945 | 14,243 |

The fixed length-4 gate fails because the bound regresses. The best positive
row, length 8, reaches only 14.4% of the 918,680-bit difference between RFC
0010 and xz. It also omits literal/control-stream cost, so it is not compressed
size.

Raw rows for every Calgary file are in
`benchmarks/reports/raw/2026-11-23-match-run-calgary.tsv`, SHA-256
`03cb4552da2b276e9b929a38f35f4b9aab4cafea131a24d67091bf7b454a0a0e`.

## Decision

Preserve the negative result. Do not implement the raw token format. A future
RFC may test entropy-coded control/length alphabets or an LZ parser with exact
complete-stream accounting. That is a new mechanism, not a parameter tweak.

RFC 0009 remains the validated enwik9 control at 315,578,766 bytes. M1 remains
achieved; M2 still requires removing 118,578,766 bytes or 37.57%.
