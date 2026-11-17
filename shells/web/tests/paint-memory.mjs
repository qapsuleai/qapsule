// RP3/M12b: memory of one paint path on a fresh page (the comparison runs paths one after
// the other on one page, so its memory figures depend on the order; T2). For each path:
// a new page at DPR 3, 402×874; replay from step 120 at 4 steps/s with the feed visible
// for `--ms`, then the first capsule's chart open for `--ms`; then the core's memory
// figures (WASM memory, JS heap: Chromium with --enable-precise-memory-info).
// Usage: node tests/paint-memory.mjs <qp> [--paths ref,canvas] [--browser chromium]
//          [--ms 15000] [--port 8455]
import { chromium, webkit } from 'playwright';
import { startDemo, settle, clickIn } from './lib.mjs';

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const paths = opt('--paths', 'ref,canvas').split(',');
const name = opt('--browser', 'chromium');
const ms = Number(opt('--ms', '15000'));
const here = new URL('..', import.meta.url).pathname;

const { child, base: b } = await startDemo(qp, here, Number(opt('--port', '8455')));
const base = new URL(b).origin.replace('localhost', '127.0.0.1');
const engine = name === 'webkit' ? webkit : chromium;
const browser = await engine.launch(name === 'chromium' ? { args: ['--enable-precise-memory-info'] } : {});
try {
  for (const p of paths) {
    const ctx = await browser.newContext({ viewport: { width: 402, height: 874 }, deviceScaleFactor: 3 });
    const page = await ctx.newPage();
    await page.goto(`${base}/?anim=0&paint=${p}`);
    await settle(page);
    await page.fill('#c_date', '120');
    await page.click('#c_go');
    await settle(page);
    await page.click('#c_play');
    await page.waitForTimeout(ms);
    const first = await page.evaluate(() => JSON.parse(window.qp.feed()).find((r) => r.id).id);
    await clickIn(page, `#list .face[data-id="${first}"] canvas`, 120, 60);
    await page.waitForTimeout(500);
    await clickIn(page, '#scene', 195, 150);
    await page.waitForTimeout(ms);
    await page.click('#c_play');
    const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    console.log(JSON.stringify({ browser: name, path: m.paint.used, memory: m.memory_bytes, scene_n: m.span_ms && m.span_ms.scene && m.span_ms.scene.n }));
    await ctx.close();
  }
} finally {
  await browser.close();
  child.kill();
}
