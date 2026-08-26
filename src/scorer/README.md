# assay

Deterministic Telegraph answer-scoring module for `wasm32-unknown-unknown`.
This repository starts from an empty project and contains the first working
module, a native measurement harness, checked-in fixtures, and an Explorer
report scraper.

## Build

The toolchain is pinned in `rust-toolchain.toml`.

```bash
cargo test --features native-harness --all-targets
cargo build --release --target wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --features real_weights
```

The module is written to
`target/wasm32-unknown-unknown/release/assay.wasm`. The normal WASM build does
not compile the native harness, so the module has a single panic implementation
and can be loaded by a wazero host.

The checked-in Go host tester uses wazero `v1.11.0` to validate the binary's
exports, linear-memory writes, ABI calls, breakdown layout, score bounds, and
repeat-call bit identity:

```bash
cd go-tester
go test ./...
go run . \
  -wasm ../target/wasm32-unknown-unknown/release/assay.wasm \
  -runtime compiler -repeat 1000
```

Use `-runtime interpreter` or `-runtime both` when checking a small projection
build. The interpreter is substantially slower for the full six-layer
real-weights module; the production host target is the compiler runtime.

The current environment has the dependency cache available but not DNS access;
the equivalent offline checks are:

```bash
RUSTUP_TOOLCHAIN=stable cargo test --features native-harness --all-targets --offline
RUSTUP_TOOLCHAIN=stable cargo build --release --target wasm32-unknown-unknown --offline
```

## ABI

The exports use pointer/length pairs for UTF-8 byte strings:

```text
alloc(size: usize) -> *mut u8
dealloc(ptr: *mut u8, size: usize)
rank_answer(question_ptr, question_len,
            ground_truth_ptr, ground_truth_len,
            miner_answer_ptr, miner_answer_len) -> f32
breakdown_answer(same arguments) -> *mut Breakdown
```

`Breakdown` is six consecutive little-endian `f32` values:

```text
0  relevance       // cosine(question, miner_answer)
4  correctness     // cosine(ground_truth, miner_answer)
8  lexical         // normalized BM25(ground_truth, miner_answer)
12 length_quality
16 raw_score       // published baseline composite, before contrast
20 score           // public, contrast-enhanced score
```

The breakdown pointer is module-owned and remains valid until the next
`breakdown_answer` call. It must not be passed to `dealloc`. Input buffers can
be released after the scoring call. The allocator resets its arena after the
last active input allocation is released.

## Scoring

The raw composite starts with the published Telegraph baseline and is accumulated
in a fixed order:

```text
0.25 cosine(question, miner_answer)
0.50 cosine(ground_truth, miner_answer)
0.15 normalized BM25(ground_truth, miner_answer)
0.10 sigmoid((miner_answer.len() - 50) / 20)
```

For weather-shaped questions, the baseline value is then passed through the
bounded `weather::adjustment` layer before the contrast curve. The detector is
allocation-free and only pairs facts when their context keys match (with the
specified equal-count positional fallback); unmatched reference facts add no
penalty. It checks Celsius/Fahrenheit consistency, temperature, percentages,
wind speed and bearing, precipitation, condition polarity, and an explicit
city token from the question. Non-weather text takes the exact baseline path.

The public score applies an endpoint-pinned logistic transform, then clamps and
rounds to six decimal places. The current measured defaults are `K = 16.0`
and `C = 0.4`: on the expanded local corpus they retain zero quantized ties,
rank agreement `1.000000` with the independent baseline vector, and increase
the baseline-reference margin from `0.278296` to `0.538656`. These are local
measurements, not live-network guarantees. `libm::expf` is used explicitly.
The projection encoder, INT8 runtime, and all lexical structures use fixed
arrays; there is no `HashMap`, time, randomness, I/O, or ambient state in the
module.

The default encoder is a deterministic hashed projection intended for pipeline
development. The `real_weights` feature embeds
`weights/minilm_l6_v2_baseline.int8.bin`, a converted copy of Telegraph's
published quantized MiniLM-L6-v2 payload, uses the tracked WordPiece
vocabulary, and runs six transformer layers followed by masked mean pooling and
L2 normalization. Source commit, SHA-256, and conversion details are recorded
in `weights/telegraph_baseline_manifest.json`. The earlier Safetensors-derived
artifact remains tracked for FP32 parity experiments but is not the active
scoring payload.

The active baseline payload uses the published baseline's quantized tensor
scales, expanded into the fixed-array record format used by this module.
LayerNorm parameters remain f32. The source model files and temporary download
directory are ignored; the compact converted artifacts are tracked so a clean
clone can reproduce the real-weights build.

To recreate the active scoring payload from the published baseline repository:

```bash
git clone https://github.com/telegraphprotocol/telegraph-wasm-baseline.git /tmp/telegraph-wasm-baseline
python3 tools/convert_baseline_weights.py \
  --source /tmp/telegraph-wasm-baseline/weights/minilm_l6_v2_q8.bin \
  --output weights/minilm_l6_v2_baseline.int8.bin
```

The source commit and hashes must be updated in
`weights/telegraph_baseline_manifest.json` if a different revision is used.

To recreate the artifacts from the source model:

