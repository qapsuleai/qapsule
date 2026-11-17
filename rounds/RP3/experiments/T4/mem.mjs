// RP3/T4 — memory over a long replay in a headless browser (GOAL 7, VERIFY 10).
//
// `qp demo --synthetic --days 1000` (30 instruments; enough steps for 5,000), one fresh
// page, the replay run at `--speed` steps/s; every `--every` steps a sample:
//   wasm      WASM linear memory (the core reads it: metrics `memory_bytes.wasm.last`)
//   js_heap   JS heap used (Chromium: CDP Performance.getMetrics JSHeapUsedSize)
//   nodes     DOM node count (CDP `Nodes` on Chromium; elements on WebKit)
//   canvases  canvases in the document
// Variants: feed (feed shown, nothing touched), chart (first capsule open to its chart),
// scroll (feed scrolled by wheel all the time), reconnect (the event stream drops every
// `--every` steps: the page starts over with a new core, as after a network drop; a
// backward seek does the same).
//
// Usage: node rounds/RP3/experiments/T4/mem.mjs <qp> [--browser chromium|webkit]
//          [--variant feed] [--steps 5000] [--every 250] [--speed 64] [--port 8480] [--web DIR]
// Prints one JSON line per sample.
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../../..');
const web = path.join(root, 'shells/web');
const { startDemo, settle } = await import(path.join(web, 'tests/lib.mjs'));
const pw = await import(path.join(web, 'node_modules/playwright/index.mjs'));

const argv = process.argv.slice(2);
const qp = argv.shift();
const opt = (k, d) => { const i = argv.indexOf(k); return i >= 0 ? argv[i + 1] : d; };
const browserName = opt('--browser', 'chromium');
const variant = opt('--variant', 'feed');
const steps = Number(opt('--steps', '5000'));
const every = Number(opt('--every', '250'));
const speed = Number(opt('--speed', '64'));
const port = Number(opt('--port', '8480'));

// `--web DIR`: the page served from another directory (e.g. a patched copy of the shell).
const served = opt('--web', web);
const { child, base } = await startDemo(qp, served, port, ['--days', '1000']);
const browser = await pw[browserName].launch();
const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 1 });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
const cdp = browserName === 'chromium' ? await ctx.newCDPSession(page) : null;
if (cdp) await cdp.send('Performance.enable');

const stepOf = async () => {
  const s = JSON.parse(await page.evaluate(() => window.qp.status()));
  return s.step ? Number(String(s.step).split('/')[0]) : 0;
};

async function sample(step, t0) {
  const m = JSON.parse(await page.evaluate(() => window.qp.metrics()));
  const dom = await page.evaluate(() => ({
    elements: document.getElementsByTagName('*').length,
    canvases: document.querySelectorAll('canvas').length,
  }));
  const out = { browser: browserName, variant, step, s: Math.round((Date.now() - t0) / 100) / 10,
    wasm: m.memory_bytes?.wasm?.last ?? null, ...dom };
  if (cdp) {
    const pm = (await cdp.send('Performance.getMetrics')).metrics;
    const g = (n) => pm.find((x) => x.name === n)?.value ?? null;
    out.js_heap = g('JSHeapUsedSize');
    out.js_heap_total = g('JSHeapTotalSize');
    out.nodes = g('Nodes');
  }
  return out;
}

// The speed select gets the wanted speed as an option (its handler hands the value to the
// core as the replay control, as the page's own choices are).
async function setSpeed(n) {
  await page.evaluate((v) => {
    const s = document.getElementById('c_speed');
    const o = document.createElement('option');
    o.value = JSON.stringify({ kind: 'speed', steps_per_s: v });
    o.textContent = `${v}`;
    s.append(o);
    s.value = o.value;
    s.dispatchEvent(new Event('change'));
  }, n);
}

const play = () => page.evaluate(() => document.getElementById('c_play').click());

// (reconnect) The stream as the browser re-opens it after a drop: a new connection with
// the same handlers (the page's own `source` is module-private).
await page.addInitScript(() => {
  const ES = window.EventSource;
  let last = null;
  window.EventSource = function T4EventSource(url, o) { last = new ES(url, o); return last; };
  window.__t4reconnect = () => {
    const url = last.url;
    const { onmessage, onerror } = last;
    last.close();
    last = new ES(url);
    last.onmessage = onmessage;
    last.onerror = onerror;
  };
});

try {
  await page.goto(`${base.replace('localhost', '127.0.0.1')}?x=1`);
  await page.waitForFunction(() => {
    try { return JSON.parse(window.qp.metrics()).open_ms.first_capsule > 0; } catch { return false; }
  }, null, { timeout: 120000, polling: 50 });
  await settle(page, 120000);
  const t0 = Date.now();
  console.log(JSON.stringify(await sample(0, t0)));
  if (variant === 'chart') {
    const id = JSON.parse(await page.evaluate(() => window.qp.feed())).find((r) => r.id)?.id;
    await page.evaluate((i) => { window.qp.openCapsule(i); }, id);
    await settle(page, 60000);
    await page.evaluate(() => window.qp.openTo('chart'));
    await settle(page, 60000);
  }
  await setSpeed(speed);
  await play();
  let next = every;
  let wheel = 0;
  while (true) {
    const st = await stepOf();
    if (st >= next) {
      // Pause while sampling (the step count stays put).
      await play();
      await settle(page, 60000).catch(() => {});
      const at = await stepOf();
      console.log(JSON.stringify(await sample(at, t0)));
      if (at >= steps) break;
      next = at + every;
      if (variant === 'reconnect') {
        // Drop the event stream: EventSource reconnects, the host sends a new start
        // frame and the page builds a new core (shells/web/main.js `start`).
        const before = JSON.parse(await page.evaluate(() => window.qp.metrics())).open_ms.start_frame;
        await page.evaluate(() => window.__t4reconnect());
        await page.waitForFunction((b) => {
          try { return JSON.parse(window.qp.metrics()).open_ms.start_frame !== b; } catch { return false; }
        }, before, { timeout: 120000, polling: 100 });
        await page.waitForFunction(() => { try { return window.qp.ready(); } catch { return false; } },
          null, { timeout: 120000, polling: 100 });
        await settle(page, 120000).catch(() => {});
      }
      await play();
    }
    if (variant === 'scroll') {
      await page.mouse.move(195, 400);
      await page.mouse.wheel(0, (wheel++ >> 3) % 2 ? -300 : 300);
      await page.waitForTimeout(40);
    } else {
      await page.waitForTimeout(100);
    }
  }
  if (errors.length) console.log(JSON.stringify({ errors }));
} finally {
  await browser.close();
  child.kill();
}
