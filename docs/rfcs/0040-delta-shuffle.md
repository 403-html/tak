# RFC 0040: Modular delta before byte shuffle

- Status: Rejected at Calgary complete-artifact gate
- Date: 2026-11-25
- Scope: Classical research artifact

## 1. Hypothesis

RFC 0038 groups bytes at the same position in fixed-width records.
Neighboring records may also contain similar values at that position.
Subtraction modulo 256 before the shuffle can concentrate those
differences. RFC 0031 tested XOR without transposition. RFC 0039 tested
raster stride XOR. This experiment uses modular subtraction and the
short widths from RFC 0038.

## 2. Transform and format

For width 2, 4, or 8, divide the original into 65,536-byte blocks and a
possible short final block. In each block, retain the first width bytes
or the entire block if shorter. At every later position i, emit
(original[i] minus original[i - width]) modulo 256. Apply the RFC 0038
byte shuffle to those differences. To decode, undo the shuffle and
reconstruct in increasing order with addition modulo 256. Reset the
predictor at every block boundary.

The wrapper uses ASCII TDS1 at offsets 0–3, the unsigned width at
offset 4, and the original CRC-32/ISO-HDLC as a little-endian unsigned
32-bit integer at offsets 5–8. One complete TK32 artifact starts at
offset 9 and extends to physical EOF. Use the same decoder state,
resource limits, checksum verification, and rejection conditions as
RFC 0038, with the inverse transform defined above. Unknown widths
must be rejected. Update neither the legacy wrapper nor TK32.

## 3. Method and gates

Compare complete artifacts against RFC 0038's best of identity and
three shuffle trials. Test the three new widths on all 18 Calgary files.
Prefer any RFC 0038 profile on ties and otherwise use the smallest
width. Charge headers and checksum bytes. Restore every candidate and
control exactly. Measure wall time and peak RSS for each operation,
and account separately for the total search and selected artifacts.

The Calgary screen requires at least 0.25% aggregate savings over
RFC 0038. If it passes, freeze the candidate and require the same
aggregate threshold and byte-for-byte restoration on independent
Canterbury, excluding ptt5. If both pass, run the four fixed enwik9
sample windows from RFC 0038 and require 0.25% over identity before
selecting a nonidentity profile for the complete file. Verify the
complete selected artifact and record time and memory. Stop at the
first failed corpus gate. This experiment introduces no TAK-C method.

## 4. Result

Every trial restores exactly. None of the 54 delta trials improves the
selected artifact for its file. Calgary remains at 893,989 bytes.
The size gate required a saving of at least 2,235 bytes. Stop before
Canterbury and enwik9.

[Measurements and reproduction](../../research/experiments/2026-11-25-delta-shuffle.md)
