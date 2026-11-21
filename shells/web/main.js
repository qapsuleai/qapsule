// Qapsule demo shell (R4/T5). Thin by rule (shells/web/ARCH.md): it moves bytes between
// the host's stream, the core (WASM) and the screen. It does no arithmetic, sorts nothing,
// builds no text from domain data and decides no layout of domain elements; every
// message it sends is made by the core, every pixel of a scene is painted by the core.
// (RP3/T2) How a scene reaches the canvas is the paint path's (shells/web/paint/,
// `?paint=<name>`, default `ref` = R4's painting).

import init, { Shell } from './pkg/qp_wasm.js';
import { choose, supportedNames } from './paint/index.js';

const L = {
  tr: {
    synthetic: 'Sentetik veri', tab_feed: 'Akış', tab_settings: 'Ayarlar', tab_metrics: 'Ölçüm',
    empty: 'Henüz kapsül yok. Oynatmayı başlatın.', back: '‹ Geri', lang: 'Dil', theme: 'Tema',
    dark: 'Koyu', light: 'Açık', sensitivity: 'Hassasiyet', default: 'Grubun varsayılanı',
    important: 'Önemli', balanced: 'Dengeli', all: 'Hepsi', alphabet: 'İşaret alfabesi',
    face: 'Yüz tasarımı', lookl: 'Görünüş', voicel: 'Dil', openl: 'Açılmış kapsül (M20)', feedl: 'Akış düzeni', orderl: 'Akış sırası (M19)', rulel: 'Yazı tura eşiği (M19)', transition: 'Geçiş', motionl: 'Hareket (M21)', reduce: 'Hareketi azalt', outcome: 'Sonuç işareti', storyl: 'Hikâye çizelgesi', hideclosed: 'Kapanmış hikâyeleri gizle', noent: 'Veri yetkisi yok (gösterim)', audience: 'İzleme listesi ve pozisyonlar',
    group: 'Kullanıcı', habituated: 'Alışkın', future: 'Geleceğin yatırımcısı', symbol: 'Sembol',
    watch: 'İzle', position: 'Pozisyon', apply: 'Uygula', phone: 'Telefondan aç:',
    copy: 'Özeti kopyala', copied: 'Kopyalandı', play: '▶', pause: '❚❚', step: '+1', go: 'Git',
    none: '—', long: 'Uzun', short: 'Kısa', capsule: 'Kapsül', chart: 'Grafik', evidence: 'Kanıt',
    hint_capsule: 'Grafiğe inmek için dokunun.', hint_chart: 'Sürükleyin, kıstırın; kanıt için işarete ya da seviyeye dokunun.',
    hint_evidence: 'Kanıt', requires: 'Bu kanıt şu yetkiyi gerektirir:', rests_on: 'Dayandığı:',
    disconnected: 'Bağlantı koptu; yeniden bağlanılıyor…',
    lan_warn: 'Uyarı: yerel ağdaki cihazlar bu gösterimi görebilir.',
    lan_hint: 'Telefondan açmak için gösterimi şöyle başlatın:',
    seen: 'Son bakışımı şimdi işaretle', seen_done: 'Son bakış işaretlendi',
    reset: 'Sıfırla', accept: 'Kabul et', reject: 'Reddet',
    m_hint: 'Telefonda: Sıfırla → 10 sn dokunmadan bekleyin → Akış\'ta 10 sn kaydırın → bir kapsülü grafiğe açıp 10 sn sürükleyin ve kıstırın → buraya dönüp Özeti kopyala.',
    compare: 'Karşılaştır', b_copy: 'Karşılaştırmayı kopyala',
    b_hint: 'Bütün boyama yollarını sırayla aynı senaryoda çalıştırır (durağan, kaydırma, grafik sürükleme, grafik açıkken tik; yol başına yaklaşık 40 sn, en çok 3 dk). Dokunmayın; bitince tablo ve kopyalama düğmesi çıkar.',
  },
  en: {
    synthetic: 'Synthetic data', tab_feed: 'Feed', tab_settings: 'Settings', tab_metrics: 'Metrics',
    empty: 'No capsules yet. Start the replay.', back: '‹ Back', lang: 'Language', theme: 'Theme',
    dark: 'Dark', light: 'Light', sensitivity: 'Sensitivity', default: "Group's default",
    important: 'Important', balanced: 'Balanced', all: 'All', alphabet: 'Glyph alphabet',
    face: 'Face design', lookl: 'Look', voicel: 'Voice', openl: 'Open capsule (M20)', feedl: 'Feed layout', orderl: 'Feed order (M19)', rulel: 'Coin-flip threshold (M19)', transition: 'Transition', motionl: 'Motion (M21)', reduce: 'Reduce motion', outcome: 'Outcome mark', storyl: 'Story chart', hideclosed: 'Hide closed stories', noent: 'No data entitlement (demo)', audience: 'Watchlist and positions',
    group: 'User', habituated: 'Habituated', future: 'Future investor', symbol: 'Symbol',
    watch: 'Watch', position: 'Position', apply: 'Apply', phone: 'Open on your phone:',
    copy: 'Copy summary', copied: 'Copied', play: '▶', pause: '❚❚', step: '+1', go: 'Go',
    none: '—', long: 'Long', short: 'Short', capsule: 'Capsule', chart: 'Chart', evidence: 'Evidence',
    hint_capsule: 'Tap to open the chart.', hint_chart: 'Drag, pinch; tap the glyph or a level for the evidence.',
    hint_evidence: 'Evidence', requires: 'This evidence requires the entitlement:', rests_on: 'Rests on:',
    disconnected: 'Disconnected; reconnecting…',
    lan_warn: 'Warning: devices on the local network can see this demo.',
    lan_hint: 'To open it on a phone, start the demo with:',
    seen: 'Mark my last look now', seen_done: 'Last look marked',
    reset: 'Reset', accept: 'Accept', reject: 'Reject',
    m_hint: 'On a phone: Reset → wait 10 s without touching → scroll the Feed for 10 s → open a capsule to its chart, drag and pinch for 10 s → come back here and Copy summary.',
    compare: 'Compare', b_copy: 'Copy comparison',
    b_hint: 'Runs every paint path in turn through the same scenario (idle, scroll, chart drag, ticks with the chart open; about 40 s per path, at most 3 min). Do not touch; a table and a copy button appear at the end.',
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
  // (R5/M19) The feed order (a presentation setting of the core) and the feed rule the
  // host applies to this page's principal (sent on connect); names from `choices()`.
  feed_order: params.get('feed_order') || '',
  feed_rule: params.get('feed_rule') || '',
  transition: params.get('transition') || '',
  outcome: params.get('outcome') || '',
  // (R5/T3) The look (design record); the names come from the core (`choices().look`).
  look: params.get('look') || '',
  // (R5/m17-int) The capsule voice (`?voice=marla|selvi|r4`; the core's default: `marla`).
  voice: params.get('voice') || '',
  // (R5/M18) The story chart design at chart depth (`choices().story`; default `r4`: none).
  story: params.get('story') || '',
  // (R5/M20) The open capsule design (`?open=tesir`; the core's default: `r4`, today's).
  open: params.get('open') || '',
  // (RP4/T7) Rows carry the event's time (`1`, the presentation's; `0`: as before).
  clock: params.get('clock') || '',
  // (R5/M21) The motion design (`?motion=`, names from `choices().motion`) and reduced
  // motion: the address or the settings page override the system setting, which the page
  // only forwards to the core (the core decides what moves). (R5/S) One parameter,
  // `?reduced_motion=1|0`; `?reduce=1|0` and `?anim=0` are its earlier names.
  motion: params.get('motion') || '',
  reduce: params.get('reduced_motion') || params.get('reduce') || (params.get('anim') === '0' ? '1' : ''),
  // (R4/S1) Human-test settings: last look (`now` or a step) and the audience, given to
  // the core as they are (it reads the comma lists); applied again on every start frame,
  // so a backward seek (a new authority) keeps them.
  seen: params.get('seen') || '',
  group: params.get('group') || '',
  watch: params.get('watch') || '',
  long: params.get('long') || '',
  short: params.get('short') || '',
};
const HINT = { capsule: 'hint_capsule', chart: 'hint_chart', evidence: 'hint_evidence' };
// Face and scene sizes come from the core (`face_size`, `open().px/style`): the face
// design states its height, the core paints at the device scale (HiDPI).
let faceSize = null;
let faceKey = '';
let lastMove = null; // the last transition's canvases, for the harness's frozen frames
const changes = new Map(); // (R5/M21) capsule id -> its last change's sources, for frozen frames
const reducedQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
// (R5/M21) The motion overlay's box elements by key (the core names and styles them).
const marks = new Map();

let core = null;
let conn = '';
let token = '';
let source = null;
let wasmReady = 0;
const early = []; // frames that arrived before the WASM was ready, in order
let chain = Promise.resolve();
const posts = new Set();
const faces = new Map(); // capsule id -> element
let mode = 'idle';
let scrollTimer = 0;
let zoomTimer = 0;
let painted = false;
let info = {};
// (RP3/T2) The paint path (`?paint=`; unknown or unsupported → `ref`, reported; R5/T0: the
// removed `cpu` → `canvas`, reported).
// (RP3/S) Default `canvas` (proposal, rounds/RP3/experiments/S.md); `ref` stays the
// deterministic reference and the fallback.
const DEFAULT_PAINT = 'canvas';
let pick = choose(params.get('paint') || DEFAULT_PAINT);
let paint = pick.path;
// (RP3/S) A path's own repaint request (font loaded, GPU device ready, gesture settled)
// waits for the next paint pass (M14: at most one paint per animation frame).
let repaintWanted = false;
const paintEnv = { repaint: () => { repaintWanted = true; } };
let benching = false;

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
  // (RP3/M15) `?mode=replica`: the R4 full replica (the measurement's baseline); the host
  // makes every other connection a viewer.
  if (params.get('mode')) url.searchParams.set('mode', params.get('mode'));
  if (ui.feed_rule) url.searchParams.set('feed_rule', ui.feed_rule);
  // (R5/S) `?feed_exempt=0`: the feed rule without the K42 exemption (human test form B;
  // the host reads it as it is).
  if (params.get('feed_exempt')) url.searchParams.set('feed_exempt', params.get('feed_exempt'));
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
  // (RP3/T2, RP3/S) A later start frame (reconnect, language change, backward seek)
  // builds a new core; it takes over the page's metrics, so the opening timeline stays the
  // first one (R4 lost it: `open_ms.start_frame` 836,034 ms next to `wasm_ready` 79 ms).
  // (RP3/T4) The old core is freed **before** the new one is built (its metrics carried
  // over), else two replicas share the WASM memory, whose peak is never given back
  // (rounds/RP3/experiments/T4.md, T4-verify.md).
  let carry = null;
  if (core) {
    carry = core.metric_detach();
    core.free();
    core = null;
  }
  const next = new Shell(JSON.stringify(v));
  if (carry) next.metric_attach(carry);
  core = next;
  core.metric_mark('start_frame', performance.now());
  core.metric_mark('wasm_ready', wasmReady);
  core.set_dpr(window.devicePixelRatio);
  sizeFaces();
  core.set_paint(pick.requested, pick.used, pick.reason);
  paint.init(core, paintEnv);
  fetch('pkg/sizes.json').then((r) => r.text()).then((s) => core.metric_env(s, navigator.userAgent)).catch(() => {});
  // (R4/D) Device facts for the summary, as the browser reports them (absent: 0).
  core.metric_device('cores', navigator.hardwareConcurrency || 0);
  core.metric_device('device_memory_gb', navigator.deviceMemory || 0);
  core.metric_device('screen_w', screen.width);
  core.metric_device('screen_h', screen.height);
  core.metric_device('dpr', window.devicePixelRatio);
  // (RP3/M14) The page's mode, as the browser reports it: on iOS a page that is not a
  // secure context runs without the JIT and without WebGPU (the core warns).
  core.metric_flag('secure_context', window.isSecureContext);
  core.metric_flag('webgpu', Boolean(navigator.gpu));
  sampleMemory();
  if (ui.theme === 'light' || ui.alphabet || ui.face) core.set_style(ui.face, ui.alphabet, ui.theme);
  // (R5/S) `face` goes with the look too: a look brings its companion face unless the
  // reader named one (the core decides).
  if (ui.feed || ui.transition || ui.outcome || ui.look || ui.feed_order || ui.feed_rule || ui.voice || ui.story || ui.open || ui.motion || ui.clock) core.set_look(JSON.stringify({ face: ui.face, feed: ui.feed, transition: ui.transition, outcome: ui.outcome, look: ui.look, feed_order: ui.feed_order, feed_rule: ui.feed_rule, voice: ui.voice, story: ui.story, open: ui.open, motion: ui.motion, clock: ui.clock }));
  core.set_reduced_motion(reducedMotion());
  if (ui.watch || ui.long || ui.short || ui.group) core.set_audience(JSON.stringify({ group: ui.group, watch: ui.watch, long: ui.long, short: ui.short }));
  if (ui.seen) core.seen_at(ui.seen);
  faces.clear();
  $('list').replaceChildren();
  $('banner').hidden = !info.synthetic;
  $('note').textContent = info.note || '';
  $('url').textContent = info.phone || info.urls[0];
  $('lanwarn').hidden = !info.lan;
  $('lanhint').hidden = Boolean(info.lan) || Boolean(info.live);
  // (R6/T7) Live data has no timeline: the replay controls are not shown.
  for (const el of $('controls').children) el.hidden = Boolean(info.live);
  flush();
}

