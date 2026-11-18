// R5/fix47 — contrast probe over every surface of the presentation (acceptance criterion 4,
// "bütün öğeler"). Derived from the auditor's `rounds/R5/audit/probe.mjs` (same hook, same
// handles), extended: every capsule of the feed opened (live, no longer holding, closed) to
// capsule depth, evidence and story chart; the shell's DOM text on the feed, the open
// capsule, the evidence tab and the settings tab; both themes and both languages; the
// look as presented (`oltan`) unless `--look` is given.
//
//   node rounds/R5/experiments/S/contrast/probe.mjs <qp> <port> <out.json> [--look oltan]
//                                                   [--max-open 12] [--shots <dir>]  (from the repo root)
//
// Writes the core's paint lists (`face_scene`, `scene_desc`, device pixels with `scale`) and
// the DOM texts' computed colours; `analyze.py` computes the contrasts.
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const { startDemo, settle, launch } = await import(path.join(root, 'shells/web/tests/lib.mjs'));
const argv = process.argv.slice(2);
const [qp, port, outPath] = argv;
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const look = opt('--look', '');
const maxOpen = Number(opt('--max-open', '12'));
const shots = opt('--shots', '');
const HOOK = `\n;(() => { const f = Shell.prototype.on_frame;
  Shell.prototype.on_frame = function (...a) { globalThis.__probe_core = this; return f.apply(this, a); }; })();\n`;

// A hard upper bound: a stuck page must not hold the port (kills the demo, exits 1).
let child = null;
const limit = setTimeout(() => {
  console.error('probe: time limit reached');
  if (child) child.kill('SIGKILL');
  process.exit(1);
}, Number(opt('--limit-s', '900')) * 1000);
const started = await startDemo(qp, 'shells/web', port);
child = started.child;
const { base } = started;
const browser = await launch();
const out = { base, look: look || '(presented)', faces: [], scenes: [], dom: [], errors: [] };

async function newPage(q) {
  const ctx = await browser.newContext({ viewport: { width: 402, height: 874 }, deviceScaleFactor: 1 });
  await ctx.route('**/pkg/qp_wasm.js', async (route) => {
    const r = await route.fetch();
    route.fulfill({ response: r, body: (await r.text()) + HOOK, headers: { ...r.headers(), 'content-type': 'text/javascript' } });
  });
  const page = await ctx.newPage();
  page.on('pageerror', (e) => out.errors.push(String(e)));
  await page.goto(`${base}${q}`);
  await settle(page);
  return { ctx, page };
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
    const c = globalThis.__probe_core;
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
      let hidden = false;
      for (let e = el; e; e = e.parentElement) if (e.hidden || getComputedStyle(e).display === 'none') hidden = true;
      if (hidden) continue;
      let op = 1; for (let e = el; e; e = e.parentElement) op *= Number(getComputedStyle(e).opacity);
      if (op === 0) continue;
      res.push({ t: t.slice(0, 60), tag: el.tagName, id: el.id, cls: String(el.className), color: cs.color, bg: bgOf(el),
        px: parseFloat(cs.fontSize), weight: cs.fontWeight, opacity: op });
    }
    // Non-text chrome: borders and outlines of controls (inputs, selects, buttons).
    const ctl = [];
    for (const el of document.querySelectorAll('input, select, button')) {
      const r = el.getBoundingClientRect();
      const cs = getComputedStyle(el);
      if (r.width === 0 || r.height === 0 || cs.visibility === 'hidden') continue;
      let hidden = false;
      for (let e = el; e; e = e.parentElement) if (e.hidden || getComputedStyle(e).display === 'none') hidden = true;
      if (hidden) continue;
      const parentBg = (() => {
        for (let e = el.parentElement; e; e = e.parentElement) {
          const c = getComputedStyle(e).backgroundColor;
          if (c && !/rgba\(0, 0, 0, 0\)|transparent/.test(c)) return c;
        }
        return getComputedStyle(document.documentElement).backgroundColor;
      })();
      ctl.push({ tag: el.tagName, id: el.id, type: el.type || '', border: cs.borderTopWidth !== '0px' ? cs.borderTopColor : null,
        fill: cs.backgroundColor, bg: parentBg });
    }
    return { texts: res, controls: ctl };
  });
}

