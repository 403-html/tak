# RFC 0039: Packed raster row prediction

- Status: Rejected at Calgary complete-artifact gate
- Date: 2026-11-25
- Scope: Classical research artifact

## 1. Hypothesis

Calgary pic remains close to 10,000 bytes above xz -9 after RFC 0038.
In a packed monochrome raster, a byte often resembles the byte one row
above it. XOR at the row stride may produce zero runs that TK32 can
compress. RFC 0031 tested power-of-two lags and did not test a stride of
216 bytes for a 1,728-pixel fax row.

## 2. Transform and format

Test row strides of 216, 256, and 304 bytes, corresponding to standard
1,728-, 2,048-, and 2,432-pixel fax widths. Split the input into
65,536-byte blocks and a possible final short block. Within each block,
copy the first stride bytes unchanged. At index i greater than or equal
to the stride, emit original[i] XOR original[i - stride]. Reverse in
ascending order using the already restored byte at i - stride. Reset at
each block. No padding or length change occurs.

The wrapper starts with ASCII TRX1 at offsets 0–3. Offsets 4–5 hold the
stride as an unsigned little-endian integer. Offsets 6–9 hold CRC-32/
ISO-HDLC of the original bytes as an unsigned little-endian integer.
Offset 10 begins one complete TK32 stream extending to physical EOF.
The inner stream supplies the output length and checks its transformed
bytes. Apply the inverse transform and check the original checksum
before writing the result. Use RFC 0038 file and screen resource limits.
Reject an unsupported stride, unknown magic, malformed header or stream,
checksum failure, or output beyond the caller's limit. Existing TK32 and
TBS1 artifacts retain their semantics. TRX1 has no TAK-C identifier.

## 3. Measurement and gates

Run all three strides on all 18 Calgary files independently. Select
the smallest complete stream among RFC 0038 identity, its three byte
shuffles, and the three row strides. Prefer the RFC 0038 selection on
ties and otherwise the smaller stride. Compare all sizes under the
same independent-file accounting. Record cumulative search time and
peak RSS separately from the cost of the selected artifacts. Verify
exact restoration for every trial.

Advance if the extended selection saves at least 0.25% relative to
RFC 0038 on Calgary. For held-out validation use the first frame of
fax2d.tif and g3test.tif from the libtiff pics-3.8.0 corpus. Unpack
the TIFF to its contiguous packed monochrome payload before all
encoders. Admit the hypothesis if extended selection saves at least
0.25% on the held-out aggregate and at least one raster selects a
row stride. Verify every output before comparing sizes. Stop at
the first failed gate.

If both pass, apply the same four fixed enwik9 windows in RFC 0038
to select one full-file profile. A nonidentity profile must save
at least 0.25% over identity across those samples. Confirm the
complete selected stream with size, timing, RSS, and exact comparison.

## 4. Result

The expanded selection saves 1,608 of 893,989 bytes (0.180%), solely
on pic with stride 216. All 126 artifacts restore byte for byte.
The required reduction was 2,235 bytes. The work stops before the
held-out fax images and enwik9.

[Experiment report](../../research/experiments/2026-11-25-row-xor.md)
