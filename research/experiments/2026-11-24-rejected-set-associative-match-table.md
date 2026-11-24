# Rejected experiment: set-associative parser match table

## Decision

Reject RFC 0028 at the Calgary validation gate. The fixed two-way table passes
the Silesia size and performance gates. It then improves only 10 of 18 Calgary
files, below the required 15. Stop before controlled Calgary timing, bounded
implementation, or enwik9.

Retain RFC 0019 as the benchmark profile. Do not tune associativity, set count,
replacement, or per-key occurrence count from this result.

## Mechanism

The RFC 0019 parser uses 1,048,576 direct-mapped entries. An unrelated exact
eight-byte key can replace both retained occurrences. The candidate uses
524,288 sets with two ways. It searches both ways for an equal key and replaces
the way with the older newest occurrence when both ways contain other keys.

The candidate retains the same 1,048,576 entries and at most two occurrence
candidates per equal key. It changes only encoder match discovery. It uses the
existing RFC 0019 parser prices, `TKT9` syntax, and decoder.

On the measured x86-64 target, an entry is 24 bytes. Each logical parser table
therefore contains 25,165,824 bytes. Process RSS includes vector and allocator
overhead.

## Silesia development result

| File | RFC 0019 bytes | Candidate bytes | Saved bytes |
|---|---:|---:|---:|
| Dickens | 314,932 | 314,303 | 629 |
| Mozilla | 1,682,999 | 1,682,250 | 749 |
| OOffice | 209,682 | 209,658 | 24 |
| Samba | 366,444 | 366,106 | 338 |
| XML | 106,742 | 106,684 | 58 |
| **Aggregate** | **2,680,799** | **2,679,001** | **1,798** |

Every artifact is smaller and restores exactly. The aggregate saving is
0.001522 bits per input byte, above the frozen 0.001 gate.

After one warm-up and five measured repetitions, the median aggregate encode
ratio is 0.911 and the median aggregate decode ratio is 0.979. Peak complete
process RSS is 145,016 KiB. These results pass the 1.25 encode, 1.10 decode,
and one-gigabyte memory limits.

## Calgary validation result

The frozen candidate reduces the independently compressed Calgary aggregate
from 942,454 to 941,663 bytes. It saves 791 bytes, or 0.001946 bits per input
byte. Ten files improve, eight are byte-identical, and none regress.

The aggregate saving exceeds the 500-byte requirement. The file-count gate
still fails because it requires 15 improvements. No post-result threshold or
replacement change is permitted. Calgary timing rows are non-publishable
parallel screening measurements and are not used for a performance claim.

The rejected candidate would remain 55,803 bytes above the measured 885,860
byte xz result. It closes only 1.40% of RFC 0019's 56,594-byte Calgary gap.

## Exactness and scope

All 23 screened artifacts and all 25 controlled Silesia artifacts restore the
input exactly through the existing RFC 0019 decoder. The candidate adds no
format fields or decoder state. Existing corruption, output-limit, distance,
extent, and checksum checks remain unchanged.

The initial screen accidentally combined the two-way table with rejected
structural parser prices. Those rows were discarded before inspection as RFC
0028 evidence. The implementation was corrected to the frozen RFC 0019 parser,
rebuilt, and all reported measurements were produced again. This report and
the raw files contain only the corrected experiment.

## Raw evidence

- Size and screening rows:
  `benchmarks/reports/raw/2026-11-24-set-associative-match-table.tsv`, SHA-256
  `b920cff62bd7f77e4e90b4afbed827a91503622a089e1b42c55754e79af30ef7`.
- Controlled Silesia rows:
  `benchmarks/reports/raw/2026-11-24-set-associative-controlled.tsv`, SHA-256
  `19f7c42d492f9c73bceb202506e89e02c9c9cefba18a83a42a61281aaf96fa54`.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one per process.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `6939e28` plus the RFC 0028 working tree.

## Distance to goals

RFC 0019 remains the best measured TAK enwik9 artifact at 249,762,881 bytes.
It remains 14,433,942 bytes above Zstandard-19, 34,088,685 bytes above xz, and
52,762,881 bytes above M2. Matching xz still requires a 13.65% reduction.

RFC 0028 has no enwik9 result. Do not extrapolate its development or Calgary
savings to enwik9.
