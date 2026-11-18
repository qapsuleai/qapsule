// R5 M21 (SPEC Ek G6): K52 during motion — display-frame intervals while the depth
// transitions and the face changes run, on the calibrated phone simulation of RP3
// (shells/web/tests/calibrate.mjs: `phone-jit-est` binding, `phone` JIT off "usable").
//
//   node k52motion.mjs <qp> <web> <out.json> [--param k=v ...] [--configs phone-jit-est,phone]
//                      [--runs 3] [--reps 6] [--seek 120] [--play-ms 8000]
//
// Per run (fresh browser and demo): seek to `--seek`, then `--reps` times: feed -> capsule
// (tap the first face), capsule -> chart (tap the scene), back, back; each window 700 ms
// after the tap. Then the replay plays for `--play-ms` (face changes on the feed: the motion
// designs animate them). Frame intervals come from a requestAnimationFrame recorder; a long
// frame is > 33.4 ms (two display frames at 60 Hz). Gate (Ek G6): JIT-on estimate: long
// frames <= 1 % in the transition windows and in the play window; JIT off: <= 10 %.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../../../..');
const cal = await import(path.join(root, 'shells/web/tests/calibrate.mjs'));
const { startDemo, settle, clickIn } = await import(path.join(root, 'shells/web/tests/lib.mjs'));

const argv = process.argv.slice(2);
const [qp, web, outFile] = argv;
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const params = argv.flatMap((a, i) => (a === '--param' ? [argv[i + 1]] : []));
const configs = opt('--configs', 'phone-jit-est,phone').split(',');
const runs = Number(opt('--runs', '3'));
const reps = Number(opt('--reps', '6'));
const seekTo = opt('--seek', '120');
const playMs = Number(opt('--play-ms', '8000'));
const reduce = argv.includes('--reduce');
const CFG = { 'phone-jit-est': cal.PHONE_JIT_EST, phone: cal.PHONE };
const LONG = 33.4;

const stat = (a) => {
  const s = [...a].sort((x, y) => x - y);
  if (!s.length) return { n: 0 };
  const q = (p) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  const long = s.filter((x) => x > LONG).length;
  return { n: s.length, p50: q(0.5), p95: q(0.95), p99: q(0.99), max: s[s.length - 1], long, long_share: long / s.length };
};

