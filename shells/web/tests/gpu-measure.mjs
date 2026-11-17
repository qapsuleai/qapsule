// RP3/M12c measurement driver: the automatic comparison (metrics page "Karşılaştır",
// RP3/T2) with the paths this browser supports (`ref`, `gpu`; no fixture), phone viewport
// 402×874 at DPR 3, in full Chromium (Metal WebGPU; optional CPU throttling through CDP)
// or WebKit. Writes the comparison summary as JSON.
//
// Usage: node tests/gpu-measure.mjs <qp> <out.json> [--browser chromium|webkit|swiftshader]
//          [--throttle 4] [--query gpu_direct=1] [--port 8462] [--phase <ms>]
// Note: CPU throttling slows the page's JavaScript and WASM, not the GPU; GPU numbers
// under throttling are not a phone's GPU.
import fs from 'node:fs';
import { chromium, webkit } from 'playwright';
import { startDemo, settle } from './lib.mjs';

const argv = process.argv.slice(2);
const [qp, outFile] = argv;
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const name = opt('--browser', 'chromium');
// --nojit: JavaScriptCore without JIT (T1's calibrated `PHONE`: options reach WebKit's
// WebContent process only as __XPC_JSC_* variables, rp3/t1-calib calibrate.mjs).
const nojit = argv.includes('--nojit');
const throttle = Number(opt('--throttle', '1'));
const query = opt('--query', '');
const port = Number(opt('--port', '8462'));
const phase = opt('--phase', '');
const here = new URL('..', import.meta.url).pathname;

const { child, base: b } = await startDemo(qp, here, port);
const base = b.replace('localhost', '127.0.0.1');
// `swiftshader`: Chromium's headless shell with WebGPU on its software adapter
// (SwiftShader, CPU emulation; numbers are not a GPU's).
const browser = name === 'webkit' ? await webkit.launch(nojit ? { env: { ...process.env, __XPC_JSC_useJIT: 'false' } } : {})
  : name === 'swiftshader' ? await chromium.launch({ args: ['--enable-unsafe-webgpu'] })
    : await chromium.launch({ channel: 'chromium' });
let code = 0;
try {
  const ctx = await browser.newContext({ viewport: { width: 402, height: 874 }, deviceScaleFactor: 3 });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  if (throttle > 1 && name !== 'webkit') {
    const cdp = await ctx.newCDPSession(page);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate: throttle });
  }
  const q = ['anim=0', query, phase ? `bench_phase=${phase}` : ''].filter(Boolean).join('&');
  await page.goto(`${base}?${q}`);
  await settle(page, 120000);
  // Let a GPU device come up before the run (the comparison switches paths itself).
  await page.waitForTimeout(1500);
  await page.click('#tabs button[data-tab="metrics"]');
  const t0 = Date.now();
  await page.click('#b_start');
  await page.waitForFunction(() => !window.qp.benching() && JSON.parse(window.qp.bench()) !== null, null, { timeout: 400000, polling: 1000 });
  const s = JSON.parse(await page.evaluate(() => window.qp.bench()));
  const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
  fs.writeFileSync(outFile, JSON.stringify({ browser: name, version: browser.version(), throttle, nojit, query, ms: Date.now() - t0, errors, summary: s, metrics: m }, null, 1));
  console.log(`${name}${nojit ? ' nojit' : ''} ×${throttle} ${query}: ${s.paths.map((p) => p.name)} in ${Date.now() - t0} ms, errors ${errors.length}`);
} catch (e) {
  console.error(String(e));
  code = 1;
} finally {
  await browser.close();
  child.kill();
}
process.exit(code);
