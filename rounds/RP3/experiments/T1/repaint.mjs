// RP3/T1: repaint frequency with a chart open — scene paints per data tick for each of
// the first capsules of the feed (which capsule is open decides how often its scene
// changes). Chromium, DPR 1 (only counts matter). Usage (repo root):
//   node rounds/RP3/experiments/T1/repaint.mjs target/release/qp [seek=700] [n=8] [ms=6000]
import { createRequire } from 'node:module';
import { startDemo, settle, clickIn } from '../../../../shells/web/tests/lib.mjs';

const require = createRequire(new URL('../../../../shells/web/package.json', import.meta.url));
const { chromium } = require('playwright');
const [qp, seek = '700', n = '8', ms = '6000'] = process.argv.slice(2);
const web = new URL('web-base/', import.meta.url).pathname;
const { child, base } = await startDemo(qp, web, 8412);
const browser = await chromium.launch();
const out = [];
try {
  const page = await (await browser.newContext({ viewport: { width: 402, height: 874 } })).newPage();
  await page.goto(`${base.replace('localhost', '127.0.0.1')}?x=1`);
  await settle(page, 60000);
  await page.evaluate((s) => { document.getElementById('c_date').value = s; document.getElementById('c_go').click(); }, seek);
  await page.waitForFunction((s) => Number(JSON.parse(window.qp.status()).step.split(' ')[0]) >= Number(s), seek, { timeout: 60000 });
  await settle(page, 60000);
  const caps = await page.evaluate(() => JSON.parse(window.qp.feed()).filter((i) => i.id && i.scene).map((i) => ({ id: i.id, closed: i.closed })));
  const play = () => page.evaluate(() => document.getElementById('c_play').click());
  for (const c of caps.slice(0, Number(n))) {
    await page.evaluate((id) => window.qp.openCapsule(id), c.id);
    await settle(page, 60000);
    await clickIn(page, '#scene', 195, 150);
    await settle(page, 60000);
    await page.waitForTimeout(600);
    const depth = JSON.parse(await page.evaluate(() => window.qp.open())).depth;
    await play();
    await page.evaluate(() => window.qp.resetMetrics());
    await page.waitForTimeout(Number(ms));
    const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    await play();
    await settle(page, 60000);
    const step = JSON.parse(await page.evaluate(() => window.qp.status())).step;
    const ticks = m.span_ms.tick ? m.span_ms.tick.n : 0;
    const scenes = m.span_ms.scene ? m.span_ms.scene.n : 0;
    out.push({ ...c, depth, step, ticks, scenes, per_tick: ticks ? scenes / ticks : null });
    console.error(JSON.stringify(out.at(-1)));
    await page.evaluate(() => document.getElementById('back').click());
    await settle(page, 60000);
    await page.evaluate(() => document.getElementById('back').click());
    await settle(page, 60000);
  }
} finally {
  await browser.close();
  child.kill();
}
console.log(JSON.stringify(out, null, 1));
