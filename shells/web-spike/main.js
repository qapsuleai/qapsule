// Minimal web shell: loads qp-wasm, renders fixtures/bars/sample.json into a canvas with
// putImageData, and exposes results on `window.__qp` for automation (compare.mjs).
// All drawing happens in Rust (qp-render); this file only moves bytes.
const params = new URLSearchParams(location.search);
// ?pkg=<relative dir> selects an alternative wasm build (e.g. the simd128 variant).
const PKG = params.get('pkg') ?? './pkg';
const mod = await import(`${PKG}/qp_wasm.js`);
const { default: init, render_rgba, render_png, render_synthetic_rgba, render_stress_rgba, default_width, default_height } = mod;
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

  // Optional spike exports (qp-wasm built with feature `spike-vello-cpu`).
  const spike = {};
  if (mod.render_rgba_vello_cpu) {
    t0 = performance.now();
    const v = mod.render_rgba_vello_cpu(json, w, h, false);
    t.velloCpuFirstFrameMs = performance.now() - t0;
    t0 = performance.now();
    mod.render_rgba_vello_cpu(json, w, h, false);
    t.velloCpuSecondFrameMs = performance.now() - t0;
    if (BENCH_N > 0) {
      t0 = performance.now();
      mod.render_synthetic_rgba_vello_cpu(BENCH_N, 7, w, h);
      t.velloCpuBenchSyntheticMs = performance.now() - t0;
    }
    spike.velloCpuSampleB64 = b64(v);
    spike.velloCpuStressB64 = b64(mod.render_stress_rgba_vello_cpu(600, 300, false));
    spike.velloCpuSampleF32B64 = b64(mod.render_rgba_vello_cpu(json, w, h, true));
  }

  // Optional spike: vello on WebGPU (qp-wasm built with feature `spike-vello-gpu`; needs a
  // browser with WebGPU, e.g. Playwright channel "chromium" + --enable-unsafe-webgpu).
  if (mod.vello_gpu_init) {
    try {
      console.log('vello_gpu: init');
      t0 = performance.now();
      await mod.vello_gpu_init();
      console.log('vello_gpu: init done');
      t.velloGpuInitMs = performance.now() - t0;
      const names = ['area', 'msaa8', 'msaa16'];
      for (let aa = 0; aa < 3; aa++) {
        t0 = performance.now();
        console.log(`vello_gpu: render ${names[aa]}`);
        const first = await mod.vello_gpu_render_rgba(json, w, h, aa);
        t[`velloGpu_${names[aa]}_FirstFrameMs`] = performance.now() - t0;
        t0 = performance.now();
        await mod.vello_gpu_render_rgba(json, w, h, aa);
        t[`velloGpu_${names[aa]}_SecondFrameMs`] = performance.now() - t0;
        // run-to-run determinism inside the browser
        let distinct = 1;
        const seen = [first];
        for (let k = 0; k < 15; k++) {
          const r = await mod.vello_gpu_render_rgba(json, w, h, aa);
          if (!seen.some((x) => x.length === r.length && x.every((v, i) => v === r[i]))) { seen.push(r); distinct++; }
        }
        spike[`velloGpu_${names[aa]}_distinctOf16`] = distinct;
        spike[`velloGpu_${names[aa]}_SampleB64`] = b64(first);
        spike[`velloGpu_${names[aa]}_StressB64`] = b64(await mod.vello_gpu_render_stress_rgba(600, 300, aa));
      }
      if (BENCH_N > 0) {
        await mod.vello_gpu_render_synthetic_rgba(BENCH_N, 7, w, h, 0);
        t0 = performance.now();
        await mod.vello_gpu_render_synthetic_rgba(BENCH_N, 7, w, h, 0);
        t.velloGpuBenchSyntheticMs = performance.now() - t0;
      }
    } catch (e) {
      spike.velloGpuError = String(e && e.stack || e);
    }
  }

  window.__qp = {
    done: true, width: w, height: h, timings: t,
    rgbaB64: b64(rgba), pngB64: b64(png), stressB64: b64(stress), spike,
  };
  document.getElementById('log').textContent = JSON.stringify(t, null, 2);
}

main().catch((e) => {
  window.__qp = { done: true, error: String(e && e.stack || e) };
  document.getElementById('log').textContent = String(e);
});
