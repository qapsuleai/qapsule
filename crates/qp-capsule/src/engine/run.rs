//! One bar of the lifecycle state machine ([`Engine::run_bar`]) and closing a story into
//! the ledger.

use super::*;

pub(super) struct BarIn<'a> {
    pub(super) instrument: &'a str,
    pub(super) series: &'a str,
    pub(super) now: u64,
    pub(super) t: i64,
    pub(super) bars: &'a [Bar],
    pub(super) facts: &'a [Fact],
    pub(super) instruments: u32,
}

#[derive(Default)]
pub(super) struct BarOut {
    pub(super) events: Vec<StoryEvent>,
    pub(super) closed: Vec<OpenStory>,
    pub(super) new_counts: BTreeMap<String, BirthCounts>,
    /// Capsule id → number of transitions made in step 3 (birth / join), first-touch
    /// order kept in `step3_order`.
    pub(super) step3: BTreeMap<String, usize>,
    pub(super) step3_order: Vec<String>,
}

impl Engine {
    pub(super) fn record_close(&mut self, s: &OpenStory, t: i64) {
        let stage = s.story.stage;
        let (outcome, state) = match stage {
            Stage::Realized => (Outcome::Confirmed, None),
            Stage::Invalidated => (Outcome::Refuted, Some(CapsuleState::Invalidated)),
            Stage::Expired => (Outcome::Refuted, Some(CapsuleState::Expired)),
            _ => return,
        };
        let rec = LedgerRecord {
            entry: LedgerEntry {
                finding: finding_id(&s.id),
                state,
                outcome,
                recorded_at: t,
                note: None,
                capsule: Some(s.id.clone()),
                key: Some(s.story.ledger_key.clone()),
                stage: Some(stage),
                bar: Some(s.story.last_bar),
            },
            retracted_at: None,
        };
        self.index.insert(&rec);
        self.ledger.push(rec);
    }

