#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 INPUT OUTPUT_DIRECTORY" >&2
  exit 64
fi
if ! command -v python3 >/dev/null; then
  echo "error: this Linux runner requires Python 3 for process measurements" >&2
  exit 69
fi

input=$(realpath "$1")
output_dir=$2
repetitions=${TAK_BENCH_REPETITIONS:-5}
repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
tak_bin="$repo_dir/target/release/tak"
measure="$repo_dir/benchmarks/runners/measure.py"
work_dir="$output_dir/.work"
results="$output_dir/results.tsv"

if [[ -e "$work_dir" || -e "$results" ]]; then
  echo "error: output directory already contains benchmark work or results" >&2
  exit 73
fi
mkdir -p "$work_dir"
cargo build --release --manifest-path "$repo_dir/Cargo.toml" -p tak-cli

{
  echo -e "key\tvalue"
  echo -e "utc_date\t$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo -e "commit\t$(git -C "$repo_dir" rev-parse HEAD)"
  echo -e "input\t$input"
  echo -e "input_bytes\t$(wc -c < "$input")"
  echo -e "input_sha256\t$(sha256sum "$input" | awk '{print $1}')"
  echo -e "os\t$(uname -srvmo)"
  echo -e "cpu\t$(awk -F: '/model name/{sub(/^ /, "", $2); print $2; exit}' /proc/cpuinfo)"
  echo -e "logical_cpus\t$(getconf _NPROCESSORS_ONLN)"
  echo -e "memory_kib\t$(awk '/MemTotal/{print $2}' /proc/meminfo)"
  echo -e "threads\t1"
  echo -e "repetitions\t$repetitions"
  echo -e "rustc\t$(rustc --version)"
  echo -e "tak\t$($tak_bin --version)"
  command -v gzip >/dev/null && echo -e "gzip\t$(gzip --version | head -1)"
  command -v xz >/dev/null && echo -e "xz\t$(xz --version | head -1)"
  command -v zstd >/dev/null && echo -e "zstd\t$(zstd --version)"
} > "$output_dir/environment.tsv"

echo -e "method\toperation\trepetition\tinput_bytes\toutput_bytes\twall_seconds\tpeak_rss_kib\troundtrip" > "$results"

record() {
  local method=$1 operation=$2 repetition=$3 input_bytes=$4 output_bytes=$5 timing=$6 roundtrip=$7
  local wall peak
  IFS=$'\t' read -r wall peak < "$timing"
  echo -e "$method\t$operation\t$repetition\t$input_bytes\t$output_bytes\t$wall\t$peak\t$roundtrip" >> "$results"
}

run_tak() {
  local method=$1 repetition=$2
  local encoded="$work_dir/$method-$repetition.takc"
  local decoded="$work_dir/$method-$repetition.out"
  local timing="$work_dir/time.txt"
  python3 "$measure" --timing "$timing" --stdout /dev/null "$tak_bin" compress "$input" -o "$encoded" --method "$method"
  record "$method" compress "$repetition" "$(wc -c < "$input")" "$(wc -c < "$encoded")" "$timing" pending
  python3 "$measure" --timing "$timing" --stdout /dev/null "$tak_bin" decompress "$encoded" -o "$decoded" --max-output-bytes "$(wc -c < "$input")"
  local verified=false
  cmp -s "$input" "$decoded" && verified=true
  record "$method" decompress "$repetition" "$(wc -c < "$encoded")" "$(wc -c < "$decoded")" "$timing" "$verified"
  rm "$encoded" "$decoded"
}

run_external() {
  local method=$1 repetition=$2
  local encoded="$work_dir/$method-$repetition.bin"
  local decoded="$work_dir/$method-$repetition.out"
  local timing="$work_dir/time.txt"
  case "$method" in
    gzip-9) python3 "$measure" --timing "$timing" --stdout "$encoded" gzip -9 -c "$input" ;;
    xz-9) python3 "$measure" --timing "$timing" --stdout "$encoded" xz -9 -c "$input" ;;
    zstd-19) python3 "$measure" --timing "$timing" --stdout "$encoded" zstd -q -19 -c "$input" ;;
  esac
  record "$method" compress "$repetition" "$(wc -c < "$input")" "$(wc -c < "$encoded")" "$timing" pending
  case "$method" in
    gzip-9) python3 "$measure" --timing "$timing" --stdout "$decoded" gzip -d -c "$encoded" ;;
    xz-9) python3 "$measure" --timing "$timing" --stdout "$decoded" xz -d -c "$encoded" ;;
    zstd-19) python3 "$measure" --timing "$timing" --stdout "$decoded" zstd -q -d -c "$encoded" ;;
  esac
  local verified=false
  cmp -s "$input" "$decoded" && verified=true
  record "$method" decompress "$repetition" "$(wc -c < "$encoded")" "$(wc -c < "$decoded")" "$timing" "$verified"
  rm "$encoded" "$decoded"
}

for method in raw order-0 order-1; do
  run_tak "$method" warmup
  for repetition in $(seq 1 "$repetitions"); do run_tak "$method" "$repetition"; done
done

for method in gzip-9 xz-9 zstd-19; do
  executable=${method%%-*}
  if command -v "$executable" >/dev/null; then
    run_external "$method" warmup
    for repetition in $(seq 1 "$repetitions"); do run_external "$method" "$repetition"; done
  fi
done

rm -r "$work_dir"
echo "Wrote $output_dir/environment.tsv and $results" >&2
