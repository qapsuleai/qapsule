//! (R5/M20) `tesir` — "iddia → dayanak → doğrulayan, yukarıdan aşağı".
//!
//! The open capsule is one scrollable column, read top to bottom, each block answering one
//! question before the next one is asked:
//!
//! 1. **İddia** (the claim): the glyph and the capsule's headline (in the reader's voice:
//!    what changed, the threshold, when, whether it still holds), coloured
//!    [`NO_LONGER_HEADLINE`] (`muted`) when it no longer holds — the same text and rule as
//!    the face `oltan`, so opening it changes nothing the reader already read. (R5/fix47)
//!    Not `invalid`: that grey is a mark colour (≥ 3:1, the meaning ladder), the headline
//!    is text and keeps 4.5:1 without counting 600 as bold (A46).
//! 2. **Dayanak** (what it rests on): the threshold by its name (the voice's glossary term
//!    the claim uses) and its value from the evidence, the evidence family and its licence,
//!    the evidence in small (the price or study line with the evidence geometry, bound by
//!    origin only, K33) and the voice's detail (why, and where things stand).
//!    **A reader without the data entitlement** gets no value and no geometry: the
//!    threshold row becomes a templated row saying **what** is locked and **which
//!    entitlement** opens it (A32): "Kilitli: flow/SYN03 yetkisi" / "Akış verisi
//!    lisanslı; değerler bu yetkiyle açılır."
//! 3. **Kim doğruladı** (who verified it): one row in its own form — "Sunucu doğruladı" or,
//!    once the device has checked it on its own data, "Bu cihazda sınandı" (K48; the same
//!    words as the open layer's trust mark) — and under it what that means here. A tap
//!    anywhere on the row opens the explanation (a glossary entry of the row, K49).
//! 4. **Karne**: the voice's track-record sentence (the expectation itself, counted).
//!
//! Every size, gap and colour comes from the look; the only numbers of this file are line
//! counts (how many lines each block holds) and the thumbnail's height in body lines.
//! The frame height follows from them ([`height`]): the same in every scene of a look, so
//! the shell core opens the capsule depth at that height and the page scrolls the layer.

use std::borrow::Cow;

use qp_schema::capsule::{EvidenceFamily, Geometry, LedgerStat, LicenseClass, Trust, Validity};
use qp_schema::grammar::{ColorSpec, Layer, Mark, Position, SizeSpec, View};
use qp_schema::protocol::{CapsuleVoice, GlossEntry, Language};
use qp_schema::Fact;

use super::{DeviceCheck, OpenDesign, TextInput};
use qp_grammar::depth::{self, layer, named_panel, panel_at, uniq};
use qp_grammar::provider::{DataProvider, SeriesKind};
use qp_look::error::GrammarError;
use qp_look::look::{Look, TypeRole, NO_LONGER_HEADLINE};

mod view;

pub use view::view;

/// The design.
pub const DESIGN: OpenDesign = OpenDesign {
    name: "tesir",
    note: "one column: the claim, what it rests on, who verified it, the track record; the evidence in small",
    view,
    view_ctx: None,
    tap: None,
    height: Some(height),
    text: Some(text_column),
    scene_text: Some(no_text),
};

// (R5/S) Line and bar counts: the look's `open` record (`look::OpenLook`).

fn no_text() -> String {
    String::new()
}

fn lh(r: &TypeRole) -> f64 {
    (r.px * r.line).round()
}

/// The six blocks' heights in `look` (CSS px), top to bottom: claim, basis, thumbnail,
/// detail, trust, track record.
struct Slots {
    sec: f64,
    tight: f64,
    month: f64,
    h: [f64; 6],
}

fn slots(look: &Look) -> Slots {
    let t = &look.types;
    let s = look.space.scale;
    let sec = f64::from(s[4]);
    let tight = f64::from(s[0]);
    let head = lh(&t.label);
    // Room under the thumbnail for its month labels (the compiler writes them under the
    // last panel with a value scale).
    let month = lh(&t.label) + f64::from(s[1]);
    let block = sec + head + tight + lh(&t.title) + tight + look.open.note_lines * lh(&t.label);
    Slots {
        sec,
        tight,
        month,
        h: [
            (look.open.claim_lines * lh(&t.headline)).max(look.glyph.card_px),
            block,
            look.open.thumb_lines * lh(&t.body),
            month + look.open.why_lines * lh(&t.body),
            block,
            sec + head + tight + look.open.ledger_lines * lh(&t.body),
        ],
    }
}

