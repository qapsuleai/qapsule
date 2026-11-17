// R4/T5 screenshot harness (GOAL 11; M8–M11 use it too): headless Chromium on `qp demo`,
// at a fixed moment of the replay and with fixed settings (language, theme, face,
// alphabet, open capsule), animations off.
//
//   node shells/web/screens.mjs <qp> <out dir> [--at <step|YYYY-MM-DD>] [--data <bars dir>]
//                               [--only <shot name>] [--paint ref]
//   --paint  (RP3/S) the paint path (`?paint=`, default `ref`: the deterministic reference
//            renderer whose canvases equal fixtures/golden/demo byte for byte; the page's
//            own default path, `canvas`, is compared with tolerance by tests/paint-canvas.mjs)
//
// For every shot it writes
//   <shot>.page.png         the phone screen (browser screenshot: chrome + scenes)
//   <shot>.<canvas>.png     each scene the core painted (face-0, face-1, …, scene), PNG
//                           bytes from the core's encoder (identical on every platform)
// Synthetic data by default (the goldens in fixtures/golden/demo are the canvas PNGs of
// the synthetic run at step 120). With --data (real local data) the output must be under
// fixtures/local/: real data and its images never enter the repository.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { startDemo, phonePage, settle, launch, clickIn } from './tests/lib.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const argv = process.argv.slice(2);
const [qp, outArg] = argv;
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const at = opt('--at', '120');
const data = opt('--data', null);
const only = opt('--only', null);
const paintPath = opt('--paint', 'ref');
const out = path.resolve(outArg);
if (data && !out.startsWith(path.join(root, 'fixtures', 'local'))) {
  console.error('screens: with --data the output must be under fixtures/local/ (real data stays local)');
  process.exit(2);
}
fs.mkdirSync(out, { recursive: true });

// The R4/T5 shots are pinned to the R3 designs (`R3`): their goldens predate the R4
// presentation defaults (face `vadrel` (S2), feed `elikov`, transition `zumlak`, outcome
// `tg5` (S1)) and do not change. The `-r4` shots show the presentation as it opens now
// (no design setting in the URL); `-fotolay`: M8's second face.
const R3 = { face: 'r3', feed: 'r3', transition: 'r3', outcome: 'r3' };
export const SHOTS = [
  { name: 'feed-tr-dark', look: R3, canvases: ['face-0', 'face-1', 'face-2'] },
  { name: 'feed-en-light', look: R3, lang: 'en', theme: 'light', canvases: ['face-0'] },
  { name: 'capsule-tr-dark', look: R3, open: 'capsule', canvases: ['scene'] },
  { name: 'chart-tr-dark', look: R3, open: 'chart', canvases: ['scene'] },
  { name: 'chart-en-light-rozet', look: R3, lang: 'en', theme: 'light', alphabet: 'rozet', open: 'chart', canvases: ['scene'] },
  { name: 'feed-visitor', look: R3, principal: 'visitor', canvases: [] },
  // R4/S1: the presentation's defaults.
  { name: 'feed-tr-dark-r4', canvases: ['face-0', 'face-1', 'face-2'] },
  { name: 'feed-tr-light-r4-closed', theme: 'light', canvases: ['closed-0', 'closed-1'] },
  // R4/S2: the presentation's face (`vadrel`) in the other language and theme, and M8's
  // second face (`fotolay`, the human test's Y2).
  { name: 'feed-en-light-r4', lang: 'en', theme: 'light', canvases: ['face-0', 'face-1'] },
  { name: 'feed-tr-dark-fotolay', face: 'fotolay', canvases: ['face-0', 'face-1'] },
];

const extra = data ? ['--data', data] : [];
const { child, base } = await startDemo(qp, here, 8398, extra);
const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
const browser = await launch();
let n = 0;
try {
  let seeked = false;
  for (const s of SHOTS) {
    if (only && s.name !== only) continue;
    const { ctx, page, errors } = await phonePage(browser);
    const q = new URLSearchParams({ anim: '0', paint: paintPath, lang: s.lang || 'tr', theme: s.theme || 'dark' });
    if (s.alphabet) q.set('alphabet', s.alphabet);
    if (s.face) q.set('face', s.face);
    if (s.principal) q.set('principal', s.principal);
    for (const [k, v] of Object.entries(s.look || {})) q.set(k, v);
    await page.goto(`${origin}/?${q}`);
    await settle(page);
    if (!seeked) {
      await page.fill('#c_date', at);
      await page.click('#c_go');
      await settle(page);
      seeked = true;
    }
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
    await page.locator('#phone').screenshot({ path: path.join(out, `${s.name}.page.png`), animations: 'disabled', caret: 'hide' });
    // Capsule rows only (a feed layout may add group headers); `closed-<i>`: the i-th
    // capsule whose story is closed (its outcome mark shows; the row's `closed`).
    const feed = (await page.evaluate(() => JSON.parse(window.qp.feed()))).filter((r) => r.id);
    const closed = feed.filter((r) => r.closed && r.scene).map((r) => r.id);
    for (const c of s.canvases) {
      const [kind, i] = c.split('-');
      const id = c === 'scene' ? 'scene' : kind === 'closed' ? closed[Number(i)] : feed[Number(i)].id;
      if (!id) throw new Error(`${s.name}: no ${c}`);
      const bytes = await page.evaluate((i) => window.qp.png(i), id);
      fs.writeFileSync(path.join(out, `${s.name}.${c}.png`), Buffer.from(bytes));
      n++;
    }
    if (errors.length) throw new Error(`${s.name}: page errors ${errors}`);
    await ctx.close();
  }
} finally {
  await browser.close();
  child.kill();
}
console.log(`screens: ${n} scene images and page screenshots in ${out}`);
