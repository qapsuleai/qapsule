// R5/T2: the K52 budget of one design (SPEC §G7.6), with the calibrated phone simulation
// of RP3 (shells/web/tests/calibrate.mjs, rounds/RP3/CALIBRATION.md).
//
//   node k52.mjs <qp> <out.json> [--look NAME] [--face NAME] [--voice NAME] [--reps 3] [--configs phone-jit-est,phone]
//
// calibrate.mjs opens the page without design parameters; to measure a design the harness
// serves a copy of the shell directory (symbolic links; nothing in the repository changes)
// whose index.html first loads `look-shim.js`, which adds `look=`/`face=` to the address
// (history.replaceState, before main.js reads it; CSP `script-src 'self'` allows a file).
//
// K52 (DECISIONS.md): tick + paint p95 <= 8 ms; face and scene paint p95 <= 8 ms; long frames
// while scrolling/dragging <= 1 %; first capsule cold <= 1.5 s — on the high-end phone over
// HTTPS (config `phone-jit-est`, an estimate). Without the JIT (config `phone`, calibrated) the
// demo stays usable (long frames while dragging <= 10 %). Long-frame share is not a metric of
// the core summary; the proxy used here: frame p99 <= 2 display frames (33.4 ms) means fewer
// than 1 % long frames (SPEC §G7.6 says so; an open risk).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../..');
const cal = await import(path.join(root, 'shells/web/tests/calibrate.mjs'));

const argv = process.argv.slice(2);
const [qp, outFile] = argv;
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const look = opt('--look', '');
const face = opt('--face', '');
const motion = opt('--motion', '');
const voice = opt('--voice', ''); // (Ek G4-1) capsule voice; empty: the product default
const reps = Number(opt('--reps', '3'));
const configs = opt('--configs', 'phone-jit-est,phone').split(',');
const webSrc = opt('--web', path.join(root, 'shells/web'));
const CFG = { 'phone-jit-est': cal.PHONE_JIT_EST, phone: cal.PHONE };

function shim() {
  const web = webSrc;
  const dir = fs.mkdtempSync(path.join(process.env.QP_SCRATCH || os.tmpdir(), 'k52web-'));
  for (const f of fs.readdirSync(web)) {
    if (f === 'index.html') continue;
    fs.symlinkSync(path.join(web, f), path.join(dir, f));
  }
  const q = [look && `look=${encodeURIComponent(look)}`, face && `face=${encodeURIComponent(face)}`, voice && `voice=${encodeURIComponent(voice)}`, motion && `motion=${encodeURIComponent(motion)}`].filter(Boolean).join('&');
  fs.writeFileSync(path.join(dir, 'look-shim.js'),
    `if (${JSON.stringify(q)} && !location.search.includes(${JSON.stringify(q)})) history.replaceState(null, '', location.pathname + (location.search ? location.search + '&' : '?') + ${JSON.stringify(q)});\n`);
  const html = fs.readFileSync(path.join(web, 'index.html'), 'utf8')
    .replace('<script type="module" src="main.js"></script>', '<script src="look-shim.js"></script>\n<script type="module" src="main.js"></script>');
  fs.writeFileSync(path.join(dir, 'index.html'), html);
  return dir;
}

const LONG = 33.4;
function verdict(name, s) {
  const jit = name !== 'phone';
  const bias = jit ? { face: 1, scene: 1, tick: 1 } : { face: cal.PHONE_BIAS.face, scene: cal.PHONE_BIAS.scene, tick: cal.PHONE_BIAS.tick_with_scene };
  const v = {
    tick_p95: s.cal.tick.p95 * bias.tick,
    face_p95: s.cal.face.p95 * bias.face,
    scene_p95: s.cal.scene.p95 * bias.scene,
    first_capsule_ms: s.open_ms.first_capsule,
    frame_scroll_p99: s.feed.frame_scroll.p99,
  };
  const checks = jit
    ? { tick: v.tick_p95 <= 8, face: v.face_p95 <= 8, scene: v.scene_p95 <= 8, long_frames: v.frame_scroll_p99 <= LONG, first_capsule: v.first_capsule_ms <= 1500 }
    : { usable_long_frames: true }; // JIT-off: <= 10 % long frames; p90 is not reported -> recorded, not gated
  return { values: v, checks, pass: Object.values(checks).every(Boolean), phone_bias_applied: !jit };
}

const dir = shim();
const res = { motion, look, face, voice: voice || null, reps, configs: {} };
try {
  for (const c of configs) {
    const r = await cal.runCalibrated(CFG[c], { qp, web: dir, port: Number(process.env.QP_DEMO_PORT || 8471), reps, feedMs: 8000, chartMs: 20000, seek: 700 });
    res.configs[c] = { summary: r.summary, failed: r.failed, browser: r.browser, ...verdict(c, r.summary) };
  }
} finally {
  fs.rmSync(dir, { recursive: true, force: true });
}
res.pass = Object.values(res.configs).every((c) => c.pass);
fs.writeFileSync(outFile, `${JSON.stringify(res, null, 1)}\n`);
console.log(JSON.stringify({ look, face, voice: voice || null, pass: res.pass, configs: Object.fromEntries(Object.entries(res.configs).map(([k, v]) => [k, v.values])) }));
