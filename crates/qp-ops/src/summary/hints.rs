//! Fast selector hints for the document summary (indexes by kind and id).

use super::*;

impl<'a> FastHints<'a> {
    pub fn new(doc: &'a Document) -> Self {
        let mut fact_at: HashMap<&str, usize> = HashMap::with_capacity(doc.facts.len());
        for (i, f) in doc.facts.iter().enumerate() {
            fact_at.entry(f.id.as_str()).or_insert(i);
        }
        let mut finding_at: HashMap<&str, usize> = HashMap::with_capacity(doc.findings.len());
        for (i, f) in doc.findings.iter().enumerate() {
            finding_at.entry(f.id.as_str()).or_insert(i);
        }
        let mut gorder: Vec<(Option<u64>, usize)> = doc
            .findings
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.facts
                        .iter()
                        .filter_map(|id| fact_at.get(id.as_str()))
                        .map(|&j| doc.facts[j].bar)
                        .max(),
                    i,
                )
            })
            .collect();
        gorder.sort_unstable();
        Self {
            doc,
            fact_at,
            finding_at,
            gorder: gorder.into_iter().map(|(_, i)| i).collect(),
            facts_of_kind: RefCell::default(),
            findings_of_kind: RefCell::default(),
            findings_of_fact: RefCell::default(),
        }
    }

    /// Lower-case kind of every fact (a copy only per distinct raw kind).
    fn lower_kinds(&self) -> Vec<Rc<str>> {
        let mut seen: HashMap<&str, Rc<str>> = HashMap::new();
        self.doc
            .facts
            .iter()
            .map(|f| {
                Rc::clone(
                    seen.entry(f.kind.as_str())
                        .or_insert_with(|| Rc::from(f.kind.to_lowercase())),
                )
            })
            .collect()
    }

    /// (R4/T6) Both kind indexes in one pass each, on the first kind asked (one pass per
    /// asked kind before). Grouping by `to_lowercase` is the resolver's case-insensitive kind match.
    fn build_kind_indexes(&self) {
        if !self.facts_of_kind.borrow().is_empty() || self.doc.facts.is_empty() {
            return;
        }
        let lower = self.lower_kinds();
        let mut facts: HashMap<Rc<str>, Vec<(u64, usize)>> = HashMap::new();
        for (i, f) in self.doc.facts.iter().enumerate() {
            facts
                .entry(Rc::clone(&lower[i]))
                .or_default()
                .push((f.bar, i));
        }
        let mut findings: HashMap<Rc<str>, Vec<usize>> = HashMap::new();
        let mut kinds: Vec<&Rc<str>> = Vec::new();
        for &i in &self.gorder {
            kinds.clear();
            for id in &self.doc.findings[i].facts {
                if let Some(&j) = self.fact_at.get(id.as_str()) {
                    if !kinds.contains(&&lower[j]) {
                        kinds.push(&lower[j]);
                    }
                }
            }
            for k in &kinds {
                findings.entry(Rc::clone(k)).or_default().push(i);
            }
        }
        let mut fk = self.facts_of_kind.borrow_mut();
        for (k, mut v) in facts {
            v.sort_unstable();
            fk.insert(
                k.to_string(),
                Rc::new(v.into_iter().map(|(_, i)| i).collect()),
            );
        }
        let mut gk = self.findings_of_kind.borrow_mut();
        for (k, v) in findings {
            gk.insert(k.to_string(), Rc::new(v));
        }
    }

    fn kind_facts(&self, lower: &str) -> Rc<Vec<usize>> {
        self.build_kind_indexes();
        self.facts_of_kind
            .borrow()
            .get(lower)
            .map_or_else(|| Rc::new(Vec::new()), Rc::clone)
    }

    fn kind_findings(&self, lower: &str) -> Rc<Vec<usize>> {
        self.build_kind_indexes();
        self.findings_of_kind
            .borrow()
            .get(lower)
            .map_or_else(|| Rc::new(Vec::new()), Rc::clone)
    }

    fn fact_findings(&self, id: &str) -> Rc<Vec<usize>> {
        let mut cache = self.findings_of_fact.borrow_mut();
        let index = cache.get_or_insert_with(|| {
            let mut m: HashMap<&'a str, Vec<usize>> = HashMap::new();
            for &i in &self.gorder {
                let facts = &self.doc.findings[i].facts;
                for (k, x) in facts.iter().enumerate() {
                    // A finding counts once per fact id, however often it names it.
                    if facts[..k].iter().all(|y| y != x) {
                        m.entry(x.as_str()).or_default().push(i);
                    }
                }
            }
            m.into_iter().map(|(k, v)| (k, Rc::new(v))).collect()
        });
        index.get(id).map_or_else(|| Rc::new(Vec::new()), Rc::clone)
    }

    pub(crate) fn fact_pos(&self, id: &str) -> Option<usize> {
        self.fact_at.get(id).copied()
    }

    pub(crate) fn finding_pos(&self, id: &str) -> Option<usize> {
        self.finding_at.get(id).copied()
    }

    /// The ids a fact selector made of `kind`, `bar`, `first`, `latest` resolves to.
    fn fact_ids(&self, s: &FactSel) -> Vec<&'a str> {
        let doc = self.doc;
        if s.first && s.latest {
            return Vec::new();
        }
        #[allow(clippy::unreachable)] // invariant: hint candidates name a kind
        let Some(k) = &s.kind
        else {
            unreachable!("hint candidates name a kind");
        };
        let rows = self.kind_facts(&k.to_lowercase());
        let mut hits = rows
            .iter()
            .filter(|&&i| s.bar.is_none_or(|b| doc.facts[i].bar == b))
            .map(|&i| doc.facts[i].id.as_str());
        // (R4/T6) `pick` without collecting every row for `first` / `latest`.
        if s.first {
            hits.next().into_iter().collect()
        } else if s.latest {
            hits.next_back().into_iter().collect()
        } else {
            hits.collect()
        }
    }

    /// The ids a finding selector made of `kind` or `fact`, `first`, `latest` resolves to.
    fn finding_ids(&self, s: &FindingSel) -> Vec<&'a str> {
        let doc = self.doc;
        if s.first && s.latest {
            return Vec::new();
        }
        let rows: Rc<Vec<usize>> = match (&s.kind, &s.fact) {
            (Some(k), None) => self.kind_findings(&k.to_lowercase()),
            (None, Some(r)) => {
                #[allow(clippy::unreachable)] // invariant: candidates select their fact
                let FactRef::Select(h) = r.as_ref() else {
                    unreachable!("hint candidates select their fact");
                };
                let set = self.fact_ids(h);
                match set.len() {
                    0 => return Vec::new(),
                    1 => self.fact_findings(set[0]),
                    _ => {
                        let mut m = self.finding_ids_slow(&set);
                        pick(&mut m, s.first, s.latest);
                        return m;
                    }
                }
            }
            #[allow(clippy::unreachable)] // invariant: candidates carry a kind or a fact
            _ => unreachable!("hint candidates: kind or fact"),
        };
        let mut m: Vec<&str> = rows.iter().map(|&i| doc.findings[i].id.as_str()).collect();
        pick(&mut m, s.first, s.latest);
        m
    }

    /// Findings resting on any of `set`, in row order (several facts: never for a hint,
    /// which resolves to one fact).
    fn finding_ids_slow(&self, set: &[&str]) -> Vec<&'a str> {
        let doc = self.doc;
        let mut order: Vec<(Option<u64>, usize)> = doc
            .findings
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f.facts
                        .iter()
                        .filter_map(|id| self.fact_at.get(id.as_str()))
                        .map(|&j| doc.facts[j].bar)
                        .max(),
                    i,
                )
            })
            .collect();
        order.sort_unstable();
        order
            .into_iter()
            .map(|(_, i)| &doc.findings[i])
            .filter(|f| f.facts.iter().any(|x| set.contains(&x.as_str())))
            .map(|f| f.id.as_str())
            .collect()
    }

    /// [`crate::select::Resolver::fact_hint`].
    pub fn fact_hint(&self, f: &Fact) -> Option<FactSel> {
        let kind = FactSel {
            kind: Some(f.kind.clone()),
            ..FactSel::default()
        };
        let cands = vec![
            kind.clone(),
            FactSel {
                latest: true,
                ..kind.clone()
            },
            FactSel {
                first: true,
                ..kind.clone()
            },
            FactSel {
                bar: Some(f.bar),
                ..kind.clone()
            },
        ];
        choose(
            cands,
            &f.id,
            |c| self.fact_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }

    /// [`crate::select::Resolver::finding_hint`].
    pub fn finding_hint(&self, f: &Finding) -> Option<FindingSel> {
        let doc = self.doc;
        let latest = f
            .facts
            .iter()
            .filter_map(|id| self.fact_at.get(id.as_str()).map(|&i| &doc.facts[i]))
            .max_by_key(|x| x.bar)?;
        let kind = FindingSel {
            kind: Some(latest.kind.clone()),
            ..FindingSel::default()
        };
        let mut cands = vec![
            kind.clone(),
            FindingSel {
                latest: true,
                ..kind.clone()
            },
            FindingSel {
                first: true,
                ..kind.clone()
            },
        ];
        if let Some(h) = self.fact_hint(latest) {
            cands.push(FindingSel {
                fact: Some(Box::new(FactRef::Select(h))),
                ..FindingSel::default()
            });
        }
        choose(
            cands,
            &f.id,
            |c| self.finding_ids(c),
            |c| c.first = true,
            |c| c.latest = true,
        )
    }
}

/// `select::pick` without the error (both flags are handled by the caller).
fn pick(m: &mut Vec<&str>, first: bool, latest: bool) {
    if first {
        m.truncate(1);
    } else if latest && m.len() > 1 {
        m.drain(..m.len() - 1);
    }
}

/// `select::Resolver::choose`.
fn choose<'x, S: Clone>(
    cands: Vec<S>,
    target: &str,
    ids: impl Fn(&S) -> Vec<&'x str>,
    set_first: impl Fn(&mut S),
    set_latest: impl Fn(&mut S),
) -> Option<S> {
    let found: Vec<Vec<&'x str>> = cands.iter().map(&ids).collect();
    for (c, m) in cands.iter().zip(&found) {
        if m.len() == 1 && m[0] == target {
            return Some(c.clone());
        }
    }
    for (c, m) in cands.iter().zip(&found) {
        if m.first().is_some_and(|x| *x == target) {
            let mut c = c.clone();
            set_first(&mut c);
            return Some(c);
        }
        if m.last().is_some_and(|x| *x == target) {
            let mut c = c.clone();
            set_latest(&mut c);
            return Some(c);
        }
    }
    None
}
