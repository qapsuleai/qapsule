//! The product alphabet (`qp_grammar::glyph::siluet`) draws exactly what the R3/M1
//! experiment crate `m1-siluet` drew: a digest of every primitive over the full M1 glyph
//! set (6 kinds × 3 directions × 5 phases × 4 strengths × 4 confidences × 5
//! confirmations × 2 freshness/server = 14 400 specs) at 16, 24 and 32 px in both themes
//! is pinned below. The same digest computed with the experiment crate's `Alphabet` is
//! recorded in `rounds/R3/experiments/S/S-B.md` (one-off cross-check program there),
//! and the experiment crate stays untouched.

use qp_grammar::glyph::{alphabet, siluet::Siluet, Alphabet, GlyphBox};
use qp_grammar::Theme;
use qp_schema::capsule::{Direction, GlyphPhase, GlyphSpec, MeaningKind};
use qp_schema::scene::{Align, Prim};

/// FNV-1a 64 over a byte stream.
pub struct Fnv(pub u64);

impl Fnv {
    pub fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
    fn bytes(&mut self, b: &[u8]) {
        for x in b {
            self.0 ^= u64::from(*x);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn f32(&mut self, v: f32) {
        self.bytes(&v.to_bits().to_le_bytes());
    }
}

/// Digest of a primitive list (every field, floats by bit pattern).
pub fn digest_prims(h: &mut Fnv, prims: &[Prim]) {
    h.bytes(&(prims.len() as u64).to_le_bytes());
    for p in prims {
        match p {
            Prim::Rect { x, y, w, h: hh, color } => {
                h.bytes(b"R");
                h.bytes(&x.to_le_bytes());
                h.bytes(&y.to_le_bytes());
                h.bytes(&w.to_le_bytes());
                h.bytes(&hh.to_le_bytes());
                h.bytes(color);
            }
            Prim::Polyline {
                points,
                width,
                color,
            } => {
                h.bytes(b"P");
                h.bytes(&(points.len() as u64).to_le_bytes());
                for [x, y] in points {
                    h.f32(*x);
                    h.f32(*y);
                }
                h.f32(*width);
                h.bytes(color);
            }
            Prim::Fill { points, color } => {
                h.bytes(b"F");
                h.bytes(&(points.len() as u64).to_le_bytes());
                for [x, y] in points {
                    h.f32(*x);
                    h.f32(*y);
                }
                h.bytes(color);
            }
            Prim::Circle { cx, cy, r, color } => {
                h.bytes(b"C");
                h.f32(*cx);
                h.f32(*cy);
                h.f32(*r);
                h.bytes(color);
            }
            Prim::Text {
                x,
                baseline,
                px,
                text,
                color,
                align,
            } => {
                h.bytes(b"T");
                h.bytes(&x.to_le_bytes());
                h.bytes(&baseline.to_le_bytes());
                h.f32(*px);
                h.bytes(text.as_bytes());
                h.bytes(color);
                h.bytes(&[u8::from(*align == Align::Right)]);
            }
        }
    }
}

/// The full M1 glyph set (the experiment crate's `all_specs`, same order).
pub fn all_specs() -> Vec<GlyphSpec> {
    let mut v = Vec::new();
    for kind in MeaningKind::ALL {
        for direction in [Direction::Up, Direction::Down, Direction::Neutral] {
            for phase in [
                GlyphPhase::Candidate,
                GlyphPhase::Final,
                GlyphPhase::Realized,
                GlyphPhase::Failed,
                GlyphPhase::Retracted,
            ] {
                for strength in [0.0, 0.2, 0.6, 1.0] {
                    for confidence in [None, Some(0.0), Some(0.3), Some(1.0)] {
                        for confirmations in 1..=5 {
                            for (freshness, server) in [(1.0, false), (0.0, true)] {
                                v.push(GlyphSpec {
                                    kind,
                                    direction,
                                    strength,
                                    confidence,
                                    phase,
                                    freshness,
                                    confirmations,
                                    server,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    v
}

pub fn digest(a: &dyn Alphabet) -> u64 {
    let mut h = Fnv::new();
    for theme in [Theme::light(), Theme::dark()] {
        for spec in all_specs() {
            for size in [16.0, 24.0, 32.0] {
                let bx = GlyphBox {
                    x: 3.0,
                    y: 5.0,
                    size,
                };
                digest_prims(&mut h, &a.draw(&spec, bx, &theme));
            }
        }
    }
    h.0
}

/// Digest of `m1_siluet::Alphabet` over the same set (cross-check in S-B.md).
const M1_SILUET_DIGEST: u64 = 0x98b3_5851_ec19_049d;

#[test]
fn product_alphabet_is_m1_siluet() {
    assert_eq!(all_specs().len(), 14_400);
    assert_eq!(alphabet().name(), "siluet");
    let d = digest(&Siluet);
    assert_eq!(d, digest(alphabet()));
    assert_eq!(d, M1_SILUET_DIGEST, "digest {d:#018x}");
}
