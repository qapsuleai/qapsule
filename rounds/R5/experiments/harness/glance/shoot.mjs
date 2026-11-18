// R5/T2 glance harness: stimulus shooter (SPEC §G1). Headless Chromium on `qp demo
// --synthetic --paused`, at the reader's size: viewport 402×874 CSS px, DPR 1 (no zoom).
//
//   node shoot.mjs <plan.json>
//
// plan = {qp, web, port, out, paint, demo_args, shots: [{
//   id,                       output file stem (a blinded code; the name↔code key is elsewhere)
//   kind,                     face | feed | evidence | chart | motion
//   step,                     replay step (all shots of one plan with the same step share one seek)
//   candidates: [capsule ids] the first one the page lists is the target (face/evidence/chart/motion)
//   lang, theme, principal,   tr|en, light|dark, arkin|visitor
//   look, face,               design selection: `?look=` (T3; ignored by a shell without it), `?face=`
//   broken,                   true: the broken-design filter (SPEC §G8) is applied in the page
//   wheel_out                 chart: wheel-out notches before the shot (M18 depth)
// }]}
//
// Per shot it writes <out>/<id>.png (the phone viewport) and <out>/<id>.json: the target,
// the feed rows fully visible, the engine-side paint lists (`face_scene`/`scene_desc`) of
// the visible faces and the open scene, the DOM text runs with computed colours, the tap
// targets (DOM boxes), the open depth and the taps the harness needed. The page itself is
// not changed: the harness (1) appends to the served `pkg/qp_wasm.js` a two-line hook that
// keeps a reference to the core (`globalThis.__qpcore`) so that engine data can be read,
// (2) hides the demo scaffolding (replay controls, "synthetic" banner) for every design
// alike, (3) for `broken` only, distorts Canvas 2D text/fills (SPEC §G8).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../../../..');
const lib = await import(path.join(root, 'shells/web/tests/lib.mjs'));
const { startDemo, settle, launch } = lib;

const plan = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const out = path.resolve(plan.out);
fs.mkdirSync(out, { recursive: true });
const VIEW = { width: 402, height: 874 };

const HOOK = `
;(() => { const f = Shell.prototype.on_frame;
  Shell.prototype.on_frame = function (...a) { globalThis.__qpcore = this; return f.apply(this, a); }; })();
`;

const SCAFFOLD_CSS = '#controls, #banner, #toast { display: none !important; }';

