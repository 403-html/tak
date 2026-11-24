# RFC 0023: Match-distance literal continuation

- Status: Rejected at Silesia opportunity gate
- Date: 2026-11-24
- Scope: Literal probability diagnostic
- Format change: None

## 1. Problem statement

RFC 0019 retains the distance of the preceding LZ match in decoder state.
During the following literal block, the decoder can read the byte at that
distance behind the current output position. This remote source advances with
the literal position.

The existing residual profile measures only exclusion of the first source
byte after a mismatch. It does not test whether the complete source
continuation improves literal probabilities throughout the literal block.

## 2. Dependency

For a literal at output position `P`, let `D` be the distance of the most
recent match block. If `P >= D`, define the source prediction as the already
decoded byte at position `P - D`.

The dependency combines the current literal context with a remote byte chosen
by prior match state. RFC 0001's preceding-byte bigram factorization cannot
represent the match distance or remote source. Increasing its rank cannot add
these modes.

This RFC does not authorize a tensor. It first tests a direct classical
mixture with no fitted metadata.

## 3. Hypothesis

A causal mixture of source-byte bonus experts will reduce RFC 0019 ideal
literal cost by at least 0.01 bits per complete input byte across the five
Silesia development tails. It MUST improve all five files.

If Silesia passes, the frozen diagnostic MUST improve at least 15 of 18
Calgary files and save at least 0.008 bits per input byte in aggregate. Do not
change the expert set, state, parse, or gates after inspecting Silesia.

## 4. Control

Use the frozen RFC 0019 parse. The control probability is the complete RFC
0019 literal probability. It includes the normalized order-2 baseline, exact
eight-byte match prediction, and RFC 0017's integer literal-phase mixture.

Match lengths, match distances, block types, and non-literal bytes are outside
the ideal-bit comparison. Report savings per complete input byte.

## 5. Candidate

Maintain one floating Bayesian mixture with six experts. Initialize every
expert weight to one. Let `T` be the complete control total and let `F` be the
control frequency of the actual literal. The experts add these bonuses to the
source prediction:

```text
T, floor(T / 2), floor(T / 4), floor(T / 8), floor(T / 16), 0
```

For bonus `B`, the exact-symbol probability is:

```text
(F + B if source prediction equals the literal, else F) / (T + B)
```

Before each eligible literal, normalize the six weights and code with their
weighted probability. After observing the literal, multiply each weight by
its expert probability. Renormalize in log space.

Do not update the mixture when no preceding match distance exists or `P < D`.
Update the preceding distance after every match block. The source advances
through all bytes in the literal block.

The diagnostic uses floating-point ideal bits. It proposes no decoder
arithmetic and transmits no parameters.

## 6. Accounting and gates

Count all literal ideal bits from the control and candidate. Record eligible
source events, correct source predictions, control bits, candidate bits,
saved bits, and saved bits per complete input byte.

If Silesia fails either Section 3 condition, reject the mechanism and stop
before Calgary. Do not tune separate mixtures by phase, distance, file type,
or source agreement after seeing the results.

If both opportunity gates pass, a later RFC MAY specify a deterministic
integer artifact. It MUST beat the complete RFC 0019 artifact, restore every
byte exactly, keep decoder memory bounded below one gigabyte, and keep median
decode time within 1.25 times RFC 0019. Ideal bits are not artifact bytes.

## 7. Tensor gate

A passing opportunity result would establish a longer-context dependency,
not a tensor win. The six-expert mixture is the mandatory classical control.

A later tensor RFC MUST name additional interacting modes that need parameter
sharing, count every parameter and metadata byte, and beat the complete
classical artifact. Merely increasing a rejected tensor rank remains
prohibited.

## 8. Stop conditions

Reject if any Silesia file regresses or aggregate savings are below 0.01 bits
per input byte. If Silesia passes and Calgary fails, preserve both results and
stop before artifact and enwik9 work.

Do not report compression, round-trip, tensor, or enwik9 improvement from this
ideal diagnostic.

## 9. Result

Reject the candidate. Mozilla saves 318,563.112 ideal bits and OOffice saves
54,947.272 bits. Dickens, Samba, and XML each regress by 2.584963 bits. This
equals the initial `log2(6)` Bayesian mixture cost before the zero-bonus expert
becomes dominant.

The aggregate saves 373,502.630 ideal bits over 9,451,681 input bytes, or
0.039517 bits per input byte. It passes the aggregate threshold but improves
only two of five files. The all-files transfer requirement fails.

Do not add a file-type selector or tune separate mixtures after seeing this
result. Calgary, artifact, enwik9, and tensor work were not run. See
`research/experiments/2026-11-24-rejected-match-distance-literal-continuation.md`.
