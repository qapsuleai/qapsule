//! M4 — theme identity (SPEC M4).
//!
//! Run: the measured theme rule (wrapped in a decision logger) with the reference outcome
//! and ranker, over (a) the labeled episode set ([`crate::episodes`], seeds) and (b) the
//! regime market / real data (unlabeled). Raw = every fact
//! `[inst, bar, kind, dir, family bit, edge, label, story, decision]` and every story
//! `[inst, kind, dir, families mask, facts]`.
//!
//! Metrics:
//! 1. **Reduction** = 1 − stories / facts (baseline: one story per fact).
//! 2. **Wrong merge** (labeled): among pairs of *labeled* facts in the same story, the
//!    share from different episodes (= 1 − pair precision); stories mixing ≥ 2 episodes;
//!    separately, stories mixing background with an episode.
//! 3. **Split** (labeled): among pairs of facts of one episode that are in stories, the
//!    share in different stories (= 1 − pair recall).
//! 4. **Lost information**: facts the rule `Ignore`d; facts the engine dropped after a
//!    `Join` (an `exit` joined to a story without that state, or an unknown id); facts in
//!    a story whose kind or direction differs from the story's (they never show in the
//!    sentence or glyph); evidence families of joined facts missing from the story's
//!    `families` (dropped families).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::rc::Rc;

use qp_capsule::ThemeRule;
use qp_schema::capsule::Edge;
use serde::{Deserialize, Serialize};

use crate::common::{
    self, dir_code, f, family_bit, kind_code, ratio, to_json, write_file, DataKind, Dataset,
};
use crate::episodes::{self, Episode, TEMPLATES};
use crate::m3::{raw_location, read_raws, write_summary};
use crate::registry;
use crate::sim;

/// Raw record of one dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M4Raw {
    /// Dataset.
    pub dataset: String,
    /// Alternative.
    pub alt: String,
    /// Rule names (theme, outcome, ranker).
    pub rules: Vec<String>,
    /// Labeled set?
    pub labeled: bool,
    /// Instruments.
    pub instruments: Vec<String>,
    /// Episodes (labeled set).
    pub episodes: Vec<Episode>,
    /// `[inst, bar, kind, dir, family bit, edge (0 point, 1 enter, 2 exit), label, story
    /// index (-1 none), decision (0 new, 1 join, 2 ignore, -1 not asked)]`.
    pub facts: Vec<[i64; 9]>,
    /// `[inst, kind, dir, families mask, facts]`.
    pub stories: Vec<[u32; 5]>,
}

/// Build the raw record of one dataset.
#[must_use]
pub fn run_dataset(
    ds: &Dataset,
    eps: &[Episode],
    labeled: bool,
    alt: &str,
    make: &dyn Fn() -> Box<dyn ThemeRule>,
) -> M4Raw {
    let log: sim::DecisionLog = Rc::default();
    let theme = Box::new(sim::LoggedTheme {
        inner: make(),
        log: Rc::clone(&log),
    });
    let (engine, facts) = sim::run(
        &ds.inputs,
        theme,
        registry::reference_outcome(),
        registry::reference_ranker(),
    );
    let (tn, on, rn) = engine.rule_names();
    let inst_idx: BTreeMap<&str, u32> = ds
        .inputs
        .iter()
        .enumerate()
        .map(|(i, x)| (x.symbol.as_str(), u32::try_from(i).unwrap_or(0)))
        .collect();
    let stories = sim::all_stories(&engine);
    let mut story_of: BTreeMap<&str, i64> = BTreeMap::new();
    let mut srows = Vec::new();
    for (k, s) in stories.iter().enumerate() {
        for fa in &s.facts {
            story_of.insert(fa.id.as_str(), i64::try_from(k).unwrap_or(-1));
        }
        let mask = s
            .story
            .families
            .iter()
            .fold(0u32, |m, x| m | u32::from(family_bit(*x)));
        srows.push([
            inst_idx
                .get(s.story.instrument.as_str())
                .copied()
                .unwrap_or(0),
            u32::from(kind_code(s.story.kind)),
            u32::from(dir_code(s.story.direction)),
            mask,
            u32::try_from(s.facts.len()).unwrap_or(0),
        ]);
    }
    let log = log.borrow();
    let mut frows = Vec::new();
    for fa in &facts {
        let Some(m) = fa.meaning.as_ref() else {
            continue;
        };
        let inst = inst_idx.get(m.instrument.as_str()).copied().unwrap_or(0);
        let edge = match m.edge {
            Edge::Point => 0,
            Edge::Enter => 1,
            Edge::Exit => 2,
        };
        let lab = if labeled {
            episodes::label(eps, inst, fa.bar)
        } else {
            -1
        };
        frows.push([
            i64::from(inst),
            i64::try_from(fa.bar).unwrap_or(-1),
            i64::from(kind_code(m.kind)),
            i64::from(dir_code(m.direction)),
            i64::from(family_bit(m.family)),
            edge,
            lab,
            story_of.get(fa.id.as_str()).copied().unwrap_or(-1),
            log.get(&fa.id).map_or(-1, |&c| i64::from(c)),
        ]);
    }
    M4Raw {
        dataset: ds.name.clone(),
        alt: alt.to_owned(),
        rules: vec![tn.to_owned(), on.to_owned(), rn.to_owned()],
        labeled,
        instruments: ds.inputs.iter().map(|x| x.symbol.clone()).collect(),
        episodes: eps.to_vec(),
        facts: frows,
        stories: srows,
    }
}

