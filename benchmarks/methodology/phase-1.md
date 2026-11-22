# Phase 1 benchmark method

## Purpose

This method compares complete compressed streams. It does not estimate missing
measurements. It does not combine size, speed, and memory into one score.

## Dataset groups

Use at least one held-out file from each group:

- natural-language text, including `enwik9` when available;
- source code;
- JSON or structured logs;
- office or document data;
- binary data; and
- a mixed directory serialized by an external, documented container.

Record each input's byte length and SHA-256 digest. Keep tuning datasets and
held-out datasets separate.

## Environment record

Every report MUST identify:

- UTC date and TAK commit;
- operating system and kernel;
- CPU model and architecture;
- logical CPU count and configured thread count;
- physical memory and imposed memory limit;
- compiler and comparison-tool versions; and
- exact command line for every method.

## Measurement

Build release binaries before timing. Run one untimed warm-up and at least five
measured repetitions. Report every raw repetition and the median. Drop no
sample without recording the reason.

For compression and decompression, record:

- complete output bytes;
- wall-clock seconds;
- throughput in MiB/s;
- peak resident memory in KiB;
- exit status; and
- exact round-trip SHA-256 verification.

The current `tak benchmark` command is a quick development check. It performs
one in-process sample and does not measure peak RSS. It MUST NOT be used as a
publication-quality result.

## Initial comparisons

Record `gzip -9`, `xz -9`, available Zstandard levels, and every TAK method.
Add a maintained PPM implementation and a strong context-mixing implementation
after their build and accounting rules are documented. Never compare a bare
payload with a complete stream without a visible warning.

Plot separate Pareto diagrams for compressed bytes against compression speed,
decompression speed, and peak resident memory. Preserve dominated points in
the raw tables.

