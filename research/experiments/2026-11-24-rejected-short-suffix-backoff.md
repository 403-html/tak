# Short-suffix literal backoff

RFC 0030 failed its aggregate Silesia gate. The fixed RFC 0019 parse
supplied the same events to both models. The candidate used the last
continuation of the preceding three bytes when the eight-byte prediction
was unavailable, with one adaptive six-expert integer mixture.

| File | Eligible literals | Hit rate | Ideal bits saved |
|---|---:|---:|---:|
| Dickens | 352,780 | 42.48% | 24,484.301 |
| Mozilla | 1,131,020 | 41.83% | 36,390.980 |
| OOffice | 157,355 | 35.24% | −3,621.358 |
| Samba | 242,696 | 48.76% | 48,932.250 |
| XML | 71,156 | 53.22% | 14,183.573 |
| **Total** | **1,954,007** | | **120,369.746** |

The candidate cleared the three-file requirement. Its aggregate gain of
0.012735 bits per input byte fell below the frozen 0.016 threshold.
The extra lookup table contains 2 MiB of entries. The largest diagnostic
process used 154,368 KiB peak RSS. Each timing is a one-sample screen.

The diagnostic measured literal coding costs. Artifact size, round-trip
restoration, and decoder throughput for this candidate are unmeasured.
The failed development gate ended the experiment before Calgary and
enwik9 validation.

The measured Calgary leaders are RFC 0019 at 942,454 bytes and xz at
885,860 bytes. A winning TAK stream must remove at least 56,595 bytes
from the existing artifact total. On enwik9, RFC 0019 is 249,762,881 bytes,
34,088,685 bytes above xz and 52,762,881 bytes above nominal M2.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-match-distance-literal
python3 benchmarks/runners/measure.py --timing timing.tsv --stdout row.tsv \
  target/release/tak-match-distance-literal --short-suffix INPUT --tail-bytes N
```

Use the exact tail length and input digest in each raw row.
The release build used Rust 1.85.0 on Linux 6.18.57 and an x86-64 Intel
Xeon Platinum 8573C. The environment exposed three logical CPUs and
10,207,480 KiB reported memory. Each run used one thread without an
experiment-specific memory limit. The runner used Python 3.12.14.
The source is commit `318beee` plus the RFC 0030 diagnostic.

Raw TSV SHA-256 is
`01276bdf6b93cdb17cbfd1713ae52dd83c5b73291a467cbfe5692aa021c60016`.
[Raw rows](../../benchmarks/reports/raw/2026-11-24-short-suffix-backoff.tsv)
