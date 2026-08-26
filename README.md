# Isobar Weather

Deterministic current weather and seven-day forecasts for Telegraph Protocol.

## Local development

Requires Node.js 22 or newer.

```sh
npm ci
npm test
npm start
```

The service listens on `127.0.0.1:8080` by default. Routes are `/health`, `/weather?q=...`, and `/forecast?q=...&days=3`; `lat` and `lon` may replace `q`.

See [deployment](docs/DEPLOYMENT.md) and [registration](telegraph/REGISTRATION.md).
