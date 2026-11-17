// RP3/M12b: the `canvas` paint path against the reference renderer (VERIFY 5, 7, 8).
//
// For each shot of the screenshot harness (screens.mjs: the six R4/T5 views and the R4
// presentation's faces, synthetic market at step 120, animations off) the page is opened
// twice, with `?paint=ref` and `?paint=canvas`, and every scene canvas (capsule faces, the
// open capsule, the chart) is read back (`getImageData`). Per image it reports the pixels
// that differ, the largest channel difference, the share of pixels more than 2 levels
// apart, and how many of the differing pixels lie on text (the text mask is where the
// canvas path's own image differs from its `?canvas=notext` image, grown by 2 px).
// It also checks that the semantic reads (compute digests, feed, open capsule) are
// byte-identical between the paths, and that the path falls back to `ref`
// ("unsupported") when FontFace is missing.
//
// Usage: node tests/paint-canvas.mjs <qp> [--browsers chromium,webkit] [--dpr 1]
//          [--out <dir>] [--variant <?canvas= value>] [--only <shot>]
//   --out  writes <browser>-<shot>.<canvas>.{ref,canvas,diff}.png and stats.json there
//          (synthetic data only)
// Tolerance (explicit; measured values in rounds/RP3/experiments/M12b.md), for the default
// variant (text from the core's glyph atlas): at most OVER2_SHARE of an image's pixels may
// differ by more than 2 levels and at most OVER32_SHARE by more than 32 (the browser's
// anti-aliasing of strokes and fills differs from tiny-skia's on edge pixels; a moved,
// missing or recoloured element exceeds this at once). With `--variant glyphs|run` the
// browser draws the text itself and the figures are only reported.
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { chromium, webkit } from 'playwright';
import { startDemo, settle, clickIn } from './lib.mjs';

const OVER2_SHARE = 0.015;
const OVER32_SHARE = 0.001;

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const browsers = opt('--browsers', 'chromium,webkit').split(',');
const dpr = Number(opt('--dpr', '1'));
const out = opt('--out', null);
const variant = opt('--variant', '');
const only = opt('--only', null);
const here = new URL('..', import.meta.url).pathname;
const ENGINES = { chromium, webkit };

// screens.mjs SHOTS (that file runs on import, so the list is repeated here).
const R3 = { face: 'r3', feed: 'r3', transition: 'r3', outcome: 'r3' };
const SHOTS = [
  { name: 'feed-tr-dark', look: R3, canvases: ['face-0', 'face-1', 'face-2'] },
  { name: 'feed-en-light', look: R3, lang: 'en', theme: 'light', canvases: ['face-0'] },
  { name: 'capsule-tr-dark', look: R3, open: 'capsule', canvases: ['scene'] },
  { name: 'chart-tr-dark', look: R3, open: 'chart', canvases: ['scene'] },
  { name: 'chart-en-light-rozet', look: R3, lang: 'en', theme: 'light', alphabet: 'rozet', open: 'chart', canvases: ['scene'] },
  { name: 'feed-tr-dark-r4', canvases: ['face-0', 'face-1', 'face-2'] },
  { name: 'feed-tr-light-r4-closed', theme: 'light', canvases: ['closed-0', 'closed-1'] },
  { name: 'feed-en-light-r4', lang: 'en', theme: 'light', canvases: ['face-0', 'face-1'] },
  { name: 'feed-tr-dark-fotolay', face: 'fotolay', canvases: ['face-0', 'face-1'] },
];

function crc32(buf) {
  let c = ~0;
  for (const b of buf) { c ^= b; for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1)); }
  return ~c >>> 0;
}

