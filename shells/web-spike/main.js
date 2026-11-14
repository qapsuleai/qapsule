// Minimal web shell: loads qp-wasm, renders fixtures/bars/sample.json into a canvas with
// putImageData, and exposes results on `window.__qp` for automation (compare.mjs).
// All drawing happens in Rust (qp-render); this file only moves bytes.
const params = new URLSearchParams(location.search);
// ?pkg=<relative dir> selects an alternative wasm build (e.g. the simd128 variant).
const PKG = params.get('pkg') ?? './pkg';
const { default: init, render_rgba, render_png, render_synthetic_rgba, render_stress_rgba, default_width, default_height } =
  await import(`${PKG}/qp_wasm.js`);
const BENCH_N = Number(params.get('bench') ?? '100000');

function b64(bytes) {
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  }
  return btoa(s);
}

async function main() {
  const t = {};
  let t0 = performance.now();
  await init();
  t.wasmInitMs = performance.now() - t0;

  const json = await (await fetch('../../fixtures/bars/sample.json')).text();
  const w = default_width();
  const h = default_height();

  t0 = performance.now();
  const rgba = render_rgba(json, w, h); // first frame: includes font parse
  t.firstFrameMs = performance.now() - t0;

  t0 = performance.now();
  render_rgba(json, w, h);
  t.secondFrameMs = performance.now() - t0;

  const canvas = document.getElementById('chart');
  canvas.width = w;
  canvas.height = h;
  canvas.style.width = `${w}px`;
  canvas.style.height = `${h}px`;
  const ctx = canvas.getContext('2d');
  t0 = performance.now();
  ctx.putImageData(new ImageData(new Uint8ClampedArray(rgba), w, h), 0, 0);
  t.putImageDataMs = performance.now() - t0;

  t0 = performance.now();
  const png = render_png(json, w, h);
  t.renderPngMs = performance.now() - t0;

  if (BENCH_N > 0) {
    t0 = performance.now();
    render_synthetic_rgba(BENCH_N, 7, w, h);
    t.benchSyntheticMs = performance.now() - t0;
    t.benchSyntheticBars = BENCH_N;
  }

  // Determinism stress scene (fractional AA strokes, translucency, many text sizes), 600x300.
  t0 = performance.now();
  const stress = render_stress_rgba(600, 300);
  t.stressMs = performance.now() - t0;

  window.__qp = {
    done: true, width: w, height: h, timings: t,
    rgbaB64: b64(rgba), pngB64: b64(png), stressB64: b64(stress),
  };
  document.getElementById('log').textContent = JSON.stringify(t, null, 2);
}

main().catch((e) => {
  window.__qp = { done: true, error: String(e && e.stack || e) };
  document.getElementById('log').textContent = String(e);
});
