//! R5/M20 `nobra`: **question cards**. The open capsule is organised around the questions a
//! reader asks of it, each a card with a one-line answer, its detail opened on touch:
//!
//! 1. **Ne oldu?** (`what`) — the capsule's glyph; whether the claim still holds, or how it
//!    closed; detail: the voice's detail sentence (A33).
//! 2. **Kim doğruladı?** (`who`) — "verified by the server" or what this device's own check
//!    found (K48); detail: what that means (licensed data the device cannot recompute, or the
//!    same rule re-run on the device's own data of this instrument only).
//! 3. **Daha önce kaç kez?** (`before`) — the track record sentence (the capsule's `ledger`
//!    text, the same field the card writes, so a transition keeps it); detail: how it is
//!    counted, "few samples" when there are few, and that it is a past frequency.
//! 4. **Neden rakam göremiyorum?** (`why`) — only for a reader whose evidence values are
//!    withheld (K33/K34, A32): the evidence exists, what it rests on, which entitlement it
//!    needs; the comment and the track record stay open to everyone.
//! 5. **Neye dayanıyor?** (`basis`) — the evidence families; the card holds the evidence
//!    geometry over the price (candles, level / band / swing points bound by origin, K33)
//!    in the panel under it; detail: what each mark is. A tap on that chart opens the
//!    chart depth.
//!
//! Every answer is visible without a tap; a tap on a card opens (or closes) its detail
//! (`ViewState::ask`), one at a time. Texts are text fields (`ask/<lang>/<key>/<part>/<check>`)
//! resolved at compile time from the provider ([`field_text`]): a view carries no text and
//! no number. A reader whose evidence is withheld gets no digit from any of them.
//!
//! Sizes: type roles `label` (question, detail), `body` (answer), glyph `card_px`, spacing
//! `space.scale`, `space.glyph_gap`, `space.panel_gap`, `space.plot`; colours: roles `muted`,
//! `text`, `warn`, `look.evidence.*`. Layout in pixels from the frame (no constant of its own).

use std::borrow::Cow;

use qp_schema::capsule::{
    EvidenceFamily, Geometry, LedgerStat, LicenseClass, Stage, Trust, Validity,
};
use qp_schema::grammar::{
    ColorSpec, Layer, Mark, Panel, PanelPos, Position, Scale, ScaleKind, SizeSpec, View,
};
use qp_schema::protocol::{CapsuleItem, HitInfo, Language, OpenDepth};
use qp_schema::Fact;

use super::{Check, OpenCtx, OpenTap};
use crate::depth::{evidence_layers, evidence_note, CHART_LOOKBACK, GLYPH_MIN_LEDGER_N};
use crate::draw::wrap;
use crate::error::{ErrorKind, GrammarError};
use crate::look::Look;
use crate::provider::{DataProvider, SeriesKind};

/// The questions, in screen order.
pub const ASKS: [&str; 5] = ["what", "who", "before", "why", "basis"];

/// Panel name prefix of a question card; the chart under `basis` is [`CHART_PANEL`].
const PANEL: &str = "ask:";
/// Panel name of the evidence chart.
pub const CHART_PANEL: &str = "ask:chart";

fn lang_key(l: Language) -> &'static str {
    match l {
        Language::En => "en",
        Language::Tr => "tr",
    }
}

fn parse_lang(s: &str) -> Option<Language> {
    match s {
        "tr" => Some(Language::Tr),
        "en" => Some(Language::En),
        _ => None,
    }
}

/// The text field of one part (`q` question, `a` answer, `d` detail) of question `key`.
fn field(lang: Language, key: &str, part: &str, check: Option<Check>) -> String {
    format!("ask/{}/{key}/{part}/{}", lang_key(lang), Check::name(check))
}

/// What the texts read, gathered once: from the provider (a reader with a scene) or from
/// the feed item (a reader without one, [`text_rows`]).
struct Facts {
    stage: Stage,
    validity: Option<Validity>,
    trust: Trust,
    ledger: LedgerStat,
    families: Vec<EvidenceFamily>,
    confirmations: u8,
    licensed: bool,
    /// The reader's evidence values are withheld (K33): no digit in any text.
    withheld: bool,
    /// The reader gets no data of the instrument at all (no scene, A32).
    no_data: bool,
    entitlement: Option<String>,
    instrument: String,
    geometry: [bool; 3],
    voice: Option<qp_schema::protocol::CapsuleVoice>,
    claim: String,
}

