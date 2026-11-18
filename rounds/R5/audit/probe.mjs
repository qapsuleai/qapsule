// R5/A (denetçi) — kendi tarayıcı denememdir; düzeneğin shoot.mjs'ini kullanmaz.
// node rounds/R5/audit/probe.mjs <qp> <port> <out.json>     (depo kökünden)
//
// Toplananlar (sentetik pazar, `qp demo --synthetic --paused`):
//  1. Gösterim varsayılanlarıyla (oltan + marla + pavka + tesir + surek) iki temada, duyarlılık
//     `all`, birkaç adımda: akıştaki her yüzün boyama listesi (`face_scene`), açılmış kapsülün ve
//     grafiğin boyama listesi (`scene_desc`), görünür DOM metinlerinin rengi ve zemini.
//     Karşıtlık Python'da (audit/colour.py) hesaplanır.
//  2. Ağ: her sayfanın (gösterim, galeri, ?paint=cpu) bütün istekleri; başka kökene istek sayısı.
//  3. `?paint=cpu`: ölçüm özetindeki paint {requested, used, fallback}.
//  4. Hareketi azalt: aynı adımda `reducedMotion: reduce` ve `no-preference` ile son ekranın
//     yüz metinleri ve işaret ilkelleri aynı mı (gerekli koşul).
//  5. Yetkisiz okur (principal=visitor): yüz boyama listelerinde ve DOM'da rakam (simge hariç).
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const { startDemo, settle, launch } = await import(path.join(root, 'shells/web/tests/lib.mjs'));
const [qp, port, outPath] = process.argv.slice(2);
const HOOK = `\n;(() => { const f = Shell.prototype.on_frame;
  Shell.prototype.on_frame = function (...a) { globalThis.__audit_core = this; return f.apply(this, a); }; })();\n`;

const { child, base } = await startDemo(qp, 'shells/web', port, ['--gallery']);
const origin = new URL(base).origin;
const browser = await launch();
const out = { base, pages: [], faces: [], scenes: [], dom: [], paint_cpu: null, reduce: [], visitor: [] };

async function newPage(q, opts = {}) {
  const ctx = await browser.newContext({ viewport: { width: 402, height: 874 }, deviceScaleFactor: 1,
    reducedMotion: opts.reduce ? 'reduce' : 'no-preference' });
  await ctx.route('**/pkg/qp_wasm.js', async (route) => {
    const r = await route.fetch();
    route.fulfill({ response: r, body: (await r.text()) + HOOK, headers: { ...r.headers(), 'content-type': 'text/javascript' } });
  });
  const page = await ctx.newPage();
  const reqs = [];
  page.on('request', (r) => reqs.push(r.url()));
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(`${base}${q}`);
  if (!opts.noSettle) await settle(page);
  out.pages.push({ q, reqs, errors });
  return { ctx, page, reqs };
}

async function setSens(page, v) {
  await page.evaluate((x) => { const e = document.getElementById('s_sens'); e.value = x; e.dispatchEvent(new Event('change')); }, v);
  await settle(page);
}
async function seek(page, step) {
  await page.fill('#c_date', String(step));
  await page.click('#c_go');
  await settle(page);
}
async function faceScenes(page) {
  return page.evaluate(() => {
    const c = globalThis.__audit_core;
    const ids = JSON.parse(window.qp.feed()).filter((r) => r.id).map((r) => r.id);
    const o = {};
    for (const id of ids) { const s = c.face_scene(id, true); if (s) o[id] = JSON.parse(s); }
    return o;
  });
}
async function domTexts(page) {
  return page.evaluate(() => {
    const bgOf = (el) => {
      for (let e = el; e; e = e.parentElement) {
        const c = getComputedStyle(e).backgroundColor;
        if (c && !/rgba\(0, 0, 0, 0\)|transparent/.test(c)) return c;
      }
      return getComputedStyle(document.documentElement).backgroundColor;
    };
    const res = [];
    const w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let n = w.nextNode(); n; n = w.nextNode()) {
      const t = n.textContent.trim();
      if (!t) continue;
      const el = n.parentElement;
      const r = el.getBoundingClientRect();
      const cs = getComputedStyle(el);
      if (r.width === 0 || r.height === 0 || cs.visibility === 'hidden' || cs.display === 'none') continue;
      let op = 1; for (let e = el; e; e = e.parentElement) op *= Number(getComputedStyle(e).opacity);
      if (op === 0) continue;
      res.push({ t: t.slice(0, 60), tag: el.tagName, id: el.id, cls: el.className, color: cs.color, bg: bgOf(el),
        px: parseFloat(cs.fontSize), weight: cs.fontWeight, opacity: op, onscreen: r.bottom > 0 && r.top < innerHeight });
    }
    return res;
  });
}