// (RP3/M14) A frame is applied to the core at once; what it changed waits in the core for
// the next animation frame's paint pass (`paintPass`), so that several frames within one
// display frame cost one paint. `tick` / `msg` time the core's work of one frame only.
function onFrame(text) {
  if (text.startsWith('{"start"')) {
    start(text);
    return;
  }
  if (!core) return;
  const t0 = performance.now();
  core.on_frame(text, t0);
  flush();
  if (text.startsWith('{"data"')) core.metric_span('tick', t0, performance.now());
  else core.metric_span('msg', t0, performance.now());
}

// (RP3/M14) The one paint pass of an animation frame, for every change since the last.
function paintPass() {
  const text = core.take_changes();
  if (!text && !repaintWanted) return;
  const t0 = performance.now();
  if (text) repaint(JSON.parse(text));
  if (repaintWanted) {
    repaintWanted = false;
    renderFeed();
    renderLayer(true);
  }
  core.metric_pass(t0, performance.now());
}

// --- painting ---------------------------------------------------------------------

function sizeFaces() {
  faceKey = core.face_size();
  faceSize = JSON.parse(faceKey);
}

function paintFace(id, force) {
  const el = faces.get(id);
  if (!el || !el.firstChild || el.firstChild.tagName !== 'CANVAS') return;
  const t0 = performance.now();
  if (paint.paintFace(el.firstChild, id, force, faceSize.px)) {
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
    // (RP3/M15) A viewer asks the authority for the faces on screen only.
    core.visible(id, e.isIntersecting);
    if (e.isIntersecting) paintFace(id, true);
  }
  flush();
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

// (RP3/M15, K48) The trust mark of a feed capsule (text from the core; absent in the full
// replica mode).
function trustOf(el, text) {
  let m = el.querySelector('.trust');
  if (!text) {
    if (m) m.remove();
    return;
  }
  if (!m) {
    m = document.createElement('span');
    m.className = 'trust';
    el.append(m);
  }
  m.textContent = text;
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
      core.visible(el.dataset.id, true);
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
    trustOf(el, item.trust);
    keep.add(item.id);
    nodes.push(el);
  }
  for (const [id, el] of faces) {
    if (!keep.has(id)) {
      io.unobserve(el);
      faces.delete(id);
      core.visible(id, false);
    }
  }
  $('list').replaceChildren(...nodes);
  if (nodes.length) core.metric_mark('feed', performance.now());
  $('empty').hidden = nodes.length !== 0;
  if (!painted) firstFaces(nodes);
  // (RP3/M14) The core names the faces to repaint: on screen, not behind the open layer.
  for (const id of JSON.parse(core.faces_to_paint())) paintFace(id, false);
}

