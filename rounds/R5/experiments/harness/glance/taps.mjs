// R5/M20 (SPEC Ek G7b): deterministic tap count per question — the fewest taps from the
// feed to a screen where the answer is visible. The harness does not guess hit regions: each
// candidate tap is performed and the core decides what it opens (`tap_at`/`hit`: depth, a
// question card, a glossary explanation). Breadth first, at most `max_taps`; every branch is
// rebuilt from the feed (Back to the feed, tap the target face, replay the path).
//
//   node taps.mjs <plan.json>
//
// plan = {qp, web, port, out, step, voice, lang, theme, grid: [dx, dy], max_taps,
//         designs: [{name, open, look}], readers: [{principal, candidates, watch}]}
//
// Candidate taps on a screen: every point of a dx × dy CSS px grid over the visible open scene,
// and the open layer's DOM tap targets (buttons, `summary`; not Back, the tabs or Settings).
// "Answer visible" predicates (Ek G7b) on the text in the 402×874 viewport (DOM text + the open
// scene's text primitives); `whole` also counts text below the fold (needs a scroll).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../../../..');
const { startDemo, settle, launch } = await import(path.join(root, 'shells/web/tests/lib.mjs'));

const plan = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const VIEW = { width: 402, height: 874 };
const HOOK = `
;(() => { const f = Shell.prototype.on_frame;
  Shell.prototype.on_frame = function (...a) { globalThis.__qpcore = this; return f.apply(this, a); }; })();
`;
const SCAFFOLD_CSS = '#controls, #banner, #toast { display: none !important; }';

const FAMILY = { price: ['fiyat', 'price'], volume: ['hacim', 'volume'], flow: ['akış', 'takas', 'flow'], news: ['haber', 'bilanço', 'news'] };
const WHO = ['sunucu doğruladı', 'bu cihazda sınandı', 'bu cihazda sınanıyor', 'kaynaklar farklı', 'cihazda sınan',
  'verified by the server', 'checked on this device', 'checking on this device', 'sources differ'];
const NUMBERS = ['yetki', 'lisans', 'abonelik', 'entitle', 'licen', 'subscri'];

function preds(text, families) {
  const t = text.toLocaleLowerCase('tr');
  const out = [];
  const stems = families.flatMap((f) => FAMILY[f] || []);
  if (stems.some((s) => t.includes(s))) out.push('e_rests');
  if (WHO.some((s) => t.includes(s))) out.push('e_who');
  if (NUMBERS.some((s) => t.includes(s))) out.push('numbers');
  return out;
}

async function screen(page) {
  return page.evaluate(() => {
    const H = innerHeight;
    const inView = [];
    const all = [];
    const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let n = walk.nextNode(); n; n = walk.nextNode()) {
      const t = n.textContent.trim();
      if (!t) continue;
      const el = n.parentElement;
      if (el.closest('#controls, #banner, #toast, #list, #tabs, .page:not(#feed)')) continue;
      const cs = getComputedStyle(el);
      if (cs.visibility === 'hidden' || cs.display === 'none' || el.closest('[hidden]')) continue;
      const range = document.createRange(); range.selectNodeContents(n);
      const r = range.getBoundingClientRect();
      if (r.width <= 0 || r.height <= 0) continue;
      all.push(t);
      if (r.bottom > 0 && r.top < H) inView.push(t);
    }
    const sc = document.getElementById('scene');
    if (sc && !sc.hidden && globalThis.__qpcore) {
      const rect = sc.getBoundingClientRect();
      let d = null;
      try { d = JSON.parse(globalThis.__qpcore.scene_desc(true) || 'null'); } catch (e) { d = null; }
      if (d && d.prims) {
        const s = Number(d.scale) || 1;
        for (const p of d.prims) {
          if (p.k !== 'text' || !p.t) continue;
          const y = rect.top + p.y / s;
          all.push(p.t);
          if (y > 0 && y < H) inView.push(p.t);
        }
      }
    }
    let open = {};
    try { open = JSON.parse(window.qp.open()); } catch (e) { /* none */ }
    return { depth: open.depth, view: inView.join(' ‖ '), whole: all.join(' ‖ ') };
  });
}

