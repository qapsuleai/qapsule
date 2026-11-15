//! R3/M6 trust path candidate **"coarse"**: coarse relation + persistence.
//!
//! The server sees the raw licensed flow; the unentitled user receives only a
//! quantized, qualitative *state* of the bar's institutional net flow relative to what
//! is usual for the symbol, and only when that state changes.
//!
//! Design (parameters in [`Config`], reasons and measurements in `NOTES.md`):
//!
//! - **Yardstick `U` ("usual"), separated from the reported quantity.** By default `U`
//!   is the median, over the last `window` observations (current bar excluded), of the
//!   largest broker's two-sided gross activity `min(buy, sell)` — the turnover it did
//!   *not* net. No `net` value ever enters `U`, so changing one bar's `net` cannot move
//!   any later yardstick and cannot pin that value through later messages. `U` is
//!   recomputed only every `refresh` observations and quantized **down** to powers of 2
//!   (or √2), so it is coarse also in the gross values it rests on.
//!   ([`Yardstick::NetMedian`], the median of past `|net|`, is kept for comparison.)
//! - **State** of the bar: `x = |net| / U` and the sign of `net`. Three states:
//!   `Usual`, `Heavy(buy)`, `Heavy(sell)`. Enter `Heavy` when `x ≥ enter`; leave it only
//!   when `x < exit` or the sign turns (hysteresis).
//! - **Persistence:** a message only when the state changes (entry, reversal, exit),
//!   plus one "persisting" note after `sustain` heavy bars (no new information: the
//!   absence of an exit already says it) and a "renewed" note on a heavy bar with
//!   `x ≥ renew`.
//! - **Delay:** every message is shown `delay` bars after its bar (published after the
//!   close, not in real time).
//! - **Wording:** TR + EN, no digits, says it is server-verified, what it rests on and
//!   which entitlement shows the figures (K34). Attachments are constant strings.
//!
//! Server facts (`flow.institutional`) are deliberately **not** used as triggers: the
//! detector's threshold `2·avg5` moves continuously with each of the last five `|net|`,
//! so echoing it would pin earlier values (the `echo` control's worst case).
//!
//! Deterministic: only IEEE basic operations (no transcendental functions), no clock,
//! no randomness, no state shared across instances.

use std::collections::{BTreeMap, VecDeque};

use qp_capsule::{FlowObservation, QualMessage, TrustPath};
use qp_schema::capsule::Trust;
use qp_schema::Fact;

/// What the "usual" level is measured on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Yardstick {
    /// Largest broker's two-sided gross `min(buy, sell)` (default; independent of `net`).
    Gross,
    /// Past `|net|` (comparison; a past `net` moves later yardsticks).
    NetMedian,
}

/// Parameters of the coarse trust path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// What `U` is measured on.
    pub yardstick: Yardstick,
    /// Observations in the yardstick window.
    pub window: usize,
    /// Observations needed before the first yardstick.
    pub min_obs: usize,
    /// The yardstick is recomputed every `refresh` observations.
    pub refresh: usize,
    /// Yardstick grid: `true` = steps of √2, `false` = steps of 2.
    pub fine_grid: bool,
    /// Enter a heavy state when `|net| / U ≥ enter`.
    pub enter: f64,
    /// Leave it when `|net| / U < exit` (or the sign turns).
    pub exit: f64,
    /// In a heavy state, a bar with `|net| / U ≥ renew` (same sign) is re-stated once
    /// ("renewed"); `None` = never.
    pub renew: Option<f64>,
    /// Bars in a heavy state before the single "persisting" note (0 = never).
    pub sustain: u32,
    /// Display delay in bars.
    pub delay: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            yardstick: Yardstick::Gross,
            window: 40,
            min_obs: 10,
            refresh: 20,
            fine_grid: false,
            enter: 0.45,
            exit: 0.15,
            renew: Some(0.9),
            sustain: 3,
            delay: 1,
        }
    }
}

/// Coarse state of the institutional flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Usual,
    Heavy { buy: bool, run: u32 },
}

/// What a message says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Say {
    Enter { buy: bool },
    Reverse { buy: bool },
    Persist { buy: bool },
    Renew { buy: bool },
    Exit,
}

/// The coarse trust path (one instance per instrument stream).
#[derive(Debug, Clone)]
pub struct CoarseTrust {
    cfg: Config,
    samples: VecDeque<f64>,
    seen: usize,
    usual: Option<f64>,
    state: State,
}

