# Handoff

Last checked: 2026-08-26.

## Current state

- Track 1 miner is deployed at `https://weather.isobars.xyz`.
- Remote `isobar.service` is enabled and active with zero restarts.
- Telegraph registration `224` is active; it is no longer pending.
- Epoch 284 baseline: `WEATHER_CHECK #6/9`, `WEATHER_FORECAST #9/11`.
- The primary failure was evaluator input incompatibility: Telegraph called
  `location` and `city`, while the old runtime only accepted `q`.
- The runtime now accepts those aliases, requests UTC/m/s data, and emits a
  concise reference-aligned WEATHER_CHECK answer plus structured fields. The
  forecast answer remains the compact two-day/nearest-hour experiment.
- The Track 2 scorer is now in `src/scorer/` and committed in `c7157b0`; its
  current 11-test native suite is green.
- Live scoring evidence is recorded in [`GROUND_TRUTH.md`](GROUND_TRUTH.md).

## Next work

- The alias/format release is deployed; validate its first complete live epoch.
- Keep a per-epoch score log and change one scorer-facing variable at a time.
- The current local replica favors the reference-aligned WEATHER_CHECK answer
  over the 12-word compact variant; this is a proxy result, not a live rank.
- Run the in-repository Track 2 scorer harness against the captured weather
  corpus, then calibrate its champion margin and ordering from a live epoch.
- Start the Track 3 route agent under `app/` when the application window opens
  on Aug 31.

This handoff is intentionally local and should be committed with the next docs
checkpoint.
