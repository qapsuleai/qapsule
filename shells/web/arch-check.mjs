// R4/T5 architecture test of the web shell (rules: shells/web/ARCH.md; GOAL 4, VERIFY 9).
// Usage: node shells/web/arch-check.mjs            (checks the shell, then self-tests)
//        node shells/web/arch-check.mjs <file.js>  (checks one JS file as shell code)
// Exit 0 iff no rule is broken and every self-test sample is caught.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));

// The core's methods the shell may call (the wasm-bindgen `Shell`, crates/qp-wasm/src/lib.rs).
export const ALLOWED_CORE = new Set([
  'on_frame', 'take_outbox', 'idle', 'feed', 'face_rgba', 'scene_rgba', 'open', 'evidence',
  'status', 'take_error', 'look', 'instruments', 'audience', 'set_frame', 'replay', 'seek',
  'set_sensitivity', 'set_audience', 'set_style', 'back', 'open_to', 'pointer', 'wheel', 'transition',
  'transition_from', 'metric_frame', 'metric_span', 'metric_mark', 'metric_env', 'metric_device', 'metric_memory', 'metric_reset', 'metrics',
  'metrics_rows', 'digests', 'png', 'sent',
  // R4/H-M8910
  'set_look', 'mark_seen', 'seen_at', 'open_capsule', 'set_dpr', 'face_size', 'choices',
  'transition_progress', 'transition_frame',
  // RP3/M15 (ARCH.md): the viewer's visibility, its subscriptions/traffic and the check
  'visible', 'viewer', 'check',
  // RP3/T2 (ARCH.md): metrics across a new core, the paint path, paint lists and the font
  // for non-pixel paint paths, the automatic comparison.
  'metric_inherit', 'set_paint', 'face_scene', 'scene_desc', 'font',
  'bench_start', 'bench_tick', 'bench_rows', 'bench_summary',
  // RP3/T4 (ARCH.md): the replaced core is freed on a new start frame (wasm-bindgen).
  'free', 'metric_detach', 'metric_attach',
  // RP3/M14 (ARCH.md): one paint pass per animation frame (changes accumulated in the
  // core), the faces it paints, its metrics; the page's mode (secure context, WebGPU, JIT).
  'take_changes', 'has_changes', 'metric_pass', 'faces_to_paint', 'metric_flag', 'jit_probe',
  // RP3/M12b (ARCH.md): the `canvas` paint path's command stream, string table, variant, glyph atlas.
  'canvas_face', 'canvas_scene', 'canvas_strings', 'canvas_config', 'canvas_atlas_px',
  // RP3/M12c (ARCH.md): the GPU paint path's triangles, draws and glyph atlas, made by the core.
  'gpu_face', 'gpu_scene', 'gpu_vertices', 'gpu_capacity', 'gpu_draws', 'gpu_clear',
  'gpu_atlas_size', 'gpu_atlas_dirty', 'gpu_atlas_rows',
  // R5/T3 (ARCH.md): the look's CSS custom properties for the chrome; the embedded font
  // assets and their bytes (a canvas path registers each with FontFace, no network).
  'css', 'fonts', 'font_bytes',
  // R5/m17-int (ARCH.md): the glossary explanation the core opened on a tap, and its dismiss.
  'gloss', 'close_gloss', 'face_desc',
  // R5/M21 (ARCH.md): the reader's reduce-motion preference, and the motion overlay of a
  // display frame (boxes and their CSS values, made by the core).
  'set_reduced_motion', 'motion_marks',
]);

// R5/G (ARCH.md "Galeri"): the gallery page's core (`Gallery`, crates/qp-wasm/src/lib.rs).
// gallery.js calls only these (its own allow-list: the page talks to no other core).
export const ALLOWED_GALLERY = new Set([
  'layout_url', 'load', 'view', 'marks', 'pick', 'note', 'summary', 'share', 'state',
  'set_device', 'toggle_lang', 'set_theme', 'lang', 'theme', 'css', 'fonts', 'font_bytes',
]);

// R6: which modules a file may import. main.js: the core's package and the paint-path
// registry; a paint path (shells/web/paint/*.js): only sibling paint files.
export const IMPORTS = {
  main: (spec) => spec === './pkg/qp_wasm.js' || spec === './paint/index.js',
  paint: (spec) => /^\.\/[a-z0-9_-]+\.js$/.test(spec),
  gallery: (spec) => spec === './pkg/qp_wasm.js',
};