```bash
python3 tools/fetch_minilm.py \
  --revision 1110a243fdf4706b3f48f1d95db1a4f5529b4d41 \
  --output-dir vendor/minilm-l6-v2
python3 tools/convert_minilm.py \
  --source vendor/minilm-l6-v2/model.safetensors \
  --vocab vendor/minilm-l6-v2/vocab.txt \
  --weights-out weights/minilm_l6_v2.int8.bin \
  --vocab-out weights/minilm_vocab.bin \
  --manifest-out weights/minilm_manifest.json \
  --revision 1110a243fdf4706b3f48f1d95db1a4f5529b4d41
```

The reference parity helper requires the optional isolated Python tooling:

```bash
python3 tools/reference_minilm.py \
  --model-dir vendor/minilm-l6-v2 \
  --output /tmp/minilm-reference.json \
  "A deterministic sentence for the model."
cargo run --release --features 'native-harness real_weights' \
  --bin assay-embed-dump -- \
  "A deterministic sentence for the model."
```

The current four-sentence check gives cosine similarity between `0.9989` and
`0.9996` against the FP32 reference. Release-mode local CPU latency is roughly
`0.25` seconds per embedding on the development machine; the scorer calls the
encoder for the reference and candidate answer, so this must be benchmarked on
the target validator runtime before submission.

The latest native release measurement over 10 full scorer calls was p50
`1.310092606` seconds and p99 `2.041652218` seconds. This is end-to-end
real-weight latency, including transformer inference; the contrast layer is
only the final scalar transform. A VPS is useful for leaving the long
1,000-iteration determinism and host-runtime checks running, but is not needed
to build the module.

## Harness

Run the real-weights report with the checked-in data:

```bash
RUSTUP_TOOLCHAIN=stable cargo run --release --offline \
  --features 'native-harness real_weights' --bin assay-harness -- \
  --raw-cache /tmp/assay-raw.tsv
```

It prints self-match, average margin, ordering, rank agreement against the
independent score vector in `data/traffic.baseline.tsv`, ties, p50/p99 latency,
and the baseline-reference comparison. The process exits non-zero if the
configured thresholds are not met. That score vector was emitted by the
unmodified Telegraph baseline at the commit recorded in its manifest; it is
not `assay`'s own raw output.

The checked-in 144-case corpus is split into 112 fit rows and 32 holdout rows.
The current `K=16,C=0.4` result is:

```text
full:    margin 0.538656, ordering 131/144, agreement 1.000000, ties 0
fit:     margin 0.532802, ordering 101/112, agreement 1.000000, ties 0
holdout: margin 0.559144, ordering 30/32,  agreement 1.000000, ties 0
```

The default local baseline-reference margin is `0.278296`; it is a proxy for
the published scorer, not the live champion's current epoch-relative margin.

The default baseline-reference margin is measured from the checked-in fixture
score vector. A current
Explorer-derived bar can be supplied explicitly:

```bash
cargo run --features native-harness --bin assay-harness -- \
  --champion-margin 0.68037784 --champion-ordering 131 \
  --fixtures data/fixtures.tsv --traffic data/traffic.tsv
```

The value above is only an epoch-271 ACADEMIC_SEARCH snapshot retained as an
example of the CLI override. Refresh the current Intent-specific margin and
ordering from the Explorer before using the report for a registration
decision.

Sweep the contrast curve and inspect the margin/agreement frontier:

```bash
RUSTUP_TOOLCHAIN=stable cargo run --release \
  --features 'native-harness real_weights' --offline \
  --bin assay-harness -- --sweep --sweep-k-max 96 \
  --sweep-centre-max 0.9 --raw-cache /tmp/assay-raw.tsv \
  > /tmp/assay-sweep.csv
python3 tools/select_frontier.py \
  --sweep /tmp/assay-sweep.csv --min-agreement 0.95 \
  --min-ordering 131 --max-ties 0
```

The sweep caches raw scores, so the transformer runs once per corpus item and
each grid point measures only the contrast curve and rank statistics.

Use `--fixtures data/fixtures-fit.tsv` or
`--fixtures data/fixtures-holdout.tsv` with a matching raw cache to evaluate
fit and holdout separately. The independent champion scores can be replaced
only explicitly with `--champion-scores PATH`; omitting that option uses the
checked-in published-baseline vector.

Fixture rows are four tab-separated fields:
`question`, `ground_truth`, `good_answer`, `bad_answer`.
`data/fixtures.baseline.tsv` and its fit/holdout variants contain the matching
unmodified-baseline scores used for fixture ordering and margin comparison.
Traffic rows are four tab-separated fields:
`id`, `question`, `ground_truth`, `miner_answer`.

## Explorer report input

`tools/scrape_explorer.py` is a read-only scraper for saved Explorer HTML or a
reachable page. It extracts registration IDs, intent labels, displayed margins,
agreement values, and rejection text into JSON for later harness calibration.
The Explorer is client-rendered in some deployments; in that case save the
rendered Failed-tab response and pass the file path instead of assuming that a
simple HTTP fetch contains the table.

```bash
python3 tools/scrape_explorer.py \
  --source /path/to/failed-tab.html \
  --output data/explorer-failed.json
```

For the structured live registry, point it at the read-only API; the scraper
normalizes champion margins, eval scores, statuses, rejection reasons, and
per-Intent counts:

```bash
python3 tools/scrape_explorer.py \
  --source 'https://explorer.telegraphprotocol.com/api/wasm?limit=500' \
  --output data/explorer-wasm.json
```

No current live-network claim is made by the checked-in sample data. The
real-traffic collapse and the current champion bar must be re-established from
the latest target-Intent evidence before registration.