function png(w, h, rgba) {
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) rgba.copy(raw, y * (w * 4 + 1) + 1, y * w * 4, (y + 1) * w * 4);
  const chunk = (t, d) => {
    const len = Buffer.alloc(4); len.writeUInt32BE(d.length);
    const td = Buffer.concat([Buffer.from(t, 'latin1'), d]);
    const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(td));
    return Buffer.concat([len, td, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 6;
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

async function openShot(browser, base, s, paint, cv) {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: dpr });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  const q = new URLSearchParams({ anim: '0', lang: s.lang || 'tr', theme: s.theme || 'dark', paint });
  if (cv) q.set('canvas', cv);
  for (const [k, v] of new URLSearchParams(opt('--q', ''))) q.set(k, v);
  if (s.alphabet) q.set('alphabet', s.alphabet);
  if (s.face) q.set('face', s.face);
  for (const [k, v] of Object.entries(s.look || {})) q.set(k, v);
  await page.goto(`${base}/?${q}`);
  await settle(page);
  await page.fill('#c_date', '120');
  await page.click('#c_go');
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
  await page.evaluate(() => { document.activeElement && document.activeElement.blur(); window.qp.repaint(); });
  await settle(page);
  // the canvas path paints after its font is in
  await page.waitForTimeout(150);
  await settle(page);
  const feed = (await page.evaluate(() => JSON.parse(window.qp.feed()))).filter((r) => r.id);
  const closed = feed.filter((r) => r.closed && r.scene).map((r) => r.id);
  const images = {};
  for (const c of s.canvases) {
    const [kind, i] = c.split('-');
    const id = c === 'scene' ? 'scene' : kind === 'closed' ? closed[Number(i)] : feed[Number(i)].id;
    images[c] = await page.evaluate((id) => {
      const el = id === 'scene' ? document.getElementById('scene') : document.querySelector(`#list .face[data-id="${id}"] canvas`);
      const d = el.getContext('2d').getImageData(0, 0, el.width, el.height).data;
      let s = '';
      for (let k = 0; k < d.length; k += 0x8000) s += String.fromCharCode.apply(null, d.subarray(k, k + 0x8000));
      return { w: el.width, h: el.height, b64: btoa(s) };
    }, id);
  }
  // `sent` (the protocol log) is not compared: it differs between two `ref` runs too (how
  // many feed queries go out depends on timing; measured).
  const reads = await page.evaluate(() => JSON.stringify([window.qp.digests(), window.qp.feed(), window.qp.open()]));
  const used = await page.evaluate(() => window.qp.paintUsed());
  await ctx.close();
  return { images, reads, used, errors };
}

function compare(a, b, t) {
  const A = Buffer.from(a.b64, 'base64'), B = Buffer.from(b.b64, 'base64'), T = Buffer.from(t.b64, 'base64');
  const { w, h } = a;
  // text mask: canvas with text vs canvas without text, grown by 2 px
  const raw = new Uint8Array(w * h);
  for (let p = 0; p < w * h; p++) {
    const k = p * 4;
    if (B[k] !== T[k] || B[k + 1] !== T[k + 1] || B[k + 2] !== T[k + 2]) raw[p] = 1;
  }
  const mask = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
    if (!raw[y * w + x]) continue;
    for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) {
      const yy = y + dy, xx = x + dx;
      if (yy >= 0 && yy < h && xx >= 0 && xx < w) mask[yy * w + xx] = 1;
    }
  }
  const st = { w, h, px: w * h, differ: 0, over2: 0, over32: 0, max: 0, text_px: 0, text_differ: 0, text_over2: 0, text_max: 0, off_differ: 0, off_over2: 0, off_max: 0 };
  const diff = Buffer.alloc(w * h * 4);
  for (let p = 0; p < w * h; p++) {
    const k = p * 4;
    const m = Math.max(Math.abs(A[k] - B[k]), Math.abs(A[k + 1] - B[k + 1]), Math.abs(A[k + 2] - B[k + 2]), Math.abs(A[k + 3] - B[k + 3]));
    const onText = mask[p] === 1;
    if (onText) st.text_px++;
    diff[k + 3] = 255;
    if (!m) continue;
    st.differ++; st.max = Math.max(st.max, m);
    if (m > 2) st.over2++;
    if (m > 32) st.over32++;
    if (onText) { st.text_differ++; st.text_max = Math.max(st.text_max, m); if (m > 2) st.text_over2++; diff[k] = Math.min(255, 64 + m * 4); }
    else { st.off_differ++; st.off_max = Math.max(st.off_max, m); if (m > 2) st.off_over2++; diff[k + 1] = Math.min(255, 64 + m * 4); }
  }
  st.over2_share = st.over2 / st.px;
  st.off_over2_share = st.off_over2 / st.px;
  st.over32_share = st.over32 / st.px;
  st.ok = st.over2_share <= OVER2_SHARE && st.over32_share <= OVER32_SHARE;
  return { st, diff };
}

