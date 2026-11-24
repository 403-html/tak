# RFC 0020: Post-literal match state

- Status: Rejected at Silesia opportunity gate
- Date: 2026-11-24
- Scope: Classical match-length and distance-bucket coding
- TAK-C method identifier: None

## 1. Problem statement

RFC 0019 exposes whether each match follows a literal block or another match.
The current artifact uses one adaptive match-length bucket distribution and one
adaptive distance-bucket distribution for both states.

The two states can represent different mechanisms. A match after a literal
starts a new matched region. A consecutive match continues a region by using a
different source or length. Merging their token statistics can spend bits.

This RFC measures that decoder-known dependency before changing the format.

## 2. Hypothesis

Two match-length bucket models and two distance-bucket models, selected by the
preceding block type, will reduce ideal token cost by at least 0.01 bits per
input byte on the five Silesia development tails. The combined candidate MUST
improve every file.

The frozen candidate MUST then improve at least 15 of 18 Calgary files and
save at least 0.008 bits per input byte in aggregate. Do not change the state,
bucket mapping, or gate after inspecting results.

## 3. Opportunity model

Define state zero when a match follows the stream start or another match.
Define state one when a match follows a literal block. The decoder derives the
state from the block sequence. No state bit is stored.

Each state has:

1. one 32-symbol adaptive match-length bucket distribution; and
2. one 32-symbol adaptive distance-bucket distribution.

All distributions start with unit frequencies and use the existing update and
halving rules. Match-length and distance remainder bits remain equiprobable
and unchanged. RFC 0019 block types, literal coding, parser, and byte model
remain frozen.

## 4. Accounting

The control is the ideal bucket and raw remainder cost from RFC 0019's global
match-length and distance distributions. The candidate replaces only the two
bucket distributions with the state-selected distributions from Section 3.

The diagnostic includes both state tables from their unit-frequency initial
conditions. It transmits no parameters. Ideal bits are an opportunity result,
not artifact bytes.

## 5. Complete-artifact gate

Implement a distinct research artifact only if both opportunity gates pass.
The complete Silesia gate requires:

- exact restoration for every file;
- all five artifacts smaller than RFC 0019;
- at least 0.008 saved bits per input byte in aggregate;
- median decode time at most 1.10 times RFC 0019; and
- encoder and decoder peak RSS below one gigabyte.

Use one warm-up and five measured repetitions. If this gate passes, freeze the
implementation and validate all 18 Calgary files. Calgary MUST improve at
least 15 files, save at least 2,500 bytes in aggregate, restore every byte, and
keep median decode time within 1.10 times RFC 0019.

Only a passing Calgary artifact permits bounded implementation and enwik9
validation. Do not tune on Calgary or enwik9.

## 6. Tensor scope

This RFC does not authorize a tensor model. Two binary parser states and four
small adaptive distributions have a direct classical representation. A tensor
would not provide useful parameter sharing.

A later tensor RFC still requires a different measured longer-context
dependency that the rejected bigram factorization and these classical state
tables cannot represent. Increasing tensor rank remains unjustified.

## 7. Stop conditions

If the opportunity gate fails, reject this state split. Do not add previous
length, previous distance, or multi-block histories on the measured corpora.
Record negative and mixed results. Do not combine failed components with an
oracle selector.

## 8. Opportunity result

Reject the candidate. It improves only one of five Silesia development files.
The combined aggregate grows by 4,993.098 ideal bits, or 0.000528 bits per
input byte. The required result is an improvement on all five files and at
least 0.01 saved bits per input byte.

The length split regresses in aggregate. The distance split also regresses in
aggregate. Do not combine them with a per-file oracle. Calgary, artifact, and
enwik9 work were not run.

See `research/experiments/2026-11-24-rejected-post-literal-match-state.md`.
