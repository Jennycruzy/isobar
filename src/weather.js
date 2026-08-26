const GEO_URL = 'https://geocoding-api.open-meteo.com/v1/search';
const FORECAST_URL = 'https://api.open-meteo.com/v1/forecast';

export const WMO = new Map([
  [0, 'Clear sky'], [1, 'Mainly clear'], [2, 'Partly cloudy'], [3, 'Overcast'],
  [45, 'Fog'], [48, 'Depositing rime fog'], [51, 'Light drizzle'],
  [53, 'Moderate drizzle'], [55, 'Dense drizzle'], [56, 'Light freezing drizzle'],
  [57, 'Dense freezing drizzle'], [61, 'Slight rain'], [63, 'Moderate rain'],
  [65, 'Heavy rain'], [66, 'Light freezing rain'], [67, 'Heavy freezing rain'],
  [71, 'Slight snow fall'], [73, 'Moderate snow fall'], [75, 'Heavy snow fall'],
  [77, 'Snow grains'], [80, 'Slight rain showers'], [81, 'Moderate rain showers'],
  [82, 'Violent rain showers'], [85, 'Slight snow showers'], [86, 'Heavy snow showers'],
  [95, 'Thunderstorm'], [96, 'Thunderstorm with slight hail'],
  [99, 'Thunderstorm with heavy hail']
]);

const COMPASS = ['N','NNE','NE','ENE','E','ESE','SE','SSE','S','SSW','SW','WSW','W','WNW','NW','NNW'];
const LONG_COMPASS = ['north','north-northeast','northeast','east-northeast','east','east-southeast','southeast','south-southeast','south','south-southwest','southwest','west-southwest','west','west-northwest','northwest','north-northwest'];

export function compass(deg) {
  const index = Math.floor((Number(deg) + 11.25) / 22.5) % 16;
  return { short: COMPASS[(index + 16) % 16], long: LONG_COMPASS[(index + 16) % 16] };
}

export function conditions(code) {
  return WMO.get(Number(code)) ?? `unknown conditions (code ${code})`;
}

export class LruCache {
  constructor(max = 500) { this.max = max; this.items = new Map(); }
  get(key) {
    const value = this.items.get(key);
    if (value === undefined) return undefined;
    this.items.delete(key); this.items.set(key, value); return value;
  }
  set(key, value) {
    this.items.delete(key); this.items.set(key, value);
    if (this.items.size > this.max) this.items.delete(this.items.keys().next().value);
  }
}

function present(value) { return value !== null && value !== undefined && Number.isFinite(Number(value)); }
function one(value) { return Number(value).toFixed(1); }
function integer(value) { return Math.round(Number(value)).toString(); }
function fahrenheit(c) { return one(Number(c) * 9 / 5 + 32); }
function label(location) { return [location.name, location.admin1, location.country].filter(Boolean).join(', '); }
function unit(units, key, fallback = '') { return units?.[key] ?? fallback; }
function conditionPhrase(code) {
  const text = conditions(code).toLowerCase();
  if (text.endsWith(' sky')) return `${text.slice(0, -4)} skies`;
  if (/(clear|cloudy|overcast)$/.test(text)) return `${text} skies`;
  return text;
}

