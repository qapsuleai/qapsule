//! (R5/M21) Motion of the feed's faces: the connection's motion design
//! (`qp_grammar::transitions::MOTIONS`, `?motion=<name>`, Settings → Motion) and the
//! device's reduce-motion flag.
//!
//! A motion design with live marks (`nabiz`) gets from here:
//! - the **still marks** drawn into each face scene (live dot, change label; the same with
//!   and without reduce motion, so no fact depends on motion);
//! - the **state changes** the device sees between two feeds (born, strengthened,
//!   weakened, invalid, closed), each as one beat with its start time;
//! - the **overlay** of a display frame ([`ShellCore::motion_marks`]): per face on screen,
//!   the boxes the design wants (pulse, beat, trail) as CSS values; the page only creates
//!   one box element per key and applies the values as they are (no arithmetic there).
//!
//! The page tells the core the reader's reduce-motion preference
//! (`prefers-reduced-motion`, or `?anim=0`) with [`ShellCore::set_reduced_motion`]; under
//! it the overlay is empty and a depth transition ends at once. A design without live marks
//! (the default `r4`) changes nothing: faces, digests and goldens stay byte-identical.

use std::collections::BTreeMap;

use qp_grammar::transitions::nabiz::{self, Beat, Change, FaceMarks, FaceMotion, Places};
use qp_schema::capsule::{Stage, Validity};
use qp_schema::protocol::Language;
use qp_schema::scene::{Rgba, SceneDesc};
use serde_json::{json, Value};

use super::ShellCore;

/// (R5/M21) The device-side motion state of a shell core.
#[derive(Debug, Default)]
pub struct MotionState {
    /// The motion design (`None`: the default, `r4`).
    pub name: Option<String>,
    /// Reduce motion (the page's `prefers-reduced-motion` or `?anim=0`).
    pub reduced: bool,
    /// What the device last saw of each capsule (stage, validity); `None` until the first
    /// feed (that feed is the baseline: no beats for what was already there).
    seen: Option<BTreeMap<String, (Stage, Validity)>>,
    /// The newest transition time among the capsules seen so far (a capsule first seen
    /// later counts as born only when its transition is newer).
    t_max: i64,
    /// Running beats by capsule: the change and its start (`None`: starts at the next
    /// frame the overlay is asked for).
    beats: BTreeMap<String, (Change, Option<f64>)>,
    /// Changes the device saw since the reader's last look (the change label stays until
    /// the next look): the state it changed to, by capsule.
    changed: BTreeMap<String, (Stage, Validity)>,
    /// The last look the `changed` set belongs to.
    changed_since: Option<i64>,
    /// The places of each face's marks and its label's width, from its last build.
    places: BTreeMap<String, (Places, i32)>,
    /// The last overlay handed out was not empty.
    shown: bool,
    /// The pulse step of the last overlay handed out (pulse-only frames are built once per
    /// [`PULSE_STEP_MS`]; a beat or trail is built every frame).
    last_step: Option<i64>,
}

/// (R5/M21) Pulse-only frames are built at most once per this many ms: the pulse is slow
/// (2.6 s), 20 steps a second keep it smooth while the core's work per display frame drops
/// (measured: one overlay with two live faces ≈ 0.43 ms on the JIT-less phone setting).
pub const PULSE_STEP_MS: f64 = 50.0;

fn css_color(c: Rgba, opacity: f64) -> String {
    let a = f64::from(c[3]) / 255.0 * opacity;
    format!(
        "rgba({}, {}, {}, {})",
        c[0],
        c[1],
        c[2],
        (a * 1000.0).round() / 1000.0
    )
}

fn px(v: f64) -> String {
    format!("{}px", (v * 100.0).round() / 100.0)
}

