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
    face: 'Yüz tasarımı', feedl: 'Akış düzeni', transition: 'Geçiş', outcome: 'Sonuç işareti', noent: 'Veri yetkisi yok (gösterim)', audience: 'İzleme listesi ve pozisyonlar',
    group: 'Kullanıcı', habituated: 'Alışkın', future: 'Geleceğin yatırımcısı', symbol: 'Sembol',
    watch: 'İzle', position: 'Pozisyon', apply: 'Uygula', phone: 'Telefondan aç:',
    copy: 'Özeti kopyala', copied: 'Kopyalandı', play: '▶', pause: '❚❚', step: '+1', go: 'Git',
    none: '—', long: 'Uzun', short: 'Kısa', capsule: 'Kapsül', chart: 'Grafik', evidence: 'Kanıt',
    hint_capsule: 'Grafiğe inmek için dokunun.', hint_chart: 'Sürükleyin, kıstırın; kanıt için işarete ya da seviyeye dokunun.',
    hint_evidence: 'Kanıt', requires: 'Bu kanıt şu yetkiyi gerektirir:', rests_on: 'Dayandığı:',
    disconnected: 'Bağlantı koptu; yeniden bağlanılıyor…',
    lan_warn: 'Uyarı: yerel ağdaki cihazlar bu gösterimi görebilir.',
    lan_hint: 'Telefondan açmak için gösterimi şöyle başlatın:',
  },
  en: {
    synthetic: 'Synthetic data', tab_feed: 'Feed', tab_settings: 'Settings', tab_metrics: 'Metrics',
    empty: 'No capsules yet. Start the replay.', back: '‹ Back', lang: 'Language', theme: 'Theme',
    dark: 'Dark', light: 'Light', sensitivity: 'Sensitivity', default: "Group's default",
    important: 'Important', balanced: 'Balanced', all: 'All', alphabet: 'Glyph alphabet',
    face: 'Face design', feedl: 'Feed layout', transition: 'Transition', outcome: 'Outcome mark', noent: 'No data entitlement (demo)', audience: 'Watchlist and positions',
    group: 'User', habituated: 'Habituated', future: 'Future investor', symbol: 'Symbol',
    watch: 'Watch', position: 'Position', apply: 'Apply', phone: 'Open on your phone:',
    copy: 'Copy summary', copied: 'Copied', play: '▶', pause: '❚❚', step: '+1', go: 'Go',
    none: '—', long: 'Long', short: 'Short', capsule: 'Capsule', chart: 'Chart', evidence: 'Evidence',
    hint_capsule: 'Tap to open the chart.', hint_chart: 'Drag, pinch; tap the glyph or a level for the evidence.',
    hint_evidence: 'Evidence', requires: 'This evidence requires the entitlement:', rests_on: 'Rests on:',
    disconnected: 'Disconnected; reconnecting…',
    lan_warn: 'Warning: devices on the local network can see this demo.',
    lan_hint: 'To open it on a phone, start the demo with:',
  },
};

const params = new URLSearchParams(location.search);
const ui = {
  principal: params.get('principal') || 'arkin',
  lang: params.get('lang') === 'en' ? 'en' : 'tr',
  theme: params.get('theme') === 'light' ? 'light' : 'dark',
  alphabet: params.get('alphabet') || '',
  face: params.get('face') || '',
  feed: params.get('feed') || '',
  transition: params.get('transition') || '',
  outcome: params.get('outcome') || '',
  anim: params.get('anim') !== '0',
};
const HINT = { capsule: 'hint_capsule', chart: 'hint_chart', evidence: 'hint_evidence' };
// Face and scene sizes come from the core (`face_size`, `open().px/style`): the face
// design states its height, the core paints at the device scale (HiDPI).
let faceSize = null;
let faceKey = '';
let lastMove = null; // the last transition's canvases, for the harness's frozen frames

let core = null;
let conn = '';
let token = '';
let source = null;
let wasmReady = 0;
const early = []; // frames that arrived before the WASM was ready, in order
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
  const headers = { 'Content-Type': 'application/json', 'X-Qp-Token': token };
  const p = chain.then(() => fetch(url, { method: 'POST', body: out, headers })).catch(() => {});
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
  source.onmessage = (e) => {
    // (R4/T6) The stream opens while the WASM still loads; its first frames wait here.
    if (!wasmReady) early.push(e.data);
    else onFrame(e.data);
  };
  source.onerror = () => toast(t('disconnected'));
}

