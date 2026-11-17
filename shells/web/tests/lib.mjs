// Shared helpers of the web shell's headless tests (R4/T5).
import { spawn } from 'node:child_process';
import { chromium } from 'playwright';

export const FACE_W = 390;

/** Starts `qp demo` (synthetic unless args say otherwise) and resolves with its base URL. */
export function startDemo(qp, web, port, extra = []) {
  // (RP3/M15) QP_DEMO_PORT: every demo on this port (agents sharing a machine use their
  // own port range; the tests start one demo at a time).
  if (process.env.QP_DEMO_PORT) port = process.env.QP_DEMO_PORT;
  const data = extra.includes('--data') ? [] : ['--synthetic'];
  const args = ['demo', '--web', web, '--port', String(port), ...data, '--paused', ...extra];
  const child = spawn(qp, args, { stdio: ['ignore', 'pipe', 'inherit'] });
  return new Promise((resolve, reject) => {
    let out = '';
    child.stdout.on('data', (d) => {
      out += d;
      const m = out.match(/qp demo: (http:\/\/\S+)/);
      if (m) resolve({ child, base: m[1] });
    });
    child.on('exit', (c) => reject(new Error(`qp demo exited (${c}): ${out}`)));
  });
}

/** A phone-sized page with every request recorded. */
export async function phonePage(browser) {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  const requests = [];
  page.on('request', (r) => requests.push(r.url()));
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  return { ctx, page, requests, errors };
}

/** Waits until the shell is idle (nothing outstanding) twice in a row. */
export async function settle(page, ms = 30000) {
  const end = Date.now() + ms;
  let ok = 0;
  while (Date.now() < end) {
    const r = await page.evaluate(() => Boolean(window.qp) && window.qp.ready());
    ok = r ? ok + 1 : 0;
    if (ok >= 3) return;
    await page.waitForTimeout(60);
  }
  throw new Error('page did not settle');
}

export async function launch() {
  return chromium.launch({ args: ['--force-color-profile=srgb', '--font-render-hinting=none'] });
}

/** Clicks at (x, y) inside the element matched by `sel`. */
export async function clickIn(page, sel, x, y) {
  const b = await page.locator(sel).first().boundingBox();
  await page.mouse.click(b.x + x, b.y + y);
}
