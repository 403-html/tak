# RFC 0003: Third-order context dependency gate

- Status: Rejected
- Date: 2026-11-22
- Scope: Research artifact only
- TAK-C method identifier: None

## 1. Problem statement

RFC 0002 established that the preceding two bytes improve prediction over the
preceding byte alone. Its three-core tensor train did not approach the adaptive
order-2 control. A larger bond dimension does not introduce a new dependency
and is not justified.

This RFC tests one specific longer dependency before it proposes another tensor
model: whether the byte at position `t-3` changes the distribution of byte `t`
after bytes `t-2` and `t-1` are fixed. Repeated four-byte fragments, source-code
tokens, markup sequences, and binary signatures can contain this dependency.
An order-2 model merges all histories with the same final two bytes. An order-3
model does not.

## 2. Hypothesis

The conditional distribution

```text
P(byte[t] | byte[t-3], byte[t-2], byte[t-1])
```

will produce a smaller complete artifact than adaptive order-2 on at least
three held-out dataset groups. Each qualifying reduction will be at least 2%.

This is a dependency gate. It does not authorize a four-core tensor train.
Tensor work requires an amendment that specifies how the model shares
third-order statistics, why that structure addresses the failures measured in
RFC 0002, and a separate stop condition.

## 3. Classical control

Implement a deterministic adaptive order-3 byte model. The first byte uses a
start context. The next two bytes use their available shorter histories. Later
bytes use the previous three bytes.

Each context starts with frequency one for every byte. The model increments the
observed byte frequency after coding it. It rescales at total 16384 with the
same rule as the order-2 control.

The implementation stores only frequency increments above the implicit value
one. This sparse representation MUST produce the same ranges as a dense table.
It changes memory use, not the probability model.

## 4. Research artifact

The complete artifact uses magic `TKO3`. It contains no transmitted model
parameters. All multi-byte integers use little-endian byte order.

| Offset | Size | Field | Required value |
|---:|---:|---|---|
| 0 | 4 | Magic | ASCII `TKO3` |
| 4 | 1 | Version | 0 |
| 5 | 1 | Flags | 0 |
| 6 | 2 | Reserved | 0 |
| 8 | 8 | Uncompressed length | 0 to 2^64 - 1 |
| 16 | 8 | Payload length | 0 to 2^64 - 1 |
| 24 | 4 | Checksum | CRC-32/ISO-HDLC of restored bytes |
| 28 | variable | Payload | Existing arithmetic coder with order-3 model |

A decoder MUST reject unsupported header values and inconsistent physical
lengths. It MUST enforce the caller's output limit before decoding. It MUST
produce exactly Uncompressed Length bytes and verify the checksum.

## 5. Gate and stop condition

Compare `TKO3` with the complete `TKO2` artifact on the same held-out inputs
from text, source code, structured documents, document binaries, and executable
binaries.

The dependency gate passes only if order-3:

1. is at least 2% smaller than order-2 on at least three dataset groups;
2. stays below one gigabyte of peak decoder resident memory;
3. decodes no more than two times slower than order-2; and
4. passes exact round-trip and malformed-input tests on all CI platforms.

Reject a third-order tensor proposal if this gate fails. Do not increase rank
or add a fourth core merely because the order-3 control passes. First identify
a parameter-sharing mechanism that addresses RFC 0002's measured modeling
loss.

## 6. Measurements

Record complete artifact size, encoding and decoding time, throughput, peak
resident memory, thread count, and exact round-trip status. Use one warm-up and
five measured repetitions for publishable timing. A single run is only a
screening measurement.

The development split MUST remain separate from a later held-out validation
corpus. Account for every header and decoder-required byte.

## 7. Alternatives considered

### 7.1. Increase the RFC 0002 rank

Reject. It adds capacity to the rejected second-order factorization but cannot
condition on `byte[t-3]`.

### 7.2. Implement a four-core tensor train immediately

Reject. The classical dependency has not passed its gate, and RFC 0002 shows
that dependency evidence alone does not validate a tensor factorization.

### 7.3. Use a fixed-size hashed context table

Defer. Hash collisions merge unrelated contexts and make the dependency test
harder to interpret. The sparse exact control is suitable for the current
screening input sizes.

## 8. Security and compatibility

`TKO3` is not TAK-C and has no stable method identifier. The decoder MUST treat
the artifact as untrusted input. It MUST validate all fixed fields, lengths,
payload extent, output limits, arithmetic decoding, and checksum.

The number of contexts cannot exceed the declared output length. The caller's
output limit therefore also bounds context creation. Production use would need
a separate explicit memory limit before this model could enter TAK-C.

## 9. Testing strategy

Tests MUST cover exact round trips, empty and short histories, rescaling,
sparse-versus-dense range equivalence, truncation, checksum failure, and output
limits. CI MUST run on Linux, macOS, and Windows.

## 10. Decision

Reject the gate. The adaptive order-3 artifact was 6.3% to 14.6% larger than
order-2 on five Silesia development groups. Order-3 decoding was 2.1 to 3.0
times slower in the same screening runs. Every artifact restored the input
exactly, and the largest complete experiment process used 116,120 KiB.

The extra context fragmented observations across too many independently
smoothed distributions. The implicit pseudocount cost outweighed the useful
third-order dependency, if any, under this model. Do not implement a four-core
tensor train and do not increase tensor rank from this result.

A later proposal MAY test an explicit escape or backoff mechanism as a
classical experiment. It must distinguish the value of the third-order
dependency from the cost of sparse-context initialization before proposing a
tensor representation.
