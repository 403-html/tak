# RFC 0041: Shuffle bit planes before TK32

- Status: Rejected at Calgary complete-artifact gate
- Date: 2026-11-25
- Scope: Classical research artifact

## 1. Hypothesis

A byte can combine fields with different statistics. RFC 0035 measured
a gain for a four-bit predictor on geo. RFC 0038 measured a gain from
grouping bytes on geo and kennedy.xls. Grouping bits of the same
significance may expose repetitions that byte grouping leaves mixed.

## 2. Transform and format

Divide the input into 65,536-byte blocks, with a shorter final block
when needed. For a block of length n let g = floor(n / 8). Each group
of eight input bytes contributes one byte to each of eight planes.
For plane p, from 0 through 7, and group i, from 0 through g-1, write
output[p*g + i]. Its bit j is bit p of input[8*i + j], where j runs
from 0 through 7. Bit 0 is the least significant bit. Copy the n%8
remaining bytes unchanged after the planes. The inverse reads the
same coordinates and restores the original bytes.

The wrapper uses ASCII TBP1 at offsets 0–3. Offset 4 contains the
fixed byte 1. Offsets 5–8 hold the original CRC-32/ISO-HDLC as a
little-endian unsigned 32-bit integer. One complete TK32 stream starts
at offset 9 and extends to physical EOF. Its declared decoded size
gives the original size. Use RFC 0038 file and screen limits, and its
checksum, decoder validation, and error handling. Reject a value
other than 1 at offset 4. Existing wrappers retain their semantics.
This experiment assigns no TAK-C method identifier.

## 3. Method and gates

On each of the 18 independent Calgary files, run all four RFC 0038
profiles and the bit-plane profile. Select by complete artifact size,
including all headers. Prefer the existing selection on ties. Restore
every output byte for byte. Record artifact sizes and digests, wall
time, throughput, and peak RSS. Report total encode search cost
separately from selected artifact encode and decode costs.

Require at least 0.25% aggregate savings over the RFC 0038 selection
to advance. Then run the same profiles on independent Canterbury,
excluding ptt5, and require at least 0.25% aggregate savings and exact
restoration. If both pass, compare the profiles on the four fixed
enwik9 windows of RFC 0038. Require at least 0.25% over identity to
select a nonidentity profile. Verify and measure the complete selected
enwik9 artifact. Stop at the first failed corpus gate.

## 4. Result

All 90 trials restore exactly. The bit-plane stream is larger than
the selected RFC 0038 stream on every Calgary file. The selection
stays at 893,989 bytes and misses the required 2,235-byte reduction.
Stop before Canterbury and enwik9.

[Experiment report](../../research/experiments/2026-11-25-bit-plane-shuffle.md)