impl CoarseTrust {
    /// A fresh path with `cfg`.
    #[must_use]
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            samples: VecDeque::with_capacity(cfg.window + 1),
            seen: 0,
            usual: None,
            state: State::Usual,
        }
    }

    fn decide(&mut self, net: f64) -> Option<Say> {
        let u = self.usual?;
        let x = net.abs() / u;
        let buy = net > 0.0;
        let strong = net != 0.0 && x >= self.cfg.enter;
        match self.state {
            State::Usual => {
                if strong {
                    self.state = State::Heavy { buy, run: 1 };
                    Some(Say::Enter { buy })
                } else {
                    None
                }
            }
            State::Heavy { buy: d, run } => {
                if strong && buy != d {
                    self.state = State::Heavy { buy, run: 1 };
                    Some(Say::Reverse { buy })
                } else if net == 0.0 || buy != d || x < self.cfg.exit {
                    self.state = State::Usual;
                    Some(Say::Exit)
                } else {
                    let run = run.saturating_add(1);
                    self.state = State::Heavy { buy: d, run };
                    if self.cfg.renew.is_some_and(|r| x >= r) {
                        Some(Say::Renew { buy: d })
                    } else {
                        (run == self.cfg.sustain).then_some(Say::Persist { buy: d })
                    }
                }
            }
        }
    }

    fn commit(&mut self, sample: Option<f64>) {
        let Some(s) = sample else {
            return;
        };
        self.samples.push_back(s);
        if self.samples.len() > self.cfg.window {
            self.samples.pop_front();
        }
        self.seen += 1;
        let due = self.usual.is_none() || self.seen.is_multiple_of(self.cfg.refresh.max(1));
        if due && self.samples.len() >= self.cfg.min_obs.max(1) {
            self.usual = quantize_down(median(&self.samples), self.cfg.fine_grid);
        }
    }
}

/// Median of the values (mean of the two middle ones for an even count).
fn median(v: &VecDeque<f64>) -> f64 {
    let mut s: Vec<f64> = v.iter().copied().collect();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        s[n / 2]
    } else {
        f64::midpoint(s[n / 2 - 1], s[n / 2])
    }
}

/// Largest power of 2 (or of √2 when `fine`) `≤ m`, by exact doublings/halvings and one
/// multiply by √2 (IEEE basic operations only). `None` for `m ≤ 0` or non-finite `m`.
fn quantize_down(m: f64, fine: bool) -> Option<f64> {
    if !(m.is_finite() && m > 0.0) {
        return None;
    }
    let mut p = 1.0_f64;
    while p * 2.0 <= m {
        p *= 2.0;
    }
    while p > m {
        p /= 2.0;
    }
    if fine && p * std::f64::consts::SQRT_2 <= m {
        p *= std::f64::consts::SQRT_2;
    }
    Some(p)
}

const BASIS_TR: &str =
    "Dayanak: lisanslı kurumsal akış verisi; rakamları görmek için kurumsal akış yetkisi gerekir.";
const BASIS_EN: &str =
    "Based on licensed institutional flow data; the figures require the institutional flow entitlement.";

