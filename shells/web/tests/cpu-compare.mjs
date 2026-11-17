// RP3/M12a measurement driver: the metrics page's automatic comparison (tests/compare.mjs
// flow: tap "Karşılaştır", wait for the one summary) at a phone-like setting — viewport,
// DPR, optional Chromium CPU throttling (CDP `Emulation.setCPUThrottlingRate`) — with any
// extra page query (e.g. `cpu_variants=1`: every `cpu` measurement variant is a path).
// Writes each run's summary and prints one line per path.
//
// Usage: node tests/cpu-compare.mjs <qp> --browser chromium|webkit [--throttle 4]
//          [--dpr 3] [--viewport 402x874] [--phase 8000] [--query 'cpu_variants=1']
//          [--runs 3] [--port 8440] [--web <dir>] [--out <dir>] [--tag <name>]
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { chromium, webkit } from 'playwright';
import { startDemo, settle } from './lib.mjs';

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const name = opt('--browser', 'chromium');
const throttle = Number(opt('--throttle', '1'));
const dpr = Number(opt('--dpr', '3'));
const [vw, vh] = opt('--viewport', '402x874').split('x').map(Number);
const phase = opt('--phase', '');
const query = opt('--query', '');
const runs = Number(opt('--runs', '1'));
const port = Number(opt('--port', '8440'));
const web = opt('--web', new URL('..', import.meta.url).pathname);
const out = opt('--out', null);
const tag = opt('--tag', `${name}-t${throttle}`);
const ENGINES = { chromium, webkit };

const { child, base: b } = await startDemo(qp, web, port);
const base = b.replace('localhost', '127.0.0.1');
const browser = await ENGINES[name].launch();
let failed = 0;
try {
  for (let r = 0; r < runs; r++) {
    const load = execSync('uptime').toString().trim();
    const ctx = await browser.newContext({ viewport: { width: vw, height: vh }, deviceScaleFactor: dpr });
    const page = await ctx.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    const q = new URLSearchParams({ anim: '0' });
    if (phase) q.set('bench_phase', phase);
    for (const [k, v] of new URLSearchParams(query)) q.set(k, v);
    await page.goto(`${base}?${q}`);
    if (throttle > 1) {
      const cdp = await ctx.newCDPSession(page);
      await cdp.send('Emulation.setCPUThrottlingRate', { rate: throttle });
    }
    await settle(page, 120000);
    await page.click('#tabs button[data-tab="metrics"]');
    const t0 = Date.now();
    await page.click('#b_start');
    await page.waitForFunction(() => !window.qp.benching() && JSON.parse(window.qp.bench()) !== null,
      null, { timeout: 400000, polling: 1000 });
    const s = JSON.parse(await page.evaluate(() => window.qp.bench()));
    s.m12a = { load, load_after: execSync('uptime').toString().trim(), throttle, dpr, viewport: [vw, vh], query, browser: name, wall_ms: Date.now() - t0, errors };
    if (out) {
      fs.mkdirSync(out, { recursive: true });
      fs.writeFileSync(path.join(out, `${tag}-r${r}.json`), JSON.stringify(s, null, 1));
    }
    console.log(`${tag} run ${r}: ${((Date.now() - t0) / 1000).toFixed(0)} s, ${load.replace(/.*load averages?: /, 'load ')}, errors ${errors.length}`);
    for (const p of s.paths) {
      const sp = p.span_ms;
      const f = (ph) => p.phases[ph] ? `${p.phases[ph].frame_ms.p50}/${p.phases[ph].frame_ms.p95}` : '-';
      console.log(`  ${p.name.padEnd(13)} tick ${sp.tick?.p50}/${sp.tick?.p95} face ${sp.face?.p50 ?? '-'}/${sp.face?.p95 ?? '-'} scene ${sp.scene?.p50 ?? '-'}/${sp.scene?.p95 ?? '-'} · frame idle ${f('idle')} scroll ${f('scroll')} chart ${f('chart_ticks')} drag ${f('drag')} · err ${p.errors.length}`);
    }
    await ctx.close();
  }
} catch (e) {
  console.error(String(e));
  failed++;
} finally {
  await browser.close();
  child.kill();
}
process.exit(failed ? 1 : 0);
