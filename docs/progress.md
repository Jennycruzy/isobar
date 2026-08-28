# Progress log

## 2026-08-25

- Track 1 service implemented locally.
- Local unit/integration suite: 6/6 passing.
- Live Open-Meteo matrix passed for Springfield, Kraków, São Paulo, Zürich, nonexistent input, Ushuaia, Verkhoyansk, Death Valley, coordinates, and a 3-day forecast.
- Repeat Springfield calls within the cache window were byte-identical; `/health` returned `upstream_ok: true`.
- Live behavior note: Open-Meteo returns normalized English display names `Krakow` and `Zurich` for the accented queries, while preserving `São Paulo`.
- Production dependency audit: 0 known vulnerabilities after pinning Fastify 5.12.1.
- Deployed to the second Lightsail VPS: `54.154.121.30`, Ubuntu 24.04, eu-west-1a, `ubuntu` SSH user, systemd service account `isobar`.
- `isobar.service` is enabled and active; nginx is enabled on port 80; UFW allows 22/80/443.
- Public `http://54.154.121.30/health` returned HTTP 200 with `upstream_ok: true`; public `/weather?q=Gujranwala` returned live prose.
- Configured nginx for `weather.isobars.xyz` and installed Certbot plus the nginx plugin; the Host-header smoke test is green.
- Namecheap authoritative DNS now serves `A weather.isobars.xyz -> 54.154.121.30` with a 1800-second TTL.
- Public DNS now resolves `weather.isobars.xyz -> 54.154.121.30`.
- Let’s Encrypt certificate issued and installed successfully; renewal is scheduled and the certificate currently expires 2026-11-23.
- Lightsail TCP/443 ingress is now enabled; `https://weather.isobars.xyz/health` returns HTTP 200 with `upstream_ok: true` from outside the VPS.
- Final HTTPS smoke test passed for `https://weather.isobars.xyz/weather?q=Gujranwala`; certificate and nginx redirect are live.
- Telegraph registration now awaits only the wizard submission with a funded Base Sepolia wallet.
- Continued Track 2 preparation in `src/scorer/`: added bounded deterministic weather typed-fact adjustments, preserving the baseline path for non-weather text.
- The scorer is now part of the Isobar repository, with its lockfile, weights, corpus, native harness, and wazero tester committed in `c7157b0`.
- Scorer checks: 11 Rust tests pass; release WASM and 1,000-iteration host checks remain part of the deployment pre-flight.

## 2026-08-26 — score recovery checkpoint

- Explorer evidence for epoch 284 is captured in [`GROUND_TRUTH.md`](GROUND_TRUTH.md).
- Registration `#224` is active with zero fetch attempts/retries; the pre-fix ranks were WEATHER_CHECK `#6/9` and WEATHER_FORECAST `#9/11`.
- The validator used `location` and `city` query aliases that the old handler rejected. The compatibility release now normalizes those aliases, maps forecast time bounds, requests UTC and m/s units, and emits scorer-facing answers with the requested current/24-hour facts where hourly data is available.
- The compatibility release is tested locally and deployed; its score impact remains pending until Explorer reports a complete post-deployment epoch.

## 2026-08-27 — epoch 285 diagnosis and deployed contract fix

