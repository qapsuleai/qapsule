// R4/HT — insan testi v2: oturum provası ve cevap anahtarı (yürütücü, oturumlardan önce).
//
//   node prepare.mjs key  [--plan plan.json] [--out .]       anahtar: key.json + key.md + links.html
//   node prepare.mjs scan --from 1 --to 600 [--stride 3] [--rows 3] [--feed r3] [--audience 1]
//                                                            aday arama (plan.json'u kurmak için)
//   node prepare.mjs scan-g2 --from 610 --to 1000 [--window 9] [--stride 9]
//                                                            G2 penceresi arama (plan.json audience ile)
// Ortak seçenekler: --qp <qp ikilisi> (varsayılan: <kök>/target/release/qp), --web <kabuk dizini>
// (varsayılan: <kök>/shells/web), --port <n> (varsayılan 8597).
//
// Yöntem: `qp demo --synthetic --paused` başlatılır; başsız Chromium telefon boyutunda
// (390×844 CSS px) gösterimi açar ve oturumu yürütücünün yapacağı sırayla oynar: plan.json'daki
// URL ayarları, oynatma çubuğuyla adıma gitme (yalnız ileri), G2'de Ayarlar sayfasından kitle,
// son bakış (`window.qp.seen()`, yürütücünün yer imi betiğinin yaptığı). Akışın satırları
// çekirdekten (`window.qp.feed()`), kapsüllerin durumu kabuğun aldığı protokol iletilerinden
// (`capsules` sonucu ve `capsule` bildirimleri; sayfaya dokunulmaz, yalnız EventSource dinlenir).
// Anahtar motorun durumundan kurulur (SCORING.md "Anahtar"), tasarımdan değil.
// Belirleyici: aynı plan + aynı gösterim → bayt eşit key.json. Gerçek veri kullanılmaz (--synthetic).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../..');

// --- arguments ----------------------------------------------------------------------
const [cmd, ...rest] = process.argv.slice(2);
const opt = {};
for (let i = 0; i < rest.length; i++) {
  if (rest[i].startsWith('--')) opt[rest[i].slice(2)] = rest[i + 1], i++;
}
const qp = path.resolve(opt.qp || path.join(root, 'target/release/qp'));
const web = path.resolve(opt.web || path.join(root, 'shells/web'));
const port = Number(opt.port || 8597);
const planPath = path.resolve(opt.plan || path.join(here, 'plan.json'));
const outDir = path.resolve(opt.out || here);
if (!['key', 'scan', 'scan-g2'].includes(cmd)) {
  console.error('kullanım: node prepare.mjs key|scan|scan-g2 [seçenekler] (dosyanın başına bakın)');
  process.exit(2);
}
const { startDemo, settle, launch } = await import(path.join(web, 'tests/lib.mjs'));
const plan = JSON.parse(fs.readFileSync(planPath, 'utf8'));

