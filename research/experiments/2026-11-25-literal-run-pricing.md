# Literal run length prices

RFC 0042 charges the change in length-bucket price when the TK32
parser extends the literal run. Both profiles encode unwrapped,
complete TK32 streams. All 36 Calgary artifacts restore exactly
with the existing decoder.

| Calgary profile | Complete bytes | Encode s | Decode s | Encode MB/s | Decode MB/s | Peak encoder KiB | Peak decoder KiB |
|---|---:|---:|---:|---:|---:|---:|---:|
| RFC 0034 control | 906,449 | 4.455 | 1.848 | 0.730 | 1.760 | 113,432 | 43,932 |
| RFC 0042 literal run prices | 907,499 | 4.410 | 1.819 | 0.737 | 1.787 | 113,304 | 44,064 |

Five files shrink, 12 grow, and paper4 retains the same size.
The five improvements total 18 bytes. Book2 alone grows by 563
bytes. The aggregate grows by 1,050 bytes (0.116%). The gate
requires a 2,267-byte decrease and at least ten smaller files.
This two-state parser keeps one literal run length per position.
The screen therefore measures the tested approximation.

Times sum the per-file wall times. Peak RSS is the maximum per-file
value. Exact comparison follows each timed decode. These are one-run
screening measurements and do not establish a speed difference.

The candidate's unwrapped total is 21,639 bytes above xz -9 on Calgary.
The retained RFC 0038 selection is 893,989 bytes, 8,129 bytes above
xz -9. Canterbury, wrapped selection, and enwik9 remain unmeasured
for this candidate. The retained enwik9 result is 243,396,020 bytes,
27,721,824 bytes above xz -9 and 46,396,020 above the nominal
197 MB M2 ceiling.

## Reproduction

```console
cargo build --release -p tak-tensor --bin tak-lz-stream
python3 benchmarks/runners/measure.py --timing enc.tsv --stdout enc.out \
  target/release/tak-lz-stream encode-variable-order-repriced INPUT ARTIFACT
python3 benchmarks/runners/measure.py --timing dec.tsv --stdout dec.out \
  target/release/tak-lz-stream decode-variable-order ARTIFACT RESTORED --max-output INPUT_BYTES
cmp INPUT RESTORED
```

Run both profiles independently on each Calgary input. Use
`encode-variable-order-literal-run` for the candidate. Record both
input and complete artifact SHA-256. All inputs match the RFC 0034
Calgary records. The control also matches every recorded RFC 0037
control artifact digest and size.

A preliminary run allowed the candidate to restart a carried literal
run at a segment boundary. The corrected run removes that transition.
The two runs emit the same artifact SHA-256 for every input and
profile. The table uses the corrected run. The raw files preserve
both timing sets.

Source is `f07e8c1` plus RFC 0042. Linux 6.18.57, Intel Xeon Platinum
8573C, x86-64, three visible logical CPUs, 10,207,480 KiB visible RAM,
and an 8 GiB cgroup limit. Rust 1.85.0 release build, Python 3.12.14,
and one codec thread.

- [Raw corrected Calgary rows](../../benchmarks/reports/raw/2026-11-25-literal-run-pricing-calgary.tsv)
- [Raw preliminary rows](../../benchmarks/reports/raw/2026-11-25-literal-run-pricing-preliminary.tsv)
