# Isobar score log

Scores and ranks are intent-specific. Values below are not comparable between
WEATHER_CHECK and WEATHER_FORECAST.

| Epoch / time | WEATHER_CHECK | WEATHER_FORECAST | Change / evidence |
|---|---:|---:|---|
| 284 baseline | #6/9, `0.013609781` | #9/11, `0.002964984` | Both validator requests used unsupported `location`/`city` aliases; captured in [`GROUND_TRUTH.md`](GROUND_TRUTH.md). |
| 2026-08-26 20:43 UTC | pending | pending | `7edb412`: alias normalization, UTC/m/s units, compact two-day forecast; deployed and smoke-tested. |
| 2026-08-26 21:19 UTC | pending | pending | `9d8e65f`: WEATHER_CHECK adds the requested feels-like and next-24-hour facts when hourly data is available; deployed separately. |

The next complete Explorer epoch is the first live measurement of the deployed
answers. Until then, the only honest result is “pending.”

## Local format probe

Using the in-repository real-weight scorer against the captured epoch-284
WEATHER_CHECK reference, with `K=16` and `C=0.4`:

| Candidate | Local score |
|---|---:|
| Epoch-284 invalid-location fallback | `0.018346` |
| Captured `weatherapi` answer | `0.961604` |
| 12-word compact answer | `0.772934` |
| Reference-aligned current + 24-hour answer | `0.997778` |

These are relative local measurements, not Explorer scores. They justify the
current format experiment but do not prove rank 1.
