//! The state set `S8` (SPEC §2 table, 12 states) and each state's answer key.
//!
//! [`find`] steps the synthetic replay one step at a time and asks the authority (a bare
//! protocol client, no audience filter) for every capsule; the first capsule that matches a
//! state's rule is recorded with the step and the capsule id. The **key** of a state (what
//! happened, when, whether it still holds, how reliable, advice: none) comes from the
//! engine's own record of that capsule (`CapsuleItem`: kind, direction, stage, validity,
//! `ledger_at_birth`, latest transition bar) and the series' bars, never from a face design.
//!
//! Rules of preference (written before any result): every state except #10 needs a ledger
//! at birth with `n ≥ 5` (otherwise every early face would read "few samples"); #10 needs
//! `n < 5`; a capsule fills at most one state; the earliest step wins.
//!
//! **When** (S2) is calendar time (SPEC: "süre ifadesi takvime göre"): the local day of the
//! capsule's latest transition bar against the local day of the replay clock — 0 today,
//! 1 yesterday, 2–6 a few days, ≥ 7 more than a week; on an intraday series the same day
//! splits at 3 hours after the bar's end ("a few hours ago" ≤ 3 h < "today"). A daily
//! capsule is born when its bar is known closed, i.e. at the next session's first step, so
//! the demo (daily series) never has a "today" event: #1 and the 60-minute state #11 come
//! from the hourly world, and so does any other state the demo does not show (#12: the
//! demo's engine confirms no downward reversal).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use qp_schema::capsule::{Direction, MeaningKind, Stage, TimeKind, Trust, Validity};
use qp_schema::protocol::CapsuleItem;

use crate::world::{Kind, Raw, World};

/// Number of states.
pub const N: usize = 12;

/// The states (SPEC §2), TR as in the table.
pub const TITLES: [&str; N] = [
    "Olay, yukarı kırılım, bugün, geçerli",
    "Olay, aşağı kırılım, 3 işlem günü önce (hafta sonu aşan), geçerli",
    "Durum, aşırılık yukarı, artık geçerli değil (no_longer), hikâye açık",
    "Durum, sıkışma, sürüyor",
    "Kapandı: gerçekleşti",
    "Kapandı: tutmadı (geçersiz)",
    "Kapandı: süresi doldu",
    "Geri çekildi",
    "Lisanslı akış, yetkisiz okuyucu (rakamsız, sunucu doğruladı)",
    "Az örnekli karne (n < 5)",
    "60 dakikalık periyot, \"2 saat önce\"",
    "Dönüş aşağı, teyit edilmiş (iki aile), dün",
];

/// S1 options (direction).
pub const S1: [&str; 4] = ["yukarı", "aşağı", "belirsiz", "anlamadım"];
/// S1b options (kind, plain words).
pub const S1B: [&str; 7] = [
    "fiyat önemli bir seviyeyi geçti",
    "yön değişiyor",
    "hareket sürüyor",
    "fiyat sıkıştı, hareketsiz",
    "aşırı gitti",
    "büyük alım ya da satım var",
    "anlamadım",
];
/// S2 options (when).
pub const S2: [&str; 6] = [
    "bugün",
    "dün",
    "birkaç gün önce",
    "bir haftadan önce",
    "birkaç saat önce",
    "bilmiyorum",
];
/// S3 options (validity).
pub const S3: [&str; 6] = [
    "evet, sürüyor",
    "hayır, artık geçerli değil",
    "kapandı: gerçekleşti",
    "kapandı: tutmadı",
    "kapandı: süresi doldu ya da geri çekildi",
    "bilmiyorum",
];
/// S4 options (reliability).
pub const S4: [&str; 5] = [
    "geçmişte çoğunlukla tuttu",
    "yarı yarıya",
    "çoğunlukla tutmadı",
    "az örnek var, bilinmiyor",
    "hiçbir bilgi göremedim",
];
/// S5 options (advice).
pub const S5: [&str; 5] = [
    "al",
    "sat",
    "bekle",
    "hiçbir şey, yalnız bilgi veriyor",
    "bilmiyorum",
];

/// The plain-word kind of a meaning (S1b; the harness's table, SPEC §2).
#[must_use]
pub fn kind_plain(k: MeaningKind) -> &'static str {
    match k {
        MeaningKind::Breakout => S1B[0],
        MeaningKind::Reversal => S1B[1],
        MeaningKind::Continuation => S1B[2],
        MeaningKind::Squeeze => S1B[3],
        MeaningKind::Extreme => S1B[4],
        MeaningKind::Flow => S1B[5],
    }
}

