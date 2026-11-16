// Page-screenshot comparison for verify step 18 (R4). Scene images (painted by the core) are
// compared byte for byte elsewhere; a *.page.png also holds browser chrome (DOM controls,
// system font), which headless Chromium under heavy machine load occasionally rasterises one
// antialias level differently on a control's edge between runs (seen on the "Git" button's
// border: 14 px, ±1 level). This compares two page screenshots and passes when they are equal
// or differ only in at most MAX_PX pixels by at most MAX_LEVEL per channel; anything larger
// (a moved element, other text) fails. Usage: node tests/pagecmp.mjs <a.png> <b.png>
import fs from 'node:fs';
import zlib from 'node:zlib';

const MAX_PX = 64;
const MAX_LEVEL = 2;

function decode(file) {
  const d = fs.readFileSync(file);
  let i = 8;
  let w = 0, h = 0, ct = 0, bd = 0, il = 0;
  const idat = [];
  while (i < d.length) {
    const n = d.readUInt32BE(i);
    const t = d.toString('latin1', i + 4, i + 8);
    const c = d.subarray(i + 8, i + 8 + n);
    if (t === 'IHDR') { w = c.readUInt32BE(0); h = c.readUInt32BE(4); bd = c[8]; ct = c[9]; il = c[12]; }
    if (t === 'IDAT') idat.push(c);
    i += 12 + n;
  }
  if (bd !== 8 || il !== 0 || (ct !== 6 && ct !== 2)) throw new Error(`${file}: unsupported PNG (depth ${bd}, type ${ct}, interlace ${il})`);
  const bpp = ct === 6 ? 4 : 3;
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const st = w * bpp;
  const out = Buffer.alloc(st * h);
  let prev = Buffer.alloc(st);
  for (let y = 0, p = 0; y < h; y++) {
    const f = raw[p];
    const line = Buffer.from(raw.subarray(p + 1, p + 1 + st));
    p += 1 + st;
    for (let x = 0; x < st; x++) {
      const a = x >= bpp ? line[x - bpp] : 0, b = prev[x], c = x >= bpp ? prev[x - bpp] : 0;
      let v = 0;
      if (f === 1) v = a; else if (f === 2) v = b; else if (f === 3) v = (a + b) >> 1;
      else if (f === 4) { const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c); v = pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      line[x] = (line[x] + v) & 255;
    }
    line.copy(out, y * st);
    prev = line;
  }
  return { w, h, bpp, px: out };
}

const [fa, fb] = process.argv.slice(2);
if (fs.readFileSync(fa).equals(fs.readFileSync(fb))) process.exit(0);
const A = decode(fa), B = decode(fb);
if (A.w !== B.w || A.h !== B.h || A.bpp !== B.bpp) { console.error(`size differs: ${A.w}x${A.h} vs ${B.w}x${B.h}`); process.exit(1); }
let n = 0, worst = 0;
for (let k = 0; k < A.px.length; k += A.bpp) {
  let m = 0;
  for (let j = 0; j < A.bpp; j++) m = Math.max(m, Math.abs(A.px[k + j] - B.px[k + j]));
  if (m) { n++; worst = Math.max(worst, m); }
}
const ok = n <= MAX_PX && worst <= MAX_LEVEL;
console.error(`${n} px differ, at most ${worst} level(s): ${ok ? 'within chrome tolerance' : 'FAIL'}`);
process.exit(ok ? 0 : 1);
