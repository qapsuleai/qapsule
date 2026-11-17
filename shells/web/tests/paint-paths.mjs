// RP3/S: every paint path opens with its URL parameter and paints (VERIFY 5, criterion 4),
// the default path is `canvas`, and the semantic reads do not depend on the path (VERIFY 8).
//
// For each browser and each path of PATHS a fresh `qp demo` (synthetic, paused) is opened
// with `?paint=<path>` at a phone viewport; the replay goes to step 120, the first capsule
// is opened (a tap on its face) to its chart (a tap on the scene), then the chart is tapped
// on a grid of points. Checked:
//  - the reported path (`metrics().paint`): the path itself, or — `gpu` only — the reported
//    fallback `unsupported` where the browser has no WebGPU adapter (Chromium's headless
//    shell); the first face and the open scene were painted (canvases not blank);
//  - the semantic reads are byte-identical to `ref`'s: the page's digests (document, faces,
//    open scene, view state), its feed and open capsule; the authority's answers on the
//    page's own connection — every `tap_at` answer (the point query of the tapped scene,
//    `hit`, and the interaction it caused) and a `screen` query sent at the end (`seq`
//    and the request `id` are left out: they count the page's other messages, whose number
//    depends on timing);
//  - without `?paint=` the page uses `canvas` (the default), reported as such, with the
//    same semantic reads.
// No page errors anywhere. The page's stream is recorded by an init script of this test
// (the shell is unchanged).
//
// Usage: node tests/paint-paths.mjs <qp> [--browsers chromium,webkit] [--dpr 1]
//          [--paths ref,canvas,cpu,gpu] [--port 8491] [--out <dir>]
import fs from 'node:fs';
import path from 'node:path';
import { chromium, webkit } from 'playwright';
import { startDemo, settle, clickIn } from './lib.mjs';

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const browsers = opt('--browsers', 'chromium,webkit').split(',');
const dpr = Number(opt('--dpr', '1'));
const PATHS = opt('--paths', 'ref,canvas,cpu,gpu').split(',');
const port = Number(opt('--port', '8491'));
const out = opt('--out', null);
const here = new URL('..', import.meta.url).pathname;
const ENGINES = { chromium, webkit };
const STEP = '120';
const TAPS = [[40, 60], [120, 150], [195, 100], [280, 200], [350, 250], [195, 150]];

let failed = 0;
function check(cond, msg) {
  if (!cond) {
    console.error(`paint-paths: ${msg}`);
    failed++;
  }
  return cond;
}

// Records the authority's answers on the page's own stream: `interaction` (tap_at, open,
// close, drag …) and `screen` results, without `seq`/`id`; and the connection's secret.
function recorder() {
  const ES = window.EventSource;
  window.__qp = { frames: [], conn: null };
  window.EventSource = class extends ES {
    constructor(url, o) {
      super(url, o);
      this.addEventListener('message', (e) => {
        const f = JSON.parse(e.data);
        if (f.start) window.__qp.conn = { conn: f.start.conn, token: f.start.token };
        const m = f.msg;
        if (m && m.type === 'result' && m.data && (m.data.kind === 'interaction' || m.data.kind === 'screen')) {
          const { seq, id, ...rest } = m;
          window.__qp.frames.push(rest);
        }
      });
    }
  };
}

// Pixels of a page canvas are not all the same (something was painted).
async function painted(page, sel) {
  return page.evaluate((s) => {
    const c = document.querySelector(s);
    if (!c || !c.width || !c.height) return false;
    const g = c.getContext('2d');
    if (!g) return false;
    const d = g.getImageData(0, 0, c.width, c.height).data;
    const first = d[0] * 65536 + d[1] * 256 + d[2];
    for (let i = 4; i < d.length; i += 4 * 97) if (d[i] * 65536 + d[i + 1] * 256 + d[i + 2] !== first) return true;
    return false;
  }, sel);
}

