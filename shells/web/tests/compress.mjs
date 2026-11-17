// RP3/M13: the demo page loads from the compressed package. Opens the page in headless
// Chromium and WebKit (fresh context each) and checks: the first capsule is painted; the
// WASM is requested exactly once (no second download from a preload WebKit does not reuse);
// it arrives compressed (`Content-Encoding` br or gzip, as the browser asked), as
// `application/wasm` (the streaming compile path; no fallback warning), with
// `Vary: Accept-Encoding`, and its body on the wire is the pre-compressed file's size.
// Then a reload in Chromium revalidates the package (304, no body).
// The pre-compressed files are made by scripts/demo.sh; when they are missing or older than
// the package this runs `QP_DEMO_PACK_ONLY=1 scripts/demo.sh` first.
// Usage: node tests/compress.mjs <qp>   (exit code 1 on any failure)
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import { chromium, webkit } from 'playwright';
import { startDemo } from './lib.mjs';

const [qp] = process.argv.slice(2);
const here = new URL('..', import.meta.url).pathname;
const pkg = `${here}pkg`;
const wasm = `${pkg}/qp_wasm_bg.wasm`;
const fresh = (f) => fs.existsSync(f) && fs.statSync(f).mtimeMs >= fs.statSync(wasm).mtimeMs;
if (!fresh(`${wasm}.gz`)) {
  execFileSync(`${here}../../scripts/demo.sh`, [], {
    env: { ...process.env, QP_DEMO_PACK_ONLY: '1' }, stdio: ['ignore', 'inherit', 'inherit'],
  });
}
const size = { gzip: fs.statSync(`${wasm}.gz`).size, br: fresh(`${wasm}.br`) ? fs.statSync(`${wasm}.br`).size : null };

const fails = [];
const check = (ok, what) => { if (!ok) fails.push(what); };
const { child, base } = await startDemo(qp, here, 8476, []);
const url = `${base.replace('localhost', '127.0.0.1')}?anim=0`;
const firstCapsule = () => {
  try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; }
};
try {
  for (const [name, bt] of [['chromium', chromium], ['webkit', webkit]]) {
    const browser = await bt.launch();
    const ctx = await browser.newContext({ viewport: { width: 390, height: 844 } });
    const page = await ctx.newPage();
    const logs = [];
    page.on('console', (m) => logs.push(m.text()));
    let wasmRes = [];
    page.on('requestfinished', (req) => {
      if (req.url().endsWith('.wasm')) wasmRes.push((async () => {
        const res = await req.response();
        return { ask: (await req.allHeaders())['accept-encoding'] || '', h: await res.allHeaders(), status: res.status(), s: await req.sizes() };
      })());
    });
    await page.goto(url);
    await page.waitForFunction(firstCapsule, null, { timeout: 120000, polling: 50 });
    await page.waitForTimeout(300);
    const got = await Promise.all(wasmRes);
    check(got.length === 1, `${name}: ${got.length} wasm requests (want 1)`);
    const r = got[0];
    if (r) {
      const want = /\bbr\b/.test(r.ask) && size.br ? 'br' : 'gzip';
      const enc = r.h['content-encoding'];
      check(enc === want, `${name}: Content-Encoding ${enc} for Accept-Encoding "${r.ask}" (want ${want})`);
      check(r.h['content-type'] === 'application/wasm', `${name}: Content-Type ${r.h['content-type']}`);
      check(/accept-encoding/i.test(r.h.vary || ''), `${name}: no Vary: Accept-Encoding`);
      check(r.s.responseBodySize === size[want], `${name}: wasm body ${r.s.responseBodySize} B on the wire, pre-compressed file ${size[want]} B`);
      console.log(`${name}: wasm ${enc}, ${r.s.responseBodySize} B on the wire (raw ${fs.statSync(wasm).size} B), asked "${r.ask}"`);
    }
    check(!logs.some((l) => /instantiateStreaming/.test(l)), `${name}: streaming compile fell back: ${logs.join(' | ')}`);
    if (name === 'chromium') {
      wasmRes = [];
      await page.reload();
      await page.waitForFunction(firstCapsule, null, { timeout: 120000, polling: 50 });
      const again = await Promise.all(wasmRes);
      const body = again.reduce((a, x) => a + x.s.responseBodySize, 0);
      check(again.every((x) => x.status === 304 || x.status === 200) && body === 0, `${name}: reload moved ${body} B of wasm (want a 304 without body)`);
      console.log(`${name}: reload, wasm statuses ${again.map((x) => x.status).join(',')}, ${body} B body`);
    }
    await browser.close();
  }
} finally {
  child.kill();
}
if (fails.length) {
  console.error(`compress: FAIL\n  ${fails.join('\n  ')}`);
  process.exit(1);
}
console.log('compress: PASS');
