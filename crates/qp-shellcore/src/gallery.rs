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

fn norm_lang(s: &str) -> Option<&'static str> {
    match s {
        "tr" => Some("tr"),
        "en" => Some("en"),
        _ => None,
    }
}

fn norm_theme(s: &str) -> Option<&'static str> {
    match s {
        "dark" => Some("dark"),
        "light" => Some("light"),
        _ => None,
    }
}

/// A note as kept: one line, trimmed, at most [`NOTE_MAX`] characters.
#[must_use]
pub fn clean_note(s: &str) -> String {
    let one: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let words: Vec<&str> = one.split_whitespace().collect();
    words.join(" ").chars().take(NOTE_MAX).collect()
}

/// A short device description from a user agent and the viewport (`iPhone iOS 27.0 ·
/// Safari · 402×874 @3`).
#[must_use]
pub fn device_label(ua: &str, w: f64, h: f64, dpr: f64) -> String {
    let os = if let Some(i) = ua.find("iPhone OS ").or_else(|| ua.find("CPU OS ")) {
        let rest = &ua[i..];
        let start = rest.find("OS ").map_or(0, |k| k + 3);
        let v: String = rest[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '_')
            .collect();
        let dev = if ua.contains("iPad") {
            "iPad"
        } else {
            "iPhone"
        };
        format!("{dev} iOS {}", v.replace('_', "."))
    } else if let Some(i) = ua.find("Android ") {
        let v: String = ua[i + 8..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        format!("Android {v}")
    } else if ua.contains("Macintosh") {
        "Mac".to_owned()
    } else if ua.contains("Windows") {
        "Windows".to_owned()
    } else if ua.contains("Linux") {
        "Linux".to_owned()
    } else {
        "?".to_owned()
    };
    let browser = if ua.contains("HeadlessChrome") {
        "HeadlessChrome"
    } else if ua.contains("CriOS") || ua.contains("Chrome/") {
        "Chrome"
    } else if ua.contains("FxiOS") || ua.contains("Firefox/") {
        "Firefox"
    } else if ua.contains("Safari/") {
        "Safari"
    } else {
        "?"
    };
    let n = |x: f64| {
        if x.is_finite() && x >= 0.0 {
            format!("{x}")
        } else {
            "?".to_owned()
        }
    };
    format!("{os} · {browser} · {}×{} @{}", n(w), n(h), n(dpr))
}

/// The labels of the page's chrome, in `lang`.
fn labels(lang: &str) -> Value {
    if tr(lang) {
        json!({
            "title": "R5 galeri",
            "intro": "Her konuda adaylar alt alta, aynı içerikle. Konu başına birini seç, istersen kısa not yaz; en altta özeti tek dokunuşla kopyala.",
            "pick": "Bunu seçerdim",
            "picked": "Seçtin",
            "note": "Kısa not",
            "copy": "Özeti kopyala",
            "copied": "Kopyalandı.",
            "select": "Pano kullanılamadı: metin seçildi, elle kopyala.",
            "summary": "Özet",
            "share": "Seçimlerin bağlantısı",
            "lang": "English",
            "theme": "Tema",
            "dark": "Koyu tema",
            "light": "Açık tema",
            "moment": "Aynı içerik",
            "loading": "Galeri hazırlanıyor (ilk açılış birkaç saniye sürebilir)…",
            "missing": "Galeri kapalı. Bilgisayarda: scripts/demo.sh --gallery",
            "empty": "Henüz karşılaştırılacak aday yok.",
        })
    } else {
        json!({
            "title": "R5 gallery",
            "intro": "Each topic's candidates one under the other, with the same content. Pick one per topic, add a short note if you like; copy the summary at the bottom in one tap.",
            "pick": "I'd pick this",
            "picked": "Picked",
            "note": "Short note",
            "copy": "Copy summary",
            "copied": "Copied.",
            "select": "No clipboard: the text is selected, copy it by hand.",
            "summary": "Summary",
            "share": "Link with your picks",
            "lang": "Türkçe",
            "theme": "Theme",
            "dark": "Dark theme",
            "light": "Light theme",
            "moment": "Same content",
            "loading": "Preparing the gallery (the first time can take a few seconds)…",
            "missing": "The gallery is off. On the computer: scripts/demo.sh --gallery",
            "empty": "No candidates to compare yet.",
        })
    }
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
mod build_impl {
    use std::collections::BTreeMap;

    use qp_session::ReplayHost;
    use serde_json::{json, Value};

    use super::{
        cand_label, candidates, tr, Build, CandOut, Layout, Shot, ShotOut, TopicOut, IMAGE_SCALE,
        PRINCIPAL, STEP, TOPICS,
    };
    use crate::shell::ShellCore;

    /// Delivers what `core` queues to `host` on `conn` and the answers back, until idle.
    fn pump(core: &mut ShellCore, host: &mut ReplayHost, conn: &str) {
        for _ in 0..1000 {
            let out = core.take_outbox();
            if out.is_empty() {
                break;
            }
            for m in out {
                for l in host.handle_raw(conn, &m.to_string()) {
                    if l.conn == conn {
                        core.on_message(&l.msg);
                    }
                }
            }
        }
    }

    struct Ctx {
        host: ReplayHost,
        lang: String,
        theme: String,
        next: u32,
        images: BTreeMap<String, Vec<u8>>,
    }

    impl Ctx {
        /// A new core on its own connection, at the host's moment, in the gallery's
        /// language and theme, painting at [`IMAGE_SCALE`].
        fn core(&mut self) -> (ShellCore, String) {
            self.next += 1;
            let conn = format!("g{}", self.next);
            let data = self.host.init().data.clone().unwrap_or_default();
            let evs = self.host.recorded_events();
            let mut core = ShellCore::new(PRINCIPAL, &self.lang, &data, &evs).expect("core");
            core.set_replay(self.host.state());
            core.set_dpr(IMAGE_SCALE);
            pump(&mut core, &mut self.host, &conn);
            core.set_style("", "", &self.theme);
            pump(&mut core, &mut self.host, &conn);
            (core, conn)
        }

        fn png(&mut self, name: String, rgba: &[u8], (w, h, k): (u32, u32, u32)) -> ShotOut {
            let png = qp_render::encode_png(rgba, w * k, h * k).unwrap_or_default();
            let src = format!("/gallery/img/{name}");
            self.images.insert(name, png);
            ShotOut {
                kind: "img".to_owned(),
                src: Some(src),
                css: Some([w, h]),
                text: None,
            }
        }
    }

    fn text(kind: &str, t: &str) -> ShotOut {
        ShotOut {
            kind: kind.to_owned(),
            src: None,
            css: None,
            text: Some(t.to_owned()),
        }
    }

    /// The feed's first capsule with a face scene under the presentation's settings: the
    /// capsule every candidate opens.
    fn reference_capsule(core: &mut ShellCore) -> Option<String> {
        core.feed_json().as_array().and_then(|rows| {
            rows.iter()
                .find(|r| r["scene"].as_bool() == Some(true))
                .and_then(|r| r["id"].as_str().map(str::to_owned))
        })
    }

    #[allow(clippy::too_many_lines)]
    fn shots(
        cx: &mut Ctx,
        core: &mut ShellCore,
        conn: &str,
        prefix: &str,
        open: Option<&str>,
        list: &[Shot],
    ) -> (Vec<ShotOut>, String) {
        let lang = cx.lang.clone();
        let mut out = Vec::new();
        let mut depth = String::new();
        let mut n = 0usize;
        for s in list {
            match *s {
                Shot::Feed(k) => {
                    let rows = core.feed_json();
                    let rows = rows.as_array().cloned().unwrap_or_default();
                    for r in rows.iter().take(k) {
                        if let Some(h) = r["header"].as_str() {
                            out.push(text("head", h));
                            continue;
                        }
                        let id = r["id"].as_str().unwrap_or("");
                        let px = if r["scene"].as_bool() == Some(true) {
                            core.face(id)
                        } else {
                            None
                        };
                        match px {
                            Some(scene) => {
                                let rgba = core.face_rgba(id, true);
                                let dims = (scene.width, scene.height, core.scale());
                                let shot = cx.png(format!("{prefix}-{n}.png"), &rgba, dims);
                                n += 1;
                                out.push(shot);
                            }
                            None => out.push(text("text", r["text"].as_str().unwrap_or(""))),
                        }
                    }
                    let (n_feed, hidden) = (core.feed().len(), core.coin_flip_hidden());
                    out.push(text(
                        "text",
                        &if tr(&lang) {
                            format!("Akışta {n_feed} kapsül; yazı tura kuralının gizlediği: {hidden}.")
                        } else {
                            format!("{n_feed} capsules in the feed; hidden by the coin-flip rule: {hidden}.")
                        },
                    ));
                    depth = if tr(&lang) {
                        format!("akışın ilk {k} satırı")
                    } else {
                        format!("the feed's first {k} rows")
                    };
                }
                Shot::Face => {
                    let Some(id) = open else { continue };
                    if let Some(scene) = core.face(id) {
                        let rgba = core.face_rgba(id, true);
                        let dims = (scene.width, scene.height, core.scale());
                        let shot = cx.png(format!("{prefix}-{n}.png"), &rgba, dims);
                        n += 1;
                        out.push(shot);
                    }
                }
                Shot::Capsule | Shot::Chart | Shot::Evidence => {
                    let Some(id) = open else { continue };
                    let to = match *s {
                        Shot::Capsule => "capsule",
                        Shot::Chart => "chart",
                        _ => "evidence",
                    };
                    if core.open_json()["depth"] == "sentence" {
                        core.open_capsule(id);
                        pump(core, &mut cx.host, conn);
                    }
                    if to != "capsule" {
                        core.open_to(to);
                        pump(core, &mut cx.host, conn);
                    }
                    let o = core.open_json();
                    if let Some(t) = o["text"].as_str() {
                        // The open layer's title once (the depths below share it).
                        if !out
                            .iter()
                            .any(|x| x.kind == "head" && x.text.as_deref() == Some(t))
                        {
                            out.push(text("head", t));
                        }
                    }
                    if let Some(scene) = core.scene() {
                        let rgba = core.scene_rgba(true);
                        let dims = (scene.width, scene.height, core.scale());
                        let shot = cx.png(format!("{prefix}-{n}.png"), &rgba, dims);
                        n += 1;
                        out.push(shot);
                    }
                    if to == "evidence" {
                        for row in core.evidence_json().as_array().cloned().unwrap_or_default() {
                            out.push(text("text", &evidence_text(&row)));
                        }
                    }
                    depth = format!(
                        "{} {}",
                        o["capsule"].as_str().unwrap_or(id),
                        o["depth"].as_str().unwrap_or("")
                    );
                }
            }
        }
        if depth.is_empty() {
            if let Some(id) = open {
                depth = format!("{id} {}", if tr(&lang) { "yüz" } else { "face" });
            }
        }
        (out, depth)
    }

    /// An evidence row as the core formatted it: `fact · name value · …`.
    fn evidence_text(row: &Value) -> String {
        let mut parts = vec![row["fact"].as_str().unwrap_or("").to_owned()];
        for v in row["values"].as_array().cloned().unwrap_or_default() {
            let a = v.as_array().cloned().unwrap_or_default();
            let name = a.first().and_then(Value::as_str).unwrap_or("");
            let val = a.get(1).and_then(Value::as_str).unwrap_or("");
            parts.push(format!("{name} {val}"));
        }
        if let Some(r) = row["requires"].as_str() {
            parts.push(r.to_owned());
        }
        parts.join(" · ")
    }

    /// Builds the gallery for `lang` (`tr`, `en`) and `theme` (`dark`, `light`): the demo's
    /// synthetic, paused world at [`STEP`]; per topic and candidate a fresh core with the
    /// presentation's settings plus the candidate's, painted at the topic's depth.
    ///
    /// # Panics
    /// Never for the built-in synthetic world.
    #[must_use]
    pub fn build(lang: &str, theme: &str) -> Build {
        let lang = if lang == "en" { "en" } else { "tr" };
        let theme = if theme == "light" { "light" } else { "dark" };
        let mut host = crate::drive::parity_host();
        host.set_record_events(true);
        let mut cx = Ctx {
            host,
            lang: lang.to_owned(),
            theme: theme.to_owned(),
            next: 0,
            images: BTreeMap::new(),
        };
        // The moment: a seeker core moves the paused host to STEP.
        let (mut seeker, conn) = cx.core();
        seeker.seek(&STEP.to_string());
        pump(&mut seeker, &mut cx.host, &conn);
        let (mut reference, _) = cx.core();
        let open = reference_capsule(&mut reference);
        let st = reference.status_json();
        let step_w = if tr(lang) { "adım" } else { "step" };
        let moment = format!(
            "{step_w} {} · {}",
            st["step"].as_str().unwrap_or(""),
            st["time"].as_str().unwrap_or("")
        );
        let choices = ShellCore::choices_json();
        let mut topics = Vec::new();
        for t in TOPICS {
            let names = candidates(t, &choices);
            if names.len() < 2 {
                continue;
            }
            let mut cands = Vec::new();
            for c in &names {
                if t.key == "feed_rule" {
                    let rule = qp_capsule::feedrule::FeedRule::named(c, true);
                    cx.host.set_principal_coin_flip_rule(PRINCIPAL, rule);
                }
                let (mut core, conn) = cx.core();
                core.set_look(&json!({ t.key: c }));
                pump(&mut core, &mut cx.host, &conn);
                core.requery_feed();
                pump(&mut core, &mut cx.host, &conn);
                let prefix = format!("{lang}-{theme}-{}-{c}", t.key);
                let (shots, depth) =
                    shots(&mut cx, &mut core, &conn, &prefix, open.as_deref(), t.shots);
                let s = core.status_json();
                let content = format!(
                    "{step_w} {} · {} · {depth}",
                    s["step"].as_str().unwrap_or(""),
                    s["time"].as_str().unwrap_or("")
                );
                cands.push(CandOut {
                    id: c.clone(),
                    label: cand_label(t.key, c, lang),
                    content,
                    shots,
                });
                if t.key == "feed_rule" {
                    cx.host.clear_principal_coin_flip_rule(PRINCIPAL);
                }
            }
            let i = usize::from(!tr(lang));
            topics.push(TopicOut {
                key: t.key.to_owned(),
                label: t.label[i].to_owned(),
                hint: t.hint[i].to_owned(),
                cands,
            });
        }
        Build {
            layout: Layout {
                lang: lang.to_owned(),
                theme: theme.to_owned(),
                moment,
                topics,
            },
            images: cx.images,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> Layout {
        let cand = |id: &str| CandOut {
            id: id.to_owned(),
            label: id.to_owned(),
            content: "adım 120 / 2080 · x".to_owned(),
            shots: vec![],
        };
        Layout {
            lang: "tr".to_owned(),
            theme: "dark".to_owned(),
            moment: "adım 120 / 2080 · 2025-06-02 10:00".to_owned(),
            topics: vec![
                TopicOut {
                    key: "voice".to_owned(),
                    label: "Dil".to_owned(),
                    hint: String::new(),
                    cands: vec![cand("marla"), cand("selvi")],
                },
                TopicOut {
                    key: "look".to_owned(),
                    label: "Görünüş".to_owned(),
                    hint: String::new(),
                    cands: vec![cand("r4"), cand("ornek")],
                },
            ],
        }
    }

    #[test]
    fn summary_is_deterministic_and_complete() {
        let mut g = GalleryCore::new("", "", "", "", "1763460000000");
        g.load(&serde_json::to_string(&layout()).unwrap()).unwrap();
        g.set_device("Mozilla/5.0 (iPhone; CPU iPhone OS 27_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.0 Mobile/15E148 Safari/604.1", 402.0, 874.0, 3.0);
        g.pick("voice", "marla");
        g.note("voice", "marla", "  kısa\nve net  ");
        g.note("look", "ornek", "sıcak");
        g.pick("look", "nope");
        let s = g.summary();
        assert_eq!(
            s,
            "R5 galeri · voice=marla (not: kısa ve net) · look=—\n\
             notlar: look/ornek: sıcak\n\
             an: adım 120 / 2080 · 2025-06-02 10:00 · tr · koyu\n\
             cihaz: iPhone iOS 27.0 · Safari · 402×874 @3 · tarih: 2025-11-18 13:00 (İstanbul)"
        );
        // Restored from the saved state: the same summary.
        let mut h = GalleryCore::new(&g.state(), "", "", "", "1763460000000");
        h.load(&serde_json::to_string(&layout()).unwrap()).unwrap();
        h.set_device("Mozilla/5.0 (iPhone; CPU iPhone OS 27_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.0 Mobile/15E148 Safari/604.1", 402.0, 874.0, 3.0);
        assert_eq!(h.summary(), s);
        // One pick per topic; a second tap takes it back.
        h.pick("voice", "selvi");
        assert_eq!(h.marks()["voice/selvi"], json!(true));
        assert_eq!(h.marks()["voice/marla"], json!(false));
        h.pick("voice", "selvi");
        assert_eq!(h.marks()["voice/selvi"], json!(false));
    }

    #[test]
    fn pick_parameter_and_share_link() {
        let mut g = GalleryCore::new("", "en", "light", "voice.selvi,look.zzz,bad", "");
        g.load(&serde_json::to_string(&layout()).unwrap()).unwrap();
        assert_eq!(g.share(), "gallery.html?pick=voice.selvi");
        assert_eq!(g.layout_url(), "/gallery/layout?lang=en&theme=light");
        assert!(g
            .summary()
            .starts_with("R5 gallery · voice=selvi · look=—\nmoment: "));
        assert!(g.summary().ends_with("device: — · date: —"));
        g.toggle_lang();
        assert_eq!(g.lang(), "tr");
    }

    #[test]
    fn notes_are_one_short_line() {
        assert_eq!(clean_note("a\tb\n\nc "), "a b c");
        assert_eq!(clean_note(&"x".repeat(500)).chars().count(), NOTE_MAX);
    }

    #[test]
    fn candidates_follow_the_registry_and_finalists() {
        let ch = json!({"voice": ["r4", "marla", "selvi"]});
        let voice = TOPICS.iter().find(|t| t.key == "voice").unwrap();
        let story = TOPICS.iter().find(|t| t.key == "story").unwrap();
        assert_eq!(candidates(voice, &ch).len(), 3);
        assert!(candidates(story, &ch).is_empty());
    }

    /// The host's build: every shown topic has at least two candidates, all with the same
    /// content (moment, capsule, depth), and the images exist.
    #[test]
    fn build_shows_the_same_content_per_topic() {
        let b = build("tr", "dark");
        let keys: Vec<&str> = b.layout.topics.iter().map(|t| t.key.as_str()).collect();
        for k in ["voice", "look", "feed_order", "feed_rule"] {
            assert!(keys.contains(&k), "{k} missing: {keys:?}");
        }
        for t in &b.layout.topics {
            assert!(t.cands.len() >= 2, "{}", t.key);
            let c0 = &t.cands[0].content;
            assert!(c0.contains("120"), "{c0}");
            for c in &t.cands {
                assert_eq!(&c.content, c0, "{} / {}", t.key, c.id);
                assert!(
                    c.shots.iter().any(|s| s.kind == "img"),
                    "{} / {}",
                    t.key,
                    c.id
                );
                for s in c.shots.iter().filter(|s| s.kind == "img") {
                    let name = s
                        .src
                        .as_deref()
                        .unwrap()
                        .trim_start_matches("/gallery/img/");
                    assert!(b.images[name].starts_with(b"\x89PNG"), "{name}");
                }
            }
        }
        // Each candidate's own setting shows: within a topic that paints the open capsule,
        // no two candidates have identical shots (voice: the title/texts; look: pixels).
        for t in b
            .layout
            .topics
            .iter()
            .filter(|t| t.key == "voice" || t.key == "look")
        {
            for (i, a) in t.cands.iter().enumerate() {
                for z in &t.cands[i + 1..] {
                    let px = |c: &CandOut| -> Vec<Vec<u8>> {
                        c.shots
                            .iter()
                            .map(|s| match &s.src {
                                Some(src) => {
                                    b.images[src.trim_start_matches("/gallery/img/")].clone()
                                }
                                None => s.text.clone().unwrap_or_default().into_bytes(),
                            })
                            .collect()
                    };
                    assert_ne!(px(a), px(z), "{}: {} = {}", t.key, a.id, z.id);
                }
            }
        }
        // Deterministic: a second build is identical.
        let again = build("tr", "dark");
        assert_eq!(again.layout, b.layout);
        assert_eq!(again.images, b.images);
    }
}