// ---------------------------------------------------------------------------
// Score
// ---------------------------------------------------------------------------

/// Lost information counts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Lost {
    /// Facts `Ignore`d by the rule.
    pub ignored: u64,
    /// Facts dropped by the engine after a join.
    pub dropped: u64,
    /// Facts in a story with another kind or direction.
    pub mismatched: u64,
    /// Of those, kind differs.
    pub kind_differs: u64,
    /// Family bits of joined facts missing from the story's families (count of
    /// story × family).
    pub dropped_families: u64,
    /// Stories with at least one dropped family.
    pub stories_dropping_family: u64,
}

/// Labeled-set metrics.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Labeled {
    /// Episodes.
    pub episodes: u64,
    /// Episodes with ≥ 1 fact.
    pub episodes_with_facts: u64,
    /// Labeled facts / all facts.
    pub labeled_share: Option<f64>,
    /// Same-story labeled pairs.
    pub pairs_same_story: u64,
    /// Of those, from different episodes.
    pub pairs_wrong: u64,
    /// Wrong-merge pair rate.
    pub wrong_merge_rate: Option<f64>,
    /// Stories mixing ≥ 2 episodes.
    pub stories_mixing: u64,
    /// Stories mixing background and an episode.
    pub stories_mixing_background: u64,
    /// Same-episode pairs (facts in stories).
    pub pairs_same_episode: u64,
    /// Of those, in different stories.
    pub pairs_split: u64,
    /// Split pair rate.
    pub split_rate: Option<f64>,
    /// Per template: `[episodes, same-episode pairs, split pairs]`.
    pub by_template: Vec<[u64; 3]>,
}

/// Dataset score.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DatasetScore {
    /// Name.
    pub dataset: String,
    /// Facts.
    pub facts: u64,
    /// Stories.
    pub stories: u64,
    /// 1 − stories / facts.
    pub reduction: Option<f64>,
    /// Lost information.
    pub lost: Lost,
    /// Labeled metrics.
    pub labeled: Option<Labeled>,
}

/// Summary.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct M4Summary {
    /// Alternative.
    pub alt: String,
    /// Data kind.
    pub data: String,
    /// Rules.
    pub rules: Vec<String>,
    /// Per dataset.
    pub datasets: Vec<DatasetScore>,
}

fn pairs(n: u64) -> u64 {
    n * n.saturating_sub(1) / 2
}

