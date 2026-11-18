// R5/M20 (SPEC Ek G7c): the K52 budget of one open-capsule design ON THE SAME SCREEN for every
// design — capsule depth — on the calibrated phone simulation of RP3
// (shells/web/tests/calibrate.mjs: `PHONE_JIT_EST` = HTTPS/JIT on, binding; `PHONE` = JIT off).
// (The calibration's fixed tap point opens the chart in `r4` but a question card in a card
// design; here nobody taps: the capsule is opened by the core's `open_capsule`.)
//
//   node k52open.mjs <qp> <web> <out.json> [--open NAME] [--look NAME] [--voice NAME]
//                    [--reps 3] [--configs phone-jit-est,phone] [--seek 700]
//
// Per config: one `qp demo --synthetic`, sought once to `--seek`; per rep a fresh context:
// the page (`?open=`), the reader's first feed capsule opened to capsule depth, then
// (1) play 10 s (tick, scene paint with data, frames while idle), (2) paused, 6 s of wheel
// scrolling over the open layer (120 px every 100 ms, direction flips every second). The
// core's metrics summary is read at the end of each phase. The wheel phase's long-frame share
// is the core's `long` share of `frame_ms` in the mode the shell gives the wheel (`scroll` or
// `zoom`, whichever has frames).
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
const openName = opt('--open', '');
const look = opt('--look', '');
const voice = opt('--voice', '');
const reps = Number(opt('--reps', '3'));
const seekTo = Number(opt('--seek', '700'));
const configs = opt('--configs', 'phone-jit-est,phone').split(',');
const CFG = { 'phone-jit-est': cal.PHONE_JIT_EST, phone: cal.PHONE };

const median = (xs) => {
  const v = xs.filter((x) => typeof x === 'number' && Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const m = v.length >> 1;
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
};
const play = (page) => page.evaluate(() => document.getElementById('c_play').click());
async function readReset(page) {
  const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
  await page.evaluate(() => window.qp.resetMetrics());
  return m;
}
const span = (m, name, k) => m.span_ms && m.span_ms[name] && m.span_ms[name][k];
function wheelFrames(m) {
  const f = m.frame_ms || {};
  const c = ['scroll', 'zoom', 'drag'].map((k) => [k, f[k]]).filter(([, v]) => v && v.n);
  if (!c.length) return { mode: null, n: 0, p99: null, long: null };
  c.sort((a, b) => b[1].n - a[1].n);
  return { mode: c[0][0], n: c[0][1].n, p99: c[0][1].p99, long: c[0][1].long };
}

async function oneRun(browser, cfg, base, first) {
  const ctx = await browser.newContext({ viewport: cfg.viewport, deviceScaleFactor: cfg.dpr });
  try {
    const page = await ctx.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    const q = new URLSearchParams({ x: '1' });
    if (openName) q.set('open', openName);
    if (look) q.set('look', look);
    if (voice) q.set('voice', voice);
    await page.goto(`${base.replace('localhost', '127.0.0.1')}?${q}`);
    await page.waitForFunction(() => {
      try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; }
    }, null, { timeout: 120000, polling: 50 });
    const openMs = JSON.parse(await page.evaluate(() => window.qp.metrics())).open_ms;
    await settle(page, 120000);
    if (first) {
      await page.evaluate((s) => { document.getElementById('c_date').value = String(s); document.getElementById('c_go').click(); }, seekTo);
      await settle(page, 120000);
    }
    const id = JSON.parse(await page.evaluate(() => window.qp.feed())).find((r) => r.id).id;
    await page.evaluate((i) => window.qp.openCapsule(i), id);
    await settle(page, 120000);
    const o = JSON.parse(await page.evaluate(() => window.qp.open()));
    if (o.depth !== 'capsule') throw new Error(`not at capsule depth: ${o.depth}`);
    await page.waitForTimeout(300);
    await page.evaluate(() => window.qp.resetMetrics());
    await play(page);
    await page.waitForTimeout(10000);
    await play(page);
    const mPlay = await readReset(page);
    await settle(page, 120000);
    await page.evaluate(() => window.qp.resetMetrics());
    const b = await page.locator('#scene').boundingBox();
    const x = b.x + b.width / 2;
    const y = Math.min(b.y + b.height / 2, cfg.viewport.height * 0.6);
    await page.mouse.move(x, y);
    const end = Date.now() + 6000;
    let n = 0;
    while (Date.now() < end) {
      const dir = Math.floor(n / 10) % 2 ? -1 : 1;
      await page.mouse.wheel(0, 120 * dir);
      await page.waitForTimeout(100);
      n += 1;
    }
    await page.waitForTimeout(200);
    const mWheel = await readReset(page);
    const o2 = JSON.parse(await page.evaluate(() => window.qp.open()));
    return { capsule: id, depth_after: o2.depth, frame: o.frame, open_ms: openMs, play: mPlay, wheel: mWheel, wheels: n, errors };
  } finally {
    await ctx.close();
  }
}

