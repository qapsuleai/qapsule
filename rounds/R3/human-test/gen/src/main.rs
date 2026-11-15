//! R3/HT — generator of the human-test package (GOAL criterion 15, K39).
//!
//! Writes, deterministically (fixed seed, no clock, no network), into the package folder
//! (`rounds/R3/human-test/`, or the directory given as the first argument):
//!
//! - `pages/alfabe-{M,N}-okuma.png`: the reading page, 16 glyphs A1…D4 (A–B: no legend,
//!   C–D: with legend), each at its true size (24 px) and at 2× (48 px);
//! - `pages/alfabe-{M,N}-ogrenme.png`: 5 labelled examples E1…E5 and 4 new glyphs L1…L4;
//! - `kitapcik-A.html`, `kitapcik-B.html`: the stimulus booklets (A: M first, B: N first);
//! - `form.html`: the printable answer form (order-independent: "Bölüm 1", "Bölüm 2");
//! - `key.json`: the answer key (glyph truths, sentence truths, booklet orders).
//!
//! Alphabets are drawn through the M1 harness (`Alphabet::draw` → `qp_render`, the
//! golden-image CPU path); sentences come from the product realizer
//! `qp_capsule::realize::Template` on the M2 harness situations. Participant-facing codes:
//! M = `siluet`, N = `rozet` (neutral letters; the real names stay in `key.json`).
//!
//! PNGs are drawn at 2 device pixels per CSS pixel: the "24 px" glyph is rasterized at
//! 24 px and doubled nearest-neighbour (its pixels are kept), the "2×" glyph is rasterized
//! at 48 px and doubled. The HTML shows every PNG at half its pixel width, so on screen and
//! on paper (96 CSS px per inch) the true-size glyph is exactly 24 CSS px.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use harness::m1::glyphset::{base, dir_name, phase_name, spec_id, strength_class};
use harness::m1::registry::{by_name, M1Entry};
use harness::m1::render::{blit, glyph_rgba, upscale};
use harness::m2::situations::{self, Situation};
use harness::rng::Rng;
use qp_capsule::realize::Template;
use qp_capsule::{Lang, Realizer};
use qp_grammar::theme::Theme;
use qp_render::scene::scene_of;
use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind};
use qp_schema::scene::{Align, Prim, Rgba};
use serde_json::{json, Map, Value};

/// The package seed. Changing it changes every page and the key.
const SEED: u64 = 1;
/// Device pixels per CSS pixel in every PNG.
const D: u32 = 2;
/// True glyph size (CSS px).
const SMALL: u32 = 24;
/// 2× glyph size (CSS px).
const BIG: u32 = 48;
/// Page width (CSS px): fits a 360 px phone without scaling.
const W: u32 = 360;
/// Alphabet name → participant-facing code.
const ALPHABETS: [(&str, &str); 2] = [("siluet", "M"), ("rozet", "N")];
/// Booklet → alphabet order (counterbalancing).
const BOOKLETS: [(&str, [&str; 2]); 2] = [("A", ["siluet", "rozet"]), ("B", ["rozet", "siluet"])];

use Direction::{Down, Neutral, Up};
use GlyphPhase::{Candidate, Failed, Final, Realized, Retracted};
use MeaningKind::{Breakout, Continuation, Extreme, Flow, Reversal, Squeeze};

/// Rows A–B (answered without the legend). Each half has all six kinds.
const HALF1: [(MeaningKind, Direction); 8] = [
    (Continuation, Up),
    (Reversal, Down),
    (Breakout, Up),
    (Squeeze, Neutral),
    (Extreme, Up),
    (Flow, Down),
    (Flow, Neutral),
    (Reversal, Up),
];
/// Rows C–D (answered with the legend). Together with `HALF1` every member of `K` appears.
const HALF2: [(MeaningKind, Direction); 8] = [
    (Continuation, Down),
    (Breakout, Down),
    (Extreme, Down),
    (Flow, Up),
    (Reversal, Down),
    (Squeeze, Neutral),
    (Breakout, Up),
    (Continuation, Up),
];
/// Phases of each half (shuffled): all five phases in both halves.
const PH1: [GlyphPhase; 8] = [
    Candidate, Candidate, Final, Final, Realized, Realized, Failed, Retracted,
];
const PH2: [GlyphPhase; 8] = [
    Candidate, Final, Final, Realized, Failed, Failed, Retracted, Retracted,
];
/// Strength values used for the two classes (`low` < 0.5 ≤ `high`).
const LOW: f64 = 0.25;
const HIGH: f64 = 0.9;
/// Learning examples (≥ 4 kinds, all five phases) and the 4 new test glyphs; no test
/// kind × direction × phase appears among the examples.
const EXAMPLES: [(MeaningKind, Direction, GlyphPhase, f64); 5] = [
    (Breakout, Up, Final, HIGH),
    (Reversal, Down, Candidate, LOW),
    (Squeeze, Neutral, Realized, HIGH),
    (Flow, Up, Failed, LOW),
    (Extreme, Down, Retracted, HIGH),
];
const TESTS: [(MeaningKind, Direction, GlyphPhase, f64); 4] = [
    (Continuation, Up, Realized, LOW),
    (Breakout, Down, Candidate, HIGH),
    (Extreme, Up, Final, LOW),
    (Flow, Neutral, Failed, HIGH),
];

