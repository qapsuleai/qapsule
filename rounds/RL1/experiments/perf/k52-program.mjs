// RL1/C (VERIFY 19, K52): shells/web/tests/metrics.mjs's run with N program instances in the
// document. The demo is started with `--mcp`; its agent (`claude`) proposes N `add_program`s
// (PR §12.5 breakout, one per instrument syn01..synN) and the page's person accepts each
// (`window.qp.decide`). Then the same replay / scroll / open / drag as metrics.mjs.
// Usage (from shells/web): node ../../rounds/RL1/experiments/perf/k52-program.mjs <qp> <cpu rate> <N> [mode]
//   mode: '' (the default viewer page, as metrics.mjs) or 'replica' (the full replica).
import { startDemo, phonePage, settle, launch, clickIn } from '../../../../shells/web/tests/lib.mjs';

const [qp, rate = '4', nArg = '0', mode = ''] = process.argv.slice(2);
const n = Number(nArg);
const web = new URL('../../../../shells/web/', import.meta.url).pathname;
const { child, base } = await startDemo(qp, web, 8397, ['--mcp']);
const origin = new URL(base).origin.replace('localhost', '127.0.0.1');
const BREAKOUT = 'param len int = 20 in 2..500\ntop = highest(high, len)[1]\nevent kirilim = close > top and defined(top)\n';
let sid = null;
async function mcp(method, params, id) {
  const headers = { 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream', Origin: origin };
  if (sid) headers['Mcp-Session-Id'] = sid;
  const body = JSON.stringify(id === undefined ? { jsonrpc: '2.0', method, params } : { jsonrpc: '2.0', id, method, params });
  const r = await fetch(`${origin}/mcp`, { method: 'POST', headers, body });
  sid = sid || r.headers.get('mcp-session-id');
  const t = await r.text();
  return t ? JSON.parse(t) : null;
}
const browser = await launch();
try {
  const { page, ctx } = await phonePage(browser);
  const cdp = await ctx.newCDPSession(page);
  const q = mode ? `&mode=${mode}` : '';
  await page.goto(`${base.replace('localhost', '127.0.0.1')}${base.includes('?') ? '&' : '?'}anim=0${q}`);
  await settle(page);
  let accepted = 0;
  if (n > 0) {
    await mcp('initialize', { protocolVersion: '2025-06-18', capabilities: {}, clientInfo: { name: 'k52', version: '0' } }, 1);
    await mcp('notifications/initialized', {});
    for (let i = 1; i <= n; i++) {
      const s = `syn${String(i).padStart(2, '0')}`;
      const r = await mcp('tools/call', { name: 'program_add', arguments: { program: BREAKOUT, inputs: [s], id: `k${i}` } }, 10 + i);
      if (r.result.structuredContent.status !== 'proposed') throw new Error(JSON.stringify(r));
    }
    await settle(page);
    const props = await page.evaluate(() => window.qp.proposals());
    for (const p of props) { await page.evaluate((id) => window.qp.decide(id, true), p.id || p.proposal); await settle(page); }
    const doc = await mcp('tools/call', { name: 'query_document', arguments: {} }, 99);
    accepted = doc.result.structuredContent.data.document.series.filter((x) => x.source.study === 'program').length;
    if (accepted !== n) throw new Error(`accepted ${accepted} of ${n}: ${JSON.stringify(props).slice(0, 400)}`);
  }
  if (rate !== '1') await cdp.send('Emulation.setCPUThrottlingRate', { rate: Number(rate) });
  await page.evaluate(() => window.qp.resetMetrics());
  await page.selectOption('#c_speed', '{"kind":"speed","steps_per_s":16}');
  await page.click('#c_play');
  await page.waitForTimeout(6000);
  for (let i = 0; i < 30; i++) { await page.mouse.move(195, 400); await page.mouse.wheel(0, 120); await page.waitForTimeout(30); }
  await page.waitForTimeout(500);
  await page.locator('#feed').evaluate((e) => { e.scrollTop = 0; });
  await page.click('#c_play');
  await settle(page);
  const first = await page.evaluate(() => JSON.parse(window.qp.feed())[0]);
  await clickIn(page, `#list .face[data-id="${first.id}"] canvas`, 120, 60); await settle(page);
  await clickIn(page, '#scene', 195, 150); await settle(page);
  const b = await page.locator('#scene').boundingBox();
  await page.mouse.move(b.x + 250, b.y + 150);
  for (let i = 0; i < 3; i++) { await page.mouse.wheel(0, -100); await settle(page); }
  for (let k = 0; k < 6; k++) {
    await page.mouse.move(b.x + 300, b.y + 150); await page.mouse.down();
    for (let i = 0; i < 20; i++) await page.mouse.move(b.x + 300 - i * 10, b.y + 150);
    await page.mouse.up();
    for (let i = 0; i < 20; i++) await page.mouse.move(b.x + 100 + i * 10, b.y + 150, { steps: 1 });
  }
  await settle(page);
  console.log(JSON.stringify({ cpu_throttling: Number(rate), programs: accepted, mode: mode || 'viewer', ...JSON.parse(await page.evaluate(() => window.qp.metrics())) }));
} finally {
  await browser.close();
  child.kill();
}
