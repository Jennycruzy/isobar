# Handoff

Last checked: 2026-08-26 21:52 UTC.

## Current state

- Track 1 miner is deployed at `https://weather.isobars.xyz`.
- Remote `isobar.service` is enabled and active with zero restarts.
- Telegraph registration `224` is active; it is no longer pending.
- Epoch 284 baseline: `WEATHER_CHECK #6/9`, `WEATHER_FORECAST #9/11`.
- The primary failure was evaluator input incompatibility: Telegraph called
  `location` and `city`, while the old runtime only accepted `q`.
- The runtime now accepts those aliases, requests UTC/m/s data, and emits a
  concise reference-aligned WEATHER_CHECK answer plus structured fields. For
  bounded forecast requests it also echoes the requested 48-hour/hourly window
  and cutoff wording, while retaining the compact two-day/nearest-hour facts.
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
- Epoch 284 remains the pre-release baseline. Explorer reported the next epoch
  boundary at `2026-08-27T00:36:55Z`; do not claim rank 1 until that result is
  visible.

## Next work

- The alias/format release is deployed; validate its first complete live epoch
  after `2026-08-27T00:36:55Z`.
- Keep a per-epoch score log and change one scorer-facing variable at a time.
- The current local replica favors the reference-aligned WEATHER_CHECK answer
  over the 12-word compact variant; this is a proxy result, not a live rank.
- Run the in-repository Isobar Scorer harness against the captured weather
  corpus, then calibrate its champion margin and ordering from a live epoch.
- Start the Track 3 route agent under `app/` when the application window opens
  on Aug 31.

This handoff is intentionally local and should be committed with the next docs
checkpoint.
