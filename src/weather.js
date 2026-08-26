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

const WMO_SLUG = new Map([
  [0, 'clear'], [1, 'mainly_clear'], [2, 'partly_cloudy'], [3, 'overcast'],
  [45, 'fog'], [48, 'depositing_rime_fog'], [51, 'light_drizzle'],
  [53, 'moderate_drizzle'], [55, 'dense_drizzle'], [56, 'light_freezing_drizzle'],
  [57, 'dense_freezing_drizzle'], [61, 'slight_rain'], [63, 'moderate_rain'],
  [65, 'heavy_rain'], [66, 'light_freezing_rain'], [67, 'heavy_freezing_rain'],
  [71, 'slight_snow_fall'], [73, 'moderate_snow_fall'], [75, 'heavy_snow_fall'],
  [77, 'snow_grains'], [80, 'slight_rain_showers'], [81, 'moderate_rain_showers'],
  [82, 'violent_rain_showers'], [85, 'slight_snow_showers'], [86, 'heavy_snow_showers'],
  [95, 'thunderstorm'], [96, 'thunderstorm_with_slight_hail'],
  [99, 'thunderstorm_with_heavy_hail']
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

export function conditionSlug(code) {
  return WMO_SLUG.get(Number(code)) ?? `unknown_${code}`;
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
function scalar(value) { return Array.isArray(value) ? value[0] : value; }
function nonEmpty(value) {
  const text = scalar(value);
  return text !== undefined && text !== null && String(text).trim() ? String(text).trim() : undefined;
}

export function normalizeInput(input = {}) {
  const q = nonEmpty(input.q) ?? nonEmpty(input.location) ?? nonEmpty(input.city) ?? nonEmpty(input.place);
  const lat = scalar(input.lat) ?? scalar(input.latitude);
  const lon = scalar(input.lon) ?? scalar(input.longitude);
  return { ...input, q, lat, lon };
}

function windMs(value, sourceUnit = '') {
  const speed = Number(value);
  const unitText = String(sourceUnit).toLowerCase();
  if (unitText.includes('km')) return speed / 3.6;
  if (unitText.includes('mph')) return speed * 0.44704;
  return speed;
}

function utcDate(value, timezone = 'UTC') {
  if (!value) return null;
  const text = String(value);
  if (/[zZ]|[+-]\d\d(?::?\d\d)?$/.test(text)) {
    const date = new Date(text);
    return Number.isFinite(date.getTime()) ? date : null;
  }
  const local = text.length === 16 ? `${text}:00` : text;
  const assumedUtc = new Date(`${local}Z`);
  if (!Number.isFinite(assumedUtc.getTime())) return null;
  if (!timezone || timezone === 'UTC') return assumedUtc;
  try {
    const parts = new Intl.DateTimeFormat('en-US', {
      timeZone: timezone, hour12: false, hourCycle: 'h23',
      year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit', second: '2-digit'
    }).formatToParts(assumedUtc);
    const get = (type) => Number(parts.find((part) => part.type === type)?.value);
    const asUtc = Date.UTC(get('year'), get('month') - 1, get('day'), get('hour'), get('minute'), get('second'));
    return new Date(assumedUtc.getTime() - (asUtc - assumedUtc.getTime()));
  } catch {
    return assumedUtc;
  }
}

function isoMinute(value, timezone = 'UTC') {
  const date = utcDate(value, timezone);
  return date ? `${date.toISOString().slice(0, 16)}Z` : String(value);
}

function isoSecond(value, timezone = 'UTC') {
  const text = nonEmpty(value);
  if (!text) return undefined;
  const date = utcDate(text, timezone);
  return date ? date.toISOString().replace('.000Z', 'Z') : text;
}

function isPrecipitationCode(code) {
  const value = Number(code);
  return (value >= 51 && value <= 67) || (value >= 71 && value <= 86) || (value >= 95 && value <= 99);
}

function horizonCondition(codes) {
  if (codes.some(isPrecipitationCode)) {
    return codes.some((code) => Number(code) >= 95)
      ? 'scattered showers or thunderstorms'
      : 'rain or showers';
  }
  if (codes.some((code) => Number(code) === 45 || Number(code) === 48)) return 'fog';
  if (codes.some((code) => Number(code) === 3)) return 'overcast';
  if (codes.some((code) => Number(code) === 2)) return 'partly cloudy';
  if (codes.some((code) => Number(code) === 1)) return 'mainly clear';
  if (codes.some((code) => Number(code) === 0)) return 'clear';
  return undefined;
}

function next24Summary(body, target) {
  const h = body.hourly ?? {};
  const times = h.time ?? [];
  const timezone = body.timezone ?? 'UTC';
  const start = utcDate(target, timezone);
  const end = start ? start.getTime() + 24 * 60 * 60 * 1000 : undefined;
  const rows = [];
  for (let i = 0; i < times.length; i++) {
    const date = utcDate(times[i], timezone);
    if (!date || (start && (date.getTime() < start.getTime() || date.getTime() > end))) continue;
    rows.push({
      temperature: h.temperature_2m?.[i],
      probability: h.precipitation_probability?.[i],
      code: h.weather_code?.[i]
    });
  }
  if (!rows.length) return null;
  const temperatures = rows.map((row) => Number(row.temperature)).filter(Number.isFinite);
  const codes = rows.map((row) => Number(row.code)).filter(Number.isFinite);
  const wet = rows.some((row) => isPrecipitationCode(row.code));
  const probabilities = rows
    .filter((row) => !wet || isPrecipitationCode(row.code))
    .map((row) => Number(row.probability))
    .filter(Number.isFinite);
  return {
    low_c: temperatures.length ? Math.min(...temperatures) : undefined,
    high_c: temperatures.length ? Math.max(...temperatures) : undefined,
    condition: horizonCondition(codes),
    probability_min_pct: probabilities.length ? Math.min(...probabilities) : undefined,
    probability_max_pct: probabilities.length ? Math.max(...probabilities) : undefined
  };
}

function datePart(value) {
  const match = String(scalar(value) ?? '').match(/^\d{4}-\d{2}-\d{2}/);
  return match?.[0];
}

function nextDate(value) {
  const date = new Date(`${value}T12:00:00Z`);
  if (!Number.isFinite(date.getTime())) return value;
  date.setUTCDate(date.getUTCDate() + 1);
  return date.toISOString().slice(0, 10);
}

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
  const horizon = next24Summary(body, c.time);
  const parts = [];
  if (present(c.temperature_2m)) parts.push(`${one(c.temperature_2m)}C`);
  if (present(c.precipitation)) parts.push(`${one(c.precipitation)}mm`);
  if (present(c.wind_speed_10m)) parts.push(`${one(windMs(c.wind_speed_10m, unit(u, 'wind_speed_10m')))}m/s`);
  if (present(c.weather_code)) parts.push(conditionSlug(c.weather_code));
  const place = location.name && location.country ? `${location.name}, ${location.country}` : location.name ?? label(location);
  let answer;
  if (present(c.temperature_2m) && horizon) {
    answer = `The current temperature in ${place} is ${one(c.temperature_2m)}C`;
    if (present(c.apparent_temperature)) answer += `, and it feels like ${one(c.apparent_temperature)}C`;
    if (horizon) {
      if (present(horizon.low_c) && present(horizon.high_c)) {
        answer += `. Over the next 24 hours, temperatures range from ${integer(horizon.low_c)}C to ${integer(horizon.high_c)}C`;
      }
      if (horizon.condition) answer += `, with a chance of ${horizon.condition}`;
      if (present(horizon.probability_min_pct) && present(horizon.probability_max_pct)) {
        answer += `${horizon.condition ? ', and' : ' with'} precipitation chances ranging from ${integer(horizon.probability_min_pct)}% to ${integer(horizon.probability_max_pct)}%`;
      }
    }
    if (c.time) answer += `. As of ${isoMinute(c.time, body.timezone ?? 'UTC')}`;
    answer += '.';
  } else {
    answer = `${location.name ?? label(location)} current: ${parts.join(', ')}`;
    if (c.time) answer += `. As of ${isoMinute(c.time, body.timezone ?? 'UTC')}`;
    answer += '.';
  }
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
      wind_speed_ms: present(c.wind_speed_10m) ? Number(one(windMs(c.wind_speed_10m, unit(u, 'wind_speed_10m')))) : undefined,
      wind_speed_kmh: present(c.wind_speed_10m) ? Number(one(windMs(c.wind_speed_10m, unit(u, 'wind_speed_10m')) * 3.6)) : undefined,
      wind_direction_deg: present(c.wind_direction_10m) ? Number(integer(c.wind_direction_10m)) : undefined,
      wind_direction_compass: present(c.wind_direction_10m) ? compass(c.wind_direction_10m).short : undefined,
      surface_pressure_hpa: present(c.surface_pressure) ? Number(one(c.surface_pressure)) : undefined,
      cloud_cover_pct: present(c.cloud_cover) ? Number(integer(c.cloud_cover)) : undefined,
      weather_code: present(c.weather_code) ? Number(c.weather_code) : undefined,
      conditions: present(c.weather_code) ? conditions(c.weather_code) : undefined,
      condition_slug: present(c.weather_code) ? conditionSlug(c.weather_code) : undefined,
      observed_at: c.time
    },
    next_24h: horizon ?? undefined,
    source: 'open-meteo', retrieved_at: stale ? `${retrievedAt} (stale)` : retrievedAt
  };
}