export function currentPayload(location, body, retrievedAt, stale = false) {
  location = { ...location, timezone: location.timezone === 'auto' || !location.timezone ? body.timezone : location.timezone };
  const c = body.current ?? {};
  const u = body.current_units ?? {};
  let answer = present(c.temperature_2m)
    ? `The current temperature in ${label(location)} is ${one(c.temperature_2m)}${unit(u, 'temperature_2m')} (${fahrenheit(c.temperature_2m)}°F)`
    : `Current conditions in ${label(location)}`;
  if (present(c.relative_humidity_2m)) answer += ` with ${integer(c.relative_humidity_2m)}${unit(u, 'relative_humidity_2m')} relative humidity`;
  if (present(c.apparent_temperature)) answer += `, feeling like ${one(c.apparent_temperature)}${unit(u, 'apparent_temperature')}`;
  answer += '.';
  if (present(c.wind_speed_10m)) {
    const direction = present(c.wind_direction_10m) ? compass(c.wind_direction_10m) : null;
    answer += ` Winds are ${one(c.wind_speed_10m)} ${unit(u, 'wind_speed_10m')}${direction ? ` from the ${direction.long} (${integer(c.wind_direction_10m)}°)` : ''}.`;
  }
  const extras = [];
  if (present(c.precipitation)) extras.push(`${one(c.precipitation)} ${unit(u, 'precipitation')} precipitation`);
  if (present(c.cloud_cover)) extras.push(`${integer(c.cloud_cover)}${unit(u, 'cloud_cover')} cloud cover`);
  if (extras.length) answer += ` There is ${extras.join(' and ')}`;
  if (present(c.weather_code)) answer += `${extras.length ? ',' : ' Conditions are'} under ${conditionPhrase(c.weather_code)}`;
  if (extras.length || present(c.weather_code)) answer += '.';
  if (present(c.surface_pressure)) answer += ` Surface pressure is ${one(c.surface_pressure)} ${unit(u, 'surface_pressure')}.`;
  if (c.time) answer += ` These conditions were observed at ${c.time} local time in the ${location.timezone ?? body.timezone ?? 'local'} timezone.`;
  if (stale) answer += ' This is the most recent cached observation because the live weather service is temporarily unavailable.';
  return {
    answer,
    location,
    current: {
      temperature_c: present(c.temperature_2m) ? Number(one(c.temperature_2m)) : undefined,
      temperature_f: present(c.temperature_2m) ? Number(fahrenheit(c.temperature_2m)) : undefined,
      apparent_temperature_c: present(c.apparent_temperature) ? Number(one(c.apparent_temperature)) : undefined,
      relative_humidity_pct: present(c.relative_humidity_2m) ? Number(integer(c.relative_humidity_2m)) : undefined,
      precipitation_mm: present(c.precipitation) ? Number(one(c.precipitation)) : undefined,
      wind_speed_kmh: present(c.wind_speed_10m) ? Number(one(c.wind_speed_10m)) : undefined,
      wind_direction_deg: present(c.wind_direction_10m) ? Number(integer(c.wind_direction_10m)) : undefined,
      wind_direction_compass: present(c.wind_direction_10m) ? compass(c.wind_direction_10m).short : undefined,
      surface_pressure_hpa: present(c.surface_pressure) ? Number(one(c.surface_pressure)) : undefined,
      cloud_cover_pct: present(c.cloud_cover) ? Number(integer(c.cloud_cover)) : undefined,
      weather_code: present(c.weather_code) ? Number(c.weather_code) : undefined,
      conditions: present(c.weather_code) ? conditions(c.weather_code) : undefined,
      observed_at: c.time
    },
    source: 'open-meteo', retrieved_at: stale ? `${retrievedAt} (stale)` : retrievedAt
  };
}

export function forecastPayload(location, body, days, retrievedAt, stale = false) {
  location = { ...location, timezone: location.timezone === 'auto' || !location.timezone ? body.timezone : location.timezone };
  const d = body.daily ?? {}; const u = body.daily_units ?? {};
  const count = Math.min(days, d.time?.length ?? 0); const sentences = [];
  for (let i = 0; i < count; i++) {
    const date = new Date(`${d.time[i]}T12:00:00Z`);
    let sentence = `${date.toLocaleDateString('en-GB', { weekday: 'long', day: 'numeric', month: 'long', timeZone: 'UTC' })}`;
    if (present(d.temperature_2m_max?.[i])) sentence += `, high ${one(d.temperature_2m_max[i])}${unit(u, 'temperature_2m_max')} (${fahrenheit(d.temperature_2m_max[i])}°F)`;
    if (present(d.temperature_2m_min?.[i])) sentence += `, low ${one(d.temperature_2m_min[i])}${unit(u, 'temperature_2m_min')} (${fahrenheit(d.temperature_2m_min[i])}°F)`;
    if (present(d.precipitation_sum?.[i])) sentence += `, ${one(d.precipitation_sum[i])} ${unit(u, 'precipitation_sum')} precipitation`;
    if (present(d.precipitation_probability_max?.[i])) sentence += ` with ${integer(d.precipitation_probability_max[i])}${unit(u, 'precipitation_probability_max')} chance`;
    if (present(d.wind_speed_10m_max?.[i])) sentence += `, winds to ${one(d.wind_speed_10m_max[i])} ${unit(u, 'wind_speed_10m_max')}`;
    if (present(d.weather_code?.[i])) sentence += `, ${conditionPhrase(d.weather_code[i])}`;
    sentences.push(sentence + '.');
  }
  let answer = `The ${count}-day forecast for ${label(location)}: ${sentences.join(' ')}`;
  if (stale) answer += ' This is the most recent cached forecast because the live weather service is temporarily unavailable.';
  return { answer, location, daily: d, daily_units: u, source: 'open-meteo', retrieved_at: stale ? `${retrievedAt} (stale)` : retrievedAt };
}