fn kind_tr(k: MeaningKind) -> &'static str {
    match k {
        Continuation => "devam",
        Reversal => "dönüş",
        Breakout => "kırılım",
        Squeeze => "sıkışma",
        Extreme => "aşırılık",
        Flow => "akış",
    }
}
fn dir_tr(d: Direction) -> &'static str {
    match d {
        Up => "yukarı",
        Down => "aşağı",
        Neutral => "yönsüz",
    }
}
fn phase_tr(p: GlyphPhase) -> &'static str {
    match p {
        Candidate => "aday",
        Final => "kesin",
        Realized => "gerçekleşti",
        Failed => "gerçekleşmedi",
        Retracted => "geri çekildi",
    }
}
fn strength_tr(s: f64) -> &'static str {
    if s < 0.5 {
        "zayıf"
    } else {
        "güçlü"
    }
}

/// A labelled glyph with the reading condition it is answered in.
#[derive(Clone)]
struct Item {
    label: String,
    spec: GlyphSpec,
    condition: &'static str,
}

fn with_rest(r: &mut Rng, k: MeaningKind, d: Direction, p: GlyphPhase, s: f64) -> GlyphSpec {
    let confirmations = 1 + u8::try_from(r.below(3)).unwrap_or(0);
    let confidence = Some([0.3, 0.8][r.below(2)]);
    GlyphSpec {
        phase: p,
        strength: s,
        confirmations,
        confidence,
        ..base(k, d)
    }
}

fn half(
    r: &mut Rng,
    kd: &[(MeaningKind, Direction); 8],
    phases: [GlyphPhase; 8],
) -> Vec<GlyphSpec> {
    let mut ph = phases.to_vec();
    r.shuffle(&mut ph);
    let mut st = vec![LOW, LOW, LOW, LOW, HIGH, HIGH, HIGH, HIGH];
    r.shuffle(&mut st);
    let mut out = Vec::new();
    for i in 0..8 {
        let (k, d) = kd[i];
        out.push(with_rest(r, k, d, ph[i], st[i]));
    }
    out
}

/// The glyph specs shared by both alphabets (same multiset → paired comparison).
struct Specs {
    h1: Vec<GlyphSpec>,
    h2: Vec<GlyphSpec>,
    examples: Vec<GlyphSpec>,
    tests: Vec<GlyphSpec>,
}

fn specs() -> Specs {
    let mut r = Rng::derive(SEED, "ht.specs");
    let h1 = half(&mut r, &HALF1, PH1);
    let h2 = half(&mut r, &HALF2, PH2);
    let examples = EXAMPLES
        .iter()
        .map(|&(k, d, p, s)| with_rest(&mut r, k, d, p, s))
        .collect();
    let tests = TESTS
        .iter()
        .map(|&(k, d, p, s)| with_rest(&mut r, k, d, p, s))
        .collect();
    Specs {
        h1,
        h2,
        examples,
        tests,
    }
}

/// Per-alphabet item order (seeded by the alphabet name): reading A1…D4, then L1…L4.
fn items_for(alt: &str, sp: &Specs) -> Vec<Item> {
    let mut r = Rng::derive(SEED, &format!("ht.order.{alt}"));
    let mut h1 = sp.h1.clone();
    let mut h2 = sp.h2.clone();
    let mut t = sp.tests.clone();
    r.shuffle(&mut h1);
    r.shuffle(&mut h2);
    r.shuffle(&mut t);
    let mut out = Vec::new();
    for (i, s) in h1.into_iter().chain(h2).enumerate() {
        let row = char::from(b'A' + u8::try_from(i / 4).unwrap_or(0));
        out.push(Item {
            label: format!("{row}{}", i % 4 + 1),
            spec: s,
            condition: if i < 8 { "nolegend" } else { "legend" },
        });
    }
    for (i, s) in t.into_iter().enumerate() {
        out.push(Item {
            label: format!("L{}", i + 1),
            spec: s,
            condition: "learn",
        });
    }
    out
}

