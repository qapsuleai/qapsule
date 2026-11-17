// Paint path `gpu` (RP3/M12c): WebGPU, with the geometry made by the core.
//
// The core turns the scene into triangles (`gpu_face` / `gpu_scene`,
// crates/qp-wasm/src/shell/paint_gpu.rs): clip-space vertices with premultiplied colours
// and glyph-atlas texels, a list of draws, and the rows of its fontdue glyph atlas that
// changed. This file only uploads those bytes and issues the draws it is given, in the
// order given: no arithmetic, no layout, no text (shells/web/ARCH.md).
//
// One GPU device serves every canvas (each face and the chart has its own WebGPU
// context, configured with the shared device); multisample targets are shared by
// canvases of the same size. Anti-aliasing is 4× MSAA (K12). Translucent shapes that
// could overlap themselves are drawn "stencil, then cover" so each sample is blended once.
//
// Readiness: the adapter, device and pipelines are requested when this module loads (long
// before the core's start frame). Until they are ready the reference path paints (copy
// mode, below); when ready, `env.repaint()` paints everything again. If WebGPU is missing
// (`supported()` false → the registry picks `ref`) or the adapter/device cannot be had,
// every paint goes to the reference path (`ref.put`) and the fallback is reported
// (`core.set_paint('gpu', 'ref', 'unsupported')`). A lost device is requested again;
// canvases already holding a WebGPU context are reconfigured with the new device.
// Contract: shells/web/paint/README.md.

import { put } from './ref.js';

// Vertex layout (crates/qp-wasm/src/shell/paint_gpu.rs `VERTEX_BYTES` = 20): clip-space
// position (2 × f32), premultiplied colour (unorm8x4), atlas texel (2 × f32). Shapes
// sample a fully covered texel, glyph quads their glyph, so one shader draws both.
const SHADER = `
struct V {
  @builtin(position) pos: vec4f,
  @location(0) color: vec4f,
  @location(1) uv: vec2f,
};
@group(0) @binding(0) var atlas: texture_2d<f32>;
@vertex fn vs(@location(0) p: vec2f, @location(1) c: vec4f, @location(2) uv: vec2f) -> V {
  var o: V;
  o.pos = vec4f(p, 0.0, 1.0);
  o.color = c;
  o.uv = uv;
  return o;
}
@fragment fn fs(i: V) -> @location(0) vec4f {
  let a = textureLoad(atlas, vec2i(floor(i.uv)), 0).r;
  return i.color * a;
}
`;

const gpu = navigator.gpu;
const direct = new URLSearchParams(location.search).get('gpu_direct') === '1';
// 'pending' (requesting), 'ready', 'failed' (no adapter or device), 'none' (no WebGPU).
let state = gpu ? 'pending' : 'none';
let device = null;
let format = 'bgra8unorm';
let pipes = null; // by draw kind: plain, stencil fill, stencil union, cover
let groupLayout = null;
let atlas = null;
let bind = null;
let vbuf = null;
let core = null;
let env = null;
let reported = false;
let painted = new WeakSet(); // canvases painted with the current device
const contexts = new WeakMap(); // canvas → { ctx, device }
const targets = new Map(); // width → Map(height → { color, stencil }) multisample views
const times = { t0: 0, adapter: 0, device: 0, pipes: 0 };

function stencilState(front, back) {
  return { format: 'stencil8', stencilFront: front, stencilBack: back };
}

function pipelines(d) {
  const module = d.createShaderModule({ code: SHADER });
  groupLayout = d.createBindGroupLayout({
    entries: [{ binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } }],
  });
  const layout = d.createPipelineLayout({ bindGroupLayouts: [groupLayout] });
  const vertex = {
    module,
    entryPoint: 'vs',
    buffers: [{
      arrayStride: 20,
      attributes: [
        { shaderLocation: 0, offset: 0, format: 'float32x2' },
        { shaderLocation: 1, offset: 8, format: 'unorm8x4' },
        { shaderLocation: 2, offset: 12, format: 'float32x2' },
      ],
    }],
  };
  const over = { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' };
  const blend = { color: over, alpha: over };
  const keep = { compare: 'always', passOp: 'keep' };
  const make = (front, back, writeMask) => d.createRenderPipelineAsync({
    layout,
    vertex,
    fragment: { module, entryPoint: 'fs', targets: [{ format, blend, writeMask }] },
    primitive: { topology: 'triangle-list', cullMode: 'none' },
    depthStencil: stencilState(front, back),
    multisample: { count: 4 },
  });
  return Promise.all([
    make(keep, keep, GPUColorWrite.ALL),
    make({ compare: 'always', passOp: 'increment-wrap' }, { compare: 'always', passOp: 'decrement-wrap' }, 0),
    make({ compare: 'always', passOp: 'increment-clamp' }, { compare: 'always', passOp: 'increment-clamp' }, 0),
    make({ compare: 'not-equal', passOp: 'zero', failOp: 'keep' }, { compare: 'not-equal', passOp: 'zero', failOp: 'keep' }, GPUColorWrite.ALL),
  ]);
}

