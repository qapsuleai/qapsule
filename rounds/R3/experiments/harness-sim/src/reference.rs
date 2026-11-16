//! **Başvuru** stratejileri (SPEC "Ortak başvuru"): ölçülmeyen iki strateji yerine
//! kullanılır. Aday değildir; bilerek en sade biçimdedir ve hiçbir alternatif görülmeden
//! yazılmıştır.
//!
//! - [`RefTheme`]: aynı enstrüman + seri + tür + yön → o açık hikâyeye katıl (en eskisi);
//!   yoksa yeni hikâye (tema anahtarı `<tür>.<yön>`). Hikâyesi olmayan bir `exit` olgusu
//!   yok sayılır (bitmiş bir durumdan hikâye doğmaz).
//! - [`RefOutcome`]: ufuk zaman türüne göre (olay 10, durum 20, beklenti 10 bar); simetrik
//!   bariyer: doğum barındaki ATR(14) (Wilder) `k = 1` katı. Kapanış hikâye yönünde
//!   `≥ k·ATR` giderse `realized`, ters yönde `≥ k·ATR` giderse `invalidated` (önce hangisi
//!   olursa). Nötr hikâye: `|hareket| ≥ k·ATR` → `realized`, geçersiz olmaz (yalnız süresi
//!   dolar). Doğum barı 13'ten küçükse (ATR yok) hüküm yok.
//! - [`RefRanker`]: aile sayısı ≥ 2 → kademe 1; değilse güç ≥ 0,5 → kademe 2; değilse 3.
//!   Önem = aile sayısı + güç.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qp_capsule::{OpenStory, OutcomeRule, RankCtx, Ranker, ThemeDecision, ThemeRule, Verdict};
use qp_schema::capsule::{Direction, Edge, MeaningKind, TimeKind};
use qp_schema::{Bar, Fact};

use crate::common::dir_name;

/// Reference theme rule (see the module docs).
#[derive(Debug, Clone, Copy, Default)]
pub struct RefTheme;

impl ThemeRule for RefTheme {
    fn name(&self) -> &'static str {
        "reference.kind_direction"
    }

    fn decide(&self, fact: &Fact, open: &[&OpenStory]) -> ThemeDecision {
        let Some(m) = fact.meaning.as_ref() else {
            return ThemeDecision::Ignore;
        };
        let hit = open.iter().find(|s| {
            s.story.series == fact.series
                && s.story.kind == m.kind
                && s.story.direction == m.direction
        });
        match hit {
            Some(s) => ThemeDecision::Join(s.id.clone()),
            None if m.edge == Edge::Exit => ThemeDecision::Ignore,
            None => ThemeDecision::New {
                theme: format!("{}.{}", m.kind.name(), dir_name(m.direction)),
            },
        }
    }
}

/// Wilder ATR(`p`) at the last bar of `bars` (first value at bar `p-1` = mean of the first
/// `p` true ranges, `tr0 = h - l`). `None` with fewer than `p` bars.
#[must_use]
pub fn wilder_atr(bars: &[Bar], p: usize) -> Option<f64> {
    if p == 0 || bars.len() < p {
        return None;
    }
    let tr = |i: usize| -> f64 {
        let b = &bars[i];
        if i == 0 {
            b.h - b.l
        } else {
            let pc = bars[i - 1].c;
            (b.h - b.l).max((b.h - pc).abs()).max((b.l - pc).abs())
        }
    };
    let mut sum = 0.0;
    for i in 0..p {
        sum += tr(i);
    }
    #[allow(clippy::cast_precision_loss)]
    let pf = p as f64;
    let mut atr = sum / pf;
    for i in p..bars.len() {
        atr = (atr * (pf - 1.0) + tr(i)) / pf;
    }
    Some(atr)
}

