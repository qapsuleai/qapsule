// Serves the repo root on 127.0.0.1, opens shells/web-spike/index.html in headless
// Chromium (Playwright), and compares what the browser produced against the native golden
// image fixtures/golden/sample.png:
//   (a) canvas pixels read back with getImageData
//   (b) a screenshot of the <canvas> element (goes through the compositor)
//   (c) the raw RGBA returned by wasm, and the PNG bytes produced by wasm render_png
// Prints differing-pixel counts, max channel diff and timings as JSON.
//   (d) the wasm render of the stress scene vs fixtures/golden/stress.png
// Exit code 1 if (a) or (d) differ from their golden in more than QP_WEB_MAX_DIFF_PIXELS
// pixels (default 0: the same Rust code must give identical pixels in the browser).
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { PNG } from 'pngjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const goldenPath = path.join(root, 'fixtures/golden/sample.png');
const outDir = path.join(root, 'target/web-spike');
const maxDiffPixels = Number(process.env.QP_WEB_MAX_DIFF_PIXELS ?? '0');
const bench = process.env.QP_WEB_BENCH_BARS ?? '100000';
const pkg = process.env.QP_WEB_PKG ?? './pkg';

const MIME = {
  '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript',
  '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png',
};

function serve() {
  const server = http.createServer((req, res) => {
    const url = new URL(req.url, 'http://x');
    const p = path.normalize(path.join(root, decodeURIComponent(url.pathname)));
    if (!p.startsWith(root + path.sep) || !fs.existsSync(p) || fs.statSync(p).isDirectory()) {
      res.writeHead(404).end();
      return;
    }
    res.writeHead(200, { 'content-type': MIME[path.extname(p)] ?? 'application/octet-stream' });
    fs.createReadStream(p).pipe(res);
  });
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(server)));
}

function diff(a, b) {
  if (a.length !== b.length) return { comparable: false, lenA: a.length, lenB: b.length };
  let pixels = 0;
  let maxChannel = 0;
  for (let i = 0; i < a.length; i += 4) {
    let d = 0;
    for (let k = 0; k < 4; k++) d = Math.max(d, Math.abs(a[i + k] - b[i + k]));
    if (d > 0) { pixels++; maxChannel = Math.max(maxChannel, d); }
  }
  return { comparable: true, differingPixels: pixels, maxChannelDiff: maxChannel };
}

const golden = PNG.sync.read(fs.readFileSync(goldenPath));
const goldenBytes = fs.readFileSync(goldenPath);
const stressGolden = PNG.sync.read(fs.readFileSync(path.join(root, 'fixtures/golden/stress.png')));
fs.mkdirSync(outDir, { recursive: true });

const server = await serve();
const port = server.address().port;
const tLaunch = performance.now();
// QP_WEB_WEBGPU=1: use the full Chromium build (headless) with WebGPU enabled; the default
// headless shell has no WebGPU adapter.
const webgpu = process.env.QP_WEB_WEBGPU === '1';
const browser = await chromium.launch(webgpu
  ? { headless: true, channel: 'chromium', args: ['--enable-unsafe-webgpu'] }
  : { headless: true });
