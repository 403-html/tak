# RFC 0005: Hierarchical order-2 smoothing

- Status: Rejected
- Date: 2026-11-22
- Scope: Classical research artifact only
- TAK-C method identifier: None

## 1. Problem statement

RFC 0004 failed held-out validation because independent higher-order rows did
not generalize. The retained adaptive order-2 control has a related cold-start
cost. A new two-byte context starts with 256 equal pseudocounts. It ignores the
already learned distribution for the final one-byte context.

This RFC tests whether an order-1 parent distribution can supply a useful
prior while an exact order-2 context accumulates local evidence.

## 2. Hypothesis

A fixed hierarchical weight will make complete artifacts at least 2% smaller
than the order-2 control on at least five of eight development groups. At
least two passing groups MUST come from Canterbury.

Test parent weights 1, 2, 4, 8, 16, 32, and 64. Select one weight across all
files. These weights and all current corpus results are development data. A
later untouched corpus must validate a selected model.

## 3. Model

The encoder and decoder maintain 257 adaptive order-1 parent distributions.
They also maintain sparse local increments for each observed two-byte context.

For symbol `s`, parent weight `w`, parent frequency `P(s)`, and local
increment `L(s)`, the coding frequency is:

```text
F(s) = w * P(s) + L(s)
```

The cumulative range uses the same sum for every symbol less than `s`. The
total is `w * P_total + L_total`. The first byte uses the start parent. The
second byte uses the parent for the first byte. They have no local increment.

After coding a byte, the model MUST update the selected order-1 parent. If two
preceding bytes exist, it MUST also update their local row. Both sides MUST
update in the same order.

Each parent starts with frequency one for every byte. It uses the adaptive
update and halving rule from the existing order-1 model. A local row starts
empty. An observed symbol adds one local increment. Before an update, the row
halves all increments when `256 + L_total` is at least 16384. It removes zero
increments after halving.

The decoder uses integer arithmetic only. For the largest screened weight,
the total is less than `64 * 16384 + 16128`, which fits in 32 bits. The
arithmetic coder's intermediate product fits in 64 bits.

This model is hierarchical additive smoothing. It is not PPM. It sends no
escape symbol or model-choice bit.

## 4. Research artifact

The complete artifact uses magic `TKS2`. All multi-byte integers use
little-endian byte order.

| Offset | Size | Field | Required value |
|---:|---:|---|---|
| 0 | 4 | Magic | ASCII `TKS2` |
| 4 | 1 | Version | 0 |
| 5 | 1 | Flags | 0 |
| 6 | 2 | Parent weight | 1, 2, 4, 8, 16, 32, or 64 |
| 8 | 8 | Uncompressed length | 0 to 2^64 - 1 |
| 16 | 8 | Payload length | 0 to 2^64 - 1 |
| 24 | 4 | Checksum | CRC-32/ISO-HDLC of restored bytes |
| 28 | variable | Payload | Arithmetic-coded bytes |

A decoder MUST reject an unsupported weight, unsupported fixed field,
inconsistent physical length, invalid arithmetic payload, output-limit
violation, or checksum failure.

## 5. Development data and accounting

Use the five final-10% Silesia groups from RFC 0003 and the three Canterbury
groups from RFC 0004. Compress each source file independently. A group size is
the sum of complete artifacts, including every 28-byte header.

Canterbury is not held out for this experiment. RFC 0004 exposed its results
before this mechanism was proposed. Do not describe a pass on Canterbury as
validation.

## 6. Gate and stop condition

Compare each fixed weight with complete `TKO2` artifacts on the same inputs.
The gate passes only if one fixed weight:

1. is at least 2% smaller than order-2 on at least five of eight groups;
2. passes that size condition on at least two Canterbury groups;
3. is not larger than order-2 on the aggregate input;
4. stays below one gigabyte of peak decoder resident memory;
5. decodes no more than two times slower than order-2; and
6. passes exact round-trip and malformed-input tests on all CI platforms.

Reject this mechanism if no fixed weight passes. Do not select a different
weight per file. Do not change the weight set after measurement. Do not use a
passing result to authorize tensor work.

## 7. Measurements

Report every weight, including dominated results. Record complete artifact
size, encoding and decoding time, throughput, peak resident memory, thread
count, and round-trip status. Label single-run results as screening data.

If the gate passes, rerun the selected weight after one warm-up for at least
five measured repetitions. Validate it later on a corpus not used by RFCs
0001 through 0005.

## 8. Security and compatibility

`TKS2` is not TAK-C. It has no stable codec identifier. The caller's output
limit bounds decoded bytes. Production use would also require a separate
memory limit for context rows.

Checksums detect accidental corruption. They do not authenticate an artifact.

## 9. Tensor scope

This RFC specifies a classical control. It tests parameter sharing between a
two-byte context and its one-byte suffix. It does not address the rejected
bigram factorization or justify a higher tensor rank.

A future tensor attempt still requires a new RFC that names a longer-context
dependency and a specific representation that this classical model cannot
express.

## 10. Decision

Reject hierarchical additive smoothing. No screened weight passed the size
gate. Weight 1 was the best aggregate candidate, but it was larger than
order-2 on all five Silesia groups. It was 4.9% larger across all development
inputs and decoded 2.15 times slower in the single-run screen.

Weight 1 made Canterbury prose 4.2% smaller and source/markup 17.3% smaller.
It made Canterbury binary data 3.0% larger. These two improvements cannot
satisfy the required five-of-eight group condition.

The raw parent counts dominated local increments. Even weight 1 assigned the
complete accumulated order-1 frequency to every prediction. Larger weights
increased this imbalance and produced monotonically larger aggregate
artifacts.

Do not tune more raw-count weights. A later proposal may test a normalized
finite-strength parent prior, but it requires a new RFC and gate. This result
does not authorize tensor work.

