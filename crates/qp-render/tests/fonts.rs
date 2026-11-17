//! (R5/T3) The embedded font files (`qp_render::fonts::FILES`) and the looks' font assets
//! (`qp_grammar::look::FONTS`) name the same assets, every file parses, its licence file is
//! in the repository, and every look's roles name an embedded asset.

use std::path::Path;

#[test]
fn font_registry_matches_the_looks() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/fonts");
    let files: Vec<&str> = qp_render::fonts::FILES.iter().map(|(id, _)| *id).collect();
    let assets: Vec<&str> = qp_grammar::look::FONTS.iter().map(|f| f.id).collect();
    assert_eq!(
        files, assets,
        "fonts::FILES and look::FONTS must list the same ids"
    );
    assert_eq!(files[0], qp_grammar::look::DEFAULT_FONT);
    for (i, a) in qp_grammar::look::FONTS.iter().enumerate() {
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
    for l in qp_grammar::look::LOOKS {
        for f in l.fonts() {
            assert!(files.contains(&f.id), "{}: font {}", l.name, f.id);
        }
    }
    assert_eq!(qp_render::fonts::index(Some("no-such-font")), 0);
    assert_eq!(qp_render::fonts::index(None), 0);
}
