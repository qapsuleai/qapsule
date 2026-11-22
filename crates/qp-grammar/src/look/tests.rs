use super::*;

#[test]
fn registry_is_well_formed() {
    assert_eq!(LOOKS[0].name, DEFAULT_LOOK);
    assert_eq!(LOOKS[0], R4);
    let mut seen = Vec::new();
    for l in LOOKS {
        assert!(!seen.contains(&l.name), "duplicate look {}", l.name);
        seen.push(l.name);
        assert!(l.name.starts_with(|c: char| c.is_ascii_lowercase()));
        assert!(l
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'));
        assert!(["dark", "light"].contains(&l.scheme), "{}", l.name);
        assert_eq!(l.dark.name, "dark");
        assert_eq!(
            l.dark.series[0], l.dark.accent,
            "{}: s1 is the accent",
            l.name
        );
        assert_eq!(
            l.light.series[0], l.light.accent,
            "{}: s1 is the accent",
            l.name
        );
        assert_eq!(l.light.name, "light");
        // Every role's font is embedded; the mark roles have distinct sizes.
        let mut sizes = Vec::new();
        for r in ROLES {
            let role = l.types.role(r).unwrap();
            assert!(
                font(role.font).is_some(),
                "{} {r}: font {}",
                l.name,
                role.font
            );
            assert!(role.px > 0.0 && role.line >= 1.0, "{} {r}", l.name);
            if ["headline", "title", "body", "label", "note"].contains(&r) {
                assert!(!sizes.contains(&role.px.to_bits()), "{} {r}", l.name);
                sizes.push(role.px.to_bits());
                assert_eq!(l.types.mark_role(role.px), Some(r));
            }
        }
        assert!(l.motion.duration_scale > 0.0);
        assert!(l.to_json()["type"]["headline"]["px"].is_number());
    }
    for f in FONTS {
        assert!(f.advance > 0.0 && f.ascent > 0.0, "{}", f.id);
    }
    assert_eq!(FONTS[0].id, DEFAULT_FONT);
    assert!(by_name(Some("nope")).is_err());
    assert_eq!(by_name(None).unwrap().name, "r4");
}

#[test]
fn ease_is_monotone_and_ends_fixed() {
    for e in [Ease::Linear, Ease::Bezier([0.2, 0.0, 0.0, 1.0])] {
        assert!(e.apply(0.0).abs() < 1e-9);
        assert!((e.apply(1.0) - 1.0).abs() < 1e-9);
        let mut prev = 0.0;
        for i in 0..=20 {
            let v = e.apply(f64::from(i) / 20.0);
            assert!(v + 1e-12 >= prev);
            prev = v;
        }
    }
    assert_eq!(Ease::Linear.apply(0.37).to_bits(), 0.37_f64.to_bits());
}

#[test]
fn r4_css_matches_the_former_stylesheet() {
    let d: std::collections::BTreeMap<_, _> = css_vars(&R4, None).into_iter().collect();
    assert_eq!(d["--bg"], "#0f1115");
    assert_eq!(d["--card"], "#161921");
    assert_eq!(d["--accent"], "#5b9cff");
    let l: std::collections::BTreeMap<_, _> = css_vars(&R4, Some("light")).into_iter().collect();
    assert_eq!(l["--bg"], "#f6f7f9");
    assert_eq!(l["--page"], "#c9ccd3");
    // (R5/T3b) The sizes the stylesheet held before they moved into the record.
    for (k, v) in [
        ("--qp-fs", "14px"),
        ("--qp-lh", "1.35"),
        ("--qp-fs-small", "12px"),
        ("--qp-fs-table", "13px"),
        ("--qp-fs-tiny", "11px"),
        ("--qp-fs-trust", "10px"),
        ("--qp-head-weight", "600"),
        ("--qp-head-tracking", "0.04em"),
        ("--qp-head-gap", "10px"),
        ("--qp-tab-h", "auto"),
        ("--qp-tab-pad", "6px"),
        ("--qp-tab-rule", "2px"),
        ("--qp-control-h", "auto"),
        ("--qp-control-pad", "4px 6px"),
        ("--qp-date-w", "120px"),
        ("--qp-bar-h", "48px"),
        ("--qp-cell-pad", "3px 4px"),
        ("--qp-badge-pad", "1px 6px"),
        ("--qp-face-pad", "14px 12px"),
        ("--qp-face-min-h", "60px"),
        ("--qp-rule", "1px"),
        ("--qp-space-0", "4px"),
        ("--qp-space-1", "6px"),
        ("--qp-space-2", "8px"),
        ("--qp-space-3", "12px"),
        ("--qp-space-5", "24px"),
    ] {
        assert_eq!(d[k], v, "{k}");
    }
}

/// (R5/T3b) A proportional file is measured glyph by glyph; a monospaced one keeps
/// R3's estimate bit for bit.
#[test]
fn text_width_uses_the_advance_table() {
    static PROP: Advances = Advances {
        upem: 1000,
        notdef: 500,
        mono: false,
        runs: &[(0x20, 0x20, 250), (0x69, 0x69, 230), (0x6d, 0x6d, 820)],
    };
    let prop = FontAsset {
        id: "prop",
        advances: &PROP,
        ..FONTS[0]
    };
    // "mi i": 820 + 230 + 250 + 230 = 1530 units; at 10 px: 15.3 → 16.
    assert_eq!(prop.width("mi i", 10.0), 16);
    // Unmapped: the missing glyph's advance.
    assert_eq!(prop.width("x", 10.0), 5);
    assert_eq!(prop.width("", 10.0), 1);
    let mono = &FONTS[0];
    assert!(mono.advances.mono);
    for (t, s) in [
        ("Benzerleri: 74'te 36 tuttu", 11.0),
        ("C 101.25", 16.0),
        ("ı", 9.0),
    ] {
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let r3 = ((t.chars().count() as f64 * s * 0.6).ceil() as i32).max(1);
        assert_eq!(mono.width(t, s), r3, "{t}");
    }
}

fn lum(c: [f64; 3]) -> f64 {
    let ch = |v: f64| {
        let v = v / 255.0;
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(c[0]) + 0.7152 * ch(c[1]) + 0.0722 * ch(c[2])
}

/// WCAG contrast of `fg` (with its alpha) over the opaque `bg`.
fn contrast(fg: Rgba, bg: Rgba) -> f64 {
    let a = f64::from(fg[3]) / 255.0;
    let mix = |i: usize| f64::from(bg[i]) + a * (f64::from(fg[i]) - f64::from(bg[i]));
    let (l1, l2) = (
        lum([mix(0), mix(1), mix(2)]),
        lum([0, 1, 2].map(|i| f64::from(bg[i]))),
    );
    (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
}

/// (R5/T3b) The trial look meets the hard thresholds from its record alone.
#[test]
fn ornek_meets_the_hard_thresholds() {
    let o = &ORNEK;
    for r in ROLES {
        assert!(o.types.role(r).unwrap().px >= 13.0, "{r}");
    }
    let c = &o.chrome;
    for px in [c.text_px, c.small_px, c.table_px, c.tiny_px, c.trust_px] {
        assert!(px >= 13.0);
    }
    assert!(c.tab_h >= 44.0 && c.control_h >= 44.0);
    assert!(
        c.bar_h >= c.control_h + 2.0 * 6.0,
        "the control bar holds a control"
    );
    for t in [o.dark, o.light] {
        let floor = [t.closed[0], t.closed[1], t.closed[2], t.closed_alpha];
        let k = contrast(floor, t.bg);
        assert!(k >= 3.0, "{}: closed glyph {k:.2}:1", t.name);
        let w = contrast(t.warn, t.bg);
        assert!(w >= 4.5, "{}: evidence note {w:.2}:1", t.name);
        let q = contrast(t.chrome.req, t.chrome.bg);
        assert!(q >= 4.5 || t.name == "dark", "{}: req {q:.2}:1", t.name);
    }
    // r4 keeps freshness alone.
    assert_eq!((R4.dark.closed_alpha, R4.light.closed_alpha), (0, 0));
}

/// CIE L* of an opaque colour (D65; what a grey filter keeps).
fn lstar(c: Rgba) -> f64 {
    let y = lum([f64::from(c[0]), f64::from(c[1]), f64::from(c[2])]);
    if y > 216.0 / 24389.0 {
        116.0 * y.cbrt() - 16.0
    } else {
        y * 24389.0 / 27.0
    }
}

/// CIEDE2000 of two greys (a* = b* = 0): the lightness term alone.
fn de00_grey(l1: f64, l2: f64) -> f64 {
    let lm = (l1 + l2) / 2.0;
    let s = 1.0 + 0.015 * (lm - 50.0).powi(2) / (20.0 + (lm - 50.0).powi(2)).sqrt();
    (l1 - l2).abs() / s
}

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
/// (R5/M16) `oltan` meets the hard thresholds from its record: type and touch sizes,
/// the closed glyph's floor, the evidence note, every chrome text pair; the meaning
/// colours (ink = `tg5` realized plate, up, down, the grey of no direction / no longer
/// / `tg5` failed) at least 3:1 on the ground and ΔE00 ≥ 10 apart in grey. (The CVD
/// simulations and the full table: `rounds/R5/experiments/M16/oltan/DESIGN.md`.)
#[test]
fn oltan_meets_the_hard_thresholds() {
    let o = &OLTAN;
    for r in ROLES {
        assert!(o.types.role(r).unwrap().px >= 13.0, "{r}");
    }
    let c = &o.chrome;
    for px in [c.text_px, c.small_px, c.table_px, c.tiny_px, c.trust_px] {
        assert!(px >= 13.0);
    }
    assert!(c.tab_h >= 44.0 && c.control_h >= 44.0);
    assert!(c.bar_h >= c.control_h + 2.0 * 6.0);
    // The headline and the ledger are drawn in different files' widths (semibold, regular).
    assert_eq!(o.role_font("headline").weight, 600);
    assert_eq!(o.role_font("body").weight, 400);
    for t in [o.dark, o.light] {
        let floor = [t.closed[0], t.closed[1], t.closed[2], t.closed_alpha];
        assert!(contrast(floor, t.bg) >= 3.0, "{}: closed floor", t.name);
        assert!(contrast(t.warn, t.bg) >= 4.5, "{}: evidence note", t.name);
        let ch = t.chrome;
        assert!(contrast(ch.req, ch.bg) >= 4.5, "{}: req", t.name);
        for (fg, bg, what) in [
            (t.text, t.bg, "text"),
            (t.muted, t.bg, "muted"),
            (ch.fg, ch.bg, "chrome fg"),
            (ch.muted, ch.bg, "chrome muted"),
            (ch.fg, ch.card, "chrome fg on card"),
            (ch.muted, ch.card, "chrome muted on card"),
            (ch.on_accent, ch.accent, "on accent"),
            (ch.note_fg, ch.note_bg, "note"),
            (ch.alert_fg, ch.alert_bg, "alert"),
        ] {
            let k = contrast(fg, bg);
            assert!(k >= 4.5, "{} {what}: {k:.2}", t.name);
        }
        // (R5/fix47, A46) The open capsule's headline when the claim no longer holds is
        // text (19 px semibold, 600 not counted as bold): 4.5:1 on the ground and the
        // surface. The chart's axis line is a non-text element: 3:1 (WCAG 1.4.11).
        let head = t
            .token(crate::open::tesir::NO_LONGER_HEADLINE)
            .unwrap_or([0; 4]);
        for bg in [t.bg, t.surface] {
            let k = contrast(head, bg);
            assert!(k >= 4.5, "{}: no-longer headline {k:.2}", t.name);
            let k = contrast(t.axis, bg);
            assert!(k >= 3.0, "{}: axis {k:.2}", t.name);
        }
        // (R5/fix47) Marks on a translucent band keep 3:1 against the band: the grey
        // strokes on a grey glyph's plate (at its ceiling), and every lane colour on the
        // story chart's selected row band.
        let over = |c: Rgba, a: u8| {
            let k = f64::from(a) / 255.0;
            let m = |i: usize| {
                (f64::from(t.bg[i]) + k * (f64::from(c[i]) - f64::from(t.bg[i]))).round()
            };
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let px = [m(0) as u8, m(1) as u8, m(2) as u8, 255];
            px
        };
        // (R5/fix47) A control's boundary: 3:1 on the chrome and on a card.
        for bg in [ch.bg, ch.card] {
            let k = contrast(ch.control, bg);
            assert!(k >= 3.0, "{}: control boundary {k:.2}", t.name);
        }
        assert!(t.grey_plate_alpha_max > 0, "{}", t.name);
        let plate = over(t.invalid, t.grey_plate_alpha_max);
        let k = contrast(t.invalid, plate);
        assert!(k >= 3.0, "{}: grey on its plate {k:.2}", t.name);
        let row = over(t.focus, o.story.focus_row_alpha);
        for (what, c) in [
            ("closed", t.closed),
            ("up", t.up),
            ("down", t.down),
            ("text", t.text),
        ] {
            let k = contrast(c, row);
            assert!(k >= 3.0, "{} {what} on the focus row {k:.2}", t.name);
        }
        assert_eq!(t.neutral, t.invalid, "{}: grey means no direction", t.name);
        // (M16 rebase) `tg5` draws a failed glyph's strongest stroke in `closed` at the
        // closed floor: it lands on `invalid`.
        let a = f64::from(t.closed_alpha) / 255.0;
        let mix = |i: usize| f64::from(t.bg[i]) + a * (f64::from(t.closed[i]) - f64::from(t.bg[i]));
        for i in 0..3 {
            assert!(
                (mix(i) - f64::from(t.invalid[i])).abs() <= 2.0,
                "{}",
                t.name
            );
        }
        let ladder = [t.text, t.up, t.down, t.invalid];
        for m in ladder {
            assert!(contrast(m, t.bg) >= 3.0, "{}: {m:?}", t.name);
        }
        for i in 0..ladder.len() {
            for j in i + 1..ladder.len() {
                let d = de00_grey(lstar(ladder[i]), lstar(ladder[j]));
                assert!(d >= 10.0, "{}: {i}/{j} grey ΔE00 {d:.1}", t.name);
            }
        }
    }
}

/// (R5/M16) `zemre` meets the hard thresholds from its record alone: no text under
/// 12 px (scene roles 13 px and up), tabs and controls 44 px or more, text colours at
/// least 4.5:1 on the background and the surface, meaning colours and the closed
/// glyph and an open glyph at freshness 0 at least 3:1, the evidence note and the
/// chrome's "requires" text 4.5:1.
#[test]
fn zemre_meets_the_hard_thresholds() {
    let z = &ZEMRE;
    for r in ROLES {
        assert!(z.types.role(r).unwrap().px >= 13.0, "{r}");
    }
    let c = &z.chrome;
    for px in [c.text_px, c.small_px, c.table_px, c.tiny_px, c.trust_px] {
        assert!(px >= 12.0);
    }
    assert!(c.tab_h >= 44.0 && c.control_h >= 44.0);
    assert!(c.bar_h >= c.control_h + 2.0 * 6.0);
    for t in [z.dark, z.light] {
        for bg in [t.bg, t.surface] {
            for (what, fg) in [("text", t.text), ("muted", t.muted), ("warn", t.warn)] {
                let k = contrast(fg, bg);
                assert!(k >= 4.5, "{} {what}: {k:.2}:1", t.name);
            }
            for (what, fg) in [
                ("up", t.up),
                ("down", t.down),
                ("neutral", t.neutral),
                ("invalid", t.invalid),
                ("axis", t.axis),
                ("accent", t.accent),
            ] {
                let k = contrast(fg, bg);
                assert!(k >= 3.0, "{} {what}: {k:.2}:1", t.name);
            }
        }
        // The title row's close is drawn in `up`/`down` at the title size: text.
        for fg in [t.up, t.down] {
            assert!(contrast(fg, t.bg) >= 4.5, "{}", t.name);
        }
        let floor = [t.closed[0], t.closed[1], t.closed[2], t.closed_alpha];
        assert!(contrast(floor, t.bg) >= 3.0, "{}: closed", t.name);
        // (R5/T3c) An open glyph at the least freshness keeps ≥ 3:1 in every direction
        // colour (the rozet's neutral is `text`); the alphabets' own check is
        // `tg5::tests::zemre_signs_keep_3_to_1_at_freshness_zero`.
        for (what, c) in [
            ("up", t.up),
            ("down", t.down),
            ("neutral", t.neutral),
            ("text", t.text),
        ] {
            let floor = [c[0], c[1], c[2], t.fresh_alpha_min];
            let k = contrast(floor, t.bg);
            assert!(k >= 3.0, "{} open {what} at freshness 0: {k:.2}:1", t.name);
        }
        let ch = t.chrome;
        for (what, fg, bg) in [
            ("fg", ch.fg, ch.bg),
            ("fg/card", ch.fg, ch.card),
            ("muted", ch.muted, ch.bg),
            ("muted/card", ch.muted, ch.card),
            ("req", ch.req, ch.bg),
            ("note", ch.note_fg, ch.note_bg),
            ("alert", ch.alert_fg, ch.alert_bg),
            ("on_accent", ch.on_accent, ch.accent),
        ] {
            let k = contrast(fg, bg);
            assert!(k >= 4.5, "{} chrome {what}: {k:.2}:1", t.name);
        }
    }
}
