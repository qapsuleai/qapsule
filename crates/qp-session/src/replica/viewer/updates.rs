//! Viewer connections on the authority: checks, start, visible messages and pushed updates.

use super::*;

impl Session {
    /// Whether any connection holds a `viewer` subscription.
    #[must_use]
    pub fn has_viewers(&self) -> bool {
        !self.viewers.is_empty()
    }

    /// Whether anything is pushed after an input: a `viewer` (§22) or a `stories` (§23)
    /// subscription.
    pub(crate) fn has_pushers(&self) -> bool {
        !self.viewers.is_empty() || self.has_stories()
    }

    /// (§22) The data subscriptions of viewer connection `conn`: the feeds its host serves
    /// it (`None`: not a viewer — such a connection gets the whole data plane).
    #[must_use]
    pub fn viewer_data(&self, conn: &str) -> Option<&[FeedKey]> {
        self.viewers.get(conn).map(|v| v.data.as_slice())
    }

    /// (§22) Viewer updates are computed by the host after each batch of data events
    /// (`true`: a replay host, once per step) instead of after each event.
    pub fn set_viewer_batch(&mut self, on: bool) {
        self.viewer_batch = on;
    }

    /// The `viewer` subscription of connection `ci` was made (`Session::subscribe`): checks
    /// it is the only one and its fields.
    pub(crate) fn viewer_check(
        &self,
        ci: usize,
        limit: Option<u32>,
        width: Option<u32>,
    ) -> crate::error::PResult<()> {
        if self.viewers.contains_key(&self.conns[ci].id) {
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

    /// The `viewer` subscription `sub` of connection `ci` is in place: remembers it and
    /// returns its first event (the feed).
    pub(crate) fn viewer_start(
        &mut self,
        ci: usize,
        sub: &str,
        limit: Option<u32>,
        width: Option<u32>,
    ) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        self.viewers.insert(
            conn,
            ViewerConn {
                sub: sub.to_owned(),
                limit: limit.unwrap_or(DEFAULT_LIMIT),
                width: width.unwrap_or(DEFAULT_FACE_WIDTH),
                ..ViewerConn::default()
            },
        );
        self.viewer_lines(ci, true)
    }

    /// A `visible` message of connection `ci`.
    pub(crate) fn visible_msg(
        &mut self,
        ci: usize,
        id: String,
        sub: &str,
        visible: Vec<String>,
    ) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        let Some(v) = self.viewers.get_mut(&conn).filter(|v| v.sub == sub) else {
            let e = E::new(
                ErrorCode::NotFound,
                format!("no `viewer` subscription `{sub}`"),
            )
            .at("/sub")
            .hint("Subscribe to {\"kind\":\"viewer\"} first.")
            .build();
            return vec![crate::session::err_line(&conn, Some(id), e)];
        };
        let mut vis = visible;
        vis.dedup();
        vis.truncate(v.limit as usize);
        v.visible = vis;
        let principal = self.conns[ci].principal.clone();
        let update = self.viewer_update(ci, false);
        vec![out(
            &conn,
            ServerMsg::Result {
                id,
                seq: self.seq_to(&principal, self.seq()),
                now: self.clock(),
                data: QueryResult::Viewer {
                    viewer: Box::new(update),
                },
                resolved: BTreeMap::new(),
            },
        )]
    }

    /// The viewer subscription was removed.
    pub(crate) fn viewer_unsubscribed(&mut self, conn: &str, sub: &str) {
        if self.viewers.get(conn).is_some_and(|v| v.sub == sub) {
            self.viewers.remove(conn);
        }
    }

    /// (§22) What changed for every viewer connection since it was last told, as `viewer`
    /// events (none when nothing changed). A host calls it after a batch of data events
    /// ([`Session::set_viewer_batch`]); the authority itself after every message.
    ///
    /// (R5/M18, §23) Then the `story` events of the `stories` subscriptions (same rhythm).
    pub fn viewer_flush(&mut self) -> Vec<OutboundLine> {
        let mut lines = Vec::new();
        if !self.viewers.is_empty() {
            let cis: Vec<usize> = (0..self.conns.len())
                .filter(|&ci| self.viewers.contains_key(&self.conns[ci].id))
                .collect();
            for ci in cis {
                lines.extend(self.viewer_lines(ci, false));
            }
        }
        if self.has_stories() {
            lines.extend(self.stories_flush());
        }
        lines
    }

    /// The `viewer` event of connection `ci` (always one when `force`).
    fn viewer_lines(&mut self, ci: usize, force: bool) -> Vec<OutboundLine> {
        let conn = self.conns[ci].id.clone();
        let principal = self.conns[ci].principal.clone();
        let before = self.viewers.get(&conn).map(|v| v.data.clone());
        let update = self.viewer_update(ci, force);
        if !force && update.is_empty() && before.as_ref() == Some(&update.data) {
            return Vec::new();
        }
        let Some(sub) = self.viewers.get(&conn).map(|v| v.sub.clone()) else {
            return Vec::new();
        };
        vec![out(
            &conn,
            ServerMsg::Notify {
                sub,
                event: Event::Viewer {
                    viewer: Box::new(update),
                    seq: self.seq_to(&principal, self.seq()),
                },
            },
        )]
    }

