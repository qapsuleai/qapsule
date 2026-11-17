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
]);

// R6: which modules a file may import. main.js: the core's package and the paint-path
// registry; a paint path (shells/web/paint/*.js): only sibling paint files.
export const IMPORTS = {
  main: (spec) => spec === './pkg/qp_wasm.js' || spec === './paint/index.js',
  paint: (spec) => /^\.\/[a-z0-9_-]+\.js$/.test(spec),
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
    for (const m of l.matchAll(/\bcore\s*\.\s*([A-Za-z_]\w*)/g)) {
      if (!ALLOWED_CORE.has(m[1])) problems.push([n, `R5 core method not allowed: ${m[1]}`]);
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
export function checkStatic(html, css) {
  const p = [];
  if (/https?:\/\/|\/\/[a-z0-9-]+\.[a-z]/i.test(html)) p.push([0, 'R1 external address in index.html']);
  if (/https?:\/\/|@import|url\(\s*['"]?\s*(https?:)?\/\//i.test(css)) p.push([0, 'R1 external address in style.css']);
  const scripts = [...html.matchAll(/<script\b([^>]*)>([\s\S]*?)<\/script>/gi)];
  for (const [, attrs, body] of scripts) {
    if (body.trim()) p.push([0, 'R6 inline script']);
    if (!/src="main\.js"/.test(attrs)) p.push([0, `R6 script other than main.js: ${attrs.trim()}`]);
  }
  if (/\son[a-z]+\s*=/i.test(html)) p.push([0, 'R6 inline event handler']);
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
let missed = 0;
for (const s of samples) if (!checkJs(s).length) { console.log(`self-test: not caught: ${s}`); missed++; }
for (const s of paintSamples) if (!checkJs(s, 'paint').length) { console.log(`self-test (paint): not caught: ${s}`); missed++; }
// …and a correct paint path's import must pass (the rule is not stricter than stated).
if (checkJs('import { put } from "./ref.js";', 'paint').length) { console.log('self-test (paint): sibling import refused'); missed++; }
const total = samples.length + paintSamples.length;
console.log(`arch: ${bad} violation(s) in main.js and ${paintFiles.length} paint file(s); self-test ${total - missed}/${total} caught`);
process.exit(bad || missed ? 1 : 0);
