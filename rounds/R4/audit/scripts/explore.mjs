// R4 denetimi (VERIFY 6, 10, 11): gösterimi başsız Chromium'da (390×844) bir kullanıcı gibi
// kullanır; sentetik pazar (`qp demo --synthetic --paused`), ekran görüntüleri
// rounds/R4/audit/shots/. Sayfanın bütün istekleri iki yoldan kaydedilir (Playwright
// `request` ve bağlamda `route('**')`); sayfanın aldığı akış çerçeveleri ve gönderdiği
// mesajlar (sayfaya eklenen ölçüm betiğiyle, kabuk koduna dokunmadan) kabuksuz tekrar için
// out/'a yazılır.
// Kullanım: node explore.mjs <qp> <shells/web> <out dir> <port>
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const [qp, web, out, port] = process.argv.slice(2);
const require = createRequire(path.join(path.resolve(web), 'package.json'));
const { chromium } = require('playwright');
const { spawn } = await import('node:child_process');
const shots = path.join(out, 'shots');
fs.mkdirSync(shots, { recursive: true });
const log = [];
const note = (s) => { log.push(s); console.log(s); };

function startDemo(extra = []) {
  const args = ['demo', '--web', web, '--port', String(port), '--synthetic', '--paused', ...extra];
  const child = spawn(qp, args, { stdio: ['ignore', 'pipe', 'pipe'] });
  let all = '';
  child.stderr.on('data', (d) => { all += d; });
  return new Promise((resolve, reject) => {
    child.stdout.on('data', (d) => {
      all += d;
      const m = all.match(/qp demo: (http:\/\/\S+)/);
      if (m) resolve({ child, base: m[1], banner: () => all });
    });
    child.on('exit', (c) => reject(new Error(`qp demo exited ${c}: ${all}`)));
  });
}

// Recorder injected into the page (not part of the shell): wraps EventSource and fetch.
const RECORDER = `
  window.__frames = []; window.__posts = [];
  const ES = window.EventSource;
  window.EventSource = function (u) { const s = new ES(u); window.__frames.push({ open: String(u) });
    s.addEventListener('message', (e) => window.__frames.push({ data: e.data })); return s; };
  window.EventSource.prototype = ES.prototype;
  const F = window.fetch;
  window.fetch = function (u, o) { if (o && o.method === 'POST') window.__posts.push(o.body); return F.apply(this, arguments); };
`;

async function settle(page, ms = 30000) {
  const end = Date.now() + ms;
  let ok = 0;
  while (Date.now() < end) {
    const r = await page.evaluate(() => Boolean(window.qp) && window.qp.ready()).catch(() => false);
    ok = r ? ok + 1 : 0;
    if (ok >= 3) return true;
    await page.waitForTimeout(80);
  }
  return false;
}

const demo = await startDemo();
const origin = new URL(demo.base).origin;
note(`demo: ${demo.base}`);
const browser = await chromium.launch({ args: ['--force-color-profile=srgb', '--font-render-hinting=none'] });
const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, hasTouch: false });
const requests = [];
const routed = [];
await ctx.route('**/*', (route) => { routed.push(route.request().url()); route.continue(); });
const page = await ctx.newPage();
page.on('request', (r) => requests.push(r.url()));
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
page.on('console', (m) => { if (m.type() === 'error' || m.type() === 'warning') errors.push(`${m.type()}: ${m.text()}`); });
await page.addInitScript(RECORDER);

const shot = async (name) => { await page.screenshot({ path: path.join(shots, `${name}.png`) }); note(`shot ${name}`); };
const text = (sel) => page.locator(sel).first().innerText().catch(() => '');
const feedRows = async () => JSON.parse(await page.evaluate(() => window.qp.feed()));
const status = async () => JSON.parse(await page.evaluate(() => window.qp.status()));
const openState = async () => JSON.parse(await page.evaluate(() => window.qp.open()));

const t0 = Date.now();
await page.goto(`${origin}/?anim=0`);
await settle(page);
note(`open+settle ${Date.now() - t0} ms; banner visible=${await page.locator('#banner').isVisible()} text="${await text('#banner')}"`);
note(`status: ${JSON.stringify(await status())}`);
note(`feed rows at start: ${(await feedRows()).length}; empty text visible=${await page.locator('#empty').isVisible()} "${await text('#empty')}"`);
await shot('01-acilis');

// Play for a while, then pause.
await page.click('#c_play');
await page.waitForTimeout(4000);
await page.click('#c_play');
await settle(page);
let rows = await feedRows();
note(`after ~4 s play: step ${(await status()).step}, ${rows.length} feed rows, headers ${rows.filter((r) => r.header !== undefined).map((r) => r.header).join(' | ')}`);
await shot('02-akis-oynatma');

