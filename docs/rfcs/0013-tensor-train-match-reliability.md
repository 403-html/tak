# RFC 0013: Tensor-train match-reliability model

- Status: Proposed
- Date: 2026-11-23
- Scope: Tensor research artifact only
- TAK-C method identifier: None

## 1. Problem statement

RFC 0012 finds residual match reliability in the exact 8-byte suffix after
conditioning on match distance and correct-run state. A 24-cell table cannot
represent suffix identity. A sparse exact-suffix table represents it but does
not share evidence and regresses on OOffice.

This is a specific dependency that RFC 0001 cannot represent. Increasing the
rank of its bigram tensor would not add suffix, distance, or run modes.

## 2. Hypothesis

A tensor train over match state can share reliability evidence across suffixes
and beat both the distance/run table and sparse exact-suffix control after all
serialized cores are counted.

The input modes are:

```text
distance bucket (4)
run bucket (6)
suffix byte 0 through suffix byte 7 (256 each)
outcome: incorrect or correct (2)
```

Test symmetric ranks 1, 2, 4, and 8. Do not test higher ranks unless a later
RFC identifies a new mechanism rather than additional capacity.

## 3. Representation

Apply eleven logical cores in the mode order above. Serialize four
nonnegative unsigned-16-bit tensors: one distance core, one run core, one
shared suffix-byte core, and one outcome core. Apply the shared byte core once
for each of the eight suffix bytes. This is a recurrent MPS transition. The
hidden state therefore depends on byte order without eight independent byte
core payloads.

Boundary ranks are one. Internal ranks equal the selected symmetric rank. A
rank-1 model cannot condition the outcome on any input mode because every
input factor cancels when the two outcome scores are normalized. Retain rank
1 only as a negative control.

The parameter count is:

```text
6*r + 262*r*r unsigned 16-bit values
```

Complete core sizes are 536 bytes at rank 1, 2,120 at rank 2, 8,432 at rank 4,
and 33,632 at rank 8. Count every byte in the artifact. The artifact MUST
serialize the shared byte core once.

## 4. Deterministic contraction

Decoder state values and core values are at most 65,535. For each internal
core slice, calculate each next state as a sum of at most eight products in an
unsigned 64-bit integer. The maximum is `8 * 65535 * 65535`, which fits.

After each core, let `M` be the largest next-state value. If `M` is zero,
reject the model. Otherwise normalize every component as:

```text
state[j] = 1 + floor(next[j] * 65534 / M)
```

Use an unsigned 128-bit intermediate for the multiplication. The final outcome
core produces two positive scores with the same contraction rule.

Training MAY use floating point. The artifact and decoder MUST use only the
quantized cores and the integer contraction above.

## 5. Classical controls and gate

First measure held-out binary log loss with core metadata amortized over the
evaluated match events. Preserve full-precision, quantized-core, and integer
contraction losses for every rank.

Compare against:

1. the global adaptive reliability control;
2. the 24-cell distance/run control; and
3. the sparse exact-suffix control from RFC 0012.

Proceed to a compression artifact only if one fixed rank beats both classical
feature controls on at least three Silesia groups, does not regress in
aggregate, and its core bytes are smaller than the modeled bit saving.

The complete artifact gate then requires at least 1% size improvement over RFC
0009 on three groups, no aggregate regression, less than two times decode
time, less than one gigabyte peak RSS, and exact restoration.

## 6. Training split

Use the first 90% of each established Silesia development partition for
training and its final 10% for evaluation. Do not train on evaluation events.
Select one rank across all groups. A passing rank requires later validation on
a corpus not used for selection.

## 7. Stop conditions

Reject the tensor if full-precision loss fails; quantization cannot repair the
model. Reject the quantized design if metadata or integer contraction removes
the gain. Do not implement a codec from log loss alone.

Preserve all failed ranks. Do not claim that this design approaches M2 before
a complete enwik9 artifact is measured.

## 8. Metadata pre-screen

The first draft assigned an independent core to each suffix position. Its
smallest dependency-capable model required 16,456 bytes at rank 2. That cost
was larger than the observed sparse-suffix saving on Dickens, Samba, XML, and
OOffice. Only Mozilla saved more than the untied rank-2 core cost.

This does not prove that an untied model cannot win because a tensor can beat
the sparse suffix control. It does show that separate byte cores spend
metadata before positional specialization is demonstrated. Use the shared
transition first. A later RFC may untie byte positions only after a controlled
residual test shows a position-specific dependency that the shared transition
cannot represent.

## 9. Compatibility and security

This RFC assigns no TAK-C method identifier. Any research artifact MUST bound
rank, core count, core dimensions, contraction arithmetic, output length, and
memory before decoding. Checksums detect accidental corruption only.