// --- the answer rules (SCORING.md "Anahtar"; R4 harness S8 rules, SPEC §2, E1) ---------
const DAY = 86_400_000;
const LOCAL = 3 * 3_600_000; // exchange time zone of the synthetic calendar (UTC+3)
const MIN_N = 20; // qp_grammar::depth::GLYPH_MIN_LEDGER_N (SPEC E1)
const DIR = { up: 'yukarı', down: 'aşağı', neutral: 'belirsiz' };
const KIND = {
  breakout: 'fiyat önemli bir seviyeyi geçti', reversal: 'yön değişiyor', continuation: 'hareket sürüyor',
  squeeze: 'fiyat sıkıştı, hareketsiz', extreme: 'aşırı gitti', flow: 'büyük alım ya da satım var',
};
const localDay = (t) => Math.floor((t + LOCAL) / DAY);
function when(it, clock) {
  const d = localDay(clock) - localDay(it.t);
  return d <= 0 ? 'bugün' : d === 1 ? 'dün' : d <= 6 ? 'birkaç gün önce' : 'bir haftadan önce';
}
function validity(it) {
  if (it.validity === 'valid') return 'evet, sürüyor';
  if (it.validity === 'no_longer') return 'hayır, artık geçerli değil';
  if (it.stage === 'realized') return 'kapandı: gerçekleşti';
  if (it.stage === 'invalidated') return 'kapandı: tutmadı';
  return 'kapandı: süresi doldu ya da geri çekildi';
}
function trust(it) {
  const { n, realized } = it.ledger_at_birth;
  if (n < MIN_N) return 'az örnek var, bilinmiyor';
  const r = realized / n;
  return r >= 0.6 ? 'geçmişte çoğunlukla tuttu' : r >= 0.4 ? 'yarı yarıya' : 'çoğunlukla tutmadı';
}
// G4: the outcome a closed capsule's mark shows; open ones are "hâlâ açık".
function outcome(it) {
  if (it.validity !== 'closed') return 'hâlâ açık';
  return it.stage === 'realized' ? 'gerçekleşti' : it.stage === 'invalidated' ? 'tutmadı' : 'süresi doldu';
}
const NEGATIVE = { weakened: 'zayıfladı', invalidated: 'tutmadı, geçersiz oldu', retracted: 'geri çekildi' };
const faceKey = (it, clock) => ({
  s1: DIR[it.direction], s1b: KIND[it.kind], s2: when(it, clock), s3: validity(it), s4: trust(it),
  s5: 'hiçbir şey, yalnız bilgi veriyor',
});
const brief = (it) => ({
  capsule: it.id, instrument: it.instrument, kind: it.kind, direction: it.direction, validity: it.validity,
  stage: it.stage, ledger: `${it.ledger_at_birth.realized}/${it.ledger_at_birth.n}`,
  t: new Date(it.t + LOCAL).toISOString().slice(0, 10), sentence: it.sentence.tr,
});

// --- placeholders -------------------------------------------------------------------
const unfilled = Object.entries(plan.faces).filter(([, v]) => /^<<.*>>$/.test(v)).map(([k]) => k);
const resolve = (v) => (typeof v === 'string' && v.startsWith('@') ? plan.faces[v.slice(1)] : v);
const base = Object.fromEntries(Object.entries(plan.base).map(([k, v]) => [k, resolve(v)]));
// For the key run an unfilled face is replaced by the registry's default: the key does not
// depend on the face (the feed order does not), only links.html does.
const runFace = (v) => (/^<<.*>>$/.test(v) ? '' : v);