function renderLayer(force) {
  const open = JSON.parse(core.open());
  document.body.classList.toggle('open', open.depth !== 'sentence');
  if (open.depth === 'sentence') return open;
  $('depth').textContent = t(open.depth);
  $('trust').textContent = open.trust || '';
  $('toevidence').hidden = open.depth === 'evidence';
  $('opentext').textContent = open.text;
  // (R5/M20 nobra) An open capsule design may give its own hint (text from the core).
  $('hint').textContent = open.hint || t(HINT[open.depth]);
  const scene = $('scene');
  scene.hidden = !open.scene;
  // (RP3/M14) The layer shows only on the feed tab; elsewhere the scene is not painted
  // (it is painted afresh when the feed tab comes back, `showTab`).
  if (open.scene && document.body.dataset.tab === 'feed') {
    const t0 = performance.now();
    Object.assign(scene.style, open.style);
    if (paint.paintScene(scene, force, open.px)) core.metric_span('scene', t0, performance.now());
  }
  renderEvidence();
  renderAsks(open.asks || []);
  renderComments();
  return open;
}

// (R5B) The open capsule's validated comments: label ("ajan yorumu"), origin and text as
// the core wrote them.
function renderComments() {
  const out = [];
  for (const r of JSON.parse(core.comments())) {
    const d = document.createElement('div');
    d.className = 'cmt';
    const a = document.createElement('b');
    a.textContent = r.label;
    const b = document.createElement('span');
    b.textContent = r.origin;
    const p = document.createElement('p');
    p.textContent = r.text;
    d.append(a, ' ', b, p);
    out.push(d);
  }
  $('comments').replaceChildren(...out);
}

