# RFC 0038: Fixed-width byte shuffle

- Status: Measured research wrapper
- Date: 2026-11-25
- Scope: Classical research artifact

## 1. Hypothesis

RFC 0035 found a strong bit-group prediction signal on Calgary geo.
Bytes at the same offset within a multi-byte field may also share a
distribution. A byte shuffle groups those positions before compression
and preserves order inside each group. RFC 0031 instead used XOR between
bytes at a fixed lag and kept their original positions.

## 2. Transform

For each 65,536-byte block, and for the final short block, use a width
of 2, 4, or 8. Concatenate the byte sequences at positions congruent to
0, 1, and so on modulo width, in that order. Keep positions in ascending
order inside each sequence. In a block of length n, sequence r has
`(n + width - 1 - r) // width` bytes. Restore by distributing each
sequence to those same positions. Blocks retain their original lengths.

For example, width 2 transforms ASCII `abcde` to `acebd`.

## 3. Research artifact

The eight-bit bytes at offsets 0–3 are ASCII `TBS1`. Offset 4 stores
the width as one unsigned byte. Offsets 5–8 hold the original CRC-32/
ISO-HDLC as an unsigned little-endian integer. Offset 9 begins exactly
one complete TK32 stream extending to physical EOF. Apply RFC 0032
validation to that stream. Its declared output length gives the original
length and block boundaries. Restore the permutation and compare the
original checksum. Reject truncated wrappers, unknown magic or width,
invalid inner streams, output beyond the caller's limit, and checksum
mismatches. The screen limits original output to 8 MiB and total wrapped
input to 32 MiB. Standalone, unwrapped TK32 represents identity.

## 4. Gates

Run identity and all three widths on each independent Calgary input.
The control is RFC 0034. Its encoder also compresses every transformed
input. Measure complete physical size, encode and decode wall time
and peak RSS, and check byte-for-byte recovery for every trial.
Select the smallest complete file, preferring identity on ties and
otherwise the smallest width. Report the sum of all trial encode times
separately from the selected encode times.

Advance if selected artifacts save at least 0.5% across Calgary.
Then apply the frozen procedure to independent Canterbury, excluding
ptt5 because it duplicates Calgary pic. Require at least 0.25% aggregate
savings there and exact restoration on all trials. Only a passing screen
justifies a bounded streaming implementation and enwik9 validation.

## 5. Bounded enwik9 validation

Use four 1 MiB samples starting at the block-aligned positions at or
immediately below offsets 0, 250,000,000, 500,000,000, and 750,000,000.
Compress each sample independently with all four profiles. Verify every
restored sample. Select a single width by total complete size across
the four samples. Require a 0.25% saving over identity to select a
shuffle. For ties, use the order from Section 4. Record all sample
trials. The samples select the width and do not stand in for complete
enwik9 accounting.

Use that one width for the complete enwik9 artifact. If identity is
selected, the complete artifact may reuse the recorded RFC 0034
stream after its input and artifact SHA-256 digests are checked.
Measure a fresh complete decode and compare every byte. For a nonzero
width, time a complete encode, time a complete decode, and compare
every byte. Count the header and the complete TK32 stream. Require a
smaller complete enwik9 artifact to claim a gain on that corpus.

For this stage the file-based wrapper transforms at most 65,536 bytes
at a time and stages the transformed stream and nested artifact on
disk. The wrapper accepts up to 4,294,967,295 original bytes and up
to 8 GiB of wrapped data, including the header. The decoder enforces
the caller's output limit before the inverse transform. The smaller
limits in Section 3 describe the corpus screening interface. During
wrapped decoding, temporary storage holds the nested artifact and up
to two copies of the decoded bytes.

## 6. Results

The selected artifacts save 12,460 bytes on Calgary and 6,370 bytes
on independent Canterbury. Each saving comes from one file. Both
size gates pass and every screened output restores exactly.

The enwik9 samples select identity. The selected complete 243,396,020-byte
artifact restores every original byte in a fresh decode. The result
retains the RFC 0034 enwik9 size. No full enwik9 artifact at a nonzero
width was measured.

[Experiment report](../../research/experiments/2026-11-25-byte-shuffle.md)