for (const theme of ['dark', 'light']) {
  for (const lang of ['tr', 'en']) {
    const { ctx, page } = await newPage(`/?theme=${theme}&lang=${lang}&anim=0`);
    await setSens(page, 'all');
    for (const step of [1, 40, 120, 200]) {
      await seek(page, step);
      const faces = await faceScenes(page);
      out.faces.push({ theme, lang, step, faces });
      if (step === 120) out.dom.push({ theme, lang, where: 'feed', texts: await domTexts(page) });
    }
    // open the first, a closed and a live capsule; capsule depth, evidence, chart
    const feed = await page.evaluate(() => JSON.parse(window.qp.feed()).filter((r) => r.id));
    for (const row of feed.slice(0, 3)) {
      await page.evaluate((id) => window.qp.openCapsule(id), row.id);
      await settle(page);
      for (const to of ['capsule', 'evidence', 'chart']) {
        try { await page.evaluate((x) => window.qp.openTo(x), to); await settle(page); } catch (e) { continue; }
        const sc = await page.evaluate(() => { const s = globalThis.__audit_core.scene_desc(true); return s ? JSON.parse(s) : null; });
        out.scenes.push({ theme, lang, id: row.id, to, scene: sc });
        if (to === 'capsule') out.dom.push({ theme, lang, where: `open:${row.id}`, texts: await domTexts(page) });
      }
      await page.keyboard.press('Escape').catch(() => {});
      await page.evaluate(() => { try { window.qp.openTo('feed'); } catch (e) {} });
    }
    await ctx.close();
  }
}

// visitor (no entitlement)
for (const theme of ['dark']) {
  const { ctx, page } = await newPage(`/?theme=${theme}&principal=visitor&anim=0`);
  await setSens(page, 'all').catch(() => {});
  for (const step of [1, 120]) {
    await seek(page, step);
    out.visitor.push({ step, faces: await faceScenes(page), dom: await domTexts(page),
      open: await page.evaluate(() => window.qp.open()) });
  }
  await ctx.close();
}

// ?paint=cpu
{
  const { ctx, page } = await newPage('/?paint=cpu');
  out.paint_cpu = await page.evaluate(() => { const m = JSON.parse(window.qp.metrics()); return m.paint || m; });
  await ctx.close();
}

// reduced motion: end-state equality at the same step
for (const reduce of [false, true]) {
  const { ctx, page } = await newPage('/?theme=dark', { reduce });
  await setSens(page, 'all');
  await seek(page, 216); await seek(page, 217);
  await page.waitForTimeout(6000);
  await settle(page);
  out.reduce.push({ reduce, faces: await faceScenes(page), dom: (await domTexts(page)).map((d) => d.t) });
  await ctx.close();
}

// gallery page: network only (and that it renders something)
{
  const { ctx, page, reqs } = await newPage('/gallery.html', { noSettle: true });
  await page.waitForTimeout(15000);
  out.gallery = { canvases: await page.evaluate(() => document.querySelectorAll('canvas, img').length), reqs: reqs.length };
  await ctx.close();
}

out.foreign = out.pages.flatMap((p) => p.reqs.filter((u) => !u.startsWith(origin) && !u.startsWith('data:') && !u.startsWith('blob:')));
await browser.close();
child.kill();
fs.writeFileSync(outPath, JSON.stringify(out));
console.log(`probe: pages ${out.pages.length}, requests ${out.pages.reduce((a, p) => a + p.reqs.length, 0)}, foreign ${out.foreign.length}, faces ${out.faces.reduce((a, f) => a + Object.keys(f.faces).length, 0)}, scenes ${out.scenes.filter((s) => s.scene).length}, errors ${out.pages.reduce((a, p) => a + p.errors.length, 0)}`);
