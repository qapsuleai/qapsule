// RP3/M12c: the `gpu` paint path (WebGPU) in a real browser.
//
// 1. Visual difference against the reference renderer: for the screenshot harness's
//    shots that hold scenes (screens.mjs SHOTS, phone viewport, DPR as given), the same
//    canvas is read from a `?paint=gpu` page and a `?paint=ref` page at the same replay
//    moment (`window.qp.png`). Per image: differing pixels, largest channel difference,
//    share of pixels more than 2 levels off, and how many of those lie on an edge (the
//    reference pixel's 3×3 neighbourhood is not uniform: anti-aliasing, glyph edges) or
//    inside a flat area. Fails beyond the tolerance below.
// 2. Fallback: WebGPU absent (`navigator.gpu` removed before the page runs) → registry
//    `ref`, reason `unsupported`; present but without an adapter (Chromium's headless
//    shell) → the path paints with `ref` and reports `unsupported`; a lost device
//    (`destroy()`) → the path asks for a new one and paints again. Each without page
//    errors, faces painted.
//
// Usage: node tests/gpu.mjs <qp> [--browsers chromium,webkit] [--dpr 3] [--out <dir>]
//          [--port 8460] [--only fallback|diff]
//   Chromium runs as the full browser (`channel: chromium`, new headless), which exposes
//   the machine's GPU (Metal on macOS); its headless shell has no WebGPU adapter.
//   --out: writes gpu-diff-<browser>.json and per-image diff maps (PNG) there.
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { chromium, webkit } from 'playwright';
import { startDemo, settle, clickIn } from './lib.mjs';

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const browsers = opt('--browsers', 'chromium,webkit').split(',');
const dpr = Number(opt('--dpr', '3'));
const out = opt('--out', null);
const port = Number(opt('--port', '8461'));
const only = opt('--only', null);
const here = new URL('..', import.meta.url).pathname;
// The screenshot harness's shots that hold scenes (screens.mjs SHOTS; copied, since
// importing screens.mjs runs it).
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
const ENGINES = { chromium: [chromium, { channel: 'chromium' }], webkit: [webkit, {}] };

// Tolerance (measured, rounds/RP3/experiments/M12c.md): at most this share of pixels more
// than 2 levels off, and at most this share of them inside flat areas (a missing or moved
// element shows up as flat-area differences; MSAA vs analytic coverage only on edges).
const MAX_OFF = 0.01;
const MAX_FLAT = 0.0001;

function check(cond, msg) {
  if (!cond) throw new Error(`gpu: ${msg}`);
}

function decode(buf) {
  let i = 8;
  let w = 0, h = 0, ct = 0;
  const idat = [];
  while (i < buf.length) {
    const n = buf.readUInt32BE(i);
    const t = buf.toString('latin1', i + 4, i + 8);
    const c = buf.subarray(i + 8, i + 8 + n);
    if (t === 'IHDR') { w = c.readUInt32BE(0); h = c.readUInt32BE(4); ct = c[9]; }
    if (t === 'IDAT') idat.push(c);
    i += 12 + n;
  }
  const bpp = ct === 6 ? 4 : 3;
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const st = w * bpp;
  const px = Buffer.alloc(st * h);
  let prev = Buffer.alloc(st);
  for (let y = 0, p = 0; y < h; y++) {
    const f = raw[p];
    const line = Buffer.from(raw.subarray(p + 1, p + 1 + st));
    p += 1 + st;
    for (let x = 0; x < st; x++) {
      const a = x >= bpp ? line[x - bpp] : 0, b = prev[x], c = x >= bpp ? prev[x - bpp] : 0;
      let v = 0;
      if (f === 1) v = a; else if (f === 2) v = b; else if (f === 3) v = (a + b) >> 1;
      else if (f === 4) { const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c); v = pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      line[x] = (line[x] + v) & 255;
    }
    line.copy(px, y * st);
    prev = line;
  }
  return { w, h, bpp, px };
}

function crc32(b) {
  let c = ~0;
  for (const x of b) { c ^= x; for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1)); }
  return ~c >>> 0;
}

