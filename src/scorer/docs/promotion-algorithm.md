# Promotion algorithm notes

This document records the promotion-gate model used by the local harness. The
thresholds and rejection examples below come from the evidence supplied with
this build, not from the whitepaper's catch-rate description. Live values are
epoch-relative and must be refreshed before registration.

## Observed gate sequence

| Gate | Requirement | Harness measurement |
| --- | --- | --- |
| 1 | The WASM module loads successfully | wazero host tester |
| 2 | Every self-match is at least `0.75` | minimum `score(gt, gt)` |
| 3 | Average good-minus-bad margin is at least `0.15` | fixture mean |
| 4 | Good answers are ordered above bad answers at least as often as the champion | fixture ordering count |
| 5 | Rank agreement with the champion is at least `0.60` | Spearman correlation on traffic |
| 6 | Average margin is strictly greater than the current champion's | champion-margin comparison |

The local report also prints ties, p50/p99 latency, and a 1,000-iteration
bit-identity check. Ties are not a separate threshold, but they can change
rank agreement and fixture ordering after the final six-decimal quantization.

## Score identity

The headline evaluation value is the average fixture margin:

```text
eval_score = mean(score(question, truth, good) - score(question, truth, bad))
```

Therefore a margin reported by the harness is the number to compare with the
current champion bar. Values from different epochs should not be compared
without re-establishing the champion and fixture pool.

## Module strategy

The scorer computes one fixed raw composite and applies an endpoint-pinned,
strictly increasing logistic transform:

```text
raw = baseline_composite(question, truth, answer)
score = quantize6(contrast_norm(raw, K, C))
```

Because the transform is monotonic, it preserves the baseline ordering before
quantization. The harness measures ties after quantization so a steep curve is
accepted only when it retains useful rank resolution.

Generate and select a measured frontier with an explicit tie constraint:

```bash
cargo run --release --features 'native-harness real_weights' \
  --bin assay-harness -- --sweep --sweep-k-max 96 \
  --raw-cache /tmp/assay-raw.tsv > /tmp/assay-sweep.csv
python3 tools/select_frontier.py \
  --sweep /tmp/assay-sweep.csv --min-agreement 0.95 \
  --min-ordering 131 --max-ties 0
```

The checked-in corpus contains 112 fit cases and 32 holdout cases. The
independent traffic vector contains 17 rows and is emitted by the unmodified
published baseline, rather than being copied from the candidate's raw score.
On the combined 144-case corpus, the selected no-tie point is:

```text
K=16.0, C=0.4
margin       0.538656
self-match   0.996657
ordering     131/144
agreement    1.000000
ties         0
```

The fit split measures margin `0.532802`, ordering `101/112`, and zero ties;
the holdout split measures margin `0.559144`, ordering `30/32`, and zero ties.
These results establish agreement with the local baseline proxy only. They do
not establish that the live champion has the same traffic scores or fixture
set. The curve is therefore intentionally moderate; a higher-K point may look
better on margin while creating thousands of quantized ties.

## Epoch-271 calibration snapshot

An earlier read-only Explorer API snapshot reported epoch `271` and an
ACADEMIC_SEARCH displayed eval score of `0.68037784` for registration `#688`;
the embedded evaluation record showed a different previous-champion margin.
Those values are epoch-relative and are retained only as an audit trail for the
CLI override example. Refresh `/api/epoch`, `/api/wasm`, and the target Intent's
latest rejection evidence immediately before registration. A local baseline
proxy margin must never be presented as the live champion bar.

## Known transfer failure

The supplied production observations show fixture margins near `0.92`–`0.96`
coexisting with factual miners scoring `0.000` on real traffic. Structured API
answers and natural-language references can have low embedding similarity even
when the factual answer is correct. Fixture separation therefore does not
establish real-traffic transfer.

## Blind spot

A monotonic transform inherits every ranking error in the baseline by design.
That is the cost of preserving agreement for Gate 5. Semantic corrections may
improve individual answers, but they can reorder miners and must be measured as
a separate experiment rather than silently added to the default scorer.