- Registration `#224` remains active. Explorer's current epoch-285 snapshot shows Isobar WEATHER_FORECAST at `#1` with score `0.008892506`; there is no current epoch-285 WEATHER_CHECK score row for Isobar.
- The numeric `0.019` expectation is not a verified live guarantee. Scores are intent-specific and vary by epoch; the next complete epoch is the measurement that matters.
- The remote journal captured a validator-shaped request using `city=Tokyo`, `start_date=2026-09-03`, `end_date=2026-09-09`, `fields=temperature,precipitation_probability,wind_speed`, and `interval=hourly`.
- The previous runtime ignored the date aliases, fetched its two-day default, and did not request hourly precipitation probability. This explains the mismatch between the requested 7-day hourly answer and the scored two-day/48-row response.
- The validated patch in `src/weather.js` accepts those aliases, sizes inclusive date windows through seven days, uses the manifest's three-day default when no days are supplied, requests hourly precipitation probability, and includes the requested fields in scorer-facing prose while preserving full JSON.
- Added regression coverage for the exact seven-day request; `npm test` passes 11/11.
- Deployed only `src/weather.js` at 2026-08-27 13:54 UTC. The remote SHA-256 matches local `81820ee8ce76946ac82cbb8675425b723867b538a9d39080b423588af1d8470e`; the service is active with zero restarts.
- The exact production request returned HTTP 200 with seven daily dates, 168 hourly rows, and hourly precipitation probabilities; `/health` returned `upstream_ok: true`.
- The epoch-285 forecast result (`#1`, `0.008892506`) was scored at 00:39 UTC, before this deployment; the next complete epoch is the first post-fix measurement.
- The root landing page remains live at `https://weather.isobars.xyz/`.

## 2026-08-27 — Alexandria natural-language prompt fix

- Alexandria sent the full natural-language question in `q` to `/weather`, so
  Isobar attempted to geocode the sentence instead of `Tokyo, Japan`.
- Added deterministic prompt parsing for location, forecast intent, day count,
  and requested fields. The existing next-24-hour current-weather wording is
  explicitly excluded from forecast rerouting.
- Added regression coverage; `npm test` passes 13/13.
- Deployed `src/weather.js` and `src/server.js` at 2026-08-27 14:27:48 UTC;
  local and remote hashes match. The exact failing request now returns Tokyo,
  Japan, seven daily dates, and 168 hourly rows. `/health` remains green.

## 2026-08-27 — epoch 286 score and forecast experiment

- Explorer reports epoch 286 with Isobar WEATHER_CHECK at `#1/9`, score
  `0.018964943`, and WEATHER_FORECAST at `#2/11`, score `0.0090172645`.
  The forecast leader is `verity-weather-forecast` at `0.0098297`; this is a
  same-intent comparison, not a cross-intent score target.
- The epoch-286 forecast question asks for a 7-day hourly Tokyo forecast
  starting next Monday and includes a cutoff at `2026-09-01T06:00:00Z`. Its
  ground truth is a refusal-style response, so the older two-day template is
  not the complete evidence for this epoch.
- Added a forecast-only natural-language framing experiment. It echoes the
  requested relative start, cutoff, and requested fields before the available
  forecast while retaining all 7 daily and 168 hourly records in structured
  JSON. The current-weather answer path is unchanged.
- Deployed `src/weather.js` after 14/14 tests passed. The exact epoch question
  returns HTTP 200, Tokyo/Japan, 7 daily rows, 168 hourly rows, and `/health`
  returns HTTP 200 with `upstream_ok: true`.
- Local and remote `src/weather.js` SHA-256 match at
  `c4f7ad789e62f6d873315feaa114b4624866d0fe3e4984a67462466c19e7ef4d`; the
  service reports active with zero restarts.
- The Explorer row above predates this deployment; do not treat it as a
  measurement of the framing experiment until a later row is scored.

## 2026-08-27 — hourly leader-shape experiment

- The current WEATHER_FORECAST leader publishes dated daily summaries followed
  by all hourly rows. Isobar now uses that shape only when the request is
  explicitly hourly; ordinary forecasts retain the compact answer.
- The rendered daily format is `date: condition, low-high C, precipitation up
  to %, wind up to km/h`; hourly rows include condition, temperature, dew
  point, precipitation probability/amount, and km/h wind. Structured JSON still
  retains metric `wind_ms` and all requested fields.
- `npm run check` passes 15/15. The live exact epoch question returns HTTP 200,
  7 daily rows, 168 hourly rows, and dew-point data; `/health` is green.
