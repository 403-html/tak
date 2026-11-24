# Rejected distance-remainder context models

## Decision

Reject adaptive distance-remainder models keyed by bit position or by distance
bucket and bit position. Do not create a format RFC. The development saving is
concentrated in Mozilla and OOffice. It does not transfer across Calgary.

This is a classical diagnostic. It does not identify a tensor opportunity.

## Hypothesis and stop condition

RFC 0016 reports that raw distance remainders are the largest metadata cost.
They use 0.513 bits per input byte on the five Silesia development tails. The
hypothesis was that decoder-known distance bucket and remainder-bit position
predict these bits.

The control charges one bit for each remainder bit. The candidates use
adaptive binary counts initialized to one for each symbol. One candidate
shares a model by bit position. The other uses one model for each pair of
distance bucket and bit position. Counts halve at the existing 16,384 total.
The diagnostic includes adaptation cost. It stores no model parameters.

The mechanism advances only if the bucket-position model improves all five
Silesia groups and at least 12 of 18 Calgary files, and its Calgary saving is
at least 0.03 bits per input byte. This threshold is less than one fifth of
the 0.1597-bit Calgary gap from RFC 0016 to xz.

## Results

| Corpus | Input bytes | Raw remainder bits | Global-position bits | Bucket-position bits | Bucket saving/input byte | Files improved |
|---|---:|---:|---:|---:|---:|---:|
| Silesia development | 9,451,681 | 2,501,101 | 2,400,271.634 | 2,335,457.887 | 0.01753 | 3/5 |
| Calgary validation | 3,251,493 | 913,478 | 903,057.676 | 899,289.920 | 0.00436 | 3/18 |

The candidate fails every transfer condition. On Silesia, Mozilla and OOffice
provide 164,957 of the 165,643 saved bucket-model bits. On Calgary, `pic` and
`obj2` provide 16,320 bits of saving, while most files regress.

An unattainable per-file oracle can select the cheapest of raw,
global-position, and bucket-position coding without signaling its selection.
It saves only 17,177 Calgary bits, or approximately 2,147 bytes. This is 3.3%
of the 64,908-byte RFC 0016-to-xz gap. RFC 0017 reduces the enwik9 result but
does not change this Calgary parse or distance sequence.

Because even this zero-overhead oracle is small, do not add mixtures, model
selection flags, match-length contexts, or tensor factorization for distance
remainders. A complete artifact and enwik9 run are not justified.

## Performance and accounting

The profiler uses the frozen RFC 0016 parse. It measures ideal remainder cost,
not a compressed artifact. The five Silesia processes take 5.60 seconds in
total and peak at 137,852 KiB. The 18 Calgary processes take 2.78 seconds in
total and peak at 54,844 KiB. Each process is single-threaded. These are
single-run screening measurements.

Raw rows are in
`benchmarks/reports/raw/2026-11-24-distance-remainder-model.tsv`, SHA-256
`fd8d19ace305dd79aa3bc70a6b76509fcb6d3238eeee072b31a3cf079ec225af`.

## Distance to goals

The accepted RFC 0017 enwik9 artifact remains 251,698,380 bytes. It remains
36,024,184 bytes above xz and needs a further 14.31% reduction. This rejected
diagnostic does not change that measured position.

## Environment

- CPU: Intel Xeon Platinum 8573C, x86-64, three visible logical CPUs.
- OS: Linux 6.18.57.
- Compiler: Rust 1.85.0, release profile.
- Threads: one per profiler process.
- Memory limit: none imposed.
- Date: 2026-11-24 Europe/Warsaw.
- Source: commit `3516393` plus the diagnostic working tree.