fn words(say: Say) -> (Vec<&'static str>, &'static str, &'static str) {
    match say {
        Say::Enter { buy: true } => (
            vec!["flow_well_above_usual", "net_buy"],
            "Sunucu doğruladı: bu mumda kurumsal net alış, bu sembolde olağan kurumsal işlem hacmine göre belirgin biçimde yüksek.",
            "Server-verified: institutional net buying on this bar is well above usual for the normal institutional trading activity in this symbol.",
        ),
        Say::Enter { buy: false } => (
            vec!["flow_well_above_usual", "net_sell"],
            "Sunucu doğruladı: bu mumda kurumsal net satış, bu sembolde olağan kurumsal işlem hacmine göre belirgin biçimde yüksek.",
            "Server-verified: institutional net selling on this bar is well above usual for the normal institutional trading activity in this symbol.",
        ),
        Say::Reverse { buy: true } => (
            vec!["flow_well_above_usual", "net_buy", "reversal"],
            "Sunucu doğruladı: kurumsal akış yön değiştirdi; bu mumda net alış, olağan kurumsal işlem hacmine göre belirgin biçimde yüksek.",
            "Server-verified: institutional flow reversed; net buying on this bar is well above usual for the normal institutional trading activity.",
        ),
        Say::Reverse { buy: false } => (
            vec!["flow_well_above_usual", "net_sell", "reversal"],
            "Sunucu doğruladı: kurumsal akış yön değiştirdi; bu mumda net satış, olağan kurumsal işlem hacmine göre belirgin biçimde yüksek.",
            "Server-verified: institutional flow reversed; net selling on this bar is well above usual for the normal institutional trading activity.",
        ),
        Say::Persist { buy: true } => (
            vec!["flow_above_usual", "net_buy", "persisting"],
            "Sunucu doğruladı: olağanın üzerindeki kurumsal net alış birkaç mumdur sürüyor.",
            "Server-verified: above-usual institutional net buying has persisted for several bars.",
        ),
        Say::Persist { buy: false } => (
            vec!["flow_above_usual", "net_sell", "persisting"],
            "Sunucu doğruladı: olağanın üzerindeki kurumsal net satış birkaç mumdur sürüyor.",
            "Server-verified: above-usual institutional net selling has persisted for several bars.",
        ),
        Say::Renew { buy: true } => (
            vec!["flow_well_above_usual", "net_buy", "renewed"],
            "Sunucu doğruladı: kurumsal net alış bu mumda yeniden olağanın belirgin biçimde üzerinde.",
            "Server-verified: institutional net buying is again well above usual on this bar.",
        ),
        Say::Renew { buy: false } => (
            vec!["flow_well_above_usual", "net_sell", "renewed"],
            "Sunucu doğruladı: kurumsal net satış bu mumda yeniden olağanın belirgin biçimde üzerinde.",
            "Server-verified: institutional net selling is again well above usual on this bar.",
        ),
        Say::Exit => (
            vec!["flow_back_to_usual"],
            "Sunucu doğruladı: kurumsal net akış bu sembolde yeniden olağan ölçülerde.",
            "Server-verified: institutional net flow in this symbol is back to usual proportions.",
        ),
    }
}

impl TrustPath for CoarseTrust {
    fn name(&self) -> &'static str {
        "coarse"
    }

    fn observe(&mut self, obs: &FlowObservation, _facts: &[Fact]) -> Vec<QualMessage> {
        let finite = |k: &str| obs.values.get(k).copied().filter(|v| v.is_finite());
        let Some(net) = finite("net") else {
            return Vec::new();
        };
        let sample = match self.cfg.yardstick {
            Yardstick::Gross => match (finite("buy"), finite("sell")) {
                (Some(b), Some(s)) => Some(b.min(s).max(0.0)),
                _ => None,
            },
            Yardstick::NetMedian => Some(net.abs()),
        };
        let say = self.decide(net);
        self.commit(sample);
        let Some(say) = say else {
            return Vec::new();
        };
        let (codes, tr, en) = words(say);
        vec![QualMessage {
            t: obs.t,
            bar: obs.bar,
            instrument: obs.instrument.clone(),
            codes: codes.into_iter().map(str::to_owned).collect(),
            tr: format!("{tr} {BASIS_TR}"),
            en: format!("{en} {BASIS_EN}"),
            trust: Trust::Server,
            shown_bar: obs.bar + self.cfg.delay,
            attachments: BTreeMap::from([
                ("basis".to_owned(), "licensed_institutional_flow".to_owned()),
                ("entitlement".to_owned(), "institutional_flow".to_owned()),
            ]),
        }]
    }
}

/// Entry point (fresh state per call) with the chosen parameters.
#[must_use]
pub fn trust_path() -> Box<dyn TrustPath> {
    Box::new(CoarseTrust::new(Config::default()))
}

