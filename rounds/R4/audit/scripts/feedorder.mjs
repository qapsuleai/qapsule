// R4 denetimi: akış düzeni (elikov) ve hassasiyet seçimi, URL ile kitle verilerek.
// Kullanım: node feedorder.mjs <qp> <shells/web> <port> <query>
import path from 'node:path';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
const [qp, web, port, query] = process.argv.slice(2);
const require = createRequire(path.join(path.resolve(web), 'package.json'));
const { chromium } = require('playwright');
const child = spawn(qp, ['demo', '--web', web, '--port', port, '--synthetic', '--paused'], { stdio: ['ignore', 'pipe', 'inherit'] });
const base = await new Promise((r) => { let a = ''; child.stdout.on('data', (d) => { a += d; const m = a.match(/qp demo: (http:\/\/\S+)/); if (m) r(m[1]); }); });
const b = await chromium.launch();
const page = await (await b.newContext({ viewport: { width: 390, height: 844 } })).newPage();
const settle = async () => { let ok = 0; for (let i = 0; i < 400 && ok < 3; i++) { ok = (await page.evaluate(() => Boolean(window.qp) && window.qp.ready())) ? ok + 1 : 0; await page.waitForTimeout(60); } };
await page.goto(`${new URL(base).origin}/?anim=0&${query}`);
await settle();
await page.fill('#c_date', '120'); await page.click('#c_go'); await settle();
const show = async (label) => {
  const rows = JSON.parse(await page.evaluate(() => window.qp.feed()));
  console.log(`== ${label}: ${rows.filter((r) => r.id).length} capsules; audience ${await page.evaluate(() => window.qp.audience())}`);
  for (const r of rows.slice(0, 14)) console.log(r.header !== undefined ? `  [${r.header}]` : `  ${r.id} ${r.mark || ''}`);
};
await show('start');
await page.click('#tabs button[data-tab="settings"]');
for (const lvl of ['important', 'balanced', 'all', '']) { await page.selectOption('#s_sens', lvl); await settle(); const n = JSON.parse(await page.evaluate(() => window.qp.feed())).filter((r) => r.id).length; console.log(`s_sens=${lvl || 'default'}: ${n} capsules; audience ${await page.evaluate(() => window.qp.audience())}`); }
await b.close(); child.kill();
