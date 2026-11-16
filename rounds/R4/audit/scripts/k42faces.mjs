// R4 denetimi (VERIFY 11, 15): seçilen kapsüllerin yüzü TR/EN × koyu/açık, sentetik, adım 120;
// ayrıca geri sarma (VERIFY 7 ek): adım 120 → 40 → +1, sayfa toparlanıyor mu.
// Kullanım: node k42faces.mjs <qp> <shells/web> <port> <out dir> <id,id,...|first6>
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
const [qp, web, port, out, ids] = process.argv.slice(2);
fs.mkdirSync(out, { recursive: true });
const require = createRequire(path.join(path.resolve(web), 'package.json'));
const { chromium } = require('playwright');
const child = spawn(qp, ['demo', '--web', web, '--port', port, '--synthetic', '--paused'], { stdio: ['ignore', 'pipe', 'inherit'] });
const base = await new Promise((r) => { let a = ''; child.stdout.on('data', (d) => { a += d; const m = a.match(/qp demo: (http:\/\/\S+)/); if (m) r(m[1]); }); });
const origin = new URL(base).origin;
const b = await chromium.launch({ args: ['--force-color-profile=srgb'] });
for (const lang of ['tr', 'en']) for (const theme of ['dark', 'light']) {
  const page = await (await b.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 1 })).newPage();
  const settle = async () => { let ok = 0; for (let i = 0; i < 500 && ok < 3; i++) { ok = (await page.evaluate(() => Boolean(window.qp) && window.qp.ready())) ? ok + 1 : 0; await page.waitForTimeout(60); } };
  await page.goto(`${origin}/?anim=0&lang=${lang}&theme=${theme}`);
  await settle();
  await page.fill('#c_date', '120'); await page.click('#c_go'); await settle();
  const list = ids === 'first6' ? JSON.parse(await page.evaluate(() => window.qp.feed())).filter((r) => r.id && r.scene).slice(0, 6).map((r) => r.id) : ids.split(',');
  for (const id of list) {
    const el = page.locator(`#list .face[data-id="${id}"] canvas`);
    if (!(await el.count())) { console.log(`${lang} ${theme} ${id}: not in feed`); continue; }
    await el.scrollIntoViewIfNeeded(); await page.waitForTimeout(200); await settle();
    await el.screenshot({ path: path.join(out, `${id}.${lang}-${theme}.png`) });
    await page.evaluate((x) => window.qp.openCapsule(x), id); await settle();
    console.log(`${lang} ${theme} ${id}: detail "${JSON.parse(await page.evaluate(() => window.qp.open())).text}"`);
    await page.click('#back'); await settle();
  }
  if (lang === 'tr' && theme === 'dark') {
    await page.fill('#c_date', '40'); await page.click('#c_go'); await settle();
    const s1 = JSON.parse(await page.evaluate(() => window.qp.status()));
    const n1 = JSON.parse(await page.evaluate(() => window.qp.feed())).filter((r) => r.id).length;
    await page.click('#c_step'); await settle();
    const s2 = JSON.parse(await page.evaluate(() => window.qp.status()));
    console.log(`backward seek 120→40: status step ${s1.step} epoch ${s1.epoch}, ${n1} capsules; after +1: step ${s2.step} epoch ${s2.epoch}; toast "${await page.locator('#toast').innerText().catch(() => '')}"`);
    await page.screenshot({ path: path.join(out, 'backward-seek.png') });
  }
  await page.context().close();
}
await b.close(); child.kill();
