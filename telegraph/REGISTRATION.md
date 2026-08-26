# Telegraph registration record

Status: Miner registered successfully; staged as pending and awaiting Explorer indexing/activation.

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
- Explorer URL and first visible epoch:
