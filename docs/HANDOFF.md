# Handoff

Last checked: 2026-08-26.

## Current state

- Track 1 miner is deployed at `https://weather.isobars.xyz`.
- Remote `isobar.service` is enabled and active with zero restarts.
- Telegraph registration `224` is active; it is no longer pending.
- Epoch 284 baseline: `WEATHER_CHECK #6/9`, `WEATHER_FORECAST #9/11`.
- The primary failure was evaluator input incompatibility: Telegraph called
  `location` and `city`, while the old runtime only accepted `q`.
- The runtime now accepts those aliases, requests UTC/m/s data, and emits
  compact scorer-facing answers while retaining detailed structured fields.
- The Track 2 scorer is now in `src/scorer/` and committed in `c7157b0`; its
  current 11-test native suite is green.
- Live scoring evidence is recorded in [`GROUND_TRUTH.md`](GROUND_TRUTH.md).

## Next work

- Deploy the format/input-compatibility release and validate the next epoch.
- Keep a per-epoch score log and change one scorer-facing variable at a time.
- Run the in-repository Track 2 scorer harness against the captured weather
  corpus, then calibrate its champion margin and ordering from a live epoch.
- Start the Track 3 route agent under `app/` when the application window opens
  on Aug 31.

This handoff is intentionally local and should be committed with the next docs
checkpoint.
