// RP3/T1 calibration run: Arkin's iPhone scenario (rounds/R4/experiments/D/
// phone-iphone-ios27-default.json) as an automatic run, so that a desktop simulation can
// be compared with the phone and later improvements are measured with the same setting
// (rounds/RP3/CALIBRATION.md).
//
// Scenario (one run, fresh `qp demo` and fresh browser context): synthetic market, 30
// instruments, replay at 4 steps/s (`qp demo` default speed), viewport 402×874 CSS px.
//   feed   feed shown, `feedMs` ms: first half untouched, second half wheel-scrolled
//          (tick, face paint, frame `idle`/`scroll`)
//   chart  first open-story capsule opened to its chart and left open while the replay plays,
//          `chartMs` ms (tick, scene paint; on the phone run the chart was open for most
//          of the replay: scene n=213 of tick n=309)
// Metrics series are reset at the start of each phase and read at its end
// (`window.qp.metrics()`, the core's summary).
//
// Simulation knobs (a config): engine `chromium` | `webkit`, `dpr` (the core paints at the
// rounded scale), `rate` (Chromium CDP `Emulation.setCPUThrottlingRate`; ignored on
// WebKit), `jsc` (WebKit: JavaScriptCore options, e.g. {useJIT: 'false'}), `bg` (macOS: the page's renderer / WebContent process gets PRIO_DARWIN_BG via
// `taskpolicy -b -p`, which puts its threads on the efficiency cores), `web` (the shell
// directory served by `qp demo`: another WASM build or an instrumented page).
//
// Usage:
//   node tests/calibrate.mjs <qp> [--config phone|desktop|NAME] [--engine E] [--dpr N]
//     [--rate N] [--bg 0|1] [--jsc useJIT=false,...] [--web DIR] [--reps 3] [--feed-ms 8000] [--chart-ms 20000]
//     [--seek 700] [--port 8410] [--raw]
// Prints a JSON result (medians over the runs) and a table against Arkin's numbers.
//
// For later rounds: `import { PHONE, launchFor, runCalibrated } from './calibrate.mjs'`.
import { execFileSync } from 'node:child_process';
import { chromium, webkit } from 'playwright';
import { startDemo, settle, clickIn } from './lib.mjs';

/** Arkin's measurements on the R4 code (p50 / p95, ms). */
export const ARKIN = {
  phone: { tick: { p50: 74, p95: 118, n: 309 }, face: { p50: 32, p95: 72, n: 13 }, scene: { p50: 71, p95: 92, n: 213 } },
  desktop: { tick: { p50: 2.8, p95: 4.6, n: 1258 }, face: { p50: 0.9, p95: 1.2, n: 567 }, scene: { p50: 2.3, p95: 4.4, n: 8 } },
};

const VIEW_PHONE = { width: 402, height: 874 };

/**
 * The calibrated phone simulation ("kalibre benzetim", rounds/RP3/CALIBRATION.md §4):
 * Playwright WebKit 26.5 (JavaScriptCore, the engine of every iOS browser) with the JIT
 * off, at DPR 3 (the core paints at scale 3), the demo WASM as built by scripts/demo.sh
 * (profile `wasm`, no SIMD, no wasm-opt). Why the JIT is off: iOS 27 runs JS and WASM
 * without the JIT on pages that are not a secure context (plain `http://` on the LAN, as
 * Arkin's measurement was taken; WebKit bug 324968, "Enhanced Security").
 * Known bias against Arkin's iPhone (p50, phone / sim): face ≈ 1.0, scene ≈ 1.85; the tick
 * depends on how many ticks repaint the scene (§4). PHONE_BIAS reads sim numbers as phone
 * numbers.
 */
export const PHONE = { name: 'phone', engine: 'webkit', dpr: 3, rate: 1, bg: false, jsc: { useJIT: 'false' }, viewport: VIEW_PHONE };

/** Phone / sim factors of the calibrated setting (CALIBRATION.md §4), p50. */
export const PHONE_BIAS = { face: 1.0, scene: 1.85, tick_with_scene: 1.6 };

/**
 * Estimate, not calibrated: the same iPhone with the JIT on (a secure-context page, e.g.
 * over HTTPS). WebKit with the JIT at DPR 3; the phone is expected ≈ 1–2× slower than this
 * (CALIBRATION.md §6). To be confirmed on the phone.
 */
export const PHONE_JIT_EST = { name: 'phone-jit-est', engine: 'webkit', dpr: 3, rate: 1, bg: false, jsc: {}, viewport: VIEW_PHONE };

