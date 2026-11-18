// R5/M18 `korda` — K52 budget of the story chart on the `canvas` paint path, calibrated
// phone settings (shells/web/tests/calibrate.mjs, rounds/RP3/CALIBRATION.md).
//
// One run (fresh `qp demo --synthetic --paused`, fresh browser context, viewport 402×874):
//   1. the page with `?story=<design>` (or none: the `r4` reference, no story chart),
//      replay to step 700 with the page's own "go" control;
//   2. the first capsule opened to its chart (`qp.openCapsule` + a tap on the scene);
//   3. zoomed out (wheel) until the whole history is in view;
//   4. `qp.benchStories(n)`: the held story list replaced by n synthetic stories spread
//      over the bars (n = 0: the real list, ~30 stories);
//   5. replay plays for `chartMs` with the chart open: tick (`tick`: the core's work of
//      one data frame), scene paint (`scene`: build + `canvas` paint of the open scene)
//      and the paint pass after data (`data_pass`) — p50/p95 from `window.qp.metrics()`.
//
// Usage (from the repository root, after `wasm-pack build` into shells/web/pkg):
//   node rounds/R5/experiments/M18/korda/measure.mjs <qp> [--config phone|phone-jit-est]
//     [--design korda] [--n 0,5,50,200] [--reps 3] [--chart-ms 10000] [--port 8470]
// Prints one JSON object (medians over the runs per n).
import { CONFIGS, launchFor } from '../../../../../shells/web/tests/calibrate.mjs';
import { startDemo, settle, clickIn } from '../../../../../shells/web/tests/lib.mjs';

const web = new URL('../../../../../shells/web/', import.meta.url).pathname;

const median = (xs) => {
  const v = xs.filter((x) => typeof x === 'number' && Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const m = v.length >> 1;
  return v.length % 2 ? v[m] : (v[m - 1] + v[m]) / 2;
};

async function runOnce(browser, cfg, o, n) {
  const { child, base } = await startDemo(o.qp, web, o.port);
  const ctx = await browser.newContext({ viewport: cfg.viewport, deviceScaleFactor: cfg.dpr });
  try {
    const page = await ctx.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(String(e)));
    const q = o.design === 'r4' ? '' : `&story=${o.design}`;
    await page.goto(`${base.replace('localhost', '127.0.0.1')}?x=1${q}`);
    await page.waitForFunction(() => {
      try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; }
    }, null, { timeout: 120000, polling: 50 });
    await settle(page, 120000);
    await page.evaluate((s) => { document.getElementById('c_date').value = String(s); document.getElementById('c_go').click(); }, 700);
    await page.waitForFunction(() => {
      try { return Number(JSON.parse(window.qp.status()).step.split(' ')[0]) >= 700; } catch { return false; }
    }, null, { timeout: 120000, polling: 100 });
    await settle(page, 120000);
    const first = await page.evaluate(() => {
      const caps = JSON.parse(window.qp.feed()).filter((i) => i.id && i.scene);
      return (caps.find((i) => !i.closed) || caps[0]).id;
    });
    await page.evaluate((id) => window.qp.openCapsule(id), first);
    await settle(page, 120000);
    await clickIn(page, '#scene', 195, 150);
    await settle(page, 120000);
    const box = await page.locator('#scene').boundingBox();
    await page.mouse.move(box.x + 200, box.y + 100);
    for (let i = 0; i < 30; i += 1) {
      await page.mouse.wheel(0, 120);
      await page.waitForTimeout(30);
    }
    await settle(page, 120000);
    let rows = null;
    if (n > 0) rows = await page.evaluate((k) => window.qp.benchStories(k), n);
    else rows = await page.evaluate(() => { const s = JSON.parse(window.qp.stories()); return s && s.stories ? s.stories.length : 0; });
    await settle(page, 120000);
    const opened = JSON.parse(await page.evaluate(() => window.qp.open()));
    await page.evaluate(() => document.getElementById('c_play').click());
    await page.evaluate(() => window.qp.resetMetrics());
    await page.waitForTimeout(o.chartMs);
    const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
    await page.evaluate(() => document.getElementById('c_play').click());
    return { rows, depth: opened.depth, spans: m.span_ms, errors };
  } finally {
    await ctx.close();
    child.kill();
    await new Promise((r) => { child.once('exit', r); setTimeout(r, 2000); });
  }
}

async function main() {
  const argv = process.argv.slice(2);
  const qp = argv.shift();
  const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
  const cfg = CONFIGS[opt('--config', 'phone')];
  const o = {
    qp,
    design: opt('--design', 'korda'),
    port: Number(opt('--port', 8470)),
    chartMs: Number(opt('--chart-ms', 10000)),
  };
  const reps = Number(opt('--reps', 3));
  const ns = opt('--n', '0,5,50,200').split(',').map(Number);
  const browser = await launchFor(cfg);
  const out = { config: cfg.name, design: o.design, chart_ms: o.chartMs, reps, browser: browser.version(), by_n: {} };
  try {
    for (const n of ns) {
      const runs = [];
      for (let r = 0; r < reps; r += 1) {
        try {
          runs.push(await runOnce(browser, cfg, o, n));
        } catch (e) {
          console.error(`measure: n=${n} run failed: ${String(e).split('\n')[0]}`);
        }
      }
      const sp = (name, k) => median(runs.map((x) => x.spans[name] && x.spans[name][k]));
      const row = { runs: runs.length, rows: median(runs.map((x) => x.rows)), depth: runs.map((x) => x.depth) };
      for (const name of ['tick', 'scene', 'data_pass', 'paint_pass', 'scene_build']) {
        row[name] = { n: sp(name, 'n'), p50: sp(name, 'p50'), p95: sp(name, 'p95') };
      }
      row.errors = runs.flatMap((x) => x.errors);
      out.by_n[n] = row;
      console.error(`measure: ${cfg.name} ${o.design} n=${n}: scene p95 ${row.scene.p95} tick p95 ${row.tick.p95} data_pass p95 ${row.data_pass.p95}`);
    }
  } finally {
    await browser.close();
  }
  console.log(JSON.stringify(out, null, 1));
}

main().catch((e) => { console.error(e); process.exit(1); });
