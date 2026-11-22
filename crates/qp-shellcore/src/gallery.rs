//! (R5/G) The comparison gallery (`rounds/R5/GOAL.md` criterion 11; VERIFY 15, 18): every
//! topic's candidates one under the other, with the same content (the same synthetic
//! world, the same replay moment, the same open capsule and depth), each painted by the
//! core with its own setting; under each an "I'd pick this" choice (one per topic) and a
//! short note; the result copied in one tap as a deterministic summary text.
//!
//! Two halves, one module:
//! - [`build`] (native only; `qp demo --gallery`): drives one [`ShellCore`] per candidate
//!   against an in-process synthetic, paused [`qp_session::ReplayHost`] (as the shell-less
//!   driver does), applies the candidate's setting and paints the topic's shots with the
//!   reference painter. Output: a [`Layout`] (JSON for the page) and the PNG images it
//!   names.
//! - [`GalleryCore`] (native and wasm; the page's core, `crate::Gallery`): holds the
//!   layout, the picks and notes, the language and theme; writes every text the page shows
//!   (labels, the summary, the share link) and the state the page keeps in local storage.
//!   The page (`shells/web/gallery.js`) only paints what it is given and passes taps and
//!   typed notes on (`shells/web/ARCH.md`).
//!
//! The topics are [`TOPICS`]; their candidates are the registries' names
//! ([`crate::shell::ShellCore::choices_json`]), narrowed by [`FINALISTS`] — the one place
//! the orchestrator updates when finalists are chosen. A topic whose registry key does
//! not exist yet (story chart, motion, open capsule before their rounds merge) is left out
//! until it does.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// RG1: the page texts moved to `gallery/labels.rs`.
mod labels;

pub use labels::{clean_note, device_label};
use labels::{labels, norm_lang, norm_theme};

/// What a candidate shows (the topic's depth).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shot {
    /// The feed's first `n` rows (group headers as text, faces as images).
    Feed(usize),
    /// The open capsule's face in the feed.
    Face,
    /// The open capsule at capsule depth (its title and scene).
    Capsule,
    /// The open capsule at chart depth.
    Chart,
    /// The open capsule at evidence depth (scene and evidence rows).
    Evidence,
}

