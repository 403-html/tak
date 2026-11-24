# RFC 0019: Structural LZ block types

- Status: Accepted; Calgary and bounded enwik9 validation complete
- Date: 2026-11-24
- Scope: Classical LZ block-type coding
- TAK-C method identifier: None

## 1. Problem statement

RFC 0017 codes every LZ block with one global adaptive distribution over
literal, match, and end symbols. The parser never emits two adjacent literal
blocks. The artifact header also declares the exact restored length. A decoder
therefore knows two structural facts:

1. A block after a literal block MUST be a match while output remains.
2. Decoding MUST stop when the declared output length is reached.

The current type alphabet spends probability on impossible symbols. This RFC
tests whether removing those symbols produces a transferable complete-stream
benefit.

## 2. Hypothesis

A structural type model will reduce ideal type cost by at least 0.01 bits per
input byte on the five Silesia development tails. It MUST improve every file.
It MUST then improve at least 15 of 18 Calgary files and save at least 0.008
bits per input byte in aggregate without parameter changes.

This opportunity gate uses the frozen RFC 0016 parse and RFC 0017 artifact as
the control. It changes no literal, length, distance, or byte model.

## 3. Structural model

At the start of the stream and after a match block, code one binary symbol:

```text
0 = literal block
1 = match block
```

After a literal block, emit no type symbol. The next block is a match. Emit no
end symbol. Stop when the restored byte count equals the header's uncompressed
length.

The binary distribution starts with unit frequencies and uses the existing
adaptive update and halving rules. Only explicit binary decisions update it.

### 3.1 Research artifact

The artifact uses ASCII magic `TKT9`. It is not TAK-C and has no stable method
identifier. The remaining 24-byte header is identical to RFC 0017:

| Offset | Size | Field | Required value |
|---:|---:|---|---|
| 0 | 4 | Magic | ASCII `TKT9` |
| 4 | 4 | Version and flags | Zero |
| 8 | 8 | Uncompressed length | Little-endian unsigned integer |
| 16 | 8 | Payload length | Little-endian unsigned integer |
| 24 | 4 | Checksum | CRC-32/ISO-HDLC of restored bytes |
| 28 | variable | Payload | Arithmetic-coded blocks |

Literal lengths, match lengths, distances, literal bytes, arithmetic bit
ordering, and checksum coverage are unchanged from RFC 0017. The only syntax
change is the structural type model in Section 3.

## 4. Opportunity accounting

The control cost is the ideal arithmetic cost of the current adaptive
three-symbol distribution, including its end symbol. The candidate cost is
the ideal cost of the binary decisions in Section 3. Implicit match types and
the structural end consume zero bits.

The diagnostic counts no estimated metadata because the state and declared
output length already exist in both encoder and decoder. It is still an ideal
cost result, not a compressed-size result.

## 5. Complete-artifact gate

Implement a distinct research artifact only if Section 2 passes. Freeze the
opportunity method before implementation. The Silesia complete-artifact gate
passes only if:

- every artifact restores the exact input;
- all five artifacts are smaller than RFC 0017;
- aggregate size decreases by at least 0.008 bits per input byte;
- median decode time is at most 1.10 times RFC 0017; and
- encoder and decoder peak RSS stay below one gigabyte.

Use one warm-up and five measured repetitions for publishable timing. If the
development gate passes, freeze the implementation and validate all 18
Calgary files. Calgary MUST improve at least 15 files and save at least 2,500
bytes in aggregate. Every output MUST restore exactly. Median decode time MUST
remain within 1.10 times RFC 0017.

Only a passing Calgary artifact permits bounded implementation and enwik9
validation. Do not tune the type model on Calgary or enwik9.

## 5.1 Opportunity results

The frozen opportunity model passes both gates.

| Corpus | Input bytes | Current type bits | Structural type bits | Saved bits/input byte | Files improved |
|---|---:|---:|---:|---:|---:|
| Silesia development | 9,451,681 | 373,061.062 | 247,080.807 | 0.01333 | 5/5 |
| Calgary validation | 3,251,493 | 116,718.272 | 74,601.093 | 0.01295 | 18/18 |

The Silesia ideal bound is 125,980 bits, or approximately 15,748 bytes. The
Calgary bound is 42,117 bits, or approximately 5,265 bytes. These values do
not include arithmetic-coder rounding interactions and are not artifact-size
claims.

Raw rows are in
`benchmarks/reports/raw/2026-11-24-structural-block-type-opportunity.tsv`.
Its SHA-256 is
`d285d95aa63786a9b89bd5ffb633ddc2ce139ddb6357529db6baff12b853fe58`.

## 5.2 Complete artifact and bounded validation

The complete artifact passes every frozen gate. It reduces the Silesia
development aggregate from 2,696,546 to 2,680,799 bytes and improves all five
files. It reduces Calgary from 947,716 to 942,454 bytes and improves all 18
files. Every decode restores the exact input. Controlled median decode ratios
stay within 1.10. Peak screened process RSS is 145,028 KiB.

The bounded implementation produces byte-identical artifacts and restores
with a 64 MiB match window. On enwik9 it produces 249,762,881 bytes, which is
1,935,499 bytes smaller than RFC 0017. Exact SHA-256 restoration passes. See
`research/experiments/2026-11-24-structural-lz-block-types.md`.

## 6. Decoder requirements

The decoder MUST reject a literal type after a literal block. It MUST reject a
block that exceeds the declared output length. It MUST stop only at the exact
declared length. It MUST preserve all existing distance, checksum, extent,
output-limit, and arithmetic rejection rules.

Empty input has no type symbols. A final literal block needs no following
match. The decoder distinguishes this case because the final literal reaches
the declared output length.

## 7. Tensor scope

This RFC does not authorize a tensor model. The dependency is one bit of
parser state and has a sufficient two-frequency classical representation. A
tensor would add no useful parameter sharing.

A later tensor proposal still requires a different measured longer-context
dependency that the rejected bigram factorization and this state machine
cannot represent. Increasing tensor rank remains unjustified.

## 8. Stop conditions

If the opportunity gate fails, reject structural type coding and do not add
previous-length, distance, or multi-block contexts on these corpora. If the
opportunity passes but the complete artifact fails, record the accounting gap
and stop before Calgary.

Do not report ideal saved bits as artifact bytes.
