// JS-shell view of the same payloads: JSON.parse (V8) vs. viewing qcol f64 blocks.
// Usage: node js-parse.mjs <payload dir>
import { readFileSync } from 'node:fs';
const dir = process.argv[2];
function median(f) {
  for (let i = 0; i < 5; i++) f();
  let t0 = performance.now(); f(); const single = Math.max(performance.now() - t0, 1e-4);
  const iters = Math.max(1, Math.ceil(0.5 / single));
  const v = [];
  for (let s = 0; s < 31; s++) { t0 = performance.now(); for (let i = 0; i < iters; i++) f(); v.push((performance.now() - t0) / iters); }
  v.sort((a, b) => a - b); return v[15] * 1000; // µs
}
console.log('| payload | bytes | JSON.parse µs | JSON.stringify µs | TextDecoder+JSON.parse µs |');
console.log('|---|---:|---:|---:|---:|');
for (const n of ['tick', 'rsi', 'macd', 'append10k', 'doc']) {
  const buf = readFileSync(`${dir}/${n}.json`);
  const txt = buf.toString('utf8');
  const td = new TextDecoder();
  const obj = JSON.parse(txt);
  let sink = 0;
  const p = median(() => { sink += JSON.parse(txt) ? 1 : 0; });
  const s = median(() => { sink += JSON.stringify(obj).length; });
  const p2 = median(() => { sink += JSON.parse(td.decode(buf)) ? 1 : 0; });
  console.log(`| ${n} | ${buf.length} | ${p.toFixed(1)} | ${s.toFixed(1)} | ${p2.toFixed(1)} |`);
}
// Exactness of V8 number parsing/printing on the same doubles: compare JSON round trip of the macd payload.
const m = JSON.parse(readFileSync(`${dir}/macd.json`, 'utf8'));
let diffs = 0, n = 0;
const f64 = new Float64Array(1), u = new BigUint64Array(f64.buffer);
for (const arr of Object.values(m.data.values.outputs)) for (const x of arr) { if (x === null) continue; n++; const y = JSON.parse(JSON.stringify(x)); if (!Object.is(x, y)) diffs++; }
console.log(`\nV8 JSON number round trip on MACD values: ${diffs} diffs of ${n}`);
// qcol: view the first f64 block of append10k without copying (offset of t block + 8 bytes * n).
const q = readFileSync(`${dir}/append10k.qcol`);
const ab = q.buffer.slice(q.byteOffset, q.byteOffset + q.byteLength);
console.log(`qcol append10k: ${q.length} bytes; Float64Array view creation µs: ${median(() => { new Float64Array(ab, 8 * Math.ceil(40 / 8), 10000); }).toFixed(3)} (offset illustrative)`);
