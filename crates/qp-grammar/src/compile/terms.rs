//! (R5/m17-int, K49; `docs/LANGUAGE.md` "Sözlük") Glossary terms marked in a capsule
//! scene's texts.
//!
//! Under a voice ([`DataProvider::voice`]) every capsule scene's text layers that draw a
//! finding (the face's ledger band, the open card's detail and ledger) are searched for the
//! voice's glossary terms. The face's headline is not: a tap on it opens the capsule (the
//! face's one action, K43), so a term there would take that tap away. The first place each term is drawn gets
//!
//! - an underline: a one-pixel [`Prim::Rect`] in the text's colour under the drawn words,
//!   added to that layer's primitives (every painter draws it, nothing else to know);
//! - a [`TermMark`] in the scene (`SceneDesc::terms`): the term, its explanation, the layer
//!   and the words' box. The point query (`hit`) reports a term under the pixel and a tap
//!   there opens the explanation instead of the capsule (`qp-session` `tap_at`).
//!
//! **Finding a term in a text** (deterministic, no dictionary): the text and the term are
//! compared lower-cased, letter by letter (Turkish `İ` → `i`, `I` → `ı`, on both sides).
//! A term's cues, tried in order, are its words, then the same with the last word cut by
//! one and two letters (Turkish suffixes soften a final consonant: "düşük" → "düşüğü"),
//! then its first words alone (a multi-word term whose first word has at least four
//! letters), then the term without its short first words (under four letters; when the word it then starts with has
//! at least four letters). A cue matches at the start of a word; the mark runs to the end
//! of that word.
//! So "en düşük fiyat" marks "en düşüğünün", "benzer durum" marks "Benzer 74 …" at
//! "Benzer", "ortalama" marks "ortalamanın". A term found nowhere is not marked; it stays
//! in the capsule's glossary list. A term is matched within one drawn line (a term that
//! wraps across two lines is marked by the part on one line, when that part starts with a
//! word of four letters or more).
//!
//! Positions use the same width estimate the text was wrapped with
//! ([`crate::draw::est_width`] with the look's advance for that size), so the boxes sit on
//! the drawn words in every painter that draws the scene's fonts at their advance.

use qp_schema::protocol::GlossEntry;
use qp_schema::scene::{Align, PixelRect, Prim, ScenePanel, TermMark};

use crate::designs::Designs;
use crate::draw::{est_width, text_metrics};
use crate::provider::DataProvider;
use qp_look::look::Look;

