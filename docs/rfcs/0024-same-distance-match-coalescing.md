# RFC 0024: Same-distance match coalescing

- Status: Rejected at Silesia complete-artifact gate
- Date: 2026-11-24
- Scope: Classical encoder post-processing
- Format change: None

## 1. Problem statement

RFC 0019 can emit consecutive match blocks with the same distance. Such
blocks describe one continuous copy operation. Each extra block repeats a
type decision, match length, and distance.

The parser works in bounded segments and caps a match at 65,535 bytes. It does
not canonicalize an adjacent same-distance run after parsing.

## 2. Hypothesis

Coalescing adjacent same-distance matches will make every complete RFC 0019
artifact smaller on the five Silesia development tails. It MUST save at least
0.001 bits per complete input byte in aggregate.

If Silesia passes, the frozen encoder MUST improve at least 15 of 18 Calgary
files and save at least 500 bytes in aggregate. Do not change the coalescing
rule or gates after inspecting Silesia.

## 3. Candidate

Use the frozen RFC 0019 price-based parse. Apply one deterministic pass to its
block sequence before entropy coding.

For each maximal run of adjacent match blocks with the same distance:

1. Sum their lengths with checked arithmetic.
2. Emit as many 65,535-byte match blocks as possible.
3. Emit one final match block for a nonzero remainder.
4. If that remainder is one through seven, shorten the preceding block so the
   final block has length eight.

Do not merge across a literal block or across different distances. Do not
change literal blocks, match search, parser prices, segment size, nice length,
or probability models.

The transformation is exact because a match copies from `distance` bytes
behind the current output position. Continuing with the same distance copies
the same byte sequence as one longer logical operation. Splitting at 65,535
preserves the existing format limit.

## 4. Complete-artifact gate

Compare complete physical `TKT9` artifacts with and without the coalescing
pass. Include the 28-byte header and arithmetic payload. Every candidate MUST
decode with the unchanged RFC 0019 decoder and restore the exact input.

The Silesia gate requires:

- every candidate artifact is smaller;
- aggregate savings are at least 0.001 bits per input byte;
- every decode restores exactly;
- median decode time is at most 1.05 times the control; and
- encoder and decoder peak RSS remain below one gigabyte.

Use one warm-up and five measured repetitions only if the size gate passes.
Then freeze the implementation for Calgary under Section 2. Calgary decode
time MUST remain within 1.05 times RFC 0019 and every decode MUST restore
exactly.

Only a passing Calgary result permits a bounded byte-identity implementation
and enwik9 validation. Do not tune on Calgary or enwik9.

## 5. Compatibility and decoder

The candidate emits the unchanged `TKT9` syntax. It assigns no identifier and
changes no decoder operation, allocation, limit, checksum, or rejection
condition. Existing RFC 0019 decoders can decode candidate artifacts.

## 6. Tensor scope

This RFC does not authorize a tensor. Same-distance adjacency is deterministic
parser state with a direct exact transformation. It has no statistical
parameter-sharing problem.

A later tensor RFC still requires a transferable longer-context dependency
that the rejected bigram factorization cannot represent. Increasing rank
remains unjustified.

## 7. Stop conditions

If the Silesia size gate fails, reject coalescing and stop before controlled
timing, Calgary, bounded identity, and enwik9. Do not add distance tolerances,
literal bridging, or parser changes after inspecting the result.

## 8. Result

Reject the candidate. It saves 31 complete-artifact bytes across 9,451,681
input bytes, or 0.000026 bits per input byte. This is below the required 0.001
bits per input byte. Dickens is byte-identical to the control, so the
requirement that every candidate be smaller also fails.

All five candidates decode through the unchanged RFC 0019 decoder and restore
exactly. The size gate failed before controlled timing and memory work.
Calgary, bounded byte identity, and enwik9 were not run.

See
`research/experiments/2026-11-24-rejected-same-distance-match-coalescing.md`.