/// Reference outcome rule (see the module docs). Parameters are public so that the
/// harness can build its own ±20 % / ±10 % variants for the self-test.
#[derive(Debug, Clone)]
pub struct RefOutcome {
    /// Horizon of event stories (bars).
    pub h_event: u32,
    /// Horizon of state stories.
    pub h_state: u32,
    /// Horizon of expectation stories.
    pub h_expect: u32,
    /// Barrier as a multiple of ATR(14) at birth.
    pub k: f64,
    /// Name.
    pub label: &'static str,
    atr_cache: RefCell<BTreeMap<(String, u64), Option<f64>>>,
}

impl Default for RefOutcome {
    fn default() -> Self {
        Self::new(10, 20, 10, 1.0, "reference.atr_barrier")
    }
}

impl RefOutcome {
    /// With explicit parameters.
    #[must_use]
    pub fn new(h_event: u32, h_state: u32, h_expect: u32, k: f64, label: &'static str) -> Self {
        Self {
            h_event,
            h_state,
            h_expect,
            k,
            label,
            atr_cache: RefCell::new(BTreeMap::new()),
        }
    }

    /// The reference's own perturbations (horizon ±20 %, barrier ±10 %).
    #[must_use]
    pub fn perturbations() -> Vec<(String, Box<dyn OutcomeRule>)> {
        vec![
            (
                "h-20%".to_owned(),
                Box::new(Self::new(8, 16, 8, 1.0, "reference.h-20")) as Box<dyn OutcomeRule>,
            ),
            (
                "h+20%".to_owned(),
                Box::new(Self::new(12, 24, 12, 1.0, "reference.h+20")),
            ),
            (
                "k-10%".to_owned(),
                Box::new(Self::new(10, 20, 10, 0.9, "reference.k-10")),
            ),
            (
                "k+10%".to_owned(),
                Box::new(Self::new(10, 20, 10, 1.1, "reference.k+10")),
            ),
        ]
    }
}

impl OutcomeRule for RefOutcome {
    fn name(&self) -> &'static str {
        self.label
    }

    fn horizon(&self, _kind: MeaningKind, time: TimeKind) -> u32 {
        match time {
            TimeKind::Event => self.h_event,
            TimeKind::State => self.h_state,
            TimeKind::Expectation => self.h_expect,
        }
    }

    fn judge(&self, s: &OpenStory, bars: &[Bar], now: u64) -> Option<(Verdict, &'static str)> {
        let born = usize::try_from(s.story.born_bar).ok()?;
        let now_i = usize::try_from(now).ok()?;
        if now_i <= born || bars.len() <= now_i {
            return None;
        }
        let key = (s.id.clone(), s.story.born_bar);
        let atr = *self
            .atr_cache
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| wilder_atr(&bars[..=born], 14));
        let atr = atr?;
        if atr <= 0.0 {
            return None;
        }
        let thr = self.k * atr;
        let mv = bars[now_i].c - bars[born].c;
        match s.story.direction {
            Direction::Up if mv >= thr => Some((Verdict::Realized, "ref.target")),
            Direction::Up if mv <= -thr => Some((Verdict::Invalidated, "ref.stop")),
            Direction::Down if mv <= -thr => Some((Verdict::Realized, "ref.target")),
            Direction::Down if mv >= thr => Some((Verdict::Invalidated, "ref.stop")),
            Direction::Neutral if mv.abs() >= thr => Some((Verdict::Realized, "ref.move")),
            _ => None,
        }
    }
}

/// Reference ranker (see the module docs).
#[derive(Debug, Clone, Copy, Default)]
pub struct RefRanker;

impl Ranker for RefRanker {
    fn name(&self) -> &'static str {
        "reference.families"
    }

    fn rank(&self, s: &OpenStory, _ctx: &RankCtx) -> (f64, u8) {
        let fam = s.story.families.len();
        let tier = if fam >= 2 {
            1
        } else if s.story.strength >= 0.5 {
            2
        } else {
            3
        };
        #[allow(clippy::cast_precision_loss)]
        (fam as f64 + s.story.strength, tier)
    }
}
