// R4/T6 startup run (GOAL 10), extended in RP3/M13: opens the demo page `runs` times, each
// in a fresh browser context (empty cache, like a first visit = "cold"), then reloads it once
// in the same context ("warm", with whatever the host lets the browser cache), and prints the
// opening timeline's medians: page open → WASM ready → connected (`welcome`) → first capsule
// painted, plus the bytes transferred and the WASM response's encoding.
// Usage: node tests/startup.mjs <qp> [cpu throttling rate] [runs] [extra qp demo args…]
// Environment (all optional):
//   QP_BROWSER=chromium|webkit   (default chromium; CPU and network throttling are Chromium
//                                 only: WebKit has no CDP, it runs on plain loopback)
//   QP_DPR=3                     device pixel ratio (default 1)
//   QP_NET=40,20                 Chromium network emulation: Mbit/s down, RTT ms (default off)
//   QP_PORT=8397                 demo port
//   QP_WEB=DIR                   web directory to serve (default: this shell)
//   QP_JSC=useJIT=false          (RP3/S) WebKit JavaScriptCore options (`__XPC_JSC_*`; T1's PHONE)
//   QP_VIEWPORT=402x874          (RP3/S) page size in CSS px (default 390x844)
//   QP_QUERY=paint=ref           (RP3/S) extra page query (default: none, the page's own path)
import { chromium, webkit } from 'playwright';
import { startDemo } from './lib.mjs';

const [qp, rate = '1', runs = '7', ...extra] = process.argv.slice(2);
const env = process.env;
const kind = env.QP_BROWSER === 'webkit' ? 'webkit' : 'chromium';
const dpr = Number(env.QP_DPR || 1);
const net = env.QP_NET ? env.QP_NET.split(',').map(Number) : null;
const here = env.QP_WEB || new URL('..', import.meta.url).pathname;
const { child, base } = await startDemo(qp, here, Number(env.QP_PORT || 8397), extra);
const jscEnv = { ...env };
for (const [k, v] of new URLSearchParams((env.QP_JSC || '').split(',').join('&'))) if (k) jscEnv[`__XPC_JSC_${k}`] = v;
const browser = await (kind === 'webkit' ? webkit : chromium).launch(
  kind === 'chromium' ? { args: ['--force-color-profile=srgb', '--font-render-hinting=none'] } : { env: jscEnv },
);
const [vw, vh] = (env.QP_VIEWPORT || '390x844').split('x').map(Number);
const url = `${base.replace('localhost', '127.0.0.1')}?anim=0${env.QP_QUERY ? `&${env.QP_QUERY}` : ''}`;

async function throttle(ctx, page) {
  if (kind !== 'chromium') return;
  const cdp = await ctx.newCDPSession(page);
  if (rate !== '1') await cdp.send('Emulation.setCPUThrottlingRate', { rate: Number(rate) });
  if (net) {
    await cdp.send('Network.enable');
    await cdp.send('Network.emulateNetworkConditions', {
      offline: false,
      latency: net[1],
      downloadThroughput: (net[0] * 1e6) / 8,
      uploadThroughput: (net[0] * 1e6) / 16,
    });
  }
}

/** One opening of the page (goto or reload); resolves with its timeline and bytes. */
async function open(page, how) {
  const got = [];
  const onDone = (req) => got.push((async () => {
    const res = await req.response();
    const s = await req.sizes().catch(() => null);
    const h = res ? await res.allHeaders() : {};
    return {
      url: req.url(),
      status: res ? res.status() : 0,
      encoding: h['content-encoding'] || 'identity',
      bytes: s ? s.responseBodySize + s.responseHeadersSize : 0,
    };
  })());
  page.on('requestfinished', onDone);
  if (how === 'goto') await page.goto(url);
  else await page.reload();
  await page.waitForFunction(() => {
    if (!window.qp || !window.qp.metrics) return false;
    try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; }
  }, null, { timeout: 180000, polling: 20 });
  const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
  const res = await page.evaluate(() => performance.getEntriesByType('resource')
    .map((e) => ({ name: e.name, end: e.responseEnd, transfer: e.transferSize })));
  page.off('requestfinished', onDone);
  const reqs = await Promise.all(got);
  const wasm = reqs.find((r) => r.url.endsWith('.wasm'));
  const wasmRt = res.find((e) => e.name.endsWith('.wasm'));
  return {
    ...m.open_ms,
    wasm_fetch_end: wasmRt ? wasmRt.end : null,
    // Bytes on the wire as the browser reports them (Playwright sizes: body + headers; a
    // response served from the cache counts what the browser says it received).
    bytes_total: reqs.reduce((a, r) => a + r.bytes, 0),
    bytes_wasm: wasm ? wasm.bytes : null,
    // Resource Timing transferSize (0 = from cache; WebKit may report 0 always).
    transfer_rt: res.reduce((a, e) => a + (e.transfer || 0), 0),
    requests: reqs.length,
    wasm_encoding: wasm ? wasm.encoding : null,
    wasm_status: wasm ? wasm.status : null,
  };
}

const cold = [];
const warm = [];
try {
  for (let k = 0; k < Number(runs); k++) {
    const opts = { viewport: { width: vw, height: vh }, deviceScaleFactor: dpr };
    const ctx = await browser.newContext(opts);
    const page = await ctx.newPage();
    await throttle(ctx, page);
    cold.push(await open(page, 'goto'));
    warm.push(await open(page, 'reload'));
    await ctx.close();
  }
} finally {
  await browser.close();
  child.kill();
}

function medians(rows) {
  const keys = [...new Set(rows.flatMap((r) => Object.keys(r)))];
  const med = {};
  for (const key of keys) {
    const v = rows.map((r) => r[key]).filter((x) => typeof x === 'number').sort((a, b) => a - b);
    if (v.length) med[key] = { median: v[Math.floor(v.length / 2)], min: v[0], max: v[v.length - 1] };
    else med[key] = [...new Set(rows.map((r) => r[key]))].join('|');
  }
  return med;
}
console.log(JSON.stringify({
  browser: kind, dpr, cpu_throttling: kind === 'chromium' ? Number(rate) : 1, jsc: env.QP_JSC || '',
  viewport: [vw, vh], query: env.QP_QUERY || '',
  network: kind === 'chromium' && net ? { down_mbit: net[0], rtt_ms: net[1] } : 'loopback',
  runs: cold.length, extra, cold: medians(cold), warm: medians(warm),
}, null, 1));
