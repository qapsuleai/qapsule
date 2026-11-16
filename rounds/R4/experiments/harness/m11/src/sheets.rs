//! SPEC §5(b): persona sheets (`m11 sheets --style <name> --seed <s>`).
//!
//! A sheet is 16 glyphs in a 4×4 grid labelled A1…D4 (row letter, column number; the label
//! above its glyph), each drawn at 32 px through the style and the project rasterizer on
//! the light theme and upscaled 4× nearest-neighbour (128 px cells). The glyphs are
//! seeded and depend only on the seed (the same seed shows every style the same 16
//! glyphs): 16 of the 18 kind × direction combinations, outcomes 6 realized, 6 failed,
//! 4 open in seeded order, strength ∈ {0.3, 0.6, 0.9}, confidence ∈ {none, 0.3, 0.7},
//! confirmations 1–3, freshness 1, no server mark.
//!
//! Blinding (SPEC §0.2): the image is `raw/screens/<code>.png`, the code a seeded hash
//! ([`crate::blind::code`]); the hidden key `raw/keys/<code>.json` (style, alphabet,
//! seed, answers) and the index `raw/key.json` (code → style) are never copied to a
//! reader. Prompts `raw/prompts/<persona>_<cond>_<code>.txt` carry `@DIR@` for the
//! reader's private directory.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use qp_grammar::glyph::outcome::{OutcomeStyle, LEGEND_MAX_WORDS};
use qp_grammar::theme::Theme;
use qp_render::scene::scene_of;
use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind, Validity};
use qp_schema::scene::{Align, Prim};
use serde::{Deserialize, Serialize};

use crate::alphabets;
use crate::blind::{code, forbidden_hits};
use crate::io::{read, write, write_json};
use crate::render::{blit, glyph_rgba, upscale};
use crate::rng::Rng;

/// Directions, canonical order.
pub const DIRS: [Direction; 3] = [Direction::Up, Direction::Down, Direction::Neutral];
/// Reading conditions: without / with the style's one-sentence explanation.
pub const CONDITIONS: [&str; 2] = ["nolegend", "legend"];
/// Personas (SPEC §1).
pub const PERSONAS: [&str; 2] = ["F", "H"];
/// Glyph size drawn for sheets.
pub const GLYPH_PX: u32 = 32;
/// Upscale factor.
pub const SCALE: u32 = 4;
/// Outcomes per sheet: realized, failed, open.
pub const MIX: [(Outcome, usize); 3] = [
    (Outcome::Realized, 6),
    (Outcome::Failed, 6),
    (Outcome::Open, 4),
];

const CELL: u32 = GLYPH_PX * SCALE;
const GAP: u32 = 24;
const LABEL_H: u32 = 24;
const MARGIN: u32 = 16;

/// The truth of one sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Closed, came true (gerçekleşti): phase `realized`.
    Realized,
    /// Closed, did not come true (tutmadı): phase `failed`.
    Failed,
    /// Still open (hâlâ açık): phase `final`, validity `valid`.
    Open,
}

impl Outcome {
    /// Record name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Realized => "realized",
            Self::Failed => "failed",
            Self::Open => "open",
        }
    }
}

/// `up` / `down` / `neutral`.
#[must_use]
pub fn dir_name(d: Direction) -> &'static str {
    match d {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Neutral => "neutral",
    }
}

/// The glyph of a kind × direction with an outcome.
#[must_use]
pub fn glyph(
    kind: MeaningKind,
    direction: Direction,
    outcome: Outcome,
    strength: f64,
    confidence: Option<f64>,
    confirmations: u8,
) -> GlyphSpec {
    let (phase, validity) = match outcome {
        Outcome::Realized => (GlyphPhase::Realized, Validity::Closed),
        Outcome::Failed => (GlyphPhase::Failed, Validity::Closed),
        Outcome::Open => (GlyphPhase::Final, Validity::Valid),
    };
    GlyphSpec {
        kind,
        direction,
        strength,
        confidence,
        phase,
        freshness: 1.0,
        confirmations,
        server: false,
        validity: Some(validity),
    }
}

