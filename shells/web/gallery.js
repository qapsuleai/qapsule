// R5/G: the comparison gallery page (rules: shells/web/ARCH.md, "Galeri"). The core
// (`Gallery`, crates/qp-wasm/src/gallery.rs) holds the layout, the picks and the notes and
// writes every text; the host (`qp demo --gallery`) paints the images. This file only puts
// what the core gives on the page and passes taps and typed notes on.
import init, { Gallery } from './pkg/qp_wasm.js';

const KEY = 'qp-gallery';
const params = new URLSearchParams(location.search);
const $ = (id) => document.getElementById(id);
const buttons = new Map(); // "topic/cand" -> its "I'd pick this" button
let core = null;
let labels = {};

function load() {
  try { return localStorage.getItem(KEY) || ''; } catch (e) { return ''; }
}

function save() {
  try { localStorage.setItem(KEY, core.state()); } catch (e) { /* private mode: no memory */ }
}

function applyCss() {
  const root = document.documentElement.style;
  for (const [k, v] of Object.entries(JSON.parse(core.css()))) root.setProperty(k, v);
}

function registerFonts() {
  if (typeof FontFace === 'undefined') return;
  for (const f of JSON.parse(core.fonts())) {
    try {
      const face = new FontFace(f.family, core.font_bytes(f.id), { weight: f.weight });
      document.fonts.add(face);
      face.load().catch(() => {});
    } catch (e) { /* the fallback family stays */ }
  }
}

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text) e.textContent = text;
  return e;
}

function refresh() {
  for (const [k, on] of Object.entries(JSON.parse(core.marks()))) {
    const b = buttons.get(k);
    if (!b) continue;
    b.classList.toggle('on', on);
    b.setAttribute('aria-pressed', on ? 'true' : 'false');
    b.textContent = on ? labels.picked : labels.pick;
  }
  $('gsummary').value = core.summary();
  $('gshare').href = core.share();
  save();
}

function shot(s) {
  if (s.kind === 'img') {
    const img = el('img', 'shot');
    img.src = s.src;
    img.width = s.css[0];
    img.height = s.css[1];
    img.alt = '';
    return img;
  }
  return el('p', s.kind === 'head' ? 'shead' : 'stext', s.text);
}

function card(c) {
  const a = el('article', 'cand');
  a.dataset.key = c.key;
  a.dataset.content = c.content;
  a.appendChild(el('h3', 'cname', c.label));
  const shots = el('div', 'shots');
  for (const s of c.shots) shots.appendChild(shot(s));
  a.appendChild(shots);
  const row = el('div', 'choose');
  const b = el('button', 'pick');
  b.type = 'button';
  b.onclick = () => { core.pick(c.topic, c.id); refresh(); };
  buttons.set(c.key, b);
  row.appendChild(b);
  const note = el('input', 'note');
  note.type = 'text';
  note.placeholder = labels.note;
  note.setAttribute('aria-label', labels.note);
  note.value = c.note;
  note.oninput = () => { core.note(c.topic, c.id, note.value); refresh(); };
  row.appendChild(note);
  a.appendChild(row);
  return a;
}

function render() {
  const v = JSON.parse(core.view());
  labels = v.labels;
  document.documentElement.lang = v.lang;
  $('gtitle').textContent = labels.title;
  $('gintro').textContent = labels.intro;
  $('glang').textContent = labels.lang;
  $('gdark').textContent = labels.dark;
  $('glight').textContent = labels.light;
  $('gtheme').value = v.theme;
  $('gtheme').setAttribute('aria-label', labels.theme);
  $('gmoment').textContent = v.moment;
  $('gsumh').textContent = labels.summary;
  $('gcopy').textContent = labels.copy;
  $('gshare').textContent = labels.share;
  $('gcopied').textContent = '';
  const box = $('topics');
  box.textContent = '';
  buttons.clear();
  for (const t of v.topics) {
    const sec = el('section', 'topic');
    sec.dataset.topic = t.key;
    sec.appendChild(el('h2', 'tlabel', t.label));
    sec.appendChild(el('p', 'thint', t.hint));
    sec.appendChild(el('p', 'tcontent muted', t.content));
    for (const c of t.cands) sec.appendChild(card(c));
    box.appendChild(sec);
  }
  $('gstatus').textContent = v.topics.length ? '' : labels.empty;
  refresh();
}

async function open() {
  document.body.dataset.ready = '0';
  applyCss();
  render();
  $('gstatus').textContent = labels.loading;
  let r = null;
  try { r = await fetch(core.layout_url()); } catch (e) { r = null; }
  if (!r || !r.ok) { $('gstatus').textContent = labels.missing; return; }
  core.load(await r.text());
  render();
  document.body.dataset.ready = '1';
}

async function copy() {
  const text = core.summary();
  const box = $('gsummary');
  box.value = text;
  try {
    await navigator.clipboard.writeText(text);
    $('gcopied').textContent = labels.copied;
  } catch (e) {
    // No Clipboard API (plain http on the LAN is not a secure context): select the text.
    box.focus();
    box.select();
    let ok = false;
    try { ok = document.execCommand('copy'); } catch (e2) { ok = false; }
    $('gcopied').textContent = ok ? labels.copied : labels.select;
  }
}

await init();
core = new Gallery(load(), params.get('lang') || '', params.get('theme') || '', params.get('pick') || '', params.get('now') || '');
core.set_device(navigator.userAgent, innerWidth, innerHeight, devicePixelRatio);
registerFonts();
$('glang').onclick = () => { core.toggle_lang(); save(); open(); };
$('gtheme').onchange = (e) => { core.set_theme(e.target.value); save(); open(); };
$('gcopy').onclick = copy;
window.gallery = { core: () => core, summary: () => core.summary(), view: () => core.view() };
open();
