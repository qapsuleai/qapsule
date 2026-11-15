//! M1 (b), (c): reading-test material (`harness m1 sheets --alt <name> --seed <s>`).
//!
//! Conditions:
//! - `nolegend`: one sheet, 16 glyphs in a 4×4 grid labelled A1…D4: all 12 of `K` plus 4
//!   channel variations (a `K` glyph with one of phase / strength / confirmations /
//!   confidence / freshness / server changed), seeded order;
//! - `legend`: the same kind of sheet; the prompt adds the alphabet's own legend
//!   (≤ 120 words, checked here);
//! - `learn`: `examples.png` with 5 labelled examples (≥ 4 kinds) and `test.png` with 12
//!   new glyphs (A1…C4) whose kind × direction × phase never appears among the examples.
//!
//! Every glyph is drawn at 32 px through the alphabet and the project rasterizer on the
//! light theme and upscaled 4× nearest-neighbour (128 px cells). The answer key goes to
//! `keys/` (never into the sheet directory the reader can see).

use std::path::Path;

use qp_grammar::theme::Theme;
use qp_render::scene::scene_of;
use qp_schema::capsule::GlyphSpec;
use qp_schema::scene::{Align, Prim};
use serde::{Deserialize, Serialize};

use super::glyphset::{
    base, core_set, dir_name, k_kind_dirs, phase_name, spec_id, strength_class, PHASES,
};
use super::registry::M1Entry;
use super::render::{blit, glyph_rgba, upscale};
use crate::io::{write, write_json};
use crate::rng::Rng;

/// Labelled glyphs.
pub type Labelled = Vec<(String, GlyphSpec)>;

/// Reading conditions.
pub const CONDITIONS: [&str; 3] = ["nolegend", "legend", "learn"];
/// Glyph size drawn for sheets.
pub const GLYPH_PX: u32 = 32;
/// Upscale factor.
pub const SCALE: u32 = 4;
/// Legend word limit.
pub const LEGEND_MAX_WORDS: usize = 120;

const CELL: u32 = GLYPH_PX * SCALE;
const GAP: u32 = 24;
const LABEL_H: u32 = 24;
const MARGIN: u32 = 16;

/// Expected answer for one label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Truth {
    /// Label (`A1`, `E1`…).
    pub label: String,
    /// Canonical spec id (for diagnosis).
    pub spec: String,
    /// Kind name.
    pub kind: String,
    /// `up` / `down` / `neutral`.
    pub direction: String,
    /// `low` / `high`.
    pub strength: String,
    /// Phase name.
    pub phase: String,
}

/// The hidden answer key of one sheet set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Key {
    /// Harness version.
    pub harness: String,
    /// Alternative.
    pub alt: String,
    /// Condition.
    pub condition: String,
    /// Seed.
    pub seed: u64,
    /// Labelled examples shown with their answers (`learn` only).
    pub examples: Vec<Truth>,
    /// Items the reader must answer.
    pub items: Vec<Truth>,
}

fn truth(label: String, s: &GlyphSpec) -> Truth {
    Truth {
        label,
        spec: spec_id(s),
        kind: s.kind.name().to_owned(),
        direction: dir_name(s.direction).to_owned(),
        strength: strength_class(s.strength).to_owned(),
        phase: phase_name(s.phase).to_owned(),
    }
}

fn grid_labels(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            format!(
                "{}{}",
                char::from(b'A' + u8::try_from(i / 4).unwrap_or(0)),
                i % 4 + 1
            )
        })
        .collect()
}

/// One channel variation of a `K` glyph (seeded).
fn variation(r: &mut Rng, b: &GlyphSpec) -> GlyphSpec {
    loop {
        let mut s = b.clone();
        match r.below(6) {
            0 => s.phase = PHASES[r.below(5)],
            1 => s.strength = [0.2, 0.9][r.below(2)],
            2 => s.confirmations = [2, 3][r.below(2)],
            3 => s.confidence = [None, Some(0.3), Some(0.8)][r.below(3)],
            4 => s.freshness = 0.3,
            _ => s.server = true,
        }
        if spec_id(&s) != spec_id(b) {
            return s;
        }
    }
}

