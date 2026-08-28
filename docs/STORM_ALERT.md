# STORM_ALERT preparation

The official Telegraph intent name is `STORM_ALERT`. Isobar's existing live
registration `#224` currently covers only `WEATHER_CHECK` and
`WEATHER_FORECAST`; this document records the local implementation prepared
for a same-slug `updateMiner` operation. The intended result is one
`isobar-weather` miner serving all three supported intents, not a second
`isobar-storm-alert` identity.

## Response contract

`GET /storm` accepts the same place-name, coordinate, alias, and natural-
language inputs as the other weather routes. It defaults to a 48-hour window
unless the request explicitly asks for a shorter horizon or says “today”.

The scorer-facing `answer` leads with:

- maximum sustained wind in km/h;
- peak gust in km/h;
- precipitation evidence;
- a deterministic 0–1 risk score and risk label;
- thunderstorm hours when WMO codes 95–99 occur; and
- periods where sustained wind exceeds 25 knots.

The structured `storm` object also includes m/s and knots conversions, hourly
rows, and `warning_data: "not-provided-by-open-meteo"`. Open-Meteo's forecast
variables include 10 m wind speed and 10 m wind gusts, but this implementation
does not claim to provide official local warning/advisory data or ERA5
`100u`/`100v` model-level components.

The risk score is deterministic and transparent: the primary signal is peak
gust divided by 160 km/h, bounded to 1.0. Precipitation probability, hourly
precipitation, and thunderstorm evidence can raise the score when wind alone
would understate disruption risk. This is a forecast-derived operational risk
indicator, not a government warning level.

## Local validation

The implementation is covered by unit tests for prompt extraction, response
format, unit conversion, caching-compatible service routing, and the gust-aware
Open-Meteo request. It must still be exercised against live Telegraph
`STORM_ALERT` traffic after the consolidated update; local fixtures are not a
substitute for real Track 3 demand.
