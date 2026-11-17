// The paint-path registry (RP3/T2; contract: shells/web/paint/README.md). A new path is
// one file in this folder plus one line in PATHS; nothing else changes.

import ref from './ref.js';
import fixture from './fixture.js';
import canvas from './canvas.js';

export const PATHS = new Map([
  [ref.name, ref],
  [canvas.name, canvas], // RP3/M12b: the browser's Canvas 2D paints the core's command stream
  [fixture.name, fixture], // test fixture (tests/compare.mjs), supported only with ?fixture=1
]);

/**
 * The path for `?paint=<name>`: `{path, requested, used, reason}`. An unknown or
 * unsupported name falls back to `ref`, and the reason is reported (`core.set_paint`).
 */
export function choose(name) {
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
