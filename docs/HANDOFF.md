# Handoff

Last checked: 2026-08-28 01:36 UTC.

## 2026-08-28 Track 2 live-bar refresh

- The live `/api/wasm?limit=500` response contains 1,430 registrations. The
  active `WEATHER_CHECK` champion is registration `#510`, with eval score and
  champion margin `0.98340964`, ordering `12/12`, worst self-match
  `0.99863017`, and Spearman agreement `0.6128585`.
- Re-running the release harness against the captured 200-row Explorer corpus
  and the exact live bar produced zero distinct-input ties in every split:
  fit margin `0.996578`, ordering `156/156`; holdout margin `0.984067`,
  ordering `44/44`; full corpus margin `0.996181`, ordering `200/200`.
  Agreement is `0.701480`; repeated identical inputs remain intentionally
  tied (`879`, `940`, and `1434` respectively).
- All three runs passed 1,000 determinism iterations and the live-bar
  comparison (`0.996578 > 0.98340964`). The current WASM artifact is
  1,103,845 bytes with SHA-256
  `b8a920df89f38245daa82e91174db4467c4941fb2e619dd98fbcb81cee116b94`.
- A temporary clone of `HEAD` with the current working diff rebuilt a
  byte-identical artifact with the same SHA-256. A true clean-clone check of
  the final committed revision remains part of the handoff.
- Rust tests pass `21/21` with the production feature set and `22/22` with
  `real_weights`; formatting and `git diff --check` pass. The native host
  checks also pass. This is strong local evidence, not a registration
  guarantee: the final source is still uncommitted, so clean-clone
  reproduction remains outstanding and no WASM submission has been made.

## 2026-08-27 epoch 287 checkpoint

- Explorer now reports `current_epoch: 287`, with the next boundary at
  `2026-08-28T03:36:55Z`.
- Registration `#224` remains active. The public `/health` endpoint is HTTP
  200 with `upstream_ok: true`; no service restart issue was observed.
- Isobar `WEATHER_CHECK` is currently `#1/9` at `0.29039782`, scored at
  `2026-08-27T18:41:50.672985Z`. The next rows are `openweathermap`
  (`0.014834604`), `weatherapi` (`0.014801264`), and
  `verity-current-weather` (`0.014577433`).
- Epoch-287 `WEATHER_FORECAST` has no scored rows yet (`total: 0`).
- The exact epoch-287 question, ground truth, converted reference, and Isobar
  answer prefix are captured in [`GROUND_TRUTH.md`](GROUND_TRUTH.md).
- Track 2 is still unregistered. The working tree now contains the scorer
  parity, typed-fact, and deterministic tie-resolution changes. Fresh fit,
  holdout, and corpus runs report zero distinct-input ties; repeated identical
  inputs remain intentionally tied. No scorer submission has been made.

## 2026-08-27 Track 2 tie-resolution checkpoint (superseded by the 2026-08-28 refresh)

- The exported `rank_answer` path, `breakdown_answer` path, and native harness
  now share the same weather typed-fact adjustment and public-score path. This
  closes the earlier harness/ABI mismatch.
- Numeric context matching ignores nearby figures and typed-fact adjustments
  are penalty-only. The public score has deterministic FNV-1a/splitmix tie
  resolution for zero, ordinary, and saturated quantized bands; no time,
  randomness, I/O, or map iteration is involved.
- Against the 156-row fit, 44-row holdout, and 200-row Explorer WEATHER_CHECK
  corpus, distinct-input ties are `0` in all three runs. Repeated identical
  `(question, ground_truth, answer)` inputs remain tied by design.
- Full release harness results with 1,000 determinism iterations and 1,000
  latency samples: fit margin `0.939391`, ordering `156/156`, agreement
  `0.721230`; holdout margin `0.938643`, ordering `43/44`; corpus margin
  `0.994568`, ordering `200/200`. All three runs passed determinism. p99
  latency was `6.62 ms`, `3.00 ms`, and `4.97 ms`, respectively.
- Native Rust tests pass `18/18` for the production feature set and `19/19`
  with `real_weights`; the release WASM passes `1000` repeated calls
  under both wazero compiler and interpreter runtimes. Artifact SHA-256 is
  `56e2c01d4977be77601b168e7de82996cb5fa51262d8912ac6efc5674f2798c5`.