/// The 16 glyphs of a `nolegend`/`legend` sheet, labelled A1…D4.
#[must_use]
pub fn read_items(seed: u64) -> Vec<(String, GlyphSpec)> {
    let mut r = Rng::derive(seed, "m1.read");
    let k = core_set();
    let mut v: Vec<GlyphSpec> = k.clone();
    for _ in 0..4 {
        let b = &k[r.below(k.len())];
        v.push(variation(&mut r, b));
    }
    r.shuffle(&mut v);
    grid_labels(v.len()).into_iter().zip(v).collect()
}

/// `(examples, test items)` of the `learn` condition.
#[must_use]
pub fn learn_items(seed: u64) -> (Labelled, Labelled) {
    let mut r = Rng::derive(seed, "m1.learn");
    let mut pool: Vec<GlyphSpec> = Vec::new();
    for (k, d) in k_kind_dirs() {
        for p in PHASES {
            for st in [0.2, 0.9] {
                pool.push(GlyphSpec {
                    phase: p,
                    strength: st,
                    ..base(k, d)
                });
            }
        }
    }
    r.shuffle(&mut pool);
    let mut ex: Vec<GlyphSpec> = Vec::new();
    for s in &pool {
        if ex.len() == 5 {
            break;
        }
        let kinds: Vec<_> = ex.iter().map(|e| e.kind).collect();
        let distinct = {
            let mut k2 = kinds.clone();
            k2.sort();
            k2.dedup();
            k2.len()
        };
        let need_new_kind = distinct < 4;
        if !need_new_kind || !kinds.contains(&s.kind) {
            ex.push(s.clone());
        }
    }
    let seen = |s: &GlyphSpec| {
        ex.iter()
            .any(|e| e.kind == s.kind && e.direction == s.direction && e.phase == s.phase)
    };
    let mut rest: Vec<GlyphSpec> = pool.iter().filter(|s| !seen(s)).cloned().collect();
    r.shuffle(&mut rest);
    rest.truncate(12);
    let exl = (1..=ex.len()).map(|i| format!("E{i}"));
    (
        exl.zip(ex).collect(),
        grid_labels(rest.len()).into_iter().zip(rest).collect(),
    )
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

fn glyph_tile(entry: &M1Entry, s: &GlyphSpec, theme: &Theme) -> Result<Vec<u8>, String> {
    let px = glyph_rgba(entry.alphabet.as_ref(), s, GLYPH_PX, theme)?;
    Ok(upscale(&px, GLYPH_PX, GLYPH_PX, SCALE))
}

/// PNG of a labelled grid (4 columns).
///
/// # Errors
/// Rasterizer or encoder failure.
pub fn grid_png(entry: &M1Entry, items: &[(String, GlyphSpec)]) -> Result<Vec<u8>, String> {
    let theme = Theme::light();
    let rows = u32::try_from(items.len().div_ceil(4)).unwrap_or(1);
    let w = 2 * MARGIN + 4 * CELL + 3 * GAP;
    let h = 2 * MARGIN + rows * (LABEL_H + CELL) + rows.saturating_sub(1) * GAP;
    let mut prims = Vec::new();
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
    for (i, (label, _)) in items.iter().enumerate() {
        let (x, y) = pos(i);
        prims.push(text(x, y + 17, 18.0, label, &theme));
    }
    let mut canvas = qp_render::render_scene_rgba(&scene_of(w, h, theme.bg, prims))
        .map_err(|e| e.to_string())?;
    for (i, (_, s)) in items.iter().enumerate() {
        let (x, y) = pos(i);
        blit(
            &mut canvas,
            w,
            &glyph_tile(entry, s, &theme)?,
            CELL,
            CELL,
            x,
            y + LABEL_H,
        );
    }
    qp_render::encode_png(&canvas, w, h).map_err(|e| e.to_string())
}

/// PNG of the labelled examples (one per row, answer written next to the glyph).
///
/// # Errors
/// Rasterizer or encoder failure.
pub fn examples_png(entry: &M1Entry, ex: &[(String, GlyphSpec)]) -> Result<Vec<u8>, String> {
    let theme = Theme::light();
    let n = u32::try_from(ex.len()).unwrap_or(0);
    let w = 2 * MARGIN + CELL + GAP + 300;
    let h = 2 * MARGIN + n * CELL + n.saturating_sub(1) * GAP;
    let mut prims = Vec::new();
    let mut tiles = Vec::new();
    for (i, (label, s)) in ex.iter().enumerate() {
        let y = MARGIN + u32::try_from(i).unwrap_or(0) * (CELL + GAP);
        let tx = MARGIN + CELL + GAP;
        let t = truth(label.clone(), s);
        let lines = [
            label.clone(),
            format!("kind: {}", t.kind),
            format!("direction: {}", t.direction),
            format!("strength: {}", t.strength),
            format!("phase: {}", t.phase),
        ];
        for (j, l) in lines.iter().enumerate() {
            let b = y + 20 + u32::try_from(j).unwrap_or(0) * 24;
            prims.push(text(tx, b, 17.0, l, &theme));
        }
        tiles.push((glyph_tile(entry, s, &theme)?, MARGIN, y));
    }
    let mut canvas = qp_render::render_scene_rgba(&scene_of(w, h, theme.bg, prims))
        .map_err(|e| e.to_string())?;
    for (t, x, y) in &tiles {
        blit(&mut canvas, w, t, CELL, CELL, *x, *y);
    }
    qp_render::encode_png(&canvas, w, h).map_err(|e| e.to_string())
}

const KINDS_TEXT: &str = "The six meaning kinds:
- continuation: the move goes on
- reversal: the move may be turning
- breakout: a boundary was crossed
- squeeze: the move is narrowing; direction unknown
- extreme: an unusual stretch (overbought / oversold)
- flow: money flowing in or out, independent of price

Fields to answer for every sign:
- kind: one of continuation, reversal, breakout, squeeze, extreme, flow
- direction: up, down or neutral
- strength: low or high
- phase: candidate (seen on an unfinished bar), final (confirmed, still open), realized (closed: it came true), failed (closed: it did not come true) or retracted (closed: withdrawn because its data was wrong)";

/// The reader prompt. `@DIR@` is replaced by `run_readers.sh` with the absolute path of the
/// directory that holds only the sheet images.
#[must_use]
pub fn prompt(condition: &str, legend: &str) -> String {
    let mut p = String::from(
        "You are taking a reading test. Signs from a market \"meaning alphabet\" are shown in \
         an image: each small sign stands for one observation about a stock's chart. The \
         rules of the signs are not given; infer them from the signs themselves.\n\n",
    );
    p.push_str(KINDS_TEXT);
    p.push_str("\n\n");
    if condition == "legend" {
        p.push_str("The alphabet's own short explanation:\n\"");
        p.push_str(legend);
        p.push_str("\"\n\n");
    }
    let example = "{\"A1\": {\"kind\": \"breakout\", \"direction\": \"up\", \"strength\": \"high\", \"phase\": \"final\"}, \"A2\": {...}, ...}";
    if condition == "learn" {
        p.push_str(
            "First open @DIR@/examples.png with the Read tool: it shows five example signs \
             (E1 to E5), each with its correct answer written next to it. Then open \
             @DIR@/test.png: it shows 12 new signs in a grid labelled A1 to C4 (row letter, \
             column number). Answer for the 12 new signs.\n\n",
        );
    } else {
        p.push_str(
            "Open @DIR@/sheet.png with the Read tool. It shows 16 signs in a 4x4 grid labelled \
             A1 to D4 (row letter, column number; the label is above its sign).\n\n",
        );
    }
    p.push_str(
        "Give your best guess for every field of every label, even when unsure. Reply with a \
         single JSON object and nothing else, keyed by label, for example:\n",
    );
    p.push_str(example);
    p.push('\n');
    p
}

/// Number of words of a legend.
#[must_use]
pub fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Writes the sheets, the prompt and the hidden key for one condition and seed.
/// Returns the sheet directory.
///
/// # Errors
/// Unknown condition, a legend over 120 words, rasterizer or I/O failure.
pub fn write_condition(
    entry: &M1Entry,
    root: &Path,
    condition: &str,
    seed: u64,
) -> Result<std::path::PathBuf, String> {
    let alt = entry.alphabet.name();
    crate::io::check_name(alt)?;
    if !CONDITIONS.contains(&condition) {
        return Err(format!(
            "unknown condition `{condition}` ({})",
            CONDITIONS.join(", ")
        ));
    }
    if condition == "legend" && word_count(entry.legend) > LEGEND_MAX_WORDS {
        return Err(format!(
            "legend of `{alt}` has {} words (max {LEGEND_MAX_WORDS})",
            word_count(entry.legend)
        ));
    }
    let raw = super::raw_dir(root, alt);
    let tag = format!("{condition}_s{seed}");
    let dir = raw.join("sheets").join(&tag);
    let (examples, items) = if condition == "learn" {
        let (ex, test) = learn_items(seed);
        write(&dir.join("examples.png"), &examples_png(entry, &ex)?)?;
        write(&dir.join("test.png"), &grid_png(entry, &test)?)?;
        (ex, test)
    } else {
        let items = read_items(seed);
        write(&dir.join("sheet.png"), &grid_png(entry, &items)?)?;
        (Vec::new(), items)
    };
    write(
        &raw.join("prompts").join(format!("{tag}.txt")),
        prompt(condition, entry.legend).as_bytes(),
    )?;
    let key = Key {
        harness: crate::HARNESS_VERSION.to_owned(),
        alt: alt.to_owned(),
        condition: condition.to_owned(),
        seed,
        examples: examples.iter().map(|(l, s)| truth(l.clone(), s)).collect(),
        items: items.iter().map(|(l, s)| truth(l.clone(), s)).collect(),
    };
    write_json(&raw.join("keys").join(format!("{tag}.json")), &key)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_sheet_is_seeded() {
        let a = read_items(3);
        assert_eq!(a.len(), 16);
        assert_eq!(a, read_items(3));
        assert_ne!(
            a.iter().map(|x| spec_id(&x.1)).collect::<Vec<_>>(),
            read_items(4)
                .iter()
                .map(|x| spec_id(&x.1))
                .collect::<Vec<_>>()
        );
        assert_eq!(a[0].0, "A1");
        assert_eq!(a[15].0, "D4");
    }

    #[test]
    fn learn_rules() {
        for seed in 0..50 {
            let (ex, test) = learn_items(seed);
            assert_eq!(ex.len(), 5);
            assert_eq!(test.len(), 12);
            let mut kinds: Vec<_> = ex.iter().map(|e| e.1.kind).collect();
            kinds.sort();
            kinds.dedup();
            assert!(kinds.len() >= 4, "seed {seed}");
            for (_, t) in &test {
                assert!(!ex.iter().any(|(_, e)| e.kind == t.kind
                    && e.direction == t.direction
                    && e.phase == t.phase));
            }
        }
    }

    #[test]
    fn prompt_has_no_answers() {
        let p = prompt("legend", "LEGEND");
        assert!(p.contains("LEGEND") && p.contains("@DIR@/sheet.png"));
        assert!(!prompt("nolegend", "LEGEND").contains("LEGEND"));
        assert!(prompt("learn", "").contains("@DIR@/examples.png"));
    }
}