impl ShellCore {
    fn motion_design(&self) -> &'static qp_grammar::transitions::MotionDesign {
        qp_grammar::transitions::motion(self.motion.name.as_deref())
    }

    /// (R5/M21) Chooses motion design `name` (unknown: ignored). Returns the transition the
    /// design asks for (the caller adds it to the `style` gesture), if any.
    pub fn set_motion(&mut self, name: &str) -> Option<&'static str> {
        let d = qp_grammar::transitions::MOTIONS
            .iter()
            .find(|m| m.name == name)?;
        self.motion.name =
            (d.name != qp_grammar::transitions::DEFAULT_MOTION).then(|| d.name.to_owned());
        self.motion.places.clear();
        self.motion.beats.clear();
        self.motion.changed.clear();
        self.motion.seen = None;
        // The faces carry (or drop) the still marks: the page repaints the feed.
        self.pending.insert("feed".to_owned(), json!(true));
        d.transition
    }

    /// (R5/M21) The motion design's name (`None`: the default).
    #[must_use]
    pub fn motion_name(&self) -> Option<&str> {
        self.motion.name.as_deref()
    }

    /// (R5/M21) The reader's reduce-motion preference, as the page reports it.
    pub fn set_reduced_motion(&mut self, on: bool) {
        self.motion.reduced = on;
    }

    /// (R5/M21) Whether motion is reduced.
    #[must_use]
    pub fn reduced_motion(&self) -> bool {
        self.motion.reduced
    }

    /// Whether the latest transition of `it` is new to the reader: the device saw it
    /// happen since the reader's last look (or since the page opened), or — with a last
    /// look — the reader could first know it after that look (the feed's own rule).
    fn new_to_reader(&mut self, it: &qp_schema::protocol::CapsuleItem) -> bool {
        if self.motion.changed_since != self.look.seen {
            // A new last look: what was new before it is seen now.
            self.motion.changed.clear();
            self.motion.changed_since = self.look.seen;
        }
        self.motion.changed.get(&it.id) == Some(&(it.stage, it.validity))
            || self.look.seen.is_some_and(|s| {
                // "Now" (the newest bar) was in view when the reader looked.
                let k = self.known_at(it);
                k != i64::MAX && k > s
            })
    }

    /// (R5/M21) After a feed answer: the state changes the device sees (one beat each;
    /// the change label until the reader's next look). The first feed is the baseline; a
    /// capsule first seen later counts as born only when its transition is newer than
    /// anything seen before (not when a filter merely lets an older one in).
    pub(crate) fn observe_feed(&mut self) {
        if self.motion_design().marks.is_none() {
            self.motion.seen = None;
            self.motion.beats.clear();
            self.motion.changed.clear();
            return;
        }
        let now: BTreeMap<String, (Stage, Validity)> = self
            .feed
            .iter()
            .map(|it| (it.id.clone(), (it.stage, it.validity)))
            .collect();
        if let Some(before) = self.motion.seen.take() {
            for it in &self.feed {
                let b = before.get(&it.id).copied();
                if b.is_none() && it.t <= self.motion.t_max {
                    continue;
                }
                if let Some(c) = Change::between(b, (it.stage, it.validity)) {
                    self.motion.beats.insert(it.id.clone(), (c, None));
                    self.motion
                        .changed
                        .insert(it.id.clone(), (it.stage, it.validity));
                }
            }
        }
        if let Some(t) = self.feed.iter().map(|it| it.t).max() {
            self.motion.t_max = self.motion.t_max.max(t);
        }
        self.motion.beats.retain(|id, _| now.contains_key(id));
        self.motion.places.retain(|id, _| now.contains_key(id));
        self.motion.seen = Some(now);
    }

    /// (R5/M21) Draws the motion design's still marks into capsule `id`'s face scene
    /// (nothing under a design without live marks) and keeps their places for the overlay.
    pub(crate) fn decorate_face(&mut self, id: &str, scene: &mut SceneDesc) {
        let Some(m) = self.motion_design().marks else {
            return;
        };
        let Some(it) = self.feed.iter().find(|i| i.id == id).cloned() else {
            return;
        };
        let look = qp_session::viewer::look_of(&self.look);
        let theme = look.theme(self.look.theme.as_deref()).unwrap_or(look.dark);
        let lang: Language = self.lang;
        let label = if self.new_to_reader(&it) {
            nabiz::label(it.stage, it.validity, lang)
        } else {
            None
        };
        let marks = FaceMarks {
            live: nabiz::is_live(it.stage, it.validity),
            label,
            change: Change::of(it.stage, it.validity),
            direction: it.direction,
        };
        if let Some(pl) = (m.places)(scene, look) {
            let w = label.map_or(0, |t| nabiz::label_width(t, look));
            self.motion.places.insert(id.to_owned(), (pl, w));
        }
        (m.draw_face)(scene, &marks, look, &theme);
    }

    /// (R5/M21) The overlay of the display frame at page time `t` (ms): `{"marks": [{face,
    /// key, style}]}` — per face on screen the motion design's boxes (pulse, beat, trail)
    /// with their CSS values (`left`, `top`, `width`, `height` in face px, `opacity`,
    /// `transform`, `border`, `borderRadius`, `background`). `None` when there is nothing
    /// to change (no live marks, reduce motion, or nothing showed and nothing shows). A
    /// beat's clock starts at the first frame after it was seen.
    pub fn motion_marks(&mut self, t: f64) -> Option<Value> {
        let design = self.motion_design();
        let marks = design
            .marks
            .filter(|_| !self.motion.reduced && t.is_finite());
        let Some(m) = marks else {
            self.motion.beats.clear();
            return std::mem::take(&mut self.motion.shown).then(|| json!({"marks": []}));
        };
        let look = qp_session::viewer::look_of(&self.look);
        let theme = look.theme(self.look.theme.as_deref()).unwrap_or(look.dark);
        for b in self.motion.beats.values_mut() {
            b.1.get_or_insert(t);
        }
        self.motion
            .beats
            .retain(|_, (_, t0)| nabiz::beat_running(t - t0.unwrap_or(t), look));
        // A frame with no beat running changes only the pulse: built once per step.
        #[allow(clippy::cast_possible_truncation)]
        let step = (t / PULSE_STEP_MS).floor() as i64;
        let pulse_only = self.motion.beats.is_empty();
        if pulse_only && self.motion.last_step == Some(step) {
            return None;
        }
        self.motion.last_step = pulse_only.then_some(step);
        let covered = self.opening.is_some();
        let mut out = Vec::new();
        if !covered {
            for it in &self.feed {
                if !self.shown.contains(&it.id) {
                    continue;
                }
                let Some(&(places, label_w)) = self.motion.places.get(&it.id) else {
                    continue;
                };
                let live = nabiz::is_live(it.stage, it.validity);
                let beat = self.motion.beats.get(&it.id).map(|(c, t0)| Beat {
                    change: *c,
                    age: t - t0.unwrap_or(t),
                });
                if !live && beat.is_none() {
                    continue;
                }
                let f = FaceMotion {
                    places,
                    live,
                    direction: it.direction,
                    beat,
                    label_w,
                    t,
                };
                for b in (m.overlay)(&f, look, &theme) {
                    let mut style = json!({
                        "left": px(b.rect[0]), "top": px(b.rect[1]),
                        "width": px(b.rect[2]), "height": px(b.rect[3]),
                        "opacity": format!("{}", (b.opacity.clamp(0.0, 1.0) * 1000.0).round() / 1000.0),
                        "transform": format!("scale({})", (b.scale * 1000.0).round() / 1000.0),
                        "border": "none", "background": "transparent", "borderRadius": "0",
                    });
                    if let Some((c, w)) = b.ring {
                        style["border"] = json!(format!("{} solid {}", px(w), css_color(c, 1.0)));
                        style["borderRadius"] = json!("50%");
                    }
                    if let Some((c, r)) = b.fill {
                        style["background"] = json!(css_color(c, 1.0));
                        style["borderRadius"] = json!(px(r));
                    }
                    out.push(json!({"face": it.id, "key": format!("{}:{}", it.id, b.key),
                                    "style": style}));
                }
            }
        }
        if out.is_empty() && !self.motion.shown {
            return None;
        }
        self.motion.shown = !out.is_empty();
        Some(json!({ "marks": out }))
    }
}
