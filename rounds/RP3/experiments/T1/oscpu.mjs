// RP3/T1: what the macOS process knobs do to Playwright WebKit's WebContent process:
// a WASM loop and a JS loop timed (5 times, median) with the JIT on and off, under
// no knob, PRIO_DARWIN_BG (`taskpolicy -b -p`), and throughput tiers (`taskpolicy -t N -p`).
// Usage (repo root): node rounds/RP3/experiments/T1/oscpu.mjs
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(new URL('../../../../shells/web/package.json', import.meta.url));
const { webkit } = require('playwright');
const bytes = [0, 97, 115, 109, 1, 0, 0, 0, 1, 6, 1, 96, 1, 127, 1, 127, 3, 2, 1, 0, 7, 5, 1, 1, 102, 0, 0, 10, 33, 1, 31, 1, 2, 127, 3, 64, 32, 2, 32, 1, 32, 1, 108, 106, 33, 2, 32, 1, 65, 1, 106, 34, 1, 32, 0, 72, 13, 0, 11, 32, 2, 11];
const pids = () => {
  try { return execFileSync('pgrep', ['-f', '--', 'WebKit.WebContent'], { encoding: 'utf8' }).split('\n').filter(Boolean); } catch { return []; }
};
const knobs = [[], ['-b'], ['-t', '1'], ['-t', '2'], ['-t', '3'], ['-t', '4'], ['-t', '5']];
const median = (v) => v.sort((a, b) => a - b)[v.length >> 1];
const out = [];
for (const jit of [true, false]) {
  for (const k of knobs) {
    const env = { ...process.env };
    if (!jit) env.__XPC_JSC_useJIT = 'false';
    const before = new Set(pids());
    const b = await webkit.launch({ env });
    const p = await b.newPage();
    await p.goto('about:blank');
    const fresh = pids().filter((x) => !before.has(x));
    for (const pid of fresh) if (k.length) execFileSync('taskpolicy', [...k, '-p', pid]);
    const r = await p.evaluate(async ({ bs, n }) => {
      const { instance } = await WebAssembly.instantiate(new Uint8Array(bs));
      const w = [];
      const j = [];
      for (let i = 0; i < 5; i++) {
        let t = performance.now();
        instance.exports.f(n);
        w.push(performance.now() - t);
        t = performance.now();
        let s = 0;
        for (let q = 0; q < n / 10; q++) s = (s + q * q) | 0;
        j.push(performance.now() - t);
      }
      return { w, j };
    }, { bs: bytes, n: jit ? 1e8 : 1e7 });
    const row = { jit, knob: k.join(' ') || 'none', procs: fresh.length, wasm_ms: median(r.w), js_ms: median(r.j) };
    out.push(row);
    console.error(JSON.stringify(row));
    await b.close();
  }
}
console.log(JSON.stringify({ load: execFileSync('uptime', { encoding: 'utf8' }).trim(), rows: out }, null, 1));