/// The capsule depth's frame height in `look`: the blocks, the gaps between them and the
/// plot margins.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn height(look: &Look) -> u32 {
    let sl = slots(look);
    let [_, _, top, bottom] = look.space.plot;
    let gaps = f64::from(look.space.panel_gap) * (sl.h.len() as f64 - 1.0);
    let sum: f64 = sl.h.iter().sum();
    (f64::from(top + bottom) + gaps + sum).ceil().max(1.0) as u32
}

// ---------------------------------------------------------------------------
// Texts (shared by the scene and the text column)
// ---------------------------------------------------------------------------

/// What the texts read.
struct Src {
    lang: Language,
    headline: String,
    voice: Option<CapsuleVoice>,
    facts: Vec<Fact>,
    trust: Trust,
    check: Option<DeviceCheck>,
    ledger: LedgerStat,
}

fn tr(lang: Language) -> bool {
    lang == Language::Tr
}

fn src(finding: &str, p: &dyn DataProvider) -> Option<Src> {
    let c = p.capsule(finding)?;
    let story = c.story.as_ref()?;
    let f = p.finding(finding)?;
    let facts: Vec<Fact> = f
        .facts
        .iter()
        .filter_map(|id| p.fact(id).map(Cow::into_owned))
        .collect();
    let voice = p.voice(finding);
    let headline = voice
        .as_ref()
        .map(|v| v.headline.clone())
        .or_else(|| p.headline(finding))
        .unwrap_or_else(|| f.claim.clone());
    Some(Src {
        lang: p.reader_lang().unwrap_or(Language::Tr),
        headline,
        voice,
        facts,
        trust: story.trust,
        check: p.device_check(finding),
        ledger: story.ledger_at_birth,
    })
}

fn header(key: &str, lang: Language) -> &'static str {
    match (key, tr(lang)) {
        ("basis", true) => "DAYANAK",
        ("basis", false) => "BASIS",
        ("trust", true) => "KİM DOĞRULADI",
        ("trust", false) => "WHO VERIFIED IT",
        (_, true) => "KARNE",
        (_, false) => "TRACK RECORD",
    }
}

/// The claim without its validity chip (the voice writes `claim · when, validity`).
fn claim(s: &Src) -> &str {
    match s.headline.rsplit_once(" · ") {
        Some((c, _)) if s.voice.is_some() => c,
        _ => &s.headline,
    }
}

/// Upper-cases the first letter (Turkish `i` → `İ`).
fn capitalize(s: &str, lang: Language) -> String {
    let mut cs = s.chars();
    match cs.next() {
        Some('i') if tr(lang) => format!("İ{}", cs.as_str()),
        Some(c) => format!("{}{}", c.to_uppercase(), cs.as_str()),
        None => String::new(),
    }
}

fn family(f: EvidenceFamily, lang: Language) -> &'static str {
    match (f, tr(lang)) {
        (EvidenceFamily::Price, true) => "Fiyat",
        (EvidenceFamily::Price, false) => "Price",
        (EvidenceFamily::Volume, true) => "Hacim",
        (EvidenceFamily::Volume, false) => "Volume",
        (EvidenceFamily::Flow, true) => "Akış",
        (EvidenceFamily::Flow, false) => "Flow",
        (EvidenceFamily::Derivative, true) => "Türev",
        (EvidenceFamily::Derivative, false) => "Derivatives",
        (EvidenceFamily::News, true) => "Haber ve temel",
        (EvidenceFamily::News, false) => "News and fundamentals",
    }
}

/// The first fact with a meaning.
fn meant(facts: &[Fact]) -> Option<&Fact> {
    facts.iter().find(|f| f.meaning.is_some())
}

/// The evidence keys the threshold row shows.
fn keys(f: &Fact) -> Vec<String> {
    let Some(m) = &f.meaning else {
        return Vec::new();
    };
    match m.geometry.first() {
        Some(Geometry::Level { key, .. }) => vec![key.clone()],
        Some(Geometry::Band { upper, lower, .. }) => vec![lower.clone(), upper.clone()],
        Some(Geometry::Points { points }) => {
            let mut v: Vec<String> = points.first().map(|p| p.key.clone()).into_iter().collect();
            if points.len() > 1 {
                v.extend(points.last().map(|p| p.key.clone()));
            }
            v
        }
        None => m.condition.iter().map(|c| c.key.clone()).collect(),
    }
}

