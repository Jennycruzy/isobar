import test from 'node:test';
import assert from 'node:assert/strict';
import { buildApp } from '../src/server.js';
import { compass, conditions, currentPayload, forecastPayload } from '../src/weather.js';

const location = { name: 'Gujranwala', admin1: 'Punjab', country: 'Pakistan', latitude: 32.15567, longitude: 74.18705, timezone: 'Asia/Karachi' };
const current = {
  timezone: 'Asia/Karachi',
  current_units: { temperature_2m: '°C', relative_humidity_2m: '%', apparent_temperature: '°C', precipitation: 'mm', wind_speed_10m: 'km/h', wind_direction_10m: '°', surface_pressure: 'hPa', cloud_cover: '%' },
  current: { time: '2026-08-25T19:15', temperature_2m: 34.2, relative_humidity_2m: 41, apparent_temperature: 38.1, precipitation: 0, weather_code: 2, wind_speed_10m: 12.4, wind_direction_10m: 215, surface_pressure: 1004.2, cloud_cover: 40 }
};
const daily = {
  daily_units: { temperature_2m_max: '°C', temperature_2m_min: '°C', precipitation_sum: 'mm', precipitation_probability_max: '%', wind_speed_10m_max: 'km/h' },
  daily: { time: ['2026-08-26', '2026-08-27'], weather_code: [63, 2], temperature_2m_max: [36.8, 35.1], temperature_2m_min: [27.1, 26.4], precipitation_sum: [2.4, 0], precipitation_probability_max: [65, 10], wind_speed_10m_max: [18.2, 12] }
};

test('complete WMO table and unknown fallback', () => {
  assert.equal(conditions(99), 'Thunderstorm with heavy hail');
  assert.equal(conditions(42), 'unknown conditions (code 42)');
});
test('16-point compass boundaries', () => {
  assert.deepEqual(compass(215), { short: 'SW', long: 'southwest' });
  assert.equal(compass(348.75).short, 'N');
});
test('current prose uses units and fixed precision', () => {
  const result = currentPayload(location, current, '2026-08-25T19:16:03.000Z');
  assert.match(result.answer, /^The current temperature/);
  assert.match(result.answer, /34\.2°C \(93\.6°F\)/);
  assert.match(result.answer, /southwest \(215°\)/);
  assert.match(result.answer, /under partly cloudy skies/);
  assert.ok(!result.answer.includes('\n'));
  assert.ok(result.answer.split(/\s+/).length >= 50 && result.answer.split(/\s+/).length <= 90);
});
test('forecast produces one sentence per day and respects units', () => {
  const result = forecastPayload(location, daily, 2, '2026-08-25T19:16:03.000Z');
  assert.match(result.answer, /^The 2-day forecast/);
  assert.match(result.answer, /36\.8°C \(98\.2°F\)/);
  assert.match(result.answer, /moderate rain/);
});
test('HTTP routes are graceful 200s and cap days', async () => {
  const calls = [];
  const service = { probe: async () => true, query: async (kind, input) => { calls.push({ kind, input }); return { answer: 'ok' }; } };
  const app = buildApp({ logger: false, service });
  assert.equal((await app.inject('/health')).statusCode, 200);
  assert.equal((await app.inject('/weather?q=Krak%C3%B3w')).statusCode, 200);
  assert.equal((await app.inject('/weather?lat=32.15&lon=74.18')).statusCode, 200);
  assert.equal((await app.inject('/weather')).statusCode, 200);
  assert.equal((await app.inject('/weather?lat=999&lon=0')).statusCode, 200);
  await app.close();
  assert.equal(calls.length, 2);
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
