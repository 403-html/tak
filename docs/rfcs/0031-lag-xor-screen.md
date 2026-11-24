# RFC 0031: Reversible lag-XOR screen

- Status: Rejected at complete-artifact screening
- Date: 2026-11-24
- Scope: Research artifacts

Test whether fixed-distance byte prediction exposes structure that the
RFC 0019 encoder can use. For lag L, copy the first L bytes unchanged.
For each subsequent index i, write the original byte XOR the original
byte at index i-L. Reverse the transform in ascending index order by
XORing each residual with the already restored byte at i-L.

Freeze the candidates at lags 1, 2, 4, 8, 16, 32, 64, 128, and 256.
Encode each residual with the RFC 0019 structural streaming encoder.
Compare against an ordinary RFC 0019 artifact from the same binary.
Select the smallest complete artifact. Prefer identity on ties, then the
smallest lag. The decoder identifies identity by its existing magic.

## Wrapper

The research wrapper starts with ASCII TLX1 at offsets 0–3, followed by
the unsigned 16-bit lag at offsets 4–5 and CRC-32/ISO-HDLC of the
original bytes at offsets 6–9. Both integers are little-endian. Offset
10 starts one complete TKT9 stream extending to physical EOF. Its
declared restored length also gives the original length. The nested
decoder validates its own checksum and framing. Invert the transform,
then validate the outer checksum. Reject unsupported lags and malformed
or truncated wrapper data. The buffered screen limits original size to
8 MiB and the wrapper to 32 MiB, and passes its output limit to TKT9.
The wrapper has no TAK-C method identifier.

## Measurement

Run the frozen selector on all 18 Calgary files and all 11 Canterbury
files independently. Calgary is exploratory because its baseline gaps
informed the research direction. A digest check before Canterbury found
that its ptt5 is identical to Calgary's pic. Preserve the ptt5 measurements
for the corpus totals and exclude that duplicate from independent validation.
Preserve all trials, physical bytes, byte-for-byte verification,
wall time, and peak RSS. The selector evaluates all ten encodes. Report
search time separately from the chosen candidate's encode time.

Advance to a bounded Rust implementation and enwik9 only if the selected
artifacts save at least 0.5% on Calgary and at least 0.5% across Canterbury
binary files unique to Canterbury (kennedy.xls and sum), with exact restoration on every
trial. This one-repetition run is a screening measurement.

## Result

Calgary saved 215 complete bytes (0.0228%). Only geo selected a
transform, at lag 4. Canterbury selected identity for all files.
All 290 trials restored exactly. Both size gates failed and the
experiment ended before enwik9. See
[the experiment report](../../research/experiments/2026-11-24-rejected-lag-xor.md).