/// One topic of the gallery.
#[derive(Debug, Clone, Copy)]
pub struct Topic {
    /// The setting's key: the `choices()` key and the `set_look` field.
    pub key: &'static str,
    /// Title (Turkish, English).
    pub label: [&'static str; 2],
    /// What to look at (Turkish, English).
    pub hint: [&'static str; 2],
    /// What each candidate shows.
    pub shots: &'static [Shot],
}

/// The gallery's topics, in page order (`rounds/R5/GOAL.md` M16–M21).
pub const TOPICS: &[Topic] = &[
    Topic {
        key: "voice",
        label: ["Dil", "Voice"],
        hint: [
            "Aynı kapsülün yüzü ve açılmış kartı. Ne olduğunu, hangi eşiği ve geçmişte kaç kez tuttuğunu bir bakışta anlıyor musun?",
            "The same capsule's face and open card. Can you tell at a glance what happened, which level, and how often it held before?",
        ],
        shots: &[Shot::Face, Shot::Capsule],
    },
    Topic {
        key: "look",
        label: ["Görünüş", "Look"],
        hint: [
            "Aynı yüz, açılmış kapsül ve grafik. İşaret ilk göze çarpan şey mi, yazı rahat okunuyor mu?",
            "The same face, open capsule and chart. Is the mark the first thing you see, is the text easy to read?",
        ],
        shots: &[Shot::Face, Shot::Capsule, Shot::Chart],
    },
    Topic {
        key: "story",
        label: ["Hikâye çizelgesi", "Story chart"],
        hint: [
            "Aynı hissenin grafiği. Hangi kapsül canlı, hangileri tuttu?",
            "The same instrument's chart. Which capsule is live, which ones held?",
        ],
        shots: &[Shot::Chart],
    },
    Topic {
        key: "feed_order",
        label: ["Akış sırası", "Feed order"],
        hint: [
            "Aynı anın akışı. Son baktığından beri ne değiştiğini ilk satırlarda görüyor musun?",
            "The same moment's feed. Do the first rows show what changed since you last looked?",
        ],
        shots: &[Shot::Feed(6)],
    },
    Topic {
        key: "feed_rule",
        label: ["Yazı tura eşiği", "Coin-flip threshold"],
        hint: [
            "Aynı anın akışı, karnesi yazı tura olanları gizleyen ve gizlemeyen kural.",
            "The same moment's feed, with and without hiding capsules whose record is a coin flip.",
        ],
        shots: &[Shot::Feed(6)],
    },
    Topic {
        key: "motion",
        label: ["Hareket", "Motion"],
        hint: [
            "Açılmış kapsülün son karesi (hareketin kendisi gösterimde denenir).",
            "The open capsule's last frame (the motion itself is tried in the demo).",
        ],
        shots: &[Shot::Face, Shot::Capsule],
    },
    Topic {
        key: "open",
        label: ["Açılmış kapsül", "Open capsule"],
        hint: [
            "Aynı kapsül açılmış ve kanıtıyla. Bu iddia neye dayanıyor, kim doğruladı?",
            "The same capsule open and with its evidence. What does the claim rest on, who checked it?",
        ],
        shots: &[Shot::Capsule, Shot::Evidence],
    },
];

/// The finalists the gallery shows per topic key, best first (R5: "each topic's first two
/// candidates"). A topic not listed here shows every candidate its registry has. **The
/// orchestrator updates this one table** when a topic's finalists are chosen; a name the
/// registry does not know is skipped.
/// (R5/S) Each R5 topic's first two (best first) and the `r4` reference.
pub const FINALISTS: &[(&str, &[&str])] = &[
    ("voice", &["marla", "selvi", "r4"]),
    ("look", &["oltan", "zemre", "r4"]),
    ("story", &["pavka", "korda", "r4"]),
    ("open", &["tesir", "nobra", "r4"]),
    ("motion", &["surek", "nabiz", "r4"]),
];

/// The replay step every candidate is shown at (synthetic, paused host).
pub const STEP: u64 = 120;
/// The principal the gallery reads as (the demo's owner).
pub const PRINCIPAL: &str = "arkin";
/// The gallery's page width (CSS px): the phone the comparison is made on.
pub const PAGE_W: u32 = 402;
/// The device scale the gallery paints its images at (sharp on a phone).
pub const IMAGE_SCALE: f64 = 2.0;
/// The longest note kept (characters).
pub const NOTE_MAX: usize = 200;

/// The candidates shown for `topic`: the registry's names (`choices[topic.key]`) narrowed
/// to the finalists (in the finalists' order) when the topic has them. Empty when the
/// registry has no such key.
#[must_use]
pub fn candidates(topic: &Topic, choices: &Value) -> Vec<String> {
    let names: Vec<String> = choices[topic.key]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    match FINALISTS.iter().find(|(k, _)| *k == topic.key) {
        Some((_, f)) => f
            .iter()
            .filter(|n| names.iter().any(|m| m == *n))
            .map(|n| (*n).to_owned())
            .collect(),
        None => names,
    }
}

/// One painted piece of a candidate: an image the host serves, or a text row as the core
/// wrote it (a feed group header, a face without a scene, the open layer's title, an
/// evidence row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShotOut {
    /// `img`, `head` (a header or title) or `text`.
    pub kind: String,
    /// The image's address (same origin).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    /// The image's CSS size `[w, h]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css: Option<[u32; 2]>,
    /// The text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// One candidate of a topic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandOut {
    /// Registry name.
    pub id: String,
    /// What the page shows as its name.
    pub label: String,
    /// The content it shows, read back from its own core: replay step and time, the open
    /// capsule and depth (the same for every candidate of a topic; the test compares it).
    pub content: String,
    /// What it shows, in order.
    pub shots: Vec<ShotOut>,
}

/// One topic as laid out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicOut {
    /// Setting key.
    pub key: String,
    /// Title.
    pub label: String,
    /// What to look at.
    pub hint: String,
    /// The candidates, one under the other.
    pub cands: Vec<CandOut>,
}