/// One sign of a sheet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// `A1` … `D4`.
    pub label: String,
    /// `kind.direction`.
    pub glyph: String,
    /// The truth.
    pub outcome: Outcome,
    /// The drawn spec.
    pub spec: GlyphSpec,
}

/// The 16 signs of seed `seed`, labelled A1…D4.
#[must_use]
pub fn items(seed: u64) -> Vec<Item> {
    let mut r = Rng::derive(seed, "m11.sheet");
    let mut combos: Vec<(MeaningKind, Direction)> = Vec::new();
    for k in MeaningKind::ALL {
        for d in DIRS {
            combos.push((k, d));
        }
    }
    r.shuffle(&mut combos);
    let mut outs: Vec<Outcome> = MIX
        .iter()
        .flat_map(|(o, n)| std::iter::repeat_n(*o, *n))
        .collect();
    r.shuffle(&mut outs);
    combos
        .into_iter()
        .zip(outs)
        .enumerate()
        .map(|(i, ((k, d), o))| {
            let strength = [0.3, 0.6, 0.9][r.below(3)];
            let confidence = [None, Some(0.3), Some(0.7)][r.below(3)];
            let confirmations = [1u8, 2, 3][r.below(3)];
            Item {
                label: format!(
                    "{}{}",
                    char::from(b'A' + u8::try_from(i / 4).unwrap_or(0)),
                    i % 4 + 1
                ),
                glyph: format!("{}.{}", k.name(), dir_name(d)),
                outcome: o,
                spec: glyph(k, d, o, strength, confidence, confirmations),
            }
        })
        .collect()
}

/// The hidden key of one sheet (`raw/keys/<code>.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Key {
    /// Harness version.
    pub harness: String,
    /// Sheet code (the image is `screens/<code>.png`).
    pub code: String,
    /// Outcome style.
    pub style: String,
    /// Alphabet.
    pub alphabet: String,
    /// Seed of the glyph selection.
    pub seed: u64,
    /// Salt of the code.
    pub salt: u64,
    /// The signs and their truth.
    pub items: Vec<Item>,
}

/// One line of `raw/key.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyEntry {
    /// Outcome style.
    pub style: String,
    /// Alphabet.
    pub alphabet: String,
    /// Seed.
    pub seed: u64,
}

fn text(x: u32, baseline: u32, px: f32, s: &str, theme: &Theme) -> Prim {
    Prim::Text {
        x: x.cast_signed(),
        baseline: baseline.cast_signed(),
        px,
        text: s.to_owned(),
        color: theme.text,
        align: Align::Left,
    }
}

/// PNG of a labelled 4-column grid.
///
/// # Errors
/// Unknown alphabet, rasterizer or encoder failure.
pub fn grid_png(
    style: &dyn OutcomeStyle,
    alphabet: &str,
    items: &[Item],
) -> Result<Vec<u8>, String> {
    let alpha = alphabets::by_name(alphabet)?;
    let theme = Theme::light();
    let rows = u32::try_from(items.len().div_ceil(4)).unwrap_or(1);
    let w = 2 * MARGIN + 4 * CELL + 3 * GAP;
    let h = 2 * MARGIN + rows * (LABEL_H + CELL) + rows.saturating_sub(1) * GAP;
    let pos = |i: usize| {
        let (c, r) = (
            u32::try_from(i % 4).unwrap_or(0),
            u32::try_from(i / 4).unwrap_or(0),
        );
        (
            MARGIN + c * (CELL + GAP),
            MARGIN + r * (LABEL_H + CELL + GAP),
        )
    };
    let prims: Vec<Prim> = items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let (x, y) = pos(i);
            text(x, y + 17, 18.0, &it.label, &theme)
        })
        .collect();
    let mut canvas = qp_render::render_scene_rgba(&scene_of(w, h, theme.bg, prims))
        .map_err(|e| e.to_string())?;
    for (i, it) in items.iter().enumerate() {
        let (x, y) = pos(i);
        let px = glyph_rgba(style, alpha, &it.spec, GLYPH_PX, &theme)?;
        let tile = upscale(&px, GLYPH_PX, GLYPH_PX, SCALE);
        blit(&mut canvas, w, &tile, CELL, CELL, x, y + LABEL_H);
    }
    qp_render::encode_png(&canvas, w, h).map_err(|e| e.to_string())
}

