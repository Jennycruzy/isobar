# Handoff

Last checked: 2026-08-27 13:57 UTC.

## Current state

- Track 1 miner is deployed at `https://weather.isobars.xyz`.
- Remote `isobar.service` is enabled and active with zero restarts.
- Telegraph registration `224` is active; it is no longer pending.
- Epoch 284 baseline: `WEATHER_CHECK #6/9`, `WEATHER_FORECAST #9/11`.
- Current epoch-285 snapshot: `WEATHER_FORECAST #1`, score `0.008892506`.
  Explorer currently has no epoch-285 `WEATHER_CHECK` score row for Isobar.
- The live score is intent-specific and is not a promise of `0.019`. The prior
  `0.019` expectation was not supported by a live measurement; do not treat it
  as a guaranteed target.
- The primary failure was evaluator input incompatibility: Telegraph called
  `location` and `city`, while the old runtime only accepted `q`.
- The runtime now accepts those aliases, requests UTC/m/s data, and emits a
  concise reference-aligned WEATHER_CHECK answer plus structured fields. For
  bounded forecast requests it also echoes the requested date/time window and
  cutoff wording, while retaining compact daily and nearest-hour facts.
- Track 2 is the in-repository Isobar Scorer in `src/scorer/`; it is not a
  separate project. The scorer’s package/binary names were aligned to Isobar in
  `2ea0af7`. Its 13-test native suite, release WASM build, and 1,000-repeat
  wazero checks pass under both compiler and interpreter runtimes.
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
  are `2d2863b1a4cb82115ab63b0cfdcee496c8aedef0bf041a736cc0c66aaac7d814`
  (`weather.js`) and
  `e8474e4eaa6b3edf99129df3c9720ba9aab41e2b8a507d15030bf3529e74b744`
  (`server.js`). The exact Alexandria-shaped request returned HTTP 200 for
  Tokyo, Japan with seven daily dates and 168 hourly rows. `/health` remains
  HTTP 200 with `upstream_ok: true`.
- This is an integration smoke test, not a new Explorer score. Alexandria may
  route future requests to whichever matching miner currently ranks highest;
  check the provider/receipt to confirm an Isobar call.

## Next work

- Validate the next complete Explorer epoch and record both intent rows. A
  higher absolute score is useful, but rank and repeatable performance matter
  more than an unverified numeric target.
- Keep a per-epoch score log and change one scorer-facing variable at a time.
- The current local replica favors the reference-aligned WEATHER_CHECK answer
  over the 12-word compact variant; this is a proxy result, not a live rank.
- Run the in-repository Isobar Scorer harness against the captured weather
  corpus, then calibrate its champion margin and ordering from a live epoch.
- Start the Track 3 route agent under `app/` when the application window opens
  on Aug 31.

This handoff is intentionally local and should be committed with the next docs
checkpoint.
