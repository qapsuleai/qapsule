// R4/D device run (GOAL 9): the demo in headless Chromium under CPU throttling
// (`Emulation.setCPUThrottlingRate`), in a fixed order, each configuration `reps` times
// (fresh `qp demo` and fresh browser context per run, like a first visit), medians out.
//
// Per run: opening timeline (page open → first capsule) → play at 16 steps/s → phases,
// each `phase` ms with the metrics series reset at its start and read at its end:
//   idle   feed shown, nothing touched          frame `idle`, tick, face paint
//   scroll feed scrolled by wheel, both ways   frame `scroll`, tick, face paint
//   drag   first capsule opened to its chart,  frame `drag`, scene paint
//          horizontal drags
//   zoom   wheel zoom in and out on the chart  frame `zoom`, scene paint
// then memory (CDP JS heap, the core's WASM memory) and the metrics page's copy button
// (the summary in the text box must be the core's JSON).
//
// Usage: node tests/device.mjs <qp> [--rates 1,4,6] [--dprs 1,3] [--looks default,r3]
//          [--reps 3] [--phase 5000] [--raw] [-- extra qp demo args, e.g. --data DIR --from D]
// Real data (`--data`): print only; the numbers are timings (aggregate), no market values.
import { startDemo, launch, settle, clickIn } from './lib.mjs';

const argv = process.argv.slice(2);
const qp = argv.shift();
const sep = argv.indexOf('--');
const extra = sep >= 0 ? argv.splice(sep).slice(1) : [];
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const rates = opt('--rates', '1,4,6').split(',').map(Number);
const dprs = opt('--dprs', '1,3').split(',').map(Number);
const looks = opt('--looks', 'default,r3').split(',');
const reps = Number(opt('--reps', '3'));
const phase = Number(opt('--phase', '5000'));
const raw = argv.includes('--raw');
const here = new URL('..', import.meta.url).pathname;
const LOOK = { default: '', r3: '&face=r3&feed=r3&transition=r3&outcome=r3' };

const median = (xs) => {
  const v = xs.filter((x) => typeof x === 'number' && Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const m = v.length >> 1;
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
};

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

// The play/pause button, clicked through the DOM (the open layer's transition can keep
// Playwright's "stable" check waiting; the handler is the same).
const play = (page) => page.evaluate(() => document.getElementById('c_play').click());

async function phaseRead(page) {
  const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
  await page.evaluate(() => window.qp.resetMetrics());
  return m;
}

async function run(browser, rate, dpr, look) {
  const { child, base } = await startDemo(qp, here, 8398, extra);
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: dpr });
  try {
    const page = await ctx.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    const cdp = await ctx.newCDPSession(page);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate });
    await page.goto(`${base.replace('localhost', '127.0.0.1')}?x=1${LOOK[look]}`);
    await page.waitForFunction(() => {
      try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; }
    }, null, { timeout: 120000, polling: 20 });
    const open = JSON.parse(await page.evaluate(() => window.qp.metrics())).open_ms;
    await settle(page, 120000);
    await page.selectOption('#c_speed', '{"kind":"speed","steps_per_s":16}');
    await play(page);
    await page.evaluate(() => window.qp.resetMetrics());
    const out = { open };
    await page.waitForTimeout(phase);
    out.idle = await phaseRead(page);
    await wheelFor(page, phase, 195, 400, 120, 25);
    out.scroll = await phaseRead(page);
    // Open the first capsule to its chart (paused, so the feed does not move under the click).
    await play(page);
    await settle(page, 120000);
    await page.locator('#feed').evaluate((e) => { e.scrollTop = 0; });
    await page.waitForTimeout(300);
    const first = await page.evaluate(() => JSON.parse(window.qp.feed()).find((i) => i.id).id);
    await clickIn(page, `#list .face[data-id="${first}"] canvas`, 120, 60); await settle(page, 120000);
    await clickIn(page, '#scene', 195, 150); await settle(page, 120000);
    await page.waitForTimeout(800); // the depth transition
    const depth = JSON.parse(await page.evaluate(() => window.qp.open())).depth;
    await play(page);
    await page.evaluate(() => window.qp.resetMetrics());
    const b = await page.locator('#scene').boundingBox();
    const end = Date.now() + phase;
    while (Date.now() < end) {
      await page.mouse.move(b.x + 300, b.y + 150); await page.mouse.down();
      for (let i = 0; i < 20; i++) { await page.mouse.move(b.x + 300 - i * 10, b.y + 150); await page.waitForTimeout(8); }
      await page.mouse.up();
    }
    out.drag = await phaseRead(page);
    await wheelFor(page, phase, b.x + 250, b.y + 150, 100, 5);
    out.zoom = await phaseRead(page);
    const heap = await cdp.send('Runtime.getHeapUsage');
    const final = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    // The metrics page: rows shown, one tap copies the core's summary.
    await page.click('#tabs button[data-tab="metrics"]');
    await page.click('#m_copy');
    const copied = await page.locator('#m_text').inputValue();
    const rows = await page.locator('#mrows tr').count();
    let copyOk = false;
    try { copyOk = JSON.parse(copied).kind === 'qapsule-demo-metrics'; } catch { copyOk = false; }
    return { open, depth, idle: out.idle, scroll: out.scroll, drag: out.drag, zoom: out.zoom,
      heap_used: heap.usedSize, memory: final.memory_bytes, device: final.device, look: final.look,
      bundle: final.bundle, copy_ok: copyOk, rows, errors };
  } finally {
    await ctx.close();
    child.kill();
    await new Promise((r) => { child.once('exit', r); setTimeout(r, 2000); });
  }
}

