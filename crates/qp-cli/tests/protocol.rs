//! `qp serve` and `qp conform` end to end.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::cognitive_complexity
)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn qp() -> Command {
    Command::cargo_bin("qp").expect("qp binary is built")
}

#[test]
fn conform_both_transports() {
    for transport in ["inproc", "serve"] {
        let mut args = vec!["conform", "crates/qp-session/tests/transcripts"];
        if root().join("fixtures/protocol/conformance").is_dir() {
            args.push("fixtures/protocol/conformance");
        }
        let out = qp()
            .current_dir(root())
            .args(&args)
            .args(["--transport", transport])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "{transport}: {text}");
        assert!(text.contains(" 0 failed"), "{text}");
    }
}

#[test]
fn conform_fails_on_a_wrong_transcript() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("bad.json");
    std::fs::write(
        &f,
        r#"{"name":"bad","description":"d","init":"fixtures/protocol/init/thyao.json","steps":[
        {"send_raw":{"conn":"a","line":"{x"},"expect":[]}]}"#,
    )
    .unwrap();
    let out = qp()
        .current_dir(root())
        .arg("conform")
        .arg(&f)
        .args(["--transport", "serve"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("FAIL bad: step 0"));
}

#[test]
fn serve_frames_and_batch_end() {
    let input = concat!(
        "not json\n",
        r#"{"conn":"a","msg":{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"desktop"}}}"#,
        "\n",
        r#"{"conn":"a","msg":"{broken"}"#,
        "\n",
        r#"{"conn":"a","msg":{"type":"query","id":"q","query":{"kind":"get","id":"px"}}}"#,
        "\n"
    );
    let out = qp()
        .current_dir(root())
        .args([
            "serve",
            "--init",
            "fixtures/protocol/init/thyao.json",
            "--batch-end",
        ])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let lines: Vec<serde_json::Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 8);
    assert_eq!(lines[0]["conn"], "");
    assert_eq!(lines[0]["msg"]["error"]["code"], "parse");
    assert_eq!(lines[1], serde_json::json!({"batch_end": true}));
    assert_eq!(lines[2]["msg"]["type"], "welcome");
    assert_eq!(lines[4]["msg"]["error"]["code"], "parse");
    assert_eq!(lines[6]["msg"]["data"]["collection"], "series");
}

#[test]
fn serve_refuses_a_document_outside_the_root() {
    let dir = tempfile::tempdir().unwrap();
    let init = dir.path().join("init.json");
    std::fs::write(
        &init,
        r#"{"document":"../../etc/passwd","principals":[],"clock":0}"#,
    )
    .unwrap();
    let out = qp()
        .current_dir(root())
        .arg("serve")
        .arg("--init")
        .arg(&init)
        .write_stdin("")
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("inside the root"));
}

/// §18.4 (RP2 T4 audit O1): a `seq` gap on the host's frame path is written back to the
/// host as `{"host":{"gap":…}}`; the binding is stale.
#[test]
fn serve_reports_a_gap_to_the_host() {
    let t: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            root().join("fixtures/protocol/conformance/32-verify-evidence.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let init = dir.path().join("init.json");
    std::fs::write(&init, t["init"].to_string()).unwrap();
    let input = concat!(
        r#"{"host":{"data":{"feed":{"symbol":"THYAO","interval":"1d"},"seq":7,"change":{"kind":"patch","t":1774396800000,"fields":{"v":1.0}}}}}"#,
        "\n",
        r#"{"conn":"a","msg":{"type":"hello","protocol":"qp/0","principal":"arkin","surface":{"kind":"headless"}}}"#,
        "\n",
        r#"{"conn":"a","msg":{"type":"query","id":"q","query":{"kind":"binding","series":"px"}}}"#,
        "\n"
    );
    let out = qp()
        .current_dir(root())
        .arg("serve")
        .arg("--init")
        .arg(&init)
        .args(["--batch-end", "--host-frames"])
        .write_stdin(input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let lines: Vec<serde_json::Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        lines[0],
        serde_json::json!({"host":{"gap":{"series":"px","feed":{"symbol":"THYAO","interval":"1d"},"expected":1,"got":7}}})
    );
    assert_eq!(lines[1], serde_json::json!({"batch_end": true}));
    let binding = lines
        .iter()
        .find(|l| l["msg"]["id"] == "q")
        .expect("binding answer");
    assert!(binding.to_string().contains("stale"), "{binding}");
}