/// SPEC §1 persona descriptions, verbatim.
#[must_use]
pub fn persona_text(persona: &str) -> Option<&'static str> {
    match persona {
        "F" => Some(
            "Yirmi yaşındasın. Hisse senedi alıp satıyorsun ama teknik analiz bilmiyorsun; \
             grafik, indikatör, mum gibi terimler sana bir şey ifade etmiyor. Telefonuna bir \
             bildirim geldi; ekrana yalnızca birkaç saniye bakacaksın. İlk bakışta ne \
             anladığını cevapla; uzun uzun inceleme.",
        ),
        "H" => Some(
            "Yıllardır borsada işlem yapıyorsun; teknik analize, mum grafiklerine ve \
             indikatörlere alışkınsın. Telefonunda bir piyasa uygulamasına bakıyorsun. \
             Gördüğünü okuyup cevapla.",
        ),
        _ => None,
    }
}

/// The reader prompt. `@DIR@` is replaced by `run_readers.sh` with the absolute path of the
/// reader's private directory (it holds only `<code>.png`). The style's name never
/// appears; `legend` is shown only in the `legend` condition.
///
/// # Errors
/// Unknown persona or condition.
pub fn prompt(persona: &str, condition: &str, code: &str, legend: &str) -> Result<String, String> {
    let who = persona_text(persona).ok_or(format!("unknown persona `{persona}` (F, H)"))?;
    if !CONDITIONS.contains(&condition) {
        return Err(format!(
            "unknown condition `{condition}` (nolegend, legend)"
        ));
    }
    let mut p = String::new();
    p.push_str(who);
    p.push_str("\n\nBu bir piyasa bilgi uygulamasının ekran görüntüsü.\n\n");
    p.push_str(&format!(
        "Read aracıyla @DIR@/{code}.png dosyasını aç. Görüntüde uygulamanın kullandığı 16 \
         küçük işaret 4×4 ızgarada duruyor; her işaretin üstünde etiketi var (A1'den D4'e: \
         satır harfi, sütun rakamı). Her işaret, uygulamanın bir hisse senedi hakkındaki bir \
         tespitidir. Bazı tespitler kapanmıştır: söylediği ya gerçekleşti ya da tutmadı. \
         Bazıları hâlâ açıktır; sonucu henüz belli değildir.\n\n"
    ));
    if condition == "legend" {
        p.push_str("Uygulamanın kısa açıklaması: \"");
        p.push_str(legend);
        p.push_str("\"\n\n");
    }
    p.push_str(
        "Her etiket için şu dört cevaptan birini seç: \"gerçekleşti\", \"tutmadı\", \"açık\" \
         ya da \"bilmiyorum\". Emin değilsen de en iyi tahminini ver. Yalnızca tek bir JSON \
         nesnesiyle cevap ver, başka bir şey yazma; anahtarlar etiketlerdir, örneğin:\n\
         {\"A1\": \"gerçekleşti\", \"A2\": \"açık\", \"A3\": \"tutmadı\", …, \"D4\": \"bilmiyorum\"}\n",
    );
    Ok(p)
}

/// Rebuilds `raw/key.json` from every `raw/keys/*.json` (sorted by code).
///
/// # Errors
/// I/O or parse failure.
pub fn rebuild_index(root: &Path) -> Result<BTreeMap<String, KeyEntry>, String> {
    let keys = root.join("raw").join("keys");
    let mut idx = BTreeMap::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&keys)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    for f in files {
        let k: Key =
            serde_json::from_str(&read(&f)?).map_err(|e| format!("{}: {e}", f.display()))?;
        idx.insert(
            k.code.clone(),
            KeyEntry {
                style: k.style,
                alphabet: k.alphabet,
                seed: k.seed,
            },
        );
    }
    write_json(&root.join("raw").join("key.json"), &idx)?;
    Ok(idx)
}

