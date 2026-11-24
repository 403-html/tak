# RFC 0022: Literal-phase order-2 residual

- Status: Rejected at Silesia opportunity gate
- Date: 2026-11-24
- Scope: Literal probability diagnostic
- Format change: None

## 1. Problem statement

RFC 0019 retains RFC 0017's literal byte model. That model conditions the
exact eight-byte match bonus on the offset inside a literal block. Its
normalized order-2 baseline remains global.

The next literal can have a different distribution at a block boundary than
later in the same literal block, even when its two preceding bytes are equal.
The current baseline cannot represent this interaction.

## 2. Dependency

The candidate dependency has four modes:

1. the decoder-known literal phase;
2. the byte two positions before the literal;
3. the byte immediately before the literal; and
4. the literal byte.

RFC 0001's bigram factorization has only the immediately preceding byte and
the next byte. Increasing its rank cannot add the literal phase or the second
history byte. The dependency is therefore a technically valid tensor
research surface. This RFC does not yet authorize a tensor implementation.

## 3. Hypothesis

A causal mixture of the complete RFC 0019 literal probability and a
phase-specific normalized order-2 probability will save at least 0.01 bits
per input byte across the five Silesia development tails. It MUST improve all
five files.

If the development gate passes, the frozen diagnostic MUST improve at least
15 of 18 Calgary files and save at least 0.008 bits per input byte in
aggregate. Do not change the phases, prior strength, update rule, or gate
after inspecting Silesia.

## 4. Control

Use the frozen RFC 0019 parse. The control probability is the complete RFC
0017 literal model used by RFC 0019. It includes:

- the normalized order-2 baseline with prior strength four;
- the exact eight-byte match prediction; and
- the four phase-specific six-expert integer mixtures.

Match lengths, distances, block types, and non-literal bytes are outside the
comparison. Report savings per complete input byte so the result cannot hide
the limited literal coverage.

## 5. Candidate

Maintain four additional normalized order-2 models with prior strength four.
Each model corresponds to one RFC 0017 literal phase: offset zero, offsets one
through three, offsets four through fifteen, and offsets sixteen and later.

Every model observes every decoded byte to maintain the exact two-byte
history. A model updates its frequency tables only when the current byte is a
literal in that model's phase. Match bytes and literals in other phases only
advance its history.

For phase `p`, maintain two floating Bayesian expert weights. Initialize both
weights to one. The experts are:

1. the complete control probability; and
2. the phase-specific order-2 probability.

Before a literal, normalize the two weights and code with their weighted
probability. After observing the literal, multiply each weight by its expert
probability. Renormalize in log space. This diagnostic uses ideal bits. It is
not a proposed decoder arithmetic.

All model state is causal and decoder-reproducible. The diagnostic transmits
no fitted parameters.

## 6. Accounting and gates

Count the negative base-2 logarithm of the exact literal probability for both
control and candidate. Include every literal from the physical RFC 0019
parse. Report literal count, control bits, candidate bits, saved bits, and
saved bits per complete input byte.

If Silesia fails either Section 3 condition, reject this dependency and stop
before Calgary. Do not tune mixture learning rates, phase boundaries, or
prior strength on the measured files.

If both opportunity gates pass, a later RFC MAY propose a deterministic
integer classical artifact. That artifact MUST beat RFC 0019 as a complete
physical artifact with exact restoration, bounded memory, and decode time no
more than 1.25 times the control. Opportunity bits are not artifact bytes.

## 7. Tensor gate

An opportunity win does not by itself authorize a tensor. The direct
phase-specific order-2 model and its two-expert mixture are the mandatory
classical control.

A later tensor RFC MUST specify how parameter sharing reduces the direct
model's state or improves its probability. It MUST count every tensor
parameter and compare against the complete classical artifact. Merely
increasing a previously rejected rank remains prohibited.

## 8. Stop conditions

Reject the candidate if any Silesia file regresses or aggregate savings are
below 0.01 bits per input byte. If Silesia passes and Calgary fails, preserve
both results and stop before an artifact or enwik9 run.

Do not report a tensor improvement, artifact improvement, or enwik9 result
from this diagnostic.

## 9. Result

Reject the candidate. It improves only Mozilla. Dickens, OOffice, Samba, and
XML each regress by four ideal bits. Those four bits are the one-bit initial
Bayesian mixture cost in each literal phase before the mixture selects the
control expert.

Mozilla saves 75,734.392 ideal bits, almost entirely in phase three. Across
all five files, the candidate saves 75,718.392 bits over 9,451,681 input
bytes, or 0.008011 bits per input byte. This is below the required 0.01 bits
per input byte and fails the requirement that every file improve.

The dependency is corpus-specific under this method. Calgary, a deterministic
artifact, enwik9, and tensor work were not run. Do not tune mixture priors,
phase boundaries, or prior strength on these files.

See
`research/experiments/2026-11-24-rejected-literal-phase-order2-residual.md`.
