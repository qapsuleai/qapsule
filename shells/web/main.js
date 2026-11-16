// Qapsule demo shell (R4/T5). Thin by rule (shells/web/ARCH.md): it moves bytes between
// the host's stream, the core (WASM) and the screen. It does no arithmetic, sorts nothing,
// builds no text from domain data and decides no layout of domain elements; every
// message it sends is made by the core, every pixel of a scene is painted by the core.

import init, { Shell } from './pkg/qp_wasm.js';

const L = {
  tr: {
    synthetic: 'Sentetik veri', tab_feed: 'Akış', tab_settings: 'Ayarlar', tab_metrics: 'Ölçüm',
    empty: 'Henüz kapsül yok. Oynatmayı başlatın.', back: '‹ Geri', lang: 'Dil', theme: 'Tema',
    dark: 'Koyu', light: 'Açık', sensitivity: 'Hassasiyet', default: 'Grubun varsayılanı',
    important: 'Önemli', balanced: 'Dengeli', all: 'Hepsi', alphabet: 'İşaret alfabesi',
    face: 'Yüz tasarımı', noent: 'Veri yetkisi yok (gösterim)', audience: 'İzleme listesi ve pozisyonlar',
    group: 'Kullanıcı', habituated: 'Alışkın', future: 'Geleceğin yatırımcısı', symbol: 'Sembol',
    watch: 'İzle', position: 'Pozisyon', apply: 'Uygula', phone: 'Telefondan aç:',
    copy: 'Özeti kopyala', copied: 'Kopyalandı', play: '▶', pause: '❚❚', step: '+1', go: 'Git',
    none: '—', long: 'Uzun', short: 'Kısa', capsule: 'Kapsül', chart: 'Grafik', evidence: 'Kanıt',
    hint_capsule: 'Grafiğe inmek için dokunun.', hint_chart: 'Sürükleyin, kıstırın; kanıt için işarete ya da seviyeye dokunun.',
    hint_evidence: 'Kanıt', requires: 'Bu kanıt şu yetkiyi gerektirir:', rests_on: 'Dayandığı:',
    disconnected: 'Bağlantı koptu; yeniden bağlanılıyor…',
  },
  en: {
    synthetic: 'Synthetic data', tab_feed: 'Feed', tab_settings: 'Settings', tab_metrics: 'Metrics',
    empty: 'No capsules yet. Start the replay.', back: '‹ Back', lang: 'Language', theme: 'Theme',
    dark: 'Dark', light: 'Light', sensitivity: 'Sensitivity', default: "Group's default",
    important: 'Important', balanced: 'Balanced', all: 'All', alphabet: 'Glyph alphabet',
    face: 'Face design', noent: 'No data entitlement (demo)', audience: 'Watchlist and positions',
    group: 'User', habituated: 'Habituated', future: 'Future investor', symbol: 'Symbol',
    watch: 'Watch', position: 'Position', apply: 'Apply', phone: 'Open on your phone:',
    copy: 'Copy summary', copied: 'Copied', play: '▶', pause: '❚❚', step: '+1', go: 'Go',
    none: '—', long: 'Long', short: 'Short', capsule: 'Capsule', chart: 'Chart', evidence: 'Evidence',
    hint_capsule: 'Tap to open the chart.', hint_chart: 'Drag, pinch; tap the glyph or a level for the evidence.',
    hint_evidence: 'Evidence', requires: 'This evidence requires the entitlement:', rests_on: 'Rests on:',
    disconnected: 'Disconnected; reconnecting…',
  },
};

const params = new URLSearchParams(location.search);
const ui = {
  principal: params.get('principal') || 'arkin',
  lang: params.get('lang') === 'en' ? 'en' : 'tr',
  theme: params.get('theme') === 'light' ? 'light' : 'dark',
  alphabet: params.get('alphabet') || '',
  face: params.get('face') || '',
  anim: params.get('anim') !== '0',
};
const HINT = { capsule: 'hint_capsule', chart: 'hint_chart', evidence: 'hint_evidence' };
const FACE_W = 390; // layout constants of the chrome (CSS px)
const FACE_H = 240;

