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
- The validator used `location` and `city` query aliases that the old handler rejected. The compatibility release now normalizes those aliases, maps forecast time bounds, requests UTC and m/s units, and emits the compact candidate format.
- The compatibility release is tested locally but still needs to be deployed and observed in a new epoch before its score impact can be claimed.
