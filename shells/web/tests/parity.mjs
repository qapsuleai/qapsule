// R4/T5 (GOAL 2 and 4; VERIFY 8, 10): drives the web shell in headless Chromium with the
// steps of parity-script.json as a user would (clicks, drag, wheel, settings), and
//  - checks that every request of the page goes to the page's own origin (network test);
//  - compares the digests the page reports (document, every face, the open scene, view
//    state) with `qp drive` — the same session driven with protocol messages only.
// RP3/M15 (§22): twice — the full replica (`?mode=replica`) against `qp drive`, and the
// viewer (the default) against `qp drive --mode viewer` (the open capsule's slice digest,
// the faces on screen, the open scene, the view state, the data subscriptions); and the
// viewer driver's faces and open scenes against the full replica driver's.
// Usage: node tests/parity.mjs <qp binary> <out dir>
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { startDemo, phonePage, settle, launch, clickIn } from './lib.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const web = path.resolve(here, '..');
const [qp, out] = process.argv.slice(2);
fs.mkdirSync(out, { recursive: true });
const scriptPath = path.join(here, 'parity-script.json');
const script = JSON.parse(fs.readFileSync(scriptPath, 'utf8'));

const { child, base } = await startDemo(qp, web, 8397);
const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
const browser = await launch();
let failed = 0;

async function drive(query) {
  const { ctx, page, requests, errors } = await phonePage(browser);
  await page.goto(`${origin}/?${query}`);
  await settle(page);
  const digests = {};
  for (const step of script) {
    if (step.seek_step !== undefined) {
      await page.fill('#c_date', String(step.seek_step));
      await page.click('#c_go');
    } else if (step.tap_face) {
      await settle(page);
      // (R4/S1) `index` counts capsule rows (a feed layout may add group headers).
      const rows = await page.evaluate(() => JSON.parse(window.qp.feed()));
      const first = rows.filter((r) => r.id)[step.tap_face.index];
      if (!first || !first.scene) throw new Error(`face ${step.tap_face.index} has no face scene`);
      await clickIn(page, `#list .face[data-id="${first.id}"] canvas`, step.tap_face.x, step.tap_face.y);
    } else if (step.tap_scene) {
      await clickIn(page, '#scene', step.tap_scene.x, step.tap_scene.y);
    } else if (step.wheel) {
      const b = await page.locator('#scene').boundingBox();
      await page.mouse.move(b.x + step.wheel.x, b.y + 150);
      await page.mouse.wheel(0, step.wheel.dy < 0 ? -100 : 100);
    } else if (step.drag) {
      const b = await page.locator('#scene').boundingBox();
      await page.mouse.move(b.x + 195, b.y + 150);
      await page.mouse.down();
      await page.mouse.move(b.x + 195 + step.drag.dx, b.y + 150, { steps: 1 });
      await page.mouse.up();
    } else if (step.style) {
      await page.click('#tabs button[data-tab="settings"]');
      if (step.style.theme) { await page.selectOption('#s_theme', step.style.theme); await settle(page); }
      if (step.style.alphabet) { await page.selectOption('#s_alpha', step.style.alphabet); await settle(page); }
      if (step.style.face) { await page.selectOption('#s_face', step.style.face); await settle(page); }
      await page.click('#tabs button[data-tab="feed"]');
    } else if (step.back) {
      await page.click('#back');
    } else if (step.digest) {
      await settle(page);
      digests[step.digest] = JSON.parse(await page.evaluate(() => window.qp.digests()));
      continue;
    }
    await settle(page);
  }
  const tag = query.includes('mode=replica') ? 'replica' : 'viewer';
  fs.writeFileSync(path.join(out, `browser-${tag}.json`), JSON.stringify(digests, null, 2));
  fs.writeFileSync(path.join(out, `sent-${tag}.json`), await page.evaluate(() => window.qp.sent()));

  // Network: only the page's own origin.
  const foreign = requests.filter((u) => new URL(u).origin !== origin);
  console.log(`${tag} network: ${requests.length} requests, ${foreign.length} to another origin`);
  if (foreign.length) { console.error('foreign requests:', foreign); failed++; }
  if (errors.length) { console.error('page errors:', errors); failed++; }
  await ctx.close();
  return digests;
}

const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
try {
  // The full replica (R4) against the shell-less driver.
  const replica = await drive('anim=0&mode=replica');
  const sl = path.join(out, 'shell-less.json');
  execFileSync(qp, ['drive', '--script', scriptPath, '--out', sl], { stdio: 'inherit' });
  const want = JSON.parse(fs.readFileSync(sl, 'utf8'));
  for (const k of Object.keys(want)) {
    const a = want[k], b = replica[k];
    const ok = b && a.document === b.document && same(a.faces, b.faces) && same(a.open, b.open) && same(a.look, b.look);
    console.log(`replica ${k}: ${ok ? 'equal' : 'DIFFERENT'} (document ${a.document}, ${a.faces.length} faces, open ${a.open.depth} ${a.open.digest})`);
    if (!ok) failed++;
  }
  // (RP3/M15) The viewer (default) against the shell-less viewer driver.
  const viewer = await drive('anim=0');
  const vl = path.join(out, 'viewer-less.json');
  execFileSync(qp, ['drive', '--mode', 'viewer', '--script', scriptPath, '--out', vl], { stdio: 'inherit' });
  const vwant = JSON.parse(fs.readFileSync(vl, 'utf8'));
  for (const k of Object.keys(vwant)) {
    const a = vwant[k], b = viewer[k];
    // Faces on the page's screen (the others never came) equal the driver's.
    const shown = b ? b.faces.filter((f) => f.digest) : [];
    // (Under an open capsule the feed is hidden: no face is on screen then.)
    const facesOk = (shown.length > 0 || b.open.depth !== 'sentence')
      && shown.every((f) => a.faces.some((g) => g.id === f.id && g.digest === f.digest));
    const ok = b && a.document === b.document && facesOk && same(a.open, b.open) && same(a.look, b.look) && same(a.data, b.data);
    console.log(`viewer ${k}: ${ok ? 'equal' : 'DIFFERENT'} (slice ${a.document}, ${shown.length} faces on screen, open ${a.open.depth} ${a.open.digest}, data ${JSON.stringify(a.data)})`);
    if (!ok) failed++;
    // The viewer driver's faces and open scene are the full replica driver's.
    const r = want[k];
    const crossOk = r && same(a.faces, r.faces) && same(a.open, r.open);
    console.log(`viewer = replica ${k}: ${crossOk ? 'equal' : 'DIFFERENT'}`);
    if (!crossOk) failed++;
  }
} finally {
  await browser.close();
  child.kill();
}
process.exit(failed ? 1 : 0);
