//! (R5/T3) The embedded font files (`qp_render::fonts::FILES`) and the looks' font assets
//! (`qp_look::look::FONTS`) name the same assets, every file parses, its licence file is
//! in the repository, and every look's roles name an embedded asset.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::path::Path;

#[test]
fn font_registry_matches_the_looks() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
    let files: Vec<&str> = qp_render::fonts::FILES.iter().map(|(id, _)| *id).collect();
    let assets: Vec<&str> = qp_look::look::FONTS.iter().map(|f| f.id).collect();
    assert_eq!(
        files, assets,
        "fonts::FILES and look::FONTS must list the same ids"
    );
    assert_eq!(files[0], qp_look::look::DEFAULT_FONT);
    for (i, a) in qp_look::look::FONTS.iter().enumerate() {
        let bytes = std::fs::read(dir.join(a.file)).expect("font file in assets/fonts");
        assert_eq!(
            bytes,
            qp_render::fonts::FILES[i].1,
            "{}: file differs",
            a.id
        );
        assert!(
            dir.join(a.license).is_file(),
            "{}: licence {}",
            a.id,
            a.license
        );
        let f = qp_render::fonts::by_index(i).expect("parses");
        // Turkish letters and figures are in the file.
        for ch in "ğşıİöüçĞŞÖÜÇ0123456789".chars() {
            assert!(f.lookup_glyph_index(ch) != 0, "{}: no glyph for {ch}", a.id);
        }
    }
    for l in qp_look::look::LOOKS {
        for f in l.fonts() {
            assert!(files.contains(&f.id), "{}: font {}", l.name, f.id);
        }
    }
    assert_eq!(qp_render::fonts::index(Some("no-such-font")), 0);
    assert_eq!(qp_render::fonts::index(None), 0);
}

/// (R5/T3b) The advance table of embedded file `i` as fontdue reads it: `(upem, notdef,
/// runs)`, runs of consecutive code points mapping to glyphs of one advance.
fn advance_runs(i: usize) -> (u16, u16, Vec<(u32, u32, u16)>) {
    let f = qp_render::fonts::by_index(i).expect("parses");
    let upem = f.units_per_em();
    // At `px = units_per_em` fontdue's scale is exactly 1: the advance in font units.
    let units = |idx: u16| {
        let a = f.metrics_indexed(idx, upem).advance_width;
        assert!(
            a >= 0.0 && a <= f32::from(u16::MAX) && a.fract() == 0.0,
            "advance {a}"
        );
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let a = a as u16;
        a
    };
    let mut chars: Vec<(u32, u16)> = f
        .chars()
        .iter()
        .map(|(c, g)| (u32::from(*c), units(g.get())))
        .collect();
    chars.sort_unstable();
    let mut runs: Vec<(u32, u32, u16)> = Vec::new();
    for (c, a) in chars {
        match runs.last_mut() {
            Some(r) if r.1 + 1 == c && r.2 == a => r.1 = c,
            _ => runs.push((c, c, a)),
        }
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let upem = upem as u16;
    (upem, units(0), runs)
}

/// The text of `qp-look/src/advances.rs` for the embedded files.
fn advances_source() -> String {
    use std::fmt::Write as _;
    let mut s = String::from(
        "//! (R5/T3b) GENERATED from the embedded font files (`qp_render::fonts::FILES`) with \
         fontdue's\n//! metrics by `QP_BLESS=1 cargo test -p qp-render --test fonts`; do not \
         edit. That test fails\n//! when a file and its table disagree. \
         `look::FontAsset::advances` points here.\n\nuse crate::look::Advances;\n",
    );
    for (i, (id, _)) in qp_render::fonts::FILES.iter().enumerate() {
        let (upem, notdef, runs) = advance_runs(i);
        // Monospaced: every spacing glyph has the missing glyph's advance (combining
        // marks and controls have none).
        let mono = runs.iter().all(|r| r.2 == notdef || r.2 == 0);
        let name = id.to_uppercase().replace('-', "_");
        let _ = write!(
            s,
            "\n/// `{id}`: {} code points in {} runs.\npub static {name}: Advances = \
             Advances {{\n    upem: {upem},\n    notdef: {notdef},\n    mono: {mono},\n    \
             runs: &[\n",
            runs.iter().map(|r| r.1 - r.0 + 1).sum::<u32>(),
            runs.len()
        );
        for (a, b, w) in &runs {
            let _ = writeln!(s, "        (0x{a:04x}, 0x{b:04x}, {w}),");
        }
        s.push_str("    ],\n};\n");
    }
    s
}

#[test]
fn advance_tables_match_the_files() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../qp-look/src/advances.rs");
    let want = advances_source();
    if std::env::var_os("QP_BLESS").is_some() {
        std::fs::write(&path, &want).expect("write advances.rs");
    }
    let have = std::fs::read_to_string(&path).expect("advances.rs");
    assert!(
        have == want,
        "qp-look/src/advances.rs is stale: QP_BLESS=1 cargo test -p qp-render --test fonts"
    );
    for (i, a) in qp_look::look::FONTS.iter().enumerate() {
        let (upem, notdef, runs) = advance_runs(i);
        assert_eq!(a.advances.upem, upem, "{}", a.id);
        assert_eq!(a.advances.notdef, notdef, "{}", a.id);
        assert_eq!(a.advances.runs, runs.as_slice(), "{}: table", a.id);
        // Characters the compiler measures have fontdue's advance (mapped or not).
        let f = qp_render::fonts::by_index(i).expect("parses");
        for ch in "Aa iİıŞşĞğ0…—’₺%\u{4e2d}".chars() {
            let px = f.units_per_em();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let fd = f.metrics(ch, px).advance_width as u16;
            assert_eq!(a.advances.units(ch), fd, "{}: {ch}", a.id);
        }
        if a.advances.mono {
            // A monospaced file keeps R3's estimate: the mean advance is the advance.
            assert_eq!(
                a.advance.to_bits(),
                (f64::from(notdef) / f64::from(upem)).to_bits(),
                "{}: advance",
                a.id
            );
        }
    }
}