- These were local relative measurements before the live-bar refresh. Track 2
  remains unregistered until the artifact is committed and a clean-clone hash
  is reproduced.

## Current state

- Track 1 miner is deployed at `https://weather.isobars.xyz`.
- Remote `isobar.service` is enabled and active with zero restarts.
- Telegraph registration `224` is active; it is no longer pending.
- Epoch 284 baseline: `WEATHER_CHECK #6/9`, `WEATHER_FORECAST #9/11`.
- Epoch 286 is historical: `WEATHER_CHECK #1/9`, score `0.018964943`;
  `WEATHER_FORECAST #2/11`, score `0.0090172645`.
- Epoch 287 is the current measurement: `WEATHER_CHECK #1/9`, score
  `0.29039782`; no WEATHER_FORECAST row has been scored yet.
- Epoch-286 WEATHER_FORECAST leader is `verity-weather-forecast` at
  `0.0098297`; its question/reference is captured in [`GROUND_TRUTH.md`](GROUND_TRUTH.md).
- The live score is intent-specific and is not a promise of `0.019`. The prior
  `0.019` expectation was not supported by a live measurement; do not treat it
  as a guaranteed target.
- The primary failure was evaluator input incompatibility: Telegraph called
  `location` and `city`, while the old runtime only accepted `q`.
- The runtime now accepts those aliases, requests UTC/m/s data, and emits a
  concise reference-aligned WEATHER_CHECK answer plus structured fields. For
  bounded forecast requests it also echoes the requested date/time window and
  cutoff wording, while retaining compact daily and nearest-hour facts.
- Natural-language forecast prompts with a relative start or cutoff now echo
  that requested framing and only the requested field names before the
  available forecast; structured output remains complete.
- Track 2 is the in-repository Isobar Scorer in `src/scorer/`; it is not a
  separate project. The scorer’s package/binary names were aligned to Isobar in
  `2ea0af7`. Its 21-test production suite (22 with `real_weights`), release
  WASM build, and 1,000-repeat
  wazero checks pass under both compiler and interpreter runtimes. The current
  local WASM artifact is 1,103,845 bytes with SHA-256
  `b8a920df89f38245daa82e91174db4467c4941fb2e619dd98fbcb81cee116b94`.
- Live scoring evidence is recorded in [`GROUND_TRUTH.md`](GROUND_TRUTH.md).

## 2026-08-26 release checkpoint

- `7edb412`: evaluator aliases, UTC/m/s output, and compact forecast shape.
- `9d8e65f`: current WEATHER_CHECK answer adds feels-like and next-24-hour
  facts when hourly data is available.
- `03e35fb`: bounded forecast requests echo the evaluator’s exact horizon and
  cutoff framing; Node suite is 10/10.
- `2ea0af7`: package the scorer as the in-repository Isobar Scorer; no separate
  scorer project is part of this repository.
- The `03e35fb` weather handler is live on the VPS. Remote and local SHA-256:
  `91878ee36a61a0d099b2d58550316e3698e76c8b6b2e6b091e3b3713842edb97`.
- Post-restart service state: `active/running`, `NRestarts=0`, started at
  `2026-08-26 21:51:33 UTC`.
- Public `/health` is HTTP 200 with `upstream_ok: true`.
- The evaluator-shaped forecast request returns a country-qualified Tokyo
  result, the new horizon/cutoff sentence, and 48 hourly rows.
- Epoch 284 remains the pre-release baseline. An epoch-285 forecast snapshot is
  visible and currently places Isobar at `#1`; it was scored before the
  date-window patch was deployed and is not a post-fix measurement.

## 2026-08-27 score diagnosis and deployed contract fix

- The current epoch-285 score record asked for a 7-day hourly Tokyo forecast,
  with `precipitation probability` and `wind speed`, and used a cutoff time.
- The remote service journal captured the evaluator-shaped request as
  `/forecast?city=Tokyo&start_date=2026-09-03&end_date=2026-09-09&fields=temperature,precipitation_probability,wind_speed&interval=hourly`.
- Before this patch, the runtime ignored `start_date`/`end_date`,
  defaulted to two forecast days, and omitted hourly
  `precipitation_probability` from its Open-Meteo request. The scored response
  therefore contained only two daily dates and 48 hourly rows.
