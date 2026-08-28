# Weather score capture — epoch 284

Captured 2026-08-26 from the public Explorer API. This is the evidence used to
separate the input-contract failure from the answer-format experiment.

- Registration: [#224](https://explorer.telegraphprotocol.com/api/miners/224)
- Scores: [`/api/scores`](https://explorer.telegraphprotocol.com/api/scores)
- Leaderboard: [`/api/leaderboard/miners?epoch_id=284`](https://explorer.telegraphprotocol.com/api/leaderboard/miners?epoch_id=284)
- Epoch status: [`/api/epoch`](https://explorer.telegraphprotocol.com/api/epoch)
- Requested signal endpoint: `/api/signals?limit=200` returned HTTP 404 at capture time.

The rows below are scored records, not synthetic test cases. Every current
epoch row for an intent shares that intent's question and ground-truth string.
Isobar was registered at 13:45:22Z and scored at 15:45:11Z / 15:48:59Z, before
the local compatibility fix was deployed.

## WEATHER_CHECK

### Question

```text
What is the current temperature and 'feels like' temperature in Tokyo, Japan, and what is the forecast for the next 24 hours including any chance of precipitation?
```

### Ground truth, verbatim from `/api/scores`

```text
The current temperature in Tokyo is 86°F (30°C), and it feels like 98°F (37°C) due to humidity. Over the next 24 hours, temperatures will range from around 81°F (27°C) to a high of 87°F (31°C). There is a chance of scattered showers or thunderstorms, with precipitation likely between 6 PM and 9 PM, and again later in the night, with chances ranging from 30% to 66% during those periods.
```

### Validator request and Isobar result

The validator request recorded in the remote service journal was:

```text
GET /weather?location=Tokyo%2C+Japan
```

The pre-fix Isobar raw payload was:

```json
{"answer":"A valid location was not provided. Supply a place name with q, or both latitude and longitude with lat and lon.","location":null,"retrieved_at":"2026-08-26T15:43:24.157Z","source":"open-meteo"}
```

Explorer converted it to:

```text
The AI service responded with an error message indicating that a valid location was not provided, and it requires either a place name or both latitude and longitude to proceed.
```

### Ten sampled scored signals

The nine epoch-284 rows are the same live question/reference above. The tenth
row is the preceding epoch's Tokyo check and is retained to reach the requested
ten-signal sample without inventing a result.

| Signal | Epoch | Miner | Rank | Score | Converted answer / observed result |
|---|---:|---|---:|---:|---|
| `5f4d6ef3-226d-40c2-a3a6-eb5cfbef877e` | 284 | `weatherapi` | 1 | `0.017398667` | The current weather in Tokyo, Japan, is overcast with a chance of thundery outbreaks nearby, with a temperature of 30.9°C (87.7°F) and a wind speed of 6.5 km/h (4 mph). |
| `381331a1-e1f5-4c80-b33f-dd5076efeff5` | 284 | `openweathermap` | 2 | `0.015579644` | The data shows current weather conditions in Tokyo, Japan, with overcast clouds and a temperature of 303.3 degrees Celsius (feels like 310.3 degrees Celsius). |
| `17980bcd-02fc-4238-a096-097dfa96dac3` | 284 | `verity-current-weather` | 3 | `0.014598295` | The weather conditions at the given location on August 26, 2026, at 15:30 UTC are partly cloudy with a temperature of 26.5°C (79.7°F), feeling like 32.4°C (90.3°F), and a wind speed of 3 km/h. |
| `1ccdf16e-20e4-40d5-a9d2-77a6e05bd50d` | 284 | `bittensor-sn18-zeus` | 4 | `0.014398679` | The data shows hourly temperature readings from 2026-08-26 12:00:00Z to 2026-09-10 12:00:00Z, with a temperature range of approximately 296.25K to 304.25K. |
| `e850f7cd-6642-44b6-8658-146cf3b2e709` | 284 | `amanat-weather-risk` | 5 | `0.013981219` | The weather forecast for August 26, 2026, at 15:00 UTC in the specified location predicts mainly clear skies with a temperature range of 25.2 to 33.9 degrees Celsius. |
| `f347b87f-b934-4c73-96cb-fd8d2729f2b2` | 284 | `isobar-weather` | 6 | `0.013609781` | Invalid-location fallback above. |
| `0e564be7-c423-4340-ae37-c62632aa5735` | 284 | `skywire-weather-check` | 7 | `0` | Empty miner answer. |
| `769b60d4-f46e-4b7f-9ee7-2b43c5c16107` | 284 | `solar-depin-miner` | 8 | `0` | Empty miner answer. |
| `94be25fb-498a-4dbb-baa0-063b36be6f2f` | 284 | `lacre-meteo` | 9 | `0` | Empty miner answer. |
| `5f2dcd45-6ceb-4bdc-958e-7d2dfcaa6e42` | 283 | `weatherapi` | 1 | `0.7675967` | The current weather in Tokyo, Japan, at 22:00 on August 26, 2026, is partly cloudy with a chance of thundery outbreaks nearby, with a temperature of 30.9°C (87.6°F) and a wind speed of 11.2 km/h (6.9 mph). |

The epoch-283 tenth row used this separate question:

```text
What is the current temperature in Tokyo, Japan and the high and low temperature forecast for today?
```

Its ground truth was:

```text
The current temperature in Tokyo, Japan is 88°F, with a 'feels like' temperature of 102°F. The high temperature forecast for today is 87°F, and the low is 77°F.
```

The live leader is `weatherapi` at `0.017398667`; Isobar's current row is
`0.013609781`. The absolute scale is specific to WEATHER_CHECK and must not be
compared with the forecast scale or another intent.

## WEATHER_FORECAST

### Question

```text
Can you provide a 48-hour hourly weather forecast for Tokyo, Japan starting from 2026-09-01T06:00:00Z UTC, with a cutoff deadline of 2026-09-01T12:00:00Z UTC and include temperature in Celsius and precipitation in millimeters?
```

### Ground truth, verbatim from `/api/scores`

```text
Unfortunately, I cannot provide the exact 48-hour hourly weather forecast for Tokyo, Japan starting from 2026-09-01T06:00:00Z UTC with a cutoff deadline of 2026-09-01T12:00:00Z UTC, including temperature in Celsius and precipitation in millimeters, as the available search results do not contain this specific data. The results show weather forecasts for various dates in August 2026 and general weather conditions, but they do not include the precise hourly forecast for the requested timeframe in September 2026.

For accurate and detailed hourly forecasts for specific future dates, I recommend checking reliable weather services or forecast websites that provide extended and hourly predictions, such as the Japan Meteorological Agency, Weather.com, or other professional meteorological platforms.
```

### Validator request and Isobar result

The validator request was:

```text
GET /forecast?city=Tokyo&end_time=2026-09-01T12%3A00%3A00Z&interval=hourly&start_time=2026-09-01T06%3A00%3A00Z&units=metric
```

The pre-fix Isobar raw payload was:

```json
{"answer":"A valid location was not provided. Supply a place name with q, or both latitude and longitude with lat and lon.","location":null,"retrieved_at":"2026-08-26T15:46:53.883Z","source":"open-meteo"}
```

Explorer converted it to the same invalid-location error answer recorded for
WEATHER_CHECK.

### Eleven scored signals

| Signal | Miner | Rank | Score | Converted answer / observed result |
|---|---|---:|---:|---|
| `6d66f252-45f2-40a7-9406-28d16926fc55` | `verity-weather-forecast` | 1 | `0.009923598` | The data shows a 2-day weather forecast for Tokyo, Japan, with varying weather conditions and precipitation levels. On August 27, 2026, the day started with clear skies and ended with drizzle, while August 28, 2026, began with drizzle and ended with rain. |
| `59241b35-2cbe-406d-b1fb-2f1d774cd822` | `weatherapi` | 2 | `0.009612181` | The current weather in Tokyo, Japan, is overcast with a chance of light rain showers, with a temperature of 30.9°C (87.7°F) and a dew point of 23.3°C (74°F). The forecast for the next day shows overcast conditions with a chance of light rain showers, with a high of 31.7°C (89.1°F) and a low of 25.8°C (78.4°F). The Explorer value ends with `and sunrise and`. |
| `a4c7f8e0-a0e2-4676-af5a-e6a6e1d97970` | `skywire-forecast` | 3 | `0.008894513` | The weather forecast for Tokyo, Japan, on August 27, 2026, predicts light drizzle with a 68% chance of rain, high of 29.1°C, and low of 24.2°C, with winds around 6 km/h. |
| `e7d9849b-c567-490c-a5b3-3526a07a680a` | `openweathermap` | 4 | `0.008597416` | The weather in Tokyo, Japan, from August 26 to August 31, 2026, is characterized by frequent light rain, with overcast conditions and scattered to broken clouds. |
| `286f8e04-defa-443c-a16c-33a48dda85b2` | `bittensor-sn18-zeus` | 5 | `0.008258144` | The data shows hourly temperature readings from 12 PM to 11 PM on August 26, 2026, in Kyoto, Japan, with a temperature range of 298.25 to 303.5 Kelvin. |
| `4b0b77c4-59a5-4b84-b42c-4fceb17f19d4` | `onlookout-weather` | 6 | `0.008137066` | The weather forecast for Midtown, Tokyo, shows today with a high of 33.9°C and low of 25.2°C, mainly clear, and tomorrow with a high of 29.1°C and low of 22.7°C, partly cloudy. |
| `13ad5a43-f9e9-43fd-b037-6b9614ba9c93` | `livecert` | 7 | `0.0069898367` | The forecast for Tokyo over the next 48 hours predicts drizzle, with temperatures ranging from 22.7°C to 33.9°C, and maximum wind speeds of 7.2 km/h. |
| `8ff84ac8-80ee-4a6a-b937-6b252c14033a` | `amanat-weather-risk` | 8 | `0.006704804` | The weather forecast for August 26, 2026, at 15:00 UTC in the specified location predicts mainly clear skies with a temperature of 26.7°C, wind speeds of 2.4 km/h, gusts up to 11.2 km/h, and no precipitation expected. |
| `9e003a7b-b573-45a2-86b4-8fad167cd1fd` | `isobar-weather` | 9 | `0.002964984` | Invalid-location fallback above. |
| `5ea21165-db04-4ccf-bae7-3a494825be42` | `oathcast-weather` | 10 | `0` | Empty answer; upstream rejected `forecast_cutoff` ordering. |
| `d4fc0eee-ace7-4b7c-b1cd-427a2f027554` | `lacre-meteo` | 11 | `0` | Empty answer; Cloudflare Tunnel returned HTTP 530. |

The live leader is `verity-weather-forecast` at `0.009923598`; Isobar's current
row is `0.002964984`. OnLookout's concise two-day answer is materially closer
to the format hypothesis than Isobar's pre-fix error, but the forecast ground
truth itself is long-form. Treat compact formatting as an experiment and let
the next live epoch decide.

## Captured implications

1. The primary epoch-284 defect was the request contract: `location` and `city`
   reached a handler that only read `q`.
2. The concise UTC/m/s format is now implemented locally, with all detailed
   values retained in structured JSON.
3. The local scorer corpus should include these real question/reference pairs,
   the competitor answers above, the invalid-location answer, and mutations for
   wrong digits, units, conditions, and locations.
4. No public request counter is exposed for registration #224; its absence is
   not evidence of zero traffic.

## Epoch 286 live comparison

Captured from the public Explorer score API on 2026-08-27. The epoch endpoint
reported `current_epoch: 286`. The excerpts below preserve the exact question,
reference, and Isobar rendered answer used for the next format experiment.

### WEATHER_CHECK

Question:

```text
What is the current temperature and 'feels like' temperature in Tokyo, Japan, and what is the hourly forecast for the next 24 hours for the variable '2t' (temperature)?
```

Ground truth, verbatim:

```text
The current temperature in Tokyo, Japan is approximately **84°F (29°C)** with a "feels like" temperature of **93°F (34°C)**.

The hourly forecast for the next 24 hours for the variable '2t' (temperature) in Tokyo shows temperatures ranging from approximately **27°C to 31°C (81°F to 88°F)**, with slight variations throughout the day.
```

Isobar rendered answer, verbatim:

```text
The current temperature in Tokyo, Japan is 24.7C, and it feels like 29.9C. Over the next 24 hours, temperatures range from 23C to 29C, with a chance of rain or showers, and precipitation chances ranging from 59% to 73%. As of 2026-08-27T09:30Z.
```

Explorer result: Isobar `#1/9`, `0.018964943`; `verity-current-weather` is `#2`,
`0.015380494`; `weatherapi` is `#3`, `0.015171461`.

### WEATHER_FORECAST

Question:

```text
Can you provide a 7-day hourly weather forecast for Tokyo, Japan starting from next Monday, including temperature in Celsius and precipitation probability, and deliver the forecast before the cutoff time of 2026-09-01T06:00:00Z?
```

Ground truth, verbatim:

```text
Sorry, I can't provide the exact 7-day hourly weather forecast for Tokyo, Japan starting from next Monday, including temperature in Celsius and precipitation probability, before the cutoff time of 2026-09-01T06:00:00Z, as the available search results do not contain the specific hourly forecast data required. The results provide general weather information and forecasts for various dates but lack the detailed hourly breakdown needed for the requested period. For precise hourly forecasts, please check a reliable weather service or app closer to the date.
```

Isobar rendered answer from the scored row, verbatim:

```text
Tokyo forecast: today high 28C low 23C moderate_rain; tomorrow high 29C low 23C moderate_drizzle. Nearest hour 2026-08-27T10:00Z: 24.5C, 0.0mm, 0.6m/s, overcast.
```

Explorer result: `verity-weather-forecast` is `#1/11`, `0.0098297`; Isobar is
`#2/11`, `0.0090172645`; `onlookout-weather` is `#3`, `0.00829008`. The Isobar
row was scored at `2026-08-27T09:38:46.719342Z`, before the later natural-
language forecast framing deployment. The deployed response is therefore a
new experiment, not a retroactive explanation of this score.

## Epoch 286 live API capture

The requested `/api/signals?limit=200` route returned HTTP 404. The equivalent
structured score capture was saved verbatim as [`signals.json`](signals.json)
from `/api/scores?intent=WEATHER_CHECK&limit=200` on 2026-08-27. It contains
200 historical WEATHER_CHECK rows, including the current epoch's question,
ground truth, raw miner payloads, converted answers, scores, ranks, and failure
reasons. The current epoch-286 WEATHER_CHECK rows were:

| Rank | Miner | Score | Converted answer / result |
|---:|---|---:|---|
| 1 | `isobar-weather` | `0.018964943` | The current weather in Tokyo, Japan, is overcast with a temperature of 24.7°C (76.5°F) and a perceived temperature of 29.9°C, with a 59-73% chance of rain or showers over the next 24 hours, ranging from 22.5°C to 29.1°C. |
| 2 | `verity-current-weather` | `0.015380494` | The weather in Tokyo, Japan, on August 27, 2026, at 9:30 UTC, is overcast with a temperature of 24.7°C (76.5°F) and a feels-like temperature of 29.9°C (85.8°F), with 92% humidity and a light wind of 2.5 km/h. |
| 3 | `weatherapi` | `0.015171461` | The current weather in Tokyo, Japan, at 18:30 local time on August 27, 2026, is described as light rain shower with 100% cloud cover, and feels like 80.4°F (26.9°C) with a dew point of 65°F (18.4°C). |
| 4 | `openweathermap` | `0.01499181` | The data shows that Tokyo, Japan, is experiencing heavy intensity rain with a visibility of 10 km and a wind speed of 4.63 m/s, with a temperature of 297.45°C (85.21°F) and humidity of 84%. |
| 5 | `amanat-weather-risk` | `0.014811547` | The weather forecast for a location near 35.69N, 139.69E over the next 24 hours predicts a temperature of 27.9°C with light winds and no precipitation, with a low storm risk of 0.148. |
| 6 | `bittensor-sn18-zeus` | `0.014068047` | The data shows temperature readings in Kelvin (K) for every hour from 2026-08-27 06:00:00Z to 2026-09-11 06:00:00Z, with a step of 361 hours. |
| 7 | `lacre-meteo` | `0` | Empty answer; upstream returned HTTP 530. |
| 8 | `skywire-weather-check` | `0` | Empty answer; the requested endpoint was not declared. |
| 9 | `solar-depin-miner` | `0` | Empty answer; upstream timed out. |

For Track 2, the 200-row history returned by the same `/api/scores` endpoint was
converted with [`extract_weather_corpus.py`](../src/scorer/tools/extract_weather_corpus.py)
into a reproducible corpus and the Explorer `score` field was used as the
independent champion vector. This is separate from the 10-row epoch-284 sample
above and is the evidence behind the scorer's current local calibration.

## Epoch 287 live capture

Captured on 2026-08-27 after the epoch boundary. `/api/epoch` reported:

```json
{"current_epoch":287,"epoch_duration":"9h0m0s","epoch_duration_seconds":32400,"next_epoch_at":"2026-08-28T03:36:55Z","settlement_epoch":287}
```

Registration `#224` remained active and the public `/health` endpoint returned
HTTP 200 with `upstream_ok: true`.

### WEATHER_CHECK

Question, verbatim:

```text
For a miner processing atmospheric data, provide a natural query requesting the current temperature and 'feels like' temperature in Tokyo, Japan, along with the probability of precipitation for the next 24 forecast hours, using the city's coordinates (35.6895°N, 139.6917°E) as input.
```

Ground truth, verbatim:

```text
Here is a natural query for a miner processing atmospheric data requesting the current temperature and 'feels like' temperature in Tokyo, Japan, along with the probability of precipitation for the next 24 forecast hours using the city's coordinates (35.6895°N, 139.6917°E) as input:

"Using coordinates 35.6895°N, 139.6917°E, provide the current temperature and 'feels like' temperature in Tokyo, Japan, along with the probability of precipitation for the next 24 forecast hours."
```

Converted reference text recorded with the row:

```text
The current weather in the specified location is characterized by light drizzle, with a temperature of 22.8°C and a perceived temperature of 27.4°C. Over the next 24 hours, temperatures will range from 22.7°C to 31.6°C, with a 60% to 83% chance of rain or showers.
```

The `miner_answer` payload begins:

```json
{"answer":"The current temperature in 35.6895, 139.6917 is 22.8C, and it feels like 27.4C. Over the next 24 hours, temperatures range from 23C to 32C, with a chance of rain or showers, and precipitation chances ranging from 60% to 83%. As of 2026-08-27T18:30Z.", ...}
```

Explorer result: Isobar `#1/9`, score `0.29039782`, scored at
`2026-08-27T18:41:50.672985Z`. The next rows were `openweathermap`
(`0.014834604`), `weatherapi` (`0.014801264`), and
`verity-current-weather` (`0.014577433`).

### WEATHER_FORECAST

The epoch-287 Explorer query returned `total: 0`; no WEATHER_FORECAST miner
answer has been scored yet for this epoch.
