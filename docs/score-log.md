# Isobar score log

Scores and ranks are intent-specific. Values below are not comparable between
WEATHER_CHECK and WEATHER_FORECAST.

| Epoch / time | WEATHER_CHECK | WEATHER_FORECAST | Change / evidence |
|---|---:|---:|---|
| 284 baseline | #6/9, `0.013609781` | #9/11, `0.002964984` | Both validator requests used unsupported `location`/`city` aliases; captured in [`GROUND_TRUTH.md`](GROUND_TRUTH.md). |
| 2026-08-26 20:43 UTC | pending | pending | `7edb412`: alias normalization, UTC/m/s units, compact two-day forecast; deployed and smoke-tested. |
| 2026-08-26 21:19 UTC | pending | pending | `9d8e65f`: WEATHER_CHECK adds the requested feels-like and next-24-hour facts when hourly data is available; deployed separately. |
| 285 current snapshot | no Isobar row | #1, `0.008892506` | Current [Explorer score record](https://explorer.telegraphprotocol.com/api/scores?intent=WEATHER_FORECAST&epoch=285&limit=1). Epoch-285 WEATHER_CHECK has no Isobar row yet. |
| 2026-08-27 13:54 UTC | pending | pending | Deployed the validated `src/weather.js` date-window/field-contract fix; production smoke test passed. The epoch-285 row above was scored at 00:39 UTC, before deployment. |
| 286 current snapshot | #1/9, `0.018964943` | #2/11, `0.0090172645` | [Explorer epoch-286 rows](https://explorer.telegraphprotocol.com/api/scores?intent=WEATHER_FORECAST&epoch=286&limit=200): Isobar leads WEATHER_CHECK; `verity-weather-forecast` leads WEATHER_FORECAST at `0.0098297`. The Isobar forecast row was scored before the natural-language framing deployment. |
| 2026-08-27 14:31 UTC | unchanged | pending | Deployed the forecast-only natural-window framing experiment; exact epoch-286 prompt returns 7 daily and 168 hourly records. Awaiting a post-deployment score. |
| 2026-08-27 15:26 UTC | unchanged | pending | Deployed the hourly leader-shape experiment: dated daily summaries plus full `Hourly:` rows, dew point, precipitation `%/mm`, and km/h wind. Production smoke test passed; await the next Explorer row. |
| 287 current snapshot | #1/9, `0.29039782` | no row (`total: 0`) | Explorer now reports epoch 287. Isobar's WEATHER_CHECK row was scored at `2026-08-27T18:41:50.672985Z`; the next rows are `openweathermap` `0.014834604`, `weatherapi` `0.014801264`, and `verity-current-weather` `0.014577433`. |

Epoch 285 now has a forecast snapshot, but the next complete post-fix Explorer
epoch is the first measurement of the deployed date-window/field-contract fix.
The absolute value is not comparable across intents, and `0.019` is not a
verified guaranteed score.

## Local format probe

Using the in-repository real-weight scorer against the captured epoch-284
WEATHER_CHECK reference, with `K=16` and `C=0.4`:

| Candidate | Local score |
|---|---:|
| Epoch-284 invalid-location fallback | `0.018346` |
| Captured `weatherapi` answer | `0.961604` |
| 12-word compact answer | `0.772934` |
| Reference-aligned current + 24-hour answer | `0.997778` |

These are relative local measurements, not Explorer scores. They justify the
current format experiment but do not prove rank 1.

## Track 2 scorer calibration

The 200-row WEATHER_CHECK history captured from Explorer on 2026-08-27 was
converted into a four-column corpus. The live champion record available at the
same capture was registration `#510`, with champion margin `0.98340964` and
WEATHER_CHECK Spearman agreement `0.6128585`.

The release artifact built from this tree is `25,340,967` bytes with SHA-256
`6a05577caf9473e0c3bce214ba5c2d2b4b3a9d5f224d80f188a2e09da08118d6`.

The submitted lexical/fact candidate measured locally as follows:

| Corpus | Self-match | Margin | Ordering | Agreement | Distinct ties | Duplicate ties |
|---|---:|---:|---:|---:|---:|---:|
| 200-row Explorer history | `0.979851` | `0.994756` | `200/200` | `0.708337` | `17310` | `1434` |
| 156-row adversarial fit | `0.999603` | `0.590560` | `117/156` | `0.708337` | `9379` | `879` |
| 44-row adversarial holdout | `0.979851` | `0.793219` | `38/44` | `0.708337` | `8658` | `940` |

All three runs passed the 1,000-iteration bit-identity check. The history
margin is a local proxy: Explorer exposes the live score vector but not the
champion's hidden fixture pairs, so it cannot prove the validator's strict
separation gate. The non-zero tie counts are retained as an explicit
pre-registration issue, not reported as a zero-tie pass.

## Track 2 uncommitted scorer rerun — 2026-08-27

