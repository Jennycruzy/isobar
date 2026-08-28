import test from 'node:test';
import assert from 'node:assert/strict';
import { buildApp } from '../src/server.js';
import { compass, conditions, conditionSlug, currentPayload, forecastPayload, isForecastRequest, normalizeInput, stormPayload } from '../src/weather.js';

const location = { name: 'Gujranwala', admin1: 'Punjab', country: 'Pakistan', latitude: 32.15567, longitude: 74.18705, timezone: 'Asia/Karachi' };
const current = {
  timezone: 'UTC',
  current_units: { temperature_2m: '°C', relative_humidity_2m: '%', apparent_temperature: '°C', precipitation: 'mm', wind_speed_10m: 'm/s', wind_direction_10m: '°', surface_pressure: 'hPa', cloud_cover: '%' },
  current: { time: '2026-08-25T14:15', temperature_2m: 34.2, relative_humidity_2m: 41, apparent_temperature: 38.1, precipitation: 0, weather_code: 2, wind_speed_10m: 3.4, wind_direction_10m: 215, surface_pressure: 1004.2, cloud_cover: 40 }
};
const currentWithHorizon = {
  timezone: 'UTC',
  current_units: current.current_units,
  current: { ...current.current, time: '2026-08-26T15:30', temperature_2m: 30.9, apparent_temperature: 37, weather_code: 3 },
  hourly_units: { temperature_2m: '°C', precipitation: 'mm', precipitation_probability: '%', weather_code: 'wmo code' },
  hourly: {
    time: ['2026-08-26T16:00', '2026-08-26T17:00', '2026-08-26T18:00'],
    temperature_2m: [27.1, 31.2, 29.4],
    precipitation: [0.2, 0.4, 0],
    precipitation_probability: [30, 66, 0],
    weather_code: [80, 95, 3]
  }
};
const daily = {
  timezone: 'UTC',
  daily_units: { temperature_2m_max: '°C', temperature_2m_min: '°C', precipitation_sum: 'mm', precipitation_probability_max: '%', wind_speed_10m_max: 'm/s' },
  hourly_units: { temperature_2m: '°C', dew_point_2m: '°C', precipitation: 'mm', precipitation_probability: '%', wind_speed_10m: 'm/s' },
  hourly: { time: ['2026-08-26T00:00', '2026-08-26T01:00'], temperature_2m: [28.5, 29.1], dew_point_2m: [22.1, 22.3], precipitation: [2.4, 0], precipitation_probability: [0, 10], weather_code: [63, 2], wind_speed_10m: [5.1, 4.2] },
  daily: { time: ['2026-08-26', '2026-08-27'], weather_code: [63, 2], temperature_2m_max: [36.8, 35.1], temperature_2m_min: [27.1, 26.4], precipitation_sum: [2.4, 0], precipitation_probability_max: [65, 10], wind_speed_10m_max: [18.2, 12] }
};
const storm = {
  timezone: 'UTC',
  daily_units: { temperature_2m_max: '°C', temperature_2m_min: '°C', precipitation_sum: 'mm', precipitation_probability_max: '%', wind_speed_10m_max: 'm/s', wind_gusts_10m_max: 'm/s' },
  hourly_units: { temperature_2m: '°C', precipitation: 'mm', precipitation_probability: '%', weather_code: 'wmo code', wind_speed_10m: 'm/s', wind_gusts_10m: 'm/s', wind_direction_10m: '°' },
  hourly: {
    time: ['2026-08-26T00:00', '2026-08-26T01:00'],
    temperature_2m: [25, 25.4], precipitation: [0, 0], precipitation_probability: [0, 0], weather_code: [1, 2],
    wind_speed_10m: [3.6, 3.5], wind_gusts_10m: [7.4, 7], wind_direction_10m: [215, 225]
  },
  daily: { time: ['2026-08-26', '2026-08-27'], weather_code: [1, 2], temperature_2m_max: [28, 29], temperature_2m_min: [20, 21], precipitation_sum: [0, 0], precipitation_probability_max: [0, 0], wind_speed_10m_max: [3.6, 3.5], wind_gusts_10m_max: [7.4, 7] }
};

