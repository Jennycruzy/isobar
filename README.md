# Isobar Weather

Deterministic current weather and forecasts for Telegraph Protocol, backed by
Open-Meteo. Each response has a scorer-facing `answer` plus structured JSON
with the detailed measurements an agent may need.

## Live deployment

The Track 1 miner is deployed at [weather.isobars.xyz](https://weather.isobars.xyz)
on the Lightsail host `54.154.121.30`.

- Telegraph registration: `#224`, `active`
- Registered intents: `WEATHER_CHECK`, `WEATHER_FORECAST`
- Baseline epoch: `284` (`WEATHER_CHECK #6/9`, `WEATHER_FORECAST #9/11`)
- Latest available epoch-285 snapshot: `WEATHER_FORECAST #1/11`, score
  `0.008892506`; no Isobar `WEATHER_CHECK` row is available yet
- Minimum price: `0.01 USDC`
- Health: [`/health`](https://weather.isobars.xyz/health)
- Explorer: [registration 224](https://explorer.telegraphprotocol.com/miners/224)

The deployment is healthy and the latest available Explorer score snapshot is
recorded above. Ranks are intent-specific; there is no meaningful cross-intent
overall rank.

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
include `start_time`/`end_time` ISO timestamps, inclusive `start_date`/
`end_date` bounds, or `hours`/`forecast_hours` horizons. Natural-language
requests may also arrive in `question`, `prompt`, or `request_text`; location,
coordinates, day/hour horizons, and requested fields are extracted when
present. Forecasts default to three days when no horizon is supplied and are
capped at seven days or 168 hourly rows.

Examples:

```sh
curl 'https://weather.isobars.xyz/weather?location=Tokyo'
curl 'https://weather.isobars.xyz/forecast?city=Tokyo&start_time=2026-09-01T06:00:00Z&end_time=2026-09-01T12:00:00Z'
```

WEATHER_CHECK answers include the current temperature, feels-like temperature,
and the next-24-hour range when the upstream hourly data is available. A live
example is:

```text
The current temperature in Tokyo, Japan is 25.0C, and it feels like 31.0C. Over the next 24 hours, temperatures range from 22C to 30C, with a chance of rain or showers, and precipitation chances ranging from 34% to 84%. As of 2026-08-26T21:15Z.
```

WEATHER_FORECAST answers contain a concise exact-window summary for short
hour-based requests and the structured `forecast` array carries exactly the
requested hourly values. Longer hourly requests use dated daily summaries plus
hourly facts; the detailed measurements remain in JSON in both routes.

## Use through Alexandria

Open [Alexandria](https://alexandria.telegraphprotocol.com/), choose **Ask
Alexandria**, and enter a natural-language weather question. For example:

```text
Give me a 7-day hourly weather forecast for Tokyo, Japan, including temperature in Celsius, precipitation probability, and wind speed.
```

Alexandria may send that question as the `q` value to a miner endpoint. Isobar
extracts the location and forecast shape from that prompt, so a city-only query
is not required. Alexandria chooses the highest-ranked matching miner; inspect
the returned provider/receipt to confirm whether the call reached Isobar
Weather (`isobar-weather`, registration `#224`).

See [deployment](docs/DEPLOYMENT.md) and [registration](telegraph/REGISTRATION.md).
