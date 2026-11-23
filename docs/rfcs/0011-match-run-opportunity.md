# RFC 0011: Match-run opportunity diagnostic

- Status: Rejected
- Date: 2026-11-23
- Scope: Research diagnostic only
- Format change: None

## 1. Problem statement

RFC 0009 codes every matched continuation as another arithmetic-coded byte.
RFC 0010 shows that confidence calibration alone cannot close the Calgary gap
to Zstandard-19 or xz. Consecutive correct predictions may be represented more
efficiently as one run token.

## 2. Hypothesis

Measure maximal consecutive runs in which the fixed RFC 0009 match prediction
equals the actual byte. For minimum lengths 2, 4, 8, and 16, record covered
bytes, run count, and the RFC 0009 ideal code bits spent on those bytes.

For a gross opportunity bound, charge one marker bit plus an unsigned LEB128
run length for each selected run. This omits literal/control-stream costs and
MUST NOT be reported as compressed size.

Proceed to a token-format RFC only if length-4 runs cover at least 10% of
Calgary and their gross opportunity exceeds the 918,680-bit gap between RFC
0010 and xz. Otherwise reject match-run tokenization at this granularity.

## 3. Method

Use the exact RFC 0009 table, hash, divisor-4 probabilities, and Calgary file
boundaries. Process every file independently. A run ends on the first missing
or incorrect prediction. Do not join runs across files.

Preserve all thresholds. Do not change the minimum length after measurement.

## 4. Stop condition

This diagnostic authorizes no format. A passing result permits a later RFC to
define control coding, length coding, literal coding, decoder state, resource
limits, and complete artifact accounting. A tensor model remains unauthorized.

## 5. Decision

Reject the proposed raw marker and LEB128 representation. Length-4 runs cover
990,179 bytes, or 30.45% of Calgary, but their token bound is 26,921 bits worse
than the RFC 0009 model bits spent on the same bytes. The fixed gate fails.

Length-8 runs have the best measured gross result: 757,095 covered bytes and
131,959 saved bits before literal/control-stream costs. This is only 14.4% of
the 918,680-bit xz gap. It cannot justify a format implementation.

The next match-format experiment must use entropy-coded control and length
symbols or a full LZ parser. Do not implement raw run markers and do not report
the gross bound as compressed size.
