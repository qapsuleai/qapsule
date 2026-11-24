//! Opening a capsule on a viewer: the face scene, the open scene and the open side.

use super::*;

/// The scene of a capsule's face in the feed (open depth `sentence`): the connection's
/// face design (`look.face`, `qp_designs::depth::face_view`) over the session's data,
/// at `w` × `h`, with the reader's sentence. `None` when the reader gets no face scene
/// (no data for the finding's series: the shell then shows the sentence as text).
#[must_use]
pub fn face_scene(
    s: &mut Session,
    item: &CapsuleItem,
    look: &ViewState,
    max_points: u32,
    w: u32,
    h: u32,
) -> Option<SceneDesc> {
    let heads = BTreeMap::from([(item.finding.clone(), item.sentence.text.clone())]);
    // (R4/H-FP) The headline pieces only for a design with a headline template.
    let parts = qp_designs::faces::headline_template(look.face.as_deref())
        .ok()
        .flatten()
        .and_then(|_| s.item_headline_parts(item, item_lang(item)))
        .map(|hp| (item.finding.as_str(), hp));
    // (R5/m17-int) The voice the authority wrote for this reader (the same function the
    // authority's own compile calls, `Session::reader_voice`).
    let p = Reader {
        inner: s.provider(),
        heads: &heads,
        parts,
        voice: item.voice.as_ref().map(|v| (item.finding.as_str(), v)),
        extra: OpenExtra::default(),
    };
    let id = format!("capsule-{}", item.finding);
    let view = qp_designs::depth::face_view_in(
        look_of(look),
        look.face.as_deref(),
        &item.finding,
        max_points,
        &p,
        &id,
    )
    .ok()?;
    let mut o = options(look, w, h, None);
    o.face.clone_from(&look.face);
    compile(&view, &p, &o).ok()
}

/// The open capsule's view at its depth, rebuilt over the session's current document and
/// data by the rule the authority's `opening.presentation` used (the card at `capsule`;
/// at `chart`/`evidence` the capsule's stored view, else the presenter's view with the id
/// the authority gave it).
fn open_view(
    p: &dyn DataProvider,
    stored: Option<View>,
    o: &Opening,
    look: &ViewState,
    max_points: u32,
    extra: OpenExtra,
) -> Option<View> {
    match o.to {
        OpenDepth::Sentence => None,
        OpenDepth::Capsule => {
            let id = o.presentation.capsule_view.as_ref()?.id.clone();
            // (R5/M20) The connection's open capsule design (`r4`: today's card).
            let [w, h] = o.frame?;
            let ctx = qp_designs::open::OpenCtx {
                ask: look.ask.as_deref(),
                lang: extra.lang.unwrap_or(Language::Tr),
                check: extra
                    .check
                    .and_then(|c| qp_designs::open::Check::of(c.device_check())),
                frame: [w, h],
            };
            qp_designs::open::capsule_view(
                look.open.as_deref(),
                look_of(look),
                &o.finding,
                max_points,
                p,
                &id,
                &ctx,
            )
            .ok()
        }
        OpenDepth::Chart | OpenDepth::Evidence => {
            let given = o.presentation.view.as_ref()?;
            if stored.is_some() {
                return stored;
            }
            let id = look.view.clone().unwrap_or_else(|| given.id.clone());
            depth::chart_view_in(look_of(look), &o.finding, p, &id).ok()
        }
    }
}

/// The scene of the open capsule (the opening's depth and frame, the view state's chart
/// range and style) over the session's own data; `None` when nothing is open or the
/// reader gets no scene at that depth (no data: the authority sent no view).
#[must_use]
pub fn open_scene(
    s: &mut Session,
    o: &Opening,
    headline: Option<&str>,
    voice: Option<&qp_schema::protocol::CapsuleVoice>,
    look: &ViewState,
    max_points: u32,
) -> Option<SceneDesc> {
    open_scene_with(
        s,
        o,
        headline,
        voice,
        look,
        max_points,
        (&[], Language::Tr),
        OpenExtra::default(),
    )
}