/// Score one raw record.
#[must_use]
pub fn score_one(r: &M4Raw) -> DatasetScore {
    let mut lost = Lost::default();
    let mut fam_union: BTreeMap<i64, u32> = BTreeMap::new();
    for x in &r.facts {
        let story = x[7];
        if x[8] == 2 {
            lost.ignored += 1;
        } else if story < 0 {
            lost.dropped += 1;
        }
        if story >= 0 {
            let s = &r.stories[usize::try_from(story).unwrap_or(0)];
            let kd = i64::from(s[1]) != x[2];
            if kd || i64::from(s[2]) != x[3] {
                lost.mismatched += 1;
                if kd {
                    lost.kind_differs += 1;
                }
            }
            *fam_union.entry(story).or_insert(0) |= u32::try_from(x[4]).unwrap_or(0);
        }
    }
    for (k, u) in &fam_union {
        let s = &r.stories[usize::try_from(*k).unwrap_or(0)];
        let miss = u & !s[3];
        if miss != 0 {
            lost.stories_dropping_family += 1;
            lost.dropped_families += u64::from(miss.count_ones());
        }
    }
    let n_f = r.facts.len() as u64;
    let n_s = r.stories.len() as u64;
    let labeled = r.labeled.then(|| {
        // Story → label counts.
        let mut per_story: BTreeMap<i64, BTreeMap<i64, u64>> = BTreeMap::new();
        let mut per_ep: BTreeMap<i64, BTreeMap<i64, u64>> = BTreeMap::new();
        let mut lab_n = 0u64;
        let mut eps_with: BTreeSet<i64> = BTreeSet::new();
        for x in &r.facts {
            if x[6] >= 0 {
                lab_n += 1;
                eps_with.insert(x[6]);
            }
            if x[7] >= 0 {
                *per_story.entry(x[7]).or_default().entry(x[6]).or_insert(0) += 1;
                if x[6] >= 0 {
                    *per_ep.entry(x[6]).or_default().entry(x[7]).or_insert(0) += 1;
                }
            }
        }
        let mut same = 0u64;
        let mut wrong = 0u64;
        let mut mixing = 0u64;
        let mut mix_bg = 0u64;
        for labs in per_story.values() {
            let labeled: Vec<u64> = labs
                .iter()
                .filter(|(l, _)| **l >= 0)
                .map(|(_, c)| *c)
                .collect();
            let tot: u64 = labeled.iter().sum();
            let within: u64 = labeled.iter().map(|&c| pairs(c)).sum();
            same += pairs(tot);
            wrong += pairs(tot) - within;
            if labeled.len() >= 2 {
                mixing += 1;
            }
            if !labeled.is_empty() && labs.contains_key(&-1) {
                mix_bg += 1;
            }
        }
        let mut ep_pairs = 0u64;
        let mut split = 0u64;
        let mut by_template = vec![[0u64; 3]; TEMPLATES.len()];
        for e in &r.episodes {
            by_template[e.template as usize][0] += 1;
        }
        for (ep, stories) in &per_ep {
            let tot: u64 = stories.values().sum();
            let within: u64 = stories.values().map(|&c| pairs(c)).sum();
            ep_pairs += pairs(tot);
            split += pairs(tot) - within;
            if let Some(e) = r.episodes.iter().find(|e| i64::from(e.id) == *ep) {
                by_template[e.template as usize][1] += pairs(tot);
                by_template[e.template as usize][2] += pairs(tot) - within;
            }
        }
        Labeled {
            episodes: r.episodes.len() as u64,
            episodes_with_facts: eps_with.len() as u64,
            labeled_share: ratio(lab_n, n_f),
            pairs_same_story: same,
            pairs_wrong: wrong,
            wrong_merge_rate: ratio(wrong, same),
            stories_mixing: mixing,
            stories_mixing_background: mix_bg,
            pairs_same_episode: ep_pairs,
            pairs_split: split,
            split_rate: ratio(split, ep_pairs),
            by_template,
        }
    });
    DatasetScore {
        dataset: r.dataset.clone(),
        facts: n_f,
        stories: n_s,
        reduction: ratio(n_s, n_f).map(|x| 1.0 - x),
        lost,
        labeled,
    }
}

/// Score raw records.
#[must_use]
pub fn score(alt: &str, data: &str, raws: &[M4Raw]) -> M4Summary {
    M4Summary {
        alt: alt.to_owned(),
        data: data.to_owned(),
        rules: raws.first().map(|r| r.rules.clone()).unwrap_or_default(),
        datasets: raws.iter().map(score_one).collect(),
    }
}