    /// Computes what changed for viewer connection `ci` (in the reader's world) and records
    /// it as sent. `all`: the whole feed even if unchanged.
    fn viewer_update(&mut self, ci: usize, all: bool) -> ViewerUpdate {
        let conn = self.conns[ci].id.clone();
        let Some(vc) = self.viewers.get(&conn).cloned() else {
            return ViewerUpdate::default();
        };
        let principal = self.conns[ci].principal.clone();
        let lang = self.lang_of(ci);
        let look = self.conns[ci].look.clone();
        let mp = self.conns[ci].profile.max_points;
        let open_cap = (look.depth != OpenDepth::Sentence)
            .then(|| look.capsule.clone())
            .flatten();
        let (update, vc) = self.as_reader(&principal, lang, |s| {
            s.viewer_compute(ci, vc, all, &look, mp, open_cap.as_deref())
        });
        self.viewers.insert(conn, vc);
        update
    }

    #[allow(clippy::too_many_lines)]
    /// [`Session::viewer_update`] inside the reader's world.
    fn viewer_compute(
        &mut self,
        ci: usize,
        mut vc: ViewerConn,
        all: bool,
        look: &ViewState,
        mp: u32,
        open_cap: Option<&str>,
    ) -> (ViewerUpdate, ViewerConn) {
        let mut up = ViewerUpdate::default();
        let principal = self.conns[ci].principal.clone();
        // The feed: the reader's audience, as the `capsules` query lists it.
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
            limit: Some(vc.limit),
        };
        let (caps, coin_flip_hidden) = match self.query(ci, &q) {
            Ok(QueryResult::Capsules {
                capsules,
                coin_flip_hidden,
                ..
            }) => (capsules, coin_flip_hidden),
            _ => (Vec::new(), 0),
        };
        let items: Vec<ViewerItem> = caps
            .into_iter()
            .map(|it| {
                let (face, known_at) = self.viewer_row(&principal, &it);
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
        let fh = qp_designs::faces::height_for(
            look_of(look),
            look.face.as_deref(),
            look.voice.is_some(),
        );
        let mut sent: BTreeMap<String, String> = BTreeMap::new();
        for id in &vc.visible {
            let Some(row) = items.iter().find(|r| &r.capsule.id == id && r.face) else {
                continue;
            };
            let Some(scene) = face_scene(self, &row.capsule, look, mp, vc.width, fh) else {
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
        // The reader's audience.
        let aud = self.doc.audiences.get(&principal).cloned();
        let ad = digest_of(&aud);
        if vc.audience_sent != Some(ad) {
            vc.audience_sent = Some(ad);
            up.audience = aud;
        }
        // The open capsule's slice and data subscriptions.
        let open = open_cap.and_then(|cap| {
            let c = self.doc.capsules.iter().find(|c| c.id == cap)?;
            let story = c.story.as_ref();
            let series = story.map(|s| s.series.clone()).or_else(|| {
                let f = self.doc.findings.iter().find(|f| f.id == c.finding)?;
                let fact = self.doc.facts.iter().find(|x| f.facts.contains(&x.id))?;
                Some(fact.series.clone())
            })?;
            let instrument = story.map(|s| s.instrument.clone()).or_else(|| {
                self.doc
                    .series
                    .iter()
                    .find(|s| s.id == series)
                    .and_then(|s| s.instrument.as_ref().map(|i| i.symbol.clone()))
            })?;
            let (doc, feeds) = slice(&self.doc, &principal, &instrument);
            let feeds: Vec<FeedKey> = feeds
                .into_iter()
                .filter(|k| self.plane.entitled(&principal, k))
                .collect();
            Some(ViewerOpen {
                capsule: cap.to_owned(),
                finding: c.finding.clone(),
                instrument,
                series,
                digest: document_digest(&doc),
                document: Box::new(doc),
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
        if let Some(m) = self.market_for(&principal) {
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
    fn viewer_row(&self, principal: &str, it: &CapsuleItem) -> (bool, Option<i64>) {
        let rd = rd!(self);
        let bars = crate::values::find_series(rd, &it.series).and_then(Series::bars);
        let entitled = crate::claims::feeds_of(&self.doc, &[it.series.as_str()])
            .is_some_and(|ks| ks.iter().all(|k| self.plane.entitled(principal, k)));
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

    /// (§22) The viewer subscription of connection `conn` whose open capsule rests on
    /// every feed in `feeds` (a claim about the open instrument goes there too).
    pub(crate) fn viewer_claim_sub(&self, conn: &str, feeds: &[FeedKey]) -> Option<&str> {
        let v = self.viewers.get(conn)?;
        (!feeds.is_empty() && !v.data.is_empty() && feeds.iter().all(|k| v.data.contains(k)))
            .then_some(v.sub.as_str())
    }
}