function png(w, h, rgb) {
  const chunk = (t, d) => {
    const len = Buffer.alloc(4); len.writeUInt32BE(d.length);
    const td = Buffer.concat([Buffer.from(t, 'latin1'), d]);
    const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(td));
    return Buffer.concat([len, td, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 2;
  const raw = Buffer.alloc((w * 3 + 1) * h);
  for (let y = 0; y < h; y++) rgb.copy(raw, y * (w * 3 + 1) + 1, y * w * 3, (y + 1) * w * 3);
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

/** Difference statistics of two decoded images, and a diff map (red: > 2 levels, grey: 1–2). */
function diff(A, B) {
  check(A.w === B.w && A.h === B.h, `size ${A.w}x${A.h} vs ${B.w}x${B.h}`);
  const { w, h, bpp } = A;
  const at = (img, x, y, j) => img.px[(y * w + x) * bpp + j];
  let n = 0, off = 0, flat = 0, worst = 0, sum = 0;
  const map = Buffer.alloc(w * h * 3);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      let m = 0;
      for (let j = 0; j < 3; j++) m = Math.max(m, Math.abs(at(A, x, y, j) - at(B, x, y, j)));
      const g = at(A, x, y, 1) >> 2;
      map[(y * w + x) * 3] = g; map[(y * w + x) * 3 + 1] = g; map[(y * w + x) * 3 + 2] = g;
      if (!m) continue;
      n++; sum += m; worst = Math.max(worst, m);
      if (m <= 2) { map[(y * w + x) * 3] = 120; map[(y * w + x) * 3 + 1] = 120; map[(y * w + x) * 3 + 2] = 160; continue; }
      off++;
      map[(y * w + x) * 3] = 255; map[(y * w + x) * 3 + 1] = 40; map[(y * w + x) * 3 + 2] = 40;
      // Edge: the reference's 3×3 neighbourhood is not uniform.
      let edge = false;
      for (let dy = -1; dy <= 1 && !edge; dy++) {
        for (let dx = -1; dx <= 1 && !edge; dx++) {
          const xx = Math.min(w - 1, Math.max(0, x + dx)), yy = Math.min(h - 1, Math.max(0, y + dy));
          for (let j = 0; j < 3; j++) if (at(B, xx, yy, j) !== at(B, x, y, j)) edge = true;
        }
      }
      if (!edge) flat++;
    }
  }
  const total = w * h;
  return {
    size: `${w}x${h}`, differing: n, differing_share: +(n / total).toFixed(5), max_level: worst,
    mean_level_of_differing: n ? +(sum / n).toFixed(2) : 0, off2: off, off2_share: +(off / total).toFixed(5),
    off2_flat: flat, off2_flat_share: +(flat / total).toFixed(6), map: png(w, h, map),
  };
}

async function shotPage(browser, base, s, paint) {
  const ctx = await browser.newContext({ viewport: { width: 402, height: 874 }, deviceScaleFactor: dpr });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  const q = new URLSearchParams({ anim: '0', lang: s.lang || 'tr', theme: s.theme || 'dark', paint });
  if (s.alphabet) q.set('alphabet', s.alphabet);
  if (s.face) q.set('face', s.face);
  for (const [k, v] of Object.entries(s.look || {})) q.set(k, v);
  await page.goto(`${base}?${q}`);
  await settle(page);
  if (s.open) {
    const first = await page.evaluate(() => JSON.parse(window.qp.feed()).filter((r) => r.id)[0]);
    await clickIn(page, `#list .face[data-id="${first.id}"] canvas`, 120, 60);
    await settle(page);
    if (s.open === 'chart') {
      await clickIn(page, '#scene', 195, 150);
      await settle(page);
    }
  }
  // The GPU path paints once its device is ready (asynchronous): wait for it.
  await page.waitForFunction(() => window.qp.paintUsed() !== 'gpu' || JSON.parse(window.qp.metrics()).span_ms.gpu_cold, null, { timeout: 15000 }).catch(() => {});
  await page.evaluate(() => window.qp.repaint());
  await settle(page);
  await page.waitForTimeout(300);
  return { ctx, page, errors };
}

async function canvasesOf(page, s) {
  const feed = (await page.evaluate(() => JSON.parse(window.qp.feed()))).filter((r) => r.id);
  const closed = feed.filter((r) => r.closed && r.scene).map((r) => r.id);
  const outImgs = {};
  for (const c of s.canvases) {
    const [kind, i] = c.split('-');
    const id = c === 'scene' ? 'scene' : kind === 'closed' ? closed[Number(i)] : feed[Number(i)].id;
    outImgs[c] = decode(Buffer.from(await page.evaluate((x) => window.qp.png(x), id)));
  }
  return outImgs;
}

async function diffs(browser, base, name) {
  const rows = [];
  for (const s of SHOTS) {
    if (!s.canvases.length) continue;
    const g = await shotPage(browser, base, s, 'gpu');
    const r = await shotPage(browser, base, s, 'ref');
    const used = await g.page.evaluate(() => window.qp.paintUsed());
    const m = JSON.parse(await g.page.evaluate(() => window.qp.metrics()));
    check(used === 'gpu' && m.paint.fallback === null, `${name}/${s.name}: paint ${JSON.stringify(m.paint)}`);
    const gi = await canvasesOf(g.page, s);
    const ri = await canvasesOf(r.page, s);
    for (const c of s.canvases) {
      const d = diff(gi[c], ri[c]);
      if (out) fs.writeFileSync(path.join(out, `gpu-diff-${name}-${s.name}.${c}.png`), d.map);
      delete d.map;
      rows.push({ shot: s.name, canvas: c, ...d });
      console.log(`  ${s.name}.${c} ${d.size}: ${d.differing} px differ (${(d.differing_share * 100).toFixed(2)}%), max ${d.max_level}, >2: ${(d.off2_share * 100).toFixed(3)}% (flat ${d.off2_flat})`);
    }
    check(g.errors.length === 0 && r.errors.length === 0, `${name}/${s.name}: page errors ${g.errors} ${r.errors}`);
    await g.ctx.close();
    await r.ctx.close();
  }
  if (out) fs.writeFileSync(path.join(out, `gpu-diff-${name}.json`), JSON.stringify({ browser: name, dpr, rows }, null, 1));
  for (const d of rows) {
    check(d.off2_share <= MAX_OFF, `${name}/${d.shot}.${d.canvas}: ${d.off2_share} of pixels > 2 levels off (limit ${MAX_OFF})`);
    check(d.off2_flat_share <= MAX_FLAT, `${name}/${d.shot}.${d.canvas}: ${d.off2_flat} flat-area pixels > 2 levels off (limit share ${MAX_FLAT})`);
  }
}

async function fallbackPage(browser, base, init) {
  const ctx = await browser.newContext({ viewport: { width: 402, height: 874 }, deviceScaleFactor: 1 });
  if (init) await ctx.addInitScript(init);
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(`${base}?anim=0&paint=gpu`);
  await settle(page);
  return { ctx, page, errors };
}

// A face canvas is painted when it is not blank (the background alone is one colour).
const painted = (page) => page.evaluate(() => {
  const id = JSON.parse(window.qp.feed()).filter((r) => r.scene)[0].id;
  const bytes = window.qp.png(id);
  return bytes.length > 2000;
});

async function fallbacks(browser, base, name, launchShell) {
  // WebGPU absent (WebKit without WebGPU, Lockdown Mode, a disabled flag).
  {
    const { ctx, page, errors } = await fallbackPage(browser, base, () => { Object.defineProperty(Navigator.prototype, 'gpu', { get: () => undefined }); });
    const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    check(m.paint.used === 'ref' && m.paint.fallback === 'unsupported', `${name} no-webgpu: paint ${JSON.stringify(m.paint)}`);
    check(await painted(page), `${name} no-webgpu: face not painted`);
    check(errors.length === 0, `${name} no-webgpu: ${errors}`);
    console.log(`  fallback no-webgpu: ${JSON.stringify(m.paint)}`);
    await ctx.close();
  }
  // No adapter (requestAdapter resolves null).
  {
    const { ctx, page, errors } = await fallbackPage(browser, base, () => { GPU.prototype.requestAdapter = () => Promise.resolve(null); });
    await page.waitForFunction(() => JSON.parse(window.qp.metrics()).paint.used === 'ref', null, { timeout: 10000 });
    await page.evaluate(() => window.qp.repaint());
    await settle(page);
    const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    check(m.paint.fallback === 'unsupported', `${name} no-adapter: paint ${JSON.stringify(m.paint)}`);
    check(await painted(page), `${name} no-adapter: face not painted`);
    check(errors.length === 0, `${name} no-adapter: ${errors}`);
    console.log(`  fallback no-adapter: ${JSON.stringify(m.paint)}`);
    await ctx.close();
  }
  // A lost device: the path asks for a new one and paints again.
  {
    const keep = () => {
      const req = GPUAdapter.prototype.requestDevice;
      window.__devices = [];
      GPUAdapter.prototype.requestDevice = function (...a) { return req.apply(this, a).then((d) => { window.__devices.push(d); return d; }); };
    };
    const { ctx, page, errors } = await fallbackPage(browser, base, keep);
    await page.waitForFunction(() => JSON.parse(window.qp.metrics()).span_ms.gpu_cold, null, { timeout: 15000 });
    await page.evaluate(() => window.__devices[0].destroy());
    await page.waitForFunction(() => window.__devices.length === 2, null, { timeout: 15000 });
    await page.waitForTimeout(500);
    await page.evaluate(() => window.qp.repaint());
    await settle(page);
    const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    check(m.paint.used === 'gpu' && m.paint.fallback === null, `${name} lost: paint ${JSON.stringify(m.paint)}`);
    check(await painted(page), `${name} lost: face not painted after the new device`);
    check(errors.length === 0, `${name} lost: ${errors}`);
    console.log(`  device lost → new device, painted: ${JSON.stringify(m.paint)}`);
    await ctx.close();
  }
  // Chromium's headless shell: navigator.gpu exists, no adapter.
  if (launchShell) {
    const shell = await chromium.launch();
    try {
      const { ctx, page, errors } = await fallbackPage(shell, base, null);
      await page.waitForFunction(() => JSON.parse(window.qp.metrics()).paint.used === 'ref', null, { timeout: 10000 });
      await page.evaluate(() => window.qp.repaint());
      await settle(page);
      check(await painted(page), 'headless shell: face not painted');
      check(errors.length === 0, `headless shell: ${errors}`);
      const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
      console.log(`  fallback chromium headless shell (no adapter): ${JSON.stringify(m.paint)}`);
      await ctx.close();
    } finally {
      await shell.close();
    }
  }
}

let failed = 0;
if (out) fs.mkdirSync(out, { recursive: true });
for (const [k, name] of browsers.entries()) {
  const { child, base: b } = await startDemo(qp, here, port + k);
  const base = b.replace('localhost', '127.0.0.1');
  const [engine, o] = ENGINES[name];
  const browser = await engine.launch(o);
  try {
    const has = await (async () => {
      const p = await browser.newPage();
      await p.goto(base);
      const r = await p.evaluate(async () => Boolean(navigator.gpu && await navigator.gpu.requestAdapter()));
      await p.close();
      return r;
    })();
    console.log(`gpu ${name} ${browser.version()}: WebGPU adapter ${has ? 'yes' : 'no'}`);
    if (only !== 'diff') await fallbacks(browser, base, name, name === 'chromium');
    if (only !== 'fallback') {
      if (has) await diffs(browser, base, name);
      else console.log(`  ${name}: no WebGPU adapter, visual comparison skipped`);
    }
  } catch (e) {
    console.error(String(e));
    failed++;
  } finally {
    await browser.close();
    child.kill();
  }
}
process.exit(failed ? 1 : 0);
