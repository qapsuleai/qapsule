// RP3/M14 measurement (rounds/RP3/experiments/M14.md): the live update frequency before
// and after the coalesced paint pass. One run = a fresh `qp demo` (synthetic, 30
// instruments), a fresh browser context, the page's own automatic comparison
// (`Karşılaştır`, the four phases of tests/compare.mjs: idle, scroll, chart_ticks, drag)
// with the paint path `ref` only; the comparison summary (`window.qp.bench()`) and the
// page's metrics (`window.qp.metrics()`: the JIT probe, secure context, WebGPU) are kept.
//
// Usage: node tests/freq-measure.mjs <qp> [--web DIR] [--config phone|chromium4]
//          [--mode viewer|replica] [--reps 3] [--phase 8000] [--port 8490] [--out FILE]
//   --web    the shell directory `qp demo` serves (another build: the "before" page)
//   --config phone = the calibrated phone setting (tests/calibrate.mjs PHONE: WebKit, JIT
//            off, DPR 3, 402×874); chromium4 = Chromium, CDP CPU throttling 4×, DPR 3
// Prints one JSON object: the runs' per-phase numbers and their medians.
import fs from 'node:fs';
import { execFileSync } from 'node:child_process';
import { PHONE, launchFor } from './calibrate.mjs';
import { startDemo, settle } from './lib.mjs';

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const here = new URL('..', import.meta.url).pathname;
const web = opt('--web', here);
const CONFIGS = {
  phone: PHONE,
  chromium4: { name: 'chromium4', engine: 'chromium', dpr: 3, rate: 4, viewport: { width: 402, height: 874 } },
};
const cfg = CONFIGS[opt('--config', 'phone')];
const mode = opt('--mode', 'viewer');
const reps = Number(opt('--reps', '3'));
const phase = opt('--phase', '8000');
const port = Number(opt('--port', '8490'));
const out = opt('--out', null);

const load = () => { try { return execFileSync('uptime', { encoding: 'utf8' }).trim().replace(/.*load averages?: /, ''); } catch { return ''; } };
const median = (xs) => {
  const v = xs.filter((x) => typeof x === 'number' && Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const m = v.length >> 1;
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
};

async function runOnce(browser) {
  const { child, base } = await startDemo(qp, web, port);
  const ctx = await browser.newContext({ viewport: cfg.viewport, deviceScaleFactor: cfg.dpr });
  try {
    const page = await ctx.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    if (cfg.engine === 'chromium' && cfg.rate > 1) {
      const cdp = await ctx.newCDPSession(page);
      await cdp.send('Emulation.setCPUThrottlingRate', { rate: cfg.rate });
    }
    const q = `?anim=0&bench_phase=${phase}${mode === 'replica' ? '&mode=replica' : ''}`;
    await page.goto(`${base.replace('localhost', '127.0.0.1')}${q}`);
    await settle(page, 120000);
    // The JIT probe runs 4 s after the start; the comparison starts after it.
    await page.waitForTimeout(5000);
    await page.click('#tabs button[data-tab="metrics"]');
    await page.click('#b_start');
    await page.waitForFunction(() => !window.qp.benching() && JSON.parse(window.qp.bench()) !== null,
      null, { timeout: 240000, polling: 500 });
    const bench = JSON.parse(await page.evaluate(() => window.qp.bench()));
    const metrics = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    return { bench, mode: metrics.mode || null, errors };
  } finally {
    await ctx.close();
    child.kill();
    await new Promise((r) => { child.once('exit', r); setTimeout(r, 2000); });
  }
}

const PHASES = ['idle', 'scroll', 'chart_ticks', 'drag'];
const pick = (s, k) => (s && s[k] !== undefined ? s[k] : null);

function phaseRow(p) {
  const sp = (n) => p.span_ms[n] || {};
  const c = p.counts || {};
  return {
    frame_p50: pick(p.frame_ms, 'p50'), frame_p95: pick(p.frame_ms, 'p95'), frame_p99: pick(p.frame_ms, 'p99'),
    frame_long: pick(p.frame_ms, 'long'),
    tick_n: pick(sp('tick'), 'n'), tick_p50: pick(sp('tick'), 'p50'), tick_p95: pick(sp('tick'), 'p95'),
    msg_n: pick(sp('msg'), 'n'), msg_p50: pick(sp('msg'), 'p50'),
    // All the page's frame work of the phase per data frame (before: tick + msg, each with
    // its repaint; after: tick + msg + paint passes); R4's `tick` was the first term.
    work_per_data_frame: sp('tick').n
      ? ['tick', 'msg', 'paint_pass'].map((k) => (sp(k).n || 0) * (sp(k).mean || 0)).reduce((a, b) => a + b, 0) / sp('tick').n
      : null,
    pass_n: pick(sp('paint_pass'), 'n'), pass_p50: pick(sp('paint_pass'), 'p50'), pass_p95: pick(sp('paint_pass'), 'p95'),
    wait_p50: pick(sp('pass_wait'), 'p50'), wait_p95: pick(sp('pass_wait'), 'p95'), wait_max: pick(sp('pass_wait'), 'max'),
    lat_p50: pick(sp('frame_to_paint'), 'p50'), lat_p95: pick(sp('frame_to_paint'), 'p95'), lat_max: pick(sp('frame_to_paint'), 'max'),
    face_n: pick(sp('face'), 'n'), face_p50: pick(sp('face'), 'p50'),
    scene_n: pick(sp('scene'), 'n'), scene_p50: pick(sp('scene'), 'p50'),
    data_frames: c.data_frames ?? null, msg_frames: c.msg_frames ?? null, paint_passes: c.paint_passes ?? null,
    passes_per_data_frame: c.passes_per_data_frame ?? null, faces_skipped_hidden: c.faces_skipped_hidden ?? null,
    scene_builds: c.scene_builds ?? null, tick_plus_pass_ms: c.tick_plus_pass_ms ?? null,
  };
}

const browser = await launchFor(cfg);
const load0 = load();
const runs = [];
try {
  for (let i = 0; i < reps; i++) {
    const r = await runOnce(browser);
    const p = r.bench.paths[0];
    const row = { mode: r.mode, errors: [...r.errors, ...p.errors], phases: {} };
    for (const ph of PHASES) if (p.phases[ph]) row.phases[ph] = phaseRow(p.phases[ph]);
    runs.push(row);
    console.error(`freq-measure: ${cfg.name} ${mode} run ${i + 1}/${reps} done (load ${load()})`);
  }
} finally {
  await browser.close();
}
const med = {};
for (const ph of PHASES) {
  med[ph] = {};
  for (const k of Object.keys(runs[0].phases[ph] || {})) med[ph][k] = median(runs.map((r) => r.phases[ph] && r.phases[ph][k]));
}
const result = { config: cfg.name, mode, web, reps, phase_ms: Number(phase), browser: browser.version(), load_before: load0, load_after: load(), median: med, runs };
if (out) fs.writeFileSync(out, JSON.stringify(result, null, 1));
console.log(JSON.stringify(result, null, 1));
