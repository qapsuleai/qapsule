// Paint path `canvas` (RP3/M12b): the browser's own Canvas 2D context paints the scene
// description; the core no longer rasterises pixels. The core hands a flat command stream
// (`core.canvas_face` / `core.canvas_scene`, a Float32Array) and its string table
// (`core.canvas_strings`); this file maps each command onto one Canvas 2D call, verbatim.
// Every coordinate, width, colour, font string and text comes ready from the core
// (crates/qp-wasm/src/shell/paint_canvas.rs): no arithmetic, no layout, no text building
// here. The stream is read with an iterator (an index would need arithmetic, R3).
// Contract: shells/web/paint/README.md ("canvas komut akışı").

// Command codes (crates/qp-wasm/src/shell/paint_canvas.rs, `op`).
const FILL_STYLE = 1;
const STROKE_STYLE = 2;
const LINE_WIDTH = 3;
const FONT = 4;
const ALIGN = 5;
const RECT = 6;
const BEGIN = 7;
const MOVE = 8;
const LINE = 9;
const CLOSE = 10;
const ARC = 11;
const STROKE = 12;
const FILL = 13;
const TEXT = 14;
const CLIP = 15;
const RESTORE = 16;
const INIT = 17;
const GLYPH = 18;
const ATLAS = 19;

// The canvases are opaque (every scene paints its opaque background first). Measured
// (M12b.md): `alpha: true` and `textRendering` did not change the pixels, `desynchronized:
// true` did not change the timings (headless Chromium, WebKit), so they are left out.
const OPTIONS = { alpha: false };
// Diagnostic: `?cx_sync=1` reads one pixel back after each paint, so the measured span
// includes the browser's rasterisation (otherwise it covers recording the calls only).
const SYNC = new URLSearchParams(location.search).get('cx_sync') === '1';

let core = null;
let env = null;
let font = null; // the core's font, registered once per page
let ready = false;
let atlas = null; // the glyph atlas (`?canvas=atlas`): a copy of the core's, on a canvas
let atlasCtx = null;
const contexts = new WeakMap(); // canvas → its 2D context
const shows = new WeakMap(); // canvas → what it shows (a face id or 'scene')

function context(canvas) {
  let c = contexts.get(canvas);
  if (!c) {
    c = canvas.getContext('2d', OPTIONS);
    contexts.set(canvas, c);
  }
  return c;
}

/** Replaces rows `y0 .. y0 + rows` of the `w × h` glyph atlas with the core's pixels. */
function atlasRows(w, h, y0, rows) {
  if (!atlas) {
    atlas = document.createElement('canvas');
    // A CPU-backed atlas (`willReadFrequently`): with the default (GPU) atlas, headless
    // WebKit now and then drew glyphs uploaded in the same paint with red and blue
    // swapped (M12b.md, "WebKit renk takası").
    atlasCtx = atlas.getContext('2d', { willReadFrequently: true });
  }
  if (atlas.width !== w) atlas.width = w;
  if (atlas.height !== h) atlas.height = h;
  const px = core.canvas_atlas_px();
  atlasCtx.putImageData(new ImageData(new Uint8ClampedArray(px.buffer, px.byteOffset, px.length), w, rows), 0, y0);
}

/** One glyph from the atlas, unscaled: source `(sx, sy, w, h)` at `(dx, dy)`. */
function glyph(ctx, sx, sy, w, h, dx, dy) {
  ctx.drawImage(atlas, sx, sy, w, h, dx, dy, w, h);
}

/** Plays a command stream on `ctx`; `S` is its string table. */
function play(ctx, ops, S) {
  const it = ops.values();
  const n = () => it.next().value;
  for (let op = n(); op !== undefined; op = n()) {
    switch (op) {
      case MOVE: ctx.moveTo(n(), n()); break;
      case LINE: ctx.lineTo(n(), n()); break;
      case RECT: ctx.fillRect(n(), n(), n(), n()); break;
      case TEXT: ctx.fillText(S[n()], n(), n()); break;
      case GLYPH: glyph(ctx, n(), n(), n(), n(), n(), n()); break;
      case ATLAS: atlasRows(n(), n(), n(), n()); break;
      case FILL_STYLE: ctx.fillStyle = S[n()]; break;
      case STROKE_STYLE: ctx.strokeStyle = S[n()]; break;
      case LINE_WIDTH: ctx.lineWidth = n(); break;
      case FONT: ctx.font = S[n()]; break;
      case ALIGN: ctx.textAlign = S[n()]; break;
      case BEGIN: ctx.beginPath(); break;
      case CLOSE: ctx.closePath(); break;
      case ARC: ctx.arc(n(), n(), n(), 0, n()); break;
      case STROKE: ctx.stroke(); break;
      case FILL: ctx.fill(); break;
      case CLIP: ctx.save(); ctx.beginPath(); ctx.rect(n(), n(), n(), n()); ctx.clip(); break;
      case RESTORE: ctx.restore(); break;
      case INIT:
        ctx.lineCap = S[n()]; ctx.lineJoin = S[n()]; ctx.miterLimit = n(); ctx.textBaseline = S[n()];
        ctx.imageSmoothingEnabled = false;
        break;
      default: return;
    }
  }
}

/**
 * Paints on `canvas` (backing size `px`) what `get(force)` hands out. A canvas this path
 * has not painted for `what` yet, or whose size changes, is painted whole (`force`).
 */
function paint(canvas, what, force, px, get) {
  if (!ready) return false;
  let whole = force || shows.get(canvas) !== what;
  if (canvas.width !== px[0]) { canvas.width = px[0]; whole = true; }
  if (canvas.height !== px[1]) { canvas.height = px[1]; whole = true; }
  const ops = get(whole);
  if (!ops.length) return false;
  const t0 = performance.now();
  const ctx = context(canvas);
  play(ctx, ops, JSON.parse(core.canvas_strings()));
  if (SYNC) ctx.getImageData(0, 0, 1, 1);
  if (ops[0] === INIT && ops[5] === CLIP) core.metric_span('canvas_part', t0, performance.now());
  shows.set(canvas, what);
  return true;
}

export default {
  name: 'canvas',
  supported: () => typeof FontFace === 'function' && typeof document.fonts === 'object'
    && Boolean(document.createElement('canvas').getContext('2d')),
  init(c, e) {
    core = c;
    env = e;
    core.canvas_config(new URLSearchParams(location.search).get('canvas') || '');
    if (font) return;
    const t0 = performance.now();
    font = new FontFace('QpMono', core.font());
    document.fonts.add(font);
    // Nothing is painted before the font is in (no flash of a fallback font). Should it
    // fail to load, the canvas paints anyway, with the browser's fallback font.
    const go = () => {
      core.metric_span('canvas_font', t0, performance.now());
      ready = true;
      env.repaint();
    };
    font.load().then(go, go);
  },
  paintFace(canvas, id, force, px) {
    return paint(canvas, id, force, px, (whole) => core.canvas_face(id, whole));
  },
  paintScene(canvas, force, px) {
    return paint(canvas, 'scene', force, px, (whole) => core.canvas_scene(whole));
  },
};
