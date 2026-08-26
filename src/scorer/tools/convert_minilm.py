#!/usr/bin/env python3
"""Convert the pinned MiniLM Safetensors model to assay's flat INT8 format.

The converter intentionally uses only Python's standard library and NumPy.
Safetensors is parsed directly so the generated artifact does not depend on a
framework-specific serialization implementation. The output order, rounding,
record layout, and hash table size are fixed constants.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
from pathlib import Path

import numpy as np


WEIGHT_MAGIC = b"ASAYWT1\0"
VOCAB_MAGIC = b"ASAYVOC1"
VERSION = 1
HIDDEN = 384
LAYERS = 6
VOCAB_SLOTS = 1 << 16

KIND_MATRIX_INT8 = 1
KIND_VECTOR_F32 = 2

ID_WORD_EMBEDDINGS = 1
ID_POSITION_EMBEDDINGS = 2
ID_TOKEN_TYPE_EMBEDDINGS = 3
ID_EMBEDDING_LN_WEIGHT = 4
ID_EMBEDDING_LN_BIAS = 5
LAYER_BASE = 16
LAYER_STRIDE = 12


def layer_id(layer: int, part: int) -> int:
    return LAYER_BASE + layer * LAYER_STRIDE + part


def fnv_hash(value: bytes) -> int:
    result = 0xCBF29CE484222325
    for byte in value:
        result ^= byte
        result = (result * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    result ^= 0x9E3779B97F4A7C15
    return result or 1


def load_safetensors(path: Path) -> tuple[bytes, dict[str, object], int]:
    raw = path.read_bytes()
    if len(raw) < 8:
        raise ValueError("Safetensors file is too short")
    header_len = struct.unpack_from("<Q", raw, 0)[0]
    header_start = 8
    header_end = header_start + header_len
    if header_end > len(raw):
        raise ValueError("Safetensors header exceeds file size")
    header = json.loads(raw[header_start:header_end])
    return raw, header, header_end


def tensor(raw: bytes, header: dict[str, object], data_start: int, name: str) -> np.ndarray:
    metadata = header.get(name)
    if not isinstance(metadata, dict):
        raise KeyError(f"missing tensor: {name}")
    if metadata.get("dtype") != "F32":
        raise ValueError(f"{name} has dtype {metadata.get('dtype')}, expected F32")
    shape = tuple(int(value) for value in metadata["shape"])
    start, end = (int(value) for value in metadata["data_offsets"])
    return np.frombuffer(raw, dtype="<f4", count=(end - start) // 4, offset=data_start + start).reshape(shape)


def quantize_rows(array: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    rows = array.reshape(array.shape[0], -1).astype(np.float32, copy=False)
    scales = np.max(np.abs(rows), axis=1).astype(np.float32) / np.float32(127.0)
    scales[scales == 0.0] = np.float32(1.0 / 127.0)
    normalized = rows / scales[:, None]
    magnitude = np.floor(np.abs(normalized) + np.float32(0.5))
    quantized = np.sign(normalized) * magnitude
    quantized = np.clip(quantized, -127.0, 127.0).astype(np.int8)
    return quantized, scales


def f32_bytes(array: np.ndarray) -> bytes:
    return np.asarray(array, dtype="<f4").tobytes(order="C")


def add_matrix(records: list[dict[str, object]], name: str, bias_name: str | None, ident: int,
               raw: bytes, header: dict[str, object], data_start: int, payload: bytearray) -> None:
    array = tensor(raw, header, data_start, name)
    if array.ndim != 2:
        raise ValueError(f"{name} must be a matrix, got {array.shape}")
    quantized, scales = quantize_rows(array)
    data_offset = len(payload)
    payload.extend(quantized.tobytes(order="C"))
    scale_offset = len(payload)
    payload.extend(f32_bytes(scales))
    bias_offset = 0
    if bias_name is not None:
        bias = tensor(raw, header, data_start, bias_name)
        if bias.shape != (array.shape[0],):
            raise ValueError(f"{bias_name} shape {bias.shape} does not match {name}")
        bias_offset = len(payload)
        payload.extend(f32_bytes(bias))
    records.append({
        "id": ident,
        "kind": KIND_MATRIX_INT8,
        "rows": int(array.shape[0]),
        "cols": int(array.shape[1]),
        "data_offset": data_offset,
        "scale_offset": scale_offset,
        "bias_offset": bias_offset,
        "name": name,
    })


def add_vector(records: list[dict[str, object]], name: str, ident: int,
               raw: bytes, header: dict[str, object], data_start: int, payload: bytearray) -> None:
    array = tensor(raw, header, data_start, name)
    if array.ndim != 1:
        raise ValueError(f"{name} must be a vector, got {array.shape}")
    data_offset = len(payload)
    payload.extend(f32_bytes(array))
    records.append({
        "id": ident,
        "kind": KIND_VECTOR_F32,
        "rows": int(array.shape[0]),
        "cols": 1,
        "data_offset": data_offset,
        "scale_offset": 0,
        "bias_offset": 0,
        "name": name,
    })


def build_weights(source: Path, output: Path, revision: str) -> dict[str, object]:
    raw, header, data_start = load_safetensors(source)
    records: list[dict[str, object]] = []
    payload = bytearray()

    add_matrix(records, "embeddings.word_embeddings.weight", None, ID_WORD_EMBEDDINGS,
               raw, header, data_start, payload)
    add_matrix(records, "embeddings.position_embeddings.weight", None, ID_POSITION_EMBEDDINGS,
               raw, header, data_start, payload)
    add_matrix(records, "embeddings.token_type_embeddings.weight", None, ID_TOKEN_TYPE_EMBEDDINGS,
               raw, header, data_start, payload)
    add_vector(records, "embeddings.LayerNorm.weight", ID_EMBEDDING_LN_WEIGHT,
               raw, header, data_start, payload)
    add_vector(records, "embeddings.LayerNorm.bias", ID_EMBEDDING_LN_BIAS,
               raw, header, data_start, payload)

    parts = [
        ("attention.self.query", "query"),
        ("attention.self.key", "key"),
        ("attention.self.value", "value"),
        ("attention.output.dense", "attention_output"),
        ("intermediate.dense", "intermediate"),
        ("output.dense", "output"),
    ]
    for layer in range(LAYERS):
        prefix = f"encoder.layer.{layer}."
        for part_index, (part, _label) in enumerate(parts):
            add_matrix(
                records,
                prefix + part + ".weight",
                prefix + part + ".bias",
                layer_id(layer, part_index),
                raw,
                header,
                data_start,
                payload,
            )
        add_vector(records, prefix + "attention.output.LayerNorm.weight", layer_id(layer, 6),
                   raw, header, data_start, payload)
        add_vector(records, prefix + "attention.output.LayerNorm.bias", layer_id(layer, 7),
                   raw, header, data_start, payload)
        add_vector(records, prefix + "output.LayerNorm.weight", layer_id(layer, 8),
                   raw, header, data_start, payload)
        add_vector(records, prefix + "output.LayerNorm.bias", layer_id(layer, 9),
                   raw, header, data_start, payload)

    header_size = 24
    record_size = 24
    records_size = record_size * len(records)
    payload_base = header_size + records_size
    output_bytes = bytearray()
    output_bytes.extend(WEIGHT_MAGIC)
    output_bytes.extend(struct.pack("<IIII", VERSION, HIDDEN, LAYERS, len(records)))
    for record in records:
        data_offset = record["data_offset"] + payload_base
        scale_offset = record["scale_offset"] + payload_base if record["scale_offset"] else 0
        bias_offset = record["bias_offset"] + payload_base if record["bias_offset"] else 0
        output_bytes.extend(struct.pack(
            "<HBBIIIII",
            int(record["id"]),
            int(record["kind"]),
            0,
            int(record["rows"]),
            int(record["cols"]),
            data_offset,
            scale_offset,
            bias_offset,
        ))
    output_bytes.extend(payload)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(output_bytes)
    return {
        "format": "assay-minilm-int8-v1",
        "revision": revision,
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "weights_sha256": hashlib.sha256(output_bytes).hexdigest(),
        "weights_bytes": len(output_bytes),
        "tensor_count": len(records),
        "quantization": "per-output-row symmetric int8; round half away from zero; f32 scales and biases",
        "excluded_tensors": ["embeddings.position_ids", "pooler.dense.weight", "pooler.dense.bias"],
        "records": [{key: value for key, value in record.items() if key != "name"} | {"name": record["name"]} for record in records],
    }


def build_vocab(source: Path, output: Path) -> dict[str, object]:
    tokens = source.read_bytes().splitlines()
    if len(tokens) >= 1 << 32:
        raise ValueError("vocabulary is too large")
    entries = [(fnv_hash(token), index, token) for index, token in enumerate(tokens)]
    table: list[tuple[int, int, int, int] | None] = [None] * VOCAB_SLOTS
    token_bytes = bytearray()
    for token_hash, ident, token in entries:
        offset = len(token_bytes)
        token_bytes.extend(token)
        slot = token_hash % VOCAB_SLOTS
        while table[slot] is not None:
            slot = (slot + 1) % VOCAB_SLOTS
        table[slot] = (token_hash, ident, offset, len(token))

    header_size = 24
    record_size = 20
    data_offset = header_size + record_size * VOCAB_SLOTS
    output_bytes = bytearray()
    output_bytes.extend(VOCAB_MAGIC)
    output_bytes.extend(struct.pack("<III", VERSION, VOCAB_SLOTS, data_offset))
    output_bytes.extend(struct.pack("<I", len(tokens)))
    for entry in table:
        if entry is None:
            output_bytes.extend(struct.pack("<QIIHH", 0, 0, 0, 0, 0))
        else:
            token_hash, ident, offset, length = entry
            output_bytes.extend(struct.pack("<QIIHH", token_hash, ident, data_offset + offset, length, 0))
    output_bytes.extend(token_bytes)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(output_bytes)
    return {
        "format": "assay-wordpiece-v1",
        "vocab_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "artifact_sha256": hashlib.sha256(output_bytes).hexdigest(),
        "artifact_bytes": len(output_bytes),
        "vocab_size": len(tokens),
        "slots": VOCAB_SLOTS,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--vocab", type=Path, required=True)
    parser.add_argument("--weights-out", type=Path, required=True)
    parser.add_argument("--vocab-out", type=Path, required=True)
    parser.add_argument("--manifest-out", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    args = parser.parse_args()

    weights = build_weights(args.source, args.weights_out, args.revision)
    vocab = build_vocab(args.vocab, args.vocab_out)
    manifest = {"weights": weights, "vocab": vocab}
    args.manifest_out.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_out.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