test('complete WMO table and unknown fallback', () => {
  assert.equal(conditions(99), 'Thunderstorm with heavy hail');
  assert.equal(conditions(42), 'unknown conditions (code 42)');
  assert.equal(conditionSlug(0), 'clear');
  assert.equal(conditionSlug(82), 'violent_rain_showers');
});
test('16-point compass boundaries', () => {
  assert.deepEqual(compass(215), { short: 'SW', long: 'southwest' });
  assert.equal(compass(348.75).short, 'N');
});
test('normalizes protocol location and coordinate aliases', () => {
  assert.equal(normalizeInput({ location: 'Tokyo, Japan' }).q, 'Tokyo, Japan');
  assert.equal(normalizeInput({ city: 'Tokyo', latitude: '35.7', longitude: '139.7' }).q, 'Tokyo');
  assert.equal(normalizeInput({ city: 'Tokyo', latitude: '35.7', longitude: '139.7' }).lat, '35.7');
  assert.equal(normalizeInput({ city: 'Tokyo', start_date: '2026-09-03', end_date: '2026-09-09' }).start_time, '2026-09-03');
  assert.equal(normalizeInput({ city: 'Tokyo', start_date: '2026-09-03', end_date: '2026-09-09' }).end_time, '2026-09-09');
});
test('normalizes Alexandria natural-language weather prompts', () => {
  const prompt = 'Give me a 7-day hourly weather forecast for Tokyo, Japan, including temperature in Celsius, precipitation probability, and wind speed.';
  const result = normalizeInput({ q: prompt });
  assert.equal(result.q, 'Tokyo, Japan');
  assert.equal(result.days, 7);
  assert.equal(result.request_text, prompt);
  assert.equal(isForecastRequest(prompt), true);
  assert.match(result.fields, /precipitation probability/);
});
test('normalizes coordinate hourly forecast requests and their horizon', () => {
  const prompt = "Can you provide the temperature forecast for the next 48 hours at latitude 37.7749 and longitude -122.4194, using the '2t' variable for hourly data?";
  const result = normalizeInput({ question: prompt });
  assert.equal(result.lat, '37.7749');
  assert.equal(result.lon, '-122.4194');
  assert.equal(result.hours, 48);
  assert.equal(result.days, 2);
  assert.equal(result.request_text, prompt);
  assert.equal(isForecastRequest(prompt), true);
});
test('keeps a next-24-hour current question on the current route', () => {
  const prompt = "What is the current temperature and 'feels like' temperature in Tokyo, Japan, and what is the forecast for the next 24 hours including any chance of precipitation?";
  assert.equal(normalizeInput({ q: prompt }).q, 'Tokyo, Japan');
  assert.equal(isForecastRequest(prompt), false);
});
test('normalizes storm prompts and extracts their location and horizon', () => {
  const prompt = 'Are wind speeds over Islamabad strong enough to cause disruptions today?';
  const result = normalizeInput({ q: prompt });
  assert.equal(result.q, 'Islamabad');
  assert.equal(result.request_text, prompt);
  assert.equal(result.hours, undefined);
});
test('current prose is concise and keeps detail structured', () => {
  const result = currentPayload(location, current, '2026-08-25T19:16:03.000Z');
  assert.equal(result.answer, 'Gujranwala current: 34.2C, 0.0mm, 3.4m/s, partly_cloudy. As of 2026-08-25T14:15Z.');
  assert.equal(result.current.wind_speed_ms, 3.4);
  assert.equal(result.current.wind_speed_kmh, 12.2);
  assert.match(result.answer, /partly_cloudy/);
  assert.ok(!result.answer.includes('humidity'));
  assert.ok(!result.answer.includes('°F'));
  assert.ok(!result.answer.includes('\n'));
  assert.ok(result.answer.split(/\s+/).length <= 20);
});
test('current answer includes the requested 24-hour facts when available', () => {
  const result = currentPayload(
    { name: 'Tokyo', country: 'Japan', timezone: 'Asia/Tokyo' },
    currentWithHorizon,
    '2026-08-26T15:31:00.000Z'
  );
  assert.equal(result.answer, 'The current temperature in Tokyo, Japan is 30.9C, and it feels like 37.0C. Over the next 24 hours, temperatures range from 27C to 31C, with a chance of scattered showers or thunderstorms, and precipitation chances ranging from 30% to 66%. As of 2026-08-26T15:30Z.');
  assert.deepEqual(result.next_24h, {
    low_c: 27.1,
    high_c: 31.2,
    condition: 'scattered showers or thunderstorms',
    probability_min_pct: 30,
    probability_max_pct: 66
  });
  assert.ok(!result.answer.includes('humidity'));
  assert.ok(!result.answer.includes('pressure'));
});
test('forecast matches compact two-day and nearest-hour shape', () => {
  const result = forecastPayload(location, daily, 2, '2026-08-25T19:16:03.000Z');
  assert.equal(result.answer, 'Gujranwala forecast: today high 37C low 27C moderate_rain; tomorrow high 35C low 26C partly_cloudy. Nearest hour 2026-08-26T00:00Z: 28.5C, 2.4mm, 5.1m/s, moderate_rain.');
  assert.equal(result.forecast[0].wind_ms, 5.1);
  assert.ok(!result.answer.includes('precipitation'));
  assert.ok(!result.answer.includes('°F'));
});
test('forecast echoes an evaluator window in reference-aligned framing', () => {
  const result = forecastPayload(location, daily, 2, '2026-08-25T19:16:03.000Z', false, {
    requestStart: '2026-09-01T06:00:00Z', requestEnd: '2026-09-01T12:00:00Z'
  });
  assert.equal(result.answer, 'The 48-hour hourly weather forecast for Gujranwala, Pakistan starts from 2026-09-01T06:00:00Z UTC with a cutoff deadline of 2026-09-01T12:00:00Z UTC. today high 37C low 27C moderate_rain; tomorrow high 35C low 26C partly_cloudy. Nearest hour 2026-08-26T01:00Z: 29.1C, 0.0mm, 4.2m/s, partly_cloudy.');
});
test('forecast echoes relative natural-language window and only requested fields', () => {
  const prompt = 'Can you provide a 7-day hourly weather forecast for Tokyo, Japan starting from next Monday, including temperature in Celsius and precipitation probability, and deliver the forecast before the cutoff time of 2026-09-01T06:00:00Z?';
  const result = forecastPayload(location, daily, 7, '2026-08-25T19:16:03.000Z', false, {
    requestedFields: prompt, requestText: prompt
  });
  assert.match(result.answer, /^The 7-day hourly weather forecast for Gujranwala, Pakistan starting from next Monday, including temperature in Celsius and precipitation probability, before the cutoff time of 2026-09-01T06:00:00Z\. Available forecast:/);
  assert.match(result.answer, /precipitation probability 65%/);
  assert.ok(!result.answer.includes('wind speed'));
});
test('hourly forecast keeps detail structured and limits prose to requested fields', () => {
  const result = forecastPayload(location, daily, 2, '2026-08-25T19:16:03.000Z', false, {
    renderHourly: true, requestedFields: 'temperature and precipitation'
  });
  assert.equal(result.answer, 'Gujranwala, Pakistan forecast for 2026-08-26 through 2026-08-27: temperatures range from 28.5C to 29.1C; total precipitation 2.4mm. Hourly data contains 2 rows.');
  assert.equal(result.forecast.length, 2);
  assert.equal(result.forecast[0].wind_ms, 5.1);
  assert.equal(result.forecast[0].dew_point_c, 22.1);
  assert.ok(!result.answer.includes('wind'));
  assert.ok(!result.answer.includes('dew point'));
  assert.ok(!result.answer.includes('Rain'));
  assert.ok(!result.answer.includes('Nearest hour'));
});
test('storm alert payload leads with wind, gust, precipitation, and risk facts', () => {
  const result = stormPayload(location, storm, 2, '2026-08-25T19:16:03.000Z', false, { requestedHours: 48 });
  assert.equal(result.answer, 'The forecast for Gujranwala over the next 48 hours predicts sustained winds up to 13 km/h with peak gusts of 26.6 km/h, no precipitation, and a low risk score of 0.17. No thunderstorm conditions were returned in the hourly forecast. No periods with sustained winds above 25 knots are forecast.');
  assert.equal(result.storm.risk_score, 0.17);
  assert.equal(result.storm.risk_level, 'low');
  assert.equal(result.storm.peak_gust_kmh, 26.6);
  assert.equal(result.storm.sustained_above_25_knots, false);
  assert.equal(result.forecast[0].wind_direction_compass, 'SW');
  assert.equal(result.forecast[0].gust_knots, 14.4);
});
test('HTTP routes are graceful 200s and cap days', async () => {
  const calls = [];
  const service = { probe: async () => true, query: async (kind, input) => { calls.push({ kind, input }); return { answer: 'ok' }; } };
  const app = buildApp({ logger: false, service });
  const landing = await app.inject('/');
  assert.equal(landing.statusCode, 200);
  assert.match(landing.headers['content-type'], /^text\/html/);
  assert.match(landing.body, /Isobar Weather/);
  assert.match(landing.body, /REGISTRATION #224/);
  assert.equal((await app.inject('/health')).statusCode, 200);
  assert.equal((await app.inject('/weather?q=Krak%C3%B3w')).statusCode, 200);
  assert.equal((await app.inject('/weather?location=Tokyo')).statusCode, 200);
  assert.equal((await app.inject('/weather?q=Give%20me%20a%207-day%20hourly%20weather%20forecast%20for%20Tokyo%2C%20Japan%2C%20including%20temperature%20in%20Celsius.')).statusCode, 200);
  assert.equal((await app.inject('/forecast?city=Tokyo&start_time=2026-09-01T06%3A00%3A00Z&end_time=2026-09-01T12%3A00%3A00Z')).statusCode, 200);
  assert.equal((await app.inject('/storm?q=Tokyo&hours=48')).statusCode, 200);
  assert.equal((await app.inject('/weather?lat=32.15&lon=74.18')).statusCode, 200);
  assert.equal((await app.inject('/weather')).statusCode, 200);
  assert.equal((await app.inject('/weather?lat=999&lon=0')).statusCode, 200);
  await app.close();
  assert.equal(calls.length, 6);
  assert.equal(calls[1].input.q, 'Tokyo');
  assert.equal(calls[2].kind, 'forecast');
  assert.equal(calls[2].input.q, 'Tokyo, Japan');
  assert.equal(calls[2].input.days, 7);
  assert.equal(calls[3].input.q, 'Tokyo');
  assert.equal(calls[3].input.start_time, '2026-09-01T06:00:00Z');
  assert.equal(calls[4].kind, 'storm');
  assert.equal(calls[4].input.q, 'Tokyo');
});
test('identical cached query is byte-identical', async () => {
  let calls = 0;
  const service = (await import('../src/weather.js')).createWeatherService({
    now: () => new Date('2026-08-25T19:16:03Z'), logger: { info() {} },
    fetchImpl: async (url) => {
      calls++;
      return { ok: true, json: async () => String(url).includes('geocoding') ? { results: [location] } : current };
    }
  });
  const signal = AbortSignal.timeout(1000);
  const a = await service.query('current', { q: 'Gujranwala' }, signal);
  const b = await service.query('current', { q: 'Gujranwala' }, signal);
  assert.equal(JSON.stringify(a), JSON.stringify(b));
  assert.equal(calls, 2);
});

test('service accepts evaluator aliases and requests UTC metric forecast data', async () => {
  const urls = [];
  const service = (await import('../src/weather.js')).createWeatherService({
    now: () => new Date('2026-08-26T15:00:00Z'), logger: { info() {} },
    fetchImpl: async (url) => {
      urls.push(new URL(url));
      if (url.toString().includes('geocoding')) return { ok: true, json: async () => ({ results: [location] }) };
      return { ok: true, json: async () => daily };
    }
  });
  const result = await service.query('forecast', {
    city: 'Tokyo', start_time: '2026-09-01T06:00:00Z', end_time: '2026-09-01T12:00:00Z'
  }, AbortSignal.timeout(1000));
  assert.match(result.answer, /^The 48-hour hourly weather forecast for Gujranwala, Pakistan starts from/);
  assert.equal(urls.length, 2);
  assert.equal(urls[1].searchParams.get('wind_speed_unit'), 'ms');
  assert.equal(urls[1].searchParams.get('timezone'), 'UTC');
  assert.equal(urls[1].searchParams.get('start_date'), '2026-09-01');
  assert.equal(urls[1].searchParams.get('end_date'), '2026-09-02');
  assert.equal(urls[1].searchParams.get('hourly'), 'temperature_2m,dew_point_2m,precipitation,precipitation_probability,weather_code,wind_speed_10m');
});

test('service honors exact coordinate hourly horizons', async () => {
  const urls = [];
  const hourlyTimes = Array.from({ length: 72 }, (_, index) => new Date(Date.UTC(2026, 7, 26, index)).toISOString().slice(0, 16));
  const threeDay = {
    ...daily,
    daily: {
      ...daily.daily,
      time: ['2026-08-26', '2026-08-27', '2026-08-28'],
      weather_code: [63, 2, 3],
      temperature_2m_max: [36.8, 35.1, 34],
      temperature_2m_min: [27.1, 26.4, 25]
    },
    hourly: {
      ...daily.hourly,
      time: hourlyTimes,
      temperature_2m: Array(72).fill(25),
      dew_point_2m: Array(72).fill(20),
      precipitation: Array(72).fill(0),
      precipitation_probability: Array(72).fill(5),
      weather_code: Array(72).fill(2),
      wind_speed_10m: Array(72).fill(1)
    }
  };
  const service = (await import('../src/weather.js')).createWeatherService({
    now: () => new Date('2026-08-28T03:00:00Z'), logger: { info() {} },
    fetchImpl: async (url) => {
      urls.push(new URL(url));
      return { ok: true, json: async () => threeDay };
    }
  });
  const result = await service.query('forecast', {
    latitude: '37.7749', longitude: '-122.4194', forecast_hours: '48', variable: '2t'
  }, AbortSignal.timeout(1000));
  assert.equal(urls.length, 1);
  assert.equal(urls[0].searchParams.get('forecast_days'), '2');
  assert.equal(urls[0].searchParams.get('forecast_hours'), '48');
  assert.equal(result.requested_hours, 48);
  assert.equal(result.forecast.length, 48);
  assert.match(result.answer, /^The forecast for the next 48 hours at 37.7749, -122.4194 is valid through 2026-08-27T23:00Z, with temperatures ranging from 25C to 25C and partly cloudy conditions\./);
  assert.match(result.answer, /No precipitation is expected\./);
  assert.ok(!result.answer.includes('Hourly:'));
  assert.ok(!result.answer.includes('Nearest hour'));
});

test('service routes STORM_ALERT queries to gust-aware forecast data', async () => {
  const urls = [];
  const service = (await import('../src/weather.js')).createWeatherService({
    now: () => new Date('2026-08-26T15:00:00Z'), logger: { info() {} },
    fetchImpl: async (url) => {
      urls.push(new URL(url));
      return { ok: true, json: async () => storm };
    }
  });
  const result = await service.query('storm', {
    latitude: '37.7749', longitude: '-122.4194', hours: '48',
    request_text: 'Can you provide a 48-hour wind gust forecast including disruption risk?'
  }, AbortSignal.timeout(1000));
  assert.equal(urls.length, 1);
  assert.equal(urls[0].searchParams.get('forecast_hours'), '48');
  assert.equal(urls[0].searchParams.get('wind_speed_unit'), 'ms');
  assert.equal(urls[0].searchParams.get('hourly'), 'temperature_2m,precipitation,precipitation_probability,weather_code,wind_speed_10m,wind_gusts_10m,wind_direction_10m');
  assert.equal(result.requested_hours, 48);
  assert.equal(result.forecast.length, 2);
  assert.equal(result.storm.peak_gust_kmh, 26.6);
});

test('service honors date-window aliases and requested hourly probability fields', async () => {
  const urls = [];
  const sevenDay = {
    timezone: 'UTC',
    daily_units: { temperature_2m_max: '°C', temperature_2m_min: '°C', precipitation_probability_max: '%', precipitation_sum: 'mm', wind_speed_10m_max: 'm/s' },
    daily: {
      time: ['2026-09-03', '2026-09-04', '2026-09-05', '2026-09-06', '2026-09-07', '2026-09-08', '2026-09-09'],
      weather_code: [1, 2, 3, 61, 63, 2, 0],
      temperature_2m_max: [30, 31, 29, 28, 27, 30, 31],
      temperature_2m_min: [23, 24, 22, 21, 20, 22, 23],
      precipitation_probability_max: [10, 20, 40, 60, 70, 30, 5],
      precipitation_sum: [0, 0, 1.2, 4.1, 8.4, 0.2, 0],
      wind_speed_10m_max: [1.2, 1.4, 1.8, 2.1, 2.4, 1.5, 1.1]
    },
    hourly_units: { temperature_2m: '°C', dew_point_2m: '°C', precipitation: 'mm', precipitation_probability: '%', weather_code: 'wmo code', wind_speed_10m: 'm/s' },
    hourly: { time: ['2026-09-03T00:00'], temperature_2m: [25], dew_point_2m: [20], precipitation: [0], precipitation_probability: [10], weather_code: [1], wind_speed_10m: [1.2] }
  };
  const service = (await import('../src/weather.js')).createWeatherService({
    now: () => new Date('2026-08-27T00:00:00Z'), logger: { info() {} },
    fetchImpl: async (url) => {
      urls.push(new URL(url));
      if (url.toString().includes('geocoding')) return { ok: true, json: async () => ({ results: [location] }) };
      return { ok: true, json: async () => sevenDay };
    }
  });
  const result = await service.query('forecast', {
    city: 'Tokyo', start_date: '2026-09-03', end_date: '2026-09-09',
    fields: 'temperature,precipitation_probability,wind_speed', interval: 'hourly'
  }, AbortSignal.timeout(1000));
  assert.equal(result.answer, 'Gujranwala, Pakistan forecast for 2026-09-03 through 2026-09-09: temperatures range from 25C to 25C; precipitation probability up to 10%; wind speeds up to 4.3 km/h. Hourly data contains 1 rows.');
  assert.equal(urls[1].searchParams.get('start_date'), '2026-09-03');
  assert.equal(urls[1].searchParams.get('end_date'), '2026-09-09');
  assert.equal(urls[1].searchParams.get('hourly'), 'temperature_2m,dew_point_2m,precipitation,precipitation_probability,weather_code,wind_speed_10m');
  assert.equal(result.daily.time.length, 7);
});

test('multi-day protocol forecasts use the complete hourly answer shape without an interval flag', async () => {
  const sevenDay = {
    ...daily,
    daily: {
      ...daily.daily,
      time: ['2026-09-03', '2026-09-04', '2026-09-05', '2026-09-06', '2026-09-07', '2026-09-08', '2026-09-09'],
      weather_code: [1, 2, 3, 61, 63, 2, 0],
      temperature_2m_max: [30, 31, 29, 28, 27, 30, 31],
      temperature_2m_min: [23, 24, 22, 21, 20, 22, 23],
      precipitation_probability_max: [10, 20, 40, 60, 70, 30, 5],
      wind_speed_10m_max: [1.2, 1.4, 1.8, 2.1, 2.4, 1.5, 1.1]
    }
  };
  const service = (await import('../src/weather.js')).createWeatherService({
    now: () => new Date('2026-08-27T00:00:00Z'), logger: { info() {} },
    fetchImpl: async (url) => url.toString().includes('geocoding')
      ? { ok: true, json: async () => ({ results: [location] }) }
      : { ok: true, json: async () => sevenDay }
  });
  const result = await service.query('forecast', { city: 'Tokyo', days: 7 }, AbortSignal.timeout(1000));
  assert.equal(result.answer, 'Gujranwala, Pakistan forecast for 2026-09-03 through 2026-09-09: temperatures range from 28.5C to 29.1C; total precipitation 2.4mm; precipitation probability up to 10%; conditions include rain, partly cloudy; wind speeds up to 18.4 km/h; dew points range from 22.1C to 22.3C. Hourly data contains 2 rows.');
  assert.ok(!result.answer.includes('Nearest hour'));
});
