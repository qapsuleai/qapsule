// node wkbench.mjs <wasm> [jit=0|1] [export] ; runs exported fn(kind) in WebKit (JIT off like PHONE)
import fs from 'node:fs';
import { createRequire } from 'node:module';
const require = createRequire('/Users/arkns/Works/Mchart/qapsule/shells/web/package.json');
const { webkit } = require('playwright');
const wasm = fs.readFileSync(process.argv[2]);
const jit = process.argv[3] === '1';
const fn = process.argv[4] || 'run';
const kinds = (process.argv[5] || '0,1,2,6').split(',').map(Number);
const env = { ...process.env };
if (!jit) env.__XPC_JSC_useJIT = 'false';
const b = await webkit.launch({ env });
const p = await b.newPage();
await p.route('http://bench.local/**', (r) => {
  if (r.request().url().endsWith('.wasm')) r.fulfill({ body: wasm, contentType: 'application/wasm' });
  else r.fulfill({ body: '<html></html>', contentType: 'text/html' });
});
await p.goto('http://bench.local/');
const res = await p.evaluate(async ([fn, kinds]) => {
  const m = await WebAssembly.instantiateStreaming(fetch('/b.wasm'), {});
  const run = m.instance.exports[fn];
  const out = {};
  for (const k of kinds) {
    for (let i = 0; i < 3; i++) run(k);
    const ts = [];
    for (let i = 0; i < 15; i++) {
      const t = performance.now();
      for (let j = 0; j < 10; j++) run(k);
      ts.push((performance.now() - t) / 10);
    }
    ts.sort((a, b) => a - b);
    out[k] = { med: ts[7], p90: ts[13], r: run(k) };
  }
  return out;
}, [fn, kinds]);
console.log(fn, JSON.stringify(res));
await b.close();