/// The reader may not see the evidence values (the authority redacted them, K33).
fn locked(f: &Fact) -> bool {
    let ks = keys(f);
    let geo_missing = f.meaning.as_ref().is_some_and(|m| {
        m.geometry.iter().any(|g| match g {
            Geometry::Level { key, .. } => !f.evidence.contains_key(key),
            Geometry::Band { upper, lower, .. } => {
                !f.evidence.contains_key(upper) || !f.evidence.contains_key(lower)
            }
            Geometry::Points { points } => points.iter().any(|p| !f.evidence.contains_key(&p.key)),
        })
    });
    geo_missing || ks.iter().any(|k| !f.evidence.contains_key(k))
}

/// Decimals of an evidence key's value and whether trailing zeros drop: the capsule
/// language's number rule (`qp_capsule::realize::key_cat`, `num::fmt`; prices 2 decimals,
/// oscillators up to 1, counts none, ratios up to 4), restated here (the grammar does not
/// depend on the engine).
fn key_digits(key: &str, rsi: bool) -> (usize, bool) {
    match key {
        "level" if rsi => (1, true),
        "value" | "prev" | "rsi" | "prev_rsi" | "p1_rsi" | "p2_rsi" => (1, true),
        "level" | "close" | "price" | "fast" | "slow" | "sma20" | "sma50" | "sma20_prev5"
        | "atr" | "p1_price" | "p2_price" | "upper" | "lower" | "middle" | "half_width" => {
            (2, false)
        }
        "net" | "avg5" | "volume" | "vol_avg" | "avg20" => (0, false),
        _ => (4, true),
    }
}

/// `|v|` with `dec` decimals in `lang` (the engine's rounding and grouping).
fn fmt_num(v: f64, dec: usize, trim: bool, lang: Language) -> String {
    let p = 10f64.powi(i32::try_from(dec).unwrap_or(0));
    let r = (v.abs() * p).round() / p;
    let mut s = format!("{r:.dec$}");
    if trim && s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i.to_owned(), Some(f.to_owned())),
        None => (s, None),
    };
    let (thou, mark) = if tr(lang) { ('.', ',') } else { (',', '.') };
    let cs: Vec<char> = int.chars().collect();
    let mut g = String::new();
    for (i, c) in cs.iter().enumerate() {
        if i > 0 && cs.len() > 3 && (cs.len() - i).is_multiple_of(3) {
            g.push(thou);
        }
        g.push(*c);
    }
    match frac {
        Some(f) => format!("{g}{mark}{f}"),
        None => g,
    }
}

fn value(f: &Fact, key: &str, lang: Language) -> Option<String> {
    let v = *f.evidence.get(key)?;
    let m = f.meaning.as_ref()?;
    let rsi = m.detector.contains("rsi")
        || m.condition
            .as_ref()
            .is_some_and(|c| c.subject == "rsi" || c.subject == "value");
    let (dec, trim) = key_digits(key, rsi);
    Some(fmt_num(v, dec, trim, lang))
}

/// The threshold's name: the voice's glossary term the claim uses first; else a word for
/// the geometry's shape.
fn threshold_name(s: &Src, f: &Fact) -> String {
    let c: Vec<char> = claim(s).chars().collect();
    let found = s.voice.as_ref().and_then(|v| {
        v.glossary
            .iter()
            .filter_map(|g| {
                qp_grammar::terms::cues(&g.term)
                    .iter()
                    .find_map(|cue| qp_grammar::terms::find(&c, cue))
                    .map(|(a, _)| (a, g.term.as_str()))
            })
            .min_by_key(|(a, _)| *a)
            .map(|(_, t)| t.to_owned())
    });
    match found {
        Some(t) => capitalize(&t, s.lang),
        None => {
            let shape = f.meaning.as_ref().and_then(|m| m.geometry.first());
            match (shape, tr(s.lang)) {
                (Some(Geometry::Band { .. }), true) => "Bant".to_owned(),
                (Some(Geometry::Band { .. }), false) => "Band".to_owned(),
                (Some(Geometry::Points { .. }), true) => "Dönüş noktaları".to_owned(),
                (Some(Geometry::Points { .. }), false) => "Turning points".to_owned(),
                (_, true) => "Eşik".to_owned(),
                (_, false) => "Threshold".to_owned(),
            }
        }
    }
}