// Calls that compute, order, filter or format (R2).
const FORBIDDEN = [
  [/\.sort\s*\(/, 'sorting (.sort)'], [/\.filter\s*\(/, 'filtering (.filter)'],
  [/\.reduce(Right)?\s*\(/, 'folding (.reduce)'], [/\btoFixed\b|\btoPrecision\b|\btoExponential\b/, 'number formatting'],
  [/\btoLocale\w*/, 'locale formatting'], [/\bIntl\s*\./, 'Intl formatting'], [/\bMath\s*\./, 'Math'],
  [/\bNumber\s*\(|\bparseInt\b|\bparseFloat\b/, 'number parsing'], [/\bDate\b/, 'dates'],
  [/\.join\s*\(|\.concat\s*\(|\.padStart\b|\.padEnd\b|\.repeat\s*\(/, 'text building'],
  [/\.toString\s*\(|\bString\s*\(/, 'number to text'],
  [/\.slice\s*\(|\.splice\s*\(|\.substring\s*\(|\.substr\s*\(/, 'cutting lists or text'],
];

/** Code with comments and the contents of string literals removed; reports template interpolation. */
export function strip(src) {
  let out = '';
  const problems = [];
  let i = 0;
  let line = 1;
  while (i < src.length) {
    const c = src[i], d = src[i + 1];
    if (c === '\n') line++;
    if (c === '/' && d === '/') { while (i < src.length && src[i] !== '\n') i++; continue; }
    if (c === '/' && d === '*') {
      i += 2;
      while (i < src.length && !(src[i] === '*' && src[i + 1] === '/')) { if (src[i] === '\n') { line++; out += '\n'; } i++; }
      i += 2; continue;
    }
    if (c === '"' || c === "'" || c === '`') {
      const q = c; i++;
      while (i < src.length && src[i] !== q) {
        if (src[i] === '\\') i++;
        else if (q === '`' && src[i] === '$' && src[i + 1] === '{') problems.push([line, 'R4 template interpolation (${…})']);
        else if (src[i] === '\n') { line++; out += '\n'; }
        i++;
      }
      i++; out += q === '`' ? '``' : '""'; continue;
    }
    out += c; i++;
  }
  return { code: out, problems };
}

/** Rule violations of a JS file of the shell: [line, rule]. `kind`: `main` or `paint`. */
export function checkJs(src, kind = 'main') {
  const { code, problems } = strip(src);
  const lines = code.split('\n');
  lines.forEach((l, k) => {
    const n = k + 1;
    for (const [re, what] of FORBIDDEN) if (re.test(l)) problems.push([n, `R2 ${what}`]);
    // Arithmetic and string concatenation: + - * / % (and ++, --, +=, …). `=>`, comparisons
    // and logic are allowed. A `-` inside a name cannot occur in stripped JS.
    const arith = l.replace(/=>/g, '').match(/[+\-*/%]/);
    if (arith) problems.push([n, `R3 arithmetic or concatenation (${arith[0]})`]);
    if (/https?:\/\//.test(src.split('\n')[k] || '')) problems.push([n, 'R1 external address']);
    const allowed = kind === 'gallery' ? ALLOWED_GALLERY : ALLOWED_CORE;
    for (const m of l.matchAll(/\bcore\s*\.\s*([A-Za-z_]\w*)/g)) {
      if (!allowed.has(m[1])) problems.push([n, `R5 core method not allowed: ${m[1]}`]);
    }
    for (const m of l.matchAll(/\bimport\b[^;]*from\s*""/g)) void m;
  });
  // Imports: only the core's package and the paint registry (main.js), only sibling
  // paint files (a paint path). Dynamic import() is not allowed anywhere.
  for (const m of src.matchAll(/import\s+[^;]*?from\s+['"]([^'"]+)['"]/g)) {
    if (!IMPORTS[kind](m[1])) problems.push([0, `R6 import from ${m[1]}`]);
  }
  if (/\bimport\s*\(/.test(strip(src).code)) problems.push([0, 'R6 dynamic import()']);
  if (/\bfetch\s*\(\s*['"`]?\s*https?:/.test(src) || /new\s+(WebSocket|EventSource)\s*\(\s*['"`]?\s*(wss?|https?):/.test(src)) {
    problems.push([0, 'R1 request to another origin']);
  }
  return problems;
}

/** Rule violations of the page and the style sheet. */
export function checkStatic(html, css, script = 'main.js') {
  const p = [];
  if (/https?:\/\/|\/\/[a-z0-9-]+\.[a-z]/i.test(html)) p.push([0, 'R1 external address in the page']);
  if (/https?:\/\/|@import|url\(\s*['"]?\s*(https?:)?\/\//i.test(css)) p.push([0, 'R1 external address in the style sheet']);
  const scripts = [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)];
  for (const [, attrs, body] of scripts) {
    if (body.trim()) p.push([0, 'R6 inline script']);
    if (!attrs.includes(`src="${script}"`)) p.push([0, `R6 script other than ${script}: ${attrs.trim()}`]);
  }
  if (/\son[a-z]+\s*=/i.test(html)) p.push([0, 'R6 inline event handler']);
  return p;
}

// R7 (R5/T3b): the style sheet holds no design value. Every length, type size, weight,
// spacing and duration comes from the core as a custom property (`look::css_vars`, frames
// from `take_css`); a declaration may hold only var(--…), keywords and these structural
// values. Exceptions, with their reason:
//  - `0`, `100%`, `100vh`, `100vw`: structural (no design choice: nothing, all of it);
//  - `z-index`, `flex`: stacking order and "take the rest", not a look's value;
//  - the two @media widths: a media query cannot read a custom property (CSS Media
//    Queries 4: `var()` is not allowed in a media feature), so the phone/desktop switch stays.
export const CSS_OK_VALUES = new Set(['0', '100%', '100vh', '100vw']);
export const CSS_OK_PROPS = new Set(['z-index', 'flex']);
export const CSS_OK_MEDIA = new Set(['(min-width: 500px)', '(max-width: 499px)']);

/** R7 violations of a style sheet: [line, rule]. */
export function checkCss(css) {
  const p = [];
  // Comments out, newlines kept (line numbers).
  const src = css.replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, ' '));
  const lineAt = (i) => src.slice(0, i).split('\n').length;
  for (const m of src.matchAll(/@media([^{]*)\{/g)) {
    if (!CSS_OK_MEDIA.has(m[1].trim())) p.push([lineAt(m.index), `R7 media query not in the exception list: ${m[1].trim()}`]);
  }
  for (const m of src.matchAll(/\{([^{}]*)\}/g)) {
    let at = m.index + 1;
    for (const decl of m[1].split(';')) {
      const i = decl.indexOf(':');
      const here = at;
      at += decl.length + 1;
      if (i < 0) continue;
      const prop = decl.slice(0, i).trim();
      if (CSS_OK_PROPS.has(prop)) continue;
      const val = decl.slice(i + 1).replace(/var\(--[\w-]+\)/g, ' ');
      for (const n of val.matchAll(/(?<![\w-])-?(\d*\.)?\d+[a-z%]*/gi)) {
        if (CSS_OK_VALUES.has(n[0])) continue;
        p.push([lineAt(here + i), `R7 number in the style sheet (use a core custom property): ${prop}: ${n[0]}`]);
      }
    }
  }
  return p;
}

function report(name, problems) {
  for (const [n, r] of problems) console.log(`${name}:${n}: ${r}`);
  return problems.length;
}

const arg = process.argv[2];
if (arg) {
  process.exit(report(arg, checkJs(fs.readFileSync(arg, 'utf8'))) ? 1 : 0);
}
let bad = 0;
bad += report('shells/web/main.js', checkJs(fs.readFileSync(path.join(here, 'main.js'), 'utf8')));
// RP3/T2: every paint path is shell code under the same rules R1–R5 (R6: sibling imports).
const paintDir = path.join(here, 'paint');
const paintFiles = fs.existsSync(paintDir) ? fs.readdirSync(paintDir).filter((f) => f.endsWith('.js')).sort() : [];
for (const f of paintFiles) {
  bad += report(`shells/web/paint/${f}`, checkJs(fs.readFileSync(path.join(paintDir, f), 'utf8'), 'paint'));
}
bad += report('shells/web/index.html', checkStatic(
  fs.readFileSync(path.join(here, 'index.html'), 'utf8'), fs.readFileSync(path.join(here, 'style.css'), 'utf8')));
bad += report('shells/web/style.css', checkCss(fs.readFileSync(path.join(here, 'style.css'), 'utf8')));
// R5/G: the gallery page (gallery.html, gallery.js, gallery.css) under the same rules; its
// script calls only the gallery core's methods and imports only the core's package.
bad += report('shells/web/gallery.js', checkJs(fs.readFileSync(path.join(here, 'gallery.js'), 'utf8'), 'gallery'));
bad += report('shells/web/gallery.html', checkStatic(
  fs.readFileSync(path.join(here, 'gallery.html'), 'utf8'), fs.readFileSync(path.join(here, 'gallery.css'), 'utf8'), 'gallery.js'));
bad += report('shells/web/gallery.css', checkCss(fs.readFileSync(path.join(here, 'gallery.css'), 'utf8')));
// Self-test: each sample breaks one rule and must be caught.
const samples = [
  'items.sort((a, b) => a.t - b.t);', 'const x = list.filter((i) => i.open);', 'el.textContent = v.toFixed(2);',
  'el.textContent = item.symbol + " " + item.text;', 'const w = width * 2;', 'el.textContent = `${a} bar`;',
  'fetch("https://cdn.example.com/x.js");', 'core.compile_view(v);', 'const r = Math.round(x);',
  'new Intl.NumberFormat("tr").format(x);', 'import x from "https://cdn.example.com/m.js";', 'n++;',
  'const d = new Date(t);', 'el.textContent = names.join(", ");',
];
// RP3/T2: a paint path breaking a rule (each must be caught when checked as a paint file).
const paintSamples = [
  'import x from "../main.js";', 'import x from "./pkg/qp_wasm.js";', 'import("./other.js");',
  'ctx.lineTo(p.x * s, p.y);', 'ctx.fillStyle = "rgba(" + c[0] + ")";', 'core.face_view(id);',
  'const w = Math.ceil(x);', 'list.sort((a, b) => a.z);',
];
// R5/T3b: style sheet samples breaking R7 (each must be caught), and one that keeps it.
const cssSamples = [
  'a { padding: 4px; }', 'a { font: 14px/1.35 x; }', 'a { width: 2rem }', 'a { letter-spacing: .04em; }',
  'a { font-weight: 600; }', 'a { transition: color 120ms; }', '@media (min-width: 640px) { a { color: red } }',
  'a { margin: var(--qp-space-1) 3px; }',
];
let missed = 0;
for (const s of cssSamples) if (!checkCss(s).length) { console.log(`self-test (css): not caught: ${s}`); missed++; }
if (checkCss('a { padding: var(--qp-space-0) 0; z-index: 3; width: 100%; bottom: calc(var(--qp-bar-h) + var(--qp-space-2)); }').length) {
  console.log('self-test (css): a sheet keeping R7 refused'); missed++;
}
for (const s of samples) if (!checkJs(s).length) { console.log(`self-test: not caught: ${s}`); missed++; }
for (const s of paintSamples) if (!checkJs(s, 'paint').length) { console.log(`self-test (paint): not caught: ${s}`); missed++; }
// …and a correct paint path's import must pass (the rule is not stricter than stated).
if (checkJs('import { put } from "./ref.js";', 'paint').length) { console.log('self-test (paint): sibling import refused'); missed++; }
// R5/G: the gallery script breaking a rule (each must be caught as a gallery file).
const gallerySamples = [
  'core.on_frame(x);', 'import x from "./main.js";', 'el.textContent = c.id + ": " + c.note;',
  'const text = `${topic}=${cand}`;', 'list.sort((a, b) => a.k);', 'fetch("https://example.com/x");',
];
for (const s of gallerySamples) if (!checkJs(s, 'gallery').length) { console.log(`self-test (gallery): not caught: ${s}`); missed++; }
if (checkJs('core.pick(c.topic, c.id);', 'gallery').length) { console.log('self-test (gallery): allowed method refused'); missed++; }
if (!checkStatic('<script src="main.js"></script>', '', 'gallery.js').length) { console.log('self-test (gallery): other script not caught'); missed++; }
const total = samples.length + paintSamples.length + cssSamples.length + gallerySamples.length + 1;
console.log(`arch: ${bad} violation(s) in main.js, ${paintFiles.length} paint file(s), style.css and the gallery page; self-test ${total - missed}/${total} caught`);
process.exit(bad || missed ? 1 : 0);
