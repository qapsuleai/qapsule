// TEST FIXTURE, not a paint path to choose (RP3/T2). It exists so that the comparison test
// (tests/compare.mjs) has a second path to run, and so that the core's interface for
// non-pixel painters is exercised in a real browser: it asks for the paint list
// (`face_scene` / `scene_desc`, with the "unchanged → empty" short-circuit), parses it,
// registers the core's font through FontFace, and then paints the reference pixels.
// Supported only when the page's address carries `fixture=1`, so it never shows up in a
// user's comparison.

import { put } from './ref.js';

let core = null;

export default {
  name: 'fixture',
  supported: () => new URLSearchParams(location.search).get('fixture') === '1',
  init(c) {
    core = c;
    const t0 = performance.now();
    const font = new FontFace('QpMono', core.font());
    document.fonts.add(font);
    font.load().then(() => core.metric_span('fixture_font', t0, performance.now()), () => {});
  },
  paintFace(canvas, id, force, px) {
    const list = core.face_scene(id, force);
    if (!list) return false;
    JSON.parse(list);
    return put(canvas, core.face_rgba(id, true), px[0], px[1]);
  },
  paintScene(canvas, force, px) {
    const list = core.scene_desc(force);
    if (!list) return false;
    JSON.parse(list);
    return put(canvas, core.scene_rgba(true), px[0], px[1]);
  },
};
