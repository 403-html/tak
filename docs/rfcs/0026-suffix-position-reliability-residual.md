# RFC 0026: Suffix-position reliability residual

- Status: Accepted; position-aware tensor design permitted
- Date: 2026-11-24
- Scope: Tensor representation gate
- Format change: None

## 1. Problem statement

RFC 0013's rejected recurrent MPS applies one shared byte-transition core at
all eight exact-suffix positions. It can represent byte order through hidden
state, but it cannot apply a different transition parameterization at each
position.

RFC 0013 permits untied position cores only after a controlled residual test
shows a position-specific dependency that the shared representation cannot
capture. This RFC performs that test with classical adaptive tables.

## 2. Dependency

For every exact eight-byte suffix prediction event, use these decoder-known
modes:

- match-distance bucket: four values;
- preceding correct-run bucket: six values;
- suffix position: eight values;
- byte at that suffix position: 256 values; and
- prediction outcome: incorrect or correct.

RFC 0001's bigram factorization cannot represent suffix identity, distance,
run state, or suffix position. Increasing its rank does not add these modes.

## 3. Classical controls

Retain RFC 0012's 24-cell distance/run Bernoulli control.

Add a shared-position byte control with one adaptive Bernoulli table for each
`(distance, run, byte)` tuple. All eight suffix positions use the same table.

Add a position-specific candidate with one adaptive Bernoulli table for each
`(position, distance, run, byte)` tuple. Every table starts with one correct
and one incorrect count.

For each model, treat the eight position predictions as experts. Maintain one
eight-expert floating Bayesian mixture. Initialize every expert weight to one.
Before an event, combine the eight exact outcome probabilities with normalized
weights. After the event, multiply each weight by its exact probability and
renormalize in log space. Then update all eight selected Bernoulli tables.

The shared and position-specific models maintain independent mixture weights.
They process events in identical causal order and transmit no fitted state.

## 4. Hypothesis and gate

The position-specific candidate MUST improve the shared-position control by
at least 0.01 bits per matched event on at least three of five Silesia
development tails. It MUST NOT regress in aggregate. It MUST also beat the
24-cell distance/run control in aggregate.

Use the established final-10% Silesia tails. Do not change table
initialization, mixture updates, distance buckets, run buckets, or the gate
after inspecting results.

Stop as soon as three files fail the per-file condition because the gate then
becomes unreachable.

## 5. Accounting

Report matched events, correct events, distance/run bits per event,
shared-position bits per event, position-specific bits per event, and the
position-specific reduction. Use one screening sample per file. This is log
loss, not compressed size.

The direct position-specific state has `8 * 4 * 6 * 256` Bernoulli cells. It
is the mandatory classical control for any later untied tensor proposal.

## 6. Tensor decision

If the gate passes, a later RFC MAY propose untied byte cores. It MUST compare
against the position-specific classical model, RFC 0012's exact-suffix table,
and RFC 0017/RFC 0019 complete artifacts. It MUST count every core byte and
use deterministic integer contraction for decoding.

Passing this residual gate would not prove a tensor compression improvement.
It would only establish a representation limitation worth testing.

If the gate fails, the result weakens the justification for untied RFC 0013
byte cores under this experiment. It does not prove that untied tensor cores
cannot work. A tensor can represent joint higher-order interactions that
these marginal Bernoulli tables do not detect.

Do not implement untied cores from this residual alone after a failure. A
later RFC MAY reopen them if it specifies a joint-interaction diagnostic or a
different representation argument, uses a strong classical control, and sets
new gates before measurement. Increasing rank or tuning the old optimizer
alone remains unjustified.

## 7. Exact decoding

No artifact is created in this RFC. In any later artifact, a tensor may only
produce integer symbol frequencies. The entropy coder MUST still encode every
source decision exactly. Approximate training or factorization MUST NOT alter
restored bytes.

## 8. Result

Accept the classical residual gate. The position-specific model improves the
shared-position control on all five Silesia development tails. Reductions
range from 0.022701 to 0.140829 bits per matched event, above the required
0.01 on every file.

Across 5,410,931 events, weighted log loss falls from 0.549707 to 0.488673
bits per event. The position-specific model also beats the 24-cell
distance/run control on every file and in aggregate. It beats the sparse
exact-suffix control in aggregate, 0.488673 versus 0.497920 bits per event,
but loses to that control on Dickens and Mozilla.

This result establishes transferable suffix-position dependence under the
tested classical model. It permits a new untied, position-aware tensor design
RFC with the position-specific and sparse-suffix models as mandatory
controls. It is not a tensor result or compression result.

Failure would not have ruled out higher-order tensor interactions, as Section
6 states. Conversely, this success does not prove that an untied tensor will
capture them efficiently or repay its metadata.

See
`research/experiments/2026-11-24-suffix-position-reliability-residual.md`.