impl Facts {
    fn read(finding: &str, p: &dyn DataProvider) -> Option<Self> {
        let f = p.finding(finding)?;
        let fact = p.fact(f.facts.first()?)?;
        let story = p.capsule(finding)?.story.clone()?;
        let validity = p.glyph(finding).and_then(|g| g.validity);
        let meaning = fact.meaning.as_ref();
        let geo = |k: u8| {
            meaning.is_some_and(|m| {
                m.geometry.iter().any(|g| {
                    matches!(
                        (k, g),
                        (0, Geometry::Level { .. })
                            | (1, Geometry::Band { .. })
                            | (2, Geometry::Points { .. })
                    )
                })
            })
        };
        Some(Self {
            stage: story.stage,
            validity,
            trust: story.trust,
            ledger: story.ledger_at_birth,
            confirmations: u8::try_from(story.families.len()).unwrap_or(u8::MAX),
            families: story.families,
            licensed: meaning.is_some_and(|m| m.license.class == LicenseClass::Licensed),
            withheld: evidence_note(&fact).is_some(),
            no_data: false,
            entitlement: meaning.and_then(|m| m.license.entitlement.clone()),
            instrument: story.instrument,
            geometry: [geo(0), geo(1), geo(2)],
            voice: p.voice(finding),
            claim: f.claim.clone(),
        })
    }

    fn of_item(item: &CapsuleItem) -> Self {
        Self {
            stage: item.stage,
            validity: Some(item.validity),
            trust: item.trust,
            ledger: item.ledger_at_birth,
            families: Vec::new(),
            confirmations: item.confirmations,
            licensed: item.trust == Trust::Server,
            withheld: true,
            no_data: true,
            entitlement: None,
            instrument: item.instrument.clone(),
            geometry: [false; 3],
            voice: item.voice.clone(),
            claim: item.sentence.text.clone(),
        }
    }
}

fn family_name(f: EvidenceFamily, l: Language) -> &'static str {
    match (f, l) {
        (EvidenceFamily::Price, Language::Tr) => "fiyat",
        (EvidenceFamily::Volume, Language::Tr) => "hacim",
        (EvidenceFamily::Flow, Language::Tr) => "para akışı",
        (EvidenceFamily::Derivative, Language::Tr) => "türev piyasa",
        (EvidenceFamily::News, Language::Tr) => "haber ve temel veri",
        (EvidenceFamily::Price, Language::En) => "price",
        (EvidenceFamily::Volume, Language::En) => "volume",
        (EvidenceFamily::Flow, Language::En) => "money flow",
        (EvidenceFamily::Derivative, Language::En) => "derivatives",
        (EvidenceFamily::News, Language::En) => "news and fundamentals",
    }
}

fn count_word(n: u8, l: Language) -> &'static str {
    match (n, l) {
        (0 | 1, Language::Tr) => "bir",
        (2, Language::Tr) => "iki",
        (3, Language::Tr) => "üç",
        (4, Language::Tr) => "dört",
        (_, Language::Tr) => "beş",
        (0 | 1, Language::En) => "one",
        (2, Language::En) => "two",
        (3, Language::En) => "three",
        (4, Language::En) => "four",
        (_, Language::En) => "five",
    }
}

/// "fiyat ve hacim verisi"; without the family names (a feed item), "iki bağımsız kanıt
/// ailesi" in words (no digit).
fn families_of(x: &Facts, l: Language) -> String {
    if x.families.is_empty() {
        let w = count_word(x.confirmations, l);
        return match (x.confirmations > 1, l) {
            (true, Language::Tr) => format!("{w} bağımsız kanıt ailesi"),
            (false, Language::Tr) => "tek bir kanıt ailesi".to_owned(),
            (true, Language::En) => format!("{w} independent evidence families"),
            (false, Language::En) => "a single evidence family".to_owned(),
        };
    }
    match l {
        Language::Tr => format!("{} verisi", families(&x.families, l)),
        Language::En => format!("{} data", families(&x.families, l)),
    }
}

/// "fiyat", "fiyat ve hacim", "fiyat, hacim ve para akışı".
fn families(fs: &[EvidenceFamily], l: Language) -> String {
    let names: Vec<&str> = fs.iter().map(|f| family_name(*f, l)).collect();
    let and = match l {
        Language::Tr => " ve ",
        Language::En => " and ",
    };
    match names.as_slice() {
        [] => family_name(EvidenceFamily::Price, l).to_owned(),
        [one] => (*one).to_owned(),
        [init @ .., last] => format!("{}{and}{last}", init.join(", ")),
    }
}