- This is a single forecast-format variable. Epoch 286 still reports the
  pre-deployment Isobar forecast row at `#2/11`; wait for the next Explorer
  measurement before accepting or reverting it.

## 2026-08-27 — epoch 287 checkpoint and Track 2 tie investigation

- Explorer has advanced to epoch 287. Registration `#224` is active and the
  public health check remains HTTP 200 with `upstream_ok: true`.
- Isobar is currently `#1/9` for WEATHER_CHECK at `0.29039782`, scored at
  `2026-08-27T18:41:50.672985Z`. Epoch-287 WEATHER_FORECAST has no scored rows
  yet. The exact live record is captured in [`GROUND_TRUTH.md`](GROUND_TRUTH.md).
- The first uncommitted scorer experiment in `src/scorer/src/weather.rs` ignored
  numeric tokens in context keys and made typed-fact adjustments penalty-only.
  Its fresh fit/holdout/traffic run reported margins
  `0.939341`/`0.938561`/`0.994781`, agreement `0.709362`, and non-zero ties;
  those measurements are superseded by the tie-resolution checkpoint below.
- Track 2 was not registered during this checkpoint. The remaining release
  work is recorded below: clean-clone reproducibility and refreshed live
  champion calibration before a WEATHER_CHECK-only submission.

## 2026-08-27 — Track 2 tie-resolution checkpoint (superseded by the 2026-08-28 refresh)

- The scorer's exported rank path now includes the same weather typed-fact
  adjustment used by `breakdown_answer` and the native harness.
- Numeric context keys exclude nearby figures, and typed-fact adjustments are
  penalty-only. A deterministic FNV-1a/splitmix secondary key resolves zero,
  ordinary, and saturated six-decimal score bands without time, randomness,
  I/O, or unordered collections.
- Release pre-flight on the captured WEATHER_CHECK data reports zero
  distinct-input ties: fit `156/156` ordering and `0.939391` margin, holdout
  `43/44` and `0.938643`, and the 200-row corpus `200/200` and `0.994568`.
  Rank agreement is `0.721230` and all three runs pass 1,000 determinism
  iterations. Repeated identical inputs remain tied intentionally.
- Native Rust tests pass `18/18` for the production feature set and `19/19`
  with `real_weights`. The default release WASM passes 1,000
  repeated calls in both wazero compiler and interpreter runtimes. Artifact
  size is 1,101,744 bytes; SHA-256 is
  `56e2c01d4977be77601b168e7de82996cb5fa51262d8912ac6efc5674f2798c5`.
- Track 2 remained unregistered at this checkpoint. The live champion bar was
  refreshed in the next checkpoint; the remaining steps are to commit the
  final source, reproduce the artifact from a clean clone, and only then submit
  one intent.

## 2026-08-28 — Track 2 live-bar refresh

- Refreshed the live WASM list: registration `#510` remains the active
  WEATHER_CHECK champion at margin `0.98340964`, ordering `12/12`, with
  Spearman agreement `0.6128585`.
- Against that live bar, the current candidate measures fit margin `0.996578`
  with ordering `156/156`, holdout margin `0.984067` with ordering `44/44`,
  and 200-row traffic margin `0.996181` with ordering `200/200`.
- Agreement is `0.701480`; distinct-input ties are zero in all three datasets,
  while repeated identical inputs remain tied by design. Each run passed 1,000
  determinism iterations.
- The release artifact is 1,103,845 bytes with SHA-256
  `b8a920df89f38245daa82e91174db4467c4941fb2e619dd98fbcb81cee116b94`.
  Rust tests pass `21/21` (`22/22` with `real_weights`), and formatting plus
  diff checks pass.
- A temporary clone of `HEAD` with the current working diff reproduced the same
  artifact bytes and SHA-256; the final clean-clone check remains after the
  source is committed.
- Registration is still intentionally paused: source changes are uncommitted,
  so clean-clone reproduction and the final artifact handoff remain.