async function depth(page) { return (await page.evaluate(() => JSON.parse(window.qp.open()))).depth; }

async function faceTap(page, id) {
  await page.evaluate((id) => {
    const el = document.querySelector(`#list .face[data-id="${id}"]`);
    const top = document.getElementById('top').getBoundingClientRect().bottom;
    el.scrollIntoView({ block: 'start' });
    const r = el.getBoundingClientRect();
    if (r.top < top) window.scrollBy(0, r.top - top - 4);
  }, id);
  await settle(page);
  const box = await page.locator(`#list .face[data-id="${id}"]`).boundingBox();
  await page.mouse.click(box.x + box.width / 2, box.y + Math.min(box.height / 2, 60));
  await settle(page);
}

async function toFeed(page) {
  for (let i = 0; i < 6 && (await depth(page)) !== 'sentence'; i++) {
    await page.evaluate(() => document.getElementById('back').click());
    await settle(page);
  }
}

async function act(page, a) {
  if (a.t === 'grid') {
    const b = await page.locator('#scene').boundingBox();
    if (!b) return false;
    await page.mouse.click(b.x + a.x, b.y + a.y);
  } else {
    const loc = page.locator(a.sel).nth(a.i);
    if (!(await loc.isVisible())) return false;
    await loc.click();
  }
  await settle(page);
  return true;
}

async function candidates(page) {
  const [gx, gy] = plan.grid || [20, 16];
  const out = [];
  const b = await page.locator('#scene').boundingBox().catch(() => null);
  const shown = b && await page.locator('#scene').isVisible();
  if (shown) {
    for (let y = gy / 2; y < b.height; y += gy) {
      if (b.y + y < 0 || b.y + y > VIEW.height) continue;
      for (let x = gx / 2; x < b.width; x += gx) {
        if (b.x + x < 0 || b.x + x > VIEW.width) continue;
        out.push({ t: 'grid', x, y });
      }
    }
  }
  for (const sel of ['#layer button:not(#back)', '#open button:not(#back)', 'summary']) {
    const n = await page.locator(sel).count();
    for (let i = 0; i < n; i++) {
      const loc = page.locator(sel).nth(i);
      if (await loc.isVisible()) {
        const id = await loc.evaluate((e) => e.id || e.textContent.trim().slice(0, 30));
        out.push({ t: 'dom', sel, i, id });
      }
    }
  }
  // the same DOM element can match two selectors: keep one
  const seen = new Set();
  return out.filter((a) => { const k = a.t === 'grid' ? `g${a.x},${a.y}` : `d${a.id}`; if (seen.has(k)) return false; seen.add(k); return true; });
}

