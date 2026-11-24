# RFC 0018: Short LZ match opportunity

- Status: Rejected after complete-artifact development gate
- Date: 2026-11-24
- Scope: Classical 3-to-7-byte LZ matches
- TAK-C method identifier: None

## 1. Problem statement

RFC 0017 retains RFC 0016's minimum LZ match length of eight bytes. Shorter
repeated strings remain literals even when their literal probability cost is
higher than a match token. Established LZ codecs commonly use shorter matches.

This RFC tests exact matches of three through seven bytes. It does not alter
the accepted RFC 0017 result until a complete artifact passes the gates below.

## 2. Opportunity method

The diagnostic uses the frozen RFC 0016 parse and the exact RFC 0017 literal
model. It maintains one latest position for each exact three-byte key in a
1,048,576-slot direct-mapped table. It searches only within literal blocks and
caps a candidate at the block end and seven bytes.

The diagnostic selects profitable candidates greedily in input order. Selected
candidates do not overlap. It inserts every input position in the table,
including match bytes and bytes covered by a selected opportunity.

For a distance `D`, the estimated token cost is:

```text
5 + floor(log2(D)) bits
```

The logarithmic term is the complete raw distance remainder width. The five
control bits optimistically represent the new block type, short length, and
distance bucket. The estimate does not charge literal-block splitting or
changes to adaptive distributions. It is an opportunity bound, not an
artifact size claim.

## 3. Opportunity gate

The diagnostic MUST improve all five Silesia development tails. It MUST then
improve at least 15 of 18 Calgary files without parameter changes. The Calgary
aggregate bound MUST be at least 0.08 bits per input byte.

If this gate fails, reject short matches and do not tune the table size,
minimum length, maximum length, fixed control cost, or selection rule on these
corpora.

## 4. Opportunity results

The opportunity passes the gate.

| Corpus | Input bytes | Events | Covered bytes | Literal bits | Estimated token bits | Estimated net bits/input byte | Files improved |
|---|---:|---:|---:|---:|---:|---:|---:|
| Silesia development | 9,451,681 | 328,975 | 1,414,542 | 6,448,349.978 | 4,561,925.000 | 0.19959 | 5/5 |
| Calgary validation | 3,251,493 | 124,802 | 554,805 | 2,156,901.928 | 1,683,503.000 | 0.14559 | 18/18 |

The Calgary bound is 473,399 bits, or approximately 59,175 bytes. This is
91.2% of the 64,908-byte RFC 0016-to-xz gap. The estimate is optimistic, so it
does not claim that a complete artifact will approach xz.

Raw rows are in
`benchmarks/reports/raw/2026-11-24-short-match-opportunity.tsv`, SHA-256
`8e97abaa3c74aca99fdc765450b4744daefec9a52d5cbb4c119510037886f9bc`.

## 5. Complete artifact

The experimental format MUST use a distinct research magic value. It MUST add
one short-match block type with lengths three through seven. It MUST preserve
RFC 0017 literal coding and ordinary matches. The encoder MUST use the frozen
opportunity search and selection rule before any result is inspected.

The Silesia complete-artifact gate passes only if:

- every output restores the exact input;
- at least four of five artifacts are smaller than RFC 0017;
- aggregate size decreases by at least 0.08 bits per input byte;
- median decode time is at most 1.25 times RFC 0017; and
- encoder and decoder peak RSS stay below one gigabyte.

If the development gate passes, freeze the implementation and validate all 18
Calgary files. Calgary MUST decrease by at least 30,000 bytes in aggregate,
every file MUST restore exactly, and median decode time MUST remain within
1.25 times RFC 0017. Use one warm-up and five measured repetitions.

Only a passing Calgary artifact permits a bounded enwik9 run. Do not tune the
method on Calgary or enwik9.

### 5.1 Result

The complete artifact fails the Silesia gate. It improves two of five files,
not the required four. It grows from 2,696,546 to 2,719,594 bytes in aggregate.
This is 23,048 bytes larger, or -0.01951 saved bits per input byte. The required
aggregate reduction is +0.08 bits per input byte.

Every screened artifact restores the exact input. The single-run decode ratios
range from 0.74 to 1.01, but these are screening values, not publishable timing
measurements. The size failure rejects the method before a memory measurement.
Calgary and enwik9 were not run.

The opportunity estimate omitted type-symbol cost, short-length cost,
literal-block fragmentation, and changed literal phases. Complete accounting
reverses the estimated gain on three files. See
`research/experiments/2026-11-24-rejected-short-lz-match-artifact.md`.

## 6. Decoder requirements

The short-match decoder MUST reject lengths outside three through seven. It
MUST reject zero distances, distances beyond the existing 64 MiB limit, and
distances beyond the restored prefix. Copy behavior MUST support overlap. All
existing output limits, extent checks, checksum coverage, and end-of-stream
rules remain mandatory.

## 7. Tensor scope

This RFC does not authorize a tensor model. The opportunity is an explicit LZ
mechanism. No tensor network is needed to copy an exact prior byte sequence.
A later tensor RFC still requires a different longer-context dependency and
must beat the complete classical control.

## 8. Testing strategy

Tests MUST cover lengths three and seven, overlapping copies, the maximum
distance, malformed lengths and distances, truncation, checksum corruption,
output limits, buffered and bounded round trips, and byte-identical bounded
encoding.

## 9. Performance and environment

The five Silesia profiler processes take 6.76 seconds in total. The 18 Calgary
processes take 2.91 seconds. Peak process RSS is 145,024 KiB. These are
single-run diagnostic measurements, not codec throughput results.

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one per profiler process.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `d776405` plus the RFC 0018 working tree.