function forecastRows(body) {
  const h = body.hourly ?? {};
  const hu = body.hourly_units ?? {};
  const times = h.time ?? [];
  const timezone = body.timezone ?? 'UTC';
  const rows = [];
  for (let i = 0; i < times.length; i++) {
    const row = { time: isoMinute(times[i], timezone) };
    if (present(h.temperature_2m?.[i])) row.temp_c = Number(one(h.temperature_2m[i]));
    if (present(h.precipitation?.[i])) row.precip_mm = Number(one(h.precipitation[i]));
    if (present(h.precipitation_probability?.[i])) row.precip_probability_pct = Number(integer(h.precipitation_probability[i]));
    if (present(h.wind_speed_10m?.[i])) row.wind_ms = Number(one(windMs(h.wind_speed_10m[i], hu.wind_speed_10m ?? 'm/s')));
    if (present(h.weather_code?.[i])) row.conditions = conditionSlug(h.weather_code[i]);
    rows.push(row);
  }
  return rows;
}

function nearestRow(rows, target) {
  if (!rows.length) return null;
  const targetDate = utcDate(target) ?? new Date();
  return rows.reduce((best, row) => {
    const bestDistance = Math.abs((utcDate(best.time)?.getTime() ?? 0) - targetDate.getTime());
    const distance = Math.abs((utcDate(row.time)?.getTime() ?? 0) - targetDate.getTime());
    return distance < bestDistance ? row : best;
  });
}

