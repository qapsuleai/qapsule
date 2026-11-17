// R4/S1: the settings page and the human-test URL settings in headless Chromium.
//  - the page opens with the presentation's defaults (the core's `PRESENTED_*`), and the
//    settings page lists every registered design with the current one selected;
//  - URL settings win (`?face=fotolay&feed=vakiro&outcome=r3`), and a settings change
//    reaches the view state (R4/S2: the face too);
//  - `?watch=…&long=…&seen=…` set the audience and the last look, and a backward seek (a new
//    authority, a new start frame) applies them again;
//  - "Mark my last look now" sets the last look at the replay clock;
//  - (R5/T3) `?look=ornek` opens the trial look (the chrome's CSS custom properties and the
//    face height come from the core), and Settings → Look switches back to `r4`.
// Usage: node tests/settings.mjs <qp binary>
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { startDemo, phonePage, settle, launch } from './lib.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const web = path.resolve(here, '..');
const [qp] = process.argv.slice(2);
const { child, base } = await startDemo(qp, web, 8396);
const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
const browser = await launch();
let failed = 0;
const check = (ok, what) => { console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`); if (!ok) failed++; };
const look = (page) => page.evaluate(() => JSON.parse(window.qp.look()));
const seek = async (page, v) => { await page.fill('#c_date', String(v)); await page.click('#c_go'); await settle(page); };
try {
  // 1. Defaults and the settings page.
  {
    const { ctx, page } = await phonePage(browser);
    await page.goto(`${origin}/?anim=0`);
    await settle(page);
    const l = await look(page);
    check(l.face === 'vadrel' && l.feed === 'elikov' && l.transition === 'zumlak' && l.outcome === 'tg5',
      `defaults vadrel/elikov/zumlak/tg5 (${JSON.stringify(l)})`);
    await page.click('#tabs button[data-tab="settings"]');
    const ch = await page.evaluate(() => JSON.parse(window.qp.choices ? window.qp.choices() : '{}'));
    const opts = async (sel) => page.$$eval(`${sel} option`, (os) => os.map((o) => o.value));
    const sel = { face: '#s_face', alphabet: '#s_alpha', feed: '#s_feed', transition: '#s_transition', outcome: '#s_outcome', look: '#s_look' };
    for (const k of Object.keys(sel)) {
      const o = await opts(sel[k]);
      check(o.length >= 2 && (!ch[k] || JSON.stringify(o) === JSON.stringify(ch[k])), `${k}: ${o.join(' ')}`);
    }
    // (R4/S2) M8's first two faces are offered; r3 and tavrik stay registered.
    const fo = await opts('#s_face');
    check(['r3', 'vadrel', 'fotolay', 'tavrik'].every((f) => fo.includes(f)), `faces offered: ${fo.join(' ')}`);
    check(await page.$eval('#s_face', (e) => e.value) === 'vadrel', 'face select shows vadrel');
    await page.selectOption('#s_face', 'fotolay');
    await settle(page);
    check((await look(page)).face === 'fotolay', 'face change reaches the view state');
    check(await page.$eval('#s_feed', (e) => e.value) === 'elikov', 'feed select shows elikov');
    check(await page.$eval('#s_outcome', (e) => e.value) === 'tg5', 'outcome select shows tg5');
    await page.selectOption('#s_transition', 'tutamir');
    await settle(page);
    check((await look(page)).transition === 'tutamir', 'settings change reaches the view state');
    // The last-look button.
    await seek(page, 40);
    await page.click('#s_seen');
    await settle(page);
    const st = await page.evaluate(() => JSON.parse(window.qp.status()));
    const s = (await look(page)).seen;
    check(Number.isInteger(s), `seen button sets the last look (${s}; status ${JSON.stringify(st).slice(0, 80)})`);
    await ctx.close();
  }
  // 2. URL settings, and again after a backward seek.
  {
    const { ctx, page, errors } = await phonePage(browser);
    // (R4/S2) `NOPE` and the repeated `SYN02` are dropped by the core (URL lists keep known
    // symbols once).
    await page.goto(`${origin}/?anim=0&face=fotolay&feed=vakiro&outcome=r3&watch=SYN02,NOPE,SYN03,SYN02&long=SYN09&seen=200`);
    await settle(page);
    let l = await look(page);
    check(l.face === 'fotolay' && l.feed === 'vakiro' && !l.outcome && l.transition === 'zumlak', `URL wins (${JSON.stringify(l)})`);
    const aud = () => page.evaluate(() => JSON.parse(window.qp.audience()));
    let a = await aud();
    check(JSON.stringify(a && a.watchlist) === '["SYN02","SYN03"]' && a.positions && a.positions.SYN09 === 1,
      `URL audience (${JSON.stringify(a)})`);
    await seek(page, 150);
    check(!(await look(page)).seen, 'seen waits for step 200');
    await seek(page, 250);
    const s1 = (await look(page)).seen;
    check(Number.isInteger(s1), `seen set on reaching step 200+ (${s1})`);
    await seek(page, 100); // backward: a new authority and start frame
    await seek(page, 210);
    a = await aud();
    l = await look(page);
    check(JSON.stringify(a && a.watchlist) === '["SYN02","SYN03"]', `audience again after a backward seek (${JSON.stringify(a)})`);
    check(l.face === 'fotolay' && l.feed === 'vakiro' && Number.isInteger(l.seen) && l.seen < s1, `look and seen again after a backward seek (${l.seen})`);
    check(errors.length === 0, `no page errors ${errors}`);
    await ctx.close();
  }
  // 3. (R5/T3) Looks: `?look=` and Settings → Look; the chrome follows the core's look.
  {
    const { ctx, page, errors } = await phonePage(browser);
    const css = (k) => page.evaluate((n) => getComputedStyle(document.documentElement).getPropertyValue(n).trim(), k);
    const faceH = () => page.evaluate(() => JSON.parse(window.qp.faceSize ? window.qp.faceSize() : 'null'));
    // (R5/T3b) The chrome's sizes come from the record: the tabs' height, the least height
    // of the visible controls and the smallest text of the page (feed and settings).
    const sizes = () => page.evaluate(() => {
      const box = (e) => e.getBoundingClientRect();
      const shown = (e) => { const r = box(e); return r.width > 0 && r.height > 0 && getComputedStyle(e).visibility !== 'hidden'; };
      const tab = box(document.querySelector('#tabs button')).height;
      let control = Infinity, font = Infinity, fontAt = '';
      for (const e of document.querySelectorAll('button, select, input')) if (shown(e)) control = Math.min(control, box(e).height);
      for (const e of document.querySelectorAll('body *')) {
        if (!shown(e) || !Array.from(e.childNodes).some((n) => n.nodeType === 3 && n.textContent.trim())) continue;
        const px = parseFloat(getComputedStyle(e).fontSize);
        if (px < font) { font = px; fontAt = e.id || e.className || e.tagName; }
      }
      return { tab, control, font, fontAt };
    });
    const pageSizes = async () => {
      const feed = await sizes();
      await page.click('#tabs button[data-tab="settings"]');
      const set = await sizes();
      await page.click('#tabs button[data-tab="feed"]');
      return { tab: feed.tab, control: Math.min(feed.control, set.control),
        font: Math.min(feed.font, set.font), fontAt: feed.font <= set.font ? feed.fontAt : set.fontAt };
    };
    await page.goto(`${origin}/?anim=0`);
    await settle(page);
    const bg4 = await css('--bg');
    check(bg4 === '#0f1115', `r4 chrome background from the core (${bg4})`);
    const h4 = await faceH();
    const s4 = await pageSizes();
    console.log(`     r4 sizes: ${JSON.stringify(s4)}`);
    check(s4.tab < 44 && s4.font === 10, `r4 keeps its chrome sizes (tab ${s4.tab}, smallest text ${s4.font} px)`);
    await page.goto(`${origin}/?anim=0&look=ornek`);
    await settle(page);
    let l = await look(page);
    check(l.look === 'ornek', `URL look (${JSON.stringify(l)})`);
    const bgO = await css('--bg');
    check(bgO === '#181412', `ornek chrome background (${bgO})`);
    const hO = await faceH();
    check(!h4 || !hO || hO.css[1] > h4.css[1], `ornek face is taller (${JSON.stringify(h4)} → ${JSON.stringify(hO)})`);
    const sO = await pageSizes();
    console.log(`     ornek sizes: ${JSON.stringify(sO)}`);
    check(sO.tab >= 44, `ornek tabs ≥ 44 px (${sO.tab})`);
    check(sO.control >= 44, `ornek controls ≥ 44 px (${sO.control})`);
    check(sO.font >= 13, `ornek smallest text ≥ 13 px (${sO.font} px, ${sO.fontAt})`);
    await page.click('#tabs button[data-tab="settings"]');
    check(await page.$eval('#s_look', (e) => e.value) === 'ornek', 'look select shows ornek');
    await page.selectOption('#s_look', 'r4');
    await settle(page);
    l = await look(page);
    check(!l.look, `look back to r4 (${JSON.stringify(l)})`);
    check(await css('--bg') === '#0f1115', 'chrome back to r4');
    check(errors.length === 0, `no page errors ${errors}`);
    await ctx.close();
  }
} finally {
  await browser.close();
  child.kill();
}
process.exit(failed ? 1 : 0);