// --- the page -----------------------------------------------------------------------
const { child, base: url0 } = await startDemo(qp, web, port);
const origin = new URL(url0).origin.replace('localhost', '127.0.0.1');
const browser = await launch();
let page;
async function open(settings) {
  const q = new URLSearchParams();
  for (const [k, v] of Object.entries(settings)) if (v) q.set(k, k === 'face' ? runFace(v) : v);
  for (const [k, v] of [...q.entries()]) if (!v) q.delete(k);
  await page.goto(`${origin}/?${q}`);
  await settle(page);
}
async function seek(step) {
  await page.fill('#c_date', String(step));
  await page.click('#c_go');
  await settle(page);
  const st = JSON.parse(await page.evaluate(() => window.qp.status()));
  const at = Number(st.step.split(' / ')[0]);
  if (at !== Number(step)) throw new Error(`adım ${step} istendi, gösterim ${st.step}`);
  return st;
}
// Everything the shell received since its last start frame: capsule id -> item, the replay clock.
async function state() {
  const frames = await page.evaluate(() => window.__frames);
  let items = new Map();
  let clock = null;
  let mark = 0;
  frames.forEach((f, i) => {
    const j = JSON.parse(f);
    if (j.start) { items = new Map(); mark = i; return; }
    const m = j.msg;
    if (!m) return;
    if (m.type === 'result' && m.data && m.data.kind === 'capsules') for (const it of m.data.capsules || m.data.items || []) items.set(it.id, it);
    if (m.type === 'notify' && m.event && m.event.kind === 'capsule') items.set(m.event.capsule.id, m.event.capsule);
    if (m.type === 'result' && m.data && m.data.kind === 'replay') clock = m.data.replay.t;
  });
  const rows = JSON.parse(await page.evaluate(() => window.qp.feed()));
  const caps = rows.filter((r) => !r.header).map((r) => r.id);
  return { items, clock, rows, caps, frames: frames.length, startAt: mark };
}
// Capsule notifications received after frame index `from` (G2: since the last look).
async function notifiesSince(from) {
  const frames = await page.evaluate(() => window.__frames);
  const out = [];
  for (const f of frames.slice(from)) {
    const j = JSON.parse(f);
    if (j.start) throw new Error('bağlantı yeniden kuruldu (G2 penceresinin ortasında)');
    const m = j.msg;
    if (m && m.type === 'notify' && m.event && m.event.kind === 'capsule') out.push(m.event.capsule);
  }
  return out;
}
async function setAudience(a) {
  await page.click('#tabs button[data-tab="settings"]');
  await page.selectOption('#s_group', a.group);
  await page.selectOption('#s_sens', a.sensitivity);
  await settle(page);
  // Every row as the plan says (a later window's audience replaces the earlier one).
  for (const sym of await page.$$eval('#instruments tr[data-sym]', (rs) => rs.map((r) => r.dataset.sym))) {
    const row = `#instruments tr[data-sym="${sym}"]`;
    await page.setChecked(`${row} input`, a.watch.includes(sym));
    const pos = a.long.includes(sym) ? 'long' : (a.short || []).includes(sym) ? 'short' : '';
    await page.selectOption(`${row} select`, pos);
  }
  await page.click('#s_apply');
  await settle(page);
  await page.click('#tabs button[data-tab="feed"]');
  await settle(page);
}

function itemAt(st, row, capsule, where) {
  const id = st.caps[row];
  const it = st.items.get(id);
  if (!it) throw new Error(`${where}: satır ${row} yok`);
  if (capsule && capsule !== id) {
    const real = st.caps.indexOf(capsule);
    throw new Error(`${where}: satır ${row} = ${id}, planda ${capsule} (gerçek satırı ${real})`);
  }
  return it;
}

// G2 key at t1 for one window: changed capsules, the held instrument's negative news, the
// most reliable capsule(s).
function g2Key(a, before, after, notes) {
  const audience = new Set([...a.watch, ...a.long, ...(a.short || [])]);
  const held = new Set([...a.long, ...(a.short || [])]);
  const inFeed = new Set(after.caps);
  const changed = new Map();
  for (const it of notes) {
    const prev = before.items.get(it.id);
    if (prev && prev.stage === it.stage && prev.last_bar === it.last_bar && prev.validity === it.validity) continue;
    changed.set(it.id, it);
  }
  const moved = [...changed.values()].filter((it) => audience.has(it.instrument));
  const g1 = [...new Set(moved.filter((it) => inFeed.has(it.id)).map((it) => `${it.instrument} ${DIR[it.direction]}`))].sort();
  const neg = moved.filter((it) => held.has(it.instrument) && NEGATIVE[it.stage]);
  const g2 = [...new Set(neg.map((it) => it.instrument))].sort();
  // G3: among the feed's capsules, those with n >= MIN_N in the highest ratio bucket.
  const rated = after.caps.map((id) => after.items.get(id)).filter((it) => it && it.ledger_at_birth.n >= MIN_N);
  const bucket = (it) => { const r = it.ledger_at_birth.realized / it.ledger_at_birth.n; return r >= 0.6 ? 2 : r >= 0.4 ? 1 : 0; };
  const top = Math.max(...rated.map(bucket));
  const g3 = rated.filter((it) => bucket(it) === top).map((it) => ({ capsule: it.id, instrument: it.instrument, ledger: `${it.ledger_at_birth.realized}/${it.ledger_at_birth.n}`, row: after.caps.indexOf(it.id) }));
  return {
    g1, g1_capsules: moved.map((it) => ({ ...brief(it), in_feed: inFeed.has(it.id) })),
    g2_which: g2, g2_what: [...new Set(neg.map((it) => NEGATIVE[it.stage]))],
    g2_capsules: neg.map((it) => ({ ...brief(it), in_feed: inFeed.has(it.id), row: after.caps.indexOf(it.id) })),
    g3_bucket: ['< %40', '%40–60', '≥ %60'][top], g3, feed_rows: after.caps.length,
  };
}

