# RFC 0017: Literal-phase match reliability

- Status: Accepted; held-out and bounded enwik9 validation complete
- Date: 2026-11-23
- Scope: Classical byte-model control before later tensor work
- TAK-C method identifier: None

## 1. Problem statement

RFC 0016 codes 1,895,245 Calgary bytes as literals. This is 58.29% of the
input. The RFC 0009 byte model gives the same exact-suffix match bonus to all
bytes. RFC 0016 exposes additional decoder-known state: the byte offset inside
the current literal block.

Exact-suffix prediction reliability changes with that offset. On the Silesia
development tails, the prediction is correct for 32.3% of offset-zero events
and 84.9% of events at offsets 16 and later. A single fixed bonus cannot
represent this interaction.

This is a longer-context dependency that RFC 0001 cannot represent. Its
bigram tensor conditions only on the preceding byte. Increasing its rank does
not add literal-block state or the preceding eight-byte suffix.

## 2. Hypothesis

A deterministic mixture of exact-match bonus experts, maintained separately
for four literal phases, reduces RFC 0016 ideal literal cost by at least 0.003
bits per input byte in aggregate on the five Silesia development groups. It
MUST improve all five groups.

The phases use literal offsets:

1. `0`;
2. `1` through `3`;
3. `4` through `15`; and
4. `16` and later.

The classical control is one mixture shared by all literal offsets. Both
models use the same six bonus experts. The candidate must also beat the fixed
RFC 0016 divisor-4 model.

## 3. Experts

Let `T` be the normalized order-2 total before an exact-match bonus. The six
experts add these frequencies to the predicted byte:

```text
T, floor(T / 2), floor(T / 4), floor(T / 8), floor(T / 16), 0
```

All other symbol frequencies stay unchanged. Each expert total is `T` plus
its bonus. The underlying normalized order-2 model and exact eight-byte match
table remain unchanged.

## 4. Deterministic integer mixture

Each phase keeps six unsigned 32-bit weights. Initialize every weight to one.
Use unsigned 128-bit intermediates in the operations below.

Set `S = 2^32` and `W = 65535`. For expert `i`, let `B[i]` be its bonus and
`w[i]` be its current weight. Before coding a predicted literal, calculate:

```text
A[i] = floor(w[i] * S / (T + B[i]))
A = sum(A[i])
C = sum(A[i] * B[i])
B = min(T, floor(C / A))
```

Add `B` to the predicted-symbol frequency. Code the literal with total
`T + B`. If no exact-suffix prediction exists, use the baseline distribution
and do not update mixture weights.

After coding the exact literal, let `F` be its baseline frequency and let `H`
be one if it equals the prediction and zero otherwise. Calculate:

```text
Q[i] = floor(w[i] * (F + H * B[i]) * S / (T + B[i]))
Q = sum(Q[i])
w[i] = max(1, floor(Q[i] * W / Q))
```

Then update the underlying byte model. The encoder and decoder MUST perform
these operations in this order. Integer overflow MUST cause rejection rather
than wrapping.

## 5. Parser and artifact

The candidate MUST use the frozen RFC 0016 price-based parse. Parser prices
remain those of RFC 0016. Thus candidate and control use identical literal and
match blocks.

The research artifact uses ASCII magic `TKP7`. The remaining 24-byte header
and the block syntax equal RFC 0014. Version, flags, lengths, checksum,
boundaries, and rejection conditions do not change. `TKP7` is not TAK-C and
has no stable identifier.

The phase mixture applies only while coding literal bytes. Match bytes update
the underlying byte model but do not update mixture weights.

## 6. Complete-artifact gate

Implement the candidate only after the opportunity gate in Section 8 passes.
Compare complete RFC 0016 and RFC 0017 artifacts on the same Silesia
development tails. The complete gate passes only if:

- every candidate artifact is smaller;
- the aggregate decrease is at least 0.002 bits per input byte;
- every candidate restores the exact input;
- candidate decode time is at most 1.25 times the RFC 0016 decode time; and
- encoder and decoder peak RSS stay below one gigabyte.

Use one warm-up and five measured repetitions for publishable timing. A
single run is screening evidence.

If the complete gate passes, freeze the method. Validate it on previously
unused files before an enwik9 run. Calgary and Canterbury are not held-out for
this RFC because their opportunity results were inspected during development.

The held-out validation uses the final 10% of `reymont`, `nci`, and `x-ray`
from Silesia. These files represent text, a chemical database, and binary
medical data. The candidate passes only if every artifact is smaller, every
artifact restores the exact input, the aggregate size decreases, median
candidate decode time is at most 1.25 times control, and peak RSS stays below
one gigabyte. Use one warm-up and five measured repetitions. This rule was
fixed before these files were measured.

## 7. Tensor scope

This RFC does not authorize a tensor implementation. It establishes a
specific parser-state dependency and a strong classical control. A later
tensor RFC MUST name additional interacting modes, include this mixture as an
ablation, count every tensor parameter, and beat the complete RFC 0017
artifact. Increasing a rejected tensor rank is still prohibited.

