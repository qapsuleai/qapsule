// R4/T6 startup run (GOAL 10): opens the demo page `runs` times, each in a fresh browser
// context (empty cache, like a first visit), and prints the opening timeline's medians:
// page open → WASM ready → connected (`welcome`) → first capsule painted, plus the wasm
// download and the start frame. Same harness as tests/metrics.mjs.
// Usage: node tests/startup.mjs <qp> [cpu throttling rate] [runs] [extra qp demo args…]
import { startDemo, launch } from './lib.mjs';

const [qp, rate = '1', runs = '7', ...extra] = process.argv.slice(2);
const here = new URL('..', import.meta.url).pathname;
const { child, base } = await startDemo(qp, here, 8397, extra);
const browser = await launch();
const rows = [];
try {
  for (let k = 0; k < Number(runs); k++) {
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 1 });
    const page = await ctx.newPage();
    const cdp = await ctx.newCDPSession(page);
    if (rate !== '1') await cdp.send('Emulation.setCPUThrottlingRate', { rate: Number(rate) });
    await page.goto(`${base.replace('localhost', '127.0.0.1')}?anim=0`);
    await page.waitForFunction(() => {
      if (!window.qp || !window.qp.metrics) return false;
      try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; }
    }, null, { timeout: 120000, polling: 20 });
    const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    const res = await page.evaluate(() => performance.getEntriesByType('resource')
      .filter((e) => e.name.endsWith('.wasm'))
      .map((e) => ({ start: e.startTime, end: e.responseEnd, bytes: e.encodedBodySize })));
    rows.push({ ...m.open_ms, wasm_fetch_end: res.length ? res[0].responseEnd || res[0].end : null, wasm_bytes: res.length ? res[0].bytes : null });
    await ctx.close();
  }
} finally {
  await browser.close();
  child.kill();
}
const keys = [...new Set(rows.flatMap((r) => Object.keys(r)))];
const med = {};
for (const key of keys) {
  const v = rows.map((r) => r[key]).filter((x) => typeof x === 'number').sort((a, b) => a - b);
  if (v.length) med[key] = { median: v[Math.floor(v.length / 2)], min: v[0], max: v[v.length - 1] };
}
console.log(JSON.stringify({ cpu_throttling: Number(rate), runs: rows.length, extra, median: med }, null, 1));