export function forecastPayload(location, body, days, retrievedAt, stale = false, options = {}) {
  location = { ...location, timezone: location.timezone === 'auto' || !location.timezone ? body.timezone : location.timezone };
  const d = body.daily ?? {}; const u = body.daily_units ?? {};
  const count = Math.min(days, d.time?.length ?? 0); const sentences = [];
  for (let i = 0; i < count; i++) {
    const labelText = i === 0 ? 'today' : i === 1 ? 'tomorrow' : d.time[i];
    const facts = [`${labelText}`];
    if (present(d.temperature_2m_max?.[i])) facts.push(`high ${integer(d.temperature_2m_max[i])}C`);
    if (present(d.temperature_2m_min?.[i])) facts.push(`low ${integer(d.temperature_2m_min[i])}C`);
    if (present(d.weather_code?.[i])) facts.push(conditionSlug(d.weather_code[i]));
    sentences.push(facts.join(' '));
  }
  const rows = forecastRows(body);
  const nearest = nearestRow(rows, options.requestStart ?? retrievedAt);
  const place = location.name && location.country ? `${location.name}, ${location.country}` : location.name ?? label(location);
  const startStamp = isoSecond(options.requestStart, body.timezone ?? 'UTC');
  const endStamp = isoSecond(options.requestEnd, body.timezone ?? 'UTC');
  let answer = startStamp && endStamp
    ? `The 48-hour hourly weather forecast for ${place} starts from ${startStamp} UTC with a cutoff deadline of ${endStamp} UTC. ${sentences.join('; ')}`
    : `${location.name ?? label(location)} forecast: ${sentences.join('; ')}`;
  if (nearest) {
    const nearestFacts = [];
    if (present(nearest.temp_c)) nearestFacts.push(`${one(nearest.temp_c)}C`);
    if (present(nearest.precip_mm)) nearestFacts.push(`${one(nearest.precip_mm)}mm`);
    if (present(nearest.wind_ms)) nearestFacts.push(`${one(nearest.wind_ms)}m/s`);
    if (nearest.conditions) nearestFacts.push(nearest.conditions);
    answer += `. Nearest hour ${nearest.time}: ${nearestFacts.join(', ')}`;
  }
  answer += '.';
  if (stale) answer += ' This is the most recent cached forecast because the live weather service is temporarily unavailable.';
  return { answer, location, daily: d, daily_units: u, forecast: rows, source: 'open-meteo', retrieved_at: stale ? `${retrievedAt} (stale)` : retrievedAt };
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
    if (!input.q) return null;
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
    input = normalizeInput(input);
    const location = await locate(input, signal);
    if (!location) return { answer: `The location “${input.q ?? 'provided input'}” was not found, so current weather data is unavailable. Please check the spelling or provide latitude and longitude.`, location: null, source: 'open-meteo', retrieved_at: now().toISOString() };
    const days = Math.min(7, Math.max(1, Number.parseInt(input.days ?? '2', 10) || 2));
    const coord = `${Number(location.latitude).toFixed(4)},${Number(location.longitude).toFixed(4)}`;
    const requestStart = scalar(input.start_time) ?? scalar(input.startTime) ?? scalar(input.start);
    const requestEnd = scalar(input.end_time) ?? scalar(input.endTime) ?? scalar(input.end);
    const startDate = datePart(requestStart); const endDate = datePart(requestEnd);
    const fetchEndDate = kind === 'forecast' && startDate
      ? (endDate && endDate > startDate ? endDate : nextDate(startDate))
      : endDate;
    const key = `${kind}:${coord}:${kind === 'forecast' ? `${days}:${startDate ?? ''}:${fetchEndDate ?? ''}` : ''}`; const hit = weather.get(key);
    if (hit && now().getTime() - hit.cachedAt < 60_000) return hit.payload;
    const url = new URL(FORECAST_URL); const params = { latitude: location.latitude, longitude: location.longitude, timezone: 'UTC', wind_speed_unit: 'ms' };
    if (kind === 'current') {
      params.current = 'temperature_2m,relative_humidity_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m,wind_direction_10m,surface_pressure,cloud_cover';
      params.hourly = 'temperature_2m,precipitation,precipitation_probability,weather_code';
      params.forecast_hours = '25';
    }
    else {
      Object.assign(params, {
        daily: 'weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,wind_speed_10m_max',
        hourly: 'temperature_2m,precipitation,weather_code,wind_speed_10m'
      });
      if (startDate) {
        params.start_date = startDate;
        params.end_date = fetchEndDate ?? nextDate(startDate);
      } else params.forecast_days = String(days);
    }
    url.search = new URLSearchParams(params);
    try {
      const body = await upstream(url, signal); const retrievedAt = now().toISOString();
      const options = { requestStart, requestEnd };
      const payload = kind === 'current' ? currentPayload(location, body, retrievedAt) : forecastPayload(location, body, days, retrievedAt, false, options);
      weather.set(key, { payload, cachedAt: now().getTime() }); stale.set(key, { body, location, retrievedAt, options }); return payload;
    } catch (error) {
      const old = stale.get(key); if (!old) throw error;
      return kind === 'current' ? currentPayload(old.location, old.body, old.retrievedAt, true) : forecastPayload(old.location, old.body, days, old.retrievedAt, true, old.options);
    }
  }
  return { query, probe: async (signal) => { const url = new URL(FORECAST_URL); url.search = new URLSearchParams({ latitude: '0', longitude: '0', current: 'temperature_2m', timezone: 'auto' }); await upstream(url, signal); return true; } };
}
