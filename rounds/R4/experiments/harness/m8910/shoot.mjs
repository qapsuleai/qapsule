// R4/H-M8910: screenshots of static sheet pages (M8) in headless Chromium.
//   node shoot.mjs <manifest.json>
// manifest: [{html, out, gray?}] — each page at the phone viewport (390×844 CSS px, DPR 2),
// the whole #phone element (taller than the viewport when the sheet is), animations off.
// Same page, same machine → byte-identical PNG.
import fs from 'node:fs';
import { launch } from '../../../../../shells/web/tests/lib.mjs';

const shots = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const browser = await launch();
try {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2 });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  for (const s of shots) {
    await page.goto(`file://${s.html}`);
    await page.waitForLoadState('load');
    await page.evaluate(() => Promise.all(Array.from(document.images).map((i) => i.decode().catch(() => {}))));
    await page.locator('#phone').screenshot({ path: s.out, animations: 'disabled', caret: 'hide' });
  }
  if (errors.length) throw new Error(`page errors: ${errors}`);
} finally {
  await browser.close();
}
console.log(`shoot: ${shots.length} screenshots`);