// (R5B) The pending proposals, author and summary as the core wrote them; the reader's
// decision goes out on this page's own connection (the core makes the command).
function renderProposals() {
  const out = [];
  for (const r of JSON.parse(core.proposals())) {
    const d = document.createElement('div');
    d.className = 'prop';
    const a = document.createElement('b');
    a.textContent = r.author;
    const p = document.createElement('p');
    p.textContent = r.text;
    const row = document.createElement('div');
    const yes = document.createElement('button');
    yes.textContent = t('accept');
    yes.onclick = () => { core.decide(r.id, true); flush(); };
    const no = document.createElement('button');
    no.textContent = t('reject');
    no.onclick = () => { core.decide(r.id, false); flush(); };
    row.append(yes, no);
    d.append(a, p, row);
    out.push(d);
  }
  $('proposals').replaceChildren(...out);
}

// (R5/M20 nobra) An open capsule design's question cards for a reader without a scene (no
// data of the instrument, A32): question, answer and detail as the core wrote them; the
// detail opens on touch (a native disclosure).
function renderAsks(rows) {
  const out = [];
  for (const r of rows) {
    const d = document.createElement('details');
    d.className = 'ask';
    const s = document.createElement('summary');
    const q = document.createElement('b');
    q.textContent = r.q;
    const a = document.createElement('span');
    a.textContent = r.a;
    s.append(q, ' ', a);
    const p = document.createElement('p');
    p.textContent = r.d;
    d.append(s, p);
    out.push(d);
  }
  $('asks').replaceChildren(...out);
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
  // (R6/T3) The market's status line: text and state from the core, shown as they are.
  const m = s.market;
  $('market').hidden = !m;
  if (m) {
    $('market').textContent = m.text;
    $('market').dataset.state = m.state;
  }
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
  if (reducedMotion()) return;
  const m = lastMove;
  const els = moveCanvases(m);
  if (!els.length) return;
  const step = (ts) => {
    if (showFrame(m, els, core.transition_progress(ts))) clearFrame(els);
    else requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}

// (R5/M21) Reduced motion as the reader set it: the page's override, else the system's.
function reducedMotion() {
  if (ui.reduce) return ui.reduce === '1';
  return reducedQuery.matches;
}

// (R5/M21) A face's own change (live value, state): the core's motion design says, per
// frame, which piece of the face before (the page's copy of its canvas) or after the change
// is drawn where and how opaque, and how opaque the repainted face is; the shell copies
// those pixels. Nothing runs under reduced motion (the core queues no face then).
function changeCanvases(m) {
  const els = [];
  const frame = JSON.parse(core.face_change_frame(m.id, 0, m.left, m.top));
  if (!frame) return els;
  for (const e of frame.elements) {
    const src = e.source === 'after' ? m.after : m.snap;
    const c = document.createElement('canvas');
    c.width = e.src[2];
    c.height = e.src[3];
    if (src) c.getContext('2d').drawImage(src, e.src[0], e.src[1], e.src[2], e.src[3], 0, 0, e.src[2], e.src[3]);
    $('fx').append(c);
    els.push(c);
  }
  return els;
}

function showChange(m, els, p) {
  const frame = JSON.parse(core.face_change_frame(m.id, p, m.left, m.top));
  if (!frame) return true;
  frame.elements.forEach((e, i) => { if (els[i]) Object.assign(els[i].style, e.style); });
  m.canvas.style.opacity = frame.scene_opacity;
  return frame.done;
}

function clearChange(m, els) {
  for (const c of els) c.remove();
  m.canvas.style.opacity = '';
}

// The face before its change, as the core kept it (the canvas already shows the bar's).
function beforeCanvas(id) {
  const px = core.face_before_rgba(id);
  if (!px.length) return null;
  const c = document.createElement('canvas');
  c.width = faceSize.px[0];
  c.height = faceSize.px[1];
  c.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(px.buffer, px.byteOffset, px.length), faceSize.px[0], faceSize.px[1]), 0, 0);
  return c;
}