// Step and seek.
await page.click('#c_step');
await settle(page);
note(`after +1: ${JSON.stringify(await status())}`);
await page.fill('#c_date', '120');
await page.click('#c_go');
await settle(page);
rows = await feedRows();
note(`after seek 120: ${JSON.stringify(await status())}; ${rows.length} rows`);
await shot('03-akis-adim120');

// Scroll the feed.
const fb = await page.locator('#feed').boundingBox();
await page.mouse.move(fb.x + 195, fb.y + 300);
for (let i = 0; i < 6; i++) { await page.mouse.wheel(0, 300); await page.waitForTimeout(100); }
await settle(page);
note(`feed scrollTop after wheel: ${await page.evaluate(() => document.getElementById('feed').scrollTop)}`);
await shot('04-akis-kaydirildi');
await page.evaluate(() => { document.getElementById('feed').scrollTop = 0; });
await page.waitForTimeout(300);

// Tap the first capsule face.
rows = await feedRows();
const caps = rows.filter((r) => r.id);
const first = caps[0];
const fc = await page.locator(`#list .face[data-id="${first.id}"] canvas, #list .face[data-id="${first.id}"] p`).first().boundingBox();
await page.mouse.click(fc.x + 150, fc.y + 60);
await settle(page);
let o = await openState();
note(`tap face ${first.id}: depth ${o.depth}; text "${await text('#opentext')}"; hint "${await text('#hint')}"`);
await shot('05-kapsul');

// Down to the chart.
const sb = await page.locator('#scene').boundingBox();
await page.mouse.click(sb.x + sb.width / 2, sb.y + sb.height / 2);
await settle(page);
o = await openState();
note(`tap scene: depth ${o.depth}; hint "${await text('#hint')}"`);
await shot('06-grafik');

// Drag and zoom.
const before = JSON.parse(await page.evaluate(() => window.qp.digests())).open;
await page.mouse.move(sb.x + 200, sb.y + 150);
await page.mouse.down();
await page.mouse.move(sb.x + 290, sb.y + 150, { steps: 6 });
await page.mouse.up();
await settle(page);
const afterDrag = JSON.parse(await page.evaluate(() => window.qp.digests())).open;
await page.mouse.move(sb.x + 250, sb.y + 150);
for (let i = 0; i < 3; i++) { await page.mouse.wheel(0, -120); await page.waitForTimeout(120); }
await settle(page);
const afterZoom = JSON.parse(await page.evaluate(() => window.qp.digests())).open;
note(`drag changed scene: ${JSON.stringify(before) !== JSON.stringify(afterDrag)}; zoom changed scene: ${JSON.stringify(afterDrag) !== JSON.stringify(afterZoom)}`);
await shot('07-grafik-surukle-yakinlastir');

// Evidence (authorized).
await page.click('#toevidence');
await settle(page);
o = await openState();
const ev = await page.locator('#evidence').innerText();
note(`evidence (arkin): depth ${o.depth}; evidence text: ${JSON.stringify(ev.slice(0, 600))}`);
await shot('08-kanit-yetkili');
await page.click('#back'); await settle(page);
note(`back -> ${(await openState()).depth}`);
await page.click('#back'); await settle(page);
note(`back -> ${(await openState()).depth}`);
await page.click('#back'); await settle(page);
note(`back -> ${(await openState()).depth}`);

// Settings: sensitivity.
await page.click('#tabs button[data-tab="settings"]');
await page.waitForTimeout(300);
await shot('09-ayarlar');
const opts = await page.evaluate(() => Object.fromEntries(['s_face', 's_alpha', 's_feed', 's_transition', 's_outcome'].map((id) => [id, [...document.getElementById(id).options].map((x) => x.value), ])));
note(`settings options: ${JSON.stringify(opts)}`);
const nBefore = (await feedRows()).filter((r) => r.id).length;
for (const lvl of ['important', 'balanced', 'all']) {
  await page.selectOption('#s_sens', lvl);
  await settle(page);
  note(`sensitivity ${lvl}: ${(await feedRows()).filter((r) => r.id).length} capsules (before ${nBefore})`);
}
// Watch list and positions.
await page.selectOption('#s_sens', 'important');
await settle(page);
const nImp = (await feedRows()).filter((r) => r.id).length;
const trs = page.locator('#instruments tbody tr');
note(`instruments listed: ${await trs.count()}`);
await trs.nth(0).locator('input').check();
await trs.nth(1).locator('select').selectOption('long');
await trs.nth(2).locator('select').selectOption('short');
await page.click('#s_apply');
await settle(page);
note(`audience after apply: ${await page.evaluate(() => window.qp.audience())}`);
rows = await feedRows();
note(`important + watch/positions: ${rows.filter((r) => r.id).length} capsules (was ${nImp}); headers ${rows.filter((r) => r.header !== undefined).map((r) => r.header).join(' | ')}`);
await page.click('#s_seen');
await settle(page);
note(`toast after last look: "${await text('#toast')}"`);
await page.click('#tabs button[data-tab="feed"]');
await page.waitForTimeout(300);
await shot('10-akis-izleme-pozisyon');
// play a bit after last look, check the "since last look" header
await page.click('#c_play'); await page.waitForTimeout(2500); await page.click('#c_play'); await settle(page);
rows = await feedRows();
note(`after last look + play: headers ${rows.filter((r) => r.header !== undefined).map((r) => r.header).join(' | ')}`);
await shot('11-son-bakis-sonrasi');