/// (R5/M18) [`open_scene`] with the open instrument's story rows and the reader's
/// language, for a connection with a story chart design (`look.story`; without one the
/// rows are not read); (R5/M20) and the reader's language and the device's check
/// ([`OpenExtra`]), which an open capsule design reads.
#[must_use]
#[allow(clippy::too_many_arguments)] // (R5/S) story rows (M18) and open extra (M20) both
pub fn open_scene_with(
    s: &mut Session,
    o: &Opening,
    headline: Option<&str>,
    voice: Option<&qp_schema::protocol::CapsuleVoice>,
    look: &ViewState,
    max_points: u32,
    stories: (&[qp_schema::protocol::StoryRow], Language),
    extra: OpenExtra,
) -> Option<SceneDesc> {
    if o.to == OpenDepth::Sentence {
        return None;
    }
    let [w, h] = o.frame?;
    let heads: BTreeMap<String, String> = headline
        .map(|t| (o.finding.clone(), t.to_owned()))
        .into_iter()
        .collect();
    let stored = o
        .presentation
        .view
        .as_ref()
        .and_then(|v| s.document().views.iter().find(|x| x.id == v.id).cloned());
    // (RP4/T7) The story rows' time, when the connection asked for it (the authority's
    // `tap_at` gives the same, `interact.rs`); computed before the provider borrows `s`.
    let stamps: BTreeMap<String, String> = if look.clock && o.to >= OpenDepth::Chart {
        stories
            .0
            .iter()
            .filter_map(|r| s.row_stamp(r, stories.1).map(|t| (r.id.clone(), t)))
            .collect()
    } else {
        BTreeMap::new()
    };
    let p = Reader {
        inner: s.provider(),
        heads: &heads,
        parts: None,
        voice: voice.map(|v| (o.finding.as_str(), v)),
        extra,
    };
    let view = open_view(&p, stored, o, look, max_points, extra)?;
    let range = look
        .range
        .as_ref()
        .filter(|r| o.to >= OpenDepth::Chart && r.view == view.id)
        .map(|r| (r.from_bar, r.to_bar));
    // (R5/M18) A story chart design: the chart at its price height, then the stories
    // (the authority's `tap_at` builds the same scene, `interact.rs`).
    let sd = if o.to >= OpenDepth::Chart {
        qp_designs::depth::story_design(look.story.as_deref())
    } else {
        None
    };
    let Some(sd) = sd else {
        return compile(&view, &p, &options(look, w, h, range)).ok();
    };
    let stamp = |r: &qp_schema::protocol::StoryRow| stamps.get(&r.id).cloned();
    let stamp_of: Option<&qp_designs::stories::StampFn<'_>> = look.clock.then_some(&stamp);
    // (R5/S) A design that compiles the whole scene (`korda`; it reads no first facts).
    if let Some(c) = sd.compile {
        let input = qp_designs::stories::StoryInput {
            rows: stories.0,
            focus: Some(&o.finding),
            hide_closed: look.hide_closed,
            lang: stories.1,
            fact_of: &|_| None,
            stamp_of,
        };
        return c(&view, &p, &options(look, w, h, range), &input).ok();
    }
    let scene = compile(
        &view,
        &p,
        &options(look, w, (sd.price_height)(look_of(look), h), range),
    )
    .ok()?;
    // (R5/M18) The first fact of each listed story's finding (R5/m18-pavka-perf: looked
    // up only if the story chart asks, i.e. when it clusters; compiling does not change
    // the document).
    let firsts = FirstFacts::new(s.document(), stories.0);
    let fact_of = |f: &str| firsts.get(f);
    let (rows, lang) = stories;
    Some((sd.extend)(
        look_of(look),
        look.theme.as_deref().or(view.theme.as_deref()),
        scene,
        h,
        &qp_designs::stories::StoryInput {
            rows,
            focus: Some(&o.finding),
            hide_closed: look.hide_closed,
            lang,
            fact_of: &fact_of,
            stamp_of,
        },
    ))
}

/// The device's side of the open capsule (§22.4): a replica of the slice over its own data
/// of the open instrument only.
pub struct OpenSide {
    /// The slice as the authority sent it.
    pub open: ViewerOpen,
    replica: Replica,
    check: StoryCheck,
    /// The last check's data version (re-checked only when its data changed).
    checked_at: Option<u64>,
    data_version: u64,
}

impl std::fmt::Debug for OpenSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenSide")
            .field("capsule", &self.open.capsule)
            .field("check", &self.check)
            .finish_non_exhaustive()
    }
}