const themes = ['dark', 'light'];
for (const theme of themes) {
  for (const lang of ['tr', 'en']) {
    const q = new URLSearchParams({ theme, lang, anim: '0' });
    if (look) q.set('look', look);
    const { ctx, page } = await newPage(`/?${q}`);
    await setSens(page, 'all');
    for (const step of [1, 40, 120, 200]) {
      await seek(page, step);
      out.faces.push({ theme, lang, step, faces: await faceScenes(page) });
      if (step === 120) out.dom.push({ theme, lang, where: 'feed', ...(await domTexts(page)) });
    }
    // Every validity: open the live, the no-longer-holding and the closed capsules (the
    // feed rows' `closed` flag; validity from the open capsule).
    const feed = await page.evaluate(() => JSON.parse(window.qp.feed()).filter((r) => r.id));
    const closed = feed.filter((r) => r.closed);
    const live = feed.filter((r) => !r.closed);
    const pick = [...live.slice(0, Math.ceil(maxOpen / 2)), ...closed.slice(0, Math.floor(maxOpen / 2))];
    let first = true;
    for (const row of pick) {
      await page.evaluate((id) => window.qp.openCapsule(id), row.id);
      await settle(page);
      for (const to of ['capsule', 'evidence', 'chart']) {
        try { await page.evaluate((x) => window.qp.openTo(x), to); await settle(page); } catch (e) { continue; }
        const sc = await page.evaluate(() => { const s = globalThis.__probe_core.scene_desc(true); return s ? JSON.parse(s) : null; });
        out.scenes.push({ theme, lang, id: row.id, closed: Boolean(row.closed), to, scene: sc });
        if (first || to === 'capsule') out.dom.push({ theme, lang, where: `open:${to}:${row.closed ? 'closed' : 'live'}`, ...(await domTexts(page)) });
      }
      first = false;
      await page.evaluate(() => { try { window.qp.openTo('feed'); } catch (e) { /* none */ } });
      await settle(page);
    }
    await page.click('#tabs button[data-tab="settings"]').catch(() => {});
    await page.waitForTimeout(200);
    out.dom.push({ theme, lang, where: 'settings', ...(await domTexts(page)) });
    // Native checkboxes are drawn by the browser (CSS colours do not apply): their pixels,
    // 2 px around the box included, for analyze.py (`--shots <dir>`).
    if (shots && lang === 'tr') {
      fs.mkdirSync(shots, { recursive: true });
      const boxes = await page.$$('input[type=checkbox]');
      let k = 0;
      for (const b of boxes) {
        if (!(await b.isVisible())) continue;
        const file = path.join(shots, `checkbox-${theme}-${k++}.png`);
        await b.scrollIntoViewIfNeeded({ timeout: 5000 });
        await b.screenshot({ path: file, timeout: 10000, animations: 'disabled' });
        const bg = await b.evaluate((el) => {
          for (let e = el.parentElement; e; e = e.parentElement) {
            const c = getComputedStyle(e).backgroundColor;
            if (c && c !== 'rgba(0, 0, 0, 0)' && c !== 'transparent') return c;
          }
          return getComputedStyle(document.documentElement).backgroundColor;
        });
        out.checkboxes = out.checkboxes || [];  // the box itself (no margin)
        out.checkboxes.push({ theme, file, bg });
      }
    }
    await ctx.close();
  }
}

await browser.close();
child.kill();
clearTimeout(limit);
fs.writeFileSync(outPath, JSON.stringify(out));
console.log(`probe: faces ${out.faces.reduce((a, f) => a + Object.keys(f.faces).length, 0)}, scenes ${out.scenes.filter((s) => s.scene).length}, dom blocks ${out.dom.length}, errors ${out.errors.length}`);
