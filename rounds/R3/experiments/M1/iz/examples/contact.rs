//! Contact sheet for the designer's eye: `cargo run --example contact -- <out_dir>`.
//! Rows: the 12 `K` glyphs. Columns: final, candidate, realized, failed, retracted,
//! strength 0.2 / 0.9, confidence none / 0.3 / 0.8, freshness 0.3, 3 and 5 confirmations,
//! server mark. One PNG per size × theme, upscaled 4× nearest-neighbour.

use m1_iz::Alphabet;
use qp_grammar::glyph::{Alphabet as _, GlyphBox};
use qp_grammar::theme::Theme;
use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind};

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
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let k = [
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
    let variants: Vec<Box<dyn Fn(GlyphSpec) -> GlyphSpec>> = vec![
        Box::new(|s| s),
        Box::new(|s| GlyphSpec {
            phase: GlyphPhase::Candidate,
            ..s
        }),
        Box::new(|s| GlyphSpec {
            phase: GlyphPhase::Realized,
            ..s
        }),
        Box::new(|s| GlyphSpec {
            phase: GlyphPhase::Failed,
            ..s
        }),
        Box::new(|s| GlyphSpec {
            phase: GlyphPhase::Retracted,
            ..s
        }),
        Box::new(|s| GlyphSpec { strength: 0.2, ..s }),
        Box::new(|s| GlyphSpec { strength: 0.9, ..s }),
        Box::new(|s| GlyphSpec {
            confidence: None,
            ..s
        }),
        Box::new(|s| GlyphSpec {
            confidence: Some(0.3),
            ..s
        }),
        Box::new(|s| GlyphSpec {
            confidence: Some(0.8),
            ..s
        }),
        Box::new(|s| GlyphSpec {
            freshness: 0.3,
            ..s
        }),
        Box::new(|s| GlyphSpec {
            confirmations: 3,
            ..s
        }),
        Box::new(|s| GlyphSpec {
            confirmations: 5,
            ..s
        }),
        Box::new(|s| GlyphSpec { server: true, ..s }),
    ];
    for size in [16_u32, 24, 32] {
        for theme in [Theme::light(), Theme::dark()] {
            let pad = size / 2;
            let cell = size + pad;
            #[allow(clippy::cast_possible_truncation)]
            let (w, h) = (
                cell * variants.len() as u32 + pad,
                cell * k.len() as u32 + pad,
            );
            let mut prims = Vec::new();
            for (r, (kind, dir)) in k.iter().enumerate() {
                for (c, f) in variants.iter().enumerate() {
                    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
                    let bx = GlyphBox {
                        x: (pad + cell * c as u32) as f32,
                        y: (pad + cell * r as u32) as f32,
                        size: size as f32,
                    };
                    prims.extend(Alphabet.draw(&f(base(*kind, *dir)), bx, &theme));
                }
            }
            let scene = qp_render::scene::scene_of(w, h, theme.bg, prims);
            let rgba = qp_render::render_scene_rgba(&scene).expect("render");
            let kx = 4_usize;
            let (wu, hu) = (w as usize, h as usize);
            let mut up = vec![0u8; wu * kx * hu * kx * 4];
            for y in 0..hu * kx {
                for x in 0..wu * kx {
                    let s = ((y / kx) * wu + x / kx) * 4;
                    let d = (y * wu * kx + x) * 4;
                    up[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
                }
            }
            #[allow(clippy::cast_possible_truncation)]
            let png = qp_render::encode_png(&up, w * kx as u32, h * kx as u32).expect("png");
            let path = format!("{out}/contact_{size}_{}.png", theme.name);
            std::fs::write(&path, png).expect("write");
            println!("{path}");
        }
    }
}
