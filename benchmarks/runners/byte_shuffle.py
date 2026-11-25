#!/usr/bin/env python3
"""Screen RFC 0038 byte shuffle wrappers around RFC 0034 streams on Linux."""

import argparse
import hashlib
from pathlib import Path
from shutil import copyfileobj
import struct
import subprocess
from tempfile import TemporaryDirectory
import zlib

from lag_xor import LIMIT, bounded_read, measure


WIDTHS = (0, 2, 4, 8)
BLOCK_BYTES = 64 << 10
SCREEN_ARTIFACT_LIMIT = 32 << 20
FILE_ARTIFACT_LIMIT = 8 << 30
FILE_INPUT_LIMIT = (1 << 32) - 1
MAGIC = b"TBS1"
HEADER = struct.Struct("<4sBI")


def transform(data, width, inverse=False):
    result = bytearray(len(data))
    for start in range(0, len(data), BLOCK_BYTES):
        end = min(start + BLOCK_BYTES, len(data))
        cursor = start
        for lane in range(width):
            count = (end - start + width - 1 - lane) // width
            if inverse:
                result[start + lane:end:width] = data[cursor:cursor + count]
            else:
                result[cursor:cursor + count] = data[start + lane:end:width]
            cursor += count
    return result


def shuffle_file(source, output, width, *, inverse=False, expected_crc=None):
    crc = 0
    with source.open("rb") as incoming, output.open("xb") as outgoing:
        while block := incoming.read(BLOCK_BYTES):
            shuffled = transform(block, width, inverse)
            crc = zlib.crc32(shuffled if inverse else block, crc)
            outgoing.write(shuffled)
    if expected_crc is not None and crc != expected_crc:
        raise ValueError("original checksum mismatch")
    return crc


def encode(codec, source, output, width):
    if source.stat().st_size > FILE_INPUT_LIMIT:
        raise ValueError("input exceeds the byte shuffle file limit")
    if width == 0:
        subprocess.run([codec, "encode-variable-order-repriced", source, output], check=True)
        return
    with TemporaryDirectory() as directory:
        shuffled = Path(directory) / "shuffled"
        inner = Path(directory) / "inner"
        crc = shuffle_file(source, shuffled, width)
        subprocess.run([codec, "encode-variable-order-repriced", shuffled, inner], check=True)
        if inner.stat().st_size > FILE_ARTIFACT_LIMIT - HEADER.size:
            raise ValueError("artifact exceeds the byte shuffle file limit")
        with inner.open("rb") as payload, output.open("xb") as artifact:
            artifact.write(HEADER.pack(MAGIC, width, crc))
            copyfileobj(payload, artifact, BLOCK_BYTES)


def decode_inner(codec, source, output, limit):
    subprocess.run(
        [codec, "decode-variable-order", source, output, "--max-output", str(limit)],
        check=True,
    )


def decode(codec, source, output, limit):
    with source.open("rb") as artifact:
        magic = artifact.read(4)
    if magic == b"TK32":
        decode_inner(codec, source, output, limit)
        return
    if source.stat().st_size > FILE_ARTIFACT_LIMIT:
        raise ValueError("artifact exceeds the byte shuffle file limit")
    with TemporaryDirectory() as directory:
        inner = Path(directory) / "inner"
        shuffled = Path(directory) / "shuffled"
        restored = Path(directory) / "restored"
        with source.open("rb") as artifact:
            header = artifact.read(HEADER.size)
            if len(header) < HEADER.size:
                raise ValueError("truncated byte shuffle wrapper")
            magic, width, crc = HEADER.unpack(header)
            if magic != MAGIC or width not in WIDTHS[1:]:
                raise ValueError("invalid byte shuffle magic or width")
            with inner.open("xb") as payload:
                copyfileobj(artifact, payload, BLOCK_BYTES)
        decode_inner(codec, inner, shuffled, limit)
        shuffle_file(shuffled, restored, width, inverse=True, expected_crc=crc)
        with restored.open("rb") as validated, output.open("xb") as destination:
            copyfileobj(validated, destination, BLOCK_BYTES)


def screen(codec, dataset):
    print(
        "file\tinput_sha256\tinput_bytes\twidth\tartifact_bytes\tartifact_sha256\t"
        "encode_seconds\tencode_rss_kib\tdecode_seconds\tdecode_rss_kib\texact",
        flush=True,
    )
    for source in sorted(dataset.iterdir()):
        original = bounded_read(source, LIMIT)
        digest = hashlib.sha256(original).hexdigest()
        for width in WIDTHS:
            with TemporaryDirectory() as directory:
                directory = Path(directory)
                artifact, restored = directory / "artifact", directory / "restored"
                enc = measure(directory, codec, "encode", source, artifact, width, runner=__file__)
                dec = measure(
                    directory, codec, "decode", artifact, restored, len(original), runner=__file__
                )
                if bounded_read(restored, len(original)) != original:
                    raise ValueError(f"round-trip mismatch for {source.name} at width {width}")
                print(
                    source.name, digest, len(original), width, artifact.stat().st_size,
                    hashlib.sha256(bounded_read(artifact, SCREEN_ARTIFACT_LIMIT)).hexdigest(),
                    *enc, *dec, "true", sep="\t", flush=True,
                )


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
    elif args.operation == "encode" and args.parameter in WIDTHS:
        encode(args.codec, args.source, args.output, args.parameter)
    elif args.operation == "decode" and 0 <= args.parameter <= FILE_INPUT_LIMIT:
        decode(args.codec, args.source, args.output, args.parameter)
    else:
        parser.error("unsupported width or output limit")


if __name__ == "__main__":
    main()
