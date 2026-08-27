import Fastify from 'fastify';
import { pathToFileURL } from 'node:url';
import { createWeatherService, isForecastRequest, normalizeInput } from './weather.js';

const LANDING_PAGE = `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Isobar Weather — Telegraph Miner</title>
  <style>
    :root { color-scheme: dark; }
    body { margin: 0; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; background: #07131f; color: #e8f5f7; display: flex; min-height: 100vh; align-items: center; justify-content: center; }
    main { max-width: 680px; padding: 2.5rem 1.5rem; }
    h1 { font-size: 1.5rem; margin: 0 0 .25rem; letter-spacing: .02em; }
    .badge { display: inline-block; border: 1px solid #53e6d0; color: #53e6d0; padding: .15rem .5rem; border-radius: 4px; font-size: .75rem; margin: .5rem 0 1.25rem; }
    p { color: #a6c8d1; line-height: 1.6; font-size: .9rem; }
    code { background: #102638; border: 1px solid #1b485d; border-radius: 4px; padding: .15rem .4rem; font-size: .85rem; }
    ul { list-style: none; padding: 0; }
    li { margin: .7rem 0; color: #d9eef0; }
    a { color: #66d9ef; text-decoration: none; }
    a:hover { text-decoration: underline; }
    .muted { color: #7fa8b4; }
  </style>
</head>
<body>
  <main>
    <h1>🌦️ Isobar Weather</h1>
    <div class="badge">ACTIVE TELEGRAPH MINER · REGISTRATION #224</div>
    <p>Deterministic current conditions and forecasts for autonomous agents, backed by Open-Meteo. Isobar serves <code>WEATHER_CHECK</code> and <code>WEATHER_FORECAST</code> through Telegraph.</p>
    <ul>
      <li><code>GET /weather?q=Tokyo</code> — current conditions and the next-24-hour weather horizon</li>
      <li><code>GET /forecast?q=Tokyo&amp;days=2</code> — compact daily summaries plus hourly forecast data</li>
      <li><code>GET /health</code> — service liveness and upstream status</li>
    </ul>
    <p class="muted"><a href="https://github.com/Jennycruzy/isobar">source &amp; methodology</a> · <a href="https://explorer.telegraphprotocol.com/miners/224">Telegraph Explorer</a> · <a href="https://weather.isobars.xyz/health">live health</a></p>
  </main>
</body>
</html>`;

export function buildApp(options = {}) {
  const app = Fastify({ logger: options.logger ?? true, requestTimeout: 5000 });
  const started = Date.now();
  const service = options.service ?? createWeatherService({ logger: app.log });
  app.get('/', async (_request, reply) => reply.type('text/html; charset=utf-8').send(LANDING_PAGE));
  app.get('/health', async (_request, reply) => {
    let upstream_ok = false;
    try { upstream_ok = await service.probe(AbortSignal.timeout(4500)); } catch {}
    return reply.code(upstream_ok ? 200 : 503).send({ status: upstream_ok ? 'ok' : 'degraded', version: '0.1.0', uptime_s: Math.floor((Date.now() - started) / 1000), upstream_ok });
  });
  const handler = (kind) => async (request, reply) => {
    const query = normalizeInput(request.query);
    const { q, lat, lon } = query;
    const coordinates = lat !== undefined || lon !== undefined;
    if ((!q && !coordinates) || (coordinates && (lat === undefined || lon === undefined || !Number.isFinite(Number(lat)) || !Number.isFinite(Number(lon)) || Number(lat) < -90 || Number(lat) > 90 || Number(lon) < -180 || Number(lon) > 180))) {
      return reply.code(200).send({ answer: 'A valid location was not provided. Supply a place name with q, or both latitude and longitude with lat and lon.', location: null, source: 'open-meteo', retrieved_at: new Date().toISOString() });
    }
    const responseKind = kind === 'current' && isForecastRequest(query.request_text) ? 'forecast' : kind;
    try { return await service.query(responseKind, query, AbortSignal.timeout(4950)); }
    catch (error) { request.log.error({ err: error }, 'weather request failed'); return reply.code(200).send({ answer: 'Current weather data is temporarily unavailable because the upstream service did not respond. Please try again shortly.', location: null, source: 'open-meteo', retrieved_at: new Date().toISOString() }); }
  };
  app.get('/weather', handler('current')); app.get('/forecast', handler('forecast'));
  return app;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const app = buildApp();
  await app.listen({ host: process.env.HOST ?? '127.0.0.1', port: Number(process.env.PORT ?? 8080) });
}