/// The answer key of one face (SPEC §2 "anahtar").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    /// S1: direction.
    pub s1: String,
    /// S1b: kind (plain).
    pub s1b: String,
    /// S2: when (bucket of the latest transition).
    pub s2: String,
    /// S3: validity and stage.
    pub s3: String,
    /// S4: reliability bucket (ledger at birth).
    pub s4: String,
    /// S5: advice (always none).
    pub s5: String,
}

/// Bar facts of a capsule's series now.
#[derive(Debug, Clone, Copy)]
pub struct Bars {
    /// The replay clock (ms).
    pub clock: i64,
    /// Index of the last bar (now).
    pub now_bar: u64,
    /// Time of the last bar.
    pub now_t: i64,
    /// Time of the capsule's latest transition bar.
    pub last_t: i64,
    /// Interval of the series (ms).
    pub interval_ms: i64,
}

const DAY_MS: i64 = 86_400_000;
/// Local offset of the synthetic and XIST calendars (UTC+3).
const LOCAL_MS: i64 = 3 * 3_600_000;

fn local_day(t: i64) -> i64 {
    (t + LOCAL_MS).div_euclid(DAY_MS)
}

/// Hours after an intraday bar's end up to which the same day is "a few hours ago".
pub const FEW_HOURS: i64 = 3;

/// Calendar days between the latest transition bar and the clock (local days).
#[must_use]
pub fn days_ago(b: &Bars) -> i64 {
    local_day(b.clock) - local_day(b.last_t)
}

/// Hours from the end of the latest transition bar to the clock.
#[must_use]
pub fn hours_ago(b: &Bars) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let h = (b.clock - (b.last_t + b.interval_ms)) as f64 / 3_600_000.0;
    h
}

/// The S2 bucket (module docs).
#[must_use]
pub fn when_bucket(_item: &CapsuleItem, b: &Bars) -> &'static str {
    match days_ago(b) {
        d if d <= 0 => {
            if b.interval_ms < DAY_MS && hours_ago(b) <= FEW_HOURS as f64 {
                S2[4]
            } else {
                S2[0]
            }
        }
        1 => S2[1],
        2..=6 => S2[2],
        _ => S2[3],
    }
}

/// The S3 answer.
#[must_use]
pub fn validity_answer(item: &CapsuleItem) -> &'static str {
    match (item.validity, item.stage) {
        (Validity::Valid, _) => S3[0],
        (Validity::NoLonger, _) => S3[1],
        (Validity::Closed, Stage::Realized) => S3[2],
        (Validity::Closed, Stage::Invalidated) => S3[3],
        (Validity::Closed, _) => S3[4],
    }
}

/// The S4 answer (ledger at birth: ≥ 60 %, 40–60 %, < 40 %, n < 5).
#[must_use]
pub fn trust_answer(item: &CapsuleItem) -> &'static str {
    let l = &item.ledger_at_birth;
    if l.n < 5 {
        return S4[3];
    }
    let r = f64::from(l.realized) / f64::from(l.n);
    if r >= 0.6 {
        S4[0]
    } else if r >= 0.4 {
        S4[1]
    } else {
        S4[2]
    }
}

/// The key of `item`.
#[must_use]
pub fn key_of(item: &CapsuleItem, b: &Bars) -> Key {
    Key {
        s1: match item.direction {
            Direction::Up => S1[0],
            Direction::Down => S1[1],
            Direction::Neutral => S1[2],
        }
        .to_owned(),
        s1b: kind_plain(item.kind).to_owned(),
        s2: when_bucket(item, b).to_owned(),
        s3: validity_answer(item).to_owned(),
        s4: trust_answer(item).to_owned(),
        s5: S5[3].to_owned(),
    }
}