export function createWeatherService({ fetchImpl = fetch, logger = console, now = () => new Date() } = {}) {
  const geocodes = new LruCache(1000); const weather = new LruCache(500); const stale = new LruCache(500);
  async function upstream(url, signal) {
    let lastError;
    for (let attempt = 0; attempt < 2; attempt++) {
      logger.info({ upstream_url: url.toString(), attempt: attempt + 1 }, 'upstream request');
      try {
        const response = await fetchImpl(url, { signal: AbortSignal.any([signal, AbortSignal.timeout(4000)]) });
        if (response.ok) return response.json();
        if (response.status < 500) throw new Error(`upstream HTTP ${response.status}`);
        lastError = new Error(`upstream HTTP ${response.status}`);
      } catch (error) { lastError = error; }
      if (attempt === 0) await new Promise((resolve, reject) => { const timer = setTimeout(resolve, 250); signal.addEventListener('abort', () => { clearTimeout(timer); reject(signal.reason); }, { once: true }); });
    }
    throw lastError;
  }
  async function locate(input, signal) {
    if (input.lat !== undefined && input.lon !== undefined) return { name: `${Number(input.lat).toFixed(4)}, ${Number(input.lon).toFixed(4)}`, latitude: Number(input.lat), longitude: Number(input.lon) };
    const key = input.q.trim().toLocaleLowerCase('en-US'); const cached = geocodes.get(key); if (cached) return cached;
    const url = new URL(GEO_URL); url.search = new URLSearchParams({ name: input.q, count: '1', language: 'en', format: 'json' });
    const data = await upstream(url, signal); const found = data.results?.[0] ?? null;
    const location = found ? {
      name: found.name, admin1: found.admin1, country: found.country,
      latitude: Number(found.latitude), longitude: Number(found.longitude), timezone: found.timezone
    } : null;
    if (location) geocodes.set(key, location); return location;
  }
  async function query(kind, input, signal) {
    const location = await locate(input, signal);
    if (!location) return { answer: `The location “${input.q}” was not found, so current weather data is unavailable. Please check the spelling or provide latitude and longitude.`, location: null, source: 'open-meteo', retrieved_at: now().toISOString() };
    const days = Math.min(7, Math.max(1, Number.parseInt(input.days ?? '3', 10) || 3));
    const coord = `${Number(location.latitude).toFixed(4)},${Number(location.longitude).toFixed(4)}`;
    const key = `${kind}:${coord}:${kind === 'forecast' ? days : ''}`; const hit = weather.get(key);
    if (hit && now().getTime() - hit.cachedAt < 60_000) return hit.payload;
    const url = new URL(FORECAST_URL); const params = { latitude: location.latitude, longitude: location.longitude, timezone: 'auto' };
    if (kind === 'current') params.current = 'temperature_2m,relative_humidity_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m,wind_direction_10m,surface_pressure,cloud_cover';
    else Object.assign(params, { daily: 'weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,wind_speed_10m_max', forecast_days: String(days) });
    url.search = new URLSearchParams(params);
    try {
      const body = await upstream(url, signal); const retrievedAt = now().toISOString();
      const payload = kind === 'current' ? currentPayload(location, body, retrievedAt) : forecastPayload(location, body, days, retrievedAt);
      weather.set(key, { payload, cachedAt: now().getTime() }); stale.set(key, { body, location, retrievedAt }); return payload;
    } catch (error) {
      const old = stale.get(key); if (!old) throw error;
      return kind === 'current' ? currentPayload(old.location, old.body, old.retrievedAt, true) : forecastPayload(old.location, old.body, days, old.retrievedAt, true);
    }
  }
  return { query, probe: async (signal) => { const url = new URL(FORECAST_URL); url.search = new URLSearchParams({ latitude: '0', longitude: '0', current: 'temperature_2m', timezone: 'auto' }); await upstream(url, signal); return true; } };
}
