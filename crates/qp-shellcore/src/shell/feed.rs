//! The feed (RG1: moved out of `shell.rs`): feed rows, local stamps and display time,
//! and the shell core's feed methods.

use super::*;

pub(super) fn lang_name(l: Language) -> &'static str {
    match l {
        Language::Tr => "tr",
        Language::En => "en",
    }
}

pub(super) fn depth_name(d: OpenDepth) -> &'static str {
    match d {
        OpenDepth::Sentence => "sentence",
        OpenDepth::Capsule => "capsule",
        OpenDepth::Chart => "chart",
        OpenDepth::Evidence => "evidence",
    }
}

/// (R4/S1) The feed rows of `items` by layout `feed` (`qp_grammar::feeds`), with the local
/// wall times the layout reads filled from the display zone: the shell core's feed and the
/// shell-less driver use the same rows. (R5/fix47) `order`: the presentation's feed order
/// (`qp_grammar::feeds::ORDERS`), which replaces the layout when it names one.
#[must_use]
pub fn feed_rows(
    order: Option<&str>,
    feed: Option<&str>,
    items: &[CapsuleItem],
    lang: Language,
    seen: Option<i64>,
    fresh: &[bool],
    now: Option<i64>,
) -> Vec<qp_grammar::feeds::FeedRow> {
    let local: Vec<qp_grammar::feeds::LocalStamp> =
        items.iter().map(|it| local_stamp(it.t)).collect();
    let ctx = qp_grammar::feeds::FeedCtx {
        items,
        lang,
        seen,
        fresh,
        now,
        local: &local,
        now_local: now.map(local_stamp),
        seen_local: seen.map(local_stamp),
    };
    match order {
        Some(o) => qp_grammar::feeds::order(o, &ctx).unwrap_or_default(),
        None => qp_grammar::feeds::layout(feed, &ctx),
    }
}

/// (R4/S1) The local wall time of `t` (ms) in the display time zone, from the zone's
/// embedded rules (`qp_calendar::Zone`); UTC if the zone were missing.
#[must_use]
pub fn local_stamp(t: i64) -> qp_grammar::feeds::LocalStamp {
    let off = qp_calendar::Zone::get(DISPLAY_TZ).map_or(0, |z| z.offset_at(t.div_euclid(1000)));
    qp_grammar::feeds::LocalStamp::at_offset(t, i64::from(off) * 1000)
}

/// `YYYY-MM-DD HH:MM` of `t` in the display time zone.
#[must_use]
pub fn display_time(t: i64) -> String {
    match qp_calendar::format_display(t, DISPLAY_TZ) {
        Ok(s) if s.len() >= 16 => format!("{} {}", &s[..10], &s[11..16]),
        Ok(s) => s,
        Err(_) => t.to_string(),
    }
}

impl ShellCore {
    /// The feed rows, top to bottom, by the presentation's feed order (`feed_order`; R5/fix47
    /// default `live_new`) or, with none (`?feed_order=layout`), the connection's feed layout
    /// (`look.feed`, `qp_grammar::feeds`; `r3`: the authority's `capsules` order): a capsule row is
    /// `{id, target, text, scene, relevant, closed, mark?}` (R4/S1 `closed`: the story is
    /// closed, its glyph shows the outcome mark) — `text` is the reader's sentence (shown
    /// as text when there is no face scene), `mark` the layout's short mark; a layout row
    /// is `{header}` (its text, written by the layout).
    pub fn feed_json(&mut self) -> Value {
        let lang = self.lang;
        let now = self.replay.as_ref().map(|r| r.t);
        let fresh: Vec<bool> = self
            .feed
            .iter()
            .map(|it| self.look.seen.is_some_and(|s| self.known_at(it) > s))
            .collect();
        let rows = if self.feed.is_empty() && self.feed_known {
            // (R5/M19, K53) The empty feed says so, and why (the core writes it).
            qp_grammar::feeds::empty_rows(lang, self.coin_flip_hidden)
        } else {
            feed_rows(
                self.feed_order.as_deref(),
                self.look.feed.as_deref(),
                &self.feed,
                lang,
                self.look.seen,
                &fresh,
                now,
            )
        };
        let out: Vec<Value> = rows
            .into_iter()
            .filter_map(|r| match r {
                qp_grammar::feeds::FeedRow::Header { text } => Some(json!({"header": text})),
                qp_grammar::feeds::FeedRow::Capsule { id, mark } => {
                    let it = self.feed.iter().find(|i| i.id == id)?;
                    // A face scene needs the reader's own bars of the capsule's series
                    // (RP3/M15: a viewer's comes ready when the reader may see them).
                    let has = match &self.viewer {
                        Some(v) => v.row(&it.id).is_some_and(|r| r.face),
                        None => self.replica.bars(&it.series).is_some_and(|b| !b.is_empty()),
                    };
                    // (R5/m17-int, A32) Without a face scene the text is the voice's text
                    // face (headline, ledger, detail; no digits for a redacted reader).
                    let text = it
                        .voice
                        .as_ref()
                        .map_or_else(|| it.sentence.text.clone(), |v| v.face.clone());
                    let mut v = json!({"id": it.id, "target": format!("face:{}", it.id),
                        "text": text, "scene": has, "relevant": it.relevant,
                        "closed": it.validity == qp_schema::capsule::Validity::Closed});
                    if let Some(m) = mark {
                        v["mark"] = json!(m);
                    }
                    // (RP3/M15, K48) The trust mark of a feed capsule: the authority's
                    // settled claim (text in the reader's language).
                    if let Some(vw) = &self.viewer {
                        v["trust"] = json!(vw.feed_trust());
                    }
                    Some(v)
                }
            })
            .collect();
        Value::Array(out)
    }

    /// (R4/H-M8910) When the reader could first know capsule `it`'s latest transition:
    /// the open time of the bar after its transition bar (the bar closed then), or — the
    /// transition being on the newest bar — `i64::MAX` (now).
    #[must_use]
    pub fn known_at(&self, it: &CapsuleItem) -> i64 {
        if let Some(v) = &self.viewer {
            return v.row(&it.id).and_then(|r| r.known_at).unwrap_or(i64::MAX);
        }
        let next = usize::try_from(it.last_bar)
            .ok()
            .and_then(|b| b.checked_add(1));
        self.replica
            .bars(&it.series)
            .and_then(|b| next.and_then(|n| b.get(n)))
            .map_or(i64::MAX, |b| b.t)
    }

    /// (R5/G) How many capsules the authority's feed rule hid from the last feed answer
    /// (`coin_flip_hidden`, K53).
    #[must_use]
    pub fn coin_flip_hidden(&self) -> u32 {
        self.coin_flip_hidden
    }

    /// (R5/G) Asks for the feed again at the next outbox (a replica core: the feed items
    /// carry the connection's voice, which a `style` change alone does not resend). The
    /// gallery's painter calls it after applying a candidate's setting.
    pub fn requery_feed(&mut self) {
        if self.viewer.is_none() {
            self.feed_dirty = true;
        }
    }

    /// (R4/H-M8910) "I looked now": the `style` gesture's `seen` at the replay clock (the
    /// feed layout may mark what changed since). Nothing without a replay state.
    pub fn mark_seen(&mut self) {
        // (R5/M21) "I looked now": the change labels go.
        self.changes_seen(None);
        let Some(t) = self.replay.as_ref().map(|r| r.t) else {
            return;
        };
        self.ask(
            "in",
            Asked::Input,
            json!({"type":"input","gesture":{"kind":"style","seen":t}}),
        );
    }
}
