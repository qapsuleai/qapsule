// R5/G: the comparison gallery in headless Chromium at phone size (402 CSS px; VERIFY 15, 18).
//  - `qp demo --gallery` serves /gallery.html; every request stays on the page's origin;
//  - the topics voice, look, feed_order and feed_rule are shown, each with at least two
//    candidates whose content (replay moment, open capsule, depth) is the same — read from
//    the page (data-content, as the core gave it) and from the core's own view — and whose
//    images loaded;
//  - a pick and a note are made; "Copy" puts the expected summary on the clipboard (the
//    expected text is written here from the layout; the date comes from `?now=`, the device
//    line from a fixed iPhone user agent); a second pick in the same topic replaces the first;
//  - picks and notes survive a reload (local storage); `?pick=` picks; without the
//    Clipboard API the summary box is selected instead;
//  - the theme and the language can be switched (the chrome's scheme and the images follow).
// Usage: node tests/gallery.mjs <qp binary>
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { startDemo, launch } from './lib.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const web = path.resolve(here, '..');
const [qp] = process.argv.slice(2);
const { child, base } = await startDemo(qp, web, 8397, ['--gallery']);
const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
const browser = await launch();
const UA = 'Mozilla/5.0 (iPhone; CPU iPhone OS 27_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.0 Mobile/15E148 Safari/604.1';
const NOW = '1763460000000'; // 2025-11-18 13:00 in İstanbul
let failed = 0;
const check = (ok, what) => { console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`); if (!ok) failed++; };

async function phone(perms = true) {
  const ctx = await browser.newContext({ viewport: { width: 402, height: 874 }, deviceScaleFactor: 1, userAgent: UA });
  if (perms) await ctx.grantPermissions(['clipboard-read', 'clipboard-write'], { origin });
  const page = await ctx.newPage();
  const requests = [];
  page.on('request', (r) => requests.push(r.url()));
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  return { ctx, page, requests, errors };
}

async function ready(page) {
  await page.waitForSelector('body[data-ready="1"]', { timeout: 120000 });
  await page.waitForFunction(() => [...document.images].every((i) => i.complete), null, { timeout: 60000 });
}

const topics = (page) => page.$$eval('section.topic', (ss) => ss.map((s) => ({
  key: s.dataset.topic,
  cands: [...s.querySelectorAll('article.cand')].map((a) => ({
    key: a.dataset.key, content: a.dataset.content,
    imgs: [...a.querySelectorAll('img')].map((i) => ({ src: i.getAttribute('src'), ok: i.naturalWidth > 0 })),
  })),
})));

try {
  let expected = '';
  let layout = null;
  {
    const { ctx, page, requests, errors } = await phone();
    await page.goto(`${origin}/gallery.html?now=${NOW}`);
    await ready(page);
    const ts = await topics(page);
    const view = JSON.parse(await page.evaluate(() => window.gallery.view()));
    layout = view;
    const keys = ts.map((t) => t.key);
    check(['voice', 'look', 'feed_order', 'feed_rule'].every((k) => keys.includes(k)), `topics: ${keys.join(' ')}`);
    for (const t of ts) {
      const v = view.topics.find((x) => x.key === t.key);
      const same = t.cands.every((c) => c.content === t.cands[0].content);
      const fromCore = v && v.cands.every((c, i) => t.cands[i] && c.content === t.cands[i].content);
      check(t.cands.length >= 2 && same && fromCore,
        `${t.key}: ${t.cands.length} candidates, same content (${t.cands[0] && t.cands[0].content})`);
      const imgs = t.cands.flatMap((c) => c.imgs);
      check(imgs.length > 0 && imgs.every((i) => i.ok), `${t.key}: ${imgs.length} images loaded`);
    }
    // The phone width: nothing wider than the 402 px viewport.
    const sw = await page.evaluate(() => document.documentElement.scrollWidth);
    check(sw <= 402, `no horizontal scroll (scrollWidth ${sw})`);
    // A pick and a note; then another pick in the same topic replaces the first.
    const voice = view.topics.find((t) => t.key === 'voice');
    const look = view.topics.find((t) => t.key === 'look');
    const [v0, v1] = voice.cands;
    const sel = (k) => `article.cand[data-key="${k}"]`;
    await page.click(`${sel(v0.key)} button.pick`);
    await page.click(`${sel(v1.key)} button.pick`);
    await page.fill(`${sel(v1.key)} input.note`, 'kısa ve\nnet  ');
    await page.click(`${sel(look.cands[0].key)} button.pick`);
    await page.fill(`${sel(look.cands[1].key)} input.note`, 'soluk');
    const pressed = await page.$$eval('button.pick[aria-pressed="true"]', (bs) => bs.map((b) => b.closest('article').dataset.key));
    check(JSON.stringify(pressed) === JSON.stringify([v1.key, look.cands[0].key]), `one pick per topic (${pressed.join(', ')})`);
    // The expected summary, from the layout.
    const picks = { voice: `${v1.id} (not: kısa ve net)`, look: look.cands[0].id };
    const line = ['R5 galeri', ...view.topics.map((t) => `${t.key}=${picks[t.key] || '—'}`)].join(' · ');
    expected = `${line}\nnotlar: look/${look.cands[1].id}: soluk\nan: ${view.moment} · tr · koyu\n`
      + 'cihaz: iPhone iOS 27.0 · Safari · 402×874 @1 · tarih: 2025-11-18 13:00 (İstanbul)';
    await page.click('#gcopy');
    await page.waitForFunction(() => document.getElementById('gcopied').textContent !== '');
    const clip = await page.evaluate(() => navigator.clipboard.readText());
    check(clip === expected, `copied summary = expected\n${clip}`);
    check(await page.$eval('#gsummary', (e) => e.value) === expected, 'summary box shows it');
    check((await page.$eval('#gshare', (e) => e.getAttribute('href'))) === `gallery.html?pick=look.${look.cands[0].id},voice.${v1.id}`, 'share link carries the picks');
    // Reload: kept.
    await page.reload();
    await ready(page);
    check(await page.$eval('#gsummary', (e) => e.value) === expected, 'picks and notes survive a reload');
    check(await page.$eval(`${sel(v1.key)} input.note`, (e) => e.value) === 'kısa ve net', 'note shown after reload');
    // Theme and language.
    await page.selectOption('#gtheme', 'light');
    await ready(page);
    const scheme = await page.evaluate(() => document.documentElement.style.getPropertyValue('--qp-scheme'));
    const srcs = await page.$$eval('img.shot', (is) => is.map((i) => i.getAttribute('src')));
    check(scheme === 'light' && srcs.length > 0 && srcs.every((s) => s.includes('/tr-light-')), `light theme (${scheme}, ${srcs[0]})`);
    await page.click('#glang');
    await ready(page);
    const title = await page.$eval('#gtitle', (e) => e.textContent);
    const s2 = await page.$eval('#gsummary', (e) => e.value);
    check(title === 'R5 gallery' && s2.startsWith(`R5 gallery · voice=${v1.id} (note: kısa ve net)`), `English (${title}; ${s2.split('\n')[0]})`);
    check(errors.length === 0, `no page errors ${errors.join(' | ')}`);
    const foreign = requests.filter((u) => new URL(u).origin !== origin);
    check(foreign.length === 0, `no request outside the origin (${requests.length} requests) ${foreign.join(' ')}`);
    await ctx.close();
  }
  // `?pick=` and the copy fallback without the Clipboard API.
  {
    const { ctx, page, requests } = await phone(false);
    await page.addInitScript(() => { Object.defineProperty(navigator, 'clipboard', { value: undefined }); });
    const v1 = layout.topics.find((t) => t.key === 'voice').cands[1];
    await page.goto(`${origin}/gallery.html?now=${NOW}&pick=voice.${v1.id},nope.x`);
    await ready(page);
    const pressed = await page.$$eval('button.pick[aria-pressed="true"]', (bs) => bs.map((b) => b.closest('article').dataset.key));
    check(JSON.stringify(pressed) === JSON.stringify([v1.key]), `?pick= picks (${pressed.join(', ')})`);
    await page.click('#gcopy');
    await page.waitForFunction(() => document.getElementById('gcopied').textContent !== '');
    const sel = await page.evaluate(() => {
      const b = document.getElementById('gsummary');
      return document.activeElement === b && b.selectionEnd > b.selectionStart;
    });
    check(sel, 'without the Clipboard API the summary is selected');
    check(requests.every((u) => new URL(u).origin === origin), 'no request outside the origin');
    await ctx.close();
  }
} catch (e) {
  console.log(`FAIL ${e.stack || e}`);
  failed++;
} finally {
  await browser.close();
  child.kill();
}
console.log(failed ? `gallery: ${failed} check(s) failed` : 'gallery: all checks passed');
process.exit(failed ? 1 : 0);