function faceMotion(id) {
  const el = faces.get(id);
  if (!el || !el.firstChild || el.firstChild.tagName !== 'CANVAS') return;
  paintFace(id, false);
  // (R5/M21, K52) The start of a face change: the face before (the core's pixels), the
  // copy after and the pieces (`face_motion` span).
  const t0 = performance.now();
  const snap = beforeCanvas(id);
  const canvas = el.firstChild;
  const box = canvas.getBoundingClientRect();
  const m = { id, snap, canvas, after: snapshot(canvas), left: box.left, top: box.top };
  changes.set(id, m);
  if (reducedMotion()) return;
  const els = changeCanvases(m);
  core.metric_span('face_motion', t0, performance.now());
  if (!els.length) return;
  const step = (ts) => {
    if (showChange(m, els, core.face_change_progress(id, ts))) clearChange(m, els);
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
  // (`connected` is marked by the core at the frame's arrival, `on_frame`.)
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
  // (R5/M21) The faces whose change the core animates.
  const moving = JSON.parse(core.take_face_motion());
  if (ch.feed) renderFeed();
  for (const id of moving) faceMotion(id);
  if (ch.scene || ch.transition) renderLayer(Boolean(ch.transition));
  if (ch.transition) transition(old, snap, rect);
  if (ch.gloss) renderGloss();
  if (ch.agent) { renderComments(); renderProposals(); }
}

// (R5/m17-int, K49) The glossary explanation of the term the last tap landed on: the
// core decides (the authority's `hit.term`), the page only shows its texts.
function renderGloss() {
  const g = JSON.parse(core.gloss());
  $('gloss').hidden = !g;
  if (!g) return;
  $('gloss_term').textContent = g.term;
  $('gloss_text').textContent = g.text;
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
    if (m === 'drag') mode = 'drag';
    if (m === 'pinch') mode = 'zoom';
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

function wireScene(scene) {
  scene.dataset.target = 'scene';
  pointers(scene);
  scene.addEventListener('wheel', (e) => {
    // (R5/M20) The core says whether the wheel is the chart's zoom; otherwise (the open
    // capsule's column) the layer scrolls.
    if (!core.wheel(e.deltaY, e.offsetX)) return;
    e.preventDefault();
    // (R4/D) Wheel on the chart is its zoom: its frames are measured as `zoom`.
    mode = 'zoom';
    clearTimeout(zoomTimer);
    zoomTimer = setTimeout(() => { mode = 'idle'; }, 150);
    flush();
  }, { passive: false });
}

function showTab(tab) {
  for (const x of document.querySelectorAll('#tabs button')) x.classList.toggle('on', x.dataset.tab === tab);
  for (const p of document.querySelectorAll('.page')) p.classList.toggle('on', p.id === tab);
  document.body.dataset.tab = tab;
  if (tab === 'feed' && core) renderLayer(true);
  if (tab === 'metrics') renderMetrics();
  if (tab === 'settings') renderSettings();
}

function wire() {
  wireScene($('scene'));
  $('back').onclick = () => { core.back(); flush(); };
  $('toevidence').onclick = () => { core.open_to('evidence'); flush(); };
  $('opentext').dataset.target = 'scene';
  pointers($('opentext'));
  for (const b of document.querySelectorAll('#tabs button')) {
    b.onclick = () => showTab(b.dataset.tab);
  }
  $('feed').addEventListener('scroll', () => {
    mode = 'scroll';
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => { mode = 'idle'; }, 150);
  }, { passive: true });
  // (R5/M20) The open layer scrolls (a tall capsule column): measured as `scroll`.
  $('layer').addEventListener('scroll', () => {
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
  const looks = { feed: 's_feed', transition: 's_transition', outcome: 's_outcome', look: 's_look', feed_order: 's_order', voice: 's_voice', story: 's_story', open: 's_open', motion: 's_motion' };
  for (const k of Object.keys(looks)) {
    $(looks[k]).onchange = (e) => { core.set_look(JSON.stringify({ [k]: e.target.value })); flush(); };
  }
  $('s_hideclosed').onchange = (e) => { core.set_look(JSON.stringify({ hide_closed: e.target.checked })); flush(); };
  $('s_reduce').checked = reducedMotion();
  $('s_reduce').onchange = (e) => {
    ui.reduce = e.target.checked ? '1' : '0';
    if (core) { core.set_reduced_motion(reducedMotion()); flush(); }
  };
  reducedQuery.addEventListener('change', () => {
    $('s_reduce').checked = reducedMotion();
    if (core) { core.set_reduced_motion(reducedMotion()); flush(); }
  });
  $('s_sens').onchange = (e) => { core.set_sensitivity(e.target.value); flush(); };
  // (R5/M19) The host applies the feed rule on connect: a new connection.
  $('s_rule').onchange = (e) => { ui.feed_rule = e.target.value; connect(); };
  $('s_noent').checked = ui.principal === 'visitor';
  $('s_noent').onchange = (e) => { ui.principal = e.target.checked ? 'visitor' : 'arkin'; connect(); };
  $('s_apply').onclick = applyAudience;
  $('gloss').onclick = () => { core.close_gloss(); renderGloss(); };
  $('s_seen').onclick = () => { core.mark_seen(); flush(); toast(t('seen_done')); };
  $('m_copy').onclick = copyMetrics;
  $('m_reset').onclick = () => { if (core) { core.metric_reset(); renderMetrics(); } };
  $('b_start').onclick = startBench;
  $('b_copy').onclick = () => copyText(core.bench_summary());
}

function fillChoices() {
  // The names each setting accepts come from the core's registries.
  const ch = JSON.parse(core.choices());
  const look = JSON.parse(core.look());
  const sel = { face: 's_face', alphabet: 's_alpha', feed: 's_feed', transition: 's_transition', outcome: 's_outcome', look: 's_look', feed_order: 's_order', feed_rule: 's_rule', voice: 's_voice', story: 's_story', open: 's_open', motion: 's_motion' };
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
  $('s_hideclosed').checked = Boolean(look.hide_closed);
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
  if (!core) return;
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

// (R4/D) Memory: the JS heap where the browser reports it (Chromium), the WASM memory
// from the core; sampled every second and before the summary is read.
function sampleMemory() {
  if (!core) return;
  const pm = performance.memory;
  core.metric_memory(pm ? pm.usedJSHeapSize : 0);
}

function copyMetrics() {
  sampleMemory();
  copyText(core.metrics());
}

function copyText(text) {
  const ta = $('m_text');
  ta.value = text;
  ta.hidden = false;
  ta.select();
  const done = () => toast(t('copied'));
  if (navigator.clipboard) navigator.clipboard.writeText(text).then(done, () => document.execCommand('copy'));
  else document.execCommand('copy');
}

// --- (RP3/T2) paint paths and the automatic comparison -----------------------------

// Switches to paint path `name` (the comparison's `paint` action): fresh canvases (a path
// may need its own kind of context), then everything visible is painted again.
function usePath(name) {
  pick = choose(name);
  paint = pick.path;
  core.set_paint(pick.requested, pick.used, pick.reason);
  for (const el of faces.values()) io.unobserve(el);
  faces.clear();
  const old = $('scene');
  const c = document.createElement('canvas');
  c.id = 'scene';
  c.width = old.width;
  c.height = old.height;
  old.replaceWith(c);
  wireScene(c);
  paint.init(core, paintEnv);
  renderFeed();
  firstFaces(Array.from($('list').children));
  renderLayer(true);
}

// The comparison (GOAL 8): the core drives the scenario; the page applies its actions as
// they are, once per animation frame.
function startBench() {
  if (!core || benching) return;
  // (RP3/S) `?bench_paths=` and `?bench_order=reverse` go to the core as they are (it selects).
  const cfg = {
    paths: supportedNames(), current: pick.requested, phase_ms: params.get('bench_phase') || '',
    only: params.get('bench_paths') || '', order: params.get('bench_order') || '',
  };
  core.bench_start(JSON.stringify(cfg), performance.now());
  benching = true;
  $('b_start').disabled = true;
  $('b_copy').hidden = true;
}

function benchStep(ts) {
  const feed = $('feed');
  const text = core.bench_tick(ts, feed.scrollHeight, feed.clientHeight);
  if (!text) return;
  const a = JSON.parse(text);
  if (a.tab) showTab(a.tab);
  if (a.paint) usePath(a.paint);
  if (a.scroll !== undefined) feed.scrollTop = a.scroll;
  flush();
  if (a.done) {
    benching = false;
    renderBench();
  }
}

function renderBench() {
  const tb = JSON.parse(core.bench_rows());
  if (!tb) return;
  const rows = [];
  for (const r of [tb.head, ...tb.rows]) {
    const tr = document.createElement('tr');
    for (const v of r) {
      const td = document.createElement(r === tb.head ? 'th' : 'td');
      td.textContent = v;
      tr.append(td);
    }
    rows.push(tr);
  }
  $('brows').tBodies[0].replaceChildren(...rows);
  $('brows').hidden = false;
  const notes = [];
  for (const n of tb.notes) {
    const p = document.createElement('p');
    p.textContent = n;
    notes.push(p);
  }
  $('bnotes').replaceChildren(...notes);
  $('b_start').disabled = false;
  $('b_copy').hidden = false;
}

// (R5/T3) The chrome's colours, font and radius come from the core's look as CSS custom
// properties (empty when unchanged); they are set as given.
function applyCss() {
  const css = core.css();
  if (!css) return;
  const root = document.documentElement.style;
  for (const [k, v] of Object.entries(JSON.parse(css))) root.setProperty(k, v);
}

// (R5/M21) The motion overlay of this display frame: the core names each box (by face and
// key) and gives its CSS values; the page creates one element per key inside its face and
// applies the values as they are; a key the core no longer names is removed.
function applyMarks(m) {
  const keep = new Set();
  for (const b of m.marks) {
    const face = faces.get(b.face);
    if (!face) continue;
    let el = marks.get(b.key);
    if (!el || el.parentNode !== face) {
      if (el) el.remove();
      el = document.createElement('div');
      el.className = 'mk';
      face.append(el);
      marks.set(b.key, el);
    }
    Object.assign(el.style, b.style);
    keep.add(b.key);
  }
  for (const [k, el] of marks) {
    if (!keep.has(k)) {
      el.remove();
      marks.delete(k);
    }
  }
}

function motionPass(ts) {
  const text = core.motion_marks(ts);
  if (text) applyMarks(JSON.parse(text));
}

function frame(ts) {
  if (core) {
    applyCss();
    core.metric_frame(mode, ts);
    if (benching) benchStep(ts);
    paintPass();
    motionPass(ts);
  }
  requestAnimationFrame(frame);
}

// --- page API for the test harnesses (screens.mjs, tests/*.mjs) -------------------

window.qp = {
  // (RP3/M14) Not ready while a paint pass is pending.
  ready: () => Boolean(core) && core.idle() && !core.has_changes() && !repaintWanted && posts.size === 0,
  digests: () => core.digests(),
  metrics: () => { sampleMemory(); return core.metrics(); },
  resetMetrics: () => core.metric_reset(),
  status: () => core.status(),
  open: () => core.open(),
  feed: () => core.feed(),
  sent: () => core.sent(),
  png: (id) => {
    const c = document.getElementById(id) || (faces.get(id) && faces.get(id).firstChild);
    // (RP3/M12c) A canvas a paint path drew with another context (WebGPU) is read
    // through a 2D copy (`snapshot`); a 2D canvas is read directly, as before.
    const g = c.getContext('2d') || snapshot(c).getContext('2d');
    const img = g.getImageData(0, 0, c.width, c.height);
    return Array.from(core.png(new Uint8Array(img.data.buffer), c.width, c.height));
  },
  repaint: () => { if (core) { renderFeed(); renderLayer(true); renderStatus(); } },
  // (R4/H-M8910) for the measurement harness (rounds/R4/experiments/harness/m8910):
  look: () => core.look(),
  seen: () => { core.mark_seen(); flush(); },
  // (R4/S1) for tests/settings.mjs
  audience: () => core.audience(),
  choices: () => core.choices(),
  // (R5/T3) for tests/settings.mjs: the face frame the core states (look-dependent).
  faceSize: () => core.face_size(),
  // (R5/m17-int) for tests/settings.mjs: a face's scene as the core holds it (its terms).
  faceDesc: (id) => JSON.parse(core.face_desc(id) || 'null'),
  // (R5/M20) for tests/settings.mjs: the open capsule's scene as the core holds it.
  openDesc: () => JSON.parse(core.open_desc() || 'null'),
  openCapsule: (id) => { core.open_capsule(id); flush(); },
  // (R5B) The agent's least trace as the page shows it, and the reader's decision.
  comments: () => JSON.parse(core.comments()),
  proposals: () => JSON.parse(core.proposals()),
  decide: (id, accept) => { core.decide(id, accept); flush(); },
  openTo: (to) => { core.open_to(to); flush(); },
  // (RP3/M15) for the viewer measurement (rounds/RP3/experiments/M15): the device's data
  // subscriptions and traffic, and the open capsule's check run again on its own data.
  viewer: () => core.viewer(),
  check: () => core.check(),
  // (R5/M18) for the story chart measurement (rounds/R5/experiments/M18/pavka).
  stories: () => core.stories(),
  // (R5/M18 korda) `n` synthetic stories in place of the held list (K52 with 5 / 50 / 200).
  benchStories: (n) => { const k = core.bench_stories(n); renderLayer(true); return k; },
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
  // (R5/M21) A face's last change frozen at progress `p` (the change animation's frames
  // are the core's, from the scene descriptions; returns the pieces drawn), its release,
  // the latest changes as facts (both modes alike) and the faces' ids that changed.
  freezeChange: (id, p) => {
    window.qp.unfreezeChange();
    const m = changes.get(id);
    if (!m) return 0;
    // The face where it is now (the feed may have scrolled since).
    const box = m.canvas.getBoundingClientRect();
    m.left = box.left;
    m.top = box.top;
    const els = changeCanvases(m);
    showChange(m, els, p);
    window.qp.frozenChange = { m, els };
    return els.length;
  },
  unfreezeChange: () => {
    const f = window.qp.frozenChange;
    if (f) clearChange(f.m, f.els);
    window.qp.frozenChange = null;
  },
  frozenChange: null,
  faceChanges: () => JSON.parse(core.face_changes()),
  changed: () => Array.from(changes.keys()),
  // (R5/M21) The motion overlay the core would hand out at page time `t` (JSON), for the
  // harness's frozen frames of pulse, beat and trail.
  marks: (t) => core.motion_marks(t),
  // (RP3/T2) for tests/compare.mjs
  benching: () => benching,
  bench: () => core.bench_summary(),
  paintUsed: () => pick.used,
};

async function main() {
  document.body.dataset.theme = ui.theme;
  if (reducedMotion()) document.body.classList.add('noanim');
  labels();
  wire();
  // (R4/T6) Connect first: the host prepares the start frame while the WASM downloads
  // and compiles (they used to run one after the other).
  connect();
  await init();
  wasmReady = performance.now();
  while (early.length) onFrame(early.shift());
  requestAnimationFrame(frame);
  // (RP3/M14) The JIT probe, once, a few seconds after the start (not during the
  // opening): the page only gives the core the raw timestamps around one call.
  setTimeout(() => {
    if (!core) return;
    const t0 = performance.now();
    core.jit_probe();
    core.metric_span('jit_probe', t0, performance.now());
  }, 4000);
  setInterval(() => {
    sampleMemory();
    if (document.body.dataset.tab === 'metrics') renderMetrics();
  }, 1000);
}

main();
