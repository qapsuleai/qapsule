// Paint path `ref` (RP3/T2): exactly R4's painting, the baseline of every comparison and
// the golden-compatible path. The core rasterises the scene description with the
// reference renderer (tiny-skia) at the device scale; this path resizes the canvas (which
// reallocates its backing store) and copies the pixels with `putImageData`.
// Contract: shells/web/paint/README.md.

let core = null;

/** Puts RGBA pixels `px` of `w` × `h` on `canvas`; false when there is nothing new. */
export function put(canvas, px, w, h) {
  if (!px.length) return false;
  canvas.width = w;
  canvas.height = h;
  canvas.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(px.buffer, px.byteOffset, px.length), w, h), 0, 0);
  return true;
}

export default {
  name: 'ref',
  supported: () => true,
  init(c) {
    core = c;
  },
  paintFace(canvas, id, force, px) {
    return put(canvas, core.face_rgba(id, force), px[0], px[1]);
  },
  paintScene(canvas, force, px) {
    return put(canvas, core.scene_rgba(force), px[0], px[1]);
  },
};
