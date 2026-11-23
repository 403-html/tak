# RFC 0007: Byte-identical streaming path

- Status: Accepted
- Date: 2026-11-23
- Scope: Research artifact implementation
- Format change: None

## 1. Problem statement

The RFC 0006 enwik9 runner peaked at 3,665,840 KiB because it retained the
complete input, compressed artifacts, and restored output. TAK requires
streaming operation and bounded memory. Block resets would bound memory but
would change probabilities and compressed bytes.

## 2. Proposal

Add streaming arithmetic-coder APIs that preserve one continuous model and
coder state. Read and write bounded chunks. Do not reset at I/O boundaries.

Add a streaming `TKN2` artifact path. The encoder MUST write a placeholder
header, stream the payload, then seek back to write the final input length,
payload length, and CRC-32/ISO-HDLC. The decoder MUST validate the physical
artifact length before decoding and MUST enforce the caller's output limit.

The artifact layout, probability operations, termination bits, padding bits,
and checksum coverage MUST remain identical to RFC 0006.

## 3. Error and integrity behavior

Streaming APIs MUST report I/O errors without panicking. A decoder MUST reject
invalid fixed fields, unsupported strengths, inconsistent lengths, invalid
arithmetic state, output-limit violations, and checksum failures.

The checksum state MUST update incrementally over the exact uncompressed byte
sequence. It detects accidental corruption and does not authenticate data.

## 4. Gate

Accept this RFC only if:

1. streaming and buffered encoders produce byte-identical artifacts for empty,
   short, structured, binary, and rescaling inputs;
2. buffered and streaming decoders accept both outputs and restore exact bytes;
3. malformed, truncated, excessive-output, and checksum tests pass;
4. memory use does not grow with input or output length except for model state
   and fixed I/O buffers; and
5. the streaming enwik9 path uses less than one gigabyte peak process RSS.

Report throughput and complete artifact size with memory. Reject any design
that meets memory only by resetting the model or changing artifact bytes.

## 5. Compatibility

This RFC assigns no TAK-C identifier and changes no research artifact version.
Existing `TKN2` artifacts MUST remain readable. Buffered APIs MAY remain as
test and research conveniences.

## 6. Non-goals

This RFC does not improve compression ratio, add random access, define archive
streaming, or authorize a tensor model. It removes an implementation resource
failure before further compression research.

## 7. Result

The implementation passed the gate on 2026-11-23. Streaming and buffered
encoders produced identical artifacts in boundary, structured, binary, and
rescaling tests. Cross-path decoding restored exact bytes. Limit, truncation,
and checksum rejection tests passed.

A single-sample enwik9 screen produced the same 368,643,002-byte complete size
as RFC 0006. Encoding took 119.869 seconds at 7.96 MiB/s. Decoding took 217.710
seconds at 4.38 MiB/s. Both processes peaked at 13,216 KiB RSS. The restored
output matched the 1,000,000,000-byte input byte for byte and had SHA-256
`159b85351e5f76e60cbe32e04c677847a9ecba3adc79addab6f4c6c7aa3744bc`.

This is non-publishable screening evidence from one sample without a warm-up.
It accepts the engineering design, not the compression model. The artifact
still exceeds M1 by 46,643,002 bytes and must remove 12.65% of its current
bytes to reach that target. Decode speed also remains far below the project
target.