fn capitalised(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

fn question(key: &str, l: Language) -> &'static str {
    match (key, l) {
        ("what", Language::Tr) => "Ne oldu?",
        ("who", Language::Tr) => "Kim doğruladı?",
        ("before", Language::Tr) => "Daha önce kaç kez?",
        ("why", Language::Tr) => "Neden rakam göremiyorum?",
        ("basis", Language::Tr) => "Neye dayanıyor?",
        ("what", Language::En) => "What happened?",
        ("who", Language::En) => "Who verified it?",
        ("before", Language::En) => "How often before?",
        ("why", Language::En) => "Why can't I see numbers?",
        ("basis", Language::En) => "What does it rest on?",
        _ => "",
    }
}

fn what_answer(x: &Facts, l: Language) -> String {
    let tr = l == Language::Tr;
    let s = x.stage;
    if s.is_closed() {
        return match (s, tr) {
            (Stage::Realized, true) => "Kapandı: beklenen oldu.",
            (Stage::Invalidated, true) => "Kapandı: geçersiz oldu, beklenen olmadı.",
            (Stage::Expired, true) => "Kapandı: süresi doldu, beklenen olmadı.",
            (_, true) => "Geri çekildi: dayandığı veri düzeltildi.",
            (Stage::Realized, false) => "Closed: it played out as expected.",
            (Stage::Invalidated, false) => "Closed: invalidated, it did not play out.",
            (Stage::Expired, false) => "Closed: its time ran out without playing out.",
            (_, false) => "Withdrawn: the data it rested on was corrected.",
        }
        .to_owned();
    }
    if x.validity == Some(Validity::NoLonger) {
        return if tr {
            "Artık geçerli değil; beklenen sonuç henüz belli değil."
        } else {
            "No longer holds; the expected outcome is still open."
        }
        .to_owned();
    }
    match (s, tr) {
        (Stage::Strengthened, true) => "Hâlâ geçerli; yeni bir olguyla güçlendi.",
        (Stage::Weakened, true) => "Hâlâ geçerli, ama karşı bir olguyla zayıfladı.",
        (Stage::Confirmed, true) => "Hâlâ geçerli; ikinci bir kanıt ailesi teyit etti.",
        (_, true) => "Hâlâ geçerli.",
        (Stage::Strengthened, false) => "Still holds; a new fact strengthened it.",
        (Stage::Weakened, false) => "Still holds, but a fact against it weakened it.",
        (Stage::Confirmed, false) => "Still holds; a second evidence family confirmed it.",
        (_, false) => "Still holds.",
    }
    .to_owned()
}

fn who_answer(check: Option<Check>, l: Language) -> &'static str {
    match (check, l) {
        (None, Language::Tr) => "Sunucu doğruladı.",
        (Some(Check::Checking), Language::Tr) => "Sunucu doğruladı; bu cihazda sınanıyor.",
        (Some(Check::Device), Language::Tr) => "Sunucu doğruladı ve bu cihazda da sınandı.",
        (Some(Check::Differs), Language::Tr) => {
            "Kaynaklar farklı: bu cihazın verisi başka sonuç veriyor."
        }
        (None, Language::En) => "Verified by the server.",
        (Some(Check::Checking), Language::En) => "Verified by the server; checking on this device.",
        (Some(Check::Device), Language::En) => {
            "Verified by the server and checked on this device too."
        }
        (Some(Check::Differs), Language::En) => {
            "Sources differ: this device's data gives another result."
        }
    }
}