// ------------------------------------------------------------------------------------
// Drawing

struct Canvas<'a> {
    entry: &'a M1Entry,
    theme: Theme,
    h: u32,
    prims: Vec<Prim>,
    tiles: Vec<(Vec<u8>, u32, u32, u32)>,
}

impl<'a> Canvas<'a> {
    fn new(entry: &'a M1Entry, h: u32) -> Self {
        Self {
            entry,
            theme: Theme::light(),
            h,
            prims: Vec::new(),
            tiles: Vec::new(),
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn text(&mut self, x: u32, baseline: u32, px: u32, s: &str, muted: bool) {
        let color = if muted {
            self.theme.muted
        } else {
            self.theme.text
        };
        self.prims.push(Prim::Text {
            x: (x * D).cast_signed(),
            baseline: (baseline * D).cast_signed(),
            px: (px * D) as f32,
            text: s.to_owned(),
            color,
            align: Align::Left,
        });
    }

    fn rule(&mut self, y: u32, color: Rgba) {
        self.prims.push(Prim::Rect {
            x: (8 * D).cast_signed(),
            y: (y * D).cast_signed(),
            w: (W - 16) * D,
            h: 1,
            color,
        });
    }

    /// Glyph rasterized at `size` CSS px and doubled to device pixels, top-left `(x, y)`.
    fn glyph(&mut self, spec: &GlyphSpec, size: u32, x: u32, y: u32) -> Result<(), String> {
        let px = glyph_rgba(self.entry.alphabet.as_ref(), spec, size, &self.theme)?;
        self.tiles
            .push((upscale(&px, size, size, D), size * D, x * D, y * D));
        Ok(())
    }

    fn png(self) -> Result<Vec<u8>, String> {
        let (w, h) = (W * D, self.h * D);
        let mut canvas = qp_render::render_scene_rgba(&scene_of(w, h, self.theme.bg, self.prims))
            .map_err(|e| e.to_string())?;
        for (t, s, x, y) in &self.tiles {
            blit(&mut canvas, w, t, *s, *s, *x, *y);
        }
        qp_render::encode_png(&canvas, w, h).map_err(|e| e.to_string())
    }
}

const COL: u32 = 86;
const LEFT: u32 = 8;
const CELL_H: u32 = SMALL + 6 + BIG + 14;

/// One grid cell: label and true-size glyph on the first line, the 2× glyph under them
/// (cells stay 38 px apart, so a 2× glyph is never read as part of the next cell).
fn cell(c: &mut Canvas<'_>, it: &Item, x: u32, y: u32) -> Result<(), String> {
    c.text(x, y + 17, 13, &it.label, false);
    c.glyph(&it.spec, SMALL, x + 24, y)?;
    c.glyph(&it.spec, BIG, x, y + SMALL + 6)?;
    Ok(())
}

fn grid(c: &mut Canvas<'_>, items: &[Item], y0: u32) -> Result<u32, String> {
    let mut y = y0;
    for row in items.chunks(4) {
        for (i, it) in row.iter().enumerate() {
            cell(c, it, LEFT + 6 + u32::try_from(i).unwrap_or(0) * COL, y)?;
        }
        y += CELL_H;
    }
    Ok(y)
}

fn reading_png(entry: &M1Entry, code: &str, items: &[Item]) -> Result<Vec<u8>, String> {
    let rule = Theme::light().muted;
    let h = 52 + 2 * (24 + 2 * CELL_H) + 8;
    let mut c = Canvas::new(entry, h);
    c.text(
        LEFT,
        18,
        14,
        &format!("Alfabe {code} - okuma sayfası"),
        false,
    );
    c.text(LEFT, 36, 10, "Etiketin yanında gerçek boyut (24 px),", true);
    c.text(LEFT, 48, 10, "altında aynı işaretin 2 kat büyüğü.", true);
    let mut y = 52;
    for (half, caption) in [
        (&items[0..8], "A1-B4: 1. adım (açıklamasız)"),
        (&items[8..16], "C1-D4: 2. adım (açıklamayı okuduktan sonra)"),
    ] {
        c.rule(y + 4, rule);
        c.text(LEFT, y + 19, 10, caption, true);
        y = grid(&mut c, half, y + 24)?;
    }
    c.png()
}

fn learning_png(
    entry: &M1Entry,
    code: &str,
    examples: &[GlyphSpec],
    tests: &[Item],
) -> Result<Vec<u8>, String> {
    let rule = Theme::light().muted;
    let ex_h = BIG + 14;
    let h = 52 + 24 + 5 * ex_h + 24 + CELL_H + 8;
    let mut c = Canvas::new(entry, h);
    c.text(
        LEFT,
        18,
        14,
        &format!("Alfabe {code} - öğrenme sayfası"),
        false,
    );
    c.text(
        LEFT,
        36,
        10,
        "E1-E5: örnek; doğru cevap yanında yazılı.",
        true,
    );
    c.text(
        LEFT,
        48,
        10,
        "L1-L4: yeni işaretler; cevabı forma yazın.",
        true,
    );
    let mut y = 52;
    c.rule(y + 4, rule);
    c.text(LEFT, y + 19, 10, "Örnekler", true);
    y += 24;
    for (i, s) in examples.iter().enumerate() {
        let x = LEFT + 4;
        c.text(x, y + 30, 13, &format!("E{}", i + 1), false);
        c.glyph(s, SMALL, x + 26, y + (BIG - SMALL) / 2)?;
        c.glyph(s, BIG, x + 26 + SMALL + 8, y)?;
        let tx = x + 26 + SMALL + 8 + BIG + 10;
        c.text(tx, y + 12, 11, &format!("tür: {}", kind_tr(s.kind)), false);
        c.text(
            tx,
            y + 28,
            11,
            &format!(
                "yön: {} · güç: {}",
                dir_tr(s.direction),
                strength_tr(s.strength)
            ),
            false,
        );
        c.text(
            tx,
            y + 44,
            11,
            &format!("aşama: {}", phase_tr(s.phase)),
            false,
        );
        y += ex_h;
    }
    c.rule(y + 4, rule);
    c.text(LEFT, y + 19, 10, "L1-L4: 3. adım (yeni işaretler)", true);
    grid(&mut c, tests, y + 24)?;
    c.png()
}

// ------------------------------------------------------------------------------------
// Sentences

/// One sentence of the sentence page.
struct Sent {
    label: String,
    sit: Situation,
    text: String,
}

/// Seven seeded situations covering event / state / expectation (each with a still-valid
/// and a no-longer-valid or unknown case, licensed and unlicensed readers) plus K26.
fn pick_sentences() -> Vec<Sent> {
    type Crit = fn(&Situation) -> bool;
    let crit: [Crit; 7] = [
        |s| s.factors.time == "event" && !s.factors.redacted && s.factors.bars_ago > 0,
        |s| s.factors.time == "event" && s.factors.redacted,
        |s| s.factors.time == "event" && s.factors.bars_ago == 0 && !s.factors.redacted,
        |s| s.factors.time == "state" && s.truth.still_valid == "yes" && !s.factors.redacted,
        |s| s.factors.time == "state" && s.truth.still_valid == "no" && s.factors.redacted,
        |s| s.factors.time == "expectation" && s.truth.still_valid == "yes" && !s.factors.redacted,
        |s| s.factors.time == "expectation" && s.truth.still_valid == "no",
    ];
    let all = situations::all();
    let mut pool: Vec<Situation> = all.iter().filter(|s| s.id != "k26").cloned().collect();
    let mut r = Rng::derive(SEED, "ht.sentences");
    r.shuffle(&mut pool);
    let mut chosen: Vec<Situation> = Vec::new();
    for c in crit {
        let used = |s: &Situation, ch: &[Situation]| ch.iter().any(|x| x.id == s.id);
        let kind_used =
            |s: &Situation, ch: &[Situation]| ch.iter().any(|x| x.truth.kind == s.truth.kind);
        let pick = pool
            .iter()
            .find(|s| c(s) && !used(s, &chosen) && !kind_used(s, &chosen))
            .or_else(|| pool.iter().find(|s| c(s) && !used(s, &chosen)))
            .cloned();
        if let Some(s) = pick {
            chosen.push(s);
        }
    }
    chosen.push(situations::k26());
    r.shuffle(&mut chosen);
    // K26 is never the first sentence (no warm-up advantage either way).
    if chosen[0].id == "k26" {
        chosen.swap(0, 1);
    }
    chosen
        .into_iter()
        .enumerate()
        .map(|(i, s)| Sent {
            label: format!("S{}", i + 1),
            text: Template.sentence(&s.input(), Lang::Tr),
            sit: s,
        })
        .collect()
}

// ------------------------------------------------------------------------------------
// HTML

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const CSS: &str = r#"
:root { --ink:#111; --muted:#555; --rule:#999; --soft:#f2f2f2; --bg:#fff; }
* { box-sizing: border-box; }
html, body { background: var(--bg); color: var(--ink); margin: 0; }
body { font-family: -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif; font-size: 15px; line-height: 1.45; }
.page { max-width: 720px; margin: 0 auto; padding: 20px 16px 40px; min-height: 100vh; border-bottom: 2px dashed var(--rule); break-after: page; page-break-after: always; }
.page:last-child { border-bottom: 0; }
h1 { font-size: 20px; margin: 0 0 8px; }
h2 { font-size: 17px; margin: 14px 0 6px; }
.tag { font-size: 12px; color: var(--muted); letter-spacing: .04em; text-transform: uppercase; }
.do { background: var(--soft); border-left: 4px solid var(--ink); padding: 8px 10px; margin: 10px 0; }
.legend { border: 1px solid var(--rule); padding: 10px 12px; margin: 10px 0; font-size: 15px; }
.fig { overflow-x: auto; margin: 10px 0; }
.fig img { display: block; max-width: none; image-rendering: auto; }
table { border-collapse: collapse; width: 100%; }
td, th { border: 1px solid var(--rule); padding: 3px 5px; vertical-align: top; text-align: left; }
th { background: var(--soft); font-weight: 600; }
.voc td:first-child { white-space: nowrap; font-weight: 600; }
.sent { font-size: 17px; margin: 0 0 14px; }
.sent b { display: inline-block; min-width: 2.2em; }
.small { font-size: 12px; color: var(--muted); }
@media print {
  @page { size: A4; margin: 12mm; }
  .page { min-height: 0; border-bottom: 0; padding: 0; max-width: none; }
  body { font-size: 12.5px; }
}
"#;

fn doc(title: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"tr\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{}</title>\n<style>{CSS}{FORM_CSS}</style>\n</head>\n<body>\n{body}</body>\n</html>\n",
        esc(title)
    )
}

fn img(path: &str, png: &[u8], alt: &str) -> String {
    // PNG IHDR: width and height at bytes 16..24 (big-endian).
    let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]) / D;
    let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]) / D;
    format!(
        "<div class=\"fig\"><img src=\"{path}\" width=\"{w}\" height=\"{h}\" alt=\"{}\"></div>\n",
        esc(alt)
    )
}

