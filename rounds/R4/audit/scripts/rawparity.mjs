// R4 denetimi (VERIFY 8, kendi yöntemim): kabuksuz eşitlik, `qp drive` kullanmadan.
// A) Başsız Chromium'da gösterim (sentetik, duraklatılmış) belirleyici bir kullanıcı
//    oturumuyla sürülür (adım/tarihe git, kapsüle dokunma, grafiğe inme, sürükleme,
//    tekerlek, kanıt, geri, hassasiyet, izleme listesi/pozisyon, tema, alfabe, yüz). Sayfaya
//    eklenen kayıt betiği (kabuk koduna dokunmaz) sayfanın aldığı her akış çerçevesini ve
//    gönderdiği her POST'u (adres + gövde) kaydeder; sonunda sayfanın özetleri (qp.digests).
// B) Taze bir `qp demo` ile, tarayıcısız ve WASM'sız düz bir HTTP istemcisi (bu dosya) aynı
//    POST gövdelerini aynı sırayla gönderir (her birinden sonra akış durulana kadar bekler) ve
//    aldığı çerçeveleri kaydeder.
// Karşılaştırma: iki çerçeve akışı, start çerçevesindeki bağlantı belirteci dışında bayt
// bayt aynı mı. Ayrıca B'nin aldığı son `capsules` ve `screen` cevapları A'nınkilerle aynı mı.
// Kullanım: node rawparity.mjs <qp> <shells/web> <out dir> <portA> <portB>
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';

const [qp, web, out, portA, portB] = process.argv.slice(2);
const require = createRequire(path.join(path.resolve(web), 'package.json'));
const { chromium } = require('playwright');
fs.mkdirSync(out, { recursive: true });

function startDemo(port) {
  const child = spawn(qp, ['demo', '--web', web, '--port', String(port), '--synthetic', '--paused'], { stdio: ['ignore', 'pipe', 'inherit'] });
  let all = '';
  return new Promise((resolve, reject) => {
    child.stdout.on('data', (d) => { all += d; const m = all.match(/qp demo: (http:\/\/\S+)/); if (m) resolve({ child, base: m[1] }); });
    child.on('exit', (c) => reject(new Error(`qp demo exited ${c}`)));
  });
}
const RECORDER = `
  window.__frames = []; window.__posts = [];
  const ES = window.EventSource;
  window.EventSource = function (u) { const s = new ES(u); window.__frames.push({ open: String(u) });
    s.addEventListener('message', (e) => window.__frames.push({ data: e.data })); return s; };
  window.EventSource.prototype = ES.prototype;
  const F = window.fetch;
  window.fetch = function (u, o) { if (o && o.method === 'POST') window.__posts.push({ u: String(u), body: o.body }); return F.apply(this, arguments); };
`;
async function settle(page, ms = 30000) {
  const end = Date.now() + ms; let ok = 0;
  while (Date.now() < end) {
    const r = await page.evaluate(() => Boolean(window.qp) && window.qp.ready()).catch(() => false);
    ok = r ? ok + 1 : 0; if (ok >= 3) return; await page.waitForTimeout(80);
  }
  throw new Error('no settle');
}

// --- A: browser -------------------------------------------------------------------
const A = await startDemo(portA);
const originA = new URL(A.base).origin;
const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 1 });
const page = await ctx.newPage();
await page.addInitScript(RECORDER);
await page.goto(`${originA}/?anim=0`);
await settle(page);
const digests = {};
const dg = async (k) => { await settle(page); digests[k] = JSON.parse(await page.evaluate(() => window.qp.digests())); };
await page.fill('#c_date', '90'); await page.click('#c_go'); await settle(page);
await page.click('#c_step'); await settle(page);
await page.click('#c_step'); await settle(page);
await dg('feed');
const rows = JSON.parse(await page.evaluate(() => window.qp.feed())).filter((r) => r.id && r.scene);
const target = rows[Math.min(2, rows.length - 1)]; // the third capsule (the shipped test taps the first)
const fb = await page.locator(`#list .face[data-id="${target.id}"] canvas`).boundingBox();
await page.mouse.click(fb.x + 200, fb.y + 50); await settle(page);
await dg('capsule');
const sb = await page.locator('#scene').boundingBox();
await page.mouse.click(sb.x + 100, sb.y + 120); await settle(page);
await dg('chart');
await page.mouse.move(sb.x + 250, sb.y + 140); await page.mouse.down();
await page.mouse.move(sb.x + 150, sb.y + 140, { steps: 4 }); await page.mouse.up(); await settle(page);
await page.mouse.move(sb.x + 120, sb.y + 140);
await page.mouse.wheel(0, 100); await settle(page);
await page.mouse.wheel(0, -200); await settle(page);
await dg('moved');
await page.click('#toevidence'); await settle(page);
await dg('evidence');
await page.click('#back'); await settle(page);
await page.click('#back'); await settle(page);
await page.click('#back'); await settle(page);
await page.click('#tabs button[data-tab="settings"]');
await page.selectOption('#s_sens', 'balanced'); await settle(page);
const trs = page.locator('#instruments tbody tr');
await trs.nth(4).locator('input').check();
await trs.nth(5).locator('select').selectOption('short');
await page.selectOption('#s_group', 'future');
await page.click('#s_apply'); await settle(page);
await page.selectOption('#s_theme', 'light'); await settle(page);
await page.selectOption('#s_alpha', 'rozet'); await settle(page);
await page.selectOption('#s_face', 'fotolay'); await settle(page);
await page.click('#tabs button[data-tab="feed"]');
await page.fill('#c_date', '130'); await page.click('#c_go'); await settle(page);
await dg('end');
const framesA = await page.evaluate(() => window.__frames);
const postsA = await page.evaluate(() => window.__posts);
await browser.close();
A.child.kill();
fs.writeFileSync(path.join(out, 'A-frames.json'), JSON.stringify(framesA));
fs.writeFileSync(path.join(out, 'A-posts.json'), JSON.stringify(postsA));
fs.writeFileSync(path.join(out, 'A-digests.json'), JSON.stringify(digests, null, 2));