const res = { open: openName || null, look: look || null, voice: voice || null, seek: seekTo, configs: {} };
for (const c of configs) {
  const cfg = CFG[c];
  const browser = await cal.launchFor(cfg);
  const { child, base } = await startDemo(qp, web, Number(process.env.QP_DEMO_PORT || 8771), []);
  const runs = [];
  try {
    for (let r = 0; r < reps; r++) {
      try {
        runs.push(await oneRun(browser, cfg, base, r === 0));
      } catch (e) {
        console.error(`k52open: ${c} rep ${r} failed: ${String(e).split('\n')[0]}`);
      }
    }
  } finally {
    await browser.close();
    child.kill();
    await new Promise((r) => { child.once('exit', r); setTimeout(r, 2000); });
  }
  const med = (f) => median(runs.map(f));
  const row = {
    runs: runs.length,
    frame: runs.map((x) => x.frame),
    depth_after_wheel: runs.map((x) => x.depth_after),
    tick_p95: med((x) => span(x.play, 'tick', 'p95')),
    tick_n: med((x) => span(x.play, 'tick', 'n')),
    scene_play_p95: med((x) => span(x.play, 'scene', 'p95')),
    scene_play_n: med((x) => span(x.play, 'scene', 'n')),
    scene_wheel_p95: med((x) => span(x.wheel, 'scene', 'p95')),
    scene_wheel_n: med((x) => span(x.wheel, 'scene', 'n')),
    idle_long: med((x) => x.play.frame_ms && x.play.frame_ms.idle && x.play.frame_ms.idle.long),
    wheel_mode: runs.map((x) => wheelFrames(x.wheel).mode),
    wheel_frames_n: med((x) => wheelFrames(x.wheel).n),
    wheel_frame_p99: med((x) => wheelFrames(x.wheel).p99),
    wheel_long: med((x) => wheelFrames(x.wheel).long),
    wheel_long_runs: runs.map((x) => wheelFrames(x.wheel).long),
    first_capsule_ms: med((x) => x.open_ms.first_capsule),
    errors: runs.flatMap((x) => x.errors),
  };
  const jit = c !== 'phone';
  const bias = jit ? { scene: 1, tick: 1 } : { scene: cal.PHONE_BIAS.scene, tick: cal.PHONE_BIAS.tick_with_scene };
  row.scene_p95 = Math.max(row.scene_play_p95 ?? 0, row.scene_wheel_p95 ?? 0);
  row.biased = { tick_p95: row.tick_p95 * bias.tick, scene_p95: row.scene_p95 * bias.scene };
  // A design whose wheel moves nothing records no wheel frames: no long frame can occur.
  const long = row.wheel_long ?? 0;
  row.checks = jit
    ? { tick: row.tick_p95 <= 8, scene: row.scene_p95 <= 8, wheel_long: long <= 0.01, first_capsule: row.first_capsule_ms <= 1500 }
    : { wheel_long_usable: long <= 0.10 };
  row.pass = runs.length > 0 && Object.values(row.checks).every(Boolean);
  res.configs[c] = row;
  console.error(`k52open: ${openName || 'r4'} ${c}: ${JSON.stringify(row.checks)} tick ${row.tick_p95} scene ${row.scene_p95} long ${row.wheel_long} (${row.wheel_mode})`);
}
res.pass = Object.values(res.configs).every((c) => c.pass);
fs.writeFileSync(outFile, `${JSON.stringify(res, null, 1)}\n`);
console.log(JSON.stringify({ open: res.open, pass: res.pass }));
