#![allow(dead_code)]
use qp_grammar::look::{Look, R4};
use qp_grammar::stories::{pavka, StoryInput};
use qp_schema::capsule::{Direction, MeaningKind, Stage, Validity};
use qp_schema::grammar::ScaleKind;
use qp_schema::protocol::{Language, StoryFace, StoryOutcome, StoryRow};
use qp_schema::scene::{PixelRect, SceneDesc, ScenePanel, XMap, YMap};
const W: u32 = 390;
const H: u32 = 640;
const BARS: u64 = 1000;

/// The chart part as the compiler would leave it: one price panel over `[from, to]`.
fn chart(from: u64, to: u64) -> SceneDesc {
    let ph = (pavka::DESIGN.price_height)(H);
    let [l, r, t, b] = R4.space.plot;
    #[allow(clippy::cast_precision_loss)]
    let slot = f64::from(W.cast_signed() - r - l) / (to - from + 1) as f64;
    SceneDesc {
        view: "present-x".to_owned(),
        width: W,
        height: ph,
        background: R4.dark.bg,
        x: XMap {
            from_bar: from,
            to_bar: to,
            left: f64::from(l),
            slot,
        },
        bars: BARS,
        lod: None,
        panels: vec![ScenePanel {
            id: 0,
            name: None,
            rect: PixelRect {
                left: l,
                top: t,
                right: W.cast_signed() - r,
                bottom: ph.cast_signed() - b,
            },
            y: YMap {
                kind: ScaleKind::Linear,
                min: 0.0,
                max: 1.0,
                top: f64::from(t),
                bottom: f64::from(ph.cast_signed() - b),
            },
            layers: Vec::new(),
        }],
        emphasis: Vec::new(),
        under: Vec::new(),
        over: Vec::new(),
    }
}

/// `n` stories over the last `span` bars: a mix of live, realized, invalidated, expired,
/// retracted, coin-flip and no-longer-valid ones (deterministic).
fn rows(n: usize, span: u64) -> Vec<StoryRow> {
    rows_life(n, span, 40)
}

/// [`rows`] with lives up to `max_life` bars.
fn rows_life(n: usize, span: u64, max_life: u64) -> Vec<StoryRow> {
    let mut v = Vec::new();
    let n64 = n as u64;
    for i in 0..n64 {
        let born = BARS - span + i * span / n64.max(1);
        let life = 5 + (i * 7) % max_life;
        let k = i % 9;
        let (closed, stage, outcome) = match k {
            0 | 5 if born + life >= BARS - 1 => (None, Stage::Born, None),
            1 | 6 => (
                Some(born + life),
                Stage::Realized,
                Some(StoryOutcome::Realized),
            ),
            2 | 7 => (
                Some(born + life),
                Stage::Invalidated,
                Some(StoryOutcome::Invalidated),
            ),
            3 => (
                Some(born + life),
                Stage::Expired,
                Some(StoryOutcome::Expired),
            ),
            4 => (
                Some(born + life),
                Stage::Retracted,
                Some(StoryOutcome::Retracted),
            ),
            _ => (
                Some(born + life),
                Stage::Realized,
                Some(StoryOutcome::Realized),
            ),
        };
        let closed = closed.map(|c: u64| c.min(BARS - 1));
        let live = closed.is_none();
        v.push(StoryRow {
            id: format!("cap.{i}"),
            finding: format!("fnd.{i}"),
            series: "px".to_owned(),
            kind: MeaningKind::Breakout,
            direction: match i % 3 {
                0 => Direction::Up,
                1 => Direction::Down,
                _ => Direction::Neutral,
            },
            tier: 1,
            born_bar: born,
            born_t: i64::try_from(born).unwrap() * 3_600_000,
            last_bar: closed.unwrap_or(born),
            last_t: 0,
            closed_bar: closed,
            closed_t: closed.map(|c| i64::try_from(c).unwrap() * 3_600_000),
            stage,
            outcome,
            validity: if live {
                if i % 2 == 0 {
                    Validity::Valid
                } else {
                    Validity::NoLonger
                }
            } else {
                Validity::Closed
            },
            in_feed: i % 4 != 3,
            hidden: None,
            coin_flip: i % 4 == 3,
            face: StoryFace {
                event: "yukarı kırılım".to_owned(),
                direction_word: "yukarı".to_owned(),
                when: "dün".to_owned(),
                moved: "dün".to_owned(),
                validity_phrase: if live { "geçerli" } else { "kapandı" }.to_owned(),
                ledger_short: "7'de 3 tuttu".to_owned(),
                few_cases: false,
                ledger_n: Some(7),
            },
            redacted: false,
        });
    }
    v
}