/// The gallery as the host built it for one language and theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    /// `tr` or `en`.
    pub lang: String,
    /// `dark` or `light`.
    pub theme: String,
    /// The replay moment, formatted (`step / steps · time`).
    pub moment: String,
    /// The topics.
    pub topics: Vec<TopicOut>,
}

fn tr(lang: &str) -> bool {
    lang != "en"
}

/// A candidate's name as the page shows it (today's design is marked as such).
#[must_use]
pub fn cand_label(topic: &str, id: &str, lang: &str) -> String {
    let note = match (topic, id) {
        (_, "r4") => Some(["bugünkü", "today's"]),
        ("feed_order", "layout") => Some(["düzenin kendi sırası", "the layout's own order"]),
        ("feed_rule", "none") => Some(["gizleme yok", "no hiding"]),
        _ => None,
    };
    match note {
        Some(n) => format!("{id} · {}", if tr(lang) { n[0] } else { n[1] }),
        None => id.to_owned(),
    }
}

// --- the page's core ------------------------------------------------------------------

/// What the page keeps in local storage (and gives back on the next visit).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Saved {
    #[serde(default)]
    lang: String,
    #[serde(default)]
    theme: String,
    /// Topic key -> candidate.
    #[serde(default)]
    picks: BTreeMap<String, String>,
    /// `topic/candidate` -> note.
    #[serde(default)]
    notes: BTreeMap<String, String>,
}

/// The gallery page's core: layout, picks, notes, language and theme, and every text the
/// page shows. Deterministic: the same layout, picks, notes, device and clock give the same
/// summary.
#[derive(Debug, Clone, Default)]
pub struct GalleryCore {
    saved: Saved,
    layout: Option<Layout>,
    pending_pick: String,
    device: String,
    now: Option<i64>,
}

impl GalleryCore {
    /// From the page's saved state (`""`: none) and its address parameters (`lang`,
    /// `theme`, `pick`, `now`; `""`: absent). Address parameters win over the saved state.
    #[must_use]
    pub fn new(saved: &str, lang: &str, theme: &str, pick: &str, now: &str) -> Self {
        let mut s: Saved = serde_json::from_str(saved).unwrap_or_default();
        if let Some(l) = norm_lang(lang) {
            l.clone_into(&mut s.lang);
        }
        if let Some(t) = norm_theme(theme) {
            t.clone_into(&mut s.theme);
        }
        if norm_lang(&s.lang).is_none() {
            "tr".clone_into(&mut s.lang);
        }
        if norm_theme(&s.theme).is_none() {
            "dark".clone_into(&mut s.theme);
        }
        s.notes = s
            .notes
            .into_iter()
            .map(|(k, v)| (k, clean_note(&v)))
            .filter(|(_, v)| !v.is_empty())
            .collect();
        Self {
            saved: s,
            layout: None,
            pending_pick: pick.to_owned(),
            device: String::new(),
            now: now.parse::<i64>().ok(),
        }
    }

    /// `tr` or `en`.
    #[must_use]
    pub fn lang(&self) -> &str {
        &self.saved.lang
    }

    /// `dark` or `light`.
    #[must_use]
    pub fn theme(&self) -> &str {
        &self.saved.theme
    }

    /// Switches the language (Türkçe ↔ English); the page loads the layout again.
    pub fn toggle_lang(&mut self) {
        let l = if self.saved.lang == "en" { "tr" } else { "en" };
        l.clone_into(&mut self.saved.lang);
    }

    /// Sets the theme (`dark`, `light`; anything else: unchanged).
    pub fn set_theme(&mut self, theme: &str) {
        if let Some(t) = norm_theme(theme) {
            t.clone_into(&mut self.saved.theme);
        }
    }

    /// The address of the layout for the current language and theme.
    #[must_use]
    pub fn layout_url(&self) -> String {
        format!(
            "/gallery/layout?lang={}&theme={}",
            self.saved.lang, self.saved.theme
        )
    }

    /// The device line of the summary, from the page's raw values.
    pub fn set_device(&mut self, ua: &str, w: f64, h: f64, dpr: f64) {
        self.device = device_label(ua, w, h, dpr);
    }

    /// Whether the clock of the summary's date line was given (`now`).
    #[must_use]
    pub fn has_now(&self) -> bool {
        self.now.is_some()
    }

