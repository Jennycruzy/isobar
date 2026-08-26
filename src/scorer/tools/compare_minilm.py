#!/usr/bin/env python3
"""Compare Rust embedding-dump output with reference JSON embeddings."""

from __future__ import annotations

import argparse
import json
import math
import struct
from pathlib import Path


def bits_to_float(value: str) -> float:
    return struct.unpack("<f", struct.pack("<I", int(value)))[0]


def cosine(left: list[float], right: list[float]) -> float:
    dot = sum(a * b for a, b in zip(left, right))
    left_norm = math.sqrt(sum(value * value for value in left))
    right_norm = math.sqrt(sum(value * value for value in right))
    return dot / (left_norm * right_norm)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--rust", type=Path, required=True)
    parser.add_argument("--min-cosine", type=float, default=0.998)
    args = parser.parse_args()

    reference = json.loads(args.reference.read_text(encoding="utf-8"))["embeddings"]
    rust = []
    for line in args.rust.read_text(encoding="utf-8").splitlines():
        fields = line.split("\t")
        rust.append([bits_to_float(value) for value in fields[1:]])
    if len(reference) != len(rust):
        raise SystemExit(f"row count mismatch: {len(reference)} != {len(rust)}")
    scores = [cosine(left, right) for left, right in zip(reference, rust)]
    print("cosines", " ".join(f"{score:.9f}" for score in scores))
    print("minimum", f"{min(scores):.9f}")
    return 0 if min(scores) >= args.min_cosine else 1


if __name__ == "__main__":
    raise SystemExit(main())
