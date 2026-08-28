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
const WMO_CODE_BY_SLUG = new Map([...WMO_SLUG.entries()].map(([code, slug]) => [slug, code]));

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

function promptLocation(value) {
  const text = nonEmpty(value);
  if (!text || !/[?!.]|\b(?:weather|forecast|temperature|conditions?|storm|wind|severe)\b/i.test(text)) return text;
  const stop = '(?=\\s*,?\\s*(?:including|with|starting|beginning|from|before|and|that|please|strong enough|expected|should|today|tomorrow|over\\s+the\\s+next|for\\s+the\\s+next|next)\\b|[?!.]|$)';
  const clean = (candidate) => candidate
    .replace(/\s*\(?\s*(?:latitude|lat|longitude|lon)\b.*$/i, '')
    .replace(/[,'"]+$/, '')
    .trim();
  const parenthetical = text.match(/\(\s*([A-Z][A-Za-z.'-]*(?:\s+[A-Z][A-Za-z.'-]*){0,4}(?:,\s*[A-Z][A-Za-z.'-]*){0,2})\s*\)/);
  if (parenthetical?.[1]) return parenthetical[1].trim();
  const namedLocation = text.match(/\blocation\s+([A-Z][A-Za-z.'-]*(?:\s+[A-Z][A-Za-z.'-]*){0,4}(?:,\s*[A-Z][A-Za-z.'-]*){0,2})(?=\s*\(|\s*,?\s*(?:including|with|starting|beginning|from|before|and|that|please|over\s+the\s+next|for\s+the\s+next|next|today|tomorrow)\b|[?!.]|$)/i);
  if (namedLocation?.[1]) return clean(namedLocation[1]);
  const patterns = [
    new RegExp(`\\b(?:forecast|storm(?:\\s+alert)?|severe weather|wind(?:\\s+speed(?:s)?)?|temperature|conditions?)\\b.*\\bfor\\s+(?!the\\s+next\\b|next\\b)(.+?)${stop}`, 'i'),
    new RegExp(`\\b(?:weather(?:\\s+forecast)?|forecast|storm(?:\\s+alert)?|severe weather|wind(?:\\s+speed)?|temperature|conditions?)\\s+(?:for|in|at|over)\\s+(.+?)${stop}`, 'i'),
    new RegExp(`\\b(?:for|in|at|over)\\s+(.+?)${stop}`, 'i')
  ];
  for (const pattern of patterns) {
    const match = text.match(pattern);
    const candidate = clean(match?.[1] ?? '');
    if (candidate) return candidate;
  }
  return text;
}

function promptCoordinates(value) {
  const text = String(nonEmpty(value) ?? '');
  if (!text) return {};
  const latitude = text.match(/\b(?:latitude|lat)\s*[:=]?\s*(-?\d+(?:\.\d+)?)/i)?.[1];
  const longitude = text.match(/\b(?:longitude|lon)\s*[:=]?\s*(-?\d+(?:\.\d+)?)/i)?.[1];
  if (latitude !== undefined && longitude !== undefined) return { latitude, longitude };
  const pair = text.match(/\bcoordinates?\s*[:=]?\s*\(?\s*(-?\d+(?:\.\d+)?)\s*[,/]\s*(-?\d+(?:\.\d+)?)\s*\)?/i);
  return pair ? { latitude: pair[1], longitude: pair[2] } : {};
}

export function isForecastRequest(value) {
  const text = String(nonEmpty(value) ?? '').toLowerCase();
  if (!text) return false;
  const shortHorizon = /\bnext\s+24\s+hours?\b/.test(text);
  const extendedShape = /\b(?:hourly|daily|tomorrow|multi[- ]day|\d+\s*[- ]?day(?:s)?|\d+\s*[- ]?hours?)\b/.test(text);
  return !shortHorizon && (extendedShape || /\bforecast\b/.test(text));
}

function promptDays(value) {
  const text = String(nonEmpty(value) ?? '');
  const dayMatch = text.match(/\b(\d{1,2})\s*[- ]?day(?:s)?\b/i);
  if (dayMatch) return Number(dayMatch[1]);
  const hours = promptHours(text);
  if (hours !== undefined) return Math.ceil(hours / 24);
  return undefined;
}

function promptHours(value) {
  const text = String(nonEmpty(value) ?? '');
  const match = text.match(/\b(\d{1,3})\s*[- ]?(?:hours?|h)\b/i);
  if (!match) return undefined;
  const hours = Number(match[1]);
  return Number.isInteger(hours) && hours > 0 ? hours : undefined;
}

function positiveInteger(value) {
  const number = Number(scalar(value));
  return Number.isInteger(number) && number > 0 ? number : undefined;
}

function requestTextInput(input, locationInput) {
  return nonEmpty(input.request_text)
    ?? nonEmpty(input.question)
    ?? nonEmpty(input.prompt)
    ?? nonEmpty(input.text)
    ?? locationInput;
}

export function normalizeInput(input = {}) {
  const locationInput = nonEmpty(input.q) ?? nonEmpty(input.location) ?? nonEmpty(input.city) ?? nonEmpty(input.place) ?? nonEmpty(input.query);
  const requestText = requestTextInput(input, locationInput);
  const q = locationInput && locationInput !== requestText ? locationInput : promptLocation(locationInput ?? requestText);
  const coordinates = promptCoordinates(requestText);
  const lat = scalar(input.lat) ?? scalar(input.latitude) ?? coordinates.latitude;
  const lon = scalar(input.lon) ?? scalar(input.longitude) ?? coordinates.longitude;
  const requestStart = nonEmpty(input.start_time) ?? nonEmpty(input.startTime) ?? nonEmpty(input.start_date) ?? nonEmpty(input.startDate) ?? nonEmpty(input.start);
  const requestEnd = nonEmpty(input.end_time) ?? nonEmpty(input.endTime) ?? nonEmpty(input.end_date) ?? nonEmpty(input.endDate) ?? nonEmpty(input.end);
  const rawRequestedHours = positiveInteger(input.hours)
    ?? positiveInteger(input.forecast_hours)
    ?? positiveInteger(input.forecastHours)
    ?? positiveInteger(input.horizon_hours)
    ?? positiveInteger(input.horizonHours)
    ?? promptHours(requestText);
  const requestedHours = rawRequestedHours === undefined ? undefined : Math.min(7 * 24, rawRequestedHours);
  const inferredDays = promptDays(requestText);
  const requestedFields = nonEmpty(input.fields) ?? (isForecastRequest(requestText) ? requestText : undefined);
  return {
    ...input, q, lat, lon, request_text: requestText,
    days: scalar(input.days) ?? (requestedHours ? Math.ceil(requestedHours / 24) : inferredDays),
    hours: requestedHours,
    fields: requestedFields,
    start_time: requestStart, end_time: requestEnd
  };
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

function dateSpanDays(start, end) {
  if (!start || !end) return undefined;
  const startDate = new Date(`${start}T00:00:00Z`);
  const endDate = new Date(`${end}T00:00:00Z`);
  if (!Number.isFinite(startDate.getTime()) || !Number.isFinite(endDate.getTime())) return undefined;
  const span = Math.round((endDate.getTime() - startDate.getTime()) / 86_400_000) + 1;
  return span > 0 ? span : undefined;
}

function hasClock(value) {
  return /T\d{2}:\d{2}/.test(String(scalar(value) ?? ''));
}

function requestStamp(value, timezone = 'UTC') {
  const text = nonEmpty(value);
  if (!text) return undefined;
  return /^\d{4}-\d{2}-\d{2}$/.test(text) ? text : isoSecond(text, timezone);
}

function forecastWindowLabel(start, end, days) {
  const span = dateSpanDays(datePart(start), datePart(end));
  if (span && !hasClock(start) && !hasClock(end)) return `${span}-day hourly weather forecast`;
  if (hasClock(start) || hasClock(end)) return '48-hour hourly weather forecast';
  return `${days}-day hourly weather forecast`;
}

function requestedFieldTerms(fields) {
  const text = String(scalar(fields) ?? '').toLowerCase().replace(/[_-]+/g, ' ');
  if (!text) return [];
  const terms = [];
  if (text.includes('temperature')) terms.push('temperature in Celsius');
  if (text.includes('precipitation probability')) terms.push('precipitation probability');
  else if (text.includes('precipitation')) terms.push('precipitation');
  if (text.includes('wind speed')) terms.push('wind speed');
  return terms;
}

function joinTerms(terms) {
  if (terms.length < 2) return terms[0] ?? '';
  if (terms.length === 2) return `${terms[0]} and ${terms[1]}`;
  return `${terms.slice(0, -1).join(', ')}, and ${terms.at(-1)}`;
}

function requestedFieldPhrase(fields) {
  const terms = requestedFieldTerms(fields);
  return terms.length ? ` Including ${joinTerms(terms)}.` : '';
}

function naturalRequestFraming(requestText, place, days, fields) {
  const text = String(scalar(requestText) ?? '').trim();
  if (!text) return '';
  const start = text.match(/\bstarting\s+(?:from|on)\s+(.+?)(?=\s*,|\s+including\b|\s+with\b|\s+before\b|[?!.]|$)/i)?.[1]?.trim();
  const cutoff = text.match(/\bbefore\s+the\s+cutoff\s+(?:time|deadline)\s+of\s+([^,?.!]+(?:Z|UTC)?)/i)?.[1]?.trim();
  if (!start && !cutoff) return '';
  const terms = requestedFieldTerms(fields);
  let framing = `The ${days}-day hourly weather forecast for ${place}`;
  if (start) framing += ` starting from ${start}`;
  if (terms.length) framing += `, including ${joinTerms(terms)}`;
  if (cutoff) framing += `, before the cutoff time of ${cutoff}`;
  return `${framing}. Available forecast:`;
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

function forecastCondition(code) {
  const value = Number(code);
  if (value >= 95 && value <= 99) return 'Thunderstorm';
  if (value >= 51 && value <= 57) return 'Drizzle';
  if ((value >= 61 && value <= 67) || (value >= 80 && value <= 82)) return 'Rain';
  if (value >= 71 && value <= 77) return 'Snow';
  if (value >= 85 && value <= 86) return 'Snow showers';
  return conditions(code);
}

function decimal(value) {
  const text = one(value);
  return text.endsWith('.0') ? text.slice(0, -2) : text;
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

function forecastRows(body, maxHours) {
  const h = body.hourly ?? {};
  const hu = body.hourly_units ?? {};
  const times = h.time ?? [];
  const timezone = body.timezone ?? 'UTC';
  const rows = [];
  const count = maxHours === undefined ? times.length : Math.min(times.length, maxHours);
  for (let i = 0; i < count; i++) {
    const row = { time: isoMinute(times[i], timezone) };
    if (present(h.temperature_2m?.[i])) row.temp_c = Number(one(h.temperature_2m[i]));
    if (present(h.precipitation?.[i])) row.precip_mm = Number(one(h.precipitation[i]));
    if (present(h.precipitation_probability?.[i])) row.precip_probability_pct = Number(integer(h.precipitation_probability[i]));
    if (present(h.wind_speed_10m?.[i])) row.wind_ms = Number(one(windMs(h.wind_speed_10m[i], hu.wind_speed_10m ?? 'm/s')));
    if (present(h.dew_point_2m?.[i])) row.dew_point_c = Number(one(h.dew_point_2m[i]));
    if (present(h.weather_code?.[i])) row.conditions = conditionSlug(h.weather_code[i]);
    rows.push(row);
  }
  return rows;
}

function stormRows(body, maxHours) {
  const h = body.hourly ?? {};
  const hu = body.hourly_units ?? {};
  const times = h.time ?? [];
  const timezone = body.timezone ?? 'UTC';
  const rows = [];
  const count = maxHours === undefined ? times.length : Math.min(times.length, maxHours);
  for (let i = 0; i < count; i++) {
    const row = { time: isoMinute(times[i], timezone) };
    if (present(h.temperature_2m?.[i])) row.temp_c = Number(one(h.temperature_2m[i]));
    if (present(h.precipitation?.[i])) row.precip_mm = Number(one(h.precipitation[i]));
    if (present(h.precipitation_probability?.[i])) row.precip_probability_pct = Number(integer(h.precipitation_probability[i]));
    if (present(h.wind_speed_10m?.[i])) {
      const speed = windMs(h.wind_speed_10m[i], hu.wind_speed_10m ?? 'm/s');
      row.wind_ms = Number(one(speed));
      row.wind_kmh = Number(one(speed * 3.6));
      row.wind_knots = Number(one(speed * 1.94384449));
    }
    if (present(h.wind_gusts_10m?.[i])) {
      const gust = windMs(h.wind_gusts_10m[i], hu.wind_gusts_10m ?? 'm/s');
      row.gust_ms = Number(one(gust));
      row.gust_kmh = Number(one(gust * 3.6));
      row.gust_knots = Number(one(gust * 1.94384449));
    }
    if (present(h.wind_direction_10m?.[i])) {
      row.wind_direction_deg = Number(integer(h.wind_direction_10m[i]));
      row.wind_direction_compass = compass(h.wind_direction_10m[i]).short;
    }
    if (present(h.weather_code?.[i])) {
      row.weather_code = Number(h.weather_code[i]);
      row.conditions = conditionSlug(h.weather_code[i]);
      row.thunderstorm = Number(h.weather_code[i]) >= 95 && Number(h.weather_code[i]) <= 99;
    }
    rows.push(row);
  }
  return rows;
}

function maximum(values) {
  const finite = values.map(Number).filter(Number.isFinite);
  return finite.length ? Math.max(...finite) : undefined;
}

function stormRiskLabel(score) {
  if (score >= 0.75) return 'high';
  if (score >= 0.5) return 'elevated';
  if (score >= 0.25) return 'moderate';
  if (score >= 0.1) return 'low';
  return 'none';
}

function stormRiskScore(rows) {
  const peakGust = maximum(rows.map((row) => row.gust_kmh));
  const peakWind = maximum(rows.map((row) => row.wind_kmh));
  const precipitationProbability = maximum(rows.map((row) => row.precip_probability_pct));
  const peakPrecipitation = maximum(rows.map((row) => row.precip_mm));
  const thunderstorm = rows.some((row) => row.thunderstorm);
  // Keep the primary signal transparent: peak gusts at 160 km/h represent a
  // score of 1.0. Precipitation and thunderstorm evidence only raise the
  // score when the wind signal alone would understate disruption risk.
  const windScore = Math.max(peakGust ?? 0, peakWind ?? 0) / 160;
  const precipitationScore = Math.min(1, ((precipitationProbability ?? 0) / 100) * 0.4 + (peakPrecipitation ?? 0) / 25);
  const stormScore = thunderstorm ? 0.25 : 0;
  return Number(Math.min(1, Math.max(windScore, precipitationScore, stormScore)).toFixed(2));
}

function stormPlace(location) {
  return location.name ?? label(location);
}

export function stormPayload(location, body, days, retrievedAt, stale = false, options = {}) {
  location = { ...location, timezone: location.timezone === 'auto' || !location.timezone ? body.timezone : location.timezone };
  const d = body.daily ?? {};
  const du = body.daily_units ?? {};
  const requestedHours = options.requestedHours ?? Math.min(48, Math.max(1, days * 24));
  const rows = stormRows(body, requestedHours);
  const peakWindKmh = maximum(rows.map((row) => row.wind_kmh));
  const peakGustKmh = maximum(rows.map((row) => row.gust_kmh));
  const peakPrecipitation = maximum(rows.map((row) => row.precip_mm));
  const precipitationTotal = rows.map((row) => Number(row.precip_mm)).filter(Number.isFinite).reduce((sum, value) => sum + value, 0);
  const precipitationProbability = maximum(rows.map((row) => row.precip_probability_pct));
  const stormHours = rows.filter((row) => row.thunderstorm).map((row) => row.time);
  const riskScore = stormRiskScore(rows);
  const riskLabel = stormRiskLabel(riskScore);
  const highWindThresholdMs = 25 * 0.514444;
  const thresholdRows = rows.filter((row) => Number(row.wind_ms) > highWindThresholdMs);
  const hasWind = rows.some((row) => Number.isFinite(Number(row.wind_kmh)));
  const hasPrecipitation = rows.some((row) => Number.isFinite(Number(row.precip_mm)));
  const place = stormPlace(location);
  const windText = peakWindKmh === undefined ? 'sustained wind data was not returned' : `sustained winds up to ${decimal(peakWindKmh)} km/h`;
  const gustText = peakGustKmh === undefined ? 'peak gust data was not returned' : `peak gusts of ${decimal(peakGustKmh)} km/h`;
  const precipitationText = !hasPrecipitation
    ? 'precipitation data was not returned'
    : precipitationTotal === 0
      ? 'no precipitation'
    : peakPrecipitation !== undefined
      ? `precipitation up to ${decimal(peakPrecipitation)} mm per hour`
      : `precipitation totaling ${decimal(precipitationTotal)} mm`;
  let answer = `The forecast for ${place} over the next ${requestedHours} hours predicts ${windText} with ${gustText}, ${precipitationText}, and a ${riskLabel} risk score of ${riskScore.toFixed(2)}.`;
  if (stormHours.length) answer += ` Thunderstorms are possible during ${stormHours.slice(0, 4).join(', ')}${stormHours.length > 4 ? ' and other forecast hours' : ''}.`;
  else answer += ' No thunderstorm conditions were returned in the hourly forecast.';
  if (!hasWind) {
    answer += ' The sustained-wind threshold could not be evaluated because hourly wind data was not returned.';
  } else if (thresholdRows.length) {
    answer += ` Sustained winds exceed 25 knots during ${thresholdRows.slice(0, 4).map((row) => row.time).join(', ')}${thresholdRows.length > 4 ? ' and other forecast hours' : ''}.`;
  } else {
    answer += ' No periods with sustained winds above 25 knots are forecast.';
  }
  if (/\b(?:100u|100v|era5)\b/i.test(options.requestText ?? '')) {
    answer += ' The returned wind values are 10 m surface forecasts; 100u/100v model-level components are not supplied by this source.';
  }
  if (stale) answer += ' This is the most recent cached storm forecast because the live weather service is temporarily unavailable.';
  return {
    answer,
    location,
    storm: {
      risk_score: riskScore,
      risk_level: riskLabel,
      active_storm_systems: stormHours.length > 0,
      thunderstorm_hours: stormHours,
      sustained_wind_max_kmh: peakWindKmh,
      sustained_wind_max_ms: peakWindKmh === undefined ? undefined : Number((peakWindKmh / 3.6).toFixed(1)),
      sustained_wind_max_knots: peakWindKmh === undefined ? undefined : Number((peakWindKmh / 1.852).toFixed(1)),
      peak_gust_kmh: peakGustKmh,
      peak_gust_ms: peakGustKmh === undefined ? undefined : Number((peakGustKmh / 3.6).toFixed(1)),
      peak_gust_knots: peakGustKmh === undefined ? undefined : Number((peakGustKmh / 1.852).toFixed(1)),
      precipitation_total_mm: Number(precipitationTotal.toFixed(1)),
      peak_hourly_precipitation_mm: peakPrecipitation,
      precipitation_probability_max_pct: precipitationProbability,
      sustained_above_25_knots: thresholdRows.length > 0,
      sustained_above_25_knots_hours: thresholdRows.map((row) => row.time),
      warning_data: 'not-provided-by-open-meteo'
    },
    daily: d,
    daily_units: du,
    forecast: rows,
    requested_hours: requestedHours,
    source: 'open-meteo', retrieved_at: stale ? `${retrievedAt} (stale)` : retrievedAt
  };
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

function windowForecastAnswer(location, rows, hours, startStamp, endStamp) {
  if (!rows.length) return `${location.name ?? label(location)} forecast: no hourly forecast data was returned`;
  const temperatures = rows.map((row) => Number(row.temp_c)).filter(Number.isFinite);
  const codes = rows.map((row) => WMO_CODE_BY_SLUG.get(row.conditions)).filter((value) => value !== undefined);
  const totalPrecipitation = rows.map((row) => Number(row.precip_mm)).filter(Number.isFinite).reduce((sum, value) => sum + value, 0);
  const place = location.name && location.country ? `${location.name}, ${location.country}` : location.name ?? label(location);
  const range = temperatures.length
    ? `temperatures ranging from ${integer(Math.min(...temperatures))}C to ${integer(Math.max(...temperatures))}C`
    : 'hourly temperature data';
  const condition = horizonCondition(codes);
  let answer = `The forecast for the next ${hours} hours at ${place}`;
  if (startStamp && endStamp) answer += ` starts from ${startStamp} UTC with a cutoff deadline of ${endStamp} UTC`;
  else answer += ` is valid through ${rows.at(-1).time}`;
  answer += `, with ${range}`;
  if (condition) answer += ` and ${condition} conditions`;
  answer += '.';
  answer += totalPrecipitation === 0
    ? ' No precipitation is expected.'
    : totalPrecipitation < 1
      ? ` Minimal precipitation is expected (${decimal(totalPrecipitation)}mm total).`
      : ` Total precipitation is expected to reach ${decimal(totalPrecipitation)}mm.`;
  return answer;
}

export function forecastPayload(location, body, days, retrievedAt, stale = false, options = {}) {
  location = { ...location, timezone: location.timezone === 'auto' || !location.timezone ? body.timezone : location.timezone };
  const d = body.daily ?? {}; const u = body.daily_units ?? {};
  const count = Math.min(days, d.time?.length ?? 0); const sentences = [];
  const requestedFields = options.requestedFields;
  const includeRequestedFields = Boolean(requestedFields);
  for (let i = 0; i < count; i++) {
    const labelText = i === 0 ? 'today' : i === 1 ? 'tomorrow' : d.time[i];
    const facts = [`${labelText}`];
    if (present(d.temperature_2m_max?.[i])) facts.push(`high ${integer(d.temperature_2m_max[i])}C`);
    if (present(d.temperature_2m_min?.[i])) facts.push(`low ${integer(d.temperature_2m_min[i])}C`);
    if (present(d.weather_code?.[i])) facts.push(conditionSlug(d.weather_code[i]));
    if (includeRequestedFields && present(d.precipitation_probability_max?.[i])) facts.push(`precipitation probability ${integer(d.precipitation_probability_max[i])}%`);
    sentences.push(facts.join(' '));
  }
  const rows = forecastRows(body, options.requestedHours);
  const nearest = nearestRow(rows, options.requestStart ?? retrievedAt);
  const place = location.name && location.country ? `${location.name}, ${location.country}` : location.name ?? label(location);
  const startStamp = requestStamp(options.requestStart, body.timezone ?? 'UTC');
  const endStamp = requestStamp(options.requestEnd, body.timezone ?? 'UTC');
  const windowLabel = forecastWindowLabel(options.requestStart, options.requestEnd, days);
  const fieldPhrase = requestedFieldPhrase(requestedFields);
  const naturalFraming = naturalRequestFraming(options.requestText, place, days, requestedFields);
  let answer = options.requestedHours !== undefined && options.requestedHours <= 48
    ? windowForecastAnswer(location, rows, options.requestedHours, startStamp, endStamp)
    : options.renderHourly
    ? conciseHourlyForecastAnswer(location, body, days, options.requestedHours, requestedFields)
    : startStamp && endStamp
    ? (hasClock(options.requestStart) || hasClock(options.requestEnd)
      ? `The ${windowLabel} for ${place} starts from ${startStamp} UTC with a cutoff deadline of ${endStamp} UTC.${fieldPhrase} ${sentences.join('; ')}`
      : `The ${windowLabel} for ${place} runs from ${startStamp} through ${endStamp} UTC.${fieldPhrase} ${sentences.join('; ')}`)
    : naturalFraming
      ? `${naturalFraming} ${location.name ?? label(location)} forecast: ${sentences.join('; ')}`
      : `${location.name ?? label(location)} forecast: ${sentences.join('; ')}`;
  if (nearest && !options.renderHourly) {
    const nearestFacts = [];
    if (present(nearest.temp_c)) nearestFacts.push(`${one(nearest.temp_c)}C`);
    if (present(nearest.precip_mm)) nearestFacts.push(`${one(nearest.precip_mm)}mm`);
    if (includeRequestedFields && present(nearest.precip_probability_pct)) nearestFacts.push(`${integer(nearest.precip_probability_pct)}% precipitation probability`);
    if (present(nearest.wind_ms)) nearestFacts.push(`${one(nearest.wind_ms)}m/s`);
    if (nearest.conditions) nearestFacts.push(nearest.conditions);
    answer += `. Nearest hour ${nearest.time}: ${nearestFacts.join(', ')}`;
  }
  if (!answer.endsWith('.')) answer += '.';
  if (stale) answer += ' This is the most recent cached forecast because the live weather service is temporarily unavailable.';
  return {
    answer, location, daily: d, daily_units: u, forecast: rows,
    requested_hours: options.requestedHours,
    source: 'open-meteo', retrieved_at: stale ? `${retrievedAt} (stale)` : retrievedAt
  };
}

function forecastFieldSelection(fields) {
  const text = String(scalar(fields) ?? '').toLowerCase().replace(/[_-]+/g, ' ');
  const explicit = /\b(?:temperature|temp|2t|precipitation|precip|rain|snow|wind|gust|dew\s*point|weather|condition|cloud|probability|chance)\b/.test(text);
  if (!explicit) {
    return { temperature: true, precipitation: true, precipitationProbability: true, conditions: true, wind: true, dewPoint: true };
  }
  return {
    temperature: /\b(?:temperature|temp|2t)\b/.test(text),
    precipitation: /\b(?:precip|rain|snow)\b/.test(text) || /\bprecipitation\b(?!\s+(?:probability|chance)\b)/.test(text),
    precipitationProbability: /\b(?:probability|chance)\b/.test(text),
    conditions: /\b(?:weather|condition|cloud|rain|snow|drizzle|thunderstorm)\b/.test(text),
    wind: /\b(?:wind|gust)\b/.test(text),
    dewPoint: /\bdew\s*point\b/.test(text)
  };
}

function conciseHourlyForecastAnswer(location, body, days, requestedHours, requestedFields) {
  const rows = forecastRows(body, requestedHours);
  const place = location.name && location.country ? `${location.name}, ${location.country}` : location.name ?? label(location);
  if (!rows.length) return `${place} forecast: no forecast data was returned`;

  const fields = forecastFieldSelection(requestedFields);
  const parts = [];
  const temperatures = rows.map((row) => Number(row.temp_c)).filter(Number.isFinite);
  if (fields.temperature && temperatures.length) {
    parts.push(`temperatures range from ${decimal(Math.min(...temperatures))}C to ${decimal(Math.max(...temperatures))}C`);
  }

  const precipitation = rows.map((row) => Number(row.precip_mm)).filter(Number.isFinite);
  if (fields.precipitation) {
    if (!precipitation.length) parts.push('precipitation data was not returned');
    else {
      const total = precipitation.reduce((sum, value) => sum + value, 0);
      parts.push(total === 0 ? 'no precipitation is expected' : `total precipitation ${decimal(total)}mm`);
    }
  }

  if (fields.precipitationProbability) {
    const probabilities = rows.map((row) => Number(row.precip_probability_pct)).filter(Number.isFinite);
    if (probabilities.length) parts.push(`precipitation probability up to ${integer(Math.max(...probabilities))}%`);
  }

  if (fields.conditions) {
    const conditionsList = [...new Set(rows.map((row) => WMO_CODE_BY_SLUG.get(row.conditions)).filter((value) => value !== undefined).map(forecastCondition))];
    if (conditionsList.length) parts.push(`conditions include ${conditionsList.slice(0, 3).join(', ').toLowerCase()}`);
  }

  if (fields.wind) {
    const wind = rows.map((row) => Number(row.wind_ms) * 3.6).filter(Number.isFinite);
    if (wind.length) parts.push(`wind speeds up to ${decimal(Math.max(...wind))} km/h`);
  }

  if (fields.dewPoint) {
    const dewPoints = rows.map((row) => Number(row.dew_point_c)).filter(Number.isFinite);
    if (dewPoints.length) parts.push(`dew points range from ${decimal(Math.min(...dewPoints))}C to ${decimal(Math.max(...dewPoints))}C`);
  }

  const dailyDates = (body.daily?.time ?? []).slice(0, Math.min(days, body.daily?.time?.length ?? 0)).map(datePart).filter(Boolean);
  const firstDate = dailyDates[0] ?? datePart(rows[0].time);
  const lastDate = dailyDates.at(-1) ?? datePart(rows.at(-1).time);
  const period = firstDate && lastDate
    ? firstDate === lastDate ? firstDate : `${firstDate} through ${lastDate}`
    : `${days}-day window`;
  let answer = `${place} forecast for ${period}`;
  if (parts.length) answer += `: ${parts.join('; ')}`;
  answer += `. Hourly data contains ${rows.length} rows.`;
  return answer;
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
    if (input.lat !== undefined && input.lon !== undefined) {
      const candidate = nonEmpty(input.q);
      const placeName = candidate && !/\b(?:weather|forecast|wind|storm|severe|latitude|longitude|temperature|conditions?|next|hours?|today|tomorrow|using|variable|disruption)\b/i.test(candidate)
        ? candidate
        : `${Number(input.lat).toFixed(4)}, ${Number(input.lon).toFixed(4)}`;
      return { name: placeName, latitude: Number(input.lat), longitude: Number(input.lon) };
    }
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
    const responseKind = kind === 'current' && isForecastRequest(input.request_text) ? 'forecast' : kind;
    const location = await locate(input, signal);
    if (!location) return { answer: `The location “${input.q ?? 'provided input'}” was not found, so current weather data is unavailable. Please check the spelling or provide latitude and longitude.`, location: null, source: 'open-meteo', retrieved_at: now().toISOString() };
    const explicitDays = Number.parseInt(scalar(input.days) ?? '', 10);
    const requestedHours = positiveInteger(input.hours);
    const stormHours = responseKind === 'storm'
      ? requestedHours ?? (/\btoday\b/i.test(input.request_text ?? '') ? 24 : 48)
      : requestedHours;
    const coord = `${Number(location.latitude).toFixed(4)},${Number(location.longitude).toFixed(4)}`;
    const requestStart = scalar(input.start_time);
    const requestEnd = scalar(input.end_time);
    const startDate = datePart(requestStart); const endDate = datePart(requestEnd);
    const boundedDays = dateSpanDays(startDate, endDate);
    const defaultDays = Number.isFinite(explicitDays) && explicitDays > 0
      ? explicitDays
      : stormHours ? Math.ceil(stormHours / 24) : 3;
    const timestampFallbackDays = startDate && endDate && boundedDays === 1 && (hasClock(requestStart) || hasClock(requestEnd)) && !Number.isFinite(explicitDays) ? 2 : undefined;
    const days = Math.min(7, Math.max(1, timestampFallbackDays ?? boundedDays ?? defaultDays));
    const fetchEndDate = (responseKind === 'forecast' || responseKind === 'storm') && startDate
      ? (endDate && endDate > startDate ? endDate : nextDate(startDate))
      : endDate;
    const key = `${responseKind}:${coord}:${responseKind === 'forecast' || responseKind === 'storm' ? `${days}:${stormHours ?? ''}:${startDate ?? ''}:${fetchEndDate ?? ''}` : ''}`; const hit = weather.get(key);
    if (hit && now().getTime() - hit.cachedAt < 60_000) return hit.payload;
    const url = new URL(FORECAST_URL); const params = { latitude: location.latitude, longitude: location.longitude, timezone: 'UTC', wind_speed_unit: 'ms' };
    if (responseKind === 'current') {
      params.current = 'temperature_2m,relative_humidity_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m,wind_direction_10m,surface_pressure,cloud_cover';
      params.hourly = 'temperature_2m,precipitation,precipitation_probability,weather_code';
      params.forecast_hours = '25';
    }
    else {
      Object.assign(params, responseKind === 'storm' ? {
        daily: 'weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,wind_speed_10m_max,wind_gusts_10m_max',
        hourly: 'temperature_2m,precipitation,precipitation_probability,weather_code,wind_speed_10m,wind_gusts_10m,wind_direction_10m'
      } : {
        daily: 'weather_code,temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,wind_speed_10m_max',
        hourly: 'temperature_2m,dew_point_2m,precipitation,precipitation_probability,weather_code,wind_speed_10m'
      });
      if (startDate) {
        params.start_date = startDate;
        params.end_date = fetchEndDate ?? nextDate(startDate);
      } else {
        params.forecast_days = String(days);
        if (stormHours !== undefined) params.forecast_hours = String(Math.min(7 * 24, stormHours));
      }
    }
    url.search = new URLSearchParams(params);
    try {
      const body = await upstream(url, signal); const retrievedAt = now().toISOString();
      const variableText = [input.variable, input.variables].map((value) => String(scalar(value) ?? '')).join(' ');
      const options = {
        requestStart, requestEnd, requestedFields: input.fields, requestText: input.request_text,
        requestedHours: responseKind === 'storm' ? stormHours : requestedHours,
        // Telegraph's forecast requests may carry a multi-day window without
        // forwarding the original "hourly" field. Long windows are the
        // hourly forecast intent in practice and must use the same complete
        // answer shape as explicitly hourly requests.
        renderHourly: responseKind === 'forecast' && (
          requestedHours !== undefined ||
          days > 2 ||
          String(input.interval ?? '').toLowerCase() === 'hourly' ||
          /\bhourly\b/i.test(input.request_text ?? '') ||
          /\b(?:2t|temperature_2m)\b/i.test(variableText)
        )
      };
      const payload = responseKind === 'current'
        ? currentPayload(location, body, retrievedAt)
        : responseKind === 'storm'
          ? stormPayload(location, body, days, retrievedAt, false, options)
          : forecastPayload(location, body, days, retrievedAt, false, options);
      weather.set(key, { payload, cachedAt: now().getTime() }); stale.set(key, { body, location, retrievedAt, options }); return payload;
    } catch (error) {
      const old = stale.get(key); if (!old) throw error;
      return responseKind === 'current'
        ? currentPayload(old.location, old.body, old.retrievedAt, true)
        : responseKind === 'storm'
          ? stormPayload(old.location, old.body, days, old.retrievedAt, true, old.options)
          : forecastPayload(old.location, old.body, days, old.retrievedAt, true, old.options);
    }
  }
  return { query, probe: async (signal) => { const url = new URL(FORECAST_URL); url.search = new URLSearchParams({ latitude: '0', longitude: '0', current: 'temperature_2m', timezone: 'auto' }); await upstream(url, signal); return true; } };
}
