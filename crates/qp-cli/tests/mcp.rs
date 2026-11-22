//! `qp mcp` without the RD1 adapter, and `qp conform --transport mcp`, end to end.

use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// (RD2/A62) The RD1 one-principal adapter `qp mcp --init … --principal …` is gone: the
/// flags are refused, `qp mcp` alone runs nothing, and `qp mcp --help` offers only
/// `serve` (the R5B server, agent principals only).
#[test]
fn the_rd1_adapter_is_removed() {
    let qp = || {
        let mut c = Command::cargo_bin("qp").unwrap();
        c.current_dir(root());
        c
    };
    for args in [
        vec![
            "mcp",
            "--init",
            "fixtures/protocol/init/thyao.json",
            "--principal",
            "arkin",
        ],
        vec!["mcp", "--principal", "arkin", "--surface", "desktop"],
        vec!["mcp"],
    ] {
        let out = qp().args(&args).write_stdin("").output().unwrap();
        assert!(!out.status.success(), "{args:?} was accepted");
        assert!(out.stdout.is_empty(), "{args:?} wrote to stdout");
    }
    let out = qp()
        .args(["mcp", "--init", "fixtures/protocol/init/thyao.json"])
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unexpected argument '--init'"), "{err}");
    let help = qp().args(["mcp", "--help"]).output().unwrap();
    assert!(help.status.success());
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("serve"), "{help}");
    for gone in ["--init", "--principal", "RD1"] {
        assert!(
            !help.contains(gone),
            "`qp mcp --help` still offers {gone}: {help}"
        );
    }
    // `qp mcp` itself takes no option but `--help`.
    let options = help.split("Options:").nth(1).unwrap_or_default();
    let flags: Vec<&str> = options
        .lines()
        .filter_map(|l| l.split_whitespace().find(|w| w.starts_with("--")))
        .collect();
    assert_eq!(flags, ["--help"], "{help}");
}

#[test]
fn conform_over_mcp() {
    let out = Command::cargo_bin("qp")
        .unwrap()
        .current_dir(root())
        .args([
            "conform",
            "fixtures/protocol/conformance",
            "crates/qp-session/tests/transcripts",
            "--transport",
            "mcp",
        ])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains(" 0 failed (mcp)"), "{text}");
}
