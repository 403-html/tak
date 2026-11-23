# RFC 0010: High-confidence exact-match control

- Status: Rejected
- Date: 2026-11-23
- Scope: Classical research artifact only
- TAK-C method identifier: None

## 1. Problem statement

RFC 0009 beats gzip on Calgary but remains 108,407 bytes larger than
Zstandard-19 and 143,620 bytes larger than xz. RFC 0008 showed that exact
8-byte continuation predictions are often correct and that a divisor-1 bonus
had lower ideal code length than divisor 4 on all Silesia groups.

## 2. Hypothesis

Keep the RFC 0009 suffix, table capacity, hash, collision behavior, baseline,
and artifact accounting unchanged. Change only the match bonus to:

```text
B = T
```

The candidate passes the Calgary target only if its aggregate complete size is
smaller than both 921,073-byte Zstandard-19 and 885,860-byte xz results. It
MUST restore every input, remain below two times RFC 0009 decode time in every
group, and use less than one gigabyte peak RSS.

## 3. Stop condition

Test divisor 1 once. Do not screen intermediate divisors or select values per
file. Reject this mechanism if it does not beat both external targets. Preserve
the result even if it improves RFC 0009.

Calgary is development data for this confidence change. A passing candidate
requires validation on data not used to choose the bonus before an enwik9 run.

## 4. Format

Use research magic `TKH8` with the RFC 0009 28-byte layout. The fixed magic
identifies divisor 1. The artifact transmits no model or table state.

## 5. Tensor scope

This is a classical probability-calibration experiment. It authorizes no
tensor model. Increasing tensor rank does not address match confidence.

## 6. Decision

Reject the mechanism. The divisor-1 artifact reduced RFC 0009 Calgary size by
2.80%, from 1,029,480 to 1,000,695 bytes. It improved prose/reference by
2.61%, technical/source by 4.78%, and binary by 1.89%. Every file restored
exactly.

The result remains 79,622 bytes larger than Zstandard-19 and 114,835 bytes
larger than xz. It therefore fails the explicit target. Do not test more bonus
divisors or promote `TKH8`.

The remaining opportunity is not match confidence alone. A later RFC may test
explicit match-run tokens that represent a complete repeated span rather than
coding each continuation byte independently.