let core = null;
let conn = '';
let source = null;
let wasmReady = 0;
let chain = Promise.resolve();
const posts = new Set();
const faces = new Map(); // capsule id -> element
const visible = new Set();
let mode = 'idle';
let scrollTimer = 0;
let painted = false;
let info = {};

const $ = (id) => document.getElementById(id);
const t = (k) => L[ui.lang][k];

function labels() {
  document.documentElement.lang = ui.lang;
  for (const el of document.querySelectorAll('[data-l]')) el.textContent = t(el.dataset.l);
}

function toast(text) {
  const el = $('toast');
  el.textContent = text;
  el.hidden = false;
  setTimeout(() => { el.hidden = true; }, 4000);
}

// --- transport ------------------------------------------------------------------

function flush() {
  if (!core) return;
  const out = core.take_outbox();
  if (out === '[]') return;
  const url = new URL('/send', location.href);
  url.searchParams.set('conn', conn);
  const p = chain.then(() => fetch(url, { method: 'POST', body: out })).catch(() => {});
  posts.add(p);
  p.finally(() => posts.delete(p));
  chain = p;
}

function connect() {
  if (source) source.close();
  const url = new URL('/events', location.href);
  url.searchParams.set('principal', ui.principal);
  url.searchParams.set('lang', ui.lang);
  source = new EventSource(url);
  source.onmessage = (e) => onFrame(e.data);
  source.onerror = () => toast(t('disconnected'));
}

function start(frame) {
  const v = JSON.parse(frame).start;
  conn = v.conn;
  info = v.info;
  core = new Shell(JSON.stringify(v));
  core.metric_mark('wasm_ready', wasmReady);
  core.set_frame('face', FACE_W, FACE_H);
  fetch('pkg/sizes.json').then((r) => r.text()).then((s) => core.metric_env(s, navigator.userAgent)).catch(() => {});
  if (ui.theme === 'light' || ui.alphabet || ui.face) core.set_style(ui.face, ui.alphabet, ui.theme);
  faces.clear();
  $('list').replaceChildren();
  $('banner').hidden = !info.synthetic;
  $('note').textContent = info.note || '';
  $('url').textContent = info.phone || info.urls[0];
  flush();
}

function onFrame(text) {
  if (text.startsWith('{"start"')) {
    start(text);
    return;
  }
  if (!core) return;
  const t0 = performance.now();
  const ch = JSON.parse(core.on_frame(text));
  repaint(ch);
  flush();
  if (text.startsWith('{"data"')) core.metric_span('tick', t0, performance.now());
}

// --- painting ---------------------------------------------------------------------

function put(canvas, px, w, h) {
  if (!px.length) return false;
  canvas.width = w;
  canvas.height = h;
  canvas.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(px.buffer, px.byteOffset, px.length), w, h), 0, 0);
  return true;
}

function paintFace(id, force) {
  const el = faces.get(id);
  if (!el || !el.firstChild || el.firstChild.tagName !== 'CANVAS') return;
  const t0 = performance.now();
  if (put(el.firstChild, core.face_rgba(id, force), FACE_W, FACE_H)) {
    core.metric_span('face', t0, performance.now());
    if (!painted) {
      painted = true;
      core.metric_mark('first_capsule', performance.now());
    }
  }
}

const io = new IntersectionObserver((entries) => {
  for (const e of entries) {
    const id = e.target.dataset.id;
    if (e.isIntersecting) {
      visible.add(id);
      paintFace(id, true);
    } else {
      visible.delete(id);
    }
  }
}, { root: $('feed') });