/// Which states `item` matches (1-based), by the rules (module docs).
#[must_use]
pub fn matches(item: &CapsuleItem, b: &Bars, principal: &str, _kind: Kind) -> Vec<usize> {
    let mut out = Vec::new();
    let n_ok = item.ledger_at_birth.n >= 5;
    let daily = b.interval_ms >= DAY_MS;
    let ago = b.now_bar.saturating_sub(item.last_bar);
    let when = when_bucket(item, b);
    let open_valid = item.validity == Validity::Valid;
    if principal == crate::VISITOR {
        if item.redacted && n_ok {
            out.push(9);
        }
        return out;
    }
    if !daily && when == S2[4] && ago == 2 && item.validity != Validity::Closed {
        out.push(11);
    }
    if !n_ok {
        if open_valid && daily {
            out.push(10);
        }
        return out;
    }
    let ev = item.time == TimeKind::Event;
    let st = item.time == TimeKind::State;
    if ev
        && item.kind == MeaningKind::Breakout
        && item.direction == Direction::Up
        && when == S2[0]
        && open_valid
    {
        out.push(1);
    }
    if daily
        && ev
        && item.kind == MeaningKind::Breakout
        && item.direction == Direction::Down
        && ago == 3
        && days_ago(b) >= 5
        && open_valid
    {
        out.push(2);
    }
    if st
        && item.kind == MeaningKind::Extreme
        && item.direction == Direction::Up
        && item.validity == Validity::NoLonger
    {
        out.push(3);
    }
    if st && item.kind == MeaningKind::Squeeze && open_valid {
        out.push(4);
    }
    if daily {
        match item.stage {
            Stage::Realized => out.push(5),
            Stage::Invalidated => out.push(6),
            Stage::Expired => out.push(7),
            Stage::Retracted => out.push(8),
            _ => {}
        }
    }
    if item.kind == MeaningKind::Reversal
        && item.direction == Direction::Down
        && item.confirmations >= 2
        && when == S2[1]
        && open_valid
    {
        out.push(12);
    }
    // Demo substitutes of #1 and #12 for M10 (its screenshots come from the demo): the
    // same kind and direction, open and valid, yesterday; #112 without the confirmation.
    if daily && when == S2[1] && open_valid {
        if ev && item.kind == MeaningKind::Breakout && item.direction == Direction::Up {
            out.push(SUB + 1);
        }
        if item.kind == MeaningKind::Reversal && item.direction == Direction::Down {
            out.push(SUB + 12);
        }
    }
    out
}

/// Offset of the demo substitutes' numbers (`101` stands in for #1, `112` for #12).
pub const SUB: usize = 100;

/// One state of `S8` as found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// State number (1-based).
    pub state: usize,
    /// SPEC title.
    pub title: String,
    /// Found in a replay.
    pub found: bool,
    /// World (`demo`, `hourly`).
    pub world: String,
    /// Replay step (steps taken) of the moment.
    pub step: u64,
    /// Replay clock of the moment (ms).
    pub t: i64,
    /// Reader principal (`arkin`, or `visitor` for #9).
    pub principal: String,
    /// Capsule id.
    pub capsule: String,
    /// Finding id.
    pub finding: String,
    /// The engine's record of the capsule at that moment, for that reader.
    pub item: Value,
    /// The answer key.
    pub key: Option<Key>,
    /// Why not found / how built.
    pub note: String,
}

/// The `S8` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S8 {
    /// Harness version.
    pub harness: String,
    /// How the states were found.
    pub method: String,
    /// The 12 states, in order.
    pub states: Vec<State>,
    /// Demo substitutes for M10 of states found only in the hourly world (`101` for #1,
    /// `112` for #12; module docs).
    #[serde(default)]
    pub substitutes: Vec<State>,
}

impl S8 {
    /// State `n` (1-based) if found.
    #[must_use]
    pub fn get(&self, n: usize) -> Option<&State> {
        self.states.iter().find(|s| s.state == n && s.found)
    }

    /// State `n` as found in the demo world: itself, else its demo substitute.
    #[must_use]
    pub fn in_demo(&self, n: usize) -> Option<&State> {
        self.get(n)
            .filter(|s| s.world == Kind::Demo.name())
            .or_else(|| {
                self.substitutes
                    .iter()
                    .find(|s| s.state == SUB + n && s.found)
            })
    }
}

fn bars_of(w: &mut World, item: &CapsuleItem) -> Option<Bars> {
    let bars = w.bars(&item.series);
    let last = bars.last()?;
    let now_bar = (bars.len() - 1) as u64;
    let lb = usize::try_from(item.last_bar).ok()?;
    let interval_ms = if bars.len() >= 2 {
        // The smallest gap between consecutive bars (the series' period).
        bars.windows(2)
            .map(|p| p[1].t - p[0].t)
            .filter(|d| *d > 0)
            .min()
            .unwrap_or(DAY_MS)
    } else {
        DAY_MS
    };
    Some(Bars {
        clock: w.now(),
        now_bar,
        now_t: last.t,
        last_t: bars.get(lb)?.t,
        interval_ms,
    })
}

fn record(st: usize, w: &mut World, principal: &str, item: &CapsuleItem, b: &Bars) -> State {
    State {
        state: st,
        title: if st > SUB {
            format!(
                "M10 demo substitute of #{}: {}",
                st - SUB,
                TITLES[st - SUB - 1]
            )
        } else {
            TITLES[st - 1].to_owned()
        },
        found: true,
        world: w.kind.name().to_owned(),
        step: w.step(),
        t: w.now(),
        principal: principal.to_owned(),
        capsule: item.id.clone(),
        finding: item.finding.clone(),
        item: serde_json::to_value(item).unwrap_or(Value::Null),
        key: Some(key_of(item, b)),
        note: String::new(),
    }
}

