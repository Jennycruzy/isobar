# Isobar Weather

Deterministic current weather and forecasts for Telegraph Protocol, backed by
Open-Meteo. The human-readable `answer` is intentionally compact for semantic
scoring; the structured response retains the detailed measurements.

## Live deployment

The Track 1 miner is deployed at [weather.isobars.xyz](https://weather.isobars.xyz)
on the Lightsail host `54.154.121.30`.

- Telegraph registration: `#224`, `active`
- Registered intents: `WEATHER_CHECK`, `WEATHER_FORECAST`
- Current checked epoch: `284`
- Latest checked ranks: `WEATHER_CHECK #6/9`, `WEATHER_FORECAST #9/11`
- Minimum price: `0.01 USDC`
- Health: [`/health`](https://weather.isobars.xyz/health)
- Explorer: [registration 224](https://explorer.telegraphprotocol.com/miners/224)

The deployment was healthy and scoring when last checked on 2026-08-26. Ranks
are intent-specific; there is no meaningful cross-intent overall rank.

## Local development

Requires Node.js 22 or newer.

```sh
npm ci
npm test
npm start
```

The service listens on `127.0.0.1:8080` by default. Routes are `/health`,
`/weather`, and `/forecast`.

Canonical inputs are `/weather?q=...` and `/forecast?q=...&days=2`; `lat` and
`lon` may replace `q`. For compatibility with evaluator-generated requests,
`location` and `city` are accepted as place-name aliases, `latitude` and
`longitude` are accepted as coordinate aliases, and forecast requests may
include `start_time` and `end_time` ISO timestamps.

Examples:

```sh
curl 'https://weather.isobars.xyz/weather?location=Tokyo'
curl 'https://weather.isobars.xyz/forecast?city=Tokyo&start_time=2026-09-01T06:00:00Z&end_time=2026-09-01T12:00:00Z'
```

Current answers follow this compact shape:

```text
Tokyo current: 30.9C, 0.0mm, 1.8m/s, partly_cloudy. As of 2026-08-26T15:30Z.
```

Forecast answers contain two compact day summaries and a nearest-hour record;
the structured `forecast` array carries the hourly values when available.

See [deployment](docs/DEPLOYMENT.md) and [registration](telegraph/REGISTRATION.md).
