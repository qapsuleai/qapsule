//! The viewer subsystem (RF1 S3.3; RP3/M15, §22): viewer connections on the authority —
//! the `viewer` topic's checks and state (a per-connection slot, [`ViewerConn`]), `visible`
//! messages and the pushed updates. The authority calls it as a pushed topic
//! (`qp_subsystem::TopicHandler`) after every input, in each viewer's reader world.

use qp_schema::protocol::{ProtocolError, Seq, Topic};
use qp_subsystem::{Moment, Phase, SessionCtx, Slots, Subsystem, TopicHandler};

use super::*;

/// The viewer subsystem: register it with the authority.
pub struct Viewer;

/// The viewer subsystem, as the composition registers it.
pub static VIEWER: Viewer = Viewer;

/// The `viewer` topic.
struct ViewerTopic;

static TOPICS: &[&dyn TopicHandler] = &[&ViewerTopic];

impl Subsystem for Viewer {
    fn name(&self) -> &'static str {
        "viewer"
    }
    fn phases(&self) -> &'static [Phase] {
        &[Phase::Unsubscribe]
    }
    /// The viewer subscription was removed.
    fn collect(&self, _phase: Phase, at: &Moment<'_>, ctx: &mut SessionCtx<'_>) {
        if let Moment::Sub { conn, sub } = at {
            if ctx
                .slots
                .conn::<ViewerConn>(conn)
                .is_some_and(|v| v.sub == *sub)
            {
                ctx.slots.take_conn::<ViewerConn>(conn);
            }
        }
    }
    fn topics(&self) -> &'static [&'static dyn TopicHandler] {
        TOPICS
    }
}

/// The `viewer` subscription of connection `conn`: checks it is the only one and its
/// fields.
fn check(
    slots: &Slots,
    conn: &str,
    limit: Option<u32>,
    width: Option<u32>,
) -> Result<(), ProtocolError> {
    if slots.conn::<ViewerConn>(conn).is_some() {
        return Err(E::new(
            ErrorCode::Duplicate,
            "a connection holds one `viewer` subscription",
        )
        .at("/topic/kind")
        .hint("Send `visible` to change what it shows.")
        .build());
    }
    if limit.is_some_and(|l| l == 0 || l > MAX_LIMIT) {
        return Err(E::new(
            ErrorCode::Invalid,
            format!("`limit` must be 1..={MAX_LIMIT}"),
        )
        .at("/topic/limit")
        .expected([format!("1..={MAX_LIMIT}")])
        .build());
    }
    if width.is_some_and(|w| !(64..=4096).contains(&w)) {
        return Err(E::new(ErrorCode::Invalid, "`width` must be 64..=4096")
            .at("/topic/width")
            .expected(["64..=4096"])
            .build());
    }
    Ok(())
}

impl TopicHandler for ViewerTopic {
    fn kind(&self) -> &'static str {
        "viewer"
    }

    /// Checks the subscription and remembers it (its first event follows the ack).
    fn subscribe(
        &self,
        conn: &str,
        sub: &str,
        t: &Topic,
        ctx: &mut SessionCtx<'_>,
    ) -> Result<(), ProtocolError> {
        let Topic::Viewer { limit, width } = t else {
            return Ok(());
        };
        check(ctx.slots, conn, *limit, *width)?;
        let vc = ViewerConn {
            sub: sub.to_owned(),
            limit: limit.unwrap_or(DEFAULT_LIMIT),
            width: width.unwrap_or(DEFAULT_FACE_WIDTH),
            ..ViewerConn::default()
        };
        ctx.slots.put_conn(conn, vc);
        Ok(())
    }

    fn pushing(&self, slots: &Slots) -> bool {
        slots.any_conn::<ViewerConn>()
    }

    fn pushes(&self, conn: &str, slots: &Slots) -> bool {
        slots.conn::<ViewerConn>(conn).is_some()
    }

    /// The `viewer` event of connection `conn` (always one when `all`).
    fn push(
        &self,
        conn: &str,
        all: bool,
        seq: Seq,
        ctx: &mut SessionCtx<'_>,
    ) -> Option<(String, Event)> {
        let before = ctx.slots.conn::<ViewerConn>(conn)?.data.clone();
        let update = viewer_update(conn, all, ctx);
        if !all && update.is_empty() && before == update.data {
            return None;
        }
        let sub = ctx.slots.conn::<ViewerConn>(conn)?.sub.clone();
        let viewer = Box::new(update);
        Some((sub, Event::Viewer { viewer, seq }))
    }

    /// A `visible` message of connection `conn`.
    fn visible(
        &self,
        conn: &str,
        sub: &str,
        visible: &[String],
        slots: &mut Slots,
    ) -> Option<Result<(), ProtocolError>> {
        let Some(v) = slots.conn_mut::<ViewerConn>(conn).filter(|v| v.sub == sub) else {
            let e = E::new(
                ErrorCode::NotFound,
                format!("no `viewer` subscription `{sub}`"),
            )
            .at("/sub")
            .hint("Subscribe to {\"kind\":\"viewer\"} first.")
            .build();
            return Some(Err(e));
        };
        let mut vis = visible.to_vec();
        vis.dedup();
        vis.truncate(v.limit as usize);
        v.visible = vis;
        Some(Ok(()))
    }

    fn visible_result(&self, conn: &str, ctx: &mut SessionCtx<'_>) -> Option<QueryResult> {
        let viewer = Box::new(viewer_update(conn, false, ctx));
        Some(QueryResult::Viewer { viewer })
    }

    /// (§22) The viewer subscription of connection `conn` whose open capsule rests on
    /// every feed in `feeds` (a claim about the open instrument goes there too).
    fn claim_sub<'s>(&self, conn: &str, feeds: &[FeedKey], slots: &'s Slots) -> Option<&'s str> {
        let v = slots.conn::<ViewerConn>(conn)?;
        (!feeds.is_empty() && !v.data.is_empty() && feeds.iter().all(|k| v.data.contains(k)))
            .then_some(v.sub.as_str())
    }

    /// (§22) The data subscriptions of viewer connection `conn`: the feeds its host serves
    /// it.
    fn data<'s>(&self, conn: &str, slots: &'s Slots) -> Option<&'s [FeedKey]> {
        slots.conn::<ViewerConn>(conn).map(|v| v.data.as_slice())
    }
}