const page = await browser.newPage({ deviceScaleFactor: 1, viewport: { width: 1300, height: 800 } });
const launchMs = performance.now() - tLaunch;
if (process.env.QP_WEB_DEBUG === '1') {
  page.on('console', (m) => console.error('[page]', m.type(), m.text()));
  page.on('pageerror', (e) => console.error('[pageerror]', e));
}
let exitCode = 0;
try {
  const tNav = performance.now();
  await page.goto(`http://127.0.0.1:${port}/shells/web-spike/index.html?bench=${bench}&pkg=${encodeURIComponent(pkg)}`);
  await page.waitForFunction(() => window.__qp && window.__qp.done, null, { timeout: 120000 });
  const pageMs = performance.now() - tNav;
  const qp = await page.evaluate(() => window.__qp);
  if (qp.error) throw new Error(`page error: ${qp.error}`);

  // (a) canvas pixels
  const canvasB64 = await page.evaluate(() => {
    const c = document.getElementById('chart');
    const d = c.getContext('2d').getImageData(0, 0, c.width, c.height).data;
    let s = '';
    for (let i = 0; i < d.length; i += 0x8000) s += String.fromCharCode.apply(null, d.subarray(i, i + 0x8000));
    return btoa(s);
  });
  const canvasPx = Buffer.from(canvasB64, 'base64');
  // (b) element screenshot
  const shot = await page.locator('#chart').screenshot();
  fs.writeFileSync(path.join(outDir, 'canvas-screenshot.png'), shot);
  const shotPx = PNG.sync.read(shot);
  // (c) raw wasm outputs
  const wasmRgba = Buffer.from(qp.rgbaB64, 'base64');
  const wasmPng = Buffer.from(qp.pngB64, 'base64');
  fs.writeFileSync(path.join(outDir, 'wasm-render.png'), wasmPng);

  const result = {
    golden: path.relative(root, goldenPath),
    size: `${golden.width}x${golden.height}`,
    canvasVsGolden: diff(canvasPx, golden.data),
    screenshotVsGolden: shotPx.width === golden.width && shotPx.height === golden.height
      ? diff(shotPx.data, golden.data)
      : { comparable: false, screenshot: `${shotPx.width}x${shotPx.height}` },
    wasmRgbaVsGolden: diff(wasmRgba, golden.data),
    stressWasmVsGolden: diff(Buffer.from(qp.stressB64, 'base64'), stressGolden.data),
    wasmPngBytesEqualGolden: Buffer.compare(wasmPng, goldenBytes) === 0,
    timingsMs: { browserLaunch: launchMs, pageLoadToDone: pageMs, ...qp.timings },
    browser: browser.version(),
    pkg,
    threshold: { maxDiffPixels },
  };
  // Spike: vello_cpu in the browser vs the same backend natively (informational; produce the
  // native PNGs with `cargo run --release -p qp-render --example spike_vello_cpu --features spike-vello-cpu`).
  if (qp.spike && qp.spike.velloCpuSampleB64) {
    const nat = (f) => {
      const p = path.join(root, 'target/spikes', f);
      return fs.existsSync(p) ? PNG.sync.read(fs.readFileSync(p)).data : null;
    };
    const arch = process.arch === 'arm64' ? 'aarch64' : 'x86_64';
    const cmp = (b64, f) => {
      const n = nat(f);
      return n ? diff(Buffer.from(b64, 'base64'), n) : { missing: f };
    };
    result.spikeVelloCpu = {
      sampleVsNative: cmp(qp.spike.velloCpuSampleB64, `vello_cpu_${arch}_speed-u8_1T_sample.png`),
      stressVsNative: cmp(qp.spike.velloCpuStressB64, `vello_cpu_${arch}_speed-u8_1T_stress.png`),
      sampleF32VsNative: cmp(qp.spike.velloCpuSampleF32B64, `vello_cpu_${arch}_quality-f32_1T_sample.png`),
      sampleVsReferenceGolden: diff(Buffer.from(qp.spike.velloCpuSampleB64, 'base64'), golden.data),
    };
  }
  if (qp.spike && (qp.spike.velloGpu_area_SampleB64 || qp.spike.velloGpuError)) {
    const out = { error: qp.spike.velloGpuError };
    for (const aa of ['area', 'msaa8', 'msaa16']) {
      if (!qp.spike[`velloGpu_${aa}_SampleB64`]) continue;
      const nat = (f) => {
        const p = path.join(root, 'target/spikes', f);
        return fs.existsSync(p) ? PNG.sync.read(fs.readFileSync(p)).data : null;
      };
      const cmp = (b64, f) => { const n = nat(f); return n ? diff(Buffer.from(b64, 'base64'), n) : { missing: f }; };
      out[aa] = {
        browserRunToRunDistinctOf16: qp.spike[`velloGpu_${aa}_distinctOf16`],
        sampleVsNativeMetal: cmp(qp.spike[`velloGpu_${aa}_SampleB64`], `vello_gpu_${aa}_sample.png`),
        stressVsNativeMetal: cmp(qp.spike[`velloGpu_${aa}_StressB64`], `vello_gpu_${aa}_stress.png`),
        sampleVsReferenceGolden: diff(Buffer.from(qp.spike[`velloGpu_${aa}_SampleB64`], 'base64'), golden.data),
      };
    }
    result.spikeVelloGpu = out;
  }
  console.log(JSON.stringify(result, null, 2));
  for (const key of ['canvasVsGolden', 'stressWasmVsGolden']) {
    const c = result[key];
    if (!c.comparable || c.differingPixels > maxDiffPixels) {
      console.error(`FAIL: ${key} ${JSON.stringify(c)}, threshold ${maxDiffPixels} px`);
      exitCode = 1;
    } else {
      console.error(`OK: ${key} within threshold (${c.differingPixels} px)`);
    }
  }
} catch (e) {
  console.error(e);
  exitCode = 1;
} finally {
  await browser.close();
  server.close();
}
process.exit(exitCode);