// SPEC §G8: the broken design. Inverted text hierarchy (large text drawn small and small
// text large), text colour pulled 65% toward the background, every non-background fill
// and stroke (the mark, the chart) at 30% opacity; DOM text at 45% opacity.
function brokenInit() {
  const P = CanvasRenderingContext2D.prototype;
  const lo = 11; const hi = 22;
  const bgOf = () => {
    const c = getComputedStyle(document.body).backgroundColor.match(/\d+(\.\d+)?/g) || [0, 0, 0];
    return c.slice(0, 3).map(Number);
  };
  const parse = (s) => {
    const m = String(s).match(/^#([0-9a-f]{6})([0-9a-f]{2})?$/i);
    if (m) { const n = parseInt(m[1], 16); return [(n >> 16) & 255, (n >> 8) & 255, n & 255]; }
    const r = String(s).match(/\d+(\.\d+)?/g);
    return r ? r.slice(0, 3).map(Number) : null;
  };
  const fillText = P.fillText;
  P.fillText = function (t, x, y, w) {
    this.save();
    const m = String(this.font).match(/(\d+(?:\.\d+)?)px/);
    if (m) {
      const s = Number(m[1]) / (window.devicePixelRatio || 1);
      const s2 = Math.max(9, Math.min(30, lo + hi - s)) * (window.devicePixelRatio || 1);
      this.font = String(this.font).replace(m[0], `${s2}px`);
    }
    const c = parse(this.fillStyle); const b = bgOf();
    if (c) this.fillStyle = `rgb(${c.map((v, i) => Math.round(v * 0.35 + b[i] * 0.65)).join(',')})`;
    const r = fillText.call(this, t, x, y, w);
    this.restore();
    return r;
  };
  const faint = (fn) => function (...a) {
    this.save(); this.globalAlpha *= 0.3; const r = fn.apply(this, a); this.restore(); return r;
  };
  P.fill = faint(P.fill);
  P.stroke = faint(P.stroke);
  const fillRect = P.fillRect;
  P.fillRect = function (x, y, w, h) {
    // The scene's background (the whole canvas) stays; every other rect is faint.
    if (x <= 0 && y <= 0 && w >= this.canvas.width && h >= this.canvas.height) return fillRect.call(this, x, y, w, h);
    this.save(); this.globalAlpha *= 0.3; const r = fillRect.call(this, x, y, w, h); this.restore(); return r;
  };
  document.addEventListener('DOMContentLoaded', () => {
    const st = document.createElement('style');
    st.textContent = '#top, #layerbar, #opentext, #evidence, #hint { opacity: .45 !important; }';
    document.head.append(st);
  });
}

// The authority's frames as the page receives them (SSE), kept for the harness: the
// reader's feed items (`CapsuleItem`), the face scenes (`SceneDesc`, with layer names, hit
// boxes and colours) and the replay time are the engine data the keys and the computed
// measures are made from (SPEC §G2, §G7). Read-only: the page gets every frame unchanged.
function recordFrames() {
  const E = window.EventSource;
  window.__frames = [];
  window.EventSource = function (u, o) {
    const e = new E(u, o);
    e.addEventListener('message', (m) => window.__frames.push(m.data));
    return e;
  };
  window.EventSource.prototype = E.prototype;
}

async function engine(page) {
  return page.evaluate(() => {
    const items = {}; const faces = {}; let t = null; let step = null;
    for (const f of window.__frames) {
      let o; try { o = JSON.parse(f); } catch (e) { continue; }
      const m = o.msg || o;
      const r = o.replay || (m.data && m.data.replay);
      if (r && r.t != null) { t = r.t; step = r.step; }
      const v = (m.event && m.event.viewer) || (m.data && m.data.viewer);
      if (!v) continue;
      for (const e of v.feed || []) items[e.capsule.id] = { ...e.capsule, known_at: e.known_at };
      for (const fc of v.faces || []) faces[fc.capsule] = { scene: fc.scene, trust: fc.trust };
    }
    return { items, faces, replay_t: t, replay_step: step };
  });
}

async function newPage(browser, s) {
  const ctx = await browser.newContext({ viewport: VIEW, deviceScaleFactor: 1 });
  await ctx.route('**/pkg/qp_wasm.js', async (route) => {
    const r = await route.fetch();
    route.fulfill({ response: r, body: (await r.text()) + HOOK, headers: { ...r.headers(), 'content-type': 'text/javascript' } });
  });
  await ctx.addInitScript(recordFrames);
  if (s.broken) await ctx.addInitScript(brokenInit);
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  return { ctx, page, errors };
}

function query(s) {
  const q = new URLSearchParams({ anim: '0', paint: plan.paint || 'canvas', lang: s.lang || 'tr', theme: s.theme || 'light' });
  if ((plan.paint || 'canvas') === 'canvas') q.set('canvas', 'run');
  if (s.principal) q.set('principal', s.principal);
  if (s.look) q.set('look', s.look);
  if (s.face) q.set('face', s.face);
  if (s.watch) q.set('watch', s.watch);
  // (M18) the story chart design at chart depth (`?story=`; a shell without it ignores it).
  if (s.story) q.set('story', s.story);
  return q;
}

async function setSens(page, sens) {
  if (!sens) return;
  await page.evaluate((v) => { const e = document.getElementById('s_sens'); e.value = v; e.dispatchEvent(new Event('change')); }, sens);
  await settle(page);
}

async function seek(page, step) {
  await page.evaluate(() => { window.__frames.length = 0; });
  // (M18) step < 0: the replay's last step (the page's status says `step / steps`).
  if (step < 0) step = Number(JSON.parse(await page.evaluate(() => window.qp.status())).step.split('/')[1]);
  await page.fill('#c_date', String(step));
  await page.click('#c_go');
  await settle(page);
}

async function domInfo(page) {
  return page.evaluate(() => {
    const vis = (r) => r.width > 0 && r.height > 0 && r.bottom > 0 && r.top < innerHeight && r.right > 0 && r.left < innerWidth;
    const bgOf = (el) => {
      for (let e = el; e; e = e.parentElement) {
        const c = getComputedStyle(e).backgroundColor;
        if (c && !/rgba\(0, 0, 0, 0\)|transparent/.test(c)) return c;
      }
      return getComputedStyle(document.body).backgroundColor;
    };
    const texts = [];
    const walk = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let n = walk.nextNode(); n; n = walk.nextNode()) {
      const t = n.textContent.trim();
      if (!t) continue;
      const el = n.parentElement;
      const cs = getComputedStyle(el);
      if (cs.visibility === 'hidden' || cs.display === 'none') continue;
      const range = document.createRange(); range.selectNodeContents(n);
      const r = range.getBoundingClientRect();
      if (!vis(r)) continue;
      let op = 1; for (let e = el; e; e = e.parentElement) op *= Number(getComputedStyle(e).opacity);
      texts.push({ t, color: cs.color, bg: bgOf(el), opacity: op, size: parseFloat(cs.fontSize), weight: cs.fontWeight, rect: [r.left, r.top, r.width, r.height], tag: el.tagName.toLowerCase(), id: el.id || '' });
    }
    const taps = [];
    for (const el of document.querySelectorAll('button, select, input, a, .face, #scene')) {
      const r = el.getBoundingClientRect();
      if (!vis(r) || getComputedStyle(el).visibility === 'hidden') continue;
      taps.push({ tag: el.tagName.toLowerCase(), id: el.id || el.dataset.id || el.dataset.tab || '', rect: [r.left, r.top, r.width, r.height] });
    }
    const faces = [...document.querySelectorAll('#list .face')].map((el) => {
      const r = el.getBoundingClientRect();
      return { id: el.dataset.id, rect: [r.left, r.top, r.width, r.height], full: r.top >= 0 && r.bottom <= innerHeight && r.height > 0 };
    });
    return { texts, taps, faces, body_bg: getComputedStyle(document.body).backgroundColor };
  });
}