    /// [`Self::summary`] with the date line at `ms` (the device's clock) unless `now` was
    /// given.
    #[must_use]
    pub fn summary_at(&self, ms: i64) -> String {
        let mut c = self.clone();
        c.now = Some(self.now.unwrap_or(ms));
        c.summary()
    }

    /// Takes the host's layout; picks and notes of candidates it does not show are kept
    /// (another language's layout shows the same candidates). Applies a pending `?pick=`.
    ///
    /// # Errors
    /// A malformed layout.
    pub fn load(&mut self, layout_json: &str) -> Result<(), String> {
        let l: Layout = serde_json::from_str(layout_json).map_err(|e| e.to_string())?;
        self.layout = Some(l);
        let pending = std::mem::take(&mut self.pending_pick);
        self.apply_pick(&pending);
        Ok(())
    }

    fn has(&self, topic: &str, cand: &str) -> bool {
        self.layout.as_ref().is_some_and(|l| {
            l.topics
                .iter()
                .any(|t| t.key == topic && t.cands.iter().any(|c| c.id == cand))
        })
    }

    /// `?pick=voice.marla,look.r4`: picks known candidates (others are ignored).
    pub fn apply_pick(&mut self, list: &str) {
        for item in list.split(',') {
            if let Some((t, c)) = item.split_once('.') {
                if self.has(t, c) {
                    self.saved.picks.insert(t.to_owned(), c.to_owned());
                }
            }
        }
    }

    /// "I'd pick this" on `cand` of `topic`: one pick per topic; a second tap on the picked
    /// candidate takes the pick back.
    pub fn pick(&mut self, topic: &str, cand: &str) {
        if !self.has(topic, cand) {
            return;
        }
        if self.saved.picks.get(topic).is_some_and(|c| c == cand) {
            self.saved.picks.remove(topic);
        } else {
            self.saved.picks.insert(topic.to_owned(), cand.to_owned());
        }
    }

    /// The note on `cand` of `topic` (cleaned: one line, at most [`NOTE_MAX`] characters).
    pub fn note(&mut self, topic: &str, cand: &str, text: &str) {
        if !self.has(topic, cand) {
            return;
        }
        let key = format!("{topic}/{cand}");
        let n = clean_note(text);
        if n.is_empty() {
            self.saved.notes.remove(&key);
        } else {
            self.saved.notes.insert(key, n);
        }
    }

    /// `{"topic/cand": picked}` for every candidate shown.
    #[must_use]
    pub fn marks(&self) -> Value {
        let mut m = serde_json::Map::new();
        if let Some(l) = &self.layout {
            for t in &l.topics {
                for c in &t.cands {
                    let on = self.saved.picks.get(&t.key).is_some_and(|p| *p == c.id);
                    m.insert(format!("{}/{}", t.key, c.id), json!(on));
                }
            }
        }
        Value::Object(m)
    }

    /// Everything the page paints: `{labels, lang, theme, moment, topics:[{key, label,
    /// hint, content, cands:[{key, topic, id, label, content, shots, picked, note}]}]}`
    /// (`moment`/`topics` empty before a layout is loaded).
    #[must_use]
    pub fn view(&self) -> Value {
        let lang = self.lang();
        let lb = labels(lang);
        let Some(l) = &self.layout else {
            return json!({"labels": lb, "lang": lang, "theme": self.theme(),
                          "moment": "", "topics": []});
        };
        let topics: Vec<Value> = l
            .topics
            .iter()
            .map(|t| {
                let content = t
                    .cands
                    .first()
                    .map(|c| c.content.clone())
                    .unwrap_or_default();
                let cands: Vec<Value> = t
                    .cands
                    .iter()
                    .map(|c| {
                        let key = format!("{}/{}", t.key, c.id);
                        json!({
                            "key": key, "topic": t.key, "id": c.id, "label": c.label,
                            "content": c.content, "shots": c.shots,
                            "picked": self.saved.picks.get(&t.key).is_some_and(|p| *p == c.id),
                            "note": self.saved.notes.get(&key).cloned().unwrap_or_default(),
                        })
                    })
                    .collect();
                json!({"key": t.key, "label": t.label, "hint": t.hint,
                       "content": format!("{}: {content}", lb["moment"].as_str().unwrap_or("")),
                       "cands": cands})
            })
            .collect();
        json!({"labels": lb, "lang": lang, "theme": self.theme(),
               "moment": l.moment, "topics": topics})
    }

