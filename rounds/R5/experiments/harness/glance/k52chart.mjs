// R5/M18: the K52 budget of one story-chart design at 5 / 50 / 200 stories in the visible
// range (GOAL M18, SPEC §G7.6, §G9 "200 kapsülde K52"), on the calibrated phone simulation
// of RP3 (shells/web/tests/calibrate.mjs: `PHONE_JIT_EST` = HTTPS/JIT on, the binding
// budget; `PHONE` = JIT off, "stays usable").
//
//   node k52chart.mjs <qp> <web> <out.json> --plan <plan.json>
//
// plan = {story, look, market: [qp demo args], open: [capsule ids], depths: [5, 50, 200],
//         rows: [canonical story rows], configs: [...], reps, play_ms, drag_ms, back_steps}
//
// Per config: one `qp demo` (the market), the replay sought once to `steps - back_steps`
// (so that it can still play); per (rep, depth) a fresh browser context: the page with
// `?story=` / `?look=` and `?watch=<instrument>`, a capsule of the instrument opened to its
// chart (the first of `open` that reaches chart depth: stories born in the history exist at
// every replay step), zoomed to the depth's visible story count (same rule as shoot.mjs),
// then (1) play `play_ms` (tick, scene paint with data) and (2) paused, drag the chart
// ±40 px every 150 ms for `drag_ms` (frame intervals in mode `drag`, scene paint). The
// core's metrics summary (`window.qp.metrics()`) is read at the end of each phase.
//
// Verdicts (K52, DECISIONS.md): JIT on — tick p95 <= 8 ms, scene paint p95 <= 8 ms, long
// frames while dragging <= 1 % (the core's own `long` share of `frame_ms.drag`); JIT off —
// long frames while dragging <= 10 % ("kullanılabilir kalır"); the JIT-off values are also
// given with `PHONE_BIAS` (calibration), as k52.mjs does.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../../../..');
const cal = await import(path.join(root, 'shells/web/tests/calibrate.mjs'));
const { startDemo, settle } = await import(path.join(root, 'shells/web/tests/lib.mjs'));

const argv = process.argv.slice(2);
const [qp, web, outFile] = argv;
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const plan = JSON.parse(fs.readFileSync(opt('--plan'), 'utf8'));
const CFG = { 'phone-jit-est': cal.PHONE_JIT_EST, phone: cal.PHONE };
const HOOK = `
;(() => { const f = Shell.prototype.on_frame;
  Shell.prototype.on_frame = function (...a) { globalThis.__qpcore = this; return f.apply(this, a); }; })();
`;