## 8. Opportunity results

The deterministic integer mixture passes the gate. It improves all five
Silesia groups and all 18 Calgary files.

| Corpus | Input bytes | Fixed RFC 0016 literal bits | Integer phase-mixture bits | Saved bits/byte |
|---|---:|---:|---:|---:|
| Silesia development | 9,451,681 | 15,997,349.680 | 15,950,811.291 | 0.00492 |
| Calgary confirmation | 3,251,493 | 5,702,403.522 | 5,677,995.141 | 0.00751 |

A floating Bayesian phase mixture saves 49,114 Silesia bits. Integer
quantization retains 46,538 bits, or 94.8% of that gain. On Calgary it retains
24,408 of 26,746 bits, or 91.3%.

Two broader variants fail:

- A fixed divisor per phase regresses the Silesia aggregate. Do not tune more
  fixed mappings.
- Separate mixtures over phase, literal length, and previous match distance
  add only 0.00065 bits per byte on Silesia and 0.00011 on Calgary. They
  improve only five Calgary files. Do not use these modes for a tensor.

Raw rows are in
`benchmarks/reports/raw/2026-11-23-literal-phase-gate.tsv`, SHA-256
`943f72f1ac2f28f910ddcecadb6f9d21eafbb8462ab840c304cd5ad7544ce61d`.

## 8.1. Complete development artifact

The complete artifact passes the five-group Silesia development gate. RFC
0016 uses 2,702,364 bytes. RFC 0017 uses 2,696,546 bytes. It saves 5,818 bytes,
or 0.00492 bits per input byte. Every artifact is smaller and restores the
exact input.

Median candidate/control decode-time ratios range from 1.05 to 1.17. The
largest measured peak RSS is 145,024 KiB. Both results pass Section 6. A
decoder-only cache avoids recalculating the exact same bonus and mixture
update inputs. It does not change artifact bytes.

The memory screen is in
`benchmarks/reports/raw/2026-11-24-literal-phase-memory-screen.tsv`, SHA-256
`172be6d8443a2d944ab53a0c195840e2f813c4c2db4c5c3846b54780aee0e86b`.

An attempted replacement of the specified 128-bit intermediate arithmetic
with 64-bit operations regressed the aggregate by 2,278 bytes. The attempt
was reverted. Implementations MUST use the Section 4 arithmetic.

## 8.2. Held-out validation

The frozen method passes all three previously unused Silesia tails. Reymont
decreases by 889 bytes, NCI by 1,853 bytes, and X-ray by 54 bytes. The
aggregate decreases by 2,796 bytes. Every output is byte-for-byte identical
to its input. Median candidate/control decode ratios are 1.10, 0.95, and 1.08.

The controlled rows are in
`benchmarks/reports/raw/2026-11-23-literal-phase-controlled.tsv`, SHA-256
`16907f56535c48e76d01e549fb6e7182c3d442f6c7c910a8858575aae41388d3`.

## 8.3. Bounded enwik9 validation

The bounded RFC 0017 encoder produces a 251,698,380-byte artifact. This is
280,109 bytes smaller than RFC 0016. Encoding takes 553.39 seconds at 1.81
MB/s with 220,796 KiB peak RSS. Bounded decoding takes 107.87 seconds at 9.27
MB/s with 96,064 KiB peak RSS.

The input and restored output contain 1,000,000,000 bytes. Both SHA-256 values
are `159b85351e5f76e60cbe32e04c677847a9ecba3adc79addab6f4c6c7aa3744bc`.
The artifact SHA-256 is
`2342b9845c52b397f539294d4fe9cf11cc50b9c3c9a57c7971a6b458b25d6415`.

RFC 0017 remains 16,369,441 bytes above Zstandard-19, 36,024,184 bytes above
xz, and 54,698,380 bytes above M2. Matching xz requires a further 14.31%
reduction from this artifact. Do not tune phase boundaries or experts on
enwik9.

Raw data is in
`benchmarks/reports/raw/2026-11-24-literal-phase-enwik9.tsv`, SHA-256
`ace40ef031dafa52357a7c2f1788a08777337a05b3c7a1a74df1ac8aede7bd2e`.

## 9. Testing strategy

Tests MUST compare the integer mixture with an independent reference
calculation. They MUST cover every phase, every expert, missing predictions,
weight normalization, and maximum arithmetic bounds. Candidate artifacts MUST
round trip through buffered and bounded decoders. Corrupt, truncated, and
over-limit artifacts MUST be rejected.

## 10. Alternatives considered

### 10.1. One raw correctness probability per phase

Reject. It discards the baseline byte probability and regresses against RFC
0016 by 0.03489 bits per byte on Silesia.

### 10.2. One count-derived bonus per phase

Reject. It regresses against RFC 0016 by 0.00340 bits per byte on Silesia.

### 10.3. Tensorize phase immediately

Reject. Four phase states and six experts need only 24 integer weights. A
tensor adds no useful parameter sharing at this size.
