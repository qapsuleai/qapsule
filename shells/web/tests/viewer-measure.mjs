// RP3/M15 (K48, VERIFY 15–17): the device's work, memory and traffic in viewer mode (the
// default) and in the full replica (`?mode=replica`, baseline a) for one universe size.
// Headless Chromium, phone viewport, DPR 3 (the core paints at scale 3), synthetic market.
//
//   node tests/viewer-measure.mjs <qp> <instruments> <out.json> [--reps 3] [--steps 9]
//                                 [--modes viewer,replica] [--port 8431]
//
// Scenario per repetition and mode (a fresh page each time; the replay continues where
// the previous one left it, paused): open the page; wait until the feed is painted; take
// `steps` replay steps on the feed screen (each waits until the page is idle); open the
// first capsule (time to its first scene paint); take `steps` steps with it open; run
// the open capsule's check five times. Per-step device work = sum of the page's `tick`
// (data frame) and `msg` (protocol frame) spans per step, both including the paints they
// cause. Memory: JS heap (precise) and WASM linear memory at the end. Traffic: bytes the
// core counted per frame kind. Subscriptions: what the device subscribes to on the feed
// screen and with a capsule open (`window.qp.viewer()`).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { startDemo, settle } from './lib.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const web = path.resolve(here, '..');
const argv = process.argv.slice(2);
const [qp, nArg, outArg] = argv;
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const reps = Number(opt('--reps', '3'));
const steps = Number(opt('--steps', '9'));
const modes = opt('--modes', 'viewer,replica').split(',');
const port = Number(opt('--port', '8431'));
const timeout = Number(opt('--timeout', '600000'));

const { child, base } = await startDemo(qp, web, port, ['--instruments', nArg]);
const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
const browser = await chromium.launch({
  args: ['--force-color-profile=srgb', '--font-render-hinting=none', '--enable-precise-memory-info', '--js-flags=--expose-gc'],
});

async function spans(page) {
  return page.evaluate(() => JSON.parse(window.qp.metrics()).span_ms);
}
const total = (s, k) => (s[k] && s[k].n ? s[k].n * s[k].mean : 0);
const work = (a, b) => (total(b, 'tick') - total(a, 'tick')) + (total(b, 'msg') - total(a, 'msg'));
const traffic = (page) => page.evaluate(() => JSON.parse(window.qp.viewer()).traffic);
const bytes = (t) => Object.values(t).reduce((x, v) => x + v.bytes, 0);

async function step(page) {
  await page.click('#c_step');
  await settle(page, timeout);
}

async function one(mode) {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 3 });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  const t0 = Date.now();
  await page.goto(`${origin}/?anim=0${mode === 'replica' ? '&mode=replica' : ''}`, { timeout });
  await settle(page, timeout);
  await page.waitForFunction(() => document.querySelectorAll('#list .face').length > 0, null, { timeout });
  await settle(page, timeout);
  const ready_ms = Date.now() - t0;
  const r = { mode, ready_ms };
  const tr0 = await traffic(page);
  r.start_bytes = tr0.start ? tr0.start.bytes : 0;
  r.subs_feed = JSON.parse(await page.evaluate(() => window.qp.viewer()));
  delete r.subs_feed.traffic;
  // Feed screen.
  let s0 = await spans(page);
  let b0 = bytes(await traffic(page));
  for (let i = 0; i < steps; i++) await step(page);
  let s1 = await spans(page);
  let b1 = bytes(await traffic(page));
  r.feed_work_ms_per_step = work(s0, s1) / steps;
  r.feed_bytes_per_step = (b1 - b0) / steps;
  // Open the first capsule: time to its first scene paint.
  const first = await page.evaluate(() => JSON.parse(window.qp.feed()).filter((x) => x.id)[0].id);
  r.open_ms = await page.evaluate(async (id) => {
    const n0 = (JSON.parse(window.qp.metrics()).span_ms.scene || { n: 0 }).n;
    const t = performance.now();
    window.qp.openCapsule(id);
    for (;;) {
      await new Promise((res) => requestAnimationFrame(res));
      const n = (JSON.parse(window.qp.metrics()).span_ms.scene || { n: 0 }).n;
      if (n > n0) return performance.now() - t;
      if (performance.now() - t > 120000) return -1;
    }
  }, first);
  await settle(page, timeout);
  r.subs_open = JSON.parse(await page.evaluate(() => window.qp.viewer()));
  delete r.subs_open.traffic;
  r.trust_open = JSON.parse(await page.evaluate(() => window.qp.open())).trust || null;
  s0 = await spans(page);
  b0 = bytes(await traffic(page));
  for (let i = 0; i < steps; i++) await step(page);
  s1 = await spans(page);
  b1 = bytes(await traffic(page));
  r.open_work_ms_per_step = work(s0, s1) / steps;
  r.open_bytes_per_step = (b1 - b0) / steps;
  // The open capsule's check on its own data (viewer only), five times.
  r.check_ms = await page.evaluate(() => {
    const out = [];
    let last = null;
    for (let i = 0; i < 5; i++) {
      const t = performance.now();
      last = window.qp.check();
      out.push(performance.now() - t);
    }
    return { runs: out, result: JSON.parse(last) };
  });
  // Memory at the end: JS heap after a full GC (CDP), WASM linear memory (the core's).
  const cdp = await ctx.newCDPSession(page);
  await cdp.send('HeapProfiler.collectGarbage');
  await cdp.send('HeapProfiler.collectGarbage');
  const hu = await cdp.send('Runtime.getHeapUsage');
  const m = await page.evaluate(() => JSON.parse(window.qp.metrics()).memory_bytes);
  r.js_heap = hu.usedSize;
  r.js_heap_sampled = m.js_heap ? m.js_heap.last : null;
  r.wasm = m.wasm ? m.wasm.last : null;
  r.traffic = await traffic(page);
  r.errors = errors;
  await ctx.close();
  return r;
}

const out = { instruments: Number(nArg), steps, reps, runs: [] };
try {
  for (let k = 0; k < reps; k++) {
    for (const mode of modes) {
      try {
        const r = await one(mode);
        console.log(JSON.stringify({ n: nArg, rep: k, mode, feed_ms: r.feed_work_ms_per_step, open_ms: r.open_ms, heap: r.js_heap, wasm: r.wasm }));
        out.runs.push(r);
      } catch (e) {
        console.log(JSON.stringify({ n: nArg, rep: k, mode, error: String(e).slice(0, 300) }));
        out.runs.push({ mode, error: String(e).slice(0, 2000) });
      }
      fs.writeFileSync(outArg, JSON.stringify(out, null, 2));
    }
  }
} finally {
  await browser.close();
  child.kill();
}