fn fact_of(f: &str) -> Option<String> {
    Some(format!("fact-of-{f}"))
}

fn draw(
    look: &Look,
    theme: Option<&str>,
    r: &[StoryRow],
    focus: &str,
    hide: bool,
    range: (u64, u64),
) -> SceneDesc {
    let inp = StoryInput {
        rows: r,
        focus: Some(focus),
        hide_closed: hide,
        lang: Language::Tr,
        fact_of: &fact_of,
    };
    (pavka::DESIGN.extend)(look, theme, chart(range.0, range.1), H, &inp)
}



thread_local! {
    static ROWS: Vec<StoryRow> = rows_life(533, 990, 30);
}

/// One extend at depth kind (0: 5, 1: 50, 2: 200), mode bit 4: hide, returns layer count.
#[no_mangle]
pub extern "C" fn run(kind: u32) -> u32 {
    let span = [12u64, 95, 380][(kind & 3) as usize];
    ROWS.with(|r| {
        let f = r[500].finding.clone();
        let s = draw(&R4, None, r, &f, kind & 4 != 0, (BARS - span, BARS - 1));
        s.panels[1].layers.len() as u32
    })
}

pub struct Fnd {
    pub id: String,
    pub facts: Vec<String>,
}

thread_local! {
    static FNDS: Vec<Fnd> = (0..1600)
        .map(|i| Fnd { id: format!("fnd.SYN02.breakout.up.{i}"), facts: vec![format!("fct.SYN02.breakout.up.{i}.a"), "x".to_owned()] })
        .collect();
    static ROWS2: Vec<StoryRow> = {
        let mut r = rows_life(533, 990, 30);
        for (i, x) in r.iter_mut().enumerate() { x.finding = format!("fnd.SYN02.breakout.up.{}", i * 3); }
        r
    };
}

/// borrowed variant (FirstFacts).
#[no_mangle]
pub extern "C" fn ff2(_k: u32) -> u32 {
    use std::collections::{BTreeMap, BTreeSet};
    FNDS.with(|doc| ROWS2.with(|rows| {
        let want: BTreeSet<&str> = rows.iter().map(|r| r.finding.as_str()).collect();
        let m: BTreeMap<&str, &str> = doc
            .iter()
            .filter(|f| want.contains(f.id.as_str()))
            .filter_map(|f| Some((f.id.as_str(), f.facts.first()?.as_str())))
            .collect();
        m.len() as u32
    }))
}

#[derive(Default)]
pub struct Fnv(u64);
impl std::hash::Hasher for Fnv {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, b: &[u8]) {
        let mut h = if self.0 == 0 { 0xcbf2_9ce4_8422_2325 } else { self.0 };
        for x in b { h = (h ^ u64::from(*x)).wrapping_mul(0x100_0000_01b3); }
        self.0 = h;
    }
}
type B = std::hash::BuildHasherDefault<Fnv>;

/// FNV hash variant.
#[no_mangle]
pub extern "C" fn ff3(_k: u32) -> u32 {
    use std::collections::{HashMap, HashSet};
    FNDS.with(|doc| ROWS2.with(|rows| {
        let want: HashSet<&str, B> = rows.iter().map(|r| r.finding.as_str()).collect();
        let m: HashMap<&str, &str, B> = doc
            .iter()
            .filter(|f| want.contains(f.id.as_str()))
            .filter_map(|f| Some((f.id.as_str(), f.facts.first()?.as_str())))
            .collect();
        m.len() as u32
    }))
}

/// first_facts as viewer.rs does it.
#[no_mangle]
pub extern "C" fn ff(_k: u32) -> u32 {
    use std::collections::{BTreeMap, BTreeSet};
    FNDS.with(|doc| ROWS2.with(|rows| {
        let want: BTreeSet<&str> = rows.iter().map(|r| r.finding.as_str()).collect();
        let m: BTreeMap<String, String> = doc
            .iter()
            .filter(|f| want.contains(f.id.as_str()))
            .filter_map(|f| Some((f.id.clone(), f.facts.first()?.clone())))
            .collect();
        m.len() as u32
    }))
}