fn who_detail(x: &Facts, check: Option<Check>, l: Language) -> String {
    let tr = l == Language::Tr;
    let base = match (x.trust == Trust::Server || x.licensed, tr) {
        (true, true) => "Bu iddia lisanslı veriden sunucuda hesaplandı. Bu cihaz o veriyi almadığı için yeniden hesaplayamaz; gördüğün, sunucunun kesinleşmiş iddiası.",
        (true, false) => "This claim was computed on the server from licensed data. This device does not receive that data and cannot recompute it; what you see is the server's settled claim.",
        (false, true) => "Akıştaki her kapsül sunucunun kesinleşmiş iddiasıdır. Açtığında bu cihaz aynı kuralı yalnız bu hissenin kendi verisinde yeniden sınar.",
        (false, false) => "Every capsule in the feed is the server's settled claim. When you open one, this device re-runs the same rule on its own data of this instrument only.",
    };
    let more = match (check, tr) {
        (None, _) => "",
        (Some(Check::Checking), true) => " Bu cihazın verisi henüz o barı kapsamıyor.",
        (Some(Check::Device), true) => " Bu cihaz kendi verisinde aynı sonucu buldu.",
        (Some(Check::Differs), true) => {
            " Bu cihaz kendi verisinde başka sonuç buldu; iki kaynaktan biri farklı."
        }
        (Some(Check::Checking), false) => " This device's data does not cover that bar yet.",
        (Some(Check::Device), false) => " This device found the same result on its own data.",
        (Some(Check::Differs), false) => {
            " This device found another result on its own data; one of the two sources differs."
        }
    };
    format!("{base}{more}")
}

fn ledger_answer(x: &Facts, l: Language) -> String {
    if let Some(v) = &x.voice {
        return v.ledger.clone();
    }
    let st = x.ledger;
    match (st.n, x.withheld, l) {
        (0, _, Language::Tr) => "Henüz karne yok.".to_owned(),
        (0, _, Language::En) => "No track record yet.".to_owned(),
        (_, true, Language::Tr) => "Karnesi var; ayrıntıya dokun.".to_owned(),
        (_, true, Language::En) => "It has a track record; tap for the detail.".to_owned(),
        (_, false, _) => crate::depth::ledger_text(&st),
    }
}

fn before_detail(x: &Facts, l: Language) -> String {
    let tr = l == Language::Tr;
    let st = x.ledger;
    let past = if tr {
        " Karne geleceği söylemez; geçmişteki sıklığı gösterir ve veri yetkisinden bağımsızdır."
    } else {
        " A track record does not tell the future; it shows a past frequency and does not depend on the data entitlement."
    };
    if st.n == 0 {
        return if tr {
            format!("Bu türden, bu kapsülden önce kapanmış bir hikâye yok.{past}")
        } else {
            format!("No story of this kind closed before this capsule.{past}")
        };
    }
    let few = st.n < GLYPH_MIN_LEDGER_N;
    if x.withheld {
        // No digit for this reader (K33): the counting rule, not the counts.
        return match (few, tr) {
            (true, true) => format!("Aynı tür, yön ve kuralla doğup bu kapsülden önce kapanan hikâyelerden sayılır; az örnek var, oran henüz güvenilir değil.{past}"),
            (false, true) => format!("Aynı tür, yön ve kuralla doğup bu kapsülden önce kapanan hikâyelerden sayılır.{past}"),
            (true, false) => format!("Counted over stories of the same kind, direction and rule that closed before this capsule; there are few samples, the rate is not reliable yet.{past}"),
            (false, false) => format!("Counted over stories of the same kind, direction and rule that closed before this capsule.{past}"),
        };
    }
    let (n, r) = (st.n, st.realized);
    let head = if tr {
        format!("Aynı tür, yön ve kuralla doğup bu kapsülden önce kapanan {n} hikâyenin {r} tanesinde beklenen oldu.")
    } else {
        format!("Of {n} stories of the same kind, direction and rule that closed before this capsule, {r} played out as expected.")
    };
    let few = match (few, tr) {
        (true, true) => " Az örnek: bu oran henüz güvenilir değil.",
        (true, false) => " Few samples: this rate is not reliable yet.",
        _ => "",
    };
    format!("{head}{few}{past}")
}

fn basis_answer(x: &Facts, l: Language) -> String {
    let fams = families_of(x, l);
    let tr = l == Language::Tr;
    let mut s = if tr {
        format!("{}ne dayanıyor.", capitalised(&fams))
    } else {
        format!("It rests on {fams}.")
    };
    if x.no_data {
        s.push_str(if tr {
            " Grafiği ve değerleri sende gizli."
        } else {
            " Its chart and values are withheld from you."
        });
    } else if x.withheld {
        s.push_str(if tr {
            " İşaretlerin değerleri sende gizli."
        } else {
            " Its marks' values are withheld from you."
        });
    } else if x.licensed {
        s.push_str(if tr {
            " Lisanslı veri: sunucuda hesaplandı."
        } else {
            " Licensed data: computed on the server."
        });
    }
    s
}

