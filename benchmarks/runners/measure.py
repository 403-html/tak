#!/usr/bin/env python3
"""Run one command and write wall time and peak child RSS as TSV."""

import argparse
import resource
import subprocess
import sys
import time


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--timing", required=True)
    parser.add_argument("--stdout", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    arguments = parser.parse_args()
    if not arguments.command:
        parser.error("a command is required")

    start = time.perf_counter()
    with open(arguments.stdout, "wb") as output:
        completed = subprocess.run(arguments.command, stdout=output, check=False)
    elapsed = time.perf_counter() - start
    peak_rss_kib = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
    with open(arguments.timing, "w", encoding="ascii") as timing:
        timing.write(f"{elapsed:.9f}\t{peak_rss_kib}\n")
    return completed.returncode


if __name__ == "__main__":
    sys.exit(main())

