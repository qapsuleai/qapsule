// RP4/T6: one headless viewer on the demo for `secs`; opens the first capsule after `open_after` s.
import { createRequire } from 'node:module';
const require = createRequire(new URL('../../../../shells/web/package.json', import.meta.url));
const { chromium } = require('playwright');
const [base, secsArg, openAfterArg, profDir] = process.argv.slice(2);
const secs = Number(secsArg), openAfter = Number(openAfterArg);
const ctx = await chromium.launchPersistentContext(profDir, { headless: true, viewport: { width: 390, height: 844 }, deviceScaleFactor: 3 });
const page = await ctx.newPage();
const errors = [];
page.on('pageerror', (e) => errors.push(String(e).slice(0, 200)));
await page.goto(`${base}/?anim=0`, { timeout: 600000 });
await page.waitForFunction(() => window.qp && document.querySelectorAll('#list .face').length > 0, null, { timeout: 600000 });
console.log(JSON.stringify({ t: Date.now(), ev: 'feed' }));
await page.waitForTimeout(openAfter * 1000);
const first = await page.evaluate(() => { const f = JSON.parse(window.qp.feed()).filter((x) => x.id); return f.length ? f[0].id : null; });
if (first) { await page.evaluate((id) => window.qp.openCapsule(id), first); }
console.log(JSON.stringify({ t: Date.now(), ev: 'open', ok: Boolean(first) }));
await page.waitForTimeout(Math.max(0, secs - openAfter) * 1000);
console.log(JSON.stringify({ t: Date.now(), ev: 'end', errors }));
await ctx.close();