async function shoot(page, s, meta) {
  const file = path.join(out, `${s.id}.png`);
  await page.evaluate(() => { document.activeElement && document.activeElement.blur(); });
  await page.screenshot({ path: file, animations: 'disabled', caret: 'hide' });
  const dom = await domInfo(page);
  const paints = await page.evaluate((ids) => {
    const c = globalThis.__qpcore;
    const o = {};
    for (const id of ids) { try { const s = c.face_scene(id, true); if (s) o[id] = JSON.parse(s); } catch (e) { o[id] = { error: String(e) }; } }
    try { const s = c.scene_desc(true); if (s) o.scene = JSON.parse(s); } catch (e) { /* none */ }
    return o;
  }, dom.faces.filter((f) => f.full).map((f) => f.id));
  const open = await page.evaluate(() => JSON.parse(window.qp.open()));
  const look = await page.evaluate(() => { try { return JSON.parse(window.qp.look()); } catch (e) { return null; } });
  const used = await page.evaluate(() => window.qp.paintUsed());
  const eng = await engine(page);
  const full = dom.faces.filter((f) => f.full).map((f) => f.id);
  const engineOut = {
    replay_t: eng.replay_t, replay_step: eng.replay_step,
    target_item: meta.target ? eng.items[meta.target] : null,
    visible_items: full.map((id) => eng.items[id]).filter(Boolean),
    face_scenes: Object.fromEntries(full.filter((id) => eng.faces[id]).map((id) => [id, eng.faces[id]])),
  };
  fs.writeFileSync(path.join(out, `${s.id}.json`), `${JSON.stringify({ shot: s, ...meta, open, look, paint_used: used, engine: engineOut, dom, paints })}\n`);
}

