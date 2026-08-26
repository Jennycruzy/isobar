import test from 'node:test';
import assert from 'node:assert/strict';
import { buildApp } from '../src/server.js';
import { compass, conditions, conditionSlug, currentPayload, forecastPayload, normalizeInput } from '../src/weather.js';

const location = { name: 'Gujranwala', admin1: 'Punjab', country: 'Pakistan', latitude: 32.15567, longitude: 74.18705, timezone: 'Asia/Karachi' };
const current = {
  timezone: 'UTC',
  current_units: { temperature_2m: '°C', relative_humidity_2m: '%', apparent_temperature: '°C', precipitation: 'mm', wind_speed_10m: 'm/s', wind_direction_10m: '°', surface_pressure: 'hPa', cloud_cover: '%' },
  current: { time: '2026-08-25T14:15', temperature_2m: 34.2, relative_humidity_2m: 41, apparent_temperature: 38.1, precipitation: 0, weather_code: 2, wind_speed_10m: 3.4, wind_direction_10m: 215, surface_pressure: 1004.2, cloud_cover: 40 }
};
const daily = {
  timezone: 'UTC',
  daily_units: { temperature_2m_max: '°C', temperature_2m_min: '°C', precipitation_sum: 'mm', precipitation_probability_max: '%', wind_speed_10m_max: 'm/s' },
  hourly_units: { temperature_2m: '°C', precipitation: 'mm', wind_speed_10m: 'm/s' },
  hourly: { time: ['2026-08-26T00:00', '2026-08-26T01:00'], temperature_2m: [28.5, 29.1], precipitation: [2.4, 0], weather_code: [63, 2], wind_speed_10m: [5.1, 4.2] },
  daily: { time: ['2026-08-26', '2026-08-27'], weather_code: [63, 2], temperature_2m_max: [36.8, 35.1], temperature_2m_min: [27.1, 26.4], precipitation_sum: [2.4, 0], precipitation_probability_max: [65, 10], wind_speed_10m_max: [18.2, 12] }
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
test('forecast matches compact two-day and nearest-hour shape', () => {
  const result = forecastPayload(location, daily, 2, '2026-08-25T19:16:03.000Z');
  assert.equal(result.answer, 'Gujranwala forecast: today high 37C low 27C moderate_rain; tomorrow high 35C low 26C partly_cloudy. Nearest hour 2026-08-26T00:00Z: 28.5C, 2.4mm, 5.1m/s, moderate_rain.');
  assert.equal(result.forecast[0].wind_ms, 5.1);
  assert.ok(!result.answer.includes('precipitation'));
  assert.ok(!result.answer.includes('°F'));
});
test('HTTP routes are graceful 200s and cap days', async () => {
  const calls = [];
  const service = { probe: async () => true, query: async (kind, input) => { calls.push({ kind, input }); return { answer: 'ok' }; } };
  const app = buildApp({ logger: false, service });
  assert.equal((await app.inject('/health')).statusCode, 200);
  assert.equal((await app.inject('/weather?q=Krak%C3%B3w')).statusCode, 200);
  assert.equal((await app.inject('/weather?location=Tokyo')).statusCode, 200);
  assert.equal((await app.inject('/forecast?city=Tokyo&start_time=2026-09-01T06%3A00%3A00Z&end_time=2026-09-01T12%3A00%3A00Z')).statusCode, 200);
  assert.equal((await app.inject('/weather?lat=32.15&lon=74.18')).statusCode, 200);
  assert.equal((await app.inject('/weather')).statusCode, 200);
  assert.equal((await app.inject('/weather?lat=999&lon=0')).statusCode, 200);
  await app.close();
  assert.equal(calls.length, 4);
  assert.equal(calls[1].input.q, 'Tokyo');
  assert.equal(calls[2].input.q, 'Tokyo');
  assert.equal(calls[2].input.start_time, '2026-09-01T06:00:00Z');
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
  assert.match(result.answer, /^Gujranwala forecast:/);
  assert.equal(urls.length, 2);
  assert.equal(urls[1].searchParams.get('wind_speed_unit'), 'ms');
  assert.equal(urls[1].searchParams.get('timezone'), 'UTC');
  assert.equal(urls[1].searchParams.get('start_date'), '2026-09-01');
  assert.equal(urls[1].searchParams.get('end_date'), '2026-09-02');
  assert.equal(urls[1].searchParams.get('hourly'), 'temperature_2m,precipitation,weather_code,wind_speed_10m');
});
