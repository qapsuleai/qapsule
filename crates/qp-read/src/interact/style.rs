//! Style and frame settings of a connection's surface.

use super::*;

impl Rd<'_> {
    #[allow(clippy::too_many_lines)]
    /// (R4/T5) The `style` gesture: the connection's face design, glyph alphabet and
    /// theme; (R4/H-M8910) feed layout, depth transition, outcome mark style and last look
    /// (view state; nothing is written). Validated before anything changes.
    pub(super) fn set_style(&mut self, ci: usize, r: &StyleReq<'_>) -> PResult<()> {
        let unknown = |field: &str, v: &str, known: Vec<&str>| {
            E::new(ErrorCode::Invalid, format!("unknown {field} `{v}`"))
                .at(ptr(GP, field))
                .expected(known)
                .build()
        };
        if let Some(f) = r.face {
            if !qp_designs::faces::is_face(f) {
                return Err(unknown("face", f, qp_designs::faces::names()));
            }
        }
        if let Some(a) = r.alphabet {
            if qp_glyph::alphabet_named(Some(a)).is_none() {
                return Err(
                    E::new(ErrorCode::Invalid, format!("unknown alphabet `{a}`"))
                        .at(ptr(GP, "alphabet"))
                        .expected(qp_glyph::ALPHABETS)
                        .build(),
                );
            }
        }
        if let Some(t) = r.theme {
            if t != "view" && !qp_look::theme::THEMES.contains(&t) {
                return Err(E::new(ErrorCode::Invalid, format!("unknown theme `{t}`"))
                    .at(ptr(GP, "theme"))
                    .expected(qp_look::theme::THEMES.into_iter().chain(["view"]))
                    .build());
            }
        }
        if let Some(f) = r.feed {
            if !qp_designs::feeds::is_feed(f) {
                return Err(unknown("feed", f, qp_designs::feeds::names()));
            }
        }
        if let Some(t) = r.transition {
            if !qp_designs::transitions::is_transition(t) {
                return Err(unknown("transition", t, qp_designs::transitions::names()));
            }
        }
        if let Some(o) = r.outcome {
            if qp_glyph::outcome::by_name(o).is_err() {
                return Err(unknown("outcome", o, qp_glyph::outcome::names()));
            }
        }
        if let Some(l) = r.look {
            if !qp_look::look::is_look(l) {
                return Err(unknown("look", l, qp_look::look::names()));
            }
        }
        if let Some(v) = r.voice {
            if !qp_voice::is_voice(v) {
                return Err(unknown("voice", v, qp_voice::names()));
            }
        }
        if let Some(st) = r.story {
            if !qp_designs::stories::is_story(st) {
                return Err(unknown("story", st, qp_designs::stories::names()));
            }
        }
        if let Some(o) = r.open {
            if !qp_designs::open::is_open(o) {
                return Err(unknown("open", o, qp_designs::open::names()));
            }
        }
        let look = &mut self.looks[ci];
        if let Some(o) = r.open {
            let next = (o != qp_designs::open::DEFAULT_OPEN).then(|| o.to_owned());
            // Another design: the capsule frame a design's height set no longer holds, nor
            // the question `nobra` had opened.
            if next != look.open {
                look.frames.remove("capsule");
                look.ask = None;
            }
            look.open = next;
        }
        if let Some(v) = r.voice {
            look.voice = (v != qp_voice::DEFAULT_VOICE).then(|| v.to_owned());
        }
        if let Some(st) = r.story {
            look.story = (st != qp_designs::stories::DEFAULT_STORY).then(|| st.to_owned());
        }
        if let Some(h) = r.hide_closed {
            look.hide_closed = h;
        }
        if let Some(c) = r.clock {
            look.clock = c;
        }
        if let Some(l) = r.look {
            look.look = (l != qp_look::look::DEFAULT_LOOK).then(|| l.to_owned());
        }
        if let Some(f) = r.face {
            let c = qp_designs::faces::canonical(f).unwrap_or(qp_designs::faces::DEFAULT_FACE);
            look.face = (c != qp_designs::faces::DEFAULT_FACE).then(|| c.to_owned());
        }
        if let Some(a) = r.alphabet {
            look.alphabet = (a != qp_glyph::ALPHABETS[0]).then(|| a.to_owned());
        }
        if let Some(t) = r.theme {
            look.theme = (t != "view").then(|| t.to_owned());
        }
        if let Some(f) = r.feed {
            look.feed = (f != qp_designs::feeds::DEFAULT_FEED).then(|| f.to_owned());
        }
        if let Some(t) = r.transition {
            look.transition =
                (t != qp_designs::transitions::DEFAULT_TRANSITION).then(|| t.to_owned());
        }
        if let Some(o) = r.outcome {
            look.outcome = (o != qp_glyph::outcome::DEFAULT_OUTCOME).then(|| o.to_owned());
        }
        if let Some(t) = r.seen {
            look.seen = (t != 0).then_some(t);
        }
        Ok(())
    }

    /// Records a frame size the surface reported for `key`.
    pub(super) fn set_frame(
        &mut self,
        ci: usize,
        key: &str,
        width: Option<u32>,
        height: Option<u32>,
    ) -> PResult<()> {
        if width.is_none() && height.is_none() {
            return Ok(());
        }
        let (w0, h0) = self.frame(ci, key);
        let (w, h) = (width.unwrap_or(w0), height.unwrap_or(h0));
        for (field, v, min) in [("width", w, MIN_WIDTH), ("height", h, MIN_HEIGHT)] {
            if v < min || v > MAX_DIM {
                return Err(E::new(
                    ErrorCode::Invalid,
                    format!("frame {field} {v} is outside {min}..={MAX_DIM}"),
                )
                .at(ptr(GP, field))
                .expected([format!("{min}..={MAX_DIM}")])
                .build());
            }
        }
        self.looks[ci].frames.insert(key.to_owned(), [w, h]);
        Ok(())
    }

    /// The frame of `key`: as reported, else the surface's default (`screen`'s frame;
    /// a face is that wide and [`FACE_HEIGHT`] high).
    pub(super) fn frame(&self, ci: usize, key: &str) -> (u32, u32) {
        let c = &self.conns[ci];
        // (R5/M20) An open capsule design that states its height opens the capsule depth
        // at it, whichever frame the surface reported (a tap on a face opens it too).
        if key == "capsule" {
            let look = look_of(&c.look);
            if let Some(h) = qp_designs::open::height_in(look, c.look.open.as_deref()) {
                let w = c
                    .look
                    .frames
                    .get(key)
                    .map_or_else(|| screen_frame(c.surface.kind).0, |f| f[0]);
                return (w, h);
            }
        }
        if let Some([w, h]) = c.look.frames.get(key) {
            return (*w, *h);
        }
        let (w, h) = screen_frame(c.surface.kind);
        if key == "face" {
            (w, FACE_HEIGHT.max(MIN_HEIGHT))
        } else {
            (w, h)
        }
    }
}