The working tree now contains a focused `src/scorer/src/weather.rs` change:
numeric tokens are ignored when building fact context keys, and typed-fact
terms are penalty-only so exact facts cannot offset a wrong-fact penalty. The
focused weather tests pass. These measurements are fresh local experiments;
the release artifact/hash above is still the last pushed artifact, and no WASM
registration has been submitted.

| Corpus | Self-match | Margin | Ordering | Agreement | Distinct ties | Duplicate ties |
|---|---:|---:|---:|---:|---:|---:|
| 156-row adversarial fit | `0.999603` | `0.939341` | `151/156` | `0.709362` | `8706` | `879` |
| 44-row adversarial holdout | `0.979851` | `0.938561` | `43/44` | `0.709362` | `8655` | `940` |
| 200-row corpus as traffic | `0.979851` | `0.994781` | `200/200` | `0.709362` | `17310` | `1434` |

The generated fixture ordering comparisons use a neutral baseline vector, so
they are diagnostic rather than proof of Gate 4. The agreement clears the
local floor, but the non-zero quantized ties still block registration.

The local replica is not calibrated enough to replace live ordering: the
deployed leader-shaped answer scored `0.507686` locally against the epoch-286
reference, while the captured `verity-weather-forecast` leader scored `0.539871`
and the old compact Isobar answer scored `0.982800`. Because that proxy orders
the old Isobar answer above the live leader, Explorer remains authoritative for
this experiment.

## Track 2 tie-resolution rerun — 2026-08-27 22:44 UTC

The scorer now applies the same weather-adjusted raw path through the exported
rank function, breakdown function, and native harness. The deterministic
secondary key covers zero, ordinary, and saturated six-decimal score bands.
Seeds and band widths are fixed in `src/scorer/src/scorer.rs`; they are not
runtime state.

| Corpus | Self-match | Margin | Ordering | Agreement | Distinct ties | Duplicate ties |
|---|---:|---:|---:|---:|---:|---:|
| 156-row adversarial fit | `0.999496` | `0.939391` | `156/156` | `0.721230` | `0` | `879` |
| 44-row adversarial holdout | `0.979960` | `0.938643` | `43/44` | `0.721230` | `0` | `940` |
| 200-row Explorer corpus as traffic | `0.979960` | `0.994568` | `200/200` | `0.721230` | `0` | `1434` |

All three release runs passed 1,000 determinism iterations. p50/p99 latency
was `0.581/6.62 ms` for fit, `0.417/3.00 ms` for holdout, and
`0.545/4.97 ms` for the corpus. Rust tests pass `18/18` for the production
feature set (`19/19` with `real_weights`); the release WASM
passed 1,000 repeated calls under both wazero runtimes. The locally rebuilt
WASM is 1,101,744 bytes with SHA-256
`56e2c01d4977be77601b168e7de82996cb5fa51262d8912ac6efc5674f2798c5`.

Distinct ties are resolved on the captured corpus, but this is not yet a
registration guarantee: the champion margin/order comparison must be refreshed
from the live WEATHER_CHECK epoch, and the final artifact still needs a clean
clone reproduction. No WASM registration has been submitted.

## Epoch-285 diagnosis

The captured live question requested a 7-day hourly Tokyo forecast with
temperature, precipitation probability, and wind speed. The old service
returned only two daily dates and 48 hourly rows because it ignored the
`start_date`/`end_date` aliases and omitted hourly precipitation probability
from its upstream request. The compatibility fix is deployed and verified; the
next complete Explorer epoch is the first post-deployment measurement. See
[`HANDOFF.md`](HANDOFF.md).

## Track 2 live-bar refresh — 2026-08-28 01:36 UTC

The live WASM list was refreshed from [`/api/wasm`](https://explorer.telegraphprotocol.com/api/wasm?limit=500).
The active `WEATHER_CHECK` champion remains registration `#510`, with eval
score/champion margin `0.98340964`, ordering `12/12`, worst self-match
`0.99863017`, and Spearman agreement `0.6128585`.

The current release candidate was measured against that bar using the captured
200-row Explorer corpus and its independent score vector:

| Corpus | Self-match | Margin | Ordering | Agreement | Distinct ties | Duplicate ties |
|---|---:|---:|---:|---:|---:|---:|
| 156-row adversarial fit | `0.999496` | `0.996578` | `156/156` | `0.701480` | `0` | `879` |
| 44-row adversarial holdout | `0.979960` | `0.984067` | `44/44` | `0.701480` | `0` | `940` |
| 200-row Explorer corpus as traffic | `0.979960` | `0.996181` | `200/200` | `0.701480` | `0` | `1434` |

All three release runs passed 1,000 determinism iterations and strict
separation against the live margin. The rebuilt WASM artifact is 1,103,845
bytes with SHA-256
`b8a920df89f38245daa82e91174db4467c4941fb2e619dd98fbcb81cee116b94`.
Commit `cd010b0` was rebuilt from a fresh clone and produced the same bytes and
SHA-256.
Distinct ties are zero only across this captured corpus; identical inputs must
remain tied. The candidate is not registered yet; the next step is the final
upload review and `WEATHER_CHECK`-only submission.
