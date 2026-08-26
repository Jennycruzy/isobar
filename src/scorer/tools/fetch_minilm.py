#!/usr/bin/env python3
"""Fetch the pinned MiniLM source files used by the converter."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from urllib.request import Request, urlopen


FILES = (
    "config.json",
    "model.safetensors",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
    "vocab.txt",
    "modules.json",
    "sentence_bert_config.json",
    "1_Pooling/config.json",
)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--revision", required=True, help="full model commit SHA")
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()

    base = "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/"
    manifest: dict[str, object] = {"revision": args.revision, "files": {}}
    for filename in FILES:
        destination = args.output_dir / filename
        destination.parent.mkdir(parents=True, exist_ok=True)
        request = Request(base + args.revision + "/" + filename, headers={"User-Agent": "isobar-scorer-model-fetch/0.1"})
        payload = urlopen(request, timeout=120).read()
        destination.write_bytes(payload)
        manifest["files"][filename] = {
            "bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
        }
        print(filename, len(payload), manifest["files"][filename]["sha256"])
    (args.output_dir / "download_manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