/**
 * Estimate, not calibrated: a 4–6 year old mid-range Android phone (Chrome, V8 with the
 * JIT; the iOS policy does not apply). Chromium CDP 5× at DPR 3, from single-core
 * benchmark ratios (CALIBRATION.md §6). No Android measurement exists yet.
 */
export const MID_ANDROID_EST = { name: 'mid-android-est', engine: 'chromium', dpr: 3, rate: 5, bg: false, viewport: { width: 393, height: 851 } };

/** Arkin's desktop run: Chrome, browser zoom 0.8 (the core paints at scale 1). */
export const DESKTOP = { name: 'desktop', engine: 'chromium', dpr: 1, rate: 1, bg: false, viewport: { width: 1280, height: 900 } };

/** R4's "mid-range" mapping (DEVICE.md §5), superseded; kept for comparison. */
export const R4_MID = { name: 'r4-mid', engine: 'chromium', dpr: 3, rate: 4, bg: false, viewport: VIEW_PHONE };

/** The numeric-only fit (face and scene near ±30%, wrong mechanism; CALIBRATION.md §4). */
export const CDP16 = { name: 'cdp16', engine: 'chromium', dpr: 3, rate: 16, bg: false, viewport: VIEW_PHONE };

export const CONFIGS = {
  phone: PHONE, 'phone-jit-est': PHONE_JIT_EST, 'mid-android-est': MID_ANDROID_EST,
  desktop: DESKTOP, 'r4-mid': R4_MID, cdp16: CDP16,
};

const here = new URL('..', import.meta.url).pathname;

