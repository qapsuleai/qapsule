// The paint-path registry (RP3/T2; contract: shells/web/paint/README.md). A new path is
// one file in this folder plus one line in PATHS; nothing else changes.

import ref from './ref.js';
import fixture from './fixture.js';
import canvas from './canvas.js';
import gpu from './gpu.js';

export const PATHS = new Map([
  [ref.name, ref],
  [canvas.name, canvas], // RP3/M12b: the browser's Canvas 2D paints the core's command stream
  [fixture.name, fixture], // test fixture (tests/compare.mjs), supported only with ?fixture=1
  [gpu.name, gpu], // RP3/M12c: WebGPU, geometry from the core (default off: ?paint=gpu)
]);

// (R5/T0) Paths that were removed, and the path that now stands in for each (K51: the
// `cpu` path is gone, `canvas` + `ref` suffice). An old address still opens the page; the
// summary says why (`fallback: removed`).
const REMOVED = new Map([
  ['cpu', canvas],
]);

/**
 * The path for `?paint=<name>`: `{path, requested, used, reason}`. A removed name
 * (`cpu`) uses its stand-in (`canvas`, else `ref`), reason `removed`; an unknown or
 * unsupported name falls back to `ref`. The reason is reported (`core.set_paint`).
 */
export function choose(name) {
  const gone = REMOVED.get(name);
  if (gone) {
    const p = gone.supported() ? gone : ref;
    return { path: p, requested: name, used: p.name, reason: 'removed' };
  }
  const p = PATHS.get(name);
  if (!p) return { path: ref, requested: name, used: ref.name, reason: 'unknown' };
  if (!p.supported()) return { path: ref, requested: name, used: ref.name, reason: 'unsupported' };
  return { path: p, requested: name, used: p.name, reason: '' };
}

/** The names of the registered paths this browser supports, in registry order. */
export function supportedNames() {
  const out = [];
  for (const p of PATHS.values()) if (p.supported()) out.push(p.name);
  return out;
}
