# Accepted independent validation: finite-strength context prior

## Status

RFC 0006 strength 4 passes independent Calgary validation. The plan below was
committed to the working tree before downloading or measuring the corpus.

## Fixed model

Validate RFC 0006 strength 4 without changing its arithmetic, rescaling,
artifact, or in-memory representation. Compare complete `TKN2` artifacts with
complete `TKO2` artifacts. Compress every corpus file independently.

## Dataset groups

Use the Calgary corpus from the Canterbury Corpus project. Assign files before
measurement:

- prose/reference: `bib`, `book1`, `book2`, and `news`;
- technical text/source: `paper1` through `paper6`, `progc`, `progl`, `progp`,
  and `trans`; and
- binary: `geo`, `obj1`, `obj2`, and `pic`.

If the downloaded corpus has a different file set, stop and update this plan
before running the codec. Do not silently omit or reassign files.

## Gate

Validation passes only if fixed strength 4:

1. is at least 2% smaller than order-2 on two of three groups;
2. is not larger than order-2 in aggregate;
3. decodes no more than two times slower than order-2 in every group;
4. uses less than one gigabyte peak process resident memory; and
5. restores every file exactly.

Run one warm-up and five measured repetitions per file. Sum file times within
each group for each repetition and report the median. Preserve every raw row.

This gate compares only with the retained order-2 research control. Also report
gzip, xz, and Zstandard sizes under matching independent-file accounting for
distance-to-goal context. External performance accounting must be labeled if
it differs from the in-process TAK runner.

Do not tune strength or implementation on Calgary before deciding this gate.

## Dataset and environment

- Source: `https://corpus.canterbury.ac.nz/resources/calgary.tar.gz`.
- Archive SHA-256: `e109eebdc19c5cee533c58bd6a49a4be3a77cc52f84ba234a089148a4f2093b7`.
- Corpus size: 3,251,493 bytes in 18 independently compressed files.
- License: the archive contains no license file. TAK does not redistribute it.
- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one.
- Memory limit: no lower external limit imposed.
- TAK repetitions: one warm-up and five measured runs per file.
- Verification: all TAK and external decodes matched the exact input.

Raw results and SHA-256 digests:

- `benchmarks/reports/raw/2026-11-22-calgary-inputs.tsv`:
  `02c67202aa4635e7902af96420e29caad011297071dda38dfeac630ba3b45891`;
- `benchmarks/reports/raw/2026-11-22-calgary-tak.tsv`:
  `8b570b1003b6c8290f2bfdee53151bd816f1e82e62862fb61570b62f0c45550c`;
- `benchmarks/reports/raw/2026-11-22-calgary-external.tsv`:
  `c00add89cb1022d5d621e857a21ea50511afcfc70f8fa9743db8a904d2757d9c`;
  and
- `benchmarks/reports/raw/2026-11-22-calgary-memory.tsv`:
  `e2f5f0825741fde8b4880b461e8a65a5a5d96e01a856dbe24555048f6401053b`.

## TAK results

| Group | Input bytes | Order-2 bytes | Strength-4 bytes | Size reduction | Order-2 decode (s) | Strength-4 decode (s) | Decode slowdown |
|---|---:|---:|---:|---:|---:|---:|---:|
| Prose/reference | 1,867,997 | 869,090 | 698,535 | 19.62% | 0.2106 | 0.3649 | 1.73x |
| Technical/source | 499,562 | 290,089 | 177,441 | 38.83% | 0.0586 | 0.0946 | 1.61x |
| Binary | 883,934 | 314,780 | 231,409 | 26.49% | 0.0802 | 0.1183 | 1.48x |
| Total | 3,251,493 | 1,473,959 | 1,107,385 | 24.87% | — | — | — |

Every group passes both the size and decoder-speed conditions. Peak
complete-process resident memory was 9,216 KiB on `book1`, far below one
gigabyte. All 90 measured TAK decodes restored their inputs exactly.

## External size context

Versions were gzip 1.13, XZ Utils 5.8.1, and Zstandard 1.5.7. Every method
compressed each file independently.

| Method | Complete bytes | Ratio |
|---|---:|---:|
| TAK order-2 | 1,473,959 | 45.33% |
| TAK normalized strength 4 | 1,107,385 | 34.06% |
| gzip -9 | 1,059,335 | 32.58% |
| xz -9 | 885,860 | 27.25% |
| zstd -19 | 921,073 | 28.33% |

External timing was not measured in this validation. Do not infer a speed
comparison from this table.

## Distance to project goals

The enwik9 milestones remain unmeasured. Do not extrapolate Calgary ratios to
enwik9.

On Calgary, normalized TAK is 48,050 bytes, or 4.5%, larger than gzip under
matching size accounting. It must remove 4.3% of its current compressed bytes
to equal gzip. It is 25.0% larger than xz and must remove 20.0% of its bytes to
equal xz. It is 20.2% larger than Zstandard level 19 and must remove 16.8% to
equal it.

This is substantially closer to the gzip baseline than the prior Canterbury
development result, where TAK was 6.9% larger. It is still not evidence that
the enwik9 M1 target is close.

## Decision

Accept independent validation of RFC 0006 strength 4. It generalizes the
development improvement to all three Calgary groups while staying within the
predeclared speed and memory limits.

Keep the model research-only. One small legacy corpus is not enough to assign
a TAK-C method identifier or claim competitive compression. The next step is
a larger held-out mixed corpus and profiling against same-data modern codecs.
This result does not authorize tensor work.