    fn birth_counts_of(&self, id: &str, out: &BarOut) -> BirthCounts {
        out.new_counts
            .get(id)
            .or_else(|| self.birth_counts.get(id))
            .copied()
            .unwrap_or_default()
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn run_bar(
        &self,
        inp: &BarIn<'_>,
        open: &mut Vec<OpenStory>,
        counters: &mut Counters,
    ) -> BarOut {
        let mut out = BarOut::default();
        let mut touched: Vec<String> = Vec::new();
        let touch = |touched: &mut Vec<String>, id: &str| {
            if !touched.iter().any(|x| x == id) {
                touched.push(id.to_owned());
            }
        };

        // (1) outcome rule.
        let mut i = 0;
        while i < open.len() {
            if let Some((v, code)) = self.cfg.outcome.judge(&open[i], inp.bars, inp.now) {
                let stage = match v {
                    Verdict::Realized => Stage::Realized,
                    Verdict::Invalidated => Stage::Invalidated,
                };
                let mut s = open.remove(i);
                let tr = push_transition(&mut s, stage, inp.now, inp.t, Vec::new(), code);
                out.events.push(event_of(&s, tr));
                touch(&mut touched, &s.id);
                out.closed.push(s);
            } else {
                i += 1;
            }
        }
        // (2) horizon.
        let mut i = 0;
        while i < open.len() {
            let st = &open[i].story;
            if inp.now >= st.born_bar.saturating_add(u64::from(st.horizon_bars)) {
                let mut s = open.remove(i);
                let tr = push_transition(
                    &mut s,
                    Stage::Expired,
                    inp.now,
                    inp.t,
                    Vec::new(),
                    "horizon",
                );
                out.events.push(event_of(&s, tr));
                touch(&mut touched, &s.id);
                out.closed.push(s);
            } else {
                i += 1;
            }
        }

        // (3) facts in (detector, code, id) order.
        let mut facts: Vec<&Fact> = inp
            .facts
            .iter()
            .filter(|f| {
                f.bar == inp.now
                    && f.series == inp.series
                    && f.meaning
                        .as_ref()
                        .is_some_and(|m| m.instrument == inp.instrument)
            })
            .collect();
        facts.sort_by(|a, b| {
            let ka = (
                a.meaning.as_ref().map_or("", |m| m.detector.as_str()),
                fact_code(a),
                a.id.as_str(),
            );
            let kb = (
                b.meaning.as_ref().map_or("", |m| m.detector.as_str()),
                fact_code(b),
                b.id.as_str(),
            );
            ka.cmp(&kb)
        });
        for f in facts {
            let Some(m) = f.meaning.as_ref() else {
                continue;
            };
            let decision = {
                let refs: Vec<&OpenStory> = open.iter().collect();
                self.cfg.theme.decide(f, &refs)
            };
            let target = match decision {
                ThemeDecision::Ignore => continue,
                ThemeDecision::Join(id) => match open.iter().position(|s| s.id == id) {
                    Some(p) => p,
                    None => continue,
                },
                ThemeDecision::New { theme } => {
                    let id = capsule_id(inp.instrument, &theme, inp.now);
                    if let Some(p) = open.iter().position(|s| s.id == id) {
                        p
                    } else {
                        // Born.
                        let key = ledger_key(f).unwrap_or_default();
                        let kind = m.kind.name();
                        let ikd =
                            format!("{}/{kind}/{}", inp.instrument, direction_name(m.direction));
                        let bc = BirthCounts {
                            same_kind_on_instrument: counters
                                .by_inst_kind_dir
                                .get(&ikd)
                                .copied()
                                .unwrap_or(0),
                            same_kind_all: counters.by_kind.get(kind).copied().unwrap_or(0),
                            stories_all: counters.total,
                        };
                        *counters.by_inst_kind_dir.entry(ikd).or_insert(0) += 1;
                        *counters.by_kind.entry(kind.to_owned()).or_insert(0) += 1;
                        counters.total += 1;
                        out.new_counts.insert(id.clone(), bc);
                        let tr = Transition {
                            stage: Stage::Born,
                            bar: inp.now,
                            t: inp.t,
                            facts: vec![f.id.clone()],
                            reason: "new_story".to_owned(),
                        };
                        let split = crate::sentence::split_condition(m);
                        let story = Story {
                            instrument: inp.instrument.to_owned(),
                            theme,
                            kind: m.kind,
                            direction: m.direction,
                            time: m.time,
                            stage: Stage::Born,
                            families: vec![m.family],
                            strength: m.strength,
                            importance: 0.0,
                            tier: 3,
                            series: inp.series.to_owned(),
                            born_bar: inp.now,
                            last_bar: inp.now,
                            horizon_bars: self.cfg.outcome.horizon(m.kind, m.time),
                            ledger_at_birth: self.ledger_stat(&key, inp.t),
                            ledger_key: key,
                            trust: if m.server {
                                Trust::Server
                            } else {
                                Trust::Replica
                            },
                            history: vec![tr.clone()],
                            condition: split.0.cloned(),
                            resolution: Some(Resolution {
                                rule: self.cfg.outcome.name().to_owned(),
                                condition: split.1.cloned(),
                            }),
                        };
                        let s = OpenStory {
                            id: id.clone(),
                            story,
                            facts: vec![f.clone()],
                        };
                        out.events.push(event_of(&s, tr));
                        open.push(s);
                        touch(&mut touched, &id);
                        *out.step3.entry(id.clone()).or_insert(0) += 1;
                        if !out.step3_order.contains(&id) {
                            out.step3_order.push(id);
                        }
                        continue;
                    }
                }
            };

            // Join.
            let s = &mut open[target];
            let (stage, reason) = if m.edge == Edge::Exit {
                let own_state = s.facts.iter().any(|g| {
                    g.meaning
                        .as_ref()
                        .is_some_and(|gm| gm.edge == Edge::Enter && gm.detector == m.detector)
                });
                if !own_state {
                    continue;
                }
                (Stage::Weakened, "state_ended")
            } else if opposite(s.story.direction, m.direction) {
                (Stage::Weakened, "opposite_fact")
            } else if s.story.families.contains(&m.family) {
                (Stage::Strengthened, "same_family")
            } else {
                (Stage::Confirmed, "new_family")
            };
            if stage != Stage::Weakened && m.strength > s.story.strength {
                s.story.strength = m.strength;
            }
            if stage == Stage::Confirmed {
                s.story.families.push(m.family);
                s.story.families.sort();
            }
            s.facts.push(f.clone());
            let tr = push_transition(s, stage, inp.now, inp.t, vec![f.id.clone()], reason);
            let id = s.id.clone();
            out.events.push(event_of(s, tr));
            touch(&mut touched, &id);
            *out.step3.entry(id.clone()).or_insert(0) += 1;
            if !out.step3_order.contains(&id) {
                out.step3_order.push(id);
            }
        }

        // (4) rank the stories that transitioned.
        let ledger_all = self.ledger_stat_all(inp.t);
        for id in &touched {
            let bc = self.birth_counts_of(id, &out);
            let s = open
                .iter()
                .chain(out.closed.iter())
                .find(|s| &s.id == id)
                .expect("touched story exists");
            let ctx = RankCtx {
                ledger: self.ledger_stat(&s.story.ledger_key, inp.t),
                ledger_all,
                same_kind_on_instrument: bc.same_kind_on_instrument,
                bars_seen: inp.now + 1,
                same_kind_all: bc.same_kind_all,
                stories_all: bc.stories_all,
                instruments: inp.instruments,
            };
            let (imp, tier) = self.cfg.ranker.rank(s, &ctx);
            for s in open.iter_mut().chain(out.closed.iter_mut()) {
                if &s.id == id {
                    s.story.importance = imp;
                    s.story.tier = tier;
                }
            }
            for e in &mut out.events {
                if &e.capsule == id {
                    e.story.importance = imp;
                    e.story.tier = tier;
                }
            }
        }
        out
    }
}
