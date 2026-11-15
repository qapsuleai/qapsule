//! Contact sheet for the designer's own inspection:
//! `cargo run --example contact -- <out.png> [scale]`.
//! Rows: the 12 `K` members. Columns: 16/24/32 px light, 16/32 px dark, then 32 px light
//! channel variations. Upscaled by nearest neighbour.

use m1_siluet::Alphabet;
use qp_grammar::glyph::{Alphabet as _, GlyphBox};
use qp_grammar::theme::Theme;
use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind};
use qp_schema::scene::Prim;

fn base(kind: MeaningKind, direction: Direction) -> GlyphSpec {
    GlyphSpec {
        kind,
        direction,
        strength: 0.6,
        confidence: Some(0.6),
        phase: GlyphPhase::Final,
        freshness: 1.0,
        confirmations: 1,
        server: false,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "contact.png".to_owned());
    let k: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let kd = [
        (MeaningKind::Continuation, Direction::Up),
        (MeaningKind::Continuation, Direction::Down),
        (MeaningKind::Reversal, Direction::Up),
        (MeaningKind::Reversal, Direction::Down),
        (MeaningKind::Breakout, Direction::Up),
        (MeaningKind::Breakout, Direction::Down),
        (MeaningKind::Squeeze, Direction::Neutral),
        (MeaningKind::Extreme, Direction::Up),
        (MeaningKind::Extreme, Direction::Down),
        (MeaningKind::Flow, Direction::Up),
        (MeaningKind::Flow, Direction::Down),
        (MeaningKind::Flow, Direction::Neutral),
    ];
    type Var = fn(&mut GlyphSpec);
    let vars: Vec<Var> = vec![
        |s| s.phase = GlyphPhase::Candidate,
        |s| s.phase = GlyphPhase::Realized,
        |s| s.phase = GlyphPhase::Failed,
        |s| s.phase = GlyphPhase::Retracted,
        |s| s.strength = 0.2,
        |s| s.strength = 0.9,
        |s| s.confidence = None,
        |s| s.confidence = Some(0.3),
        |s| s.confidence = Some(0.8),
        |s| s.confirmations = 3,
        |s| s.confirmations = 5,
        |s| s.freshness = 0.3,
        |s| s.server = true,
    ];
    let cell = 40.0_f32;
    let cols = 5 + vars.len();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (w, h) = ((cell * cols as f32) as u32, (cell * kd.len() as f32) as u32);
    let light = Theme::light();
    let dark = Theme::dark();
    let mut prims: Vec<Prim> = Vec::new();
    for (row, (kind, dir)) in kd.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let y = row as f32 * cell + 4.0;
        let b = base(*kind, *dir);
        let mut col = 0usize;
        let mut put = |spec: &GlyphSpec, size: f32, theme: &Theme, col: usize| {
            #[allow(clippy::cast_precision_loss)]
            let x = col as f32 * cell + 4.0;
            if theme.name == "dark" {
                #[allow(clippy::cast_possible_truncation)]
                prims.push(Prim::Rect {
                    x: (col as f32 * cell) as i32,
                    y: (row as f32 * cell) as i32,
                    w: cell as u32,
                    h: cell as u32,
                    color: theme.bg,
                });
            }
            prims.extend(Alphabet.draw(spec, GlyphBox { x, y, size }, theme));
        };
        for size in [16.0, 24.0, 32.0] {
            put(&b, size, &light, col);
            col += 1;
        }
        for size in [16.0, 32.0] {
            put(&b, size, &dark, col);
            col += 1;
        }
        for v in &vars {
            let mut s = b.clone();
            v(&mut s);
            put(&s, 32.0, &light, col);
            col += 1;
        }
    }
    let scene = qp_render::scene::scene_of(w, h, light.bg, prims);
    let rgba = qp_render::render_scene_rgba(&scene).expect("render");
    let mut big = vec![0u8; (w * k * h * k * 4) as usize];
    for yy in 0..h * k {
        for xx in 0..w * k {
            let src = (((yy / k) * w + xx / k) * 4) as usize;
            let dst = ((yy * w * k + xx) * 4) as usize;
            big[dst..dst + 4].copy_from_slice(&rgba[src..src + 4]);
        }
    }
    let png = qp_render::encode_png(&big, w * k, h * k).expect("png");
    std::fs::write(&out, png).expect("write");
}
