// (temporary, M21 K52) Frame intervals during depth transitions and the core cost of the
// motion overlay, on the calibrated phone settings (shells/web/tests/calibrate.mjs).
//   node tests/_m21trans.mjs <qp> <out.json> [--motion nabiz] [--transition zumlak] [--config phone-jit-est]
import fs from 'node:fs';
import { startDemo, settle, clickIn } from './lib.mjs';
import * as cal from './calibrate.mjs';

const argv = process.argv.slice(2);
const [qp, outFile] = argv;
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const motion = opt('--motion', '');
const transition = opt('--transition', '');
const cfgName = opt('--config', 'phone-jit-est');
const reps = Number(opt('--reps', '4'));
const seek = opt('--seek', '700');
const CFG = { 'phone-jit-est': cal.PHONE_JIT_EST, phone: cal.PHONE };
const cfg = CFG[cfgName];
const browser = await cal.launchFor(cfg);
const { child, base } = await startDemo(qp, process.cwd(), Number(process.env.QP_DEMO_PORT || 8738));
const q = new URLSearchParams({ x: '1' });
if (motion) q.set('motion', motion);
if (transition) q.set('transition', transition);
if (opt('--order', '')) q.set('feed_order', opt('--order', ''));
const res = { idle: [], scroll: [], motion, transition, config: cfgName, open: [], chart: [], marks_us: null };
try {
  const ctx = await browser.newContext({ viewport: cfg.viewport, deviceScaleFactor: cfg.dpr });
  const page = await ctx.newPage();
  await page.goto(`${base.replace('localhost', '127.0.0.1')}?${q}`);
  await settle(page, 120000);
  await page.evaluate((s) => { document.getElementById('c_date').value = s; document.getElementById('c_go').click(); }, seek);
  await page.waitForTimeout(1500);
  await settle(page, 120000);
  // rAF recorder
  await page.evaluate(() => {
    window.__d = [];
    window.__rec = false;
    let last = 0;
    const f = (ts) => { if (window.__rec && last) window.__d.push(ts - last); last = ts; requestAnimationFrame(f); };
    requestAnimationFrame(f);
  });
  const take = async (fn, ms) => {
    await page.evaluate(() => { window.__d = []; window.__rec = true; });
    await fn();
    await page.waitForTimeout(ms);
    return page.evaluate(() => { window.__rec = false; return window.__d; });
  };
  res.idle = await take(async () => {}, 5000);
  res.marks_live = await page.evaluate(() => document.querySelectorAll('.mk').length);
  res.scroll = await take(async () => {
    await page.mouse.move(195, 450);
    for (let k = 0; k < 60; k++) { await page.mouse.wheel(0, Math.floor(k / 12) % 2 ? -120 : 120); await page.waitForTimeout(40); }
  }, 100);
  await page.locator('#feed').evaluate((e) => { e.scrollTop = 0; });
  await page.waitForTimeout(300);
  const caps = await page.evaluate(() => JSON.parse(window.qp.feed()).filter((i) => i.id && i.scene).map((i) => i.id));
  const id = caps[0];
  for (let r = 0; r < reps; r++) {
    await page.locator('#feed').evaluate((e) => { e.scrollTop = 0; });
    await page.waitForTimeout(200);
    res.open.push(...await take(() => clickIn(page, `#list .face[data-id="${id}"] canvas`, 200, 80), 700));
    await settle(page, 60000);
    res.chart.push(...await take(() => clickIn(page, '#scene', 195, 150), 700));
    await settle(page, 60000);
    await page.evaluate(() => document.getElementById('back').click());
    await settle(page, 60000);
    await page.evaluate(() => document.getElementById('back').click());
    await settle(page, 60000);
  }
  // Core cost of one overlay frame (the faces on screen at the feed's top).
  res.marks_us = await page.evaluate(() => {
    const n = 2000;
    const t0 = performance.now();
    for (let i = 0; i < n; i++) window.qp.marks(t0 + i * 16);
    return ((performance.now() - t0) / n) * 1000;
  });
  res.marks_sample = await page.evaluate(() => window.qp.marks(performance.now()));
} finally {
  await browser.close();
  child.kill();
}
const stat = (a) => {
  const s = [...a].sort((x, y) => x - y);
  const q = (p) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return { n: s.length, p50: q(0.5), p95: q(0.95), p99: q(0.99), max: s[s.length - 1], long: s.filter((x) => x > 33.4).length };
};
const out = { idle: stat(res.idle), scroll: stat(res.scroll), marks_live: res.marks_live, motion, transition, config: cfgName, open: stat(res.open), chart: stat(res.chart), marks_us: res.marks_us, marks_sample: String(res.marks_sample).slice(0, 300) };
fs.writeFileSync(outFile, `${JSON.stringify(out, null, 1)}\n`);
console.log(JSON.stringify(out));