/// Computes what changed for viewer connection `conn` (in the reader's world: the call
/// context's view) and records it as sent. `all`: the whole feed even if unchanged.
fn viewer_update(conn: &str, all: bool, ctx: &mut SessionCtx<'_>) -> ViewerUpdate {
    let Some(vc) = ctx.slots.conn::<ViewerConn>(conn).cloned() else {
        return ViewerUpdate::default();
    };
    let Some(c) = ctx.view.conn(conn) else {
        return ViewerUpdate::default();
    };
    let principal = c.principal.to_owned();
    let mp = c.profile.max_points;
    let Some(ci) = ctx.view.conn_ids().iter().position(|x| *x == conn) else {
        return ViewerUpdate::default();
    };
    let look = ctx.looks[ci].clone();
    let open_cap = (look.depth != OpenDepth::Sentence)
        .then(|| look.capsule.clone())
        .flatten();
    let at = Push {
        conn,
        principal: &principal,
        look: &look,
        mp,
    };
    let (update, vc) = viewer_compute(&at, vc, all, open_cap.as_deref(), ctx);
    ctx.slots.put_conn(conn, vc);
    update
}

/// Who a push is for: the connection, its principal, its view state and its points limit.
struct Push<'a> {
    conn: &'a str,
    principal: &'a str,
    look: &'a ViewState,
    mp: u32,
}

/// The reader's feed: its audience, as the `capsules` query lists it (asked of the
/// subsystem that answers the query, as the authority would).
fn feed_items(at: &Push<'_>, limit: u32, ctx: &mut SessionCtx<'_>) -> (Vec<CapsuleItem>, u32) {
    let q = Query::Capsules {
        instrument: None,
        stages: Vec::new(),
        open: None,
        meaning: None,
        since_bar: None,
        since_t: None,
        born_since_bar: None,
        born_since_t: None,
        latest: false,
        audience: true,
        limit: Some(limit),
    };
    let reg = ctx.registry;
    match reg.query(&q).map(|h| h.answer(at.conn, &q, ctx)) {
        Some(Ok(QueryResult::Capsules {
            capsules,
            coin_flip_hidden,
            ..
        })) => (capsules, coin_flip_hidden),
        _ => (Vec::new(), 0),
    }
}

