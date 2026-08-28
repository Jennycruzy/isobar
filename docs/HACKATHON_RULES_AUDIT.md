# Telegraph hackathon general-rules audit

Checked against the official rules and supported-intent catalog on 2026-08-28.

## Universal requirements

| Requirement | Isobar evidence | Status |
|---|---|---|
| Intent-specific independent leaderboards | Explorer captures and scorer calibration are separated by `WEATHER_CHECK`, `WEATHER_FORECAST`, and `STORM_ALERT` | Ready locally |
| Real Telegraph miner traffic | Live registration `#224` is active and serves the two registered weather intents | Pass for the registered intents |
| No simulated or mocked data for Track 3 | Calibration used public Explorer captures only; no Track 3 demand is claimed | Must preserve |
| Stay live through Track 3 | `https://weather.isobars.xyz/health` is healthy at capture time | Must maintain |
| Public tagged X updates | No post URL has been recorded in this repository | Evidence still needed; tag `@Telegraphprotoc` |
| Official Hackathon Discord participation | Not observable from the repository | Participant must verify |
| No artificial inflation or gaming | No fabricated requests or score submissions were used | Pass for this work |

## Track-specific state

### Miner Track

`WEATHER_CHECK` and `WEATHER_FORECAST` are registered in #224. The local
`STORM_ALERT` route is prepared but is not registered, so it currently earns no
live Storm Alert score. The official intent is `STORM_ALERT`, not
`STORM_CHECK`.

The rules also require an eligible intent to have at least three active miners
and at least 100 real requests from Track 3 applications before it can qualify
for the global cash-prize guardrail. The captured Storm Alert epoch has five
active miners, but Isobar has no proof of 100 real Track 3 requests yet.

### Script Author Track

The default `WEATHER_CHECK` scorer artifact is locally reproducible and passed
the fresh 200-row live corpus calibration. The separate `WEATHER_FORECAST`
artifact also passed its local calibration. Neither local result is an official
Telegraph automated-eval result until the WASM is uploaded, pinned, and
registered through the official integration flow.

### Application Track

No application or real Track 3 request volume is claimed by this repository.
An application must use real Telegraph miners; replaying local calibration
rows would not satisfy that rule.

## Current gates before public submission

1. Keep the existing live weather routes healthy.
2. Review the local `/storm` output against fresh public `STORM_ALERT` traffic.
3. Deploy and externally validate `/storm` only after approving that change.
4. Register `STORM_ALERT` as a separate intent only after deployment validation.
5. Upload/pin the scorer artifact through the official flow; a local hash is
   reproducibility evidence, not registration.
6. Record public X update URLs and verify Discord participation.
