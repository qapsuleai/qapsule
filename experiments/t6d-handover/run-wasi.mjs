// Runs the experiment compiled to wasm32-wasip1 under Node's WASI.
// Usage: node run-wasi.mjs <file.wasm> [args...]
import { WASI } from 'node:wasi';
import { readFileSync } from 'node:fs';
const [file, ...args] = process.argv.slice(2);
const wasi = new WASI({ version: 'preview1', args: ['t6d-handover', ...args], env: {} });
const t0 = performance.now();
const mod = await WebAssembly.compile(readFileSync(file));
const inst = await WebAssembly.instantiate(mod, wasi.getImportObject());
console.error(`wasm compile+instantiate ms = ${(performance.now() - t0).toFixed(1)}`);
wasi.start(inst);