/// Markdown.
#[must_use]
pub fn markdown(s: &M4Summary) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "# M4 — `{}` ({})\n", s.alt, s.data);
    let _ = writeln!(
        o,
        "Kurallar: tema `{}`, sonuç `{}`, sıralayıcı `{}`.\n",
        s.rules.first().map_or("", String::as_str),
        s.rules.get(1).map_or("", String::as_str),
        s.rules.get(2).map_or("", String::as_str)
    );
    let _ = writeln!(o, "## 1, 4. Azalma ve kaybolan bilgi\n");
    let _ = writeln!(o, "| Veri | Olgu | Hikâye | Azalma | Yok sayılan | Motorun düşürdüğü | Tür/yön farklı (tür) | Düşen aile (hikâye) |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|");
    for d in &s.datasets {
        let l = &d.lost;
        let _ = writeln!(
            o,
            "| {} | {} | {} | {} | {} | {} | {} ({}) | {} ({}) |",
            d.dataset,
            d.facts,
            d.stories,
            f(d.reduction, 3),
            l.ignored,
            l.dropped,
            l.mismatched,
            l.kind_differs,
            l.dropped_families,
            l.stories_dropping_family
        );
    }
    let lab: Vec<_> = s
        .datasets
        .iter()
        .filter_map(|d| d.labeled.as_ref().map(|l| (d, l)))
        .collect();
    if !lab.is_empty() {
        let _ = writeln!(
            o,
            "\n## 2–3. Etiketli bölüm kümesi: yanlış birleştirme ve bölünme\n"
        );
        let _ = writeln!(o, "| Veri | Bölüm (olgulu) | Etiketli olgu payı | Aynı hikâye çifti | Yanlış birleştirme oranı | Karışık hikâye | Arka planla karışık | Aynı bölüm çifti | Bölünme oranı |");
        let _ = writeln!(o, "|---|---|---|---|---|---|---|---|---|");
        for (d, l) in &lab {
            let _ = writeln!(
                o,
                "| {} | {} ({}) | {} | {} | {} | {} | {} | {} | {} |",
                d.dataset,
                l.episodes,
                l.episodes_with_facts,
                f(l.labeled_share, 3),
                l.pairs_same_story,
                f(l.wrong_merge_rate, 3),
                l.stories_mixing,
                l.stories_mixing_background,
                l.pairs_same_episode,
                f(l.split_rate, 3)
            );
        }
        let _ = writeln!(
            o,
            "\nŞablon başına bölünme (tüm etiketli kümeler toplamı):\n"
        );
        let _ = writeln!(o, "| Şablon | Bölüm | Aynı bölüm çifti | Bölünen | Oran |");
        let _ = writeln!(o, "|---|---|---|---|---|");
        for (ti, name) in TEMPLATES.iter().enumerate() {
            let mut c = [0u64; 3];
            for (_, l) in &lab {
                for (ck, x) in c.iter_mut().zip(l.by_template[ti].iter()) {
                    *ck += x;
                }
            }
            let _ = writeln!(
                o,
                "| {name} | {} | {} | {} | {} |",
                c[0],
                c[1],
                c[2],
                f(ratio(c[2], c[1]), 3)
            );
        }
    }
    o
}

/// The datasets of M4 for a data kind: synthetic → labeled episode sets (seeds) and the
/// regime market (seeds); real → real data (unlabeled).
#[must_use]
pub fn datasets(data: DataKind, seeds: &[u64]) -> Vec<(Dataset, Vec<Episode>, bool)> {
    match data {
        DataKind::Synthetic => {
            let mut v: Vec<(Dataset, Vec<Episode>, bool)> = seeds
                .iter()
                .map(|&s| {
                    let (d, e) = episodes::build(s);
                    (d, e, true)
                })
                .collect();
            v.extend(
                common::datasets(data, seeds)
                    .into_iter()
                    .map(|d| (d, Vec::new(), false)),
            );
            v
        }
        DataKind::Real => common::datasets(data, seeds)
            .into_iter()
            .map(|d| (d, Vec::new(), false))
            .collect(),
    }
}

/// `m4 run`.
///
/// # Errors
/// Unknown alternative, missing data, I/O.
pub fn cmd_run(alt: &str, data: DataKind, seeds: &[u64]) -> Result<String, String> {
    let fac = registry::find(&registry::themes(), alt)
        .ok_or(format!("bilinmeyen M4 alternatifi: {alt}"))?;
    let ds = datasets(data, seeds);
    if ds.is_empty() {
        return Err("veri yok".to_owned());
    }
    let raws = common::par_map(&ds, |(d, e, l)| run_dataset(d, e, *l, alt, &fac));
    let dir = raw_location("M4", alt, data)?;
    for r in &raws {
        write_file(
            &dir.join(common::raw_file_name(data, &r.dataset)),
            &to_json(r),
        )
        .map_err(|e| e.to_string())?;
    }
    cmd_score(alt, data)
}

/// `m4 score`.
///
/// # Errors
/// Missing raw, I/O.
pub fn cmd_score(alt: &str, data: DataKind) -> Result<String, String> {
    let dir = raw_location("M4", alt, data)?;
    let raws: Vec<M4Raw> = read_raws(&dir, data)?;
    let s = score(alt, data.name(), &raws);
    write_summary("M4", alt, data, &to_json(&s), &markdown(&s))
}
