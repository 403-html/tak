# RFC 0001: Quantized bigram factorization screening experiment

- Status: Proposed
- Date: 2026-11-22
- Scope: Research artifact only
- TAK-C method identifier: None

## 1. Problem statement

TAK needs a small experiment that can falsify the simplest tensor-modeling
hypothesis before the project builds a high-order TT or MPS codec.

The current order-1 model stores one adaptive distribution for each preceding
byte. It does not transmit model parameters, but it starts every context from a
uniform distribution. A pretrained low-rank model could improve early
predictions and share statistical strength between contexts. Its parameters,
contractions, and quantization can also cost more than the saved payload.

## 2. Proposed experiment

Construct a 256 by 256 byte-bigram count matrix from a training partition.
Approximate this matrix with two nonnegative cores:

```text
score(context, symbol) = sum(k = 0 .. rank-1,
                             left[context, k] * right[k, symbol])
```

This is a two-site tensor network with bond dimension `rank`. It is also an
ordinary nonnegative matrix factorization. The project MUST call it a screening
experiment, not evidence that a higher-order TT or MPS model works.

Test ranks 1, 2, 4, and 8. Train the cores with a deterministic initialization
and a documented nonnegative-factorization update. Training MAY use floating
point because the encoder serializes the resulting cores. Decoding MUST NOT use
floating point.

## 3. Quantized decoder model

The encoder MUST quantize and serialize both cores as unsigned 16-bit integers.
The research artifact MUST include the quantized core bytes in its measured
size. The decoder MUST calculate each score with unsigned integer operations
and at least 64-bit accumulators.

For each context, the decoder MUST convert all 256 scores into nonzero integer
frequencies whose total does not exceed 16384. The RFC implementation MUST
specify the rounding and remainder rules exactly. Encoder and decoder MUST
derive identical frequencies from the serialized cores.

The first byte uses a uniform distribution. Each later byte uses the preceding
decoded byte as its context.

## 4. Artifact and accounting

The experiment MUST write a complete research artifact with a magic value that
is not `TAKC`. It MUST contain:

- an artifact version;
- the rank;
- the uncompressed length;
- both quantized cores;
- the entropy-coded payload length and payload;
- a checksum of the restored bytes; and
- every value required for independent decoding.

The reported compressed size MUST be the physical artifact size. The project
MUST NOT assign a TAK-C method identifier during this experiment.

## 5. Controls and datasets

Run the same entropy coder and block boundaries with:

- the existing adaptive order-0 model;
- the existing adaptive order-1 model;
- tensor rank 1;
- tensor ranks 2, 4, and 8; and
- a disabled-tensor path through the experiment harness.

Use versioned tuning and held-out partitions. Include text, source code, JSON or
logs, documents, and binary data. `enwik8` MAY be a development input.
`enwik9` MUST NOT be the only reported input.

## 6. Measurements

Report for each artifact:

- payload bytes, model bytes, header bytes, and total bytes;
- compression and decompression wall time and throughput;
- peak resident memory;
- factorization training time, reported separately;
- model construction or contraction time at the decoder;
- exact round-trip status; and
- CPU, OS, compiler, thread count, commit, and dataset digest.

The report MUST include the rejected ranks and failed runs.

## 7. Acceptance and stop conditions

Continue to a higher-order tensor experiment only if one tested rank:

1. reduces complete artifact size against the order-1 control on at least three
   held-out dataset groups;
2. recovers its model metadata on those groups;
3. decodes no more than two times slower than the order-1 control;
4. stays within the decoder memory target; and
5. produces identical output on Linux, Windows, and macOS.

Reject this direction if no rank meets these conditions. A result that improves
cross-entropy but increases the complete artifact size is a negative result.

## 8. Alternatives considered

### 8.1. Start with a high-order TT

This could represent longer contexts, but it adds rank selection, more cores,
more metadata, and more contractions before the basic accounting path is
validated. Reject for the first experiment.

### 8.2. Do not quantize the cores

Floating-point cores are easier to train. They make portable decoding and exact
probability normalization harder. Reject for the decoder. Training may still
use floating point before quantization.

### 8.3. Measure cross-entropy only

Cross-entropy omits framing, quantized parameters, rounding loss, and arithmetic
coder finalization. Reject as the primary metric. It MAY be a diagnostic.

### 8.4. Add the method directly to TAK-C version 0

The experiment has no evidence of value or interoperability. Reject. Use a
separate research artifact.

## 9. Compatibility and security impact

This RFC does not change TAK-C. Research artifacts MUST be treated as untrusted
input. A decoder MUST validate rank, length arithmetic, parameter counts,
payload extent, output limits, and checksum before accepting an artifact.

Ranks above the experiment's compiled limit MUST be rejected before allocation
or contraction. A decoder MUST prevent integer overflow when it multiplies
quantized core values.

## 10. Testing strategy

The implementation MUST include:

- deterministic quantization tests;
- frequency-total and nonzero-frequency properties;
- exact artifact round trips for all ranks;
- truncated and malformed artifact tests;
- output-limit tests;
- cross-platform fixtures with serialized quantized cores; and
- a test that all model and header bytes are included in reported size.

## 11. Decision

No decision has been made. Implementation and held-out measurements are
required before this RFC can be accepted or rejected.