#[allow(clippy::expect_used)]
/// The basis row: the threshold by name and value, or (locked) what is locked and which
/// entitlement opens it.
fn basis(s: &Src) -> String {
    let Some(f) = meant(&s.facts) else {
        return String::new();
    };
    let m = f.meaning.as_ref().expect("meant");
    if locked(f) {
        return match (&m.license.entitlement, tr(s.lang)) {
            (Some(e), true) => format!("Kilitli: {e} yetkisi"),
            (Some(e), false) => format!("Locked: the {e} entitlement"),
            (None, true) => format!(
                "Kilitli: {} {} verisi yetkisi",
                m.instrument,
                family(m.family, s.lang).to_lowercase()
            ),
            (None, false) => format!(
                "Locked: the {} {} data entitlement",
                m.instrument,
                family(m.family, s.lang).to_lowercase()
            ),
        };
    }
    let vals: Vec<String> = keys(f).iter().filter_map(|k| value(f, k, s.lang)).collect();
    let sep = match m.geometry.first() {
        Some(Geometry::Points { .. }) => " → ",
        _ => " – ",
    };
    let name = threshold_name(s, f);
    if vals.is_empty() {
        name
    } else {
        format!("{name} · {}", vals.join(sep))
    }
}

#[allow(clippy::expect_used)]
/// The basis row's note: the evidence family and its licence (locked: that the values
/// open with that entitlement).
fn basis2(s: &Src) -> String {
    let Some(f) = meant(&s.facts) else {
        return String::new();
    };
    let m = f.meaning.as_ref().expect("meant");
    let fam = family(m.family, s.lang);
    let licensed = m.license.class == LicenseClass::Licensed;
    match (locked(f), licensed, tr(s.lang)) {
        (true, true, true) => format!("{fam} verisi lisanslı; değerler bu yetkiyle açılır."),
        (true, true, false) => {
            format!("{fam} data is licensed; the values open with this entitlement.")
        }
        (true, false, true) => format!("{fam} verisi; değerler bu yetkiyle açılır."),
        (true, false, false) => format!("{fam} data; the values open with this entitlement."),
        (false, true, true) => format!("{fam} verisi · lisanslı, sunucuda hesaplanır"),
        (false, true, false) => format!("{fam} data · licensed, computed on the server"),
        (false, false, true) => format!("{fam} verisi · herkese açık, cihaz yeniden hesaplar"),
        (false, false, false) => format!("{fam} data · public, the device recomputes it"),
    }
}

fn why(s: &Src) -> String {
    s.voice
        .as_ref()
        .map(|v| v.detail.clone())
        .unwrap_or_default()
}

/// The trust row: the same words as the open layer's trust mark.
fn trust(s: &Src) -> &'static str {
    match (s.check, tr(s.lang)) {
        (Some(DeviceCheck::Device), true) => "Bu cihazda sınandı",
        (Some(DeviceCheck::Device), false) => "Checked on this device",
        (Some(DeviceCheck::Checking), true) => "Bu cihazda sınanıyor",
        (Some(DeviceCheck::Checking), false) => "Checking on this device",
        (Some(DeviceCheck::Differs), true) => "Kaynaklar farklı",
        (Some(DeviceCheck::Differs), false) => "Sources differ",
        (_, true) => "Sunucu doğruladı",
        (_, false) => "Verified by the server",
    }
}

/// What the trust row means here.
fn trust2(s: &Src) -> &'static str {
    let server = s.trust == Trust::Server;
    match (s.check, server, tr(s.lang)) {
        (Some(DeviceCheck::Device), _, true) => {
            "Sunucu da doğruladı; telefon aynı kuralı kendi verisiyle yeniden işletti."
        }
        (Some(DeviceCheck::Device), _, false) => {
            "The server verified it too; the phone ran the same rule on its own data."
        }
        (Some(DeviceCheck::Checking), _, true) => {
            "Sunucu doğruladı; bu cihazın verisi henüz o anı kapsamıyor."
        }
        (Some(DeviceCheck::Checking), _, false) => {
            "The server verified it; this device's data does not reach that moment yet."
        }
        (Some(DeviceCheck::Differs), _, true) => {
            "Sunucu doğruladı ama bu cihazın verisi başka bir sonuç veriyor."
        }
        (Some(DeviceCheck::Differs), _, false) => {
            "The server verified it, but this device's data gives another result."
        }
        (_, true, true) => "Lisanslı veri: bu cihaz kendisi sınayamaz.",
        (_, true, false) => "Licensed data: this device cannot check it itself.",
        (_, false, true) => "Açıkken bu cihaz da kendi verisiyle sınar.",
        (_, false, false) => "While open, this device checks it on its own data too.",
    }
}