// --- commands -----------------------------------------------------------------------
const warnings = [];
try {
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2 });
  page = await ctx.newPage();
  await page.addInitScript(() => {
    const ES = window.EventSource;
    window.__frames = [];
    window.EventSource = class extends ES {
      constructor(u) { super(u); this.addEventListener('message', (e) => window.__frames.push(e.data)); }
    };
  });
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));

  if (cmd === 'scan') {
    const from = Number(opt.from || 1); const to = Number(opt.to || 600); const stride = Number(opt.stride || 3);
    const nrows = Number(opt.rows || 3);
    await open({ ...base, feed: opt.feed || base.feed });
    for (let s = from; s <= to; s += stride) {
      await seek(s);
      if (opt.audience && s === from) await setAudience(plan.G2.audience);
      const st = await state();
      const cells = st.caps.slice(0, nrows).map((id, i) => {
        const it = st.items.get(id);
        const k = faceKey(it, st.clock);
        return `${i}:${id.replace('cap.', '')} ${it.validity}/${it.stage} ${k.s2} ${it.ledger_at_birth.realized}/${it.ledger_at_birth.n}`;
      });
      console.log(`${s}\t${cells.join('\t')}`);
    }
  } else if (cmd === 'scan-g2') {
    const a = plan.G2.audience;
    const from = Number(opt.from || 610); const to = Number(opt.to || 1000);
    const win = Number(opt.window || 9); const stride = Number(opt.stride || win);
    await open({ ...base, feed: plan.comparisons.feed.A });
    await seek(from);
    await setAudience(a);
    for (let t0 = from; t0 + win <= to; t0 += stride) {
      if (t0 !== from) await seek(t0);
      await page.evaluate(() => { window.qp.seen(); });
      await settle(page);
      const before = await state();
      await seek(t0 + win);
      const after = await state();
      const notes = await notifiesSince(before.frames);
      const k = g2Key(a, before, after, notes);
      console.log(`${t0}-${t0 + win}\tG1 ${k.g1.join(', ') || '-'}\tG2 ${k.g2_which.join(', ') || '-'} ${k.g2_what.join(', ')}\tG3 ${k.g3_bucket} ${k.g3.map((x) => `${x.instrument}@${x.row}`).join(' ')}`);
    }
  } else {
    // --- key: the whole session in the executor's order (plan.order) ------------------
    // Seeking backward rebuilds the authority (the audience is lost; URL settings stay), so
    // only G2, which needs the audience, must come last and move forward.
    const key = { plan: path.basename(planPath), generated_by: 'prepare.mjs key', synthetic: true, order: plan.order, items: {} };
    const clockText = (st) => new Date(st.clock + LOCAL).toISOString().slice(0, 16).replace('T', ' ');
    const one = async (id, task, set, x, mk) => {
      await seek(x.step);
      const st = await state();
      const it = itemAt(st, x.row, x.capsule, id);
      key.items[id] = { task, set, step: x.step, row: x.row, clock: clockText(st), ...brief(it), key: mk(it, st) };
    };
    const screen = async (id, task, set, x) => {
      await seek(x.step);
      const st = await state();
      const rows = x.rows.map((r) => (r < 0 ? st.caps.length + r : r));
      const its = rows.map((r, i) => itemAt(st, r, x.capsules && x.capsules[i], `${id} satır ${r}`));
      key.items[id] = {
        task, set, step: x.step, clock: clockText(st), feed_rows: st.caps.length,
        rows: its.map((it, i) => ({ row: rows[i], ...brief(it), key: { outcome: outcome(it), s1: DIR[it.direction], s1b: KIND[it.kind], s3: validity(it) } })),
      };
    };
    await open(base);
    for (const task of plan.order) {
      if (task === 'P0') await one('P0', 'practice', 0, plan.practice, (it, st) => faceKey(it, st.clock));
      if (task === 'G1') for (const [si, set] of plan.G1.sets.entries()) for (const [i, x] of set.entries()) await one(`G1-${si + 1}${'abc'[i]}`, 'G1', si + 1, x, (it, st) => faceKey(it, st.clock));
      if (task === 'G3') for (const [si, set] of plan.G3.sets.entries()) for (const [i, x] of set.entries()) await one(`G3-${si + 1}${'ab'[i]}`, 'G3', si + 1, x, (it) => ({ same: 'evet', s3: validity(it) }));
      if (task === 'G4' || task === 'G5') for (const [si, x] of plan[task].sets.entries()) await screen(`${task}-${si + 1}`, task, si + 1, x);
      if (task === 'G2' && plan.order[plan.order.length - 1] !== 'G2') throw new Error('G2 son görev olmalı (kitle geri gidince silinir)');
    }
    // G2 (feed layout), for both layouts as the executor does it: per window load the
    // layout's URL, go to t0, (first window) set the audience, mark the last look, go to t1.
    for (const layout of ['A', 'B']) {
      const feed = plan.comparisons.feed[layout];
      let lastG2 = 0;
      for (const [si, w] of plan.G2.sets.entries()) {
        const id = `G2-${si + 1}`;
        if (w.t0 < lastG2 || w.t1 <= w.t0) throw new Error(`${id}: pencere geriye gidiyor`);
        await open({ ...base, feed });
        await seek(w.t0);
        const aud = w.audience || plan.G2.audience;
        if (si === 0 || w.audience) await setAudience(aud);
        await page.evaluate(() => { window.qp.seen(); });
        await settle(page);
        const before = await state();
        await seek(w.t1);
        const after = await state();
        const notes = await notifiesSince(before.frames);
        const k = g2Key(aud, before, after, notes);
        k.layout_rows = after.rows.slice(0, 12).map((r) => (r.header ? `# ${r.header}` : `${r.id}${r.mark ? ` [${r.mark}]` : ''}`));
        if (!k.g2_which.length) warnings.push(`${id} (${feed}): elde tutulan hissede olumsuz geçiş yok`);
        if (k.g2_capsules.some((c) => !c.in_feed)) warnings.push(`${id} (${feed}): olumsuz geçiş akışta görünmüyor`);
        const prevKey = key.items[id];
        const content = { g1: k.g1, g2_which: k.g2_which, g2_what: k.g2_what, g3: k.g3.map((x) => x.capsule) };
        if (prevKey) {
          const was = { g1: prevKey.key.g1, g2_which: prevKey.key.g2_which, g2_what: prevKey.key.g2_what, g3: prevKey.key.g3.map((x) => x.capsule) };
          if (JSON.stringify(was) !== JSON.stringify(content)) warnings.push(`${id}: anahtar düzene göre değişiyor (${JSON.stringify(was)} / ${JSON.stringify(content)})`);
          prevKey.layouts[feed] = k.layout_rows;
        } else {
          key.items[id] = { task: 'G2', set: si + 1, t0: w.t0, t1: w.t1, held: [...aud.long, ...(aud.short || [])], watch: aud.watch, key: k, layouts: { [feed]: k.layout_rows } };
          delete k.layout_rows;
        }
        lastG2 = w.t1;
      }
    }
    // Cross-check with the M8 harness's S8 keys where a capsule is the same at the same step.
    const s8p = path.join(root, 'rounds/R4/experiments/harness/s8.json');
    if (fs.existsSync(s8p)) {
      const s8 = JSON.parse(fs.readFileSync(s8p, 'utf8')).states.filter((s) => s.found && s.world === 'demo' && s.principal === 'arkin');
      for (const [id, x] of Object.entries(key.items)) {
        const s = x.key && x.key.s1b && s8.find((q) => q.capsule === x.capsule && q.step === x.step);
        if (!s) continue;
        const diff = Object.keys(s.key).filter((k) => s.key[k] !== x.key[k] && !(k === 's4'));
        key.items[id].s8_check = diff.length ? `S8 #${s.state} ile fark: ${diff.join(', ')}` : `S8 #${s.state} ile aynı`;
      }
    }
    if (errors.length) warnings.push(`sayfa hataları: ${errors.join('; ')}`);
    key.warnings = warnings;
    key.unfilled_faces = unfilled;
    fs.writeFileSync(path.join(outDir, 'key.json'), `${JSON.stringify(key, null, 2)}\n`);
    fs.writeFileSync(path.join(outDir, 'key.md'), keyMd(key));
    fs.writeFileSync(path.join(outDir, 'links.html'), linksHtml());
    console.log(`key.json, key.md, links.html yazıldı (${Object.keys(key.items).length} madde)`);
    for (const w of warnings) console.log(`UYARI: ${w}`);
    if (unfilled.length) console.log(`UYARI: yüz adları doldurulmadı (${unfilled.join(', ')}): plan.json "faces"; links.html yer tutucu taşır`);
  }
} finally {
  await browser.close();
  child.kill();
}