const median = (xs) => {
  const v = xs.filter((x) => typeof x === 'number' && Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const m = v.length >> 1;
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
};

/** Launches the config's browser. */
export async function launchFor(cfg) {
  if (cfg.engine === 'webkit') {
    // JavaScriptCore options (`jsc`, e.g. {useJIT: 'false'}) reach the WebContent XPC
    // service only as __XPC_-prefixed variables of the launching process (verified:
    // rounds/RP3/experiments/T1/jit.mjs; plain JSC_* variables have no effect).
    const env = { ...process.env };
    for (const [k, v] of Object.entries(cfg.jsc || {})) env[`__XPC_JSC_${k}`] = String(v);
    return webkit.launch({ env });
  }
  return chromium.launch({ args: ['--force-color-profile=srgb', '--font-render-hinting=none'] });
}

/** A browser for the calibrated phone setting. */
export const launchPhone = () => launchFor(PHONE);

function pids(pattern) {
  try {
    return execFileSync('pgrep', ['-f', '--', pattern], { encoding: 'utf8' }).split('\n').filter(Boolean).map(Number);
  } catch {
    return [];
  }
}

// The processes that run the page's JS and WASM: WebKit's WebContent service, Chromium's
// renderer. New ones are those not running before the page was created.
const PAGE_PROC = { webkit: 'WebKit.WebContent', chromium: '--type=renderer' };

function background(pidList) {
  for (const p of pidList) {
    try { execFileSync('taskpolicy', ['-b', '-p', String(p)]); } catch { /* gone */ }
  }
}

async function wheelFor(page, ms, x, y, dy, flip) {
  await page.mouse.move(x, y);
  const end = Date.now() + ms;
  let k = 0;
  while (Date.now() < end) {
    await page.mouse.wheel(0, Math.floor(k / flip) % 2 ? -dy : dy);
    k += 1;
    await page.waitForTimeout(40);
  }
}

const play = (page) => page.evaluate(() => document.getElementById('c_play').click());

async function phaseRead(page) {
  const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
  await page.evaluate(() => window.qp.resetMetrics());
  return m;
}

/**
 * One run of the scenario under `cfg`. `opt`: {qp, web, port, feedMs, chartMs, bench}.
 * `bench`: when the served page defines `window.qpBench` (an instrumented experiment
 * page), it is called at the end with the replay paused and its result returned.
 */
export async function runOnce(browser, cfg, opt) {
  const web = opt.web || cfg.web || here;
  const { child, base } = await startDemo(opt.qp, web, opt.port);
  const before = new Set(pids(PAGE_PROC[cfg.engine]));
  const ctx = await browser.newContext({ viewport: cfg.viewport, deviceScaleFactor: cfg.dpr });
  try {
    const page = await ctx.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    if (cfg.engine === 'chromium' && cfg.rate > 1) {
      const cdp = await ctx.newCDPSession(page);
      await cdp.send('Emulation.setCPUThrottlingRate', { rate: cfg.rate });
    }
    await page.goto(`${base.replace('localhost', '127.0.0.1')}?x=1`);
    const fresh = pids(PAGE_PROC[cfg.engine]).filter((p) => !before.has(p));
    if (cfg.bg) background(fresh);
    await page.waitForFunction(() => {
      try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; }
    }, null, { timeout: 120000, polling: 50 });
    const open = JSON.parse(await page.evaluate(() => window.qp.metrics())).open_ms;
    await settle(page, 120000);
    if (opt.seek) {
      // The replay position of the phone run (it ended at step 1017): the page's own
      // "go" control with a step number.
      await page.evaluate((s) => { document.getElementById('c_date').value = String(s); document.getElementById('c_go').click(); }, opt.seek);
      await page.waitForFunction((s) => {
        try { return Number(JSON.parse(window.qp.status()).step.split(' ')[0]) >= s; } catch { return false; }
      }, opt.seek, { timeout: 120000, polling: 100 });
      await settle(page, 120000);
      await page.waitForTimeout(500);
    }
    // Feed phase: replay at the host's speed (4 steps/s).
    await play(page);
    await page.evaluate(() => window.qp.resetMetrics());
    await page.waitForTimeout(opt.feedMs / 2);
    await wheelFor(page, opt.feedMs / 2, cfg.viewport.width / 2, 450, 120, 12);
    const feed = await phaseRead(page);
    // Chart phase: the first capsule opened to its chart (paused while opening, so the
    // feed does not move under the click), then the replay plays on with it open.
    await play(page);
    await settle(page, 120000);
    await page.locator('#feed').evaluate((e) => { e.scrollTop = 0; });
    await page.waitForTimeout(300);
    // The first capsule whose story is still open (its chart follows the live bar, so
    // its scene changes with the data: on the phone run 213 scene paints for 309 ticks);
    // the first capsule when every story is closed.
    const first = await page.evaluate(() => {
      const caps = JSON.parse(window.qp.feed()).filter((i) => i.id && i.scene);
      return (caps.find((i) => !i.closed) || caps[0]).id;
    });
    // Opened through the page API (the core's `open_capsule`, the same path as a tap on
    // its face): the capsule need not be on screen.
    await page.evaluate((id) => window.qp.openCapsule(id), first);
    await settle(page, 120000);
    await clickIn(page, '#scene', 195, 150);
    await settle(page, 120000);
    await page.waitForTimeout(800);
    const opened = JSON.parse(await page.evaluate(() => window.qp.open()));
    await play(page);
    await page.evaluate(() => window.qp.resetMetrics());
    await page.waitForTimeout(opt.chartMs);
    const chart = await phaseRead(page);
    let bench = null;
    if (opt.bench && (await page.evaluate(() => typeof window.qpBench === 'function'))) {
      await play(page);
      await settle(page, 120000);
      bench = await page.evaluate((a) => window.qpBench(a), opt.bench);
    }
    const status = JSON.parse(await page.evaluate(() => window.qp.status()));
    return { open, feed, chart, bench, capsule: first, depth: opened.depth, scene_px: opened.px, status, device: chart.device, bg_pids: cfg.bg ? fresh : [], errors };
  } finally {
    await ctx.close();
    child.kill();
    await new Promise((r) => { child.once('exit', r); setTimeout(r, 2000); });
  }
}

function load() {
  try { return execFileSync('uptime', { encoding: 'utf8' }).trim().replace(/.*load averages?: /, ''); } catch { return ''; }
}

/** Medians of a config's runs. */
export function summarize(runs) {
  const sp = (ph, name) => {
    const g = (k) => median(runs.map((r) => r[ph].span_ms[name] && r[ph].span_ms[name][k]));
    return { n: g('n'), p50: g('p50'), p95: g('p95') };
  };
  const fr = (ph, mode) => {
    const g = (k) => median(runs.map((r) => r[ph].frame_ms[mode] && r[ph].frame_ms[mode][k]));
    return { n: g('n'), p50: g('p50'), p95: g('p95'), p99: g('p99') };
  };
  return {
    open_ms: { wasm_ready: median(runs.map((r) => r.open.wasm_ready)), first_capsule: median(runs.map((r) => r.open.first_capsule)) },
    feed: { tick: sp('feed', 'tick'), face: sp('feed', 'face'), frame_idle: fr('feed', 'idle'), frame_scroll: fr('feed', 'scroll') },
    chart: { tick: sp('chart', 'tick'), scene: sp('chart', 'scene'), face: sp('chart', 'face'), frame_idle: fr('chart', 'idle') },
    // The calibration numbers: tick and scene with the chart open (the phone run's mix),
    // face from the feed phase.
    cal: { tick: sp('chart', 'tick'), face: sp('feed', 'face'), scene: sp('chart', 'scene') },
    wasm_mb: median(runs.map((r) => r.chart.memory_bytes && r.chart.memory_bytes.wasm && r.chart.memory_bytes.wasm.max / 1048576)),
    scene_px: runs[0].scene_px,
    scale: runs[0].device && runs[0].device.scale,
    depth: runs.map((r) => r.depth),
    capsule: runs.map((r) => r.capsule),
    scenes_per_tick: median(runs.map((r) => (r.chart.span_ms.scene ? r.chart.span_ms.scene.n : 0) / ((r.chart.span_ms.tick && r.chart.span_ms.tick.n) || 1))),
    errors: runs.flatMap((r) => r.errors),
  };
}

