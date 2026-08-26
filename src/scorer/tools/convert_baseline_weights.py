#!/usr/bin/env python3
"""Convert Telegraph's published MLM2 payload to assay's fixed record format."""

from __future__ import annotations

import argparse
import struct
from pathlib import Path

KIND_MATRIX = 1
KIND_VECTOR = 2
WEIGHT_MAGIC = b"ASAYWT1\0"


def convert(source: Path, output: Path) -> None:
    raw = source.read_bytes()
    cursor = 0
    if raw[cursor : cursor + 4] != b"MLM2":
        raise ValueError("source is not an MLM2 baseline payload")
    cursor += 4
    layers, hidden, heads, intermediate, vocab, positions = struct.unpack_from(
        "<6I", raw, cursor
    )
    cursor += 24
    if (layers, hidden, heads, intermediate, vocab, positions) != (
        6,
        384,
        12,
        1536,
        30522,
        128,
    ):
        raise ValueError("unexpected MiniLM dimensions in source payload")

    records: list[tuple[int, int, int, int, int, int, int]] = []
    payload = bytearray()

    def add_matrix(identifier: int, rows: int, columns: int, with_bias: bool) -> None:
        nonlocal cursor
        scale = struct.unpack_from("<f", raw, cursor)[0]
        cursor += 4
        count = rows * columns
        data_offset = len(payload)
        payload.extend(raw[cursor : cursor + count])
        cursor += count
        scale_offset = len(payload)
        payload.extend(struct.pack("<" + "f" * rows, *([scale] * rows)))
        bias_offset = 0
        if with_bias:
            bias_offset = len(payload)
            payload.extend(raw[cursor : cursor + rows * 4])
            cursor += rows * 4
        records.append(
            (
                identifier,
                KIND_MATRIX,
                rows,
                columns,
                data_offset,
                scale_offset,
                bias_offset,
            )
        )

    def add_vector(identifier: int, length: int) -> None:
        nonlocal cursor
        data_offset = len(payload)
        payload.extend(raw[cursor : cursor + length * 4])
        cursor += length * 4
        records.append((identifier, KIND_VECTOR, length, 1, data_offset, 0, 0))

    add_matrix(1, vocab, hidden, False)
    add_matrix(2, positions, hidden, False)
    add_matrix(3, 1, hidden, False)
    add_vector(4, hidden)
    add_vector(5, hidden)

    for layer in range(layers):
        base = 16 + layer * 12
        for part in range(4):
            add_matrix(base + part, hidden, hidden, True)
        add_vector(base + 6, hidden)
        add_vector(base + 7, hidden)
        add_matrix(base + 4, intermediate, hidden, True)
        add_matrix(base + 5, hidden, intermediate, True)
        add_vector(base + 8, hidden)
        add_vector(base + 9, hidden)

    if cursor != len(raw):
        raise ValueError(f"source payload has {len(raw) - cursor} trailing bytes")

    payload_base = 24 + 24 * len(records)
    output_bytes = bytearray(WEIGHT_MAGIC)
    output_bytes.extend(struct.pack("<4I", 1, hidden, layers, len(records)))
    for identifier, kind, rows, columns, data, scales, bias in records:
        output_bytes.extend(
            struct.pack(
                "<HBBIIIII",
                identifier,
                kind,
                0,
                rows,
                columns,
                payload_base + data,
                payload_base + scales if scales else 0,
                payload_base + bias if bias else 0,
            )
        )
    output_bytes.extend(payload)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(output_bytes)
    print(f"wrote {output} ({len(output_bytes)} bytes)")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    convert(args.source, args.output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