function faceElement(item) {
  const el = document.createElement('div');
  el.className = 'face';
  el.dataset.id = item.id;
  if (item.scene) {
    const c = document.createElement('canvas');
    c.width = FACE_W;
    c.height = FACE_H;
    c.dataset.target = item.target;
    pointers(c);
    el.append(c);
  } else {
    const p = document.createElement('p');
    p.textContent = item.text;
    p.dataset.target = item.target;
    pointers(p);
    el.append(p);
  }
  io.observe(el);
  return el;
}

function renderFeed() {
  const items = JSON.parse(core.feed());
  const nodes = [];
  const keep = new Set();
  for (const item of items) {
    let el = faces.get(item.id);
    const isCanvas = el && el.firstChild.tagName === 'CANVAS';
    if (!el || isCanvas !== item.scene) {
      if (el) io.unobserve(el);
      el = faceElement(item);
      faces.set(item.id, el);
    } else if (!item.scene) {
      el.firstChild.textContent = item.text;
    }
    keep.add(item.id);
    nodes.push(el);
  }
  for (const [id, el] of faces) {
    if (!keep.has(id)) {
      io.unobserve(el);
      faces.delete(id);
      visible.delete(id);
    }
  }
  $('list').replaceChildren(...nodes);
  $('empty').hidden = items.length !== 0;
  for (const id of visible) paintFace(id, false);
}

function renderLayer(force) {
  const open = JSON.parse(core.open());
  const layer = $('layer');
  if (open.depth === 'sentence') {
    layer.hidden = true;
    return open;
  }
  layer.hidden = false;
  $('depth').textContent = t(open.depth);
  $('opentext').textContent = open.text;
  $('hint').textContent = t(HINT[open.depth]);
  const scene = $('scene');
  scene.hidden = !open.scene;
  if (open.scene) {
    const t0 = performance.now();
    if (put(scene, core.scene_rgba(force), open.frame[0], open.frame[1])) core.metric_span('scene', t0, performance.now());
  }
  renderEvidence();
  return open;
}

function renderEvidence() {
  const rows = JSON.parse(core.evidence());
  const out = [];
  for (const r of rows) {
    const d = document.createElement('div');
    d.className = 'ev';
    const h = document.createElement('div');
    h.textContent = r.family;
    d.append(h);
    for (const [k, v] of r.values) {
      const line = document.createElement('div');
      const a = document.createElement('b');
      a.textContent = k;
      const b = document.createElement('span');
      b.textContent = v;
      line.append(a, ' ', b);
      d.append(line);
    }
    if (r.requires) {
      const q = document.createElement('div');
      q.className = 'req';
      const a = document.createElement('span');
      a.textContent = t('requires');
      const b = document.createElement('code');
      b.textContent = r.requires;
      q.append(a, ' ', b);
      d.append(q);
    }
    const o = document.createElement('div');
    const a = document.createElement('span');
    a.textContent = t('rests_on');
    const b = document.createElement('span');
    b.textContent = r.rests_on;
    o.append(a, ' ', b);
    d.append(o);
    out.push(d);
  }
  $('evidence').replaceChildren(...out);
}

function renderStatus() {
  const s = JSON.parse(core.status());
  if (!s.time) return;
  $('clock').textContent = s.time;
  $('c_step_n').textContent = s.step;
  $('c_play').textContent = s.paused || s.done ? t('play') : t('pause');
  $('c_play').dataset.paused = s.paused ? '1' : '';
  $('banner').hidden = !s.synthetic;
  if (s.note) $('note').textContent = s.note;
}

function transition(oldCanvas, snap) {
  if (!ui.anim || !oldCanvas) return;
  const a = oldCanvas.getBoundingClientRect();
  const b = $('scene').getBoundingClientRect();
  const moves = JSON.parse(core.transition(a.left, a.top, b.left, b.top));
  if (!moves.length) return;
  const fx = $('fx');
  const els = [];
  for (const m of moves) {
    const c = document.createElement('canvas');
    c.width = m.src[2];
    c.height = m.src[3];
    c.getContext('2d').drawImage(snap, m.src[0], m.src[1], m.src[2], m.src[3], 0, 0, m.src[2], m.src[3]);
    Object.assign(c.style, m.from);
    fx.append(c);
    els.push([c, m]);
  }
  $('scene').classList.add('fade');
  requestAnimationFrame(() => requestAnimationFrame(() => {
    for (const [c, m] of els) Object.assign(c.style, m.to);
    $('scene').classList.remove('fade');
    setTimeout(() => { for (const [c] of els) c.remove(); }, 300);
  }));
}