fn basis_detail(x: &Facts, l: Language) -> String {
    let tr = l == Language::Tr;
    let has = |k: usize| x.geometry[k];
    let mut parts: Vec<&str> = Vec::new();
    if x.no_data {
        return if tr {
            "Grafik ve kanıt işaretleri bu hissenin verisini gerektirir; veri yetkin olmadığı için çizilmedi."
        } else {
            "The chart and the evidence marks need this instrument's data; without the entitlement they are not drawn."
        }
        .to_owned();
    }
    if x.withheld {
        parts.push(if tr {
            "İşaretler, değerleri sende gizli olduğu için çizilmedi."
        } else {
            "The marks are not drawn: their values are withheld from you."
        });
    } else {
        if has(0) {
            parts.push(if tr {
                "Vurgulu çizgi, iddianın dayandığı seviye."
            } else {
                "The highlighted line is the level the claim rests on."
            });
        }
        if has(1) {
            parts.push(if tr {
                "Gölgeli bant, iddianın dayandığı aralık."
            } else {
                "The shaded band is the range the claim rests on."
            });
        }
        if has(2) {
            parts.push(if tr {
                "Noktalar, iddianın dayandığı dönüş noktaları."
            } else {
                "The points are the turns the claim rests on."
            });
        }
    }
    parts.push(if tr {
        "Grafiğe geçmek için grafiğe dokun."
    } else {
        "Tap the chart to open it."
    });
    parts.join(" ")
}

fn why_answer(l: Language) -> &'static str {
    match l {
        Language::Tr => "Kanıtın değerleri veri yetkisi gerektiriyor.",
        Language::En => "The evidence's values need a data entitlement.",
    }
}

fn why_detail(x: &Facts, l: Language) -> String {
    let fams = families_of(x, l);
    let i = &x.instrument;
    match (x.no_data, x.entitlement.as_deref(), l) {
        (true, _, Language::Tr) => format!("Kanıt var ve {fams}ne dayanıyor. Rakamları ve grafiği görmek için {i} verisinin yetkisi gerekiyor. Yorum ve karne herkese açık."),
        (false, Some(e), Language::Tr) => format!("Kanıt var ve {fams}ne dayanıyor. Değerlerini görmek için gereken yetki: {e}. Yorum ve karne herkese açık."),
        (false, None, Language::Tr) => format!("Kanıt var ve {fams}ne dayanıyor; değerleri bu okuyucuya açık değil. Yorum ve karne herkese açık."),
        (true, _, Language::En) => format!("The evidence exists and rests on {fams}. Seeing its numbers and chart needs the data entitlement for {i}. The comment and the track record are open to everyone."),
        (false, Some(e), Language::En) => format!("The evidence exists and rests on {fams}. The entitlement needed to see its values: {e}. The comment and the track record are open to everyone."),
        (false, None, Language::En) => format!("The evidence exists and rests on {fams}; its values are not open to this reader. The comment and the track record are open to everyone."),
    }
}

/// The text of one part of one question, from the provider's data (`None`: the question
/// does not apply, e.g. `why` for a reader who sees the values).
fn text(x: &Facts, key: &str, part: &str, l: Language, check: Option<Check>) -> Option<String> {
    if key == "why" && !x.withheld {
        return None;
    }
    Some(match (key, part) {
        (_, "q") => question(key, l).to_owned(),
        ("what", "a") => what_answer(x, l),
        ("what", "d") => match &x.voice {
            Some(v) => v.detail.clone(),
            None => x.claim.clone(),
        },
        ("who", "a") => who_answer(check, l).to_owned(),
        ("who", "d") => who_detail(x, check, l),
        ("before", "a") => ledger_answer(x, l),
        ("before", "d") => before_detail(x, l),
        ("why", "a") => why_answer(l).to_owned(),
        ("why", "d") => why_detail(x, l),
        ("basis", "a") => basis_answer(x, l),
        ("basis", "d") => basis_detail(x, l),
        _ => return None,
    })
}

/// The text of field `rest` (`<lang>/<key>/<part>/<check>`, after `ask/`) for `finding`.
#[must_use]
pub fn field_text(rest: &str, finding: &str, p: &dyn DataProvider) -> Option<String> {
    let mut it = rest.split('/');
    let (l, key, part, check) = (it.next()?, it.next()?, it.next()?, it.next()?);
    let l = parse_lang(l)?;
    let check = Check::parse(check)?;
    let x = Facts::read(finding, p)?;
    text(&x, key, part, l, check)
}