async function targetOf(page, s) {
  const feed = (await page.evaluate(() => JSON.parse(window.qp.feed()))).filter((r) => r.id);
  const ids = new Set(feed.map((r) => r.id));
  const target = (s.candidates || []).find((c) => ids.has(c));
  return { feed, target, row: feed.find((r) => r.id === target) };
}

async function scrollTo(page, id) {
  await page.evaluate((id) => {
    const el = document.querySelector(`#list .face[data-id="${id}"]`);
    const top = document.getElementById('top').getBoundingClientRect().bottom;
    el.scrollIntoView({ block: 'start' });
    const r = el.getBoundingClientRect();
    if (r.top < top) window.scrollBy(0, r.top - top - 4);
  }, id);
  await settle(page);
}

// (M18, GOAL "görünen aralıkta 5 / 50 / 200 kapsül") Stories whose life meets the visible
// range (PROTOCOL §23.1 rule: born <= to, end >= from; an open story ends at the last bar),
// from the device's own list and the view state's range; null when either is missing.
async function visibleStories(page, list) {
  return page.evaluate((rows) => {
    let st = null;
    try { st = JSON.parse(globalThis.__qpcore.stories()); } catch (e) { /* none */ }
    const all = rows || (st && st.stories);
    let look = null;
    try { look = JSON.parse(window.qp.look()); } catch (e) { /* none */ }
    if (!all || !look || !look.range) return null;
    const { from_bar: a, to_bar: b } = look.range;
    let last = 0;
    for (const r of all) last = Math.max(last, r.last_bar);
    const n = all.filter((r) => r.born_bar <= b && (r.closed_bar === undefined || r.closed_bar === null ? last : r.closed_bar) >= a).length;
    return { n, range: [a, b], rows: all.length };
  }, list || null);
}

// Wheel over the right end of the price plot (the last bar stays in view) until the visible
// story count is the smallest >= `want` the wheel reaches (or the range stops moving);
// `list`: the canonical story list (a design whose device holds none, e.g. the reference).
async function zoomTo(page, want, list) {
  const b = await page.locator('#scene').boundingBox();
  const x = b.x + b.width * 0.78; const y = b.y + b.height * 0.3;
  await page.mouse.move(x, y);
  let v = await visibleStories(page, list);
  const trail = [v];
  if (!v) return { error: 'no story list or range', trail };
  let n = 0;
  // Out while too few (positive wheel = out), in while more than enough; stop at the first
  // state that has >= want stories after a zoom-in would drop below want.
  for (; n < 80; n++) {
    const dir = v.n < want ? 1 : -1;
    await page.mouse.wheel(0, 120 * dir);
    await settle(page);
    const w = await visibleStories(page, list);
    trail.push(w);
    if (!w || (w.range[0] === v.range[0] && w.range[1] === v.range[1])) { v = w || v; break; }
    if (dir === -1 && w.n < want) {
      // one notch back out: the smallest count >= want
      await page.mouse.wheel(0, 120);
      await settle(page);
      v = await visibleStories(page, list);
      trail.push(v);
      break;
    }
    v = w;
    if (dir === 1 && v.n >= want) break;
  }
  return { want, notches: n, visible: v, trail: trail.slice(-6) };
}

// (M18) Chart-state probe: at the last step, the story list of every instrument in the
// reader's feed (the device's list at chart depth), for the state rule in glance.py.
async function chartProbe() {
  const { child, base } = await startDemo(plan.qp, plan.web, plan.port || 8461, plan.demo_args || []);
  const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
  const browser = await launch();
  const res = { instruments: {} };
  try {
    const { ctx, page } = await newPage(browser, {});
    const url = `${origin}/?${query({ kind: 'feed', principal: 'arkin', theme: 'light' })}`;
    await page.goto(url);
    await settle(page);
    await setSens(page, plan.sensitivity);
    await seek(page, -1);
    res.status = JSON.parse(await page.evaluate(() => window.qp.status()));
    const feed = JSON.parse(await page.evaluate(() => window.qp.feed())).filter((r) => r.id);
    const firsts = {};
    for (const r of feed) { const i = r.id.split('.')[1]; if (!firsts[i]) firsts[i] = r.id; }
    for (const [inst, id] of Object.entries(firsts)) {
      await page.evaluate((id) => window.qp.openCapsule(id), id);
      await settle(page);
      const b = await page.locator('#scene').boundingBox();
      await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
      await settle(page);
      const st = await page.evaluate(() => { try { return JSON.parse(globalThis.__qpcore.stories()); } catch (e) { return null; } });
      res.instruments[inst] = { opened: id, stories: st };
      await page.goto(url);
      await settle(page);
    }
    await ctx.close();
  } finally {
    await browser.close();
    child.kill();
  }
  fs.writeFileSync(path.join(out, 'chart-probe.json'), `${JSON.stringify(res)}\n`);
  console.log(`chart probe: ${Object.keys(res.instruments).length} instruments`);
}