// --- outputs ------------------------------------------------------------------------
function keyMd(key) {
  const L = [];
  const w = (s = '') => L.push(s);
  w('# Cevap anahtarı (yürütücü; katılımcı görmez)');
  w();
  w('`node prepare.mjs key` üretti (sentetik pazar, `qp demo --synthetic`). Kurallar `SCORING.md`. ' +
    'Satır = akışın yukarıdan kaçıncı kapsülü (0 = en üstteki; başlık satırları sayılmaz).');
  w();
  if (key.warnings.length) { w('**Uyarılar:**'); for (const x of key.warnings) w(`- ${x}`); w(); }
  for (const [id, x] of Object.entries(key.items)) {
    if (x.task === 'practice' || x.task === 'G1') {
      w(`## ${id} — adım ${x.step}, satır ${x.row}${x.task === 'practice' ? ' (alıştırma, puanlanmaz)' : ''}`);
      w();
      w(`${x.instrument} · ${x.sentence}`);
      w();
      w(`| S1 yön | S1b ne | S2 ne zaman | S3 geçerli mi | S4 güven | S5 |\n|---|---|---|---|---|---|\n| ${x.key.s1} | ${x.key.s1b} | ${x.key.s2} | ${x.key.s3} | ${x.key.s4} (${x.ledger}) | ${x.key.s5} |`);
      if (x.s8_check) w(`\n_${x.s8_check}_`);
      w();
    } else if (x.task === 'G3') {
      w(`## ${id} — adım ${x.step}, satır ${x.row}`);
      w();
      w(`${x.instrument} · ${x.sentence}`);
      w();
      w(`İşaretli yer: grafikteki **vurgulu sütun** (olgunun barı) ve üstündeki işaret; cümlede seviye varsa o seviye. Aynı şey mi: **${x.key.same}**. Hâlâ geçerli mi: **${x.key.s3}**.`);
      w();
    } else if (x.task === 'G4' || x.task === 'G5') {
      w(`## ${id} — adım ${x.step}`);
      w();
      w('| sıra | satır | kapsül | sonuç (G4) | yön | ne | geçerli mi |\n|---|---:|---|---|---|---|---|');
      x.rows.forEach((r, i) => w(`| ${i + 1} | ${r.row} | ${r.instrument} (${r.kind}.${r.direction}, ${r.ledger}) | ${r.key.outcome} | ${r.key.s1} | ${r.key.s1b} | ${r.key.s3} |`));
      w();
    } else if (x.task === 'G2') {
      const k = x.key;
      w(`## ${id} — son bakış adım ${x.t0}, bakış adım ${x.t1}; elde: ${x.held.join(', ')}`);
      w();
      w(`- **G2a ne değişti:** ${k.g1.join('; ') || '(yok)'}`);
      w(`- **G2b olumsuz:** ${k.g2_which.join(', ') || '(yok)'} — ${k.g2_what.join(', ')}`);
      w(`- **G2c en güvenilir** (n ≥ 20, kova ${k.g3_bucket}; ${k.g3.length} kapsül doğru): ${k.g3.map((c) => `${c.instrument} ${c.ledger}`).join(', ')}`);
      for (const [feed, rows] of Object.entries(x.layouts)) w(`- akışın ilk satırları (\`${feed}\`): ${rows.slice(0, 8).join(' · ')}`);
      w();
    }
  }
  return `${L.join('\n')}\n`;
}

