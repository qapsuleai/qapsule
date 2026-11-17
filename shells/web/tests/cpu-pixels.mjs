// RP3/M12a: the `cpu` paint path against the reference path (`ref`) in a real browser.
// For the demo shots of screens.mjs (faces and scenes, R3 and the presentation's default
// look) at DPR 1 and 3, the canvas pixels of a `?paint=cpu` page are compared with those
// of a `?paint=ref` page at the same replay moment; then, with the chart open on both,
// several ticks (each a dirty-rectangle update on the cpu page), a zoom and a drag
// (painted at scale 1 while moving, at the device scale after) are compared again.
// Tolerance: TOL_PX pixels may differ by at most TOL_LEVEL per channel (both 0: the path
// is byte-identical by design; anything else is a bug). At DPR 1 the cpu canvases are also
// compared with fixtures/golden/demo.
//
// Usage: node tests/cpu-pixels.mjs <qp> [--browsers chromium,webkit] [--port 8444]
//          [--out <dir>]   (--out writes a JSON report per browser)
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { chromium, webkit } from 'playwright';
import { startDemo, settle, clickIn } from './lib.mjs';

const TOL_PX = 0;
const TOL_LEVEL = 0;

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const browsers = opt('--browsers', 'chromium,webkit').split(',');
const port = Number(opt('--port', '8444'));
const out = opt('--out', null);
const here = new URL('..', import.meta.url).pathname;
const golden = path.resolve(here, '../../fixtures/golden/demo');
const ENGINES = { chromium, webkit };

const R3 = { face: 'r3', feed: 'r3', transition: 'r3', outcome: 'r3' };
const SHOTS = [
  { name: 'feed-tr-dark', look: R3, canvases: ['face-0', 'face-1', 'face-2'] },
  { name: 'feed-en-light', look: R3, lang: 'en', theme: 'light', canvases: ['face-0'] },
  { name: 'capsule-tr-dark', look: R3, open: 'capsule', canvases: ['scene'] },
  { name: 'chart-tr-dark', look: R3, open: 'chart', canvases: ['scene'] },
  { name: 'chart-en-light-rozet', look: R3, lang: 'en', theme: 'light', alphabet: 'rozet', open: 'chart', canvases: ['scene'] },
  { name: 'feed-tr-dark-r4', canvases: ['face-0', 'face-1', 'face-2'] },
  { name: 'feed-en-light-r4', lang: 'en', theme: 'light', canvases: ['face-0', 'face-1'] },
  { name: 'feed-tr-dark-fotolay', face: 'fotolay', canvases: ['face-0', 'face-1'] },
];

function decode(buf) {
  let i = 8, w = 0, h = 0;
  const idat = [];
  while (i < buf.length) {
    const n = buf.readUInt32BE(i);
    const t = buf.toString('latin1', i + 4, i + 8);
    const c = buf.subarray(i + 8, i + 8 + n);
    if (t === 'IHDR') { w = c.readUInt32BE(0); h = c.readUInt32BE(4); if (c[8] !== 8 || c[9] !== 6) throw new Error('unsupported PNG'); }
    if (t === 'IDAT') idat.push(c);
    i += 12 + n;
  }
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const st = w * 4;
  const px = Buffer.alloc(st * h);
  let prev = Buffer.alloc(st);
  for (let y = 0, p = 0; y < h; y++) {
    const f = raw[p];
    const line = Buffer.from(raw.subarray(p + 1, p + 1 + st));
    p += 1 + st;
    for (let x = 0; x < st; x++) {
      const a = x >= 4 ? line[x - 4] : 0, b = prev[x], c = x >= 4 ? prev[x - 4] : 0;
      let v = 0;
      if (f === 1) v = a; else if (f === 2) v = b; else if (f === 3) v = (a + b) >> 1;
      else if (f === 4) { const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c); v = pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      line[x] = (line[x] + v) & 255;
    }
    line.copy(px, y * st);
    prev = line;
  }
  return { w, h, px };
}

/** {px, max, over2, share, box}: differing pixels, max channel difference, pixels > 2 levels, their box. */
function diff(a, b) {
  if (a.w !== b.w || a.h !== b.h) return { size: `${a.w}x${a.h} vs ${b.w}x${b.h}`, px: a.w * a.h, max: 255, over2: a.w * a.h, share: 1 };
  let n = 0, max = 0, over2 = 0;
  const box = [a.w, a.h, -1, -1];
  for (let k = 0; k < a.px.length; k += 4) {
    let m = 0;
    for (let j = 0; j < 4; j++) m = Math.max(m, Math.abs(a.px[k + j] - b.px[k + j]));
    if (!m) continue;
    n++;
    max = Math.max(max, m);
    if (m > 2) over2++;
    const x = (k / 4) % a.w, y = Math.floor(k / 4 / a.w);
    box[0] = Math.min(box[0], x); box[1] = Math.min(box[1], y); box[2] = Math.max(box[2], x); box[3] = Math.max(box[3], y);
  }
  return { px: n, max, over2, share: over2 / (a.w * a.h), box: n ? box : null };
}

