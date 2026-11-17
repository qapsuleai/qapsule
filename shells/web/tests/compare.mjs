// RP3/T2 automatic paint-path comparison (GOAL 8, VERIFY 6), headless: opens the demo,
// taps the metrics page's "Karşılaştır" button and waits, with no other input, for the one
// comparative summary; asserts it finished in under 3 minutes and lists every path.
// Also checks the paint-path fallback (`?paint=` unknown / unsupported → `ref`, reported).
//
// The test fixture path `fixture` (shells/web/paint/fixture.js) exists only with
// `?fixture=1` (put in the address when `--paths` names it).
//
// Usage: node tests/compare.mjs <qp> [--browsers chromium,webkit] [--phase <ms>]
//          [--dpr 1] [--out <dir>] [--paths ref,canvas] [--order reverse] [--throttle <rate>]
//          [--viewport 390x844] [--port 8420] [--q <extra query>] [--jsc useJIT=false]
//   --phase     phase length passed as `?bench_phase=` (default: the core's, 8000 ms)
//   --out       writes compare-<browser>.json (the summary) there
//   --paths     the paths to run (`?bench_paths=`; the core keeps those the browser supports,
//               in registry order). Default: every registered path the browser supports.
//   --order     (RP3/S) `reverse` runs the paths in reversed order (`?bench_order=reverse`)
//   --throttle  (RP3/M12b) Chromium CPU throttling rate (CDP Emulation.setCPUThrottlingRate)
//   --viewport  (RP3/M12b) the page size in CSS px
//   --q         (RP3/M12b) extra query parameters for the page (e.g. `canvas=full`)
//   --jsc       (RP3/M12b) WebKit JavaScriptCore options, e.g. `useJIT=false` (T1's PHONE)
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
const pathsArg = opt('--paths', '');
const paths = pathsArg ? pathsArg.split(',') : [];
const order = opt('--order', '');
const throttle = Number(opt('--throttle', '1'));
const [vw, vh] = opt('--viewport', '390x844').split('x').map(Number);
const port0 = Number(opt('--port', '8420'));
const extraQ = opt('--q', '');
const jsc = opt('--jsc', '');
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
  const { child, base: b } = await startDemo(qp, here, port0 + k);
  const base = b.replace('localhost', '127.0.0.1');
  // (RP3/M12b) WebKit JavaScriptCore options reach the WebContent process only as
  // __XPC_JSC_* variables (RP3/T1, calibrate.mjs): `--jsc useJIT=false` = T1's PHONE setting.
  const env = { ...process.env };
  for (const [k, v] of new URLSearchParams(jsc.split(',').join('&'))) if (k) env[`__XPC_JSC_${k}`] = v;
  const browser = await ENGINES[name].launch(name === 'webkit' ? { env } : {});
  try {
    await fallback(browser, base, 'paint=nope', { requested: 'nope', used: 'ref', fallback: 'unknown' });
    await fallback(browser, base, 'paint=fixture', { requested: 'fixture', used: 'ref', fallback: 'unsupported' });
    const ctx = await browser.newContext({ viewport: { width: vw, height: vh }, deviceScaleFactor: dpr });
    const page = await ctx.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    if (throttle !== 1 && name === 'chromium') {
      const cdp = await ctx.newCDPSession(page);
      await cdp.send('Emulation.setCPUThrottlingRate', { rate: throttle });
    }
    const fx = paths.includes('fixture') ? '&fixture=1' : '';
    const sel = `${pathsArg ? `&bench_paths=${pathsArg}` : ''}${order ? `&bench_order=${order}` : ''}`;
    await page.goto(`${base}?anim=0${fx}${sel}${phase ? `&bench_phase=${phase}` : ''}${extraQ ? `&${extraQ}` : ''}`);
    await settle(page);
    // The expected list: the registry's supported paths (the page's own registry), those of
    // `--paths` among them, reversed with `--order reverse`.
    let want = await page.evaluate(async () => (await import('/paint/index.js')).supportedNames());
    if (paths.length) want = want.filter((n) => paths.includes(n));
    if (order === 'reverse') want.reverse();
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
    // (RP3/S) The paths run are the registry's supported ones (or `--paths` among them),
    // in registry order or reversed (`--order reverse`).
    const names = s.paths.map((p) => p.name);
    check(JSON.stringify(names) === JSON.stringify(want), `${name}: paths ${names}, want ${want}`);
    check(table > 10, `${name}: the table has ${table} rows`);
    check(font || !want.some((p) => p === 'fixture' || p === 'canvas'), `${name}: the core's font was not registered through FontFace`);
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