const median = (xs) => {
  const v = xs.filter((x) => typeof x === 'number' && Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const m = v.length >> 1;
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
};

async function visible(page, rows) {
  return page.evaluate((all) => {
    let look = null;
    try { look = JSON.parse(window.qp.look()); } catch (e) { /* none */ }
    if (!look || !look.range) return null;
    const { from_bar: a, to_bar: b } = look.range;
    let last = 0;
    for (const r of all) last = Math.max(last, r.last_bar);
    const n = all.filter((r) => r.born_bar <= b && (r.closed_bar === undefined || r.closed_bar === null ? last : r.closed_bar) >= a).length;
    return { n, range: [a, b] };
  }, rows);
}

async function zoomTo(page, want, rows) {
  const b = await page.locator('#scene').boundingBox();
  await page.mouse.move(b.x + b.width * 0.78, b.y + b.height * 0.3);
  let v = await visible(page, rows);
  if (!v) return null;
  for (let n = 0; n < 80; n++) {
    const dir = v.n < want ? 1 : -1;
    await page.mouse.wheel(0, 120 * dir);
    await settle(page, 120000);
    const w = await visible(page, rows);
    if (!w || (w.range[0] === v.range[0] && w.range[1] === v.range[1])) break;
    if (dir === -1 && w.n < want) {
      await page.mouse.wheel(0, 120);
      await settle(page, 120000);
      v = await visible(page, rows);
      break;
    }
    v = w;
    if (dir === 1 && v.n >= want) break;
  }
  return v;
}

const play = (page) => page.evaluate(() => document.getElementById('c_play').click());
async function readReset(page) {
  const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
  await page.evaluate(() => window.qp.resetMetrics());
  return m;
}

async function oneRun(browser, cfg, base, depth, first) {
  const ctx = await browser.newContext({ viewport: cfg.viewport, deviceScaleFactor: cfg.dpr });
  await ctx.route('**/pkg/qp_wasm.js', async (route) => {
    const r = await route.fetch();
    route.fulfill({ response: r, body: (await r.text()) + HOOK, headers: { ...r.headers(), 'content-type': 'text/javascript' } });
  });
  try {
    const page = await ctx.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    const q = new URLSearchParams({ x: '1', watch: plan.instrument });
    if (plan.story) q.set('story', plan.story);
    if (plan.look) q.set('look', plan.look);
    await page.goto(`${base.replace('localhost', '127.0.0.1')}?${q}`);
    await page.waitForFunction(() => {
      try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; }
    }, null, { timeout: 120000, polling: 50 });
    const open = JSON.parse(await page.evaluate(() => window.qp.metrics())).open_ms;
    await settle(page, 120000);
    if (first) {
      const steps = Number(JSON.parse(await page.evaluate(() => window.qp.status())).step.split('/')[1]);
      const to = Math.max(1, steps - (plan.back_steps || 400));
      await page.evaluate((s) => { document.getElementById('c_date').value = String(s); document.getElementById('c_go').click(); }, to);
      await page.waitForFunction((s) => {
        try { return Number(JSON.parse(window.qp.status()).step.split(' ')[0]) >= s; } catch { return false; }
      }, to, { timeout: 600000, polling: 100 });
      await settle(page, 120000);
    }
    let opened = null;
    for (const id of plan.open) {
      await page.evaluate((i) => window.qp.openCapsule(i), id);
      await settle(page, 120000);
      const b = await page.locator('#scene').boundingBox();
      if (!b) continue;
      await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
      await settle(page, 120000);
      const o = JSON.parse(await page.evaluate(() => window.qp.open()));
      if (o.depth === 'chart') { opened = id; break; }
    }
    if (!opened) throw new Error('no capsule reached chart depth');
    const vis = await zoomTo(page, depth, plan.rows);
    await page.waitForTimeout(300);
    await page.evaluate(() => window.qp.resetMetrics());
    await play(page);
    await page.waitForTimeout(plan.play_ms || 5000);
    await play(page);
    const mPlay = await readReset(page);
    await settle(page, 120000);
    await page.evaluate(() => window.qp.resetMetrics());
    const b = await page.locator('#scene').boundingBox();
    const x = b.x + b.width / 2; const y = b.y + b.height * 0.4;
    const end = Date.now() + (plan.drag_ms || 6000);
    let k = 0;
    // One drag = pointer down, 48 moves of a triangle wave (±40 px) about 16 ms apart
    // (≈ 0.8 s held, so that consecutive animation frames are in mode `drag`), pointer up.
    while (Date.now() < end) {
      await page.mouse.move(x, y);
      await page.mouse.down();
      for (let i = 1; i <= 48; i++) {
        const t = (i % 24) / 24;
        const dx = (t < 0.5 ? t * 4 - 1 : 3 - t * 4) * 40;
        await page.mouse.move(x + dx, y);
        await page.waitForTimeout(16);
      }
      await page.mouse.up();
      await page.waitForTimeout(150);
      k += 1;
    }
    const mDrag = await readReset(page);
    const status = JSON.parse(await page.evaluate(() => window.qp.status()));
    return { opened, visible: vis, open_ms: open, play: mPlay, drag: mDrag, drags: k, status: status.step, errors };
  } finally {
    await ctx.close();
  }
}

function pick(m, ph, name, k) { return m[ph].span_ms[name] && m[ph].span_ms[name][k]; }

