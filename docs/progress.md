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
