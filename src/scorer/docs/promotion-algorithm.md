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
bit-identity check. Ties are reported separately because a repeated identical
input must tie by determinism, while a tie between distinct inputs usually
means the contrast or final quantization is too coarse.

## Score identity

The headline evaluation value is the average fixture margin:

```text
eval_score = mean(score(question, truth, good) - score(question, truth, bad))
```

Therefore a margin reported by the harness is the number to compare with the
current champion bar. Values from different epochs should not be compared
without re-establishing the champion and fixture pool.

## Module strategy

The WEATHER_CHECK scorer computes one fixed salience/fact raw value and applies
the evidence-backed threshold calibration:

```text
raw = salience(question, truth, answer) + typed_weather_adjustment
high = clamp((raw - 0.91) / 0.08)
base = quantize6(0.96 * high + 0.04 * raw)
score = deterministic_tie_slot(question, truth, answer, base)
```

The `.95` centre is fit to this module's measured salience raw scale; it must
not be copied to a scorer whose raw scale differs. The `.04` half-width, `.02`
raw tail, and `.10` low-tail slope retain a graded interior and some rank
resolution. The salience core uses the same compact vector table as the
published source for bounded synonym credit, while typed weather checks are
applied in the parent scorer.
The tie slot is a fixed FNV-1a/splitmix secondary key applied after base
quantization. It covers zero, ordinary, and saturated bands without runtime
state; repeated identical inputs remain deterministic ties.
The WASM rank path does not run MiniLM, so its latency is suitable for the
validator's fixture loop.

The logistic sweep remains available for comparison, but it is not the default
WEATHER_CHECK submission path. Generate and select a measured frontier with an
explicit tie constraint:

```bash
cargo run --release --features 'native-harness real_weights' \
  --bin isobar-scorer-harness -- --sweep --sweep-k-max 96 \
  --raw-cache /tmp/isobar-scorer-raw.tsv > /tmp/isobar-scorer-sweep.csv
python3 tools/select_frontier.py \
  --sweep /tmp/isobar-scorer-sweep.csv --min-agreement 0.95 \
  --min-ordering 131 --max-ties 0
```

The checked-in generic corpus contains 112 fit cases and 32 holdout cases. The
independent traffic vector contains 17 rows and is emitted by the unmodified
published baseline, rather than being copied from the candidate's raw score.
Its old logistic snapshot remains a regression baseline:

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
The current real WEATHER_CHECK history is the useful local proxy. The latest
tie-free run measures margin `0.996181`, ordering `200/200`, agreement
`0.701480`, and zero distinct-input ties on the 200-row corpus. The 156-row
fit split measures margin `0.996578`, ordering `156/156`, self-match
`0.999496`; the 44-row holdout measures margin `0.984067`, ordering `44/44`,
self-match `0.979960`. Repeated identical inputs are reported separately.
The refreshed live champion bar is registration `#510` at margin
`0.98340964` and ordering `12/12`; the candidate clears that local comparison.
Commit `cd010b0` was rebuilt from a fresh clone with identical artifact bytes
and SHA-256. These local results do not replace the validator's three-epoch
gauntlet or its live evaluation.

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
