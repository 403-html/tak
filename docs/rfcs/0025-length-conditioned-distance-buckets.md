# RFC 0025: Length-conditioned distance buckets

- Status: Rejected at Silesia opportunity gate
- Date: 2026-11-24
- Scope: Classical match-distance probability diagnostic
- Format change: None during opportunity stage

## 1. Problem statement

RFC 0019 uses one adaptive 32-symbol distribution for every match-distance
bucket. Short and long matches can have different distance distributions.
The decoder knows the match length before it decodes the distance, so match
length can select a distance model without a selector bit.

The current model cannot represent this dependency. RFC 0020 tested the
preceding block type, not the current match length.

## 2. Hypothesis

Four distance-bucket distributions selected by current match length will save
at least 0.005 bits per complete input byte across the five Silesia
development tails. The candidate MUST improve all five files.

If Silesia passes, the frozen candidate MUST improve at least 15 of 18 Calgary
files and save at least 0.004 bits per input byte in aggregate. Do not change
the classes, distributions, initialization, or gates after inspecting
Silesia.

## 3. Models

The control is RFC 0019's one adaptive 32-symbol distance-bucket
distribution. The candidate uses four independent distributions. Select the
candidate distribution as:

```text
class = min(match_length - 8, 3)
```

Thus the classes are match lengths 8, 9, 10, and 11 or more bytes. This is a
fixed minimum-length-relative mapping. It is not selected from results.

Every distribution starts with one count per symbol. Use RFC 0019's existing
adaptive update and halving rules. Distance bucket mapping and equiprobable
remainder bits do not change.

## 4. Accounting

Use the frozen RFC 0019 parse. Process matches in decoder order. For each
match, measure the ideal arithmetic cost of the distance-bucket symbol under
the global control and selected candidate distribution. Count all raw
distance remainder bits equally in both totals.

Report match count, control bits, candidate bits, saved bits, and saved bits
per complete input byte. The candidate transmits no parameters because the
decoder reproduces all adaptive state.

This is an opportunity result. It is not compressed size.

## 5. Artifact gate

If both opportunity gates pass, a later RFC MAY define a distinct research
artifact. It MUST retain RFC 0019's literal model, parse, structural block
types, lengths, checksum, and resource limits. Only distance-bucket model
selection may change.

The complete artifact MUST:

- be smaller than RFC 0019 on every Silesia development file;
- save at least 0.003 bits per input byte in aggregate;
- restore every byte exactly;
- keep median decode time within 1.10 times RFC 0019; and
- keep encoder and decoder peak RSS below one gigabyte.

Use one warm-up and five measured repetitions. Freeze the implementation
before Calgary. Only a passing Calgary artifact permits bounded identity and
enwik9 validation.

## 6. Tensor scope

This RFC does not authorize a tensor. Four length classes and four small
adaptive tables have a direct classical representation. No parameter-sharing
problem has been established.

A later tensor RFC still requires a different transferable longer-context
dependency that the rejected bigram factorization cannot represent and that
beats its direct classical control. Increasing rank remains unjustified.

## 7. Stop conditions

If any Silesia file regresses or aggregate savings are below 0.005 bits per
input byte, reject the candidate and stop before Calgary and artifact work.
Do not tune length thresholds or add preceding-length state after seeing the
result.

## 8. Result

Reject the candidate. All five Silesia files regress. Across 259,743 matches
and 9,451,681 input bytes, the four tables add 3,824.659 ideal bits, or
0.000405 bits per input byte.

The length classes fragment evidence more than they improve distance-bucket
probabilities. Do not tune class thresholds or add previous-length state on
these files. Calgary, artifact, and enwik9 work were not run.

See
`research/experiments/2026-11-24-rejected-length-conditioned-distance.md`.
