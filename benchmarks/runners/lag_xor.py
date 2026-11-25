#!/usr/bin/env python3
"""Screen RFC 0031 wrappers around RFC 0019 streams on Linux."""

import argparse
import hashlib
from pathlib import Path
import struct
import subprocess
import sys
from tempfile import TemporaryDirectory
import zlib


LAGS = (0, 1, 2, 4, 8, 16, 32, 64, 128, 256)
LIMIT = 8 << 20
HEADER = struct.Struct("<4sHI")


def bounded_read(path, limit):
    with path.open("rb") as source:
        data = source.read(limit + 1)
    if len(data) > limit:
        raise ValueError("input exceeds the screen limit")
    return data


def transform(data, lag, inverse=False):
    result = bytearray(data)
    if lag:
        history = result if inverse else data
        for i in range(lag, len(result)):
            result[i] ^= history[i - lag]
    return result


def encode(codec, source, output, lag):
    data = bounded_read(source, LIMIT)
    if lag == 0:
        subprocess.run([codec, "encode-structural", source, output], check=True)
        return
    with TemporaryDirectory() as directory:
        residual = Path(directory) / "residual"
        inner = Path(directory) / "inner"
        residual.write_bytes(transform(data, lag))
        subprocess.run([codec, "encode-structural", residual, inner], check=True)
        payload = bounded_read(inner, (32 << 20) - HEADER.size)
        with output.open("xb") as artifact:
            artifact.write(HEADER.pack(b"TLX1", lag, zlib.crc32(data)))
            artifact.write(payload)


def decode(codec, source, output, limit):
    with source.open("rb") as artifact:
        magic = artifact.read(4)
    if magic == b"TKT9":
        subprocess.run(
            [codec, "decode-structural", source, output, "--max-output", str(limit)],
            check=True,
        )
        return
    data = bounded_read(source, 32 << 20)
    if len(data) < HEADER.size:
        raise ValueError("truncated wrapper")
    magic, lag, crc = HEADER.unpack_from(data)
    if magic != b"TLX1" or lag not in LAGS[1:]:
        raise ValueError("invalid wrapper magic or lag")
    with TemporaryDirectory() as directory:
        inner = Path(directory) / "inner"
        residual = Path(directory) / "residual"
        inner.write_bytes(data[HEADER.size:])
        subprocess.run(
            [codec, "decode-structural", inner, residual, "--max-output", str(limit)],
            check=True,
        )
        restored = transform(bounded_read(residual, limit), lag, inverse=True)
        if zlib.crc32(restored) != crc:
            raise ValueError("original checksum mismatch")
        with output.open("xb") as destination:
            destination.write(restored)


def measure(directory, codec, operation, source, output, parameter, *, runner=__file__):
    timing = directory / "timing"
    stdout = directory / "stdout"
    subprocess.run(
        [
            sys.executable, Path(__file__).with_name("measure.py"),
            "--timing", timing, "--stdout", stdout, sys.executable, runner,
            operation, codec, source, output, str(parameter),
        ],
        check=True,
    )
    return timing.read_text(encoding="ascii").strip().split("\t")


def screen(codec, dataset):
    print(
        "file\tsha256\tinput_bytes\tlag\tartifact_bytes\tencode_seconds\t"
        "encode_rss_kib\tdecode_seconds\tdecode_rss_kib\texact",
        flush=True,
    )
    for source in sorted(dataset.iterdir()):
        original = bounded_read(source, LIMIT)
        digest = hashlib.sha256(original).hexdigest()
        for lag in LAGS:
            with TemporaryDirectory() as directory:
                directory = Path(directory)
                artifact, restored = directory / "artifact", directory / "restored"
                enc = measure(directory, codec, "encode", source, artifact, lag)
                dec = measure(directory, codec, "decode", artifact, restored, len(original))
                if bounded_read(restored, len(original)) != original:
                    raise ValueError(f"round-trip mismatch for {source.name} at lag {lag}")
                row = (
                    source.name, digest, len(original), lag, artifact.stat().st_size,
                    *enc, *dec, "true",
                )
                print(*row, sep="\t", flush=True)


def main():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="operation", required=True)
    for name in ("encode", "decode", "screen"):
        command = commands.add_parser(name)
        command.add_argument("codec", type=Path)
        command.add_argument("source", type=Path)
        if name != "screen":
            command.add_argument("output", type=Path)
            command.add_argument("parameter", type=int)
    args = parser.parse_args()
    if args.operation == "screen":
        screen(args.codec, args.source)
    elif args.operation == "encode" and args.parameter in LAGS:
        encode(args.codec, args.source, args.output, args.parameter)
    elif args.operation == "decode" and 0 <= args.parameter <= LIMIT:
        decode(args.codec, args.source, args.output, args.parameter)
    else:
        parser.error("unsupported lag or output limit")


if __name__ == "__main__":
    main()