/// What [`write_sheet`] wrote.
#[derive(Debug, Clone)]
pub struct Written {
    /// Sheet code.
    pub code: String,
    /// The image.
    pub image: PathBuf,
    /// The prompts written.
    pub prompts: Vec<PathBuf>,
}

/// Writes the sheet image, its hidden key, the key index and the prompts of the given
/// personas × conditions for one style, alphabet and seed. Fails if a prompt contains a
/// forbidden word, the legend is too long, or the code collides with another sheet.
///
/// # Errors
/// See above; rasterizer or I/O failure.
pub fn write_sheet(
    style: &dyn OutcomeStyle,
    alphabet: &str,
    seed: u64,
    salt: u64,
    personas: &[&str],
    conditions: &[&str],
    root: &Path,
) -> Result<Written, String> {
    let name = style.name();
    crate::io::check_name(name)?;
    alphabets::by_name(alphabet)?;
    let legend = style.legend(alphabet);
    let words = legend.split_whitespace().count();
    if words > LEGEND_MAX_WORDS {
        return Err(format!(
            "legend of `{name}` for `{alphabet}` has {words} words (max {LEGEND_MAX_WORDS})"
        ));
    }
    let c = code(name, alphabet, seed, salt);
    let raw = root.join("raw");
    let key_path = raw.join("keys").join(format!("{c}.json"));
    if let Ok(old) = read(&key_path) {
        let old: Key =
            serde_json::from_str(&old).map_err(|e| format!("{}: {e}", key_path.display()))?;
        if old.style != name || old.alphabet != alphabet || old.seed != seed {
            return Err(format!("code collision `{c}`; use another --salt"));
        }
    }
    let its = items(seed);
    let image = raw.join("screens").join(format!("{c}.png"));
    write(&image, &grid_png(style, alphabet, &its)?)?;
    let key = Key {
        harness: crate::HARNESS_VERSION.to_owned(),
        code: c.clone(),
        style: name.to_owned(),
        alphabet: alphabet.to_owned(),
        seed,
        salt,
        items: its,
    };
    write_json(&key_path, &key)?;
    rebuild_index(root)?;
    let mut prompts = Vec::new();
    for p in personas {
        for cond in conditions {
            let text = prompt(p, cond, &c, legend)?;
            let hits = forbidden_hits(&text, &[]);
            if !hits.is_empty() {
                return Err(format!(
                    "prompt {p}/{cond} for `{name}` contains forbidden words: {}",
                    hits.join(", ")
                ));
            }
            let path = raw.join("prompts").join(format!("{p}_{cond}_{c}.txt"));
            write(&path, text.as_bytes())?;
            prompts.push(path);
        }
    }
    Ok(Written {
        code: c,
        image,
        prompts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_are_seeded_and_mixed() {
        let a = items(3);
        assert_eq!(a.len(), 16);
        assert_eq!(a, items(3));
        assert_ne!(a, items(4));
        assert_eq!(a[0].label, "A1");
        assert_eq!(a[15].label, "D4");
        for (o, n) in MIX {
            assert_eq!(a.iter().filter(|i| i.outcome == o).count(), n);
        }
        let mut g: Vec<&str> = a.iter().map(|i| i.glyph.as_str()).collect();
        g.sort_unstable();
        g.dedup();
        assert_eq!(g.len(), 16);
    }

    #[test]
    fn prompt_has_persona_and_legend_only_when_asked() {
        let p = prompt("F", "legend", "0badc0de", "AÇIKLAMA").unwrap();
        assert!(p.starts_with("Yirmi yaşındasın."));
        assert!(p.contains("AÇIKLAMA") && p.contains("@DIR@/0badc0de.png"));
        assert!(!p.contains("tavsiye"));
        let q = prompt("H", "nolegend", "0badc0de", "AÇIKLAMA").unwrap();
        assert!(!q.contains("AÇIKLAMA") && q.starts_with("Yıllardır"));
        assert!(prompt("X", "legend", "c", "").is_err());
        assert!(prompt("F", "learn", "c", "").is_err());
    }
}