/** Runs `cfg` `reps` times; returns {cfg, load_before, load_after, summary, runs}. */
export async function runCalibrated(cfg, opt) {
  const browser = await launchFor(cfg);
  const load0 = load();
  const runs = [];
  let failed = 0;
  try {
    while (runs.length < (opt.reps || 3)) {
      try {
        runs.push(await runOnce(browser, cfg, opt));
      } catch (e) {
        failed += 1;
        console.error(`calibrate: run failed (${failed}): ${String(e).split('\n')[0]}`);
        if (failed > (opt.reps || 3)) throw e;
      }
    }
  } finally {
    await browser.close();
  }
  return { cfg, browser: browser.version(), load_before: load0, load_after: load(), failed, summary: summarize(runs), runs: opt.raw ? runs : undefined };
}

/** Sim / phone ratio per metric (1.0 = exact; ±30% = 0.7…1.3). */
export function residuals(s, ref = ARKIN.phone) {
  const out = {};
  for (const k of ['tick', 'face', 'scene']) {
    out[k] = { p50: s.cal[k].p50 / ref[k].p50, p95: s.cal[k].p95 / ref[k].p95 };
  }
  return out;
}

async function main() {
  const argv = process.argv.slice(2);
  const qp = argv.shift();
  const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
  const base = CONFIGS[opt('--config', 'phone')] || { ...PHONE, name: opt('--config') };
  const cfg = {
    ...base,
    engine: opt('--engine', base.engine),
    dpr: Number(opt('--dpr', base.dpr)),
    rate: Number(opt('--rate', base.rate)),
    bg: opt('--bg', base.bg ? '1' : '0') === '1',
    jsc: opt('--jsc', null) === null ? base.jsc
      : Object.fromEntries(opt('--jsc').split(',').filter(Boolean).map((kv) => kv.split('='))),
  };
  const web = opt('--web', null);
  if (web) cfg.web = web;
  const o = {
    qp, web, port: Number(opt('--port', '8410')), reps: Number(opt('--reps', '3')),
    feedMs: Number(opt('--feed-ms', '8000')), chartMs: Number(opt('--chart-ms', '20000')), seek: Number(opt('--seek', '700')),
    raw: argv.includes('--raw'), bench: opt('--bench', null) ? JSON.parse(opt('--bench')) : null,
  };
  const r = await runCalibrated(cfg, o);
  const ref = cfg.name === 'desktop' ? ARKIN.desktop : ARKIN.phone;
  r.residuals = residuals(r.summary, ref);
  console.log(JSON.stringify(r, null, 1));
  const s = r.summary.cal;
  const f = (x) => (x === null || x === undefined ? '-' : Number(x).toFixed(1));
  console.error(`calibrate: ${cfg.name} ${cfg.engine} dpr${cfg.dpr} rate${cfg.rate} bg${cfg.bg ? 1 : 0} jsc ${JSON.stringify(cfg.jsc || {})} load ${r.load_before} -> ${r.load_after}`);
  for (const k of ['tick', 'face', 'scene']) {
    console.error(`  ${k.padEnd(5)} p50 ${f(s[k].p50).padStart(6)} (ref ${ref[k].p50}, x${r.residuals[k].p50.toFixed(2)})  p95 ${f(s[k].p95).padStart(6)} (ref ${ref[k].p95}, x${r.residuals[k].p95.toFixed(2)})  n ${f(s[k].n)}`);
  }
  if (r.summary.errors.length) process.exit(1);
}

if (import.meta.url === `file://${process.argv[1]}`) await main();