// The executor's link page (opened on the phone from `python3 -m http.server`): one neutral
// code per block URL. The names are in the URLs (the executor's eyes only).
function linksHtml() {
  const links = [];
  const add = (code, label, over) => {
    const s = { ...base, ...over };
    const q = new URLSearchParams();
    for (const k of ['lang', 'theme', 'face', 'alphabet', 'feed', 'transition', 'outcome']) if (s[k]) q.set(k, s[k]);
    links.push({ code, label, query: `?${q}` });
  };
  add('U0', 'taban (alıştırma)', {});
  const c = plan.comparisons;
  let n = 1;
  for (const name of ['face', 'transition', 'outcome', 'alphabet', 'feed']) {
    for (const side of ['A', 'B']) {
      const v = resolve(c[name][side]);
      add(`U${n++}`, `${c[name].task} · ${name} ${side}`, { [c[name].param]: v });
    }
  }
  const rows = links.map((l) => `<tr><td><a data-q="${l.query}" href="#">${l.code}</a></td><td>${l.label}</td><td><code>${l.query.replace(/&/g, '&amp;')}</code></td></tr>`).join('\n');
  return `<!doctype html>
<html lang="tr"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Oturum bağlantıları</title>
<style>body{font:15px/1.4 system-ui,sans-serif;margin:16px;max-width:640px}td{padding:6px 8px;border-bottom:1px solid #ddd}
a{font-weight:700;font-size:18px}code{font-size:11px;color:#555;word-break:break-all}input{font-size:16px;width:14em}</style></head>
<body>
<h1>Oturum bağlantıları (yürütücü)</h1>
<p>Gösterimin adresi (konsolun ya da Ayarlar sekmesinin yazdığı): <input id="host" placeholder="http://192.168.1.20:8390"></p>
<p>Kodlar tarafsızdır; hangi kodun hangi blokta açılacağı oturum sırası tablosunda (GUIDE.md). Bu sayfayı katılımcıya göstermeyin.</p>
<table>${rows}</table>
<h2>Son bakış (G2)</h2>
<p>Gösterimde "son bakış"ı koyan bir düğme yok; yalnız sayfanın <code>window.qp.seen()</code> işlevi var.
Telefonun tarayıcısına bir kez şu adresli bir yer imi ekleyin (ad: <b>sb</b>):</p>
<p><code>javascript:qp.seen();void(0)</code></p>
<p>G2'de son bakış adımına gelince gösterim sayfasındayken yer imini açın (iOS Safari: Yer İmleri'nden dokunun;
Android Chrome: adres çubuğuna <b>sb</b> yazıp öneriden seçin). Çalışmazsa G2'yi masaüstü tarayıcısında telefon
görünümüyle (390×844) ve konsolda <code>qp.seen()</code> ile yapın, formda gösterimi "masaüstü" yazın.</p>
<script>
const h = document.getElementById('host');
try { h.value = localStorage.getItem('qp-host') || ''; } catch (e) {}
h.onchange = () => { try { localStorage.setItem('qp-host', h.value.trim()); } catch (e) {} };
for (const a of document.querySelectorAll('a[data-q]')) a.onclick = (e) => {
  e.preventDefault();
  const b = h.value.trim().replace(/\\/+$/, '');
  if (!b) { alert('Önce gösterimin adresini yazın.'); return; }
  location.href = b + '/' + a.dataset.q;
};
</script>
</body></html>
`;
}