async function shotPage(browser, origin, s, paint, dpr) {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: dpr });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  const q = new URLSearchParams({ anim: '0', lang: s.lang || 'tr', theme: s.theme || 'dark', paint });
  if (s.alphabet) q.set('alphabet', s.alphabet);
  if (s.face) q.set('face', s.face);
  for (const [k, v] of Object.entries(s.look || {})) q.set(k, v);
  await page.goto(`${origin}/?${q}`);
  await settle(page);
  if (s.open) {
    const first = await page.evaluate(() => JSON.parse(window.qp.feed())[0]);
    await clickIn(page, `#list .face[data-id="${first.id}"] canvas`, 120, 60);
    await settle(page);
    if (s.open === 'chart') {
      await clickIn(page, '#scene', 195, 150);
      await settle(page);
    }
  }
  const used = await page.evaluate(() => window.qp.paintUsed());
  if (used !== paint) throw new Error(`${s.name}: paint ${used}, want ${paint}`);
  return { ctx, page, errors };
}

async function grab(page, s, c) {
  const feed = (await page.evaluate(() => JSON.parse(window.qp.feed()))).filter((r) => r.id);
  const [, i] = c.split('-');
  const id = c === 'scene' ? 'scene' : feed[Number(i)].id;
  return decode(Buffer.from(await page.evaluate((x) => window.qp.png(x), id)));
}

let failed = 0;
for (const [k, name] of browsers.entries()) {
  const { child, base } = await startDemo(qp, here, port + k);
  const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
  const browser = await ENGINES[name].launch();
  const report = [];
  const bad = (what) => { console.error(`cpu-pixels ${name}: ${what}`); failed++; };
  const check = (label, d) => {
    report.push({ label, ...d });
    if (d.size || d.px > TOL_PX || d.max > TOL_LEVEL) bad(`${label}: ${JSON.stringify(d)}`);
  };
  try {
    // The replay moment of the goldens (shared by every page of this host).
    {
      const ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
      const page = await ctx.newPage();
      await page.goto(`${origin}/?anim=0`);
      await settle(page);
      await page.fill('#c_date', '120');
      await page.click('#c_go');
      await settle(page);
      await ctx.close();
    }
    for (const dpr of [1, 3]) {
      for (const s of SHOTS) {
        const a = await shotPage(browser, origin, s, 'ref', dpr);
        const b = await shotPage(browser, origin, s, 'cpu', dpr);
        for (const c of s.canvases) {
          const ra = await grab(a.page, s, c), rb = await grab(b.page, s, c);
          check(`dpr${dpr} ${s.name}.${c}`, diff(ra, rb));
          const g = path.join(golden, `${s.name}.${c}.png`);
          if (dpr === 1 && fs.existsSync(g)) check(`dpr1 ${s.name}.${c} vs golden`, diff(decode(fs.readFileSync(g)), rb));
        }
        if (a.errors.length || b.errors.length) bad(`${s.name}: page errors ${a.errors} ${b.errors}`);
        await a.ctx.close();
        await b.ctx.close();
      }
    }
    // The chart open on both pages; ticks, a zoom and a drag; compare after each.
    const s = SHOTS[3];
    const a = await shotPage(browser, origin, s, 'ref', 3);
    const b = await shotPage(browser, origin, s, 'cpu', 3);
    const both = async (label) => {
      await settle(a.page);
      await settle(b.page);
      await b.page.waitForTimeout(100); // the settle repaint (one animation frame) after a drag
      check(`dpr3 chart ${label}`, diff(await grab(a.page, s, 'scene'), await grab(b.page, s, 'scene')));
    };
    for (const step of [121, 122, 123, 125, 128]) {
      await b.page.evaluate(() => window.qp.resetMetrics());
      await a.page.fill('#c_date', String(step));
      await a.page.click('#c_go');
      await both(`step ${step}`);
    }
    for (const p of [a.page, b.page]) {
      const box = await p.locator('#scene').boundingBox();
      await p.mouse.move(box.x + 250, box.y + 150);
      await p.mouse.wheel(0, -100);
    }
    await both('wheel');
    for (const p of [a.page, b.page]) {
      const box = await p.locator('#scene').boundingBox();
      await p.mouse.move(box.x + 200, box.y + 150);
      await p.mouse.down();
      for (let x = 1; x <= 6; x++) await p.mouse.move(box.x + 200 + 15 * x, box.y + 150);
      await settle(p);
      if (p === b.page) {
        const w = await p.evaluate(() => document.getElementById('scene').width);
        report.push({ label: 'dpr3 chart while dragging: cpu canvas width', value: w });
        if (w !== 390) bad(`while dragging the cpu scene is ${w} px wide, want 390 (scale 1)`);
      }
      await p.mouse.up();
    }
    await both('after drag');
    const w = await b.page.evaluate(() => document.getElementById('scene').width);
    if (w !== 1170) bad(`after the drag the cpu scene is ${w} px wide, want 1170`);
    const m = JSON.parse(await b.page.evaluate(() => window.qp.metrics()));
    report.push({ label: 'cpu spans during the chart part', spans: Object.fromEntries(Object.entries(m.span_ms || {}).filter(([k]) => k.startsWith('cpu') || k === 'scene')) });
    if (a.errors.length || b.errors.length) bad(`chart: page errors ${a.errors} ${b.errors}`);
    await a.ctx.close();
    await b.ctx.close();
    console.log(`cpu-pixels ${name}: ${report.filter((r) => 'px' in r).length} comparisons, ${report.filter((r) => r.px > 0).length} with differences`);
  } catch (e) {
    bad(String(e.stack || e));
  } finally {
    if (out) {
      fs.mkdirSync(out, { recursive: true });
      fs.writeFileSync(path.join(out, `cpu-pixels-${name}.json`), JSON.stringify(report, null, 1));
    }
    await browser.close();
    child.kill();
  }
}
process.exit(failed ? 1 : 0);
