#!/usr/bin/env python3
"""Screen reversible transforms around RFC 0034 streams on Linux."""

import argparse
import hashlib
from pathlib import Path
from shutil import copyfileobj
import struct
import subprocess
from tempfile import TemporaryDirectory
import zlib

from lag_xor import LIMIT, bounded_read, measure, transform as lag_xor


WIDTHS = (0, 2, 4, 8)
ROW_STRIDES = (216, 256, 304)
DELTA_PARAMETER_OFFSET = 100
DELTA_WIDTHS = tuple(DELTA_PARAMETER_OFFSET + width for width in WIDTHS[1:])
BLOCK_BYTES = 64 << 10
SCREEN_ARTIFACT_LIMIT = 32 << 20
FILE_ARTIFACT_LIMIT = 8 << 30
FILE_INPUT_LIMIT = (1 << 32) - 1
MAGIC = b"TBS1"
HEADER = struct.Struct("<4sBI")
ROW_MAGIC = b"TRX1"
DELTA_MAGIC = b"TDS1"
ROW_HEADER = struct.Struct("<4sHI")


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


def delta_shuffle(block, parameter, inverse=False):
    width = parameter - DELTA_PARAMETER_OFFSET
    if inverse:
        restored = transform(block, width, inverse=True)
        for i in range(width, len(restored)):
            restored[i] = (restored[i] + restored[i - width]) & 255
        return restored
    differences = bytearray(block)
    for i in range(width, len(block)):
        differences[i] = (block[i] - block[i - width]) & 255
    return transform(differences, width)


def transform_file(source, output, parameter, *, inverse=False, expected_crc=None):
    if parameter in ROW_STRIDES:
        convert = lag_xor
    elif parameter in DELTA_WIDTHS:
        convert = delta_shuffle
    else:
        convert = transform
    crc = 0
    with source.open("rb") as incoming, output.open("xb") as outgoing:
        while block := incoming.read(BLOCK_BYTES):
            shuffled = convert(block, parameter, inverse)
            crc = zlib.crc32(shuffled if inverse else block, crc)
            outgoing.write(shuffled)
    if expected_crc is not None and crc != expected_crc:
        raise ValueError("original checksum mismatch")
    return crc


def encode(codec, source, output, parameter):
    if source.stat().st_size > FILE_INPUT_LIMIT:
        raise ValueError("input exceeds the transform file limit")
    if parameter == 0:
        subprocess.run([codec, "encode-variable-order-repriced", source, output], check=True)
        return
    with TemporaryDirectory() as directory:
        shuffled = Path(directory) / "shuffled"
        inner = Path(directory) / "inner"
        crc = transform_file(source, shuffled, parameter)
        if parameter in DELTA_WIDTHS:
            header, magic, field = HEADER, DELTA_MAGIC, parameter - DELTA_PARAMETER_OFFSET
        elif parameter in ROW_STRIDES:
            header, magic, field = ROW_HEADER, ROW_MAGIC, parameter
        else:
            header, magic, field = HEADER, MAGIC, parameter
        subprocess.run([codec, "encode-variable-order-repriced", shuffled, inner], check=True)
        if inner.stat().st_size > FILE_ARTIFACT_LIMIT - header.size:
            raise ValueError("artifact exceeds the transform file limit")
        with inner.open("rb") as payload, output.open("xb") as artifact:
            artifact.write(header.pack(magic, field, crc))
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
        raise ValueError("artifact exceeds the transform file limit")
    with TemporaryDirectory() as directory:
        inner = Path(directory) / "inner"
        shuffled = Path(directory) / "shuffled"
        restored = Path(directory) / "restored"
        with source.open("rb") as artifact:
            if magic in (MAGIC, DELTA_MAGIC):
                header, supported = HEADER, WIDTHS[1:]
            elif magic == ROW_MAGIC:
                header, supported = ROW_HEADER, ROW_STRIDES
            else:
                raise ValueError("unknown transform wrapper magic")
            data = artifact.read(header.size)
            if len(data) != header.size:
                raise ValueError("truncated transform wrapper")
            header_magic, parameter, crc = header.unpack(data)
            if header_magic != magic or parameter not in supported:
                raise ValueError("invalid transform wrapper parameter")
            with inner.open("xb") as payload:
                copyfileobj(artifact, payload, BLOCK_BYTES)
        decode_inner(codec, inner, shuffled, limit)
        if magic == DELTA_MAGIC:
            parameter += DELTA_PARAMETER_OFFSET
        transform_file(shuffled, restored, parameter, inverse=True, expected_crc=crc)
        with restored.open("rb") as validated, output.open("xb") as destination:
            copyfileobj(validated, destination, BLOCK_BYTES)


def screen(codec, dataset, parameters=WIDTHS):
    print(
        "file\tinput_sha256\tinput_bytes\twidth\tartifact_bytes\tartifact_sha256\t"
        "encode_seconds\tencode_rss_kib\tdecode_seconds\tdecode_rss_kib\texact",
        flush=True,
    )
    for source in sorted(dataset.iterdir()):
        original = bounded_read(source, LIMIT)
        digest = hashlib.sha256(original).hexdigest()
        for parameter in parameters:
            with TemporaryDirectory() as directory:
                directory = Path(directory)
                artifact, restored = directory / "artifact", directory / "restored"
                enc = measure(directory, codec, "encode", source, artifact, parameter, runner=__file__)
                dec = measure(
                    directory, codec, "decode", artifact, restored, len(original), runner=__file__
                )
                if bounded_read(restored, len(original)) != original:
                    raise ValueError(f"round-trip mismatch for {source.name} at parameter {parameter}")
                print(
                    source.name, digest, len(original), parameter, artifact.stat().st_size,
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
        if name == "screen":
            profiles = command.add_mutually_exclusive_group()
            profiles.add_argument("--row-xor", action="store_true")
            profiles.add_argument("--delta-shuffle", action="store_true")
        else:
            command.add_argument("output", type=Path)
            command.add_argument("parameter", type=int)
    args = parser.parse_args()
    if args.operation == "screen":
        parameters = WIDTHS
        if args.row_xor:
            parameters += ROW_STRIDES
        elif args.delta_shuffle:
            parameters += DELTA_WIDTHS
        screen(args.codec, args.source, parameters)
    elif args.operation == "encode" and args.parameter in WIDTHS + ROW_STRIDES + DELTA_WIDTHS:
        encode(args.codec, args.source, args.output, args.parameter)
    elif args.operation == "decode" and 0 <= args.parameter <= FILE_INPUT_LIMIT:
        decode(args.codec, args.source, args.output, args.parameter)
    else:
        parser.error("unsupported transform parameter or output limit")


if __name__ == "__main__":
    main()