/// Lower-cases one letter so that both sides of a comparison keep their lengths.
fn low(c: char) -> char {
    match c {
        'İ' => 'i',
        'I' => 'ı',
        _ => c.to_lowercase().next().unwrap_or(c),
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '’'
}

/// The cues of `term`, longest first (see the module docs).
#[must_use]
pub fn cues(term: &str) -> Vec<Vec<char>> {
    let words: Vec<Vec<char>> = term
        .split_whitespace()
        .map(|w| w.chars().map(low).collect())
        .collect();
    let mut out: Vec<Vec<char>> = Vec::new();
    let mut push = |v: Vec<char>| {
        if !v.is_empty() && !out.contains(&v) {
            out.push(v);
        }
    };
    let join = |ws: &[Vec<char>]| -> Vec<char> {
        let mut v = Vec::new();
        for (i, w) in ws.iter().enumerate() {
            if i > 0 {
                v.push(' ');
            }
            v.extend(w);
        }
        v
    };
    let n = words.len();
    for k in (1..=n).rev() {
        if k == 1 && n > 1 && words[0].len() < 4 {
            break;
        }
        let ws = &words[..k];
        push(join(ws));
        let last = &ws[k - 1];
        // Cut the last word by one and two letters, keeping at least three (four for a
        // word of a multi-word cue).
        let keep = if k == 1 && n == 1 { 3 } else { 4 };
        for cut in 1..=2 {
            if last.len() >= 4 && last.len() - cut >= keep {
                let mut v = join(&ws[..k - 1]);
                if k > 1 {
                    v.push(' ');
                }
                v.extend(&last[..last.len() - cut]);
                push(v);
            }
        }
    }
    // Then the term without its first words (a term wrapped after a short first word: "en
    // / düşük fiyatının"), when the remaining first word has at least four letters.
    // Only short words are dropped ("en", "the"): "similar cases" never becomes "cases".
    for j in 1..n {
        if words[j].len() < 4 || words[..j].iter().any(|w| w.len() >= 4) {
            continue;
        }
        push(join(&words[j..]));
    }
    out
}

/// Where `cue` first matches `text` at the start of a word: the char range of the whole
/// words it covers (`[start, end)`).
#[must_use]
pub fn find(text: &[char], cue: &[char]) -> Option<(usize, usize)> {
    if cue.is_empty() || cue.len() > text.len() {
        return None;
    }
    let lt: Vec<char> = text.iter().copied().map(low).collect();
    (0..=lt.len() - cue.len()).find_map(|i| {
        let start_ok = i == 0 || !is_word(lt[i - 1]);
        if !start_ok || lt[i..i + cue.len()] != *cue {
            return None;
        }
        let mut end = i + cue.len();
        while end < lt.len() && is_word(lt[end]) {
            end += 1;
        }
        Some((i, end))
    })
}

#[allow(clippy::too_many_lines)]
/// Marks the glossary terms of every capsule text layer in `panels` (see the module docs)
/// and returns them in paint order.
pub(crate) fn mark(
    panels: &mut [ScenePanel],
    p: &dyn DataProvider,
    look: &Look,
    designs: &Designs,
) -> Vec<TermMark> {
    let mut out: Vec<TermMark> = Vec::new();
    for panel in panels.iter_mut() {
        for layer in &mut panel.layers {
            let qp_schema::scene::DataSource::Finding { finding } = &layer.data else {
                continue;
            };
            // The face's headline is where a tap opens the capsule (K43, the face's one
            // action): its terms are not marked there (the open capsule's texts and the
            // face's ledger band carry them).
            if layer.mark != qp_schema::grammar::Mark::Text || layer.id == "headline" {
                continue;
            }
            // (R5/M20) A design's layer with its own entry: the whole panel opens it, the
            // first line is underlined (`Designs::layer_term`).
            if let Some(g) = (designs.layer_term)(&layer.id, finding, p) {
                if !out.iter().any(|t| t.term == g.term) {
                    underline_layer(&mut out, &mut layer.prims, &layer.id, panel.rect, g, look);
                }
            }
            if !(designs.voice_terms_in)(&layer.id) {
                continue;
            }
            let Some(voice) = p.voice(finding) else {
                continue;
            };
            let mut lines: Vec<Prim> = Vec::new();
            for prim in &layer.prims {
                let Prim::Text {
                    x,
                    baseline,
                    px,
                    text,
                    color,
                    align: Align::Left,
                    ..
                } = prim
                else {
                    continue;
                };
                let chars: Vec<char> = text.chars().collect();
                let size = f64::from(*px);
                let (adv, _, asc) = text_metrics(look, size);
                for g in &voice.glossary {
                    if out.iter().any(|t| t.term == g.term) {
                        continue;
                    }
                    let Some((a, b)) = locate(&chars, g) else {
                        continue;
                    };
                    let pre: String = chars[..a].iter().collect();
                    let word: String = chars[..b].iter().collect();
                    let x0 = x + if a == 0 {
                        0
                    } else {
                        est_width(&pre, size, adv)
                    };
                    let x1 = x + est_width(&word, size, adv);
                    #[allow(clippy::cast_possible_truncation)]
                    let top = baseline - (size * asc).round() as i32;
                    #[allow(clippy::cast_possible_truncation)]
                    let under = baseline + (size * 0.18).round().max(2.0) as i32;
                    let w = u32::try_from((x1 - x0).max(1)).unwrap_or(1);
                    lines.push(Prim::Rect {
                        x: x0,
                        y: under,
                        w,
                        h: 1,
                        color: *color,
                    });
                    out.push(TermMark {
                        term: g.term.clone(),
                        text: g.text.clone(),
                        layer: layer.id.clone(),
                        hit: PixelRect {
                            left: x0,
                            top,
                            right: x1,
                            bottom: under + 2,
                        },
                    });
                }
            }
            layer.prims.extend(lines);
        }
    }
    out
}

/// (R5/M20; RF1/G2b: split out of [`mark`]) A design layer's own glossary entry `g`: the
/// layer's first text line is underlined and the term's box is the whole panel `hit`.
fn underline_layer(
    out: &mut Vec<TermMark>,
    prims: &mut Vec<Prim>,
    layer: &str,
    hit: PixelRect,
    g: GlossEntry,
    look: &Look,
) {
    if let Some(Prim::Text {
        x,
        baseline,
        px,
        text,
        color,
        ..
    }) = prims.iter().find(|q| matches!(q, Prim::Text { .. }))
    {
        let size = f64::from(*px);
        let (adv, _, _) = text_metrics(look, size);
        #[allow(clippy::cast_possible_truncation)]
        let under = baseline + (size * 0.18).round().max(2.0) as i32;
        let w = u32::try_from(est_width(text, size, adv).max(1)).unwrap_or(1);
        let line = Prim::Rect {
            x: *x,
            y: under,
            w,
            h: 1,
            color: *color,
        };
        out.push(TermMark {
            term: g.term,
            text: g.text,
            layer: layer.to_owned(),
            hit,
        });
        prims.push(line);
    }
}

/// Where glossary entry `g`'s term is drawn in `line` (the first cue that matches).
fn locate(line: &[char], g: &GlossEntry) -> Option<(usize, usize)> {
    cues(&g.term).iter().find_map(|c| find(line, c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str, term: &str) -> Option<String> {
        let t: Vec<char> = text.chars().collect();
        let g = GlossEntry {
            term: term.into(),
            text: String::new(),
        };
        locate(&t, &g).map(|(a, b)| t[a..b].iter().collect())
    }

    #[test]
    fn terms_are_found_in_their_inflected_forms() {
        let h = "SYN30 20 günün en düşüğünün (6,17) altına indi · dün, artık geçerli değil";
        assert_eq!(at(h, "en düşük fiyat").as_deref(), Some("en düşüğünün"));
        assert_eq!(
            at(h, "artık geçerli değil").as_deref(),
            Some("artık geçerli değil")
        );
        assert_eq!(
            at(
                "SYN19 son 20 günün en yüksek fiyatını (65,11) aştı",
                "en yüksek fiyat"
            )
            .as_deref(),
            Some("en yüksek fiyatını")
        );
        assert_eq!(
            at("Benzer 74 durumun 36'sında fiyat yükseldi.", "benzer durum").as_deref(),
            Some("Benzer")
        );
        assert_eq!(
            at("SYN03 20 günlük ortalamanın altında", "ortalama").as_deref(),
            Some("ortalamanın")
        );
        assert_eq!(at("RSI 70'in üstüne çıktı", "RSI").as_deref(), Some("RSI"));
        assert_eq!(at("its RSI went above 70", "RSI").as_deref(), Some("RSI"));
        assert_eq!(
            at("Few cases: the price fell in 1 of 2", "similar cases"),
            None,
            "a term absent from the text is not marked"
        );
        // A word start only: "bant" does not match inside "abant".
        assert_eq!(at("abant", "bant"), None);
        // A short first word alone is not a cue.
        assert_eq!(at("en son", "en düşük fiyat"), None);
        // A term wrapped after its short first word: the rest is marked.
        assert_eq!(
            at("düşük fiyatının (110,95) altına indi", "en düşük fiyat").as_deref(),
            Some("düşük fiyatının")
        );
    }

    #[test]
    fn cues_are_deterministic_and_longest_first() {
        let c = cues("en düşük fiyat");
        let s: Vec<String> = c.iter().map(|v| v.iter().collect()).collect();
        assert_eq!(s[0], "en düşük fiyat");
        assert!(s.contains(&"en düşü".to_owned()), "{s:?}");
        assert_eq!(cues("en düşük fiyat"), c);
    }
}