function start(frame) {
  const v = JSON.parse(frame).start;
  conn = v.conn;
  token = v.token;
  info = v.info;
  core = new Shell(JSON.stringify(v));
  core.metric_mark('start_frame', performance.now());
  core.metric_mark('wasm_ready', wasmReady);
  core.set_dpr(window.devicePixelRatio);
  sizeFaces();
  fetch('pkg/sizes.json').then((r) => r.text()).then((s) => core.metric_env(s, navigator.userAgent)).catch(() => {});
  if (ui.theme === 'light' || ui.alphabet || ui.face) core.set_style(ui.face, ui.alphabet, ui.theme);
  if (ui.feed || ui.transition || ui.outcome) core.set_look(JSON.stringify({ feed: ui.feed, transition: ui.transition, outcome: ui.outcome }));
  faces.clear();
  $('list').replaceChildren();
  $('banner').hidden = !info.synthetic;
  $('note').textContent = info.note || '';
  $('url').textContent = info.phone || info.urls[0];
  $('lanwarn').hidden = !info.lan;
  $('lanhint').hidden = Boolean(info.lan);
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

function sizeFaces() {
  faceKey = core.face_size();
  faceSize = JSON.parse(faceKey);
}

function paintFace(id, force) {
  const el = faces.get(id);
  if (!el || !el.firstChild || el.firstChild.tagName !== 'CANVAS') return;
  const t0 = performance.now();
  if (put(el.firstChild, core.face_rgba(id, force), faceSize.px[0], faceSize.px[1])) {
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
  el.dataset.size = faceKey;
  if (item.scene) {
    const c = document.createElement('canvas');
    c.width = faceSize.px[0];
    c.height = faceSize.px[1];
    Object.assign(c.style, faceSize.style);
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

function markOf(el, text) {
  let m = el.querySelector('.mark');
  if (!text) {
    if (m) m.remove();
    return;
  }
  if (!m) {
    m = document.createElement('span');
    m.className = 'mark';
    el.append(m);
  }
  m.textContent = text;
}

// (R4/T6) The first paint does not wait for the intersection observer's first callback
// (a frame or more later): the faces already inside the list's box are painted now. The
// observer keeps the set up to date from then on.
function firstFaces(nodes) {
  const box = $('feed').getBoundingClientRect();
  for (const el of nodes) {
    if (!el.dataset.id) continue; // group headers (M9 feed layouts) are not faces
    const r = el.getBoundingClientRect();
    if (r.bottom > box.top && r.top < box.bottom) {
      visible.add(el.dataset.id);
      paintFace(el.dataset.id, true);
    }
  }
}

function renderFeed() {
  const items = JSON.parse(core.feed());
  sizeFaces();
  const nodes = [];
  const keep = new Set();
  for (const item of items) {
    if (item.header !== undefined) {
      // A layout row: the feed layout's group header, its text from the core.
      const h = document.createElement('h4');
      h.className = 'fhead';
      h.textContent = item.header;
      nodes.push(h);
      continue;
    }
    let el = faces.get(item.id);
    const isCanvas = el && el.firstChild.tagName === 'CANVAS';
    if (!el || isCanvas !== item.scene || el.dataset.size !== faceKey) {
      if (el) io.unobserve(el);
      el = faceElement(item);
      faces.set(item.id, el);
    } else if (!item.scene) {
      el.firstChild.textContent = item.text;
    }
    markOf(el, item.mark);
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
  if (nodes.length) core.metric_mark('feed', performance.now());
  $('empty').hidden = nodes.length !== 0;
  if (!painted) firstFaces(nodes);
  for (const id of visible) paintFace(id, false);
}

function renderLayer(force) {
  const open = JSON.parse(core.open());
  document.body.classList.toggle('open', open.depth !== 'sentence');
  if (open.depth === 'sentence') return open;
  $('depth').textContent = t(open.depth);
  $('toevidence').hidden = open.depth === 'evidence';
  $('opentext').textContent = open.text;
  $('hint').textContent = t(HINT[open.depth]);
  const scene = $('scene');
  scene.hidden = !open.scene;
  if (open.scene) {
    const t0 = performance.now();
    Object.assign(scene.style, open.style);
    if (put(scene, core.scene_rgba(force), open.px[0], open.px[1])) core.metric_span('scene', t0, performance.now());
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

// The depth transition: the core's design (`transition_frame`) says, per frame, which
// element of the old (or new) scene is drawn where and how opaque, and how opaque the new
// scene is; the shell copies those pixels into those boxes. The progress comes from the
// core too (`transition_progress` turns the frame timestamp into 0..1).
function moveCanvases(m) {
  const els = [];
  const frame = JSON.parse(core.transition_frame(0, m.a.left, m.a.top, m.b.left, m.b.top));
  for (const e of frame.elements) {
    const src = e.source === 'after' ? m.after : m.snap;
    const c = document.createElement('canvas');
    c.width = e.src[2];
    c.height = e.src[3];
    c.getContext('2d').drawImage(src, e.src[0], e.src[1], e.src[2], e.src[3], 0, 0, e.src[2], e.src[3]);
    $('fx').append(c);
    els.push(c);
  }
  return els;
}

function showFrame(m, els, p) {
  const frame = JSON.parse(core.transition_frame(p, m.a.left, m.a.top, m.b.left, m.b.top));
  frame.elements.forEach((e, i) => { if (els[i]) Object.assign(els[i].style, e.style); });
  $('scene').style.opacity = frame.scene_opacity;
  return frame.done;
}

function clearFrame(els) {
  for (const c of els) c.remove();
  $('scene').style.opacity = '';
}

function transition(oldCanvas, snap, a) {
  if (!oldCanvas) return;
  const b = $('scene').getBoundingClientRect();
  lastMove = { a, b, snap, after: snapshot($('scene')) };
  if (!ui.anim) return;
  const m = lastMove;
  const els = moveCanvases(m);
  if (!els.length) return;
  const step = (ts) => {
    if (showFrame(m, els, core.transition_progress(ts))) clearFrame(els);
    else requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
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
  let rect = null;
  if (ch.transition) {
    const from = core.transition_from();
    const open = JSON.parse(core.open());
    if (from === 'sentence' && open.capsule && faces.get(open.capsule)) old = faces.get(open.capsule).firstChild;
    else if (from !== 'sentence' && document.body.classList.contains('open')) old = $('scene');
    if (old && old.tagName === 'CANVAS') {
      snap = snapshot(old);
      rect = old.getBoundingClientRect(); // before the layer covers the feed
    } else old = null;
  }
  if (ch.feed) renderFeed();
  if (ch.scene || ch.transition) renderLayer(Boolean(ch.transition));
  if (ch.transition) transition(old, snap, rect);
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
  $('toevidence').onclick = () => { core.open_to('evidence'); flush(); };
  $('opentext').dataset.target = 'scene';
  pointers($('opentext'));
  for (const b of document.querySelectorAll('#tabs button')) {
    b.onclick = () => {
      for (const x of document.querySelectorAll('#tabs button')) x.classList.toggle('on', x === b);
      for (const p of document.querySelectorAll('.page')) p.classList.toggle('on', p.id === b.dataset.tab);
      document.body.dataset.tab = b.dataset.tab;
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
  const looks = { feed: 's_feed', transition: 's_transition', outcome: 's_outcome' };
  for (const k of Object.keys(looks)) {
    $(looks[k]).onchange = (e) => { core.set_look(JSON.stringify({ [k]: e.target.value })); flush(); };
  }
  $('s_sens').onchange = (e) => { core.set_sensitivity(e.target.value); flush(); };
  $('s_noent').checked = ui.principal === 'visitor';
  $('s_noent').onchange = (e) => { ui.principal = e.target.checked ? 'visitor' : 'arkin'; connect(); };
  $('s_apply').onclick = applyAudience;
  $('m_copy').onclick = copyMetrics;
}

function fillChoices() {
  // The names each setting accepts come from the core's registries.
  const ch = JSON.parse(core.choices());
  const look = JSON.parse(core.look());
  const sel = { face: 's_face', alphabet: 's_alpha', feed: 's_feed', transition: 's_transition', outcome: 's_outcome' };
  for (const k of Object.keys(sel)) {
    const el = $(sel[k]);
    const opts = [];
    for (const n of ch[k]) {
      const o = document.createElement('option');
      o.value = n;
      o.textContent = n;
      opts.push(o);
    }
    el.replaceChildren(...opts);
    el.value = look[k] || ch[k][0];
  }
}

function renderSettings() {
  fillChoices();
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
  // (R4/H-M8910) for the measurement harness (rounds/R4/experiments/harness/m8910):
  look: () => core.look(),
  seen: () => { core.mark_seen(); flush(); },
  openCapsule: (id) => { core.open_capsule(id); flush(); },
  openTo: (to) => { core.open_to(to); flush(); },
  // Shows the last transition frozen at progress `p` (a value the harness gives).
  freeze: (p) => {
    if (window.qp.frozen) clearFrame(window.qp.frozen);
    if (!lastMove) return 0;
    window.qp.frozen = moveCanvases(lastMove);
    showFrame(lastMove, window.qp.frozen, p);
    return window.qp.frozen.length;
  },
  unfreeze: () => { if (window.qp.frozen) clearFrame(window.qp.frozen); window.qp.frozen = null; },
  frozen: null,
};

async function main() {
  document.body.dataset.theme = ui.theme;
  if (!ui.anim) document.body.classList.add('noanim');
  labels();
  wire();
  // (R4/T6) Connect first: the host prepares the start frame while the WASM downloads
  // and compiles (they used to run one after the other).
  connect();
  await init();
  wasmReady = performance.now();
  while (early.length) onFrame(early.shift());
  requestAnimationFrame(frame);
}

main();
