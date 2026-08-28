# Telegraph registration record

Status: Miner registered successfully and active in the Telegraph Explorer.
The status and score snapshot below were checked on 2026-08-26 before the
format-compatibility deployment.

Use `integrate.telegraphprotocol.com` and the wizard; do not hand-write a manifest.

| Field | Value |
|---|---|
| slug | `isobar-weather` |
| name | `Isobar Weather` |
| protocol | `generic` |
| intents | `WEATHER_CHECK`, `WEATHER_FORECAST` |
| endpoint | `https://weather.isobars.xyz/weather` |
| min_price | `0.01 USDC` |
| description | Deterministic current-conditions and multi-day forecast data for any global location, drawn from an ensemble of 30+ national weather models including ECMWF, NOAA, DWD, Météo-France, and JMA. Every answer leads with the numbers — temperature in both Celsius and Fahrenheit, apparent temperature, humidity, wind speed and bearing, precipitation, pressure, and WMO condition code — and is exactly reproducible from the logged upstream request. |

## Evidence log

- Domain and external health check: `https://weather.isobars.xyz/health` returned HTTP 200 from an external network; `upstream_ok: true`.
- Wallet address: `0x39Dd06180B445B3215FD093a3bF7A3Bf42dfbe96` (Base Sepolia).
- Wizard screens/fields: Basics, Connection, Endpoints (`/weather` GET, `/forecast` GET), Semantics (`answer`, `WEATHER_CHECK`, `WEATHER_FORECAST`), On-Chain (`direct`, 0.01 USDC).
- IPFS manifest URI: `ipfs://QmYmTZGvkxhLDUJFc1HtadXX77VziX3PgL1FZ8AGeH46jt`
- IPFS gateway: `https://gateway.pinata.cloud/ipfs/QmYmTZGvkxhLDUJFc1HtadXX77VziX3PgL1FZ8AGeH46jt`
- YAML SHA-256: `0x678e7cc097ff5b807cccbf536f1b4fb791d23d2222a37f46407a8cec94165ab2`
- Endpoint validation: `/weather` HTTP 200 (853 ms); `/forecast` HTTP 200 (872 ms).
- Registration contract: `0x5a2324aA18613FAD4e44bDF0d6c73Ec1f6D87ff8`
- Fee address: `0x39Dd06180B445B3215FD093a3bF7A3Bf42dfbe96`
- Floor price: `0.01 USDC`
- Base Sepolia transaction hash: `0xcbb1de99b4ca7997…e4efe1ff` (the Telegraph confirmation screen displayed a shortened hash; retrieve the full value from the wallet or Base Sepolia explorer if needed).
- Registration ID: `224`
- Explorer URL: `https://explorer.telegraphprotocol.com/miners/224`
- First visible/scored epoch: `284`
- Epoch 284 baseline ranks: `WEATHER_CHECK #6/9` (`0.013609781`),
  `WEATHER_FORECAST #9/11` (`0.002964984`)

## Runtime compatibility

The registered schema uses `q` or `lat`/`lon`. The runtime also accepts the
evaluator's observed aliases `location` and `city`, plus `latitude` and
`longitude`, forecast `start_time`/`end_time` fields, and `hours`/
`forecast_hours` horizons. This keeps the registered miner usable when
Telegraph generates query parameters from natural-language questions.
Alexandria may place the full question in `q`, `question`, `prompt`, or
`request_text`; the runtime extracts the location, coordinates, forecast
duration, and requested fields from that value and supports both `/weather` and
`/forecast` routes.

## Scoring evidence

The Explorer's documented `/api/signals?limit=200` path returned HTTP 404 at
capture time. The equivalent scored records were available through
`/api/scores?intent=...&epoch=284&limit=200`; the captured details are in
[`docs/GROUND_TRUTH.md`](../docs/GROUND_TRUTH.md).

The epoch-284 validator called:

```text
GET /weather?location=Tokyo%2C+Japan
GET /forecast?city=Tokyo&end_time=2026-09-01T12%3A00%3A00Z&interval=hourly&start_time=2026-09-01T06%3A00%3A00Z&units=metric
```

The previous release accepted neither place-name alias, so both scored answers
were the invalid-location fallback. This was the primary cause of the low
epoch-284 result; formatting improvements are a second-stage optimization.

## Post-deployment experiment

The alias/format release was installed on 2026-08-26 at 21:19 UTC. The exact
validator-shaped requests now resolve to Tokyo, return `upstream_ok` data, and
include the current 24-hour horizon or requested forecast window. The first
complete epoch after this deployment is the measurement point; no rank-1 claim
is made until Explorer reports it.
