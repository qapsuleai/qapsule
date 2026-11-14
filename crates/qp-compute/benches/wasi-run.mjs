// Runs a wasm32-wasip1 build of the sweep benchmark in Node (node:wasi), so the RP1 load
// profiles can be timed in V8 without touching qp-wasm.
//
//   cargo build -p qp-compute --example sweep_bench --target wasm32-wasip1 --profile wasm
//   node crates/qp-compute/benches/wasi-run.mjs target/wasm32-wasip1/wasm/examples/sweep_bench.wasm -- --reps 5
import { readFile } from "node:fs/promises";
import { WASI } from "node:wasi";
import { argv, versions } from "node:process";

const [file, ...rest] = argv.slice(2);
const args = rest[0] === "--" ? rest.slice(1) : rest;
console.log(`node ${versions.node} (V8 ${versions.v8}), wasm32-wasip1: ${file}`);
const wasi = new WASI({ version: "preview1", args: ["sweep_bench", ...args], env: {}, returnOnExit: true });
const mod = await WebAssembly.compile(await readFile(file));
const inst = await WebAssembly.instantiate(mod, wasi.getImportObject());
const code = wasi.start(inst);
if (code) process.exit(code);