// One configuration's medians over its runs.
function summarize(runs) {
  const pick = (f) => median(runs.map(f));
  const fr = (ph, mode) => {
    const g = (k) => pick((r) => r[ph].frame_ms[mode] && r[ph].frame_ms[mode][k]);
    const gaps = pick((r) => (r[ph].frame_ms[mode] && r[ph].frame_ms[mode].gaps_1s) || 0);
    // Frames delivered per second in the phase (headless Chromium paints at 120 Hz).
    return { n: g('n'), fps: g('n') === null ? 0 : g('n') / (phase / 1000), p50: g('p50'), p95: g('p95'), p99: g('p99'),
      max: g('max'), long_16_7: g('long_16_7'), long_33_3: g('long_33_3'), gaps_1s: gaps };
  };
  const sp = (phases, name) => {
    const g = (k) => median(runs.flatMap((r) => phases.map((ph) => r[ph].span_ms[name] && r[ph].span_ms[name][k])));
    return { n: g('n'), p50: g('p50'), p95: g('p95'), p99: g('p99') };
  };
  return {
    open_ms: { wasm_ready: pick((r) => r.open.wasm_ready), connected: pick((r) => r.open.connected), first_capsule: pick((r) => r.open.first_capsule) },
    frame: { idle: fr('idle', 'idle'), scroll: fr('scroll', 'scroll'), drag: fr('drag', 'drag'), zoom: fr('zoom', 'zoom') },
    tick: sp(['idle', 'scroll'], 'tick'),
    face: sp(['idle', 'scroll'], 'face'),
    scene: sp(['drag', 'zoom'], 'scene'),
    js_heap_mb: pick((r) => r.heap_used / 1048576),
    wasm_mb: pick((r) => r.memory.wasm && r.memory.wasm.max / 1048576),
    depth: runs.map((r) => r.depth),
    copy_ok: runs.every((r) => r.copy_ok && r.rows > 0),
    errors: runs.flatMap((r) => r.errors),
  };
}

const browser = await launch();
const result = { browser: browser.version(), node: process.version, extra, phase_ms: phase, reps, configs: [] };
try {
  for (const look of looks) {
    for (const dpr of dprs) {
      for (const rate of rates) {
        const runs = [];
        let failed = 0;
        while (runs.length < reps) {
          try {
            runs.push(await run(browser, rate, dpr, look));
          } catch (e) {
            failed += 1;
            console.error(`device: run failed (${failed}): ${String(e).split('\n')[0]}`);
            if (failed > reps) throw e;
          }
        }
        const cfg = { look, dpr, rate, failed_runs: failed, ...summarize(runs), bundle: runs[0].bundle, device: runs[0].device, look_used: runs[0].look };
        if (raw) cfg.runs = runs;
        result.configs.push(cfg);
        console.error(`device: ${look} dpr${dpr} ${rate}x first_capsule ${cfg.open_ms.first_capsule} scroll p95 ${cfg.frame.scroll.p95} zoom p95 ${cfg.frame.zoom.p95}`);
      }
    }
  }
} finally {
  await browser.close();
}
console.log(JSON.stringify(result, null, 1));
if (!result.configs.every((c) => c.copy_ok && !c.errors.length)) process.exit(1);