/// The vocabulary card (meanings only, never drawing rules; CAPSULE §4 and §3).
const VOCAB: &str = r#"<table class="voc">
<tr><th colspan="2">Tür (işaretin anlamı)</th></tr>
<tr><td>Devam (DV)</td><td>Hareket sürüyor.</td></tr>
<tr><td>Dönüş (DN)</td><td>Hareket tersine dönüyor olabilir.</td></tr>
<tr><td>Kırılım (KR)</td><td>Bir sınır aşıldı.</td></tr>
<tr><td>Sıkışma (SK)</td><td>Hareket daralıyor; yön belirsiz.</td></tr>
<tr><td>Aşırılık (AŞ)</td><td>Olağandışı uzama (aşırı alım / aşırı satım).</td></tr>
<tr><td>Akış (AK)</td><td>Fiyattan bağımsız para girişi ya da çıkışı.</td></tr>
<tr><th colspan="2">Yön</th></tr>
<tr><td>↑ / ↓ / —</td><td>Yukarı, aşağı ya da yönsüz (belirsiz).</td></tr>
<tr><th colspan="2">Güç</th></tr>
<tr><td>Z / G</td><td>Zayıf ya da güçlü.</td></tr>
<tr><th colspan="2">Aşama (gözlemin yaşamı)</th></tr>
<tr><td>Aday (AD)</td><td>Bar daha kapanmadan görüldü; kaybolabilir.</td></tr>
<tr><td>Kesin (KE)</td><td>Bar kapandı, gözlem kesinleşti; hâlâ açık.</td></tr>
<tr><td>Gerçekleşti (GE)</td><td>Kapandı: beklenen oldu.</td></tr>
<tr><td>Gerçekleşmedi (GM)</td><td>Kapandı: beklenen olmadı.</td></tr>
<tr><td>Geri çekildi (GÇ)</td><td>Kapandı: dayandığı veri hatalı çıktı, geri alındı.</td></tr>
</table>
"#;