async function depth(page) { return (await page.evaluate(() => JSON.parse(window.qp.open()))).depth; }

// Taps the harness needs from the feed to a depth (SPEC §G7, M18/M20 "dokunma sayısı"):
// tap the target face, then the open scene's centre, until the depth is reached (at most 4).
async function tapTo(page, id, want) {
  let taps = 0;
  const box = await page.locator(`#list .face[data-id="${id}"]`).boundingBox();
  await page.mouse.click(box.x + box.width / 2, box.y + Math.min(box.height / 2, 60));
  taps++;
  await settle(page);
  while ((await depth(page)) !== want && taps < 4) {
    const order = ['sentence', 'capsule', 'chart', 'evidence'];
    const d = await depth(page);
    if (order.indexOf(d) > order.indexOf(want)) break;
    if (want === 'evidence' && await page.locator('#toevidence').isVisible()) await page.click('#toevidence');
    else { const b = await page.locator('#scene').boundingBox(); await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2); }
    taps++;
    await settle(page);
  }
  return taps;
}

// Probe mode (SPEC §G2): the reader's feed ids at each step (the authority's own feed
// query, the same for every design), so that the state finder only picks capsules the
// page lists.
async function probe() {
  const { child, base } = await startDemo(plan.qp, plan.web, plan.port || 8461, plan.demo_args || []);
  const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
  const browser = await launch();
  const res = {};
  try {
    for (const principal of plan.principals || ['arkin']) {
      const { ctx, page } = await newPage(browser, {});
      await page.goto(`${origin}/?${query({ kind: 'feed', principal, theme: 'light' })}`);
      await settle(page);
      await setSens(page, plan.sensitivity);
      res[principal] = {};
      for (const st of plan.probe) {
        await seek(page, st);
        const f = JSON.parse(await page.evaluate(() => window.qp.feed())).filter((r) => r.id);
        const eng = await engine(page);
        res[principal][st] = { replay_t: eng.replay_t, replay_step: eng.replay_step,
          rows: f.map((r) => ({ id: r.id, text: r.text, item: eng.items[r.id] || null })) };
      }
      await ctx.close();
    }
  } finally {
    await browser.close();
    child.kill();
  }
  fs.writeFileSync(path.join(out, 'probe.json'), `${JSON.stringify(res)}\n`);
  console.log(`probe: ${plan.probe.length} steps`);
}