async function run(browser, paint) {
  const { child, base } = await startDemo(qp, here, port);
  const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: dpr });
  await ctx.addInitScript(recorder);
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  try {
    await page.goto(`${origin}/?anim=0${paint ? `&paint=${paint}` : ''}`);
    await settle(page);
    await page.fill('#c_date', STEP);
    await page.click('#c_go');
    await settle(page);
    const feed = JSON.parse(await page.evaluate(() => window.qp.feed())).filter((r) => r.id && r.scene);
    const faceOk = await painted(page, `#list .face[data-id="${feed[0].id}"] canvas`);
    await clickIn(page, `#list .face[data-id="${feed[0].id}"] canvas`, 120, 60);
    await settle(page);
    await clickIn(page, '#scene', 195, 150);
    await settle(page);
    const sceneOk = await painted(page, '#scene');
    for (const [x, y] of TAPS) {
      await clickIn(page, '#scene', x, y);
      await settle(page);
    }
    // A `screen` query on the page's own connection (its answer is recorded above).
    await page.evaluate(async () => {
      const { conn, token } = window.__qp.conn;
      const n = window.__qp.frames.length;
      await fetch(`/send?conn=${encodeURIComponent(conn)}`, {
        method: 'POST',
        body: JSON.stringify([{ type: 'query', id: 'paint-paths-screen', query: { kind: 'screen' } }]),
        headers: { 'Content-Type': 'application/json', 'X-Qp-Token': token },
      });
      for (let i = 0; i < 200 && window.__qp.frames.length === n; i++) await new Promise((r) => setTimeout(r, 25));
    });
    await settle(page);
    const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    const frames = await page.evaluate(() => window.__qp.frames);
    const reads = {
      digests: JSON.parse(await page.evaluate(() => window.qp.digests())),
      feed: await page.evaluate(() => window.qp.feed()),
      open: await page.evaluate(() => window.qp.open()),
      answers: frames,
    };
    return { paint: m.paint, faceOk, sceneOk, reads, errors };
  } finally {
    await ctx.close();
    child.kill();
  }
}

const kinds = (r) => {
  const n = {};
  for (const f of r.reads.answers) n[f.data.kind] = (n[f.data.kind] || 0) + 1;
  return n;
};

for (const name of browsers) {
  const browser = name === 'chromium'
    ? await chromium.launch({ args: ['--force-color-profile=srgb', '--font-render-hinting=none'] })
    : await ENGINES[name].launch();
  try {
    const res = {};
    for (const p of PATHS) {
      const r = await run(browser, p);
      res[p] = r;
      const fellBack = p === 'gpu' && r.paint.used === 'ref' && r.paint.fallback === 'unsupported';
      const k = kinds(r);
      check(r.paint.requested === p, `${name}/${p}: requested ${JSON.stringify(r.paint)}`);
      check((r.paint.used === p && r.paint.fallback === null) || fellBack, `${name}/${p}: paint ${JSON.stringify(r.paint)}`);
      check(r.faceOk, `${name}/${p}: the first face is blank`);
      check(r.sceneOk, `${name}/${p}: the open scene is blank`);
      check(r.errors.length === 0, `${name}/${p}: page errors ${r.errors}`);
      check(k.screen === 1 && k.interaction >= TAPS.length + 2, `${name}/${p}: answers ${JSON.stringify(k)}`);
      console.log(`${name}/${p}: paint ${JSON.stringify(r.paint)}${fellBack ? ' (WebGPU unavailable: the reported fallback)' : ''}; face ${r.faceOk ? 'painted' : 'BLANK'}, scene ${r.sceneOk ? 'painted' : 'BLANK'}; answers ${JSON.stringify(k)}`);
    }
    if (res.ref) {
      const want = JSON.stringify(res.ref.reads);
      for (const p of PATHS) {
        if (p === 'ref') continue;
        const same = JSON.stringify(res[p].reads) === want;
        check(same, `${name}/${p}: semantic reads differ from ref`);
        console.log(`${name}/${p} = ref: digests, feed, open, ${res[p].reads.answers.length} authority answers (tap_at/hit, screen) ${same ? 'byte-identical' : 'DIFFERENT'}`);
      }
    }
    const d = await run(browser, '');
    check(d.paint.requested === 'canvas' && d.paint.used === 'canvas' && d.paint.fallback === null, `${name}/default: paint ${JSON.stringify(d.paint)}`);
    check(d.faceOk && d.sceneOk && d.errors.length === 0, `${name}/default: painted ${d.faceOk}/${d.sceneOk}, errors ${d.errors}`);
    if (res.ref) check(JSON.stringify(d.reads) === JSON.stringify(res.ref.reads), `${name}/default: semantic reads differ from ref`);
    console.log(`${name}/default: paint ${JSON.stringify(d.paint)}`);
    if (out) {
      fs.mkdirSync(out, { recursive: true });
      fs.writeFileSync(path.join(out, `paint-paths-${name}.json`), JSON.stringify(res.ref ? res.ref.reads : d.reads, null, 1));
    }
  } finally {
    await browser.close();
  }
}
console.log(failed ? `paint-paths: ${failed} failure(s)` : 'paint-paths: ok');
process.exit(failed ? 1 : 0);