struct Pages {
    reading: Vec<u8>,
    learning: Vec<u8>,
}

fn booklet(
    name: &str,
    order: [&str; 2],
    pages: &[(&str, &str, &str, Pages)],
    sents: &[Sent],
) -> String {
    let mut b = String::new();
    let _ = write!(
        b,
        "<section class=\"page\"><div class=\"tag\">Kitapçık {name}</div>\n<h1>İşaret okuma denemesi</h1>\n\
<p>Bu kitapçıkta bir piyasa \"anlam alfabesi\"nin işaretleri var. Her küçük işaret, bir hisse grafiği hakkında bir gözlemi anlatır. Bu bir sınav değil; ölçülen sizin değil, işaretlerin anlaşılırlığı. Emin olmadığınızda tahmin edin.</p>\n\
<p>Sayfaları görevli söyleyince çevirin. Önceki bir adıma dönüp cevap değiştirmeyin.</p>\n\
<h2>Sözlük</h2>\n<p class=\"small\">Bu tablo yalnız sözcüklerin anlamını verir; işaretlerin nasıl çizildiğini söylemez. Kodlar formda daire içine alacağınız kısaltmalardır.</p>\n{VOCAB}</section>\n"
    );
    for (sec, alt) in order.iter().enumerate() {
        let (_, code, legend, p) = pages.iter().find(|x| x.0 == *alt).expect("alphabet pages");
        let s = sec + 1;
        let read = img(
            &format!("pages/alfabe-{code}-okuma.png"),
            &p.reading,
            &format!("Alfabe {code} okuma sayfası"),
        );
        let learn = img(
            &format!("pages/alfabe-{code}-ogrenme.png"),
            &p.learning,
            &format!("Alfabe {code} öğrenme sayfası"),
        );
        let _ = write!(
            b,
            "<section class=\"page\"><div class=\"tag\">Kitapçık {name} · Bölüm {s} · Alfabe {code}</div>\n<h1>Bölüm {s} — 1. adım: açıklamasız okuma</h1>\n\
<div class=\"do\">Formda <b>Bölüm {s} → 1. adım</b> satırlarını (A1–B4) doldurun. Her işaret için tür, yön, güç, aşama ve ne kadar emin olduğunuz. C ve D satırlarını şimdilik boş bırakın.</div>\n\
<p class=\"small\">Önce etiketin yanındaki küçük (gerçek boyutlu) işarete bakın; altındaki aynı işaretin büyütülmüşüdür.</p>\n{read}</section>\n\
<section class=\"page\"><div class=\"tag\">Kitapçık {name} · Bölüm {s} · Alfabe {code}</div>\n<h1>Bölüm {s} — 2. adım: açıklama ile okuma</h1>\n\
<p>Alfabe {code}'nin kendi kısa açıklaması:</p>\n<div class=\"legend\">{}</div>\n\
<div class=\"do\">Açıklamayı okuyun, sonra formda <b>Bölüm {s} → 2. adım</b> satırlarını (C1–D4) doldurun. A–B cevaplarınızı değiştirmeyin.</div>\n{read}</section>\n\
<section class=\"page\"><div class=\"tag\">Kitapçık {name} · Bölüm {s} · Alfabe {code}</div>\n<h1>Bölüm {s} — 3. adım: örneklerle öğrenme</h1>\n\
<div class=\"do\">E1–E5 örneklerini cevaplarıyla inceleyin (yaklaşık yarım dakika). Sonra formda <b>Bölüm {s} → 3. adım</b> satırlarını (L1–L4) doldurun.</div>\n{learn}</section>\n",
            esc(legend)
        );
    }
    let _ = write!(
        b,
        "<section class=\"page\"><div class=\"tag\">Kitapçık {name} · Bölüm 3</div>\n<h1>Bölüm 3 — Kapsül cümleleri</h1>\n\
<div class=\"do\">Her cümle için formda: ne oldu (tür ve yön), ne zaman (geçmişte / şu an / bir koşula bağlı) ve hâlâ geçerli mi (evet / hayır / bilinmiyor). Cümlede yazmayanı bilinmiyor sayın.</div>\n"
    );
    for s in sents {
        let _ = writeln!(
            b,
            "<p class=\"sent\"><b>{}</b> {}</p>",
            s.label,
            esc(&s.text)
        );
    }
    b.push_str("<p class=\"small\">\"Bar\": grafikteki bir mum, yani bir zaman dilimi (ör. bir gün).</p></section>\n");
    let _ = write!(
        b,
        "<section class=\"page\"><div class=\"tag\">Kitapçık {name} · Bölüm 4</div>\n<h1>Bölüm 4 — Tercih</h1>\n\
<div class=\"do\">Formdaki son üç soruyu cevaplayın. Hatırlamak için iki okuma sayfası aşağıda.</div>\n"
    );
    for alt in order {
        let (_, code, _, p) = pages.iter().find(|x| x.0 == alt).expect("alphabet pages");
        b.push_str(&img(
            &format!("pages/alfabe-{code}-okuma.png"),
            &p.reading,
            &format!("Alfabe {code} okuma sayfası"),
        ));
    }
    b.push_str("</section>\n");
    doc(&format!("Kitapçık {name}"), &b)
}

