#!/usr/bin/env python3
"""Produce FP32 Sentence-Transformer reference embeddings for parity checks."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import torch
from transformers import AutoModel, AutoTokenizer


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("text", nargs="+")
    args = parser.parse_args()

    torch.set_num_threads(1)
    tokenizer = AutoTokenizer.from_pretrained(args.model_dir, local_files_only=True)
    model = AutoModel.from_pretrained(args.model_dir, local_files_only=True)
    model.eval()
    batch = tokenizer(
        args.text,
        padding=True,
        truncation=True,
        max_length=256,
        return_tensors="pt",
    )
    mask = batch["attention_mask"].unsqueeze(-1).to(torch.float32)
    with torch.no_grad():
        hidden = model(**batch).last_hidden_state
        pooled = (hidden * mask).sum(dim=1) / mask.sum(dim=1).clamp_min(1.0e-9)
        embeddings = torch.nn.functional.normalize(pooled, p=2, dim=1)
    args.output.write_text(
        json.dumps(
            {
                "texts": args.text,
                "embeddings": embeddings.tolist(),
                "input_ids": batch["input_ids"].tolist(),
                "attention_mask": batch["attention_mask"].tolist(),
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
