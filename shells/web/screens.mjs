// R4/T5 screenshot harness (GOAL 11; M8–M11 use it too): headless Chromium on `qp demo`,
// at a fixed moment of the replay and with fixed settings (language, theme, face,
// alphabet, open capsule), animations off.
//
//   node shells/web/screens.mjs <qp> <out dir> [--at <step|YYYY-MM-DD>] [--data <bars dir>]
//                               [--only <shot name>]
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
const out = path.resolve(outArg);
if (data && !out.startsWith(path.join(root, 'fixtures', 'local'))) {
  console.error('screens: with --data the output must be under fixtures/local/ (real data stays local)');
  process.exit(2);
}
fs.mkdirSync(out, { recursive: true });

export const SHOTS = [
  { name: 'feed-tr-dark', canvases: ['face-0', 'face-1', 'face-2'] },
  { name: 'feed-en-light', lang: 'en', theme: 'light', canvases: ['face-0'] },
  { name: 'capsule-tr-dark', open: 'capsule', canvases: ['scene'] },
  { name: 'chart-tr-dark', open: 'chart', canvases: ['scene'] },
  { name: 'chart-en-light-rozet', lang: 'en', theme: 'light', alphabet: 'rozet', open: 'chart', canvases: ['scene'] },
  { name: 'feed-visitor', principal: 'visitor', canvases: [] },
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
    const q = new URLSearchParams({ anim: '0', lang: s.lang || 'tr', theme: s.theme || 'dark' });
    if (s.alphabet) q.set('alphabet', s.alphabet);
    if (s.face) q.set('face', s.face);
    if (s.principal) q.set('principal', s.principal);
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
    const feed = await page.evaluate(() => JSON.parse(window.qp.feed()));
    for (const c of s.canvases) {
      const id = c === 'scene' ? 'scene' : feed[Number(c.split('-')[1])].id;
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