function snapshot(canvas) {
  const s = document.createElement('canvas');
  s.width = canvas.width;
  s.height = canvas.height;
  s.getContext('2d').drawImage(canvas, 0, 0);
  return s;
}

function repaint(ch) {
  if (ch.welcome) core.metric_mark('connected', performance.now());
  if (ch.error) toast(core.take_error());
  if (ch.status) renderStatus();
  let old = null;
  let snap = null;
  if (ch.transition) {
    const from = core.transition_from();
    const open = JSON.parse(core.open());
    if (from === 'sentence' && open.capsule && faces.get(open.capsule)) old = faces.get(open.capsule).firstChild;
    else if (from !== 'sentence' && !$('layer').hidden) old = $('scene');
    if (old && old.tagName === 'CANVAS') snap = snapshot(old);
    else old = null;
  }
  if (ch.feed) renderFeed();
  if (ch.scene || ch.transition) renderLayer(Boolean(ch.transition));
  if (ch.transition) transition(old, snap);
}

// --- input ------------------------------------------------------------------------

function pointers(el) {
  el.addEventListener('pointerdown', (e) => {
    el.setPointerCapture(e.pointerId);
    core.pointer(el.dataset.target, 'down', e.pointerId, e.offsetX, e.offsetY);
    flush();
  });
  el.addEventListener('pointermove', (e) => {
    if (!el.hasPointerCapture(e.pointerId)) return;
    const m = core.pointer(el.dataset.target, 'move', e.pointerId, e.offsetX, e.offsetY);
    if (m === 'drag' || m === 'pinch') mode = 'drag';
    flush();
  });
  for (const ev of ['pointerup', 'pointercancel']) {
    el.addEventListener(ev, (e) => {
      core.pointer(el.dataset.target, ev === 'pointerup' ? 'up' : 'cancel', e.pointerId, e.offsetX, e.offsetY);
      mode = 'idle';
      flush();
    });
  }
}

function wire() {
  const scene = $('scene');
  scene.dataset.target = 'scene';
  pointers(scene);
  scene.addEventListener('wheel', (e) => {
    e.preventDefault();
    core.wheel(e.deltaY, e.offsetX);
    flush();
  }, { passive: false });
  $('back').onclick = () => { core.back(); flush(); };
  for (const b of document.querySelectorAll('#tabs button')) {
    b.onclick = () => {
      for (const x of document.querySelectorAll('#tabs button')) x.classList.toggle('on', x === b);
      for (const p of document.querySelectorAll('.page')) p.classList.toggle('on', p.id === b.dataset.tab);
      if (b.dataset.tab === 'metrics') renderMetrics();
      if (b.dataset.tab === 'settings') renderSettings();
    };
  }
  $('feed').addEventListener('scroll', () => {
    mode = 'scroll';
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => { mode = 'idle'; }, 150);
  }, { passive: true });
  $('c_play').onclick = () => {
    core.replay($('c_play').dataset.paused ? '{"kind":"resume"}' : '{"kind":"pause"}');
    flush();
  };
  $('c_step').onclick = () => { core.replay('{"kind":"step","n":1}'); flush(); };
  $('c_speed').onchange = (e) => { core.replay(e.target.value); flush(); };
  $('c_go').onclick = () => {
    if ($('c_date').value) core.seek($('c_date').value);
    flush();
  };
  $('s_lang').value = ui.lang;
  $('s_lang').onchange = (e) => { ui.lang = e.target.value; labels(); connect(); };
  $('s_theme').value = ui.theme;
  $('s_theme').onchange = (e) => {
    ui.theme = e.target.value;
    document.body.dataset.theme = ui.theme;
    core.set_style('', '', ui.theme);
    flush();
  };
  $('s_alpha').onchange = (e) => { core.set_style('', e.target.value, ''); flush(); };
  $('s_face').onchange = (e) => { core.set_style(e.target.value, '', ''); flush(); };
  $('s_sens').onchange = (e) => { core.set_sensitivity(e.target.value); flush(); };
  $('s_noent').checked = ui.principal === 'visitor';
  $('s_noent').onchange = (e) => { ui.principal = e.target.checked ? 'visitor' : 'arkin'; connect(); };
  $('s_apply').onclick = applyAudience;
  $('m_copy').onclick = copyMetrics;
}

