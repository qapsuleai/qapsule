// R4/T5 metrics run (GOAL 9): opens the demo in headless Chromium, lets the replay run,
// scrolls the feed, opens a chart and drags it, then prints the metrics page's summary.
// Usage: node tests/metrics.mjs <qp> [cpu throttling rate, e.g. 4] [extra qp demo args…]
import { startDemo, phonePage, settle, launch, clickIn } from './lib.mjs';

const [qp, rate = '1', ...extra] = process.argv.slice(2);
const here = new URL('..', import.meta.url).pathname;
const { child, base } = await startDemo(qp, here, 8396, extra);
const browser = await launch();
try {
  const { page, ctx } = await phonePage(browser);
  const cdp = await ctx.newCDPSession(page);
  if (rate !== '1') await cdp.send('Emulation.setCPUThrottlingRate', { rate: Number(rate) });
  await page.goto(`${base.replace('localhost', '127.0.0.1')}?anim=0`);
  await settle(page);
  await page.selectOption('#c_speed', '{"kind":"speed","steps_per_s":16}');
  await page.click('#c_play');
  await page.waitForTimeout(6000);
  for (let i = 0; i < 30; i++) { await page.mouse.move(195, 400); await page.mouse.wheel(0, 120); await page.waitForTimeout(30); }
  await page.waitForTimeout(500);
  await page.locator('#feed').evaluate((e) => { e.scrollTop = 0; });
  await page.click('#c_play');
  await settle(page);
  const first = await page.evaluate(() => JSON.parse(window.qp.feed())[0]);
  await clickIn(page, `#list .face[data-id="${first.id}"] canvas`, 120, 60); await settle(page);
  await clickIn(page, '#scene', 195, 150); await settle(page);
  const b = await page.locator('#scene').boundingBox();
  await page.mouse.move(b.x + 250, b.y + 150);
  for (let i = 0; i < 3; i++) { await page.mouse.wheel(0, -100); await settle(page); }
  for (let k = 0; k < 6; k++) {
    await page.mouse.move(b.x + 300, b.y + 150); await page.mouse.down();
    for (let i = 0; i < 20; i++) await page.mouse.move(b.x + 300 - i * 10, b.y + 150);
    await page.mouse.up();
    for (let i = 0; i < 20; i++) await page.mouse.move(b.x + 100 + i * 10, b.y + 150, { steps: 1 });
  }
  await settle(page);
  console.log(JSON.stringify({ cpu_throttling: Number(rate), ...JSON.parse(await page.evaluate(() => window.qp.metrics())) }, null, 1));
} finally {
  await browser.close();
  child.kill();
}