async function run() {
  const groups = new Map();
  for (const s of plan.shots) {
    const k = String(s.step);
    if (!groups.has(k)) groups.set(k, []);
    groups.get(k).push(s);
  }
  const { child, base } = await startDemo(plan.qp, plan.web, plan.port || 8461, plan.demo_args || []);
  const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
  const browser = await launch();
  const log = [];
  try {
    for (const [step, shots] of groups) {
      let seeked = false;
      for (const s of shots) {
        const { ctx, page, errors } = await newPage(browser, s);
        await page.goto(`${origin}/?${query(s)}`);
        await settle(page);
        // The reader's sensitivity (SPEC §G1.3: `all`, so that the fixed state set is in
        // every design's feed; the feed rule itself is M19's subject, not M16's).
        await setSens(page, s.sensitivity ?? plan.sensitivity);
        if (!seeked) { await seek(page, step); seeked = true; }
        await page.addStyleTag({ content: SCAFFOLD_CSS });
        await settle(page);
        const meta = { step: Number(step) };
        if (s.kind !== 'feed') {
          const { target, row } = await targetOf(page, s);
          if (!target && !(s.kind === 'chart' && s.open_direct)) { log.push({ id: s.id, error: 'no candidate in the feed' }); await ctx.close(); continue; }
          meta.target = target || s.candidates[0];
          meta.row_text = row ? row.text : null;
          if (target) await scrollTo(page, target);
          if (s.kind === 'evidence' || s.kind === 'chart') {
            if (target) meta.taps = await tapTo(page, target, s.kind);
            else {
              // (M18) the target is not in the reader's feed: opened through the page API
              // (the core's `open_capsule`, the path of a tap on its face), then one tap on
              // the scene; the taps are recorded as 1 + 1 and the record says so.
              await page.evaluate((id) => window.qp.openCapsule(id), meta.target);
              await settle(page);
              const b = await page.locator('#scene').boundingBox();
              await page.mouse.click(b.x + b.width / 2, b.y + b.height / 2);
              await settle(page);
              meta.taps = 2; meta.opened_direct = true;
            }
            meta.depth = await depth(page);
            if (s.kind === 'evidence') meta.evidence = await page.evaluate(() => (globalThis.__qpcore ? JSON.parse(globalThis.__qpcore.evidence()) : []));
            for (let i = 0; i < (s.wheel_out || 0); i++) {
              const b = await page.locator('#scene').boundingBox();
              await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
              await page.mouse.wheel(0, 240);
              await settle(page);
            }
            if (s.kind === 'chart' && s.zoom_to) meta.zoom = await zoomTo(page, s.zoom_to, s.story_rows || null);
            if (s.kind === 'chart') {
              meta.view = await page.evaluate(() => { try { return JSON.parse(window.qp.look()); } catch (e) { return null; } });
              meta.stories = await page.evaluate(() => { try { return JSON.parse(globalThis.__qpcore.stories()); } catch (e) { return null; } });
              const sb = await page.locator('#scene').boundingBox();
              meta.scene_rect = sb ? [sb.x, sb.y, sb.width, sb.height] : null;
            }
          }
          if (s.kind === 'motion') {
            // The opening transition from the feed; the shell keeps it (`lastMove`) and the
            // harness freezes it at the given progress values (`window.qp.freeze(p)`); the
            // frozen pieces' boxes (#fx canvases) give the M21 region key (SPEC §G4.6).
            const before = path.join(out, `${s.id}.before.png`);
            await page.screenshot({ path: before, animations: 'disabled', caret: 'hide' });
            meta.taps = await tapTo(page, target, s.to || 'capsule');
            await settle(page);
            meta.frames = [];
            for (const p of s.progress || [0.5]) {
              const n = await page.evaluate((x) => window.qp.freeze(x), p);
              const boxes = await page.evaluate(() => [...document.querySelectorAll('#fx canvas')].map((c) => {
                const r = c.getBoundingClientRect(); return [r.left, r.top, r.width, r.height, Number(getComputedStyle(c).opacity)];
              }));
              const f = path.join(out, `${s.id}.p${Math.round(p * 100)}.png`);
              await page.screenshot({ path: f, animations: 'disabled', caret: 'hide' });
              meta.frames.push({ p, elements: n, boxes, file: path.basename(f) });
              await page.evaluate(() => window.qp.unfreeze());
            }
            await settle(page);
          }
        }
        await shoot(page, s, meta);
        if (errors.length) log.push({ id: s.id, errors });
        await ctx.close();
      }
    }
  } finally {
    await browser.close();
    child.kill();
  }
  fs.writeFileSync(path.join(out, 'shoot-log.json'), `${JSON.stringify(log, null, 1)}\n`);
  console.log(`shoot: ${plan.shots.length} shots in ${out}; problems: ${log.length}`);
}

if (plan.chart_probe) await chartProbe(); else if (plan.probe) await probe(); else await run();