let failed = 0;
const all = {};
for (const [k, name] of browsers.entries()) {
  const { child, base: b } = await startDemo(qp, here, 8450 + k);
  const base = new URL(b).origin.replace('localhost', '127.0.0.1');
  const browser = await ENGINES[name].launch(name === 'chromium' ? { args: ['--force-color-profile=srgb', '--font-render-hinting=none'] } : {});
  try {
    // fallback: no FontFace → ref, reported "unsupported"
    {
      const ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
      await ctx.addInitScript(() => { delete window.FontFace; });
      const page = await ctx.newPage();
      await page.goto(`${base}/?anim=0&paint=canvas`);
      await settle(page);
      const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
      await ctx.close();
      const want = { requested: 'canvas', used: 'ref', fallback: 'unsupported' };
      if (!['requested', 'used', 'fallback'].every((x) => m.paint[x] === want[x])) {
        console.error(`paint-canvas ${name}: fallback ${JSON.stringify(m.paint)}, want ${JSON.stringify(want)}`);
        failed++;
      } else console.log(`paint-canvas ${name}: without FontFace → ref (unsupported)`);
    }
    for (const s of SHOTS) {
      if (only && s.name !== only) continue;
      const r = await openShot(browser, base, s, 'ref', '');
      const c = await openShot(browser, base, s, 'canvas', variant);
      const t = await openShot(browser, base, s, 'canvas', variant ? `${variant},notext` : 'notext');
      for (const x of [r, c, t]) if (x.errors.length) { console.error(`${name}/${s.name}: page errors ${x.errors}`); failed++; }
      if (c.used !== 'canvas') { console.error(`${name}/${s.name}: path used ${c.used}`); failed++; }
      if (r.reads !== c.reads) {
        const [ra, ca] = [JSON.parse(r.reads), JSON.parse(c.reads)];
        const which = ['digests', 'feed', 'open'].filter((_, i) => ra[i] !== ca[i]);
        console.error(`${name}/${s.name}: semantic reads differ between ref and canvas: ${which}`);
        if (out) { fs.mkdirSync(out, { recursive: true }); fs.writeFileSync(path.join(out, `${name}-${s.name}.reads.json`), JSON.stringify({ ref: ra, canvas: ca }, null, 1)); }
        failed++;
      }
      for (const cv of s.canvases) {
        const { st, diff } = compare(r.images[cv], c.images[cv], t.images[cv]);
        all[`${name}/${s.name}.${cv}`] = st;
        const pct = (v) => `${(100 * v).toFixed(3)}%`;
        const enforce = !variant || variant.split(',').every((v) => v !== 'glyphs' && v !== 'run');
        console.log(`  ${name} ${s.name}.${cv} ${st.w}x${st.h}: differ ${st.differ} (${pct(st.differ / st.px)}), max ${st.max}, >2 ${pct(st.over2_share)}, >32 ${pct(st.over32_share)}`
          + ` | text ${st.text_differ} px (max ${st.text_max}) | off-text ${st.off_differ} px, >2 ${st.off_over2}, max ${st.off_max} ${st.ok ? 'ok' : enforce ? 'FAIL' : 'over (reported)'}`);
        if (!st.ok && enforce) failed++;
        if (out) {
          fs.mkdirSync(out, { recursive: true });
          const f = (tag, buf) => fs.writeFileSync(path.join(out, `${name}-${s.name}.${cv}.${tag}.png`), png(st.w, st.h, buf));
          f('ref', Buffer.from(r.images[cv].b64, 'base64'));
          f('canvas', Buffer.from(c.images[cv].b64, 'base64'));
          f('diff', diff);
        }
      }
    }
  } catch (e) {
    console.error(String(e && e.stack || e));
    failed++;
  } finally {
    await browser.close();
    child.kill();
  }
}
if (out) fs.writeFileSync(path.join(out, 'stats.json'), JSON.stringify({ dpr, variant, stats: all }, null, 1));
console.log(`paint-canvas: ${Object.keys(all).length} images, ${failed ? `${failed} failure(s)` : 'all within tolerance'}`);
process.exit(failed ? 1 : 0);