async function main() {
  const { child, base } = await startDemo(plan.qp, plan.web, plan.port || 8661, []);
  const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
  const browser = await launch();
  const res = { grid: plan.grid || [20, 16], max_taps: plan.max_taps || 4, readers: {} };
  try {
    let seeked = false;
    for (const rd of plan.readers) {
      const d = plan.design;
      const ctx = await browser.newContext({ viewport: VIEW, deviceScaleFactor: 1 });
      await ctx.route('**/pkg/qp_wasm.js', async (route) => {
        const r = await route.fetch();
        route.fulfill({ response: r, body: (await r.text()) + HOOK, headers: { ...r.headers(), 'content-type': 'text/javascript' } });
      });
      const page = await ctx.newPage();
      const q = new URLSearchParams({ anim: '0', paint: 'canvas', canvas: 'run', lang: plan.lang || 'tr', theme: plan.theme || 'light' });
      q.set('principal', rd.principal);
      if (d.look) q.set('look', d.look);
      if (d.open) q.set('open', d.open);
      if (plan.voice) q.set('voice', plan.voice);
      if (rd.watch) q.set('watch', rd.watch);
      await page.goto(`${origin}/?${q}`);
      await settle(page);
      await page.evaluate((v) => { const e = document.getElementById('s_sens'); e.value = v; e.dispatchEvent(new Event('change')); }, 'all');
      await settle(page);
      if (!seeked) {
        await page.fill('#c_date', String(plan.step));
        await page.click('#c_go');
        await settle(page);
        seeked = true;
      }
      await page.addStyleTag({ content: SCAFFOLD_CSS });
      await settle(page);
      const feed = JSON.parse(await page.evaluate(() => window.qp.feed())).filter((r) => r.id).map((r) => r.id);
      const target = rd.candidates.find((c) => feed.includes(c));
      if (!target) { res.readers[rd.principal] = { error: 'no candidate in the feed' }; await ctx.close(); continue; }
      // the key family from the engine's evidence rows (evidence depth), then back to the feed
      await faceTap(page, target);
      await page.evaluate(() => window.qp.openTo('evidence'));
      await settle(page);
      const ev = await page.evaluate(() => JSON.parse(globalThis.__qpcore.evidence()));
      const families = [...new Set(ev.map((e) => e.family).filter((f) => FAMILY[f]))];
      const fam = families.length ? families : (plan.families || []);
      await toFeed(page);

      // "neden rakam göremiyorum" is a question of the unentitled reader only.
      const qs = rd.principal === 'visitor' ? ['e_rests', 'e_who', 'numbers'] : ['e_rests', 'e_who'];
      const best = {}; const bestWhole = {};
      const note = (taps, scr, path) => {
        for (const k of preds(scr.view, fam)) if (!(k in best)) best[k] = { taps, depth: scr.depth, path };
        for (const k of preds(scr.whole, fam)) if (!(k in bestWhole)) bestWhole[k] = { taps, depth: scr.depth, path };
      };
      await faceTap(page, target);
      const s1 = await screen(page);
      note(1, s1, []);
      const seen = new Set([`${s1.depth}|${s1.view}`]);
      let frontier = [[]];
      let tried = 0;
      for (let taps = 2; taps <= (plan.max_taps || 4); taps++) {
        if (qs.every((k) => k in best)) break;
        const next = [];
        for (const pathSoFar of frontier) {
          // rebuild the state, list its candidate taps
          await toFeed(page); await faceTap(page, target);
          for (const a of pathSoFar) await act(page, a);
          const cands = await candidates(page);
          for (const a of cands) {
            await toFeed(page); await faceTap(page, target);
            let ok = true;
            for (const p of pathSoFar) ok = ok && await act(page, p);
            ok = ok && await act(page, a);
            tried++;
            if (!ok) continue;
            const scr = await screen(page);
            const sig = `${scr.depth}|${scr.view}`;
            const p2 = [...pathSoFar, a.t === 'grid' ? { t: 'grid', x: a.x, y: a.y } : { t: 'dom', sel: a.sel, i: a.i, id: a.id }];
            note(taps, scr, p2);
            if (!seen.has(sig)) { seen.add(sig); next.push(p2); }
          }
          if (qs.every((k) => k in best)) break;
        }
        frontier = next;
        console.error(`taps: ${d.name} ${rd.principal} level ${taps}: ${next.length} new screens, tried ${tried}, found ${Object.keys(best)}`);
      }
      res.readers[rd.principal] = { target, families: fam, first_screen_depth: s1.depth, best, best_with_scroll: bestWhole,
        screens_seen: seen.size, taps_tried: tried };
      await ctx.close();
    }
  } finally {
    await browser.close();
    child.kill();
  }
  fs.writeFileSync(plan.out, `${JSON.stringify(res, null, 1)}\n`);
  console.log(JSON.stringify({ design: plan.design.name, readers: Object.fromEntries(Object.entries(res.readers).map(([k, v]) => [k, v.best ? Object.fromEntries(Object.entries(v.best).map(([q, b]) => [q, b.taps])) : v])) }));
}

await main();