/// Entry point with explicit parameters (experiments).
#[must_use]
pub fn trust_path_with(cfg: Config) -> Box<dyn TrustPath> {
    Box::new(CoarseTrust::new(cfg))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One bar: `buy − sell = net`, `min(buy, sell) = gross`.
    fn obs(bar: u64, net: f64, gross: f64) -> FlowObservation {
        FlowObservation {
            t: 1_000 * i64::try_from(bar).unwrap_or(0),
            bar,
            instrument: "TEST".to_owned(),
            values: BTreeMap::from([
                ("net".to_owned(), net),
                ("buy".to_owned(), gross + net.max(0.0)),
                ("sell".to_owned(), gross + (-net).max(0.0)),
            ]),
        }
    }

    /// A deterministic, varied flow (`net`, `gross`) with spikes of both signs.
    fn series(n: u64) -> Vec<(f64, f64)> {
        let mut s = 0x9E37_79B9_7F4A_7C15_u64;
        let mut u01 = move || {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            #[allow(clippy::cast_precision_loss)]
            let u = (s >> 11) as f64 / (1u64 << 53) as f64;
            u
        };
        (0..n)
            .map(|i| {
                let gross = 1_000.0 + 3_000.0 * u01();
                let base = 600.0 * (u01() - 0.5);
                let net = if i % 23 == 7 {
                    base + 2_500.0
                } else if i % 31 == 11 {
                    base - 2_500.0
                } else {
                    base
                };
                (net, gross)
            })
            .collect()
    }

    fn run(cfg: Config, bars: &[(f64, f64)]) -> Vec<QualMessage> {
        let mut tp = CoarseTrust::new(cfg);
        bars.iter()
            .enumerate()
            .flat_map(|(i, &(n, g))| tp.observe(&obs(i as u64, n, g), &[]))
            .collect()
    }

    #[test]
    fn emits_messages_on_spikes() {
        let msgs = run(Config::default(), &series(400));
        assert!(msgs.iter().any(|m| m.codes.contains(&"net_buy".to_owned())));
        assert!(msgs
            .iter()
            .any(|m| m.codes.contains(&"net_sell".to_owned())));
        assert!(msgs
            .iter()
            .any(|m| m.codes.contains(&"flow_back_to_usual".to_owned())));
    }

    #[test]
    fn no_digits_server_trust_honest_wording() {
        let msgs = run(Config::default(), &series(400));
        assert!(!msgs.is_empty());
        for m in &msgs {
            assert_eq!(m.trust, Trust::Server);
            for s in [&m.tr, &m.en]
                .into_iter()
                .chain(m.codes.iter())
                .chain(m.attachments.keys())
                .chain(m.attachments.values())
            {
                assert!(!s.chars().any(char::is_numeric), "digit in {s:?}");
            }
            let tr = m.tr.to_lowercase();
            assert!(tr.contains("sunucu") && tr.contains("doğrula"), "{tr}");
            assert!(tr.contains("lisans") && tr.contains("yetki"), "{tr}");
            let en = m.en.to_lowercase();
            assert!(en.contains("server") && en.contains("verif"), "{en}");
            assert!(
                en.contains("licensed") && en.contains("entitlement"),
                "{en}"
            );
            assert_eq!(m.shown_bar, m.bar + 1);
        }
    }

    #[test]
    fn every_wording_variant_is_clean() {
        for say in [
            Say::Enter { buy: true },
            Say::Enter { buy: false },
            Say::Reverse { buy: true },
            Say::Reverse { buy: false },
            Say::Persist { buy: true },
            Say::Persist { buy: false },
            Say::Renew { buy: true },
            Say::Renew { buy: false },
            Say::Exit,
        ] {
            let (codes, tr, en) = words(say);
            for s in codes.iter().copied().chain([tr, en, BASIS_TR, BASIS_EN]) {
                assert!(!s.chars().any(char::is_numeric), "digit in {s:?}");
            }
            assert!(tr.starts_with("Sunucu doğruladı:"), "{tr}");
            assert!(en.starts_with("Server-verified:"), "{en}");
        }
    }

    #[test]
    fn deterministic_and_fresh_per_call() {
        let bars = series(500);
        let a = run(Config::default(), &bars);
        let b = run(Config::default(), &bars);
        assert_eq!(a, b);
        // The factory gives independent instances, equal to the default config.
        let mut p = trust_path();
        let mut q = trust_path();
        let ma: Vec<_> = bars
            .iter()
            .enumerate()
            .flat_map(|(i, &(n, g))| p.observe(&obs(i as u64, n, g), &[]))
            .collect();
        let mb: Vec<_> = bars
            .iter()
            .enumerate()
            .flat_map(|(i, &(n, g))| q.observe(&obs(i as u64, n, g), &[]))
            .collect();
        assert_eq!(ma, mb);
        assert_eq!(ma, a);
        assert_eq!(p.name(), "coarse");
    }

    #[test]
    fn only_state_changes_are_emitted() {
        // Usual flow (gross 1000 → U = 512), then a sustained heavy buy, then back.
        let cfg = Config {
            renew: None,
            ..Config::default()
        };
        let mut bars = vec![(100.0, 1_000.0); 30];
        bars.extend([(300.0, 1_000.0); 6]);
        bars.extend([(50.0, 1_000.0); 5]);
        let msgs = run(cfg, &bars);
        let codes: Vec<Vec<String>> = msgs.iter().map(|m| m.codes.clone()).collect();
        assert_eq!(msgs.len(), 3, "{codes:?}");
        assert_eq!(msgs[0].bar, 30);
        assert!(codes[0].contains(&"flow_well_above_usual".to_owned()));
        assert_eq!(msgs[1].bar, 32);
        assert!(codes[1].contains(&"persisting".to_owned()));
        assert_eq!(msgs[2].bar, 36);
        assert_eq!(codes[2], vec!["flow_back_to_usual".to_owned()]);
    }

    #[test]
    fn renew_restates_a_strong_bar_inside_a_heavy_state() {
        let mut bars = vec![(100.0, 1_000.0); 30];
        bars.extend([(300.0, 1_000.0), (200.0, 1_000.0), (600.0, 1_000.0)]);
        let msgs = run(Config::default(), &bars);
        let codes: Vec<Vec<String>> = msgs.iter().map(|m| m.codes.clone()).collect();
        assert_eq!(msgs.len(), 2, "{codes:?}");
        assert_eq!(msgs[1].bar, 32);
        assert!(codes[1].contains(&"renewed".to_owned()));
    }

    #[test]
    fn a_past_net_reaches_later_bars_only_through_its_own_state() {
        // Gross yardstick: varying one bar's `net` (its gross fixed) never moves the
        // yardstick, and later messages depend on it only through the coarse state it
        // leaves behind (which its own message already discloses).
        let base = series(300);
        let b = 120;
        let trace = |bars: &[(f64, f64)]| {
            let mut tp = CoarseTrust::new(Config::default());
            let mut usual = Vec::new();
            let mut after = None;
            let mut later = Vec::new();
            for (i, &(n, g)) in bars.iter().enumerate() {
                let m = tp.observe(&obs(i as u64, n, g), &[]);
                usual.push(tp.usual);
                if i == b {
                    after = Some(tp.state);
                }
                if i > b {
                    later.extend(m);
                }
            }
            (usual, after, later)
        };
        let (u0, _, _) = trace(&base);
        let mut by_state: Vec<(Option<State>, Vec<QualMessage>)> = Vec::new();
        for k in 0..=100 {
            let mut v = base.clone();
            v[b].0 = -5_000.0 + 100.0 * f64::from(k);
            let (u, st, later) = trace(&v);
            assert_eq!(u, u0, "yardstick moved at {k}");
            match by_state.iter().find(|x| x.0 == st) {
                Some(x) => assert_eq!(x.1, later, "net at {k}"),
                None => by_state.push((st, later)),
            }
        }
        assert!(by_state.len() >= 2);
    }

    #[test]
    fn net_median_yardstick_is_monotone_in_one_value() {
        // Comparison yardstick: the usual level is a non-decreasing step function of
        // any one past `|net|`, so a later decision bounds it only from one side.
        let cfg = Config {
            yardstick: Yardstick::NetMedian,
            ..Config::default()
        };
        let base = series(60);
        let level = |bars: &[(f64, f64)]| {
            let mut tp = CoarseTrust::new(cfg);
            for (i, &(n, g)) in bars.iter().enumerate() {
                tp.observe(&obs(i as u64, n, g), &[]);
            }
            tp.usual.unwrap_or(0.0)
        };
        let mut prev = 0.0;
        for k in 0..=200 {
            let mut b = base.clone();
            b[45].0 = 20.0 * f64::from(k);
            let l = level(&b);
            assert!(l >= prev, "{l} < {prev} at {k}");
            prev = l;
        }
    }

    #[test]
    fn quantize_grid() {
        assert_eq!(quantize_down(1.0, true), Some(1.0));
        assert_eq!(quantize_down(1.5, true), Some(std::f64::consts::SQRT_2));
        assert_eq!(
            quantize_down(3.0, true),
            Some(2.0 * std::f64::consts::SQRT_2)
        );
        assert_eq!(quantize_down(2.5, true), Some(2.0));
        assert_eq!(quantize_down(3.0, false), Some(2.0));
        assert_eq!(quantize_down(1_000.0, false), Some(512.0));
        assert_eq!(quantize_down(0.3, true), Some(0.25));
        assert_eq!(quantize_down(0.0, true), None);
        assert_eq!(quantize_down(f64::NAN, true), None);
    }

    #[test]
    fn missing_value_is_silent_and_keeps_state() {
        let mut tp = CoarseTrust::new(Config::default());
        let mut o = obs(0, 1.0, 10.0);
        o.values.clear();
        assert!(tp.observe(&o, &[]).is_empty());
        assert_eq!(tp.seen, 0);
    }
}