impl OpenSide {
    /// The open side of `open` at log position `seq`, its replica's own data from
    /// `provider` (the open instrument's feeds the host served), its replica's authority
    /// built with subsystems `subs`.
    #[must_use]
    pub fn new(
        principal: &str,
        open: ViewerOpen,
        seq: u64,
        provider: qp_source::MemoryProvider,
        calendar: Option<qp_schema::Calendar>,
        subs: crate::Subsystems,
    ) -> Self {
        let mut replica = Replica::with_memory(principal, provider, subs);
        replica.set_headline_calendar(calendar);
        replica.start((*open.document).clone(), seq);
        Self {
            open,
            replica,
            check: StoryCheck::Pending,
            checked_at: None,
            data_version: 0,
        }
    }

    /// A new slice of the same capsule (the authority's document changed): the replica
    /// reloads it and keeps its data and the claims it knows.
    pub fn update(&mut self, open: ViewerOpen, seq: u64) {
        self.replica.start((*open.document).clone(), seq);
        self.open = open;
        self.checked_at = None;
    }

    /// A data event of the open instrument's feed (its own data).
    pub fn data_event(&mut self, ev: &DataEvent) {
        let _ = self.replica.data_event(ev);
        self.data_version += 1;
    }

    /// A message of the authority for the open capsule (its claims).
    pub fn on_message(&mut self, msg: &ServerMsg) {
        if let ServerMsg::Notify {
            event: Event::Claim { .. },
            ..
        } = msg
        {
            self.replica.on_message(msg);
            self.checked_at = None;
        }
    }

    /// The replica (its session compiles the open scene; its display holds the claims).
    pub fn replica_mut(&mut self) -> &mut Replica {
        &mut self.replica
    }

    /// The replica (read access).
    #[must_use]
    pub fn replica(&self) -> &Replica {
        &self.replica
    }

    /// Runs the check if its data or claims changed since the last one; returns the
    /// story check.
    pub fn check(&mut self) -> &StoryCheck {
        if self.checked_at != Some(self.data_version) {
            let bars = self.replica.bars(&self.open.series).map(<[Bar]>::to_vec);
            self.check = check_story(&self.open.document, &self.open.capsule, bars.as_deref());
            self.checked_at = Some(self.data_version);
        }
        &self.check
    }

    /// The trust state: the story check, overruled by the display rule over the claims on
    /// this capsule (a disputed claim: sources differ).
    pub fn trust(&mut self) -> TrustState {
        let cap = self.open.capsule.clone();
        let disputed = self.replica.display().iter().any(|d| {
            d.state == Shown::Disputed
                && matches!(&d.claim, qp_schema::protocol::ClaimRef::Capsule { capsule, .. } if *capsule == cap)
        });
        if disputed {
            return TrustState::Differs;
        }
        match self.check() {
            StoryCheck::Held => TrustState::Device,
            StoryCheck::Pending => TrustState::Checking,
            StoryCheck::Differs => TrustState::Differs,
            StoryCheck::Server => TrustState::Server,
        }
    }

    /// Capsule candidates on the open bar of the open series, from its own data (K28).
    #[must_use]
    pub fn candidates(&self) -> Vec<qp_capsule::CandidateStory> {
        self.replica.candidates(&self.open.series)
    }
}

/// A memory provider over the feeds a host served for the open instrument: their initial
/// bars and the data events since (§22.5 `bind`).
///
/// # Errors
/// An invalid feed description.
pub fn open_provider(
    principal: &str,
    feeds: &[qp_schema::protocol::InitFeed],
    events: &[DataEvent],
) -> Result<qp_source::MemoryProvider, String> {
    let init = qp_schema::protocol::InitData {
        feeds: feeds.to_vec(),
        entitlements: BTreeMap::from([(
            principal.to_owned(),
            feeds.iter().map(|f| f.feed.symbol.clone()).collect(),
        )]),
        pending: Vec::new(),
        settled: None,
    };
    let mut p = qp_source::MemoryProvider::from_init(&init, std::path::Path::new("."))
        .map_err(|e| e.to_string())?;
    for ev in events {
        let _ = p.apply(ev);
    }
    Ok(p)
}