const FORM_CSS: &str = r#"
.f td { padding: 2px 4px; font-size: 13px; }
.f td.l { font-weight: 700; white-space: nowrap; width: 3.2em; }
.o { display: inline-block; min-width: 1.9em; padding: 0 2px; margin: 1px 1px; border: 1px solid #bbb; border-radius: 9px; text-align: center; font-size: 12px; }
.hdr td { padding: 6px 8px; }
.line { border-bottom: 1px solid #555; height: 1.6em; margin: 4px 0; }
"#;

fn opts(v: &[&str]) -> String {
    v.iter()
        .map(|o| format!("<span class=\"o\">{o}</span>"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn glyph_rows(b: &mut String, labels: &[String]) {
    b.push_str("<table class=\"f\"><tr><th></th><th>Tür</th><th>Yön</th><th>Güç</th><th>Aşama</th><th>Ne kadar eminim</th></tr>\n");
    for l in labels {
        let _ = writeln!(
            b,
            "<tr><td class=\"l\">{l}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            opts(&["DV", "DN", "KR", "SK", "AŞ", "AK"]),
            opts(&["↑", "↓", "—"]),
            opts(&["Z", "G"]),
            opts(&["AD", "KE", "GE", "GM", "GÇ"]),
            opts(&["1", "2", "3", "4", "5"])
        );
    }
    b.push_str("</table>\n");
}

fn form(sents: &[Sent]) -> String {
    let mut b = String::new();
    b.push_str("<section class=\"page\"><div class=\"tag\">Cevap formu · sayfa 1</div>\n<h1>İşaret okuma denemesi — cevap formu</h1>\n\
<table class=\"hdr\"><tr><td>Katılımcı no: ________</td><td>Kitapçık: <span class=\"o\">A</span> <span class=\"o\">B</span></td><td>Gösterim: <span class=\"o\">kâğıt</span> <span class=\"o\">ekran</span></td></tr>\n\
<tr><td colspan=\"3\">Grup: <span class=\"o\">alışkın kullanıcı (grafiğe düzenli bakar / işlem yapar)</span> <span class=\"o\">genç, işlem yapmayan</span></td></tr></table>\n\
<p class=\"small\">Ad, iletişim ya da başka bir kişisel bilgi yazmayın. Her satırda her sütunda bir seçeneği daire içine alın. Emin değilseniz tahmin edin ve \"ne kadar eminim\"e düşük puan verin (1 = tamamen tahmin, 5 = çok eminim). Kodların anlamı kitapçığın ilk sayfasındaki sözlükte.</p>\n");
    let labels = |range: std::ops::Range<usize>| -> Vec<String> {
        range
            .map(|i| {
                format!(
                    "{}{}",
                    char::from(b'A' + u8::try_from(i / 4).unwrap_or(0)),
                    i % 4 + 1
                )
            })
            .collect()
    };
    for s in 1..=2 {
        if s == 2 {
            b.push_str("</section>\n<section class=\"page\"><div class=\"tag\">Cevap formu · sayfa 2</div>\n");
        }
        let _ = writeln!(
            b,
            "<p>Bölüm {s} hangi alfabe? (kitapçıkta yazanı daire içine alın) <span class=\"o\">M</span> <span class=\"o\">N</span></p>"
        );
        let _ = writeln!(b, "<h2>Bölüm {s} — 1. adım: açıklamasız (A1–B4)</h2>");
        glyph_rows(&mut b, &labels(0..8));
        let _ = writeln!(b, "<h2>Bölüm {s} — 2. adım: açıklama ile (C1–D4)</h2>");
        glyph_rows(&mut b, &labels(8..16));
        let _ = writeln!(b, "<h2>Bölüm {s} — 3. adım: örneklerden sonra (L1–L4)</h2>");
        glyph_rows(
            &mut b,
            &(1..=4).map(|i| format!("L{i}")).collect::<Vec<_>>(),
        );
    }
    b.push_str("</section>\n<section class=\"page\"><div class=\"tag\">Cevap formu · sayfa 3</div>\n<h2>Bölüm 3 — Kapsül cümleleri</h2>\n\
<table class=\"f\"><tr><th></th><th>Ne oldu: tür</th><th>yön</th><th>Ne zaman</th><th>Hâlâ geçerli mi?</th></tr>\n");
    for s in sents {
        let _ = writeln!(
            b,
            "<tr><td class=\"l\">{}</td><td>{}</td><td>{}</td><td>{} ___ bar önce<br>{}<br>{}</td><td>{}</td></tr>",
            s.label,
            opts(&["DV", "DN", "KR", "SK", "AŞ", "AK"]),
            opts(&["↑", "↓", "—"]),
            opts(&["geçmişte"]),
            opts(&["şu an sürüyor"]),
            opts(&["bir koşula bağlı"]),
            opts(&["evet", "hayır", "bilinmiyor"])
        );
    }
    b.push_str("</table>\n<p class=\"small\">\"Geçmişte\" seçtiyseniz cümle söylüyorsa kaç bar önce olduğunu yazın; söylemiyorsa boş bırakın.</p>\n\
<h2>Bölüm 4 — Tercih</h2>\n\
<p><b>T1.</b> Hangi alfabe daha kolay okundu? <span class=\"o\">M</span> <span class=\"o\">N</span> <span class=\"o\">fark yok</span></p>\n\
<p><b>T2.</b> Hangi işaretler karıştı, neyle karıştı? (ör. \"M-B3 dönüş mü devam mı anlamadım\")</p><div class=\"line\"></div><div class=\"line\"></div>\n\
<p><b>T3.</b> Eklemek istediğiniz başka bir şey (serbest)</p><div class=\"line\"></div><div class=\"line\"></div>\n</section>\n");
    doc("Cevap formu", &b)
}

// ------------------------------------------------------------------------------------
// Key

fn glyph_truth(it: &Item) -> Value {
    json!({
        "condition": it.condition,
        "kind": it.spec.kind.name(),
        "direction": dir_name(it.spec.direction),
        "strength": strength_class(it.spec.strength),
        "phase": phase_name(it.spec.phase),
        "spec": spec_id(&it.spec),
    })
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn run(out: &Path) -> Result<(), String> {
    let sp = specs();
    let sents = pick_sentences();
    let mut pages: Vec<(&str, &str, &str, Pages)> = Vec::new();
    let mut key_alph = Map::new();
    for (alt, code) in ALPHABETS {
        let entry = by_name(alt)?;
        let items = items_for(alt, &sp);
        let reading = reading_png(&entry, code, &items[0..16])?;
        let learning = learning_png(&entry, code, &sp.examples, &items[16..20])?;
        write(
            &out.join(format!("pages/alfabe-{code}-okuma.png")),
            &reading,
        )?;
        write(
            &out.join(format!("pages/alfabe-{code}-ogrenme.png")),
            &learning,
        )?;
        let mut it = Map::new();
        for i in &items {
            it.insert(i.label.clone(), glyph_truth(i));
        }
        key_alph.insert(
            alt.to_owned(),
            json!({ "code": code, "legend": entry.legend, "items": it }),
        );
        pages.push((alt, code, entry.legend, Pages { reading, learning }));
    }
    let examples: Vec<Value> = sp
        .examples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            glyph_truth(&Item {
                label: format!("E{}", i + 1),
                spec: s.clone(),
                condition: "example",
            })
        })
        .collect();
    let mut key_sent = Map::new();
    for s in &sents {
        key_sent.insert(
            s.label.clone(),
            json!({
                "situation": s.sit.id,
                "text": s.text,
                "time": s.sit.factors.time,
                "stage": s.sit.factors.stage,
                "redacted": s.sit.factors.redacted,
                "kind": s.sit.truth.kind,
                "direction": s.sit.truth.direction,
                "when": s.sit.truth.when,
                "bars_ago": s.sit.truth.bars_ago,
                "still_valid": s.sit.truth.still_valid,
            }),
        );
    }
    let mut booklets = Map::new();
    for (name, order) in BOOKLETS {
        booklets.insert(name.to_owned(), json!(order));
        let html = booklet(name, order, &pages, &sents);
        write(&out.join(format!("kitapcik-{name}.html")), html.as_bytes())?;
    }
    let key = json!({
        "package": "r3-human-test/1",
        "seed": SEED,
        "note": "Answer key. Participants never see this file. M = siluet, N = rozet.",
        "booklets": booklets,
        "alphabets": key_alph,
        "examples": examples,
        "sentences": key_sent,
        "strength_values": {"low": LOW, "high": HIGH},
    });
    let mut kb = serde_json::to_vec_pretty(&key).map_err(|e| e.to_string())?;
    kb.push(b'\n');
    write(&out.join("key.json"), &kb)?;
    write(&out.join("form.html"), form(&sents).as_bytes())?;
    for s in &sents {
        println!(
            "{} [{} {} {}] {}",
            s.label, s.sit.id, s.sit.factors.time, s.sit.truth.still_valid, s.text
        );
    }
    Ok(())
}

fn main() {
    let out: PathBuf = std::env::args_os().nth(1).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join(".."),
        PathBuf::from,
    );
    if let Err(e) = run(&out) {
        eprintln!("ht-gen: {e}");
        std::process::exit(1);
    }
}