const res = { story: plan.story || null, look: plan.look || null, market: plan.market, configs: {} };
for (const c of plan.configs || ['phone-jit-est', 'phone']) {
  const cfg = CFG[c];
  const browser = await cal.launchFor(cfg);
  const { child, base } = await startDemo(qp, web, Number(process.env.QP_DEMO_PORT || 8771), plan.market);
  const out = { browser: browser.version(), by_depth: {} };
  try {
    let first = true;
    for (const depth of plan.depths) {
      const runs = [];
      for (let r = 0; r < (plan.reps || 3); r++) {
        try {
          runs.push(await oneRun(browser, cfg, base, depth, first));
          first = false;
        } catch (e) {
          console.error(`k52chart: ${c} depth ${depth} rep ${r} failed: ${String(e).split('\n')[0]}`);
        }
      }
      const med = (f) => median(runs.map(f));
      const row = {
        runs: runs.length,
        visible: runs.map((x) => x.visible && x.visible.n),
        range: runs.map((x) => x.visible && x.visible.range),
        tick_p95: med((x) => pick(x, 'play', 'tick', 'p95')),
        tick_n: med((x) => pick(x, 'play', 'tick', 'n')),
        scene_play_p95: med((x) => pick(x, 'play', 'scene', 'p95')),
        scene_play_n: med((x) => pick(x, 'play', 'scene', 'n')),
        scene_drag_p95: med((x) => pick(x, 'drag', 'scene', 'p95')),
        scene_drag_n: med((x) => pick(x, 'drag', 'scene', 'n')),
        paint_pass_drag_p95: med((x) => pick(x, 'drag', 'paint_pass', 'p95')),
        scene_build_drag_p95: med((x) => pick(x, 'drag', 'scene_build', 'p95')),
        drag_frames_n: med((x) => x.drag.frame_ms.drag && x.drag.frame_ms.drag.n),
        drag_frame_p99: med((x) => x.drag.frame_ms.drag && x.drag.frame_ms.drag.p99),
        drag_long: med((x) => x.drag.frame_ms.drag && x.drag.frame_ms.drag.long),
        drag_long_runs: runs.map((x) => x.drag.frame_ms.drag && x.drag.frame_ms.drag.long),
        scene_drag_p95_runs: runs.map((x) => pick(x, 'drag', 'scene', 'p95')),
        first_capsule_ms: med((x) => x.open_ms.first_capsule),
        errors: runs.flatMap((x) => x.errors),
        steps: runs.map((x) => x.status),
      };
      const jit = c !== 'phone';
      const bias = jit ? { scene: 1, tick: 1 } : { scene: cal.PHONE_BIAS.scene, tick: cal.PHONE_BIAS.tick_with_scene };
      row.scene_p95 = Math.max(row.scene_play_p95 ?? 0, row.scene_drag_p95 ?? 0);
      row.biased = { tick_p95: row.tick_p95 * bias.tick, scene_p95: row.scene_p95 * bias.scene };
      row.checks = jit
        ? { tick: row.tick_p95 <= 8, scene: row.scene_p95 <= 8, drag_long: row.drag_long !== null && row.drag_long <= 0.01 }
        : { drag_long_usable: row.drag_long !== null && row.drag_long <= 0.10 };
      row.pass = runs.length > 0 && Object.values(row.checks).every(Boolean);
      out.by_depth[depth] = row;
      console.error(`k52chart: ${plan.story || plan.look || 'r4'} ${c} depth ${depth}: ${JSON.stringify(row.checks)} tick ${row.tick_p95} scene ${row.scene_p95} long ${row.drag_long}`);
    }
  } finally {
    await browser.close();
    child.kill();
    await new Promise((r) => { child.once('exit', r); setTimeout(r, 2000); });
  }
  out.pass = Object.values(out.by_depth).every((r) => r.pass);
  res.configs[c] = out;
}
res.pass = Object.values(res.configs).every((c) => c.pass);
fs.writeFileSync(outFile, `${JSON.stringify(res, null, 1)}\n`);
console.log(JSON.stringify({ story: res.story, pass: res.pass }));
