// Pixel diff of two PNGs: `node pngdiff.mjs a.png b.png`.
// Prints {differingPixels, maxChannelDiff, meanAbsChannelDiff} as JSON.
import fs from 'node:fs';
import { PNG } from 'pngjs';

const [a, b] = process.argv.slice(2).map((p) => PNG.sync.read(fs.readFileSync(p)));
if (a.width !== b.width || a.height !== b.height) {
  console.log(JSON.stringify({ comparable: false, a: `${a.width}x${a.height}`, b: `${b.width}x${b.height}` }));
  process.exit(1);
}
let px = 0;
let max = 0;
let sum = 0;
for (let i = 0; i < a.data.length; i += 4) {
  let d = 0;
  for (let k = 0; k < 4; k++) {
    const ad = Math.abs(a.data[i + k] - b.data[i + k]);
    sum += ad;
    d = Math.max(d, ad);
  }
  if (d > 0) { px++; max = Math.max(max, d); }
}
console.log(JSON.stringify({ differingPixels: px, maxChannelDiff: max, meanAbsChannelDiff: +(sum / a.data.length).toFixed(4) }));