#[allow(clippy::too_many_lines)]
/// [`viewer_update`] inside the reader's world.
fn viewer_compute(
    at: &Push<'_>,
    mut vc: ViewerConn,
    all: bool,
    open_cap: Option<&str>,
    ctx: &mut SessionCtx<'_>,
) -> (ViewerUpdate, ViewerConn) {
    let mut up = ViewerUpdate::default();
    let (principal, look) = (at.principal, at.look);
    let (caps, coin_flip_hidden) = feed_items(at, vc.limit, ctx);
    let items: Vec<ViewerItem> = caps
        .into_iter()
        .map(|it| {
            let (face, known_at) = viewer_row(ctx, principal, &it);
            ViewerItem {
                capsule: it,
                known_at,
                face,
            }
        })
        .collect();
    // (R5/M19) The coin-flip count joins the digest only when there is one (without a
    // host rule the digest is the R4 one).
    let fd = if coin_flip_hidden == 0 {
        digest_of(&items)
    } else {
        digest_of(&(&items, coin_flip_hidden))
    };
    if all || vc.feed_sent != Some(fd) {
        vc.feed_sent = Some(fd);
        up.feed = Some(items.clone());
        up.coin_flip_hidden = coin_flip_hidden;
    }
    // The faces on screen.
    let fh =
        qp_designs::faces::height_for(look_of(look), look.face.as_deref(), look.voice.is_some());
    let mut sent: BTreeMap<String, String> = BTreeMap::new();
    for id in &vc.visible {
        let Some(row) = items.iter().find(|r| &r.capsule.id == id && r.face) else {
            continue;
        };
        let Some(scene) = face_scene_in(ctx, &row.capsule, look, at.mp, vc.width, fh) else {
            continue;
        };
        let d = scene_digest(&scene);
        if vc.faces_sent.get(id) != Some(&d) {
            up.faces.push(ViewerFace {
                capsule: id.clone(),
                digest: d.clone(),
                scene: Box::new(scene),
                trust: TrustMark::Server,
            });
        }
        sent.insert(id.clone(), d);
    }
    vc.faces_sent = sent;
    let doc = ctx.view.document();
    // The reader's audience.
    let aud = doc.audiences.get(principal).cloned();
    let ad = digest_of(&aud);
    if vc.audience_sent != Some(ad) {
        vc.audience_sent = Some(ad);
        up.audience = aud;
    }
    // The open capsule's slice and data subscriptions.
    let open = open_cap.and_then(|cap| {
        let c = doc.capsules.iter().find(|c| c.id == cap)?;
        let story = c.story.as_ref();
        let series = story.map(|s| s.series.clone()).or_else(|| {
            let f = doc.findings.iter().find(|f| f.id == c.finding)?;
            let fact = doc.facts.iter().find(|x| f.facts.contains(&x.id))?;
            Some(fact.series.clone())
        })?;
        let instrument = story.map(|s| s.instrument.clone()).or_else(|| {
            doc.series
                .iter()
                .find(|s| s.id == series)
                .and_then(|s| s.instrument.as_ref().map(|i| i.symbol.clone()))
        })?;
        let (sl, feeds) = slice(doc, principal, &instrument);
        let feeds: Vec<FeedKey> = feeds
            .into_iter()
            .filter(|k| ctx.view.plane().entitled(principal, k))
            .collect();
        Some(ViewerOpen {
            capsule: cap.to_owned(),
            finding: c.finding.clone(),
            instrument,
            series,
            digest: document_digest(&sl),
            document: Box::new(sl),
            feeds,
        })
    });
    match open {
        Some(o) => {
            let key = (o.capsule.clone(), o.digest.clone());
            vc.data.clone_from(&o.feeds);
            if vc.open_sent.as_ref() != Some(&key) {
                vc.open_sent = Some(key);
                up.open = Some(o);
            }
        }
        None => {
            vc.data.clear();
            if vc.open_sent.take().is_some() {
                up.closed = true;
            }
        }
    }
    up.data.clone_from(&vc.data);
    // (R6/T3, §24) The market status (only once the host gave one).
    // Projected for this reader (§20.5): only the phases of instruments it may see;
    // the digest is of the projected value, per connection.
    if let Some(m) = ctx.read.and_then(|r| r.market_for(principal)) {
        let md = digest_of(&m);
        if all || vc.market_sent != Some(md) {
            vc.market_sent = Some(md);
            up.market = Some(m);
        }
    }
    (up, vc)
}

/// Whether the reader gets a face scene for `it` (entitled to every feed its series
/// rests on, with bars) and when it could first know its latest transition.
fn viewer_row(ctx: &SessionCtx<'_>, principal: &str, it: &CapsuleItem) -> (bool, Option<i64>) {
    let rd = ctx.view.reading();
    let bars = qp_plane::values::find_series(rd, &it.series).and_then(Series::bars);
    let entitled = qp_authority::feeds_of(ctx.view.document(), &[it.series.as_str()])
        .is_some_and(|ks| ks.iter().all(|k| ctx.view.plane().entitled(principal, k)));
    let face = entitled && bars.is_some_and(|b| !b.is_empty());
    let known = if entitled {
        usize::try_from(it.last_bar)
            .ok()
            .and_then(|b| b.checked_add(1))
            .and_then(|n| bars.and_then(|b| b.get(n)))
            .map(|b| b.t)
    } else {
        None
    };
    (face, known)
}
