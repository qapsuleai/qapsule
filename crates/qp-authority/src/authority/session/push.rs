//! (RF1 S3.3) Subscriptions to a registered topic and pushed topics: after every input the authority asks each topic handler that
//! pushes (the viewer, §22: `qp-viewer`) what changed for each of its connections, in the
//! connection's reader world, then the `stories` subscriptions (§23). The authority knows
//! no viewer: it keeps the batch flag, the call points and the lines.

use qp_schema::data::FeedKey;
use qp_subsystem::{Moment, Phase, TopicHandler};

use super::*;

impl Session {
    /// A subscription: the built-in checks, then its topic kind's handler (a refusal
    /// drops the subscription again: audit-s30 B5), then [`Phase::Subscribe`]. Returns the
    /// topic's handler (a pushed topic sends its first event after the ack).
    pub(super) fn dispatch_subscribe(
        &mut self,
        ci: usize,
        id: &str,
        topic: Topic,
    ) -> PResult<Option<&'static dyn TopicHandler>> {
        let h = self.registry.topic(&topic).map(|h| (h, topic.clone()));
        self.subscribe(ci, id, topic)?;
        let conn = self.conns[ci].id.clone();
        if let Some((h, t)) = &h {
            if let Err(e) = self.with_ctx(|_, ctx| h.subscribe(&conn, id, t, ctx)) {
                self.conns[ci].subs.retain(|s| s.id != id);
                return Err(e);
            }
        }
        self.phase(
            Phase::Subscribe,
            Moment::Sub {
                conn: &conn,
                sub: id,
            },
        );
        Ok(h.map(|(h, _)| h))
    }

    /// After subscription `sub` of `conn` was dropped: [`Phase::Unsubscribe`].
    pub(super) fn unsubscribed(&mut self, conn: &str, sub: &str) {
        self.phase(Phase::Unsubscribe, Moment::Sub { conn, sub });
    }

    /// Whether any connection holds a pushed subscription (a `viewer`, §22).
    #[must_use]
    pub fn has_viewers(&self) -> bool {
        self.registry
            .topic_handlers()
            .any(|h| h.pushing(&self.slots))
    }

    /// Whether anything is pushed after an input: a pushed topic (`viewer`, §22) or a
    /// `stories` (§23) subscription.
    pub(crate) fn has_pushers(&self) -> bool {
        self.has_viewers() || self.has_stories()
    }

    /// (§22) The data subscriptions of viewer connection `conn`: the feeds its host serves
    /// it (`None`: not a viewer — such a connection gets the whole data plane).
    #[must_use]
    pub fn viewer_data(&self, conn: &str) -> Option<&[FeedKey]> {
        self.registry
            .topic_handlers()
            .find_map(|h| h.data(conn, &self.slots))
    }

    /// (§22) Viewer updates are computed by the host after each batch of data events
    /// (`true`: a replay host, once per step) instead of after each event.
    pub fn set_viewer_batch(&mut self, on: bool) {
        self.viewer_batch = on;
    }

    /// (§22) What changed for every viewer connection since it was last told, as `viewer`
    /// events (none when nothing changed). A host calls it after a batch of data events
    /// ([`Session::set_viewer_batch`]); the authority itself after every message.
    ///
    /// (R5/M18, §23) Then the `story` events of the `stories` subscriptions (same rhythm).
    pub fn viewer_flush(&mut self) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        let hs: Vec<&'static dyn TopicHandler> = self
            .registry
            .topic_handlers()
            .filter(|h| h.pushing(&self.slots))
            .collect();
        for h in hs {
            for ci in 0..self.conns.len() {
                if h.pushes(&self.conns[ci].id, &self.slots) {
                    lines.extend(self.push_lines(h, ci, false));
                }
            }
        }
        if self.has_stories() {
            lines.extend(self.stories_flush());
        }
        lines
    }

    /// Topic handler `h`'s event for connection `ci`, in its reader world (`all`: its first
    /// event, after the subscription's ack).
    pub(crate) fn push_lines(
        &mut self,
        h: &'static dyn TopicHandler,
        ci: usize,
        all: bool,
    ) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let seq = self.seq_to(&principal, self.seq());
        let got = self.as_reader(&principal, lang, |s| {
            s.with_read_ctx(|_, ctx| h.push(&conn, all, seq, ctx))
        });
        got.map(|(sub, event)| out(&conn, ServerMsg::Notify { sub, event }))
            .into_iter()
            .collect()
    }

    /// A `visible` message of connection `ci` (§22.2): the topic handler that takes it
    /// records it and answers what changed (none: `unsupported`).
    pub(crate) fn dispatch_visible(
        &mut self,
        ci: usize,
        id: String,
        sub: &str,
        visible: &[String],
    ) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        let mut took = None;
        for h in self.registry.topic_handlers() {
            if let Some(r) = h.visible(&conn, sub, visible, &mut self.slots) {
                took = Some((h, r));
                break;
            }
        }
        let h = match took {
            Some((h, Ok(()))) => h,
            Some((_, Err(e))) => return vec![err_line(&conn, Some(id), e)],
            None => {
                let e = E::new(ErrorCode::Unsupported, "`visible` is not served here").build();
                return vec![err_line(&conn, Some(id), e)];
            }
        };
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let data = self.as_reader(&principal, lang, |s| {
            s.with_read_ctx(|_, ctx| h.visible_result(&conn, ctx))
        });
        let Some(data) = data else {
            return Vec::new();
        };
        vec![out(
            &conn,
            ServerMsg::Result {
                id,
                seq: self.seq_to(&principal, self.seq()),
                now: self.clock(),
                data,
                resolved: BTreeMap::new(),
            },
        )]
    }

    /// (§22) The pushed subscription of connection `conn` that also gets a claim resting
    /// on every feed in `feeds` (a viewer's open instrument).
    pub(crate) fn push_claim_sub(&self, conn: &str, feeds: &[FeedKey]) -> Option<String> {
        self.registry
            .topic_handlers()
            .find_map(|h| h.claim_sub(conn, feeds, &self.slots))
            .map(str::to_owned)
    }
}
