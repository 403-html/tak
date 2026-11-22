# TAK-EXP-001: Experimental Compression Stream Version 0

## 1. Status

This document defines an experimental format. It is not a stable TAK format.
An implementation MUST NOT use version 0 for durable storage. A future format
MAY replace all fields and decoding rules in this document.

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** in
this document are to be interpreted as described in BCP 14 when, and only when,
they appear in bold capitals.

## 2. Conventions

A byte is eight bits. The first byte has offset zero. Multi-byte integers use
little-endian byte order. The arithmetic-coded payload writes the most
significant bit of each byte first.

## 3. Frame

Version 0 contains one frame and one payload.

```text
  0                   1                   2                   3
  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                         Magic "TAKC"                          |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 | Version = 0   |    Method     |          Flags = 1            |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                    Uncompressed Length (64)                   |
 +                                                               +
 |                                                               |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                       Payload Length (64)                     |
 +                                                               +
 |                                                               |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                    CRC-32/ISO-HDLC (32)                       |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
 |                         Payload ...                           |
 +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

| Offset | Size | Field | Required value |
|---:|---:|---|---|
| 0 | 4 | Magic | ASCII `TAKC` |
| 4 | 1 | Version | 0 |
| 5 | 1 | Method | Section 4 |
| 6 | 2 | Flags | 1 |
| 8 | 8 | Uncompressed Length | 0 to 2^64 - 1 |
| 16 | 8 | Payload Length | 0 to 2^64 - 1 |
| 24 | 4 | Checksum | CRC-32/ISO-HDLC of the uncompressed bytes |
| 28 | variable | Payload | Exactly Payload Length bytes |

A decoder **MUST** reject an unsupported version, method, or flag value. A
decoder **MUST** reject a frame whose physical size is not exactly 28 plus
Payload Length. A decoder **MUST** check its configured output limit before it
allocates storage for Uncompressed Length.

## 4. Methods

### 4.1. Method 0: Raw

The payload is the uncompressed byte sequence. Payload Length **MUST** equal
Uncompressed Length.

### 4.2. Method 1: Adaptive order-0

The payload uses the arithmetic coder in Section 5. One distribution contains
256 frequencies. Each frequency starts at 1. The encoder and decoder update the
distribution after each byte.

### 4.3. Method 2: Adaptive order-1

The payload uses the arithmetic coder in Section 5. There are 257 independent
distributions. Context 256 codes the first byte. After a byte is coded, its
unsigned value selects the distribution for the next byte.

## 5. Model update

Each distribution has 256 unsigned integer frequencies and a total. Before an
update, if the total is at least 16384, replace every frequency `f` with
`ceil(f / 2)` and recompute the total. Then increment the frequency of the
coded byte and the total by one.

## 6. Arithmetic decoding

The coder uses inclusive 32-bit intervals represented with wider integer
intermediates. Define `MAX = 2^32 - 1`, `HALF = 2^31`, `Q1 = 2^30`, and
`Q3 = 3 * 2^30`. Set `low = 0` and `high = MAX`. Read 32 payload bits into
`code`. Bits beyond Payload Length are zero for arithmetic finalization only.

For each of Uncompressed Length output bytes:

```text
width  = high - low + 1
scaled = floor(((code - low + 1) * total - 1) / width)
symbol = the unique symbol where cum_low <= scaled < cum_high
high   = low + floor(width * cum_high / total) - 1
low    = low + floor(width * cum_low  / total)
```

Renormalize repeatedly:

1. If `high < HALF`, do not subtract an offset.
2. Else, if `low >= HALF`, subtract `HALF` from `low`, `high`, and `code`.
3. Else, if `low >= Q1` and `high < Q3`, subtract `Q1` from all three values.
4. Else, stop renormalization.
5. Shift `low` left by one. Shift `high` left and set its low bit. Shift `code`
   left and append the next payload bit.

Append the decoded symbol. Apply the model update in Section 5. Continue until
the decoder has produced Uncompressed Length bytes.

All multiplication in these operations **MUST** use at least 64 unsigned bits.
A decoder **MUST** reject an invalid interval instead of wrapping an integer.

## 7. Integrity and completion

After decoding, the decoder **MUST** compute CRC-32/ISO-HDLC over all output
bytes. The polynomial in reflected form is `0xEDB88320`. The initial value and
final XOR value are `0xFFFFFFFF`. A decoder **MUST** reject a checksum mismatch.

The checksum detects accidental corruption. It does not authenticate a frame.

## 8. Resource limits

An order-1 decoder uses 257 by 256 16-bit frequencies plus implementation
state. Its model working set is approximately 132 KiB. The output allocation is
separate. Implementations **SHOULD** support streaming output in a later format
revision. Version 0 currently decodes one complete frame in memory.

