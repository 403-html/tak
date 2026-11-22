# RFC 0002: Second-order byte-context tensor train

- Status: Phase A passed; Phase B authorized
- Date: 2026-11-22
- Scope: Research artifact only
- TAK-C method identifier: None

## 1. Problem statement

RFC 0001 modeled `P(next byte | previous byte)` with a two-core factorization.
It failed because the factorized bigram model predicted worse than the adaptive
order-1 control. Increasing its bond dimension cannot add context. It only
improves the representation of the same bigram dependency.

Many byte sequences need the two preceding bytes. The previous byte can have a
different continuation after different earlier bytes. Examples include letter
pairs in natural language, two-character source-code operators, escape
sequences, markup delimiters, and binary field signatures. An order-1 model
merges these cases into one distribution.

## 2. Hypothesis

The conditional distribution

```text
P(next byte | byte[t-2], byte[t-1])
```

contains useful dependencies that the rejected bigram factorization cannot
represent. A three-core tensor train might share these second-order statistics
with less metadata than a full 256 by 256 by 256 table.

This RFC does not assume that the dependency is large enough to justify a
tensor model. A classical order-2 control MUST pass the gate in Section 4 before
the tensor implementation begins.

## 3. Phase A: classical dependency control

Implement a deterministic adaptive order-2 model. It uses separate 256-symbol
distributions for:

- the start state;
- each one-byte context for the second byte; and
- each two-byte context for later bytes.

Every distribution starts with frequency one for every symbol. It uses the
same update and rescaling rules as the existing order-0 and order-1 controls.
The implementation MAY allocate two-byte distributions lazily.

The control MUST use the existing arithmetic coder. It MUST be stored in a
complete research artifact with magic `TKO2`. The artifact MUST contain its
version, output length, payload length, checksum, and payload. It MUST NOT use a
TAK-C method identifier.

All multi-byte integers use little-endian byte order. The artifact fields are:

| Offset | Size | Field | Required value |
|---:|---:|---|---|
| 0 | 4 | Magic | ASCII `TKO2` |
| 4 | 1 | Version | 0 |
| 5 | 1 | Flags | 0 |
| 6 | 2 | Reserved | 0 |
| 8 | 8 | Uncompressed length | 0 to 2^64 - 1 |
| 16 | 8 | Payload length | 0 to 2^64 - 1 |
| 24 | 4 | Checksum | CRC-32/ISO-HDLC of restored bytes |
| 28 | variable | Payload | Existing arithmetic coder with order-2 model |

A decoder MUST reject unsupported header values and inconsistent physical
lengths. It MUST enforce its output limit before decoding. It MUST produce
exactly Uncompressed Length bytes and verify the checksum.

## 4. Gate for tensor implementation

Run order-1 and order-2 controls on the same held-out partitions from text,
source, structured data, documents, and binary data.

Proceed to Phase B only if order-2:

1. produces a smaller complete artifact than order-1 on at least three dataset
   groups;
2. improves complete size by at least 2% on each of those groups;
3. remains within the one-gigabyte decoder memory target;
4. decodes no more than two times slower than order-1; and
5. passes exact round-trip and malformed-input tests on all CI platforms.

Reject this RFC if the control fails the gate. More tensor rank cannot create a
dependency that is absent from the data.

## 5. Phase B: proposed tensor train

If Phase A passes, approximate the conditional score tensor with three
nonnegative cores:

```text
score(a, b, s) = sum(i = 0 .. r1-1,
                     sum(j = 0 .. r2-1,
                         G0[a, i] * G1[i, b, j] * G2[j, s]))
```

Here `a` and `b` are the two preceding bytes, `s` is the candidate next byte,
and `r1` and `r2` are bond dimensions. This structure can produce a different
distribution for the same `b` when `a` changes. RFC 0001 cannot do that.

Test symmetric ranks 1, 2, 4, and 8. The serialized 16-bit core cost is:

```text
2 * (256*r + r*256*r + r*256) bytes
```

The first byte uses a uniform distribution. The second byte MUST use a defined
one-byte fallback model. The encoder MUST include all fallback parameters in
the artifact size.

## 6. Numerical requirements

Training MAY use floating point. The artifact MUST serialize quantized integer
cores. Decoding MUST use only specified fixed-width integer operations.

The design MUST prove accumulator bounds for rank 8 before implementation. It
MUST define score normalization, rounding, nonzero frequencies, and maximum
frequency total. Encoder and decoder MUST derive identical cumulative ranges
from the serialized cores.

## 7. Measurements and ablations

Phase B MUST report:

- adaptive order-1 and order-2 complete artifact sizes;
- tensor ranks 1, 2, 4, and 8;
- a disabled-tensor path through the same harness;
- full-precision, quantized-core, and integer-frequency cross-entropy;
- header, core, fallback-model, payload, and total bytes;
- training, encoding, and decoding time;
- compression and decompression throughput;
- peak resident memory; and
- exact round-trip status.

Use the dataset and environment records required by `AGENTS.md`. Preserve every
rank, including failed or dominated results.

## 8. Alternatives considered

### 8.1. Increase RFC 0001 rank

Reject. It cannot condition on the second preceding byte. Measurements also
show non-monotonic rank behavior on binary inputs.

### 8.2. Use a full static trigram table

This is a useful upper bound but requires up to 16,777,216 values before
compression. It MAY be measured as a diagnostic. It is not the proposed
portable model.

### 8.3. Add an unrestricted context mixer

This could be a stronger compressor, but it would not test the specific tensor
hypothesis. Keep it as an external comparison.

### 8.4. Move directly to longer contexts

Reject for this experiment. Second-order context is the smallest dependency
that RFC 0001 cannot represent.

## 9. Compatibility and security

This RFC does not change TAK-C. Both `TKO2` and any future tensor artifact are
research formats.

Decoders MUST validate magic, version, flags, lengths, ranks, parameter counts,
payload extent, output limits, arithmetic bounds, and checksum. They MUST reject
unsupported values before large allocation or contraction.

## 10. Testing strategy

Phase A MUST include:

- exact order-2 round trips;
- first-byte and second-byte boundary cases;
- deterministic rescaling tests;
- truncation, corruption, and output-limit tests;
- bounded lazy-allocation behavior; and
- Linux, Windows, and macOS CI.

Phase B requires a separate implementation update and cross-platform fixtures.

## 11. Decision

Phase A passed on `enwik8` and five Silesia files. Adaptive order-2 reduced
complete artifact size on every input by 6.4% to 16.2% relative to order-1.
Order-2 decoding took 1.05 to 1.31 times the order-1 time. The complete
experiment process, including a 100 MB source buffer and both controls, peaked
at 155,492 KiB. All artifacts passed exact round-trip verification.

Authorize Phase B. This decision authorizes only the three-core dependency in
Section 5, ranks 1, 2, 4, and 8, and the stated ablations. It does not authorize
a TAK-C method identifier or a general increase in tensor complexity.

