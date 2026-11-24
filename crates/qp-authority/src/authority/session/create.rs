//! Building a session: the full constructor behind `Session::build_with`.

use super::*;

impl Session {
    #[allow(clippy::too_many_lines)]
    pub(super) fn build_with_keys(
        init: SessionInit,
        repo_root: &Path,
        provider: Provider,
        schema: Value,
        trust_keys: Option<(Vec<u8>, Vec<u8>)>,
        subs: Subsystems,
    ) -> Result<Self, String> {
        let capsules = init.capsules.clone();
        // (R5B, §25.3) The agent policy, by name.
        let agent_policy = match init.agent_policy.as_deref() {
            None => false,
            Some(crate::agent::AGENT_POLICY_R5B) => true,
            Some(other) => {
                return Err(format!(
                    "init.agent_policy: unknown policy `{other}` (one of {})",
                    crate::agent::AGENT_POLICY_R5B
                ))
            }
        };
        // (R5/M19, K53) The host's feed rule, by name.
        let coin_flip = match &init.feed_rule {
            None => None,
            Some(f) => Some(
                qp_capsule::feedrule::FeedRule::named(&f.rule, f.exempt_negative).ok_or_else(
                    || {
                        let names: Vec<&str> = qp_capsule::feedrule::FEED_RULES
                            .iter()
                            .map(|(n, _)| *n)
                            .collect();
                        format!(
                            "init.feed_rule.rule: unknown rule `{}` (one of {})",
                            f.rule,
                            names.join(", ")
                        )
                    },
                )?,
            ),
        };
        // (RP4) The capsule engine's named constant record.
        if let Some(name) = capsules.as_ref().and_then(|c| c.engine.as_deref()) {
            if qp_capsule::EngineConfig::named(name).is_none() {
                return Err(format!(
                    "init.capsules.engine: unknown record `{name}` (one of {})",
                    qp_capsule::ENGINES.join(", ")
                ));
            }
        }
        let doc: Document = match init.document {
            DocumentRef::Inline(d) => *d,
            DocumentRef::Path(p) => read_json(&confine(repo_root, &p)?)?,
        };
        // Canonical form of every view (GRAMMAR.md §11); the log replays from it.
        let doc = qp_ops::canon::canon_document_owned(doc)?;
        let mut limits = default_limits();
        if agent_policy {
            limits.max_watches = Some(qp_ops::watch::MAX_WATCHES);
        }
        let host = init
            .principals
            .iter()
            .find(|p| p.role == Role::Host)
            .map(|p| p.id.clone());
        let mut plane = DataPlane::new(provider, host, limits.max_bind_bars);
        // (R5B, §25.4) The comment validator's strictness.
        if let Some(st) = init.comment_strictness {
            plane.comment_strictness = st;
        }
        // (R5B, §25.7) Agents reading for a person: the person must be a known human
        // principal that works for nobody itself (no chains, no cycles).
        for p in &init.principals {
            let Some(person) = &p.on_behalf else { continue };
            let ok = init.principals.iter().any(|q| {
                &q.id == person
                    && q.id != p.id
                    && q.on_behalf.is_none()
                    && matches!(q.author, qp_schema::Author::Human { .. })
            });
            if !ok {
                return Err(format!(
                    "principal `{}`: on_behalf `{person}` must be another known principal whose author is human and who works for nobody",
                    p.id
                ));
            }
            plane.delegates.insert(p.id.clone(), person.clone());
        }
        let s = Self {
            initial: doc.clone(),
            doc,
            log: Vec::new(),
            principals: init.principals,
            clock: init.clock,
            epoch: init.epoch,
            ttl: init.proposal_ttl_ms,
            limits,
            conns: Vec::new(),
            cache: Cache::default(),
            intent_state: BTreeMap::new(),
            directives: BTreeMap::new(),
            focus: BTreeMap::new(),
            att_counter: 0,
            input_counter: 0,
            schema,
            plane,
            rd: None,
            pending_stamps: Vec::new(),
            incremental: true,
            pending_views: Vec::new(),
            view_reads: None,
            claims: crate::claims::Claims::default(),
            struct_gen: 1,
            live_index: None,
            series_writer: BTreeMap::new(),
            invalidation_writer: BTreeMap::new(),
            intent_epoch: 1,
            settle: crate::dispute::Settle::default(),
            runner: None,
            trust_keys,
            proj: None,
            fact_ix: std::cell::RefCell::default(),
            cap_ix: std::cell::RefCell::default(),
            sentences: std::cell::RefCell::default(),
            item_memo: std::cell::RefCell::default(),
            projs: std::cell::RefCell::default(),
            head_state: qp_plane::capidx::HeadState::default(),
            views: Vec::new(),
            real_facts: Vec::new(),
            headlines: BTreeMap::new(),
            headline_cal: None,
            audience_seen: BTreeSet::new(),
            other: None,
            in_public: false,
            pending_pub: None,
            pub_caps: BTreeSet::new(),
            full_to_pub: Vec::new(),
            pub_at: Vec::new(),
            prop_map: BTreeMap::new(),
            viewers: BTreeMap::new(),
            viewer_batch: false,
            coin_flip,
            coin_flip_for: BTreeMap::new(),
            market: None,
            agent: crate::agent::AgentState {
                policy: agent_policy,
                ..crate::agent::AgentState::default()
            },
            comment_audit: Vec::new(),
            registry: qp_subsystem::Registry::default(),
            slots: qp_subsystem::Slots::default(),
            outbox: qp_subsystem::Outbox::default(),
            wake: None,
            subs,
            emit_depth: 0,
        };
        s.load(capsules.as_ref())
    }

    /// (RF1 S3.1) The load, once the fields are built: the subsystems are registered before
    /// anything runs (the load evaluates intents and runs the capsule engine).
    fn load(mut self, capsules: Option<&qp_schema::protocol::CapsuleInit>) -> Result<Self, String> {
        let s = &mut self;
        for sub in s.subs.list() {
            s.register(*sub)?;
        }
        s.audience_seen = initial_audiences(&s.doc);
        s.start_capsules(capsules);
        // Resolve the bindings on load (§18.3); a first ready resolution is anchored by
        // an engine entry. No connection exists yet: its notifications go nowhere.
        s.pending_stamps = s.plane.sync(&s.doc);
        s.rd = reading_doc(&s.doc, &s.plane);
        if let Some(w) = s.other.as_mut() {
            w.rd = reading_doc(&w.doc, &s.plane);
        }
        if let Some(seq) = s.stamp_anchors() {
            let _ = s.invalidations(seq);
        }
        s.eval_intents(s.seq());
        let _ = s.eval_intents_public();
        // The capsule engine catches up with the final bars at load (R3, §20).
        let _ = s.run_capsules();
        s.pending_views.clear();
        // No connection exists at load: claims asserted now are registered (for their
        // finalisation) but announce nothing.
        let _ = s.claims.take();
        Ok(self)
    }
}

/// The principals whose capsule preferences or watches the initial document holds (other
/// readers get projections, §20, §25.5).
fn initial_audiences(doc: &Document) -> BTreeSet<String> {
    let mut seen: BTreeSet<String> = doc.audiences.keys().cloned().collect();
    seen.extend(
        doc.intents
            .iter()
            .filter_map(|i| i.watch.as_ref().map(|w| w.for_.clone())),
    );
    seen
}
