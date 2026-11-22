# RFC 0004: Mature third-order context backoff

- Status: Classical gate accepted; tensor scope unchanged
- Date: 2026-11-22
- Scope: Classical research artifact only
- TAK-C method identifier: None

## 1. Problem statement

RFC 0003 rejected independent adaptive order-3 distributions. They produced
larger artifacts and slower decoding than order-2 on every tested group. Each
new three-byte context paid 256 implicit pseudocounts before it had useful
evidence. That result does not distinguish absent third-order information from
cold-start fragmentation.

This RFC tests whether mature three-byte contexts improve prediction while
rare contexts reuse the accepted order-2 control.

## 2. Hypothesis

An adaptive model that selects order-3 only after an exact three-byte context
has enough observations will produce a complete artifact at least 2% smaller
than order-2 on at least three Silesia development groups.

Test minimum observation thresholds 2, 4, 8, 16, 32, and 64. These fixed
values are development parameters. A later held-out corpus must validate any
selected threshold.

## 3. Model

The encoder and decoder maintain an adaptive order-2 model and sparse order-3
distributions. Both models update after every byte.

The component distributions use the pseudocount, update, and rescaling rules
from RFCs 0002 and 0003. The order-3 observation total is the current sum of
stored frequency increments above the 256 implicit pseudocounts.

For each byte after the first three bytes:

1. Find the distribution for the exact preceding three bytes.
2. Use that order-3 distribution if its observation total is at least the
   artifact threshold.
3. Otherwise, use the order-2 distribution for the final two bytes.
4. Code the byte with the selected distribution.
5. Update both models with the exact byte.

The selector uses only prior decoded bytes and model state. It sends no choice
bit. The decoder MUST make the same selection as the encoder.

This model is a hard backoff control. It is not PPM and does not use exclusion
or an escape symbol.

## 4. Research artifact

The complete artifact uses magic `TKB3`. All multi-byte integers use
little-endian byte order.

| Offset | Size | Field | Required value |
|---:|---:|---|---|
| 0 | 4 | Magic | ASCII `TKB3` |
| 4 | 1 | Version | 0 |
| 5 | 1 | Flags | 0 |
| 6 | 2 | Minimum observations | 1 to 65535 |
| 8 | 8 | Uncompressed length | 0 to 2^64 - 1 |
| 16 | 8 | Payload length | 0 to 2^64 - 1 |
| 24 | 4 | Checksum | CRC-32/ISO-HDLC of restored bytes |
| 28 | variable | Payload | Arithmetic-coded bytes |

A decoder MUST reject zero thresholds, unsupported fixed fields, inconsistent
physical lengths, invalid arithmetic payloads, output-limit violations, and
checksum failures.

## 5. Gate and stop condition

Compare every threshold with complete `TKO2` artifacts on the same final-10%
Silesia partitions used by RFC 0003.

The gate passes only if one fixed threshold:

1. is at least 2% smaller than order-2 on at least three dataset groups;
2. stays below one gigabyte of peak decoder resident memory;
3. decodes no more than two times slower than order-2; and
4. passes exact round-trip and malformed-input tests on all CI platforms.

Reject the hard-backoff model if no threshold passes. Do not select a different
threshold per file. Do not use this experiment to authorize tensor work.

## 6. Measurements

Report all thresholds, including dominated results. Record complete artifact
size, encoding and decoding time, throughput, peak resident memory, thread
count, and round-trip status. Label single-run results as screening data.

## 7. Security and compatibility

`TKB3` is not TAK-C. It has no stable codec identifier. The caller's output
limit bounds the number of contexts, but production use would require a
separate memory limit.

Checksums detect accidental corruption. They do not authenticate an artifact.

## 8. Tensor scope

This RFC specifies no tensor model. Passing the gate would show only that
mature third-order contexts add practical value. A tensor proposal would still
need a new RFC with a specific parameter-sharing mechanism that addresses RFC
0002's factorization loss.

## 9. Decision

Threshold 64 passed the size hypothesis on three development groups. It made
the complete artifact 3.7% smaller on source code, 2.2% smaller on structured
documents, and 2.5% smaller on executable data. It was 1.6% larger on English
text and 0.2% larger on office data.

The initial implementation failed the speed condition. Its sparse decoder
scanned all 256 symbols even though most contexts stored four or fewer distinct
continuations. An exact gap decoder now iterates only the stored entries and
computes ranges for implicit unit-frequency symbols directly. It does not
change any probability or artifact byte.

Accept the classical gate after this optimization. After one warm-up and five
measured repetitions, median decoding was 1.30 to 1.78 times slower than
order-2. Peak complete-process memory was at most 130,872 KiB. Every artifact
restored its input exactly.

The result is development-corpus evidence that mature third-order contexts
carry useful information on three domains. It does not authorize a tensor
model or a TAK-C method identifier. Threshold 64 still requires validation on
an independent held-out corpus.

A two-level direct context table was also rejected. It preserved artifact
bytes but increased peak memory to as much as 231,852 KiB and decoded more
slowly than the deterministic hash map. Retain the hash map and gap decoder as
the current Pareto point. Further changes require a new measured bottleneck.

A row-index cache and four-entry inline storage were also rejected. The cache
added a random vector access and did not meet the speed limit. Inline storage
increased row size and peak memory without a consistent speed benefit.
