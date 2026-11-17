// Paint path `cpu` (RP3/M12a): the reference renderer made smart. The core keeps each
// face's and the open scene's last frame, repaints only the rectangle the changed
// primitives touch (from the image of the unchanged prefix: grid, history bars), with the
// reference renderer's own drawing code: the pixels equal `ref`'s byte for byte. This
// file only moves them: it reads the frame through a view into the WASM memory (no copy
// across the boundary), keeps the canvas when its size is unchanged (setting
// `canvas.width` reallocates and clears it), and `putImageData`s only the changed
// rectangle. While a chart drag or pinch is in progress the core paints at scale 1; when
// it ends (`core.cpu_moving()` false) the page repaints once at the device scale.
// Every number (pointer, sizes, rectangle) comes from the core.
//
// Answer of `cpu_face` / `cpu_scene`: [ptr, len, w, h, dx, dy, dw, dh, settle, reuse, view].
// `?cpu=<words>` switches techniques off for measurements (`nodirty`, `nolayer`,
// `noreduce`, `noreuse`, `noview`, `off`); `?cpu_variants=1` registers one path per switch.
// Contract: shells/web/paint/README.md.

let core = null;
let env = null;
let polling = false;
const drawn = new WeakSet(); // canvases this path has painted whole since they were made

function put(canvas, r) {
  if (!r.length) return false;
  const t0 = performance.now();
  let whole = !drawn.has(canvas);
  if (r[9] !== 1 || canvas.width !== r[2] || canvas.height !== r[3]) {
    canvas.width = r[2];
    canvas.height = r[3];
    whole = true;
  }
  const data = r[10] === 1
    ? new Uint8ClampedArray(core.cpu_memory().buffer, r[0], r[1])
    : new Uint8ClampedArray(core.cpu_pixels().buffer);
  const img = new ImageData(data, r[2], r[3]);
  const ctx = canvas.getContext('2d');
  if (whole) ctx.putImageData(img, 0, 0);
  else ctx.putImageData(img, 0, 0, r[4], r[5], r[6], r[7]);
  drawn.add(canvas);
  core.metric_span('cpu_put', t0, performance.now());
  if (r[8] === 1) settle();
  return true;
}

// After a reduced-scale paint: once the gesture ends, everything visible is painted
// again (the scene at the device scale).
function settle() {
  if (polling) return;
  polling = true;
  const poll = () => {
    if (core && core.cpu_moving()) {
      requestAnimationFrame(poll);
      return;
    }
    polling = false;
    if (env) env.repaint();
  };
  requestAnimationFrame(poll);
}

function path(name, opts, flag) {
  return {
    name,
    supported: () => typeof ImageData === 'function' && (!flag || new URLSearchParams(location.search).get('cpu_variants') === '1'),
    init(c, e) {
      core = c;
      env = e;
      core.cpu_options(opts === null ? new URLSearchParams(location.search).get('cpu') || '' : opts);
    },
    paintFace(canvas, id, force) {
      return put(canvas, core.cpu_face(id, force));
    },
    paintScene(canvas, force) {
      return put(canvas, core.cpu_scene(force));
    },
  };
}

const cpu = path('cpu', null, false);

// Measurement variants (each with one technique off), only with `?cpu_variants=1`.
const VARIANTS = [
  path('cpu-nodirty', 'nodirty', true),
  path('cpu-nolayer', 'nolayer', true),
  path('cpu-noreduce', 'noreduce', true),
  path('cpu-noreuse', 'noreuse', true),
  path('cpu-noview', 'noview', true),
  path('cpu-off', 'off', true),
];

/** Registry entries: `cpu`, then the variants (each supported only with the flag). */
export function entries() {
  const out = [[cpu.name, cpu]];
  for (const v of VARIANTS) out.push([v.name, v]);
  return out;
}

export default cpu;