    /// The summary to copy: one line of picks (with the picked candidate's note), one of
    /// the other notes (if any), the moment and the device/date line.
    #[must_use]
    pub fn summary(&self) -> String {
        let en = !tr(self.lang());
        let (title, note_w, notes_w, moment_w, device_w, date_w, none) = if en {
            (
                "R5 gallery",
                "note",
                "notes",
                "moment",
                "device",
                "date",
                "—",
            )
        } else {
            ("R5 galeri", "not", "notlar", "an", "cihaz", "tarih", "—")
        };
        let mut parts = vec![title.to_owned()];
        let mut others = Vec::new();
        let mut moment = String::new();
        if let Some(l) = &self.layout {
            moment.clone_from(&l.moment);
            for t in &l.topics {
                let pick = self.saved.picks.get(&t.key);
                let mut p = format!("{}={}", t.key, pick.map_or(none, String::as_str));
                if let Some(n) = pick.and_then(|c| self.saved.notes.get(&format!("{}/{c}", t.key)))
                {
                    p.push_str(&format!(" ({note_w}: {n})"));
                }
                parts.push(p);
                for c in &t.cands {
                    if pick.is_some_and(|p| *p == c.id) {
                        continue;
                    }
                    if let Some(n) = self.saved.notes.get(&format!("{}/{}", t.key, c.id)) {
                        others.push(format!("{}/{}: {n}", t.key, c.id));
                    }
                }
            }
        }
        let mut out = parts.join(" · ");
        if !others.is_empty() {
            out.push_str(&format!("\n{notes_w}: {}", others.join(" · ")));
        }
        let theme = match (self.theme(), en) {
            ("light", false) => "açık",
            ("light", true) => "light",
            (_, false) => "koyu",
            (_, true) => "dark",
        };
        out.push_str(&format!(
            "\n{moment_w}: {moment} · {} · {theme}",
            self.lang()
        ));
        let date = self.now.map_or_else(
            || none.to_owned(),
            |t| format!("{} (İstanbul)", crate::shell::display_time(t)),
        );
        let device = if self.device.is_empty() {
            none
        } else {
            self.device.as_str()
        };
        out.push_str(&format!("\n{device_w}: {device} · {date_w}: {date}"));
        out
    }

    /// A relative link that opens the gallery with the same picks (no notes).
    #[must_use]
    pub fn share(&self) -> String {
        let picks: Vec<String> = self
            .saved
            .picks
            .iter()
            .map(|(t, c)| format!("{t}.{c}"))
            .collect();
        if picks.is_empty() {
            return "gallery.html".to_owned();
        }
        format!("gallery.html?pick={}", picks.join(","))
    }

    /// The state the page keeps in local storage.
    #[must_use]
    pub fn state(&self) -> String {
        serde_json::to_string(&self.saved).unwrap_or_default()
    }

    /// The chrome's CSS custom properties (`{name: value}`): the default look in the
    /// current theme, and the page width.
    #[must_use]
    pub fn css(&self) -> Value {
        let look = qp_grammar::look::or_default(None);
        let mut m = serde_json::Map::new();
        for (k, v) in qp_grammar::look::css_vars(look, Some(self.theme())) {
            m.insert(k, json!(v));
        }
        m.insert("--qp-gallery-w".to_owned(), json!(format!("{PAGE_W}px")));
        Value::Object(m)
    }
}

// --- the host's half: painting the candidates ------------------------------------------

/// (native) The gallery's layout and the images it names (`file name -> PNG`).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct Build {
    /// The layout (as the page loads it).
    pub layout: Layout,
    /// The images, by file name (`/gallery/img/<name>`).
    pub images: BTreeMap<String, Vec<u8>>,
}

#[cfg(not(target_arch = "wasm32"))]
pub use build_impl::build;

#[cfg(not(target_arch = "wasm32"))]
mod build_impl;

#[cfg(test)]
mod tests;