function setup() {
  state = 'pending';
  times.t0 = performance.now();
  gpu.requestAdapter().then((a) => {
    times.adapter = performance.now();
    if (!a) throw new Error('no WebGPU adapter');
    return a.requestDevice();
  }).then((d) => {
    times.device = performance.now();
    d.lost.then(() => lost(d));
    format = gpu.getPreferredCanvasFormat();
    return pipelines(d).then((p) => {
      times.pipes = performance.now();
      device = d;
      pipes = p;
      painted = new WeakSet();
      state = 'ready';
      report();
      if (env) env.repaint();
    });
  }).catch(() => {
    state = 'failed';
    report();
    if (env) env.repaint();
  });
}

// A lost device (driver reset, tab in the background on some systems, `destroy()`):
// forget everything made with it and ask again.
function lost(d) {
  if (d !== device) return;
  device = null;
  pipes = null;
  atlas = null;
  bind = null;
  vbuf = null;
  targets.clear();
  setup();
}

// Cold start (once per page) and the fallback, in the core's summary.
function report() {
  if (!core) return;
  if (state === 'failed') core.set_paint('gpu', 'ref', 'unsupported');
  if (state !== 'ready' || reported) return;
  reported = true;
  core.metric_span('gpu_adapter', times.t0, times.adapter);
  core.metric_span('gpu_device', times.adapter, times.device);
  core.metric_span('gpu_pipelines', times.device, times.pipes);
  core.metric_span('gpu_cold', times.t0, times.pipes);
}

if (gpu) setup();

// The canvas's WebGPU context, configured with the current device (null: the canvas
// already has another kind of context).
function context(canvas, px) {
  let c = contexts.get(canvas);
  if (!c) {
    const ctx = canvas.getContext('webgpu');
    if (!ctx) return null;
    c = { ctx, device: null };
    contexts.set(canvas, c);
  }
  if (canvas.width !== px[0]) canvas.width = px[0];
  if (canvas.height !== px[1]) canvas.height = px[1];
  if (c.device !== device) {
    c.ctx.configure({ device, format, alphaMode: 'premultiplied' });
    c.device = device;
  }
  return c.ctx;
}

// Multisample colour and stencil targets for a canvas size (shared; drawing is in queue
// order). Memoryless on tile-based GPUs where the browser offers transient attachments.
function target(px) {
  let row = targets.get(px[0]);
  if (!row) {
    row = new Map();
    targets.set(px[0], row);
  }
  let t = row.get(px[1]);
  if (!t) {
    // WebKit 26.5 refuses a transient stencil8 attachment ("depth stencil texture is
    // not renderable", M12c.md), so only the colour target is transient.
    const usage = GPUTextureUsage.RENDER_ATTACHMENT;
    t = {
      color: device.createTexture({ size: px, sampleCount: 4, format, usage: usage | (GPUTextureUsage.TRANSIENT_ATTACHMENT || 0) }).createView(),
      stencil: device.createTexture({ size: px, sampleCount: 4, format: 'stencil8', usage }).createView(),
    };
    row.set(px[1], t);
  }
  return t;
}

// The glyph atlas texture and the rows of it the core changed.
function upload() {
  let full = false;
  if (!atlas) {
    atlas = device.createTexture({ size: core.gpu_atlas_size(), format: 'r8unorm', usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST });
    bind = device.createBindGroup({ layout: groupLayout, entries: [{ binding: 0, resource: atlas.createView() }] });
    full = true;
  }
  const r = core.gpu_atlas_dirty(full);
  if (!r.length) return;
  device.queue.writeTexture({ texture: atlas, origin: [0, r[0]] }, core.gpu_atlas_rows(), { bytesPerRow: r[1] }, [r[1], r[2]]);
}