fn layer(id: &str, mark: Mark, panel: u32) -> Layer {
    let mut l = Layer::new(mark, panel);
    l.id = Some(id.to_owned());
    l
}

fn at(px: f64) -> Position {
    Position::Panel(PanelPos { panel: 0.0, px })
}

fn panel(id: u32, name: &str, weight: f64) -> Panel {
    Panel {
        id: Some(id),
        name: Some(name.to_owned()),
        weight: Some(weight),
        y: Scale {
            kind: ScaleKind::Linear,
            domain: None,
            id: Some(format!("y{id}")),
        },
    }
}

/// Pixel sizes of the look the layout uses.
struct Metrics {
    q_px: f64,
    q_h: f64,
    a_px: f64,
    a_h: f64,
    d_px: f64,
    d_h: f64,
    pad: f64,
    inner: f64,
}

impl Metrics {
    fn of(look: &Look) -> Self {
        let t = &look.types;
        Self {
            q_px: t.label.px,
            q_h: (t.label.px * t.label.line).round(),
            a_px: t.body.px,
            a_h: (t.body.px * t.body.line).round(),
            d_px: t.label.px,
            d_h: (t.label.px * t.label.line).round(),
            pad: f64::from(look.space.scale[1]),
            inner: f64::from(look.space.scale[0]),
        }
    }
}

fn lines(look: &Look, role: &str, text: &str, room: i32, px: f64) -> usize {
    wrap(text, room, px, look.role_font(role)).len().max(1)
}

