// WASM sweep benchmark (R1/T3): 10,000 bars x 1,000 parameter sets per study, timed from
// JavaScript around one `sweep_bench` call (single thread). Also prints the output
// digest, which must equal the native one (`cargo run --release -p qp-compute --example
// sweep_bench`). Usage: scripts/wasm-sweep-bench.sh  (builds the nodejs package first).
import { createRequire } from 'node:module';
import { performance } from 'node:perf_hooks';

const require = createRequire(import.meta.url);
const pkg = process.argv[2];
const wasm = require(`${pkg}/qp_wasm.js`);
const bars = Number(process.env.BARS ?? 10000);
const reps = Number(process.env.REPS ?? 15);
const seed = 20261114;

console.log(`node ${process.version} (V8 ${process.versions.v8}) bars=${bars} reps=${reps}`);
console.log('study      variants   min ms    med ms    max ms  digest');
for (const study of ['sma', 'ema', 'rsi', 'macd', 'bollinger']) {
  const digest = wasm.sweep_digest(study, bars, seed); // also warms up
  wasm.sweep_bench(study, bars, seed);
  const t = [];
  for (let i = 0; i < reps; i++) {
    const t0 = performance.now();
    wasm.sweep_bench(study, bars, seed);
    t.push(performance.now() - t0);
  }
  t.sort((a, b) => a - b);
  const f = (v) => v.toFixed(2).padStart(9);
  console.log(`${study.padEnd(10)} ${'1000'.padStart(8)} ${f(t[0])} ${f(t[reps >> 1])} ${f(t[reps - 1])}  ${digest}`);
}
console.log(`compute_digest ${wasm.compute_digest()}`);
