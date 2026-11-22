//! The gallery's topics and finalists (RG1: moved out of `gallery.rs`): what a candidate
//! shows, the topic registry and the candidates shown per topic.

use super::*;

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