// Theme light, alphabet rozet, face fotolay.
await page.click('#tabs button[data-tab="settings"]');
await page.selectOption('#s_theme', 'light'); await settle(page);
await page.selectOption('#s_alpha', 'rozet'); await settle(page);
await page.click('#tabs button[data-tab="feed"]'); await page.waitForTimeout(300);
await shot('12-acik-rozet');
await page.click('#tabs button[data-tab="settings"]');
await page.selectOption('#s_face', 'fotolay'); await settle(page);
await page.click('#tabs button[data-tab="feed"]'); await page.waitForTimeout(300);
await shot('13-acik-fotolay');
note(`look after style: ${await page.evaluate(() => window.qp.look())}`);

// Language EN (reconnects).
const openBefore = await openState();
await page.click('#tabs button[data-tab="settings"]');
await page.selectOption('#s_lang', 'en');
await page.waitForTimeout(500);
await settle(page);
await page.click('#tabs button[data-tab="feed"]'); await page.waitForTimeout(300);
note(`EN: status ${JSON.stringify(await status())}; look ${await page.evaluate(() => window.qp.look())}; audience ${await page.evaluate(() => window.qp.audience())}; rows ${(await feedRows()).length}`);
await shot('14-en');

// Metrics.
await page.click('#tabs button[data-tab="metrics"]');
await page.waitForTimeout(1500);
await shot('15-olcum');
const metrics = JSON.parse(await page.evaluate(() => window.qp.metrics()));
fs.writeFileSync(path.join(out, 'metrics-desktop.json'), JSON.stringify(metrics, null, 2));
await page.click('#tabs button[data-tab="feed"]');

// Record the frames and posts of this whole session.
fs.writeFileSync(path.join(out, 'frames-explore.json'), JSON.stringify(await page.evaluate(() => window.__frames)));
fs.writeFileSync(path.join(out, 'posts-explore.json'), JSON.stringify(await page.evaluate(() => window.__posts)));

// Unentitled reader (visitor).
await page.click('#tabs button[data-tab="settings"]');
await page.check('#s_noent');
await page.waitForTimeout(500);
await settle(page);
await page.click('#tabs button[data-tab="feed"]');
await page.waitForTimeout(400);
await page.fill('#c_date', '120'); await page.click('#c_go'); await settle(page);
rows = await feedRows();
note(`visitor: ${rows.length} rows; first texts: ${JSON.stringify(rows.filter((r) => r.id).slice(0, 3).map((r) => r.text || '(scene)'))}`);
await shot('16-yetkisiz-akis');
const v = rows.filter((r) => r.id)[0];
if (v) {
  await page.evaluate((id) => window.qp.openCapsule(id), v.id);
  await settle(page);
  await page.evaluate(() => window.qp.openTo('evidence'));
  await settle(page);
  note(`visitor evidence: depth ${(await openState()).depth}; text ${JSON.stringify((await page.locator('#evidence').innerText()).slice(0, 500))}`);
  await shot('17-yetkisiz-kanit');
}

note(`requests (page.on): ${requests.length}, (route): ${routed.length}`);
const foreign = [...new Set([...requests, ...routed])].filter((u) => new URL(u).origin !== origin);
note(`foreign: ${JSON.stringify(foreign)}`);
note(`distinct paths: ${JSON.stringify([...new Set([...requests, ...routed].map((u) => new URL(u).pathname))])}`);
note(`page errors/warnings: ${JSON.stringify(errors)}`);
fs.writeFileSync(path.join(out, 'explore.log'), log.join('\n') + '\n');
await browser.close();
demo.child.kill();