// --- B: raw client ----------------------------------------------------------------
const B = await startDemo(portB);
const baseB = new URL(B.base).origin;
const framesB = [];
let lastFrame = Date.now();
const openUrl = new URL(framesA.find((f) => f.open).open);
const evUrl = `${baseB}/events${openUrl.search}`;
framesB.push({ open: evUrl });
let startFrame = null;
await new Promise((resolve) => {
  http.get(evUrl, { headers: { Accept: 'text/event-stream' } }, (res) => {
    let buf = '';
    res.setEncoding('utf8');
    res.on('data', (d) => {
      buf += d;
      let i;
      while ((i = buf.indexOf('\n\n')) >= 0) {
        const ev = buf.slice(0, i); buf = buf.slice(i + 2);
        const data = ev.split('\n').filter((l) => l.startsWith('data:')).map((l) => l.slice(5).replace(/^ /, '')).join('\n');
        if (!data) continue;
        framesB.push({ data }); lastFrame = Date.now();
        if (!startFrame && data.startsWith('{"start"')) { startFrame = JSON.parse(data).start; resolve(); }
      }
    });
  });
});
const quiet = async (ms = 400) => { while (Date.now() - lastFrame < ms) await new Promise((r) => setTimeout(r, 50)); };
await quiet();
function post(body) {
  return new Promise((resolve, reject) => {
    const u = new URL(`${baseB}/send`); u.searchParams.set('conn', startFrame.conn);
    const req = http.request(u, { method: 'POST', headers: { 'Content-Type': 'application/json', 'X-Qp-Token': startFrame.token, Origin: baseB } }, (res) => { res.resume(); res.on('end', () => resolve(res.statusCode)); });
    req.on('error', reject); req.end(body);
  });
}
const codes = [];
for (const p of postsA) { lastFrame = Date.now(); codes.push(await post(p.body)); await quiet(250); }
await quiet(800);
B.child.kill();
fs.writeFileSync(path.join(out, 'B-frames.json'), JSON.stringify(framesB));

// --- compare ------------------------------------------------------------------------
const norm = (fs_) => fs_.filter((f) => f.data).map((f) => (f.data.startsWith('{"start"')
  ? JSON.stringify({ ...JSON.parse(f.data).start, token: '<t>', info: { ...JSON.parse(f.data).start.info, urls: '<u>', phone: '<p>' } })
  : f.data));
const a = norm(framesA), b = norm(framesB);
let firstDiff = -1;
for (let i = 0; i < Math.max(a.length, b.length); i++) if (a[i] !== b[i]) { firstDiff = i; break; }
const res = {
  posts: postsA.length, post_status: [...new Set(codes)],
  frames_browser: a.length, frames_raw: b.length,
  bytes_browser: a.join('\n').length, bytes_raw: b.join('\n').length,
  identical: firstDiff < 0, first_diff: firstDiff,
  diff_a: firstDiff >= 0 ? (a[firstDiff] || '').slice(0, 300) : null,
  diff_b: firstDiff >= 0 ? (b[firstDiff] || '').slice(0, 300) : null,
};
fs.writeFileSync(path.join(out, 'result.json'), JSON.stringify(res, null, 2));
console.log(JSON.stringify(res, null, 2));
process.exit(0);
