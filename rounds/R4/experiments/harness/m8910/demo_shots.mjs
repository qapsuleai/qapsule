// R4/H-M8910: screenshots of the live demo (`qp demo --synthetic --paused`) for M9 and
// M10, in headless Chromium at the phone viewport (390×844 CSS px, DPR 2), animations off.
//   node demo_shots.mjs <plan.json>        (plans are written by `m8910 m9` / `m8910 m10`)
// The page is the web shell as a user has it: settings by URL (feed layout / transition,
// language, theme), the replay moved with the control bar, the audience set on the settings
// page, capsules opened by the core's `open` (what a face tap sends) and by tapping the
// scene. Labels (M9 card labels, M10 chart regions) are the harness's overlays, never the
// shell's code. Same plan, same machine → byte-identical PNGs.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { startDemo, settle, launch, clickIn } from '../../../../../shells/web/tests/lib.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const web = path.resolve(here, '../../../../../shells/web');
const plan = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
fs.mkdirSync(plan.out, { recursive: true });

const LABEL_CSS = `.hl{position:absolute;right:6px;top:34px;z-index:2;min-width:22px;height:22px;padding:0 5px;
  border-radius:11px;background:#111;color:#fff;font:700 13px/22px system-ui,sans-serif;text-align:center;
  box-shadow:0 0 0 2px #fff}
  .rl{position:fixed;z-index:30;width:24px;height:24px;margin:-12px 0 0 -12px;border-radius:12px;
  background:#111;color:#fff;font:700 14px/24px system-ui,sans-serif;text-align:center;box-shadow:0 0 0 2px #fff;
  pointer-events:none}`;

async function seek(page, step) {
  await page.fill('#c_date', String(step));
  await page.click('#c_go');
  await settle(page);
}

async function shot(page, file) {
  await page.evaluate(() => document.activeElement && document.activeElement.blur());
  await page.waitForTimeout(120);
  await settle(page);
  await page.locator('#phone').screenshot({ path: path.join(plan.out, file), animations: 'disabled', caret: 'hide' });
}

async function m9(page) {
  await seek(page, plan.t0_step);
  // The audience, on the settings page as a user sets it.
  await page.click('#tabs button[data-tab="settings"]');
  await page.selectOption('#s_group', plan.choice.group);
  await page.selectOption('#s_sens', plan.choice.sensitivity);
  await settle(page);
  for (const s of plan.choice.watch) await page.check(`#instruments tr[data-sym="${s}"] input`);
  for (const s of plan.choice.long) await page.selectOption(`#instruments tr[data-sym="${s}"] select`, 'long');
  for (const s of plan.choice.short) await page.selectOption(`#instruments tr[data-sym="${s}"] select`, 'short');
  await page.click('#s_apply');
  await settle(page);
  await page.evaluate(() => window.qp.seen());
  await settle(page);
  await seek(page, plan.t1_step);
  await page.click('#tabs button[data-tab="feed"]');
  await page.evaluate(() => window.qp.repaint());
  await settle(page);
  await page.evaluate((labels) => {
    for (const el of document.querySelectorAll('#list .face[data-id]')) {
      const l = labels[el.dataset.id];
      if (!l || el.querySelector('.hl')) continue;
      const s = document.createElement('span');
      s.className = 'hl';
      s.textContent = l;
      el.append(s);
    }
  }, plan.labels);
  const { h, total } = await page.$eval('#feed', (e) => ({ h: e.clientHeight, total: e.scrollHeight }));
  const n = Math.max(1, Math.min(plan.max_screens, Math.ceil(total / h)));
  const seen = {};
  for (let k = 0; k < n; k++) {
    await page.$eval('#feed', (e, y) => { e.scrollTop = y; }, k * h);
    await page.waitForTimeout(200);
    await page.evaluate(() => window.qp.repaint());
    await settle(page);
    const vis = await page.evaluate(() => {
      const f = document.getElementById('feed').getBoundingClientRect();
      const out = [];
      for (const el of document.querySelectorAll('#list .face')) {
        const r = el.getBoundingClientRect();
        const ov = Math.min(r.bottom, f.bottom) - Math.max(r.top, f.top);
        const l = el.querySelector('.hl');
        if (l && ov >= r.height / 2) out.push(l.textContent);
      }
      return out;
    });
    for (const l of vis) (seen[l] ||= []).push(k + 1);
    await shot(page, `${plan.code}-${k + 1}.png`);
  }
  fs.writeFileSync(plan.positions, `${JSON.stringify({ screens: n, labels: seen }, null, 2)}\n`);
}

async function m10(page) {
  for (const st of plan.states) {
    await seek(page, st.step);
    await page.evaluate((id) => window.qp.openCapsule(id), st.capsule);
    await settle(page);
    await shot(page, `${plan.code}-${st.i}-1.png`);
    await clickIn(page, '#scene', plan.tap[0], plan.tap[1]);
    await settle(page);
    const depth = await page.evaluate(() => JSON.parse(window.qp.open()).depth);
    if (depth !== 'chart') throw new Error(`state ${st.i}: tap opened ${depth}`);
    for (const [j, p] of plan.frames.entries()) {
      const n = await page.evaluate((x) => window.qp.freeze(x), p);
      if (!n) console.warn(`state ${st.i}: frozen frame has no element`);
      await shot(page, `${plan.code}-${st.i}-${j + 2}.png`);
      await page.evaluate(() => window.qp.unfreeze());
    }
    await page.evaluate((regions) => {
      const r = document.getElementById('scene').getBoundingClientRect();
      for (const g of regions) {
        const d = document.createElement('div');
        d.className = 'rl';
        d.textContent = g.label;
        d.style.left = `${r.left + g.x}px`;
        d.style.top = `${r.top + g.y}px`;
        document.body.append(d);
      }
    }, st.regions);
    await shot(page, `${plan.code}-${st.i}-4.png`);
    await page.evaluate(() => { for (const d of document.querySelectorAll('.rl')) d.remove(); });
    await page.click('#back');
    await settle(page);
    await page.click('#back');
    await settle(page);
  }
}

const { child, base } = await startDemo(plan.qp, web, plan.port);
const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
const browser = await launch();
try {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2 });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  // (R4/S2) The presentation now opens with its own defaults (face `vadrel`, feed `elikov`,
  // transition `zumlak`, outcome `tg5`); the M9/M10 screens were taken on the R3 designs,
  // so every design setting a plan does not give is pinned to `r3` (a re-run looks the same).
  const R3 = { face: 'r3', feed: 'r3', transition: 'r3', outcome: 'r3' };
  const q = new URLSearchParams({ anim: '0', ...R3, ...plan.settings });
  await page.goto(`${origin}/?${q}`);
  await settle(page);
  await page.addStyleTag({ content: LABEL_CSS });
  if (plan.kind === 'm9') await m9(page);
  else await m10(page);
  if (errors.length) throw new Error(`page errors: ${errors}`);
} finally {
  await browser.close();
  child.kill();
}
console.log(`demo_shots: ${plan.kind} ${plan.code} done`);
