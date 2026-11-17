// RP3/T2 automatic paint-path comparison (GOAL 8, VERIFY 6), headless: opens the demo,
// taps the metrics page's "Karşılaştır" button and waits, with no other input, for the one
// comparative summary; asserts it finished in under 3 minutes and lists every path.
// Also checks the paint-path fallback (`?paint=` unknown / unsupported → `ref`, reported).
//
// The registry's second path here is `fixture` (shells/web/paint/fixture.js), a test
// fixture that exists only with `?fixture=1`.
//
// Usage: node tests/compare.mjs <qp> [--browsers chromium,webkit] [--phase <ms>]
//          [--dpr 1] [--out <dir>]
//   --phase  phase length passed as `?bench_phase=` (default: the core's, 8000 ms)
//   --out    writes compare-<browser>.json (the summary) there
import fs from 'node:fs';
import path from 'node:path';
import { chromium, webkit } from 'playwright';
import { startDemo, settle } from './lib.mjs';

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const browsers = opt('--browsers', 'chromium,webkit').split(',');
const phase = opt('--phase', '');
const dpr = Number(opt('--dpr', '1'));
const out = opt('--out', null);
const here = new URL('..', import.meta.url).pathname;
const ENGINES = { chromium, webkit };
const LIMIT_MS = 180000;

function check(cond, msg) {
  if (!cond) throw new Error(`compare: ${msg}`);
}

async function fallback(browser, base, q, want) {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
  const page = await ctx.newPage();
  await page.goto(`${base}?anim=0&${q}`);
  await settle(page);
  const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
  await ctx.close();
  check(m.paint && ['requested', 'used', 'fallback'].every((k) => m.paint[k] === want[k]), `${q}: paint ${JSON.stringify(m.paint)}, want ${JSON.stringify(want)}`);
}

let failed = 0;
for (const [k, name] of browsers.entries()) {
  const { child, base: b } = await startDemo(qp, here, 8420 + k);
  const base = b.replace('localhost', '127.0.0.1');
  const browser = await ENGINES[name].launch();
  try {
    await fallback(browser, base, 'paint=nope', { requested: 'nope', used: 'ref', fallback: 'unknown' });
    await fallback(browser, base, 'paint=fixture', { requested: 'fixture', used: 'ref', fallback: 'unsupported' });
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: dpr });
    const page = await ctx.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    await page.goto(`${base}?anim=0&fixture=1${phase ? `&bench_phase=${phase}` : ''}`);
    await settle(page);
    await page.click('#tabs button[data-tab="metrics"]');
    const t0 = Date.now();
    await page.click('#b_start');
    await page.waitForFunction(() => !window.qp.benching() && JSON.parse(window.qp.bench()) !== null,
      null, { timeout: LIMIT_MS + 20000, polling: 500 });
    const ms = Date.now() - t0;
    const s = JSON.parse(await page.evaluate(() => window.qp.bench()));
    const table = await page.locator('#brows tr').count();
    const font = await page.evaluate(() => [...document.fonts].some((f) => f.family.replace(/"/g, '') === 'QpMono' && f.status === 'loaded'));
    if (out) {
      fs.mkdirSync(out, { recursive: true });
      fs.writeFileSync(path.join(out, `compare-${name}.json`), JSON.stringify(s, null, 1));
    }
    check(ms < LIMIT_MS, `${name}: took ${ms} ms`);
    check(errors.length === 0, `${name}: page errors ${errors}`);
    check(JSON.stringify(s.paths.map((p) => p.name)) === '["ref","fixture"]', `${name}: paths ${s.paths.map((p) => p.name)}`);
    check(table > 10, `${name}: the table has ${table} rows`);
    check(font, `${name}: the fixture did not register the core's font through FontFace`);
    for (const p of s.paths) {
      check(p.errors.length === 0, `${name}/${p.name}: ${p.errors}`);
      check(p.paint.used === p.name && p.paint.fallback === null, `${name}/${p.name}: paint ${JSON.stringify(p.paint)}`);
      for (const ph of s.config.phases) check(p.phases[ph] && p.phases[ph].frame_ms.n > 0, `${name}/${p.name}: no frames in ${ph}`);
      check(p.span_ms.tick.n > 0, `${name}/${p.name}: no ticks`);
      check(p.span_ms.scene.n > 0, `${name}/${p.name}: no scene paints`);
      check(p.phases.idle.steps[0] === s.config.seek_step, `${name}/${p.name}: idle starts at step ${p.phases.idle.steps[0]}`);
    }
    const row = (p) => {
      const f = (ph) => p.phases[ph].frame_ms;
      return `${p.name}: tick p50 ${p.span_ms.tick.p50} p95 ${p.span_ms.tick.p95} · face p50 ${p.span_ms.face.p50 ?? '-'} · scene p50 ${p.span_ms.scene.p50}`
        + ` · frame p95 idle ${f('idle').p95} scroll ${f('scroll').p95} drag ${f('drag').p95} chart ${f('chart_ticks').p95}`;
    };
    console.log(`compare ${name}: ${(ms / 1000).toFixed(1)} s, refresh ${s.refresh.hz} Hz, ${s.paths.length} paths, total_ms ${s.total_ms}`);
    for (const p of s.paths) console.log(`  ${row(p)}`);
    await ctx.close();
  } catch (e) {
    console.error(String(e));
    failed++;
  } finally {
    await browser.close();
    child.kill();
  }
}
process.exit(failed ? 1 : 0);
