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
// (R5/S) The `v1` shots (fixtures/golden/demo/*.png) are pinned to the designs before R5
// (`V1`: `?look=r4&voice=r4&face=vadrel&story=r4&open=r4&motion=r4&feed_order=layout`) and
// stay byte-identical;
// the `v2` shots (fixtures/golden/demo/v2/*.png, written under <out dir>/v2/) are the
// presentation as R5 opened it (R5's picks; `V2_PIN` names the one that has since changed,
// so the v2 goldens stay byte-identical).
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
fs.mkdirSync(path.join(out, 'v2'), { recursive: true });

// The R4/T5 shots are pinned to the R3 designs (`R3`): their goldens predate the R4
// presentation defaults (face `vadrel` (S2), feed `elikov`, transition `zumlak`, outcome
// `tg5` (S1)) and do not change. The `-r4` shots show the presentation as it opens now
// (no design setting in the URL); `-fotolay`: M8's second face.
const R3 = { face: 'r3', feed: 'r3', transition: 'r3', outcome: 'r3' };
// (R5/S) The designs the `v1` goldens were drawn with (before R5's presentation defaults).
// (R5/fix47) `feed_order: 'layout'`: the connection's layout orders the feed, as before the
// presentation's feed order (`live_new`).
// (RP4/T7) `clock: '0'`: the rows as before the row time (the presentation's `clock`).
const V1 = { look: 'r4', voice: 'r4', face: 'vadrel', story: 'r4', open: 'r4', motion: 'r4', feed_order: 'layout', clock: '0' };
// (R5/S) The `v2` shots: R5's presentation defaults in both languages and schemes — the
// feed's faces (open and closed), the open capsule (`tesir`) and the story chart (`pavka`).
// (R6/T0, K57) The presentation's story chart is now `korda`; the v2 goldens were drawn
// with `pavka`, so the v2 shots name it (`?story=pavka`) and stay byte-identical.
// (RP4/T7) The rows' time (`clock`) came after them: the v2 shots name it off.
const V2_PIN = { story: 'pavka', clock: '0' };
const V2 = [];
for (const lang of ['tr', 'en']) {
  for (const theme of ['dark', 'light']) {
    const k = `${lang}-${theme}`;
    V2.push({ v2: true, name: `feed-${k}`, lang, theme, canvases: ['face-0', 'face-1', 'face-2'] });
    V2.push({ v2: true, name: `closed-${k}`, lang, theme, canvases: ['closed-0', 'closed-1'] });
    V2.push({ v2: true, name: `capsule-${k}`, lang, theme, open: 'capsule', canvases: ['scene'] });
    V2.push({ v2: true, name: `chart-${k}`, lang, theme, open: 'chart', canvases: ['scene'] });
  }
}
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
  ...V2,
  // (RP4/T7) The presentation as it opens now (no design setting in the URL): the rows
  // carry the event's time (`clock`). New goldens (`p4-*`); the v1/v2 ones stay as they were.
  { present: true, name: 'p4-feed-tr-dark', canvases: ['face-0', 'face-1', 'face-2'] },
  { present: true, name: 'p4-feed-en-light', lang: 'en', theme: 'light', canvases: ['face-0', 'face-1'] },
  { present: true, name: 'p4-closed-tr-light', theme: 'light', canvases: ['closed-0'] },
  { present: true, name: 'p4-chart-tr-dark', open: 'chart', canvases: ['scene'] },
  { present: true, name: 'p4-chart-en-light', lang: 'en', theme: 'light', open: 'chart', canvases: ['scene'] },
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
    if (s.principal) q.set('principal', s.principal);
    // (R5/m17-int, R5/S) The v1 goldens predate R5's designs: they are drawn with the
    // designs before it (`V1`, today's text `?voice=r4`); the v2 shots name none.
    // (RP4/T7) `present`: the presentation's own defaults, nothing pinned.
    if (s.present) { /* as the page opens */ } else if (!s.v2) for (const [k, v] of Object.entries(V1)) q.set(k, v);
    else for (const [k, v] of Object.entries(V2_PIN)) q.set(k, v);
    if (s.face) q.set('face', s.face);
    for (const [k, v] of Object.entries(s.look || {})) q.set(k, v);
    await page.goto(`${origin}/?${q}`);
    await settle(page);
    if (!seeked) {
      await page.fill('#c_date', at);
      await page.click('#c_go');
      await settle(page);
      seeked = true;
    }
    if (s.open && (s.v2 || s.present)) {
      // (R5/S) Through the core's own handles (a tap's place depends on the design).
      const first = await page.evaluate(() => JSON.parse(window.qp.feed()).filter((r) => r.id && r.scene)[0]);
      await page.evaluate((id) => window.qp.openCapsule(id), first.id);
      await settle(page);
      if (s.open === 'chart') {
        await page.evaluate(() => window.qp.openTo('chart'));
        await settle(page);
      }
    } else if (s.open) {
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
    const dir = s.v2 ? path.join(out, 'v2') : out;
    await page.locator('#phone').screenshot({ path: path.join(dir, `${s.name}.page.png`), animations: 'disabled', caret: 'hide' });
    // Capsule rows only (a feed layout may add group headers); `closed-<i>`: the i-th
    // capsule whose story is closed (its outcome mark shows; the row's `closed`).
    const feed = (await page.evaluate(() => JSON.parse(window.qp.feed()))).filter((r) => r.id);
    const closed = feed.filter((r) => r.closed && r.scene).map((r) => r.id);
    for (const c of s.canvases) {
      const [kind, i] = c.split('-');
      const id = c === 'scene' ? 'scene' : kind === 'closed' ? closed[Number(i)] : feed[Number(i)].id;
      if (!id) throw new Error(`${s.name}: no ${c}`);
      const bytes = await page.evaluate((i) => window.qp.png(i), id);
      fs.writeFileSync(path.join(dir, `${s.name}.${c}.png`), Buffer.from(bytes));
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