/// The trust row's explanation (K49; at most 160 characters, no digits): by the trust
/// path only, so that every scene of the capsule (the authority's and the device's) has
/// the same entry.
fn trust_gloss(s: &Src) -> GlossEntry {
    let server = s.trust == Trust::Server;
    let text = match (server, tr(s.lang)) {
        (true, true) => "Sunucu doğruladı: iddia sunucuda, lisanslı veriyle hesaplanıp kesinleşti. Bu cihaz o veriyi almadığı için kendisi yeniden sınayamaz.",
        (true, false) => "Verified by the server: computed and settled on the server from licensed data. This device does not get that data, so it cannot re-check it.",
        (false, true) => "Sunucu doğruladı: iddia sunucuda kesinleşti. Bu cihazda sınandı: telefon aynı kuralı kendi aldığı fiyatlarla yeniden işletti, aynı sonucu buldu.",
        (false, false) => "Verified by the server: settled on the server. Checked on this device: the phone ran the same rule on the prices it got itself and found the same.",
    };
    GlossEntry {
        term: if tr(s.lang) {
            "Kim doğruladı?"
        } else {
            "Who verified it?"
        }
        .to_owned(),
        text: text.to_owned(),
    }
}

fn ledger(s: &Src) -> String {
    s.voice
        .as_ref()
        .map_or_else(|| depth::ledger_text(&s.ledger), |v| v.ledger.clone())
}

/// A finding row's `open.*` field.
pub(super) fn field(key: &str, finding: &str, p: &dyn DataProvider) -> Option<String> {
    let s = src(finding, p)?;
    Some(match key {
        "open.h.basis" => header("basis", s.lang).to_owned(),
        "open.h.trust" => header("trust", s.lang).to_owned(),
        "open.h.ledger" => header("ledger", s.lang).to_owned(),
        "open.basis" => basis(&s),
        "open.basis2" => basis2(&s),
        "open.why" => why(&s),
        "open.trust" => trust(&s).to_owned(),
        "open.trust2" => trust2(&s).to_owned(),
        _ => return None,
    })
}

/// The layer whose panel opens the trust explanation.
const TRUST_LAYER: &str = "open-trust";

pub(super) fn owns_term(layer: &str) -> bool {
    layer.starts_with(TRUST_LAYER)
}

pub(super) fn layer_term(layer: &str, finding: &str, p: &dyn DataProvider) -> Option<GlossEntry> {
    (layer == TRUST_LAYER)
        .then(|| src(finding, p))
        .flatten()
        .map(|s| trust_gloss(&s))
}

/// The text column for a reader without a scene (A32): the same blocks as lines.
fn text_column(t: &TextInput<'_>) -> String {
    let s = Src {
        lang: t.lang,
        headline: t
            .voice
            .map_or_else(|| t.headline.to_owned(), |v| v.headline.clone()),
        voice: t.voice.cloned(),
        facts: t.facts.to_vec(),
        trust: t.trust,
        check: t.check,
        ledger: t.ledger,
    };
    let mut out: Vec<String> = vec![s.headline.clone(), String::new()];
    out.push(header("basis", s.lang).to_owned());
    for l in [basis(&s), basis2(&s), why(&s)] {
        if !l.is_empty() {
            out.push(l);
        }
    }
    out.push(String::new());
    out.push(header("trust", s.lang).to_owned());
    out.push(trust(&s).to_owned());
    // No scene: this reader gets no data of its own to check it on.
    out.push(
        match (s.check, tr(s.lang)) {
            (Some(DeviceCheck::Device | DeviceCheck::Checking | DeviceCheck::Differs), _) => {
                trust2(&s)
            }
            (_, true) => {
                "Sunucu hesapladı ve doğruladı; bu cihaz veriyi almadığı için kendisi sınayamaz."
            }
            (_, false) => {
                "The server computed and verified it; this device gets no data to check it itself."
            }
        }
        .to_owned(),
    );
    out.push(String::new());
    out.push(header("ledger", s.lang).to_owned());
    out.push(ledger(&s));
    let _ = t.validity;
    out.join("\n")
}