function renderSettings() {
  const body = $('instruments').tBodies[0];
  const rows = [];
  for (const sym of JSON.parse(core.instruments())) {
    const tr = document.createElement('tr');
    tr.dataset.sym = sym;
    const a = document.createElement('td');
    a.textContent = sym;
    const b = document.createElement('td');
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    b.append(cb);
    const c = document.createElement('td');
    const sel = document.createElement('select');
    for (const v of ['', 'long', 'short']) {
      const o = document.createElement('option');
      o.value = v;
      o.textContent = v ? t(v) : t('none');
      sel.append(o);
    }
    c.append(sel);
    tr.append(a, b, c);
    rows.push(tr);
  }
  body.replaceChildren(...rows);
}

function applyAudience() {
  const choice = { group: $('s_group').value, sensitivity: $('s_sens').value, watch: [], long: [], short: [] };
  for (const tr of $('instruments').tBodies[0].rows) {
    const sym = tr.dataset.sym;
    if (tr.querySelector('input').checked) choice.watch.push(sym);
    const pos = tr.querySelector('select').value;
    if (pos) choice[pos].push(sym);
  }
  core.set_audience(JSON.stringify(choice));
  flush();
}

function renderMetrics() {
  const rows = [];
  for (const [k, v] of JSON.parse(core.metrics_rows())) {
    const tr = document.createElement('tr');
    const a = document.createElement('td');
    a.textContent = k;
    const b = document.createElement('td');
    b.textContent = v;
    tr.append(a, b);
    rows.push(tr);
  }
  $('mrows').tBodies[0].replaceChildren(...rows);
}

function copyMetrics() {
  const text = core.metrics();
  const ta = $('m_text');
  ta.value = text;
  ta.hidden = false;
  ta.select();
  const done = () => toast(t('copied'));
  if (navigator.clipboard) navigator.clipboard.writeText(text).then(done, () => document.execCommand('copy'));
  else document.execCommand('copy');
}

function frame(ts) {
  if (core) core.metric_frame(mode, ts);
  requestAnimationFrame(frame);
}

// --- page API for the test harnesses (screens.mjs, tests/*.mjs) -------------------

window.qp = {
  ready: () => Boolean(core) && core.idle() && posts.size === 0,
  digests: () => core.digests(),
  metrics: () => core.metrics(),
  status: () => core.status(),
  open: () => core.open(),
  feed: () => core.feed(),
  sent: () => core.sent(),
  png: (id) => {
    const c = document.getElementById(id) || (faces.get(id) && faces.get(id).firstChild);
    const img = c.getContext('2d').getImageData(0, 0, c.width, c.height);
    return Array.from(core.png(new Uint8Array(img.data.buffer), c.width, c.height));
  },
  repaint: () => { if (core) { renderFeed(); renderLayer(true); renderStatus(); } },
};

async function main() {
  document.body.dataset.theme = ui.theme;
  if (!ui.anim) document.body.classList.add('noanim');
  labels();
  wire();
  await init();
  wasmReady = performance.now();
  connect();
  requestAnimationFrame(frame);
}

main();