/// The capsule-depth view of `nobra` (see the module). A finding without a story falls
/// back to the reference card.
///
/// # Errors
/// Unknown finding/fact, a finding without facts, or a series without bars.
#[allow(clippy::too_many_lines)]
pub fn view(
    look: &Look,
    finding: &str,
    max_points: u32,
    p: &dyn DataProvider,
    id: &str,
    ctx: &OpenCtx<'_>,
) -> Result<View, GrammarError> {
    let Some(x) = Facts::read(finding, p) else {
        return crate::depth::capsule_view_in(look, finding, max_points, p, id);
    };
    let not_found = |why: String| GrammarError::new(ErrorKind::NotFound, "/finding", why);
    let f0 = p
        .finding(finding)
        .ok_or_else(|| not_found(format!("finding `{finding}` does not exist")))?;
    let fact = f0
        .facts
        .first()
        .and_then(|id| p.fact(id))
        .ok_or_else(|| not_found(format!("finding `{finding}` has no facts")))?;
    let bs = match p.series_kind(&fact.series) {
        Some(SeriesKind::Bars) => Some(fact.series.clone()),
        Some(SeriesKind::Values) => p.input_bars(&fact.series),
        _ => None,
    }
    .ok_or_else(|| not_found(format!("series `{}` has no bars", fact.series)))?;
    let bars = p
        .bars(&bs)
        .filter(|b| !b.is_empty())
        .ok_or_else(|| not_found(format!("series `{bs}` has no bars")))?;
    let n = bars.len() as u64;
    let l = ctx.lang;
    let m = Metrics::of(look);
    let [w, h] = ctx.frame;
    let [left, right, top, bottom] = look.space.plot;
    let gap = f64::from(look.space.panel_gap);
    #[allow(clippy::cast_possible_wrap)]
    let room = w as i32 - left - right;
    let glyph_w = look.glyph.card_px + look.space.glyph_gap;
    let asks: Vec<&str> = ASKS
        .iter()
        .copied()
        .filter(|k| text(&x, k, "q", l, None).is_some())
        .collect();
    let open = ctx.ask.filter(|a| asks.contains(a));
    // One card panel per question and the chart: the gaps between them.
    #[allow(clippy::cast_precision_loss)]
    let gaps = gap * asks.len() as f64;
    let avail = (f64::from(h) - f64::from(top + bottom) - gaps).max(0.0);
    // Collapsed heights: the question line and the answer (at most two lines; the `who`
    // answer at the longest of its check states, so the authority's layout — which knows no
    // device check — is the device's).
    let mut heights: Vec<f64> = Vec::new();
    let mut a_lines: Vec<usize> = Vec::new();
    for k in &asks {
        let x_room = if *k == "what" {
            #[allow(clippy::cast_possible_truncation)]
            let g = glyph_w.ceil() as i32;
            room - g
        } else {
            room
        };
        let checks: &[Option<Check>] = if *k == "who" { &Check::ALL } else { &[None] };
        let n_a = checks
            .iter()
            .filter_map(|c| text(&x, k, "a", l, *c))
            .map(|t| lines(look, "body", &t, x_room, m.a_px))
            .max()
            .unwrap_or(1)
            .min(2);
        a_lines.push(n_a);
        #[allow(clippy::cast_precision_loss)]
        let mut hh = m.q_h + m.inner + n_a as f64 * m.a_h + m.pad;
        if *k == "what" {
            hh = hh.max(look.glyph.card_px + m.pad);
        }
        heights.push(hh);
    }
    // The opened question's detail: its lines (the `who` detail at the longest state).
    let chart_min = (avail / 4.0).max(48.0).floor();
    if let Some(k) = open {
        let i = asks.iter().position(|a| *a == k).unwrap_or(0);
        let x_room = if k == "what" {
            #[allow(clippy::cast_possible_truncation)]
            let g = glyph_w.ceil() as i32;
            room - g
        } else {
            room
        };
        let checks: &[Option<Check>] = if k == "who" { &Check::ALL } else { &[None] };
        let n_d = checks
            .iter()
            .filter_map(|c| text(&x, k, "d", l, *c))
            .map(|t| lines(look, "label", &t, x_room, m.d_px))
            .max()
            .unwrap_or(1);
        #[allow(clippy::cast_precision_loss)]
        let want = m.inner + n_d as f64 * m.d_h;
        let used: f64 = heights.iter().sum();
        let spare = (avail - used - chart_min).max(m.d_h);
        heights[i] += want.min(spare).floor();
    }
    let used: f64 = heights.iter().sum();
    let chart_h = (avail - used).max(10.0);

    let mut panels = Vec::new();
    let mut layers = Vec::new();
    for (pi, k) in asks.iter().enumerate() {
        let pid = u32::try_from(pi).unwrap_or(0);
        panels.push(panel(pid, &format!("{PANEL}{k}"), heights[pi].max(10.0)));
        let x0 = if *k == "what" {
            let mut g = layer("glyph", Mark::Glyph, pid);
            g.finding = Some(finding.to_owned());
            g.encoding.x = Some(at(0.0));
            g.encoding.y = Some(at(0.0));
            g.encoding.size = Some(SizeSpec::Constant(look.glyph.card_px));
            layers.push(g);
            glyph_w
        } else {
            0.0
        };
        let opened = open == Some(*k);
        let text_layer = |part: &str, y: f64, px: f64, color: &str, check: Option<Check>| {
            let mut t = layer(&format!("{k}-{part}"), Mark::Text, pid);
            t.finding = Some(finding.to_owned());
            t.encoding.text = Some(field(l, k, part, check));
            t.encoding.x = Some(at(x0));
            t.encoding.y = Some(at(y));
            t.encoding.size = Some(SizeSpec::Constant(px));
            t.encoding.color = Some(ColorSpec::Constant(color.to_owned()));
            t
        };
        layers.push(text_layer("q", 0.0, m.q_px, "muted", None));
        let a_y = m.q_h + m.inner;
        let check = if *k == "who" { ctx.check } else { None };
        let a_color = if check == Some(Check::Differs) {
            "warn"
        } else {
            "text"
        };
        if *k == "before" {
            // The capsule's own `ledger` text (the card's field: a transition keeps it).
            let mut t = layer("before-a", Mark::Text, pid);
            t.finding = Some(finding.to_owned());
            t.encoding.text = Some(if p.voice(finding).is_some() || !x.withheld {
                "ledger".to_owned()
            } else {
                field(l, k, "a", None)
            });
            t.encoding.x = Some(at(x0));
            t.encoding.y = Some(at(a_y));
            t.encoding.size = Some(SizeSpec::Constant(m.a_px));
            t.encoding.color = Some(ColorSpec::Constant("text".to_owned()));
            layers.push(t);
        } else {
            layers.push(text_layer("a", a_y, m.a_px, a_color, check));
        }
        if opened {
            #[allow(clippy::cast_precision_loss)]
            let d_y = a_y + a_lines[pi] as f64 * m.a_h + m.inner;
            layers.push(text_layer("d", d_y, m.d_px, "text", check));
        }
    }
    // The evidence chart under `basis`: candles of the bar series and the evidence geometry
    // bound by origin (the chart depth's rule, `depth::evidence_layers`).
    let cid = u32::try_from(asks.len()).unwrap_or(0);
    panels.push(panel(cid, CHART_PANEL, chart_h));
    let mut from = fact.bar.saturating_sub(CHART_LOOKBACK);
    for g in fact.meaning.iter().flat_map(|mm| &mm.geometry) {
        let first = match g {
            Geometry::Level { from_bar, .. } | Geometry::Band { from_bar, .. } => *from_bar,
            Geometry::Points { points } => points.iter().map(|q| q.bar).min().unwrap_or(fact.bar),
        };
        from = from.min(first.saturating_sub(5));
    }
    let from = from.min(n - 1);
    let mut candles = layer("candles", Mark::Candle, cid);
    candles.series = Some(bs.clone());
    let mut taken: Vec<String> = layers
        .iter()
        .filter_map(|l| l.id.clone())
        .chain(["candles".to_owned()])
        .collect();
    let mut chart = vec![candles];
    let facts: Vec<Cow<'_, Fact>> = f0.facts.iter().filter_map(|f| p.fact(f)).collect();
    for f in &facts {
        if f.series == bs {
            for e in evidence_layers(look, f, cid, &mut taken) {
                if e.mark == Mark::Band {
                    chart.insert(0, e);
                } else {
                    chart.push(e);
                }
            }
        }
    }
    layers.extend(chart);
    let i = |b: u64| usize::try_from(b).unwrap_or(0);
    #[allow(clippy::cast_precision_loss)]
    let x_scale = Scale {
        kind: ScaleKind::Time,
        domain: Some([bars[i(from)].t as f64, bars[i(n - 1)].t as f64]),
        id: Some("x".to_owned()),
    };
    Ok(View {
        id: id.to_owned(),
        title: None,
        display_tz: None,
        theme: None,
        x: x_scale,
        y: Vec::new(),
        panels,
        layers,
        emphasis: Vec::new(),
    })
}

