# Isobar score log

Scores and ranks are intent-specific. Values below are not comparable between
WEATHER_CHECK and WEATHER_FORECAST.

| Epoch / time | WEATHER_CHECK | WEATHER_FORECAST | Change / evidence |
|---|---:|---:|---|
| 284 baseline | #6/9, `0.013609781` | #9/11, `0.002964984` | Both validator requests used unsupported `location`/`city` aliases; captured in [`GROUND_TRUTH.md`](GROUND_TRUTH.md). |
| 2026-08-26 20:43 UTC | pending | pending | `7edb412`: alias normalization, UTC/m/s units, compact two-day forecast; deployed and smoke-tested. |
| 2026-08-26 21:19 UTC | pending | pending | `9d8e65f`: WEATHER_CHECK adds the requested feels-like and next-24-hour facts when hourly data is available; deployed separately. |
| 285 current snapshot | no Isobar row | #1, `0.008892506` | Current [Explorer score record](https://explorer.telegraphprotocol.com/api/scores?intent=WEATHER_FORECAST&epoch=285&limit=1). Epoch-285 WEATHER_CHECK has no Isobar row yet. |
| 2026-08-27 13:54 UTC | pending | pending | Deployed the validated `src/weather.js` date-window/field-contract fix; production smoke test passed. The epoch-285 row above was scored at 00:39 UTC, before deployment. |
| 286 current snapshot | #1/9, `0.018964943` | #2/11, `0.0090172645` | [Explorer epoch-286 rows](https://explorer.telegraphprotocol.com/api/scores?intent=WEATHER_FORECAST&epoch=286&limit=200): Isobar leads WEATHER_CHECK; `verity-weather-forecast` leads WEATHER_FORECAST at `0.0098297`. The Isobar forecast row was scored before the natural-language framing deployment. |
| 2026-08-27 14:31 UTC | unchanged | pending | Deployed the forecast-only natural-window framing experiment; exact epoch-286 prompt returns 7 daily and 168 hourly records. Awaiting a post-deployment score. |
| 2026-08-27 15:26 UTC | unchanged | pending | Deployed the hourly leader-shape experiment: dated daily summaries plus full `Hourly:` rows, dew point, precipitation `%/mm`, and km/h wind. Production smoke test passed; await the next Explorer row. |

Epoch 285 now has a forecast snapshot, but the next complete post-fix Explorer
epoch is the first measurement of the deployed date-window/field-contract fix.
The absolute value is not comparable across intents, and `0.019` is not a
verified guaranteed score.

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

The local replica is not calibrated enough to replace live ordering: the
deployed leader-shaped answer scored `0.507686` locally against the epoch-286
reference, while the captured `verity-weather-forecast` leader scored `0.539871`
and the old compact Isobar answer scored `0.982800`. Because that proxy orders
the old Isobar answer above the live leader, Explorer remains authoritative for
this experiment.

## Epoch-285 diagnosis

The captured live question requested a 7-day hourly Tokyo forecast with
temperature, precipitation probability, and wind speed. The old service
returned only two daily dates and 48 hourly rows because it ignored the
`start_date`/`end_date` aliases and omitted hourly precipitation probability
from its upstream request. The compatibility fix is deployed and verified; the
next complete Explorer epoch is the first post-deployment measurement. See
[`HANDOFF.md`](HANDOFF.md).
