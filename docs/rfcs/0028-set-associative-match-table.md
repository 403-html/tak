# RFC 0028: Set-associative parser match table

- Status: Proposed
- Date: 2026-11-24
- Scope: Classical LZ match finder
- Format change: None

## 1. Problem statement

RFC 0019 uses a direct-mapped parser table with 1,048,576 entries. Each entry
stores the two most recent positions for one exact eight-byte key. A different
key with the same table index replaces the complete entry. The parser then
loses both occurrences even if the evicted key appears again.

RFC 0015 prohibits another candidate-count screen. This RFC does not increase
the two candidates retained for an exact key. It tests collision handling
between different keys while keeping the total entry count fixed.

## 2. Hypothesis

A two-way set-associative table will preserve useful exact-key histories that
the direct-mapped table evicts. The recovered matches will make complete RFC
0019 artifacts smaller without increasing match-table memory or changing the
decoder.

## 3. Candidate table

Use 524,288 sets with two entries per set. The table therefore contains the
same 1,048,576 entries as the control.

Map each exact eight-byte key to one set with the existing multiplicative
hash, using its upper 19 bits. Search both ways for an equal key.

Insert a key as follows:

1. If one way contains the key, update its two occurrence positions with the
   RFC 0015 rule.
2. Otherwise, use an unoccupied way if one exists.
3. Otherwise, replace the way whose newest occurrence position is older.
4. If both newest positions are equal, replace way zero.

Store no recency metadata. The newest source positions define replacement
order. Keep at most two occurrence candidates for an equal key.

## 4. Frozen codec

Change only the parser match table. Retain these RFC 0019 components without
parameter changes:

- the price-based dynamic-programming parser;
- 65,536-byte segments;
- the 1,024-byte nice-match condition;
- the tested match lengths and prices;
- the structural block-type syntax;
- literal, length, and distance models; and
- the existing decoder.

The candidate artifact uses the existing `TKT9` syntax. Different valid
parser decisions do not require a format identifier.

## 5. Development gate

Use the five established final-10% Silesia development tails. Compare complete
candidate and RFC 0019 artifacts from the same source revision.

The candidate MUST:

- restore every input byte exactly through the existing RFC 0019 decoder;
- make all five artifacts smaller;
- save at least 0.001 bits per input byte in aggregate;
- use less than one gigabyte peak process RSS;
- keep median encoding time within 1.25 times the control; and
- keep median decoding time within 1.10 times the control.

Use one screening run for the size gate. If it passes, use one warm-up and
five measured repetitions for time and memory. Stop before Calgary if any
size condition fails.

## 6. Calgary validation

Freeze the implementation before Calgary. Compress all 18 files independently
under the established accounting method.

The candidate MUST improve at least 15 files and save at least 500 bytes in
aggregate. Every file MUST restore exactly. The median aggregate encode and
decode ratios MUST satisfy the development limits.

Do not change associativity, set count, replacement, segment size, or parser
prices after inspecting Silesia or Calgary results.

Only a passing Calgary result permits a bounded implementation and enwik9
validation. The bounded encoder MUST produce byte-identical candidate
artifacts. Enwik9 MUST restore all 1,000,000,000 bytes exactly.

## 7. Accounting and reporting

Report complete artifact bytes, per-file size change, encoding and decoding
wall time, throughput, peak RSS, and exact SHA-256 restoration. Record input
digests and every raw repetition.

Also report parser table bytes. The candidate and control each allocate the
same number of entry values. Vector and alignment overhead MUST be included in
process RSS measurements.

Do not extrapolate Silesia or Calgary savings to enwik9. Do not report a
collision count as compressed bytes.

## 8. Exactness and security

The candidate changes encoder decisions only. The existing decoder reconstructs
literal and match blocks exactly. It MUST continue to enforce output length,
distance, payload extent, checksum, and caller output limits.

The parser MUST use checked positions and existing maximum-distance and
maximum-match limits. Hash collisions MUST compare the complete eight-byte key
before returning candidates.

## 9. Stop conditions

Reject the candidate if the Silesia or Calgary gate fails. Do not tune the
number of ways or add more occurrences per key from a failed result.

If the candidate passes Calgary but does not improve enwik9, preserve all
measurements and retain RFC 0019 as the benchmark profile. A later match-finder
experiment must introduce a different search mechanism, not another
associativity value.

## 10. Tensor scope

This RFC does not authorize a tensor model. Table collision is a deterministic
data-structure limitation with a direct classical control. It does not define
a longer-context statistical dependency.

