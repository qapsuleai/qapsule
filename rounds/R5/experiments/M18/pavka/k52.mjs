// R5/M18 `pavka`: the K52 budget of the story chart at 5 / 50 / 200 stories in the visible
// range, on the default `canvas` paint path, in the calibrated phone simulation (`PHONE`,
// shells/web/tests/calibrate.mjs; rounds/RP3/CALIBRATION.md).
//
// Setting: `qp demo --synthetic --paused --history 1000 --days 1200` (the synthetic market
// with a long history: the first feed capsule's instrument has ~200 stories), replay at
// step 200, `?story=<design>`. The first feed capsule is opened to its chart; the visible
// range is set with the wheel so that ≈ N stories meet it (N = 5, 50, 200; the count is
// read from the device's story list and the view state's range). Then, per N, for `ms`
// milliseconds the replay plays (4 steps/s: ticks; the chart's last bar moves) and the
// chart is dragged by ±40 px (`--drag`) every 300 ms (scene builds and paints over the same range).
// Metrics are reset at the start of each phase and read at its end
// (`window.qp.metrics()`: span p50/p95 of `tick`, `scene` (paint), `scene_build`).
//
// Usage (from the repository root, after `scripts/demo.sh` or the verify `demo` step
// built shells/web/pkg):
//   node rounds/R5/experiments/M18/pavka/k52.mjs <qp> [--config phone] [--story pavka|r4]
//     [--ms 10000] [--port 8471]
// Prints one JSON object.
import { CONFIGS, PHONE, launchFor } from '../../../../../shells/web/tests/calibrate.mjs';
import { startDemo, settle } from '../../../../../shells/web/tests/lib.mjs';

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const cfg = CONFIGS[opt('--config', 'phone')] || PHONE;
const story = opt('--story', 'pavka');
const ms = Number(opt('--ms', '10000'));
const port = Number(opt('--port', '8471'));
const DRAG = Number(opt('--drag', '40'));
const web = new URL('../../../../../shells/web/', import.meta.url).pathname;

const visible = (st, look) => {
  const r = look.range;
  if (!st || !r) return 0;
  return st.stories.filter((s) => s.born_bar <= r.to_bar && (s.closed_bar === null || s.closed_bar === undefined ? Infinity : s.closed_bar) >= r.from_bar).length;
};

const read = async (page) => {
  const st = JSON.parse(await page.evaluate(() => window.qp.stories()));
  const look = JSON.parse(await page.evaluate(() => window.qp.look()));
  return { n: visible(st, look), range: look.range, total: st ? st.stories.length : 0 };
};

async function wheel(page, dy, times) {
  const box = await page.locator('#scene').boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + 60);
  for (let i = 0; i < times; i += 1) {
    await page.mouse.wheel(0, dy);
    await page.waitForTimeout(60);
    await settle(page, 60000);
  }
}

async function phase(page, target) {
  const box = await page.locator('#scene').boundingBox();
  const play = () => page.evaluate(() => document.getElementById('c_play').click());
  await page.evaluate(() => window.qp.resetMetrics());
  await play();
  const end = Date.now() + ms;
  let k = 0;
  while (Date.now() < end) {
    const x = box.x + box.width / 2;
    const y = box.y + 60;
    await page.mouse.move(x, y);
    await page.mouse.down();
    await page.mouse.move(x + (k % 2 ? -DRAG : DRAG), y, { steps: 4 });
    await page.mouse.up();
    k += 1;
    await page.waitForTimeout(300);
  }
  await play();
  await settle(page, 60000);
  const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
  const sp = (n) => m.span_ms[n] && { n: m.span_ms[n].n, p50: m.span_ms[n].p50, p95: m.span_ms[n].p95 };
  return { target, ...(await read(page)), tick: sp('tick'), scene: sp('scene'), scene_build: sp('scene_build'), spans: Object.fromEntries(Object.entries(m.span_ms).map(([k, v]) => [k, [v.n, v.p50, v.p95]])), counts: m.counts, paint: m.paint };
}

const browser = await launchFor(cfg);
const { child, base } = await startDemo(qp, web, port, ['--history', '1000', '--days', '1200']);
const out = { config: cfg.name, browser: browser.version(), story, ms, phases: [] };
try {
  const ctx = await browser.newContext({ viewport: cfg.viewport, deviceScaleFactor: cfg.dpr });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(`${base.replace('localhost', '127.0.0.1')}?story=${story}&x=1`);
  await page.waitForFunction(() => { try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; } }, null, { timeout: 180000, polling: 100 });
  await settle(page, 180000);
  await page.evaluate(() => { document.getElementById('c_date').value = '200'; document.getElementById('c_go').click(); });
  await page.waitForFunction(() => { try { return Number(JSON.parse(window.qp.status()).step.split(' ')[0]) >= 200; } catch { return false; } }, null, { timeout: 180000, polling: 100 });
  await settle(page, 180000);
  const first = await page.evaluate(() => JSON.parse(window.qp.feed()).filter((i) => i.id && i.scene)[0].id);
  await page.evaluate((id) => window.qp.openCapsule(id), first);
  await settle(page, 60000);
  await page.evaluate(() => window.qp.openTo('chart'));
  await settle(page, 60000);
  await page.waitForFunction(() => { try { const s = JSON.parse(window.qp.stories()); return s && s.complete; } catch { return false; } }, null, { timeout: 60000, polling: 100 });
  out.capsule = first;
  out.paint_used = await page.evaluate(() => window.qp.paintUsed());
  // Zoom in fully, then out until each target is met.
  await wheel(page, -240, 14);
  for (const target of [5, 50, 200]) {
    let r = await read(page);
    for (let i = 0; i < 40 && r.n < target; i += 1) {
      await wheel(page, 120, 1);
      const r2 = await read(page);
      if (r2.range && r.range && r2.range.from_bar === r.range.from_bar && r2.range.to_bar === r.range.to_bar) break;
      r = r2;
    }
    out.phases.push(await phase(page, target));
    if (process.env.K52_SHOT) await page.screenshot({ path: `${process.env.K52_SHOT}/k52-${story}-${target}.png` });
  }
  out.errors = errors;
  await ctx.close();
} finally {
  await browser.close();
  child.kill();
}
console.log(JSON.stringify(out, null, 1));
