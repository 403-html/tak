# RFC 0027: Position-aware match-reliability tensor

- Status: Rejected
- Date: 2026-11-24
- Scope: Tensor representation gate
- Format change: None

## 1. Problem statement

RFC 0013 applies one byte-transition matrix at all eight positions of an
exact eight-byte suffix. RFC 0026 shows that a byte's relationship with match
reliability changes with its suffix position. The position-specific
classical model improves the shared-position control on all five measured
Silesia development tails.

The classical result does not determine whether a tensor will win. Marginal
Bernoulli tables do not represent joint interactions among multiple suffix
bytes. A position-aware tensor can represent such interactions through its
contracted hidden state.

## 2. Hypothesis

A rank-2 position-aware matrix product state can use joint interactions among
match distance, correct-run state, suffix position, and ordered suffix bytes
to beat the RFC 0026 classical controls after quantization and complete model
metadata are counted.

This is not a general claim about arbitrary binary data. Incompressible input
cannot be compressed. The hypothesis applies only when the measured match
state contains transferable dependency.

## 3. Frozen representations

Use the RFC 0013 mode order:

```text
distance bucket (4)
run bucket (6)
suffix byte 0 through suffix byte 7 (256 each)
outcome: incorrect or correct (2)
```

Fix the internal rank at two. Do not test a larger rank in this RFC.

Rerun the RFC 0013 shared-transition rank-2 model as a frozen tensor control.
Test these two position-aware candidates.

### 3.1 Position-modulated candidate

Serialize one shared byte core and eight position gates. For suffix position
`p`, byte `b`, left state `i`, and right state `j`, define the effective
transition as:

```text
transition[p, b, i, j] = byte[b, i, j] * gate[p, i, j]
```

All factors are positive. This factorization adds a position-byte interaction
without serializing eight complete byte cores. It contains:

```text
6*r + 270*r*r unsigned 16-bit values
```

At rank two, the complete core payload is 2,184 bytes.

### 3.2 Fully untied candidate

Serialize one independent byte-transition core for each suffix position. It
contains:

```text
6*r + 2054*r*r unsigned 16-bit values
```

At rank two, the complete core payload is 16,456 bytes.

The fully untied candidate tests representation quality even if its metadata
gate fails. A metadata failure does not show that untied cores lack predictive
value.

## 4. Training method

Use RFC 0013's deterministic full-batch exponentiated-gradient method. Use
100 epochs. Set the initial learning rate to 1.0 and divide it by the square
root of the epoch number.

Initialize the shared and untied byte transitions near the identity. Initialize
every position gate near one so that the position-modulated candidate starts
near the shared-transition control. Do not tune epochs, learning rate, rank,
or initialization after inspecting development results.

Training MAY use floating-point arithmetic. Preserve full-precision,
quantized, and integer-contraction losses separately.

## 5. Data split and controls

Use the established final-10% Silesia development tails. Within each tail,
use the first 90% of bytes for training and the final 10% for evaluation.
Rebuild the bounded match table from empty state in each partition.

Score these causal controls on the evaluation partition:

1. the 24-cell distance/run model;
2. the sparse exact-suffix model;
3. RFC 0026's shared-position model;
4. RFC 0026's position-specific model; and
5. RFC 0013's shared-transition rank-2 tensor.

The classical controls MUST adapt only to earlier evaluation events. They
MUST NOT receive fitted training state. Use independent mixture weights for
the shared-position and position-specific controls.

## 6. Gates and stop conditions

Evaluate the position-modulated and fully untied candidates independently.
For a candidate to pass the representation gate, its full-precision loss MUST
beat both the sparse exact-suffix and position-specific controls on at least
three of five files. It MUST NOT regress against either control in aggregate.

For a candidate to pass the implementation gate, its deterministic integer
loss plus every serialized core bit MUST satisfy the same condition. Its core
payload MUST be smaller than its modeled aggregate saving against the stronger
classical control.

Stop a candidate as soon as three per-file representation failures make its
gate unreachable. Preserve all measured failures. Quantization cannot repair
a full-precision failure.

If neither candidate passes, reject these exact representations and training
method. Such a result weakens their justification under this experiment. It
does not rule out other tensor topologies, other joint interactions, or tensor
networks in general. Rank or optimizer tuning alone does not justify another
attempt.

If one candidate passes, freeze it before held-out Calgary validation. Do not
implement a compression artifact from development log loss alone.

## 7. Deterministic integer contraction

Quantize each serialized factor to a positive unsigned 16-bit integer.

For the position-modulated candidate, calculate each next-state component as:

```text
next[j] = sum(state[i] * byte[b, i, j] * gate[p, i, j], i = 0..1)
```

The maximum sum is less than `2 * 65536^3`, which fits in an unsigned 64-bit
integer. The fully untied candidate uses RFC 0013's two-factor transition.

After each transition, normalize with RFC 0013's unsigned 128-bit scaling:

```text
state[j] = 1 + floor(next[j] * 65534 / max(next))
```

Reject a model if the maximum is zero. Produce two positive outcome scores.
Map them to positive integer entropy-coder frequencies with a specified total
before any compression artifact is implemented.

## 8. Exactness and scope

This RFC measures probability models. It creates no compressed artifact and
assigns no TAK-C method identifier.

A later artifact MUST encode every source decision with a lossless entropy
coder. Approximate tensor training MUST NOT alter reconstructed bytes. The
decoder MUST use only bounded fixed-width integer arithmetic and transmitted
model data. Exact restoration, not exact prediction, provides losslessness.

## 9. Reporting

Report, for every measured file and candidate:

- training and evaluation event counts;
- all four classical-control losses;
- shared-tensor, full-precision, quantized, and integer losses;
- core bytes and integer loss with amortized metadata;
- training and evaluation wall time;
- peak resident memory; and
- the exact input size and SHA-256 digest.

Use one screening sample only for the representation gate. Label its time and
memory measurements non-publishable. If a candidate reaches an artifact,
follow the repository benchmark methodology and verify exact byte restoration.

## 10. Result

Reject both candidates. Dickens, XML, and OOffice all fail the
full-precision representation gate, so the required three passes are
unreachable. Stop without Samba or Mozilla.

The fully untied tensor improves the frozen shared tensor on all three files.
Its weighted loss is 0.564542 rather than 0.590796 bits per event. This shows
that positional specialization has value inside the tested tensor family.
The position-specific classical control is still stronger on every file. Its
weighted loss is 0.467389 bits per event.

The position-modulated tensor has weighted loss of 0.590531 bits per event.
It does not retain the untied model's improvement. Quantization and integer
contraction add negligible loss, but metadata makes both complete diagnostics
substantially worse.

This result rejects the exact rank-2 position-modulated and fully untied MPS
representations under the frozen optimizer. It does not rule out tensor
networks or higher-order interactions in general. Do not retry with a higher
rank or optimizer tuning alone.

See
`research/experiments/2026-11-24-rejected-position-aware-tensor.md`.