async function once(cfg) {
  const browser = await cal.launchFor(cfg);
  const { child, base } = await startDemo(qp, web, Number(process.env.QP_DEMO_PORT || 8738));
  const q = new URLSearchParams();
  for (const kv of params) { const [k, ...v] = kv.split('='); q.set(k, v.join('=')); }
  const w = { open: [], chart: [], back: [], play: [] };
  let changes = null;
  try {
    const ctx = await browser.newContext({ viewport: cfg.viewport, deviceScaleFactor: cfg.dpr, reducedMotion: reduce ? 'reduce' : 'no-preference' });
    const page = await ctx.newPage();
    await page.goto(`${base.replace('localhost', '127.0.0.1')}?${q}`);
    await settle(page, 120000);
    await page.evaluate((s) => { document.getElementById('c_date').value = s; document.getElementById('c_go').click(); }, seekTo);
    await page.waitForTimeout(1000);
    await settle(page, 120000);
    await page.evaluate(() => {
      window.__d = []; window.__rec = false; let last = 0;
      const f = (ts) => { if (window.__rec && last) window.__d.push(ts - last); last = ts; requestAnimationFrame(f); };
      requestAnimationFrame(f);
    });
    const take = async (fn, ms) => {
      await page.evaluate(() => { window.__d = []; window.__rec = true; });
      await fn();
      await page.waitForTimeout(ms);
      return page.evaluate(() => { window.__rec = false; return window.__d; });
    };
    const id = await page.evaluate(() => JSON.parse(window.qp.feed()).filter((i) => i.id)[0].id);
    for (let r = 0; r < reps; r++) {
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.waitForTimeout(150);
      w.open.push(...await take(() => clickIn(page, `#list .face[data-id="${id}"] canvas`, 180, 60), 700));
      await settle(page, 60000);
      w.chart.push(...await take(() => clickIn(page, '#scene', 195, 150), 700));
      await settle(page, 60000);
      w.back.push(...await take(() => page.evaluate(() => document.getElementById('back').click()), 700));
      await settle(page, 60000);
      w.back.push(...await take(() => page.evaluate(() => document.getElementById('back').click()), 700));
      await settle(page, 60000);
    }
    // (Ek G6, supplement) face changes in view: seek to N, the changing capsule scrolled into
    // view, one replay step, 1000 ms window (`--changes N:id,...`).
    w.change = [];
    for (const spec of (opt('--changes', '') || '').split(',').filter(Boolean)) {
      const [n, cid] = spec.split(':');
      for (let r = 0; r < Number(opt('--change-reps', '3')); r++) {
        await page.evaluate((s) => { document.getElementById('c_date').value = s; document.getElementById('c_go').click(); }, n);
        await page.waitForTimeout(500);
        await settle(page, 120000);
        await page.evaluate((id) => { const e = document.querySelector(`#list .face[data-id="${id}"]`); if (e) e.scrollIntoView({ block: 'center' }); }, cid);
        await page.waitForTimeout(300);
        await settle(page, 60000);
        w.change.push(...await take(() => page.evaluate(() => document.getElementById('c_step').click()), 1000));
      }
    }
    changes = await page.evaluate(() => (window.qp.changed ? window.qp.changed().length : null));
    if (w.change.length) { await ctx.close(); return { change: stat(w.change), changed_faces: changes }; }
    await page.evaluate(() => window.scrollTo(0, 0));
    w.play = await take(() => page.evaluate(() => document.getElementById('c_play').click()), playMs);
    await page.evaluate(() => document.getElementById('c_play').click());
    changes = await page.evaluate(() => (window.qp.changed ? window.qp.changed().length : null));
    await ctx.close();
  } finally {
    await browser.close();
    child.kill();
  }
  const trans = [...w.open, ...w.chart, ...w.back];
  return { open: stat(w.open), chart: stat(w.chart), back: stat(w.back), transitions: stat(trans), play: stat(w.play), changed_faces: changes };
}

const res = { params, reduce, runs, reps, seek: seekTo, play_ms: playMs, configs: {} };
for (const c of configs) {
  const rr = [];
  for (let i = 0; i < runs; i++) rr.push(await once(CFG[c]));
  const med = (f) => { const v = rr.map(f).sort((a, b) => a - b); return v[Math.floor(v.length / 2)]; };
  if (rr[0].change) {
    const pool = rr.flatMap((r) => [r.change.long_share]);
    const med2 = (f) => { const v = rr.map(f).sort((a, b) => a - b); return v[Math.floor(v.length / 2)]; };
    res.configs[c] = { runs: rr, summary: { change_long_share: med2((r) => r.change.long_share), change_p99: med2((r) => r.change.p99), change_max: med2((r) => r.change.max) },
      limit_long_share: c === 'phone' ? 0.10 : 0.01, pass: med2((r) => r.change.long_share) <= (c === 'phone' ? 0.10 : 0.01), pool };
    continue;
  }
  const summary = {
    transitions_long_share: med((r) => r.transitions.long_share), transitions_p99: med((r) => r.transitions.p99),
    transitions_max: med((r) => r.transitions.max), play_long_share: med((r) => r.play.long_share),
    play_p99: med((r) => r.play.p99), play_max: med((r) => r.play.max),
  };
  const lim = c === 'phone' ? 0.10 : 0.01;
  res.configs[c] = { runs: rr, summary, limit_long_share: lim,
    pass: summary.transitions_long_share <= lim && summary.play_long_share <= lim };
}
fs.writeFileSync(outFile, `${JSON.stringify(res, null, 1)}\n`);
console.log(JSON.stringify({ params, configs: Object.fromEntries(Object.entries(res.configs).map(([k, v]) => [k, { ...v.summary, pass: v.pass }])) }));
