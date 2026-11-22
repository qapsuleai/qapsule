//! The gallery page's texts (RG1: moved out of `gallery.rs`): language and theme
//! normalisation, notes, the device label and the page labels.

use super::*;

pub(super) fn norm_lang(s: &str) -> Option<&'static str> {
    match s {
        "tr" => Some("tr"),
        "en" => Some("en"),
        _ => None,
    }
}

pub(super) fn norm_theme(s: &str) -> Option<&'static str> {
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
pub(super) fn labels(lang: &str) -> Value {
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
