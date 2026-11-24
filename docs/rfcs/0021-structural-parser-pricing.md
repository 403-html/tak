# RFC 0021: Structural parser pricing

- Status: Rejected at Silesia complete-artifact gate
- Date: 2026-11-24
- Scope: Classical encoder parser only
- Format change: None

## 1. Problem statement

RFC 0019 changes block-type coding, but it retains RFC 0016's price-based
parse. That parser prices every block with the old global three-symbol type
distribution. The `TKT9` artifact instead uses one binary literal-or-match
distribution, omits a match type after a literal, and omits the end symbol.

The parser can therefore reject or accept a match using a type price that the
encoder does not pay. This RFC aligns parser prices with the accepted syntax.

## 2. Hypothesis

Structural pricing will make complete `TKT9` artifacts smaller than RFC 0019
on all five Silesia development tails. It MUST save at least 0.005 bits per
input byte in aggregate.

If the development gate passes, the frozen parser MUST improve at least 15 of
18 Calgary files and save at least 1,500 bytes in aggregate. Do not change any
price, parser limit, candidate rule, or gate after inspecting results.

## 3. Method

Retain RFC 0016's match table, two candidates, 65,536-position segment, 1,024
nice length, priced length set, literal byte costs, and adaptive length and
distance models.

Replace only block-type prices and updates:

1. A literal block pays symbol zero from the RFC 0019 binary distribution.
2. A match after a match or stream start pays symbol one.
3. A match after a literal pays zero type bits.
4. No end-symbol price is used.
5. Only explicit binary symbols update the type distribution.

The dynamic-programming states already distinguish a path ending in a literal
from a path ending in a match. No additional parse state is required.

The byte model still updates for every input byte independently of the chosen
parse. Floating-point prices affect encoder choices only. Decoding remains
integer-only and deterministic for a given artifact.

## 4. Complete-artifact gate

Use the complete physical `TKT9` artifact as the accounting unit. Compare the
candidate with RFC 0019 on identical inputs. The Silesia gate requires:

- every artifact is smaller;
- aggregate savings are at least 0.005 bits per input byte;
- every output restores exactly;
- median decode time is at most 1.05 times RFC 0019; and
- encoder and decoder peak RSS stay below one gigabyte.

Use one warm-up and five measured repetitions if the size gate passes. Then
freeze the parser and validate Calgary under Section 2. Calgary decode time
MUST remain within 1.05 times RFC 0019, and every output MUST restore exactly.

Only a passing Calgary result permits a bounded byte-identity implementation
and enwik9 validation. Do not tune on Calgary or enwik9.

## 5. Compatibility and decoder

The candidate emits the unchanged `TKT9` syntax. It assigns no method
identifier and changes no decoder operation, limit, checksum, or rejection
condition. Different encoders may already emit different valid parses.

## 6. Tensor scope

This RFC does not authorize a tensor model. It corrects a deterministic
encoder cost mismatch. No statistical parameter-sharing problem exists.

A later tensor RFC still requires a specific measured longer-context
dependency that the rejected bigram factorization cannot represent. Increasing
rank remains unjustified.

## 7. Stop conditions

If the Silesia gate fails, reject structural repricing and do not tune parser
prices or limits on those files. If Silesia passes and Calgary fails, preserve
both results and stop before enwik9.

## 8. Result

Reject the candidate. Mozilla grows by 313 bytes, so the candidate fails the
requirement that every Silesia artifact be smaller. The candidate saves only
494 bytes across 9,451,681 input bytes, or 0.000418 bits per input byte. This
is below the required 0.005 bits per input byte.

All five screening outputs restore exactly. The size gate failed before the
controlled timing and memory stage. Calgary, bounded byte identity, and
enwik9 were not run. Do not tune structural parser prices on these files.

See
`research/experiments/2026-11-24-rejected-structural-parser-pricing.md`.