/// Steps world `kind` one step at a time and fills `found` with the first match of each
/// wanted state (module docs). `max_steps`: give up after so many steps.
fn search(
    kind: Kind,
    wanted: &[usize],
    found: &mut Vec<State>,
    max_steps: u64,
) -> Result<(), String> {
    let mut w = World::new(kind)?;
    let mut a = Raw::connect(&mut w, "s8", "arkin", "tr");
    let mut v = (kind == Kind::Demo).then(|| Raw::connect(&mut w, "s8v", crate::VISITOR, "tr"));
    let steps = w.steps().min(max_steps);
    let mut used: Vec<String> = found.iter().map(|s| s.capsule.clone()).collect();
    for _ in 0..steps {
        if wanted.iter().all(|n| found.iter().any(|s| s.state == *n)) {
            break;
        }
        a.replay(&mut w, json!({"kind":"step","n":1}));
        let mut readers: Vec<(&str, Vec<CapsuleItem>)> =
            vec![("arkin", a.capsules(&mut w, json!({})))];
        if let Some(v) = v.as_mut() {
            if wanted.contains(&9) && !found.iter().any(|s| s.state == 9) {
                readers.push((crate::VISITOR, v.capsules(&mut w, json!({}))));
            }
        }
        for (who, items) in readers {
            for it in &items {
                if used.contains(&it.id) {
                    continue;
                }
                let Some(b) = bars_of(&mut w, it) else {
                    continue;
                };
                for st in matches(it, &b, who, kind) {
                    if wanted.contains(&st) && !found.iter().any(|s| s.state == st) {
                        let mut r = record(st, &mut w, who, it, &b);
                        if st == 9 && it.trust != Trust::Server {
                            r.note = "trust path `replica` (no `server` capsule for an unentitled reader in the demo)".into();
                        }
                        found.push(r);
                        used.push(it.id.clone());
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Finds `S8` (module docs): the demo world for every state but #11, the hourly world for
/// #11. States not found are recorded with `found: false` and the reason.
///
/// # Errors
/// A world does not start.
pub fn find(max_steps: u64) -> Result<S8, String> {
    let mut found: Vec<State> = Vec::new();
    let demo: Vec<usize> = (1..=N)
        .filter(|n| *n != 11)
        .chain([SUB + 1, SUB + 12])
        .collect();
    search(Kind::Demo, &demo, &mut found, max_steps)?;
    // The hourly world: #11, and what the demo cannot show (module docs). #9 (unentitled
    // reader) and #10 stay in the demo (its daily ledgers).
    let rest: Vec<usize> = (1..=N)
        .filter(|n| ![9, 10].contains(n) && !found.iter().any(|s| s.state == *n))
        .collect();
    search(Kind::Hourly, &rest, &mut found, max_steps)?;
    let mut states = Vec::new();
    let substitutes: Vec<State> = found.iter().filter(|s| s.state > SUB).cloned().collect();
    for n in 1..=N {
        if let Some(s) = found.iter().find(|s| s.state == n) {
            states.push(s.clone());
        } else {
            states.push(State {
                state: n,
                title: TITLES[n - 1].to_owned(),
                found: false,
                world: String::new(),
                step: 0,
                t: 0,
                principal: String::new(),
                capsule: String::new(),
                finding: String::new(),
                item: Value::Null,
                key: None,
                note: not_found_note(n),
            });
        }
    }
    Ok(S8 {
        harness: crate::HARNESS_VERSION.to_owned(),
        method: "first capsule matching the state's rule, stepping the synthetic replay one step at a time (s8.rs; demo world = qp demo --synthetic; #11 in the hourly world)".into(),
        states,
        substitutes,
    })
}

fn not_found_note(n: usize) -> String {
    match n {
        8 => "no retracted capsule in the synthetic replay: retraction needs an upheld dispute or a data correction (§20.7), which the replay never produces; building one needs a corrected-bar provider (out of the harness's reach without a product change) — see README".into(),
        11 => "no capsule on a 60-minute series two hours old in the hourly world within the searched steps".into(),
        1 => "no upward breakout event today: in the demo (daily series) a capsule is born at the next session, so never today; none in the hourly world either".into(),
        _ => "no matching capsule within the searched steps".into(),
    }
}
