// RP3/T1: does a JavaScriptCore option reach Playwright WebKit's WebContent process, and
// what does running JS / WASM without the JIT cost? A JS loop and a tiny WASM loop, per
// environment. Usage: node rounds/RP3/experiments/T1/jit.mjs  (from the repo root)
import { createRequire } from 'node:module';

const require = createRequire(new URL('../../../../shells/web/package.json', import.meta.url));
const { webkit } = require('playwright');

const envs = [
  {},
  { JSC_useJIT: 'false' },
  { JSC_useWasmJIT: 'false' },
  { JSC_useBBQJIT: 'false', JSC_useOMGJIT: 'false' },
  { JSC_useOMGJIT: 'false' },
  // XPC services (WebContent) get the launching process's __XPC_-prefixed variables.
  { __XPC_JSC_useJIT: 'false' },
  { __XPC_JSC_useWasmJIT: 'false' },
  { __XPC_JSC_useBBQJIT: 'false', __XPC_JSC_useOMGJIT: 'false' },
  { __XPC_JSC_useOMGJIT: 'false' },
  { __XPC_JSC_useWasmIPInt: 'false', __XPC_JSC_useBBQJIT: 'false', __XPC_JSC_useOMGJIT: 'false' },
];
// (func (param i32) (result i32) loop: s += i*i; i++ while i < n; return s)
const bytes = [0, 97, 115, 109, 1, 0, 0, 0, 1, 6, 1, 96, 1, 127, 1, 127, 3, 2, 1, 0, 7, 5, 1, 1, 102, 0, 0, 10, 33, 1, 31, 1, 2, 127, 3, 64, 32, 2, 32, 1, 32, 1, 108, 106, 33, 2, 32, 1, 65, 1, 106, 34, 1, 32, 0, 72, 13, 0, 11, 32, 2, 11];
for (const env of envs) {
  const b = await webkit.launch({ env: { ...process.env, ...env } });
  const p = await b.newPage();
  const r = await p.evaluate(async (bs) => {
    let js = performance.now();
    let s = 0;
    for (let i = 0; i < 3e7; i++) s = (s + i * i) | 0;
    js = performance.now() - js;
    const { instance } = await WebAssembly.instantiate(new Uint8Array(bs));
    instance.exports.f(1000);
    let w = performance.now();
    instance.exports.f(1e8);
    w = performance.now() - w;
    return { js_ms: js, wasm_ms: w, s };
  }, bytes);
  console.log(JSON.stringify(env), JSON.stringify(r));
  await b.close();
}