/// The hint line under the open capsule.
#[must_use]
pub fn hint(l: Language) -> &'static str {
    match l {
        Language::Tr => "Bir soruya dokunun: ayrıntısı açılır. Grafik için grafiğe dokunun.",
        Language::En => "Tap a question for its detail. Tap the chart to open it.",
    }
}

/// A tap on `nobra`'s capsule depth: on a question card its detail opens (or closes, if
/// it was the one open); on the evidence chart the chart depth opens; elsewhere (the title
/// row) nothing. A glossary term is handled before (its explanation opens).
#[must_use]
pub fn tap(view: &View, hit: &HitInfo, ask: Option<&str>) -> OpenTap {
    let Some(name) = hit
        .panel
        .and_then(|id| view.panels.iter().find(|p| p.id == Some(id)))
        .and_then(|p| p.name.as_deref())
    else {
        return OpenTap::Nothing;
    };
    if name == CHART_PANEL {
        return OpenTap::Open(OpenDepth::Chart);
    }
    match name.strip_prefix(PANEL) {
        Some(k) if ask == Some(k) => OpenTap::Ask(None),
        Some(k) => OpenTap::Ask(Some(k.to_owned())),
        None => OpenTap::Nothing,
    }
}

/// One question card as text: question, answer, detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskRow {
    /// Question key (`what`, `who`, `before`, `why`, `basis`).
    pub key: &'static str,
    /// The question.
    pub q: String,
    /// The one-line answer.
    pub a: String,
    /// The detail (opened on touch).
    pub d: String,
}

/// The question cards of a reader **without a scene** (no data of the instrument, A32):
/// the same questions and texts as the scene, from the feed item (its voice, validity,
/// trust path and track record), with "Why can't I see numbers?" and no digit beyond the
/// instrument's own label (K33).
#[must_use]
pub fn text_rows(item: &CapsuleItem, l: Language, check: Option<Check>) -> Vec<AskRow> {
    let x = Facts::of_item(item);
    ASKS.iter()
        .filter_map(|k| {
            let c = if *k == "who" { check } else { None };
            Some(AskRow {
                key: k,
                q: text(&x, k, "q", l, c)?,
                a: text(&x, k, "a", l, c)?,
                d: text(&x, k, "d", l, c)?,
            })
        })
        .collect()
}
