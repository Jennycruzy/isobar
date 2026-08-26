# MiniLM runtime artifacts

The active `real_weights` build uses the baseline-compatible artifact, not the
original Safetensors-derived artifact:

- `minilm_l6_v2_baseline.int8.bin` — Telegraph's published quantized
  MiniLM-L6-v2 payload converted to Isobar's fixed-array v1 format.
- `telegraph_baseline_manifest.json` — source commit, source hash, artifact
  hash, and conversion details.

These compact artifacts are generated from the Apache-2.0
`sentence-transformers/all-MiniLM-L6-v2` model at revision
`1110a243fdf4706b3f48f1d95db1a4f5529b4d41`.

- `minilm_l6_v2.int8.bin` — retained Safetensors-derived six-layer transformer
  for FP32 parity experiments; it is not embedded by the scorer.
- `minilm_vocab.bin` — 30,522-entry WordPiece lookup table.
- `minilm_manifest.json` — source and generated SHA-256 values plus tensor
  layout.

The original 90.9 MB Safetensors file belongs in the ignored `vendor/` folder
and can be recreated with `tools/fetch_minilm.py`. Do not replace these files
with a different model revision without regenerating and reviewing the
manifest.