- The compatibility patch in `src/weather.js`:
  date aliases, inclusive date-window sizing up to seven days, the manifest's
  three-day default, hourly precipitation probability, and requested-field
  wording, passed the full Node suite (13/13 tests).
- Only `src/weather.js` was deployed at 2026-08-27 13:54 UTC. Local and remote
  SHA-256 are both
  `81820ee8ce76946ac82cbb8675425b723867b538a9d39080b423588af1d8470e`.
- The post-restart service is active with `NRestarts=0`. The exact evaluator
  request returned HTTP 200 with seven daily dates, 168 hourly rows, and
  hourly `precip_probability_pct` values; `/health` returned HTTP 200 with
  `upstream_ok: true`.
- The epoch-285 Explorer row was scored at 00:39 UTC, before this deployment,
  so its current rank-1 result is not evidence for the deployed patch.

## 2026-08-27 Alexandria natural-language prompt fix

- Alexandria forwarded the full question as
  `/weather?q=Give+me+a+7-day+hourly+weather+forecast+for+Tokyo%2C+Japan...`.
  The old runtime tried to geocode the entire sentence and returned the
  location-not-found response.
- `src/weather.js` now extracts the location and requested day count/fields
  from natural-language `q` values, and automatically serves a strong
  multi-day/hourly prompt as a forecast even when Alexandria selects `/weather`.
  The existing next-24-hour current-weather prompt remains on the current path.
- The full Node suite passes 13/13. `src/weather.js` and `src/server.js` were
  deployed at 2026-08-27 14:27:48 UTC. Their local and remote SHA-256 values
  were `2d2863b1a4cb82115ab63b0cfdcee496c8aedef0bf041a736cc0c66aaac7d814`
  (`weather.js`) and
  `e8474e4eaa6b3edf99129df3c9720ba9aab41e2b8a507d15030bf3529e74b744`
  (`server.js`). The exact Alexandria-shaped request returned HTTP 200 for
  Tokyo, Japan with seven daily dates and 168 hourly rows. `/health` remains
  HTTP 200 with `upstream_ok: true`.
- This is an integration smoke test, not a new Explorer score. Alexandria may
  route future requests to whichever matching miner currently ranks highest;
  check the provider/receipt to confirm an Isobar call.

## 2026-08-27 epoch-286 forecast experiment

- Live epoch 286 is currently `WEATHER_CHECK #1/9` (`0.018964943`) and
  `WEATHER_FORECAST #2/11` (`0.0090172645`). The forecast leader is
  `verity-weather-forecast` (`0.0098297`).
- The epoch-286 forecast row was scored before the latest forecast framing
  deployment. The deployed response now echoes the exact relative-start and
  cutoff language from the question and keeps the full forecast in JSON.
- `npm run check` passes 14/14. Production smoke test: HTTP 200, Tokyo/Japan,
  7 daily rows, 168 hourly rows; `/health` is HTTP 200 with `upstream_ok:true`.
  The current deployed `weather.js` SHA-256 is
  `a83208b5c7f3edec555ef9ca8ad39720a1ef86b484315a67684ecf81973978f8` on both
  local and remote hosts; the service is active with zero restarts.
- The hourly leader-shape experiment is now live: explicit hourly forecasts
  render dated daily summaries and full hourly rows with dew point and km/h
  wind, matching the current WEATHER_FORECAST leader's structure. Compact
  rendering remains for ordinary forecast calls.

## Next work

- Validate the next complete Explorer epoch and record both intent rows. A
  higher absolute score is useful, but rank and repeatable performance matter
  more than an unverified numeric target.
- Keep a per-epoch score log and change one scorer-facing variable at a time.
- The captured scorer corpus now has zero distinct-input ties and the live
  champion bar has been refreshed. Before registration, commit the final
  scorer change, reproduce the artifact from a clean clone, and rerun the full
  pre-flight for `WEATHER_CHECK` only.
- The current local replica favors the reference-aligned WEATHER_CHECK answer
  over the 12-word compact variant; this is a proxy result, not a live rank.
- The in-repository Isobar Scorer harness has been run against the captured
  weather corpus and calibrated against the refreshed live champion bar.
- Start the Track 3 route agent under `app/` when the application window opens
  on Aug 31.

This handoff is intentionally local and should be committed with the next docs
checkpoint.
