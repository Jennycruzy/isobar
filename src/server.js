import Fastify from 'fastify';
import { pathToFileURL } from 'node:url';
import { createWeatherService, normalizeInput } from './weather.js';

export function buildApp(options = {}) {
  const app = Fastify({ logger: options.logger ?? true, requestTimeout: 5000 });
  const started = Date.now();
  const service = options.service ?? createWeatherService({ logger: app.log });
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
    try { return await service.query(kind, query, AbortSignal.timeout(4950)); }
    catch (error) { request.log.error({ err: error }, 'weather request failed'); return reply.code(200).send({ answer: 'Current weather data is temporarily unavailable because the upstream service did not respond. Please try again shortly.', location: null, source: 'open-meteo', retrieved_at: new Date().toISOString() }); }
  };
  app.get('/weather', handler('current')); app.get('/forecast', handler('forecast'));
  return app;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const app = buildApp();
  await app.listen({ host: process.env.HOST ?? '127.0.0.1', port: Number(process.env.PORT ?? 8080) });
}
