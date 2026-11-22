//! R2 views (`rounds/R2/tasks/VIEWS.md`, VERIFY 4): each view is built only from its
//! protocol transcript in `fixtures/views/`, rendered twice with `qp render`; both PNGs
//! are byte-identical and equal to the golden image. V0 equals the R0 golden
//! (`fixtures/golden/sample.png`, criterion 3). The `qp explain` dumps are stable and
//! match `fixtures/views/*.explain.txt`. The fixtures are what the generator produces.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

#[path = "support/views_gen.rs"]
mod views_gen;

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use qp_session::conform::{run_file, TransportKind};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn qp(args: &[String]) -> Vec<u8> {
    let out = Command::cargo_bin("qp")
        .expect("qp binary is built")
        .current_dir(root())
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "qp {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// (stem, `qp` arguments selecting the view and frame, golden path).
fn cases() -> Vec<(String, Vec<String>, String)> {
    let mut out = Vec::new();
    for (stem, view) in views_gen::VIEWS {
        let golden = if *stem == "v0-r0" {
            "fixtures/golden/sample.png".to_owned()
        } else {
            format!("fixtures/golden/views/{stem}.png")
        };
        out.push((
            (*stem).to_owned(),
            vec!["--view".into(), (*view).into()],
            golden,
        ));
    }
    for (stem, surface, w, h) in views_gen::DEPTHS {
        out.push((
            (*stem).to_owned(),
            vec![
                "--present".into(),
                views_gen::FINDING.into(),
                "--surface".into(),
                (*surface).into(),
                "--width".into(),
                w.to_string(),
                "--height".into(),
                h.to_string(),
            ],
            format!("fixtures/golden/views/{stem}.png"),
        ));
    }
    out
}

#[test]
fn fixtures_are_generated() {
    let root = root();
    for (rel, content) in views_gen::files(&root).unwrap() {
        let have = std::fs::read_to_string(root.join(&rel)).unwrap_or_default();
        assert!(
            have == content,
            "{rel} differs from the generator; run `cargo run -p qp-cli --example make_views`"
        );
    }
}

#[test]
fn transcripts_pass_in_process() {
    let root = root();
    for (stem, _, _) in cases() {
        let f = root.join(format!("fixtures/views/{stem}.json"));
        let o = run_file(&f, &root, &TransportKind::InProc);
        assert!(o.failure.is_none(), "{stem}: {:?}", o.failure);
    }
}

#[test]
fn views_render_twice_identically_and_match_their_goldens() {
    let root = root();
    let dir = tempfile::tempdir().unwrap();
    for (stem, sel, golden) in cases() {
        let mut pngs = Vec::new();
        for i in 0..2 {
            let out = dir.path().join(format!("{stem}-{i}.png"));
            let mut args = vec![
                "render".to_owned(),
                "--transcript".into(),
                format!("fixtures/views/{stem}.json"),
                "--out".into(),
                out.display().to_string(),
            ];
            args.extend(sel.iter().cloned());
            qp(&args);
            pngs.push(std::fs::read(&out).unwrap());
        }
        assert!(pngs[0] == pngs[1], "{stem}: two renders differ");
        let want = std::fs::read(root.join(&golden)).unwrap();
        assert!(pngs[0] == want, "{stem}: render differs from {golden}");
    }
}

#[test]
fn explain_dumps_are_stable() {
    let root = root();
    for (stem, sel, _) in cases() {
        let mut args = vec![
            "explain".to_owned(),
            "--transcript".into(),
            format!("fixtures/views/{stem}.json"),
        ];
        args.extend(sel.iter().cloned());
        let a = qp(&args);
        let b = qp(&args);
        assert_eq!(a, b, "{stem}: explain is not stable");
        let want = std::fs::read(root.join(format!("fixtures/views/{stem}.explain.txt"))).unwrap();
        assert!(
            a == want,
            "{stem}: explain differs from fixtures/views/{stem}.explain.txt:\n{}",
            String::from_utf8_lossy(&a)
        );
    }
}

#[test]
fn render_needs_a_view_or_a_presentation() {
    let out = Command::cargo_bin("qp")
        .unwrap()
        .current_dir(root())
        .args([
            "render",
            "--transcript",
            "fixtures/views/v0-r0.json",
            "--out",
            "/dev/null",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--view or --present"));
    let out = Command::cargo_bin("qp")
        .unwrap()
        .current_dir(root())
        .args([
            "render",
            "--transcript",
            "fixtures/views/d1-capsule.json",
            "--present",
            "fnd-overbought",
            "--surface",
            "watch",
            "--out",
            "/dev/null",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a watch presentation has no view");
    assert!(String::from_utf8_lossy(&out.stderr).contains("has no view"));
}