// Draws the frame the core just built onto `ctx`.
function draw(ctx, px) {
  upload();
  const v = core.gpu_vertices();
  if (!vbuf || v.byteLength > vbuf.size) {
    if (vbuf) vbuf.destroy();
    vbuf = device.createBuffer({ size: core.gpu_capacity(), usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST });
  }
  device.queue.writeBuffer(vbuf, 0, v);
  const t = target(px);
  const enc = device.createCommandEncoder();
  const pass = enc.beginRenderPass({
    colorAttachments: [{ view: t.color, resolveTarget: ctx.getCurrentTexture().createView(), clearValue: core.gpu_clear(), loadOp: 'clear', storeOp: 'discard' }],
    depthStencilAttachment: { view: t.stencil, stencilClearValue: 0, stencilLoadOp: 'clear', stencilStoreOp: 'discard' },
  });
  pass.setBindGroup(0, bind);
  pass.setVertexBuffer(0, vbuf);
  const firsts = core.gpu_draws(1);
  const counts = core.gpu_draws(2);
  core.gpu_draws(0).forEach((k, i) => {
    pass.setPipeline(pipes[k]);
    pass.draw(counts[i], 1, firsts[i]);
  });
  pass.end();
  device.queue.submit([enc.finish()]);
  const c = core;
  const t0 = performance.now();
  device.queue.onSubmittedWorkDone().then(() => c.metric_span('gpu_gpu', t0, performance.now()), () => {});
  return true;
}

// The offscreen WebGPU canvas of a size (copy mode; one per size, shared by canvases).
function offscreen(px) {
  const t = target(px);
  if (!t.off) {
    const off = typeof OffscreenCanvas === 'function' ? new OffscreenCanvas(px[0], px[1]) : document.createElement('canvas');
    off.width = px[0];
    off.height = px[1];
    t.off = off;
    t.ctx = off.getContext('webgpu');
    t.ctx.configure({ device, format, alphaMode: 'premultiplied' });
  }
  return t;
}

// One paint: `build(force)` asks the core for the triangles (false: unchanged), `rgba()`
// for the reference pixels.
//
// Copy mode (default): the frame is drawn on a shared offscreen WebGPU canvas and copied
// into the page's canvas with `drawImage` in the same task. Page canvases stay 2D, so
// they can be read back (`window.qp.png`, the depth transition's snapshot: a presented
// WebGPU canvas reads back blank) and painted by `ref` while the device is not ready or
// lost. Direct mode (`?gpu_direct=1`, measured for comparison): each page canvas has its
// own WebGPU context; no copy, but no readback after presentation and no fallback for a
// canvas once it holds that context.
function paint(canvas, force, px, build, rgba) {
  if (state !== 'ready') {
    if (direct && state === 'pending') return false;
    if (contexts.has(canvas)) return false;
    return put(canvas, rgba(force), px[0], px[1]);
  }
  if (direct) {
    const ctx = context(canvas, px);
    if (!ctx) return put(canvas, rgba(force), px[0], px[1]);
    // A canvas not yet painted with this device is painted whatever the record says.
    if (!build(force || !painted.has(canvas))) return false;
    painted.add(canvas);
    return draw(ctx, px);
  }
  // A canvas not yet painted with this device (painted by `ref` while it was not ready)
  // is painted whatever the record says.
  if (!build(force || !painted.has(canvas))) return false;
  painted.add(canvas);
  const t = offscreen(px);
  draw(t.ctx, px);
  if (canvas.width !== px[0]) canvas.width = px[0];
  if (canvas.height !== px[1]) canvas.height = px[1];
  canvas.getContext('2d').drawImage(t.off, 0, 0);
  return true;
}

export default {
  name: 'gpu',
  // WebGPU present (and not known to have failed). The adapter itself is asynchronous;
  // when it turns out missing, the path paints with `ref` and reports the fallback.
  supported: () => Boolean(gpu) && state !== 'failed',
  init(c, e) {
    core = c;
    env = e;
    report();
  },
  paintFace(canvas, id, force, px) {
    return paint(canvas, force, px, (f) => core.gpu_face(id, f), (f) => core.face_rgba(id, f));
  },
  paintScene(canvas, force, px) {
    return paint(canvas, force, px, (f) => core.gpu_scene(f), (f) => core.scene_rgba(f));
  },
};
