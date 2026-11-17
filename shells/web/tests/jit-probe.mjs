// RP3/M14: the JIT probe's time under the engines and settings the thresholds are set
// from (rounds/RP3/experiments/M14.md §3). For each setting, `--reps` fresh pages: the
// page runs the probe once, 4 s after the start; its time is read from the summary
// (`mode.jit.probe_ms`).
//
// Usage: node tests/jit-probe.mjs <qp> [--reps 3] [--port 8493]
import { execFileSync } from 'node:child_process';
import { launchFor, PHONE } from './calibrate.mjs';
import { startDemo, settle } from './lib.mjs';

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const reps = Number(opt('--reps', '3'));
const port = Number(opt('--port', '8493'));
const here = new URL('..', import.meta.url).pathname;
const view = { width: 402, height: 874 };
const SETTINGS = [
  { name: 'webkit-jit', engine: 'webkit', dpr: 3, jsc: {}, viewport: view },
  { ...PHONE, name: 'webkit-nojit (PHONE)' },
  { name: 'webkit-nojit-dpr1', engine: 'webkit', dpr: 1, jsc: { useJIT: 'false' }, viewport: view },
  { name: 'chromium-1x', engine: 'chromium', dpr: 3, rate: 1, viewport: view },
  { name: 'chromium-4x', engine: 'chromium', dpr: 3, rate: 4, viewport: view },
  { name: 'chromium-16x', engine: 'chromium', dpr: 3, rate: 16, viewport: view },
];
const load = () => { try { return execFileSync('uptime', { encoding: 'utf8' }).trim().replace(/.*load averages?: /, ''); } catch { return ''; } };

const { child, base } = await startDemo(qp, here, port);
try {
  for (const cfg of SETTINGS) {
    const browser = await launchFor(cfg);
    const ms = [];
    let state = '';
    try {
      for (let i = 0; i < reps; i++) {
        const ctx = await browser.newContext({ viewport: cfg.viewport, deviceScaleFactor: cfg.dpr });
        const page = await ctx.newPage();
        if (cfg.engine === 'chromium' && cfg.rate > 1) {
          const cdp = await ctx.newCDPSession(page);
          await cdp.send('Emulation.setCPUThrottlingRate', { rate: cfg.rate });
        }
        await page.goto(`${base.replace('localhost', '127.0.0.1')}?anim=0`);
        await settle(page, 120000);
        await page.waitForFunction(() => JSON.parse(window.qp.metrics()).mode.jit.probe_ms !== null, null, { timeout: 60000, polling: 200 });
        const m = JSON.parse(await page.evaluate(() => window.qp.metrics())).mode;
        ms.push(m.jit.probe_ms);
        state = m.jit.state;
        await ctx.close();
      }
    } finally {
      await browser.close();
    }
    console.log(`${cfg.name.padEnd(22)} probe ms ${ms.join(' / ')}  (${state}; load ${load()})`);
  }
} finally {
  child.kill();
}
