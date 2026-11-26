//! (RL1, PS §3.3) `qp program check | run | explain`: answers, the full bar-by-bar output,
//! exit codes; and VERIFY 16 — `qp program check` and the MCP `program_check` tool give
//! the same answer on the same input (that half needs the program subsystem in the
//! session and is ignored until `qp-program`, rl1/program, is merged).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::{json, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn qp() -> Command {
    Command::cargo_bin("qp").expect("qp binary is built")
}

const SERIES_EVENT: &str =
    "r = rsi(close, 14)\nseries rsi_line = r\nevent up = cross_above(r, 70)\n";
const SIGNAL_STATE: &str = "r = rsi(close, 14)\n\
entry = cooldown(cross_below(r, 30), 12)\n\
signal rsi_dip = entry side al target 3% stop 1.5% max_bars 24 source \"rsi_dip\" \"1\"\n\
state zone { lvl = bucket(r, [30, 50, 70]) }\n";
const UNKNOWN_OP: &str = "r = rsj(close, 14)\nseries x = r\n";
const REJECT_TEXT: &str = "reject E_LOOKAHEAD \"the next close is not known\"\n";
const IR: &str = r#"{"version":"qp-ir/0","nodes":[{"id":"n0","op":"input","field":"close"},{"id":"n1","op":"rsi","x":"n0","period":14},{"id":"n2","op":"const","value":70.0},{"id":"n3","op":"cross_above","a":"n1","b":"n2"}],"outputs":[{"name":"up","kind":"event","node":"n3"}]}"#;
const BAD_IR: &str = r#"{"version":"qp-ir/0","nodes":[{"id":"cd","op":"input","field":"close"},{"id":"next","op":"lag","x":"cd","n":-1}],"outputs":[{"name":"x","kind":"series","node":"next"}]}"#;
const REJECT_IR: &str =
    r#"{"reject":{"code":"E_LOOKAHEAD","reason":"tomorrow's close is not known"}}"#;

/// The check cases: name, file content, language, expected exit code.
const CASES: [(&str, &str, Option<&str>, i32); 7] = [
    ("text.qp", SERIES_EVENT, None, 0),
    ("signal.qp", SIGNAL_STATE, Some("tr"), 0),
    ("unknown.qp", UNKNOWN_OP, Some("tr"), 1),
    ("reject.qp", REJECT_TEXT, None, 0),
    ("ir.json", IR, None, 0),
    ("bad.json", BAD_IR, Some("en"), 1),
    ("reject.json", REJECT_IR, None, 0),
];

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, content).unwrap();
    p
}

/// Runs `qp program <args>`; (exit code, stdout as JSON if it is one line of JSON).
fn run(args: &[&str]) -> (i32, String) {
    let out = qp().arg("program").args(args).output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
    )
}

fn check_json(file: &Path, lang: Option<&str>) -> (i32, Value) {
    let mut args = vec!["check", "--json"];
    if let Some(l) = lang {
        args.extend(["--lang", l]);
    }
    let f = file.to_str().unwrap();
    args.push(f);
    let (code, out) = run(&args);
    (code, serde_json::from_str(out.trim()).unwrap())
}

#[test]
fn check_answers_every_case_with_its_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    for (name, content, lang, want) in CASES {
        let f = write(dir.path(), name, content);
        let (code, v) = check_json(&f, lang);
        assert_eq!(code, want, "{name}: {v}");
        assert_eq!(v["kind"], "program_check", "{name}");
        assert_eq!(v["ok"], want == 0, "{name}: {v}");
        assert!(v["limits"]["program.try.max_bars"].is_u64(), "{name}: {v}");
    }
    let (_, v) = check_json(&write(dir.path(), "a.qp", SERIES_EVENT), None);
    assert!(v["ir"].is_object(), "text gives its IR: {v}");
    assert_eq!(v["outputs"][0]["name"], "rsi_line");
    assert_eq!(v["outputs"][1]["kind"], "event");
    let (_, v) = check_json(&write(dir.path(), "a.json", IR), None);
    assert!(v.get("ir").is_none(), "IR in, no IR out: {v}");
    let (_, v) = check_json(&write(dir.path(), "u.qp", UNKNOWN_OP), Some("tr"));
    assert_eq!(v["errors"][0]["reason"], "E_UNKNOWN_OP", "{v}");
    assert_eq!(v["errors"][0]["span"]["line"], 1, "{v}");
    assert_eq!(v["errors"][0]["did_you_mean"][0], "rsi", "{v}");
    let (_, en) = check_json(&write(dir.path(), "u2.qp", UNKNOWN_OP), Some("en"));
    assert_ne!(v["errors"][0]["message"], en["errors"][0]["message"]);
    let (_, v) = check_json(&write(dir.path(), "r.json", REJECT_IR), None);
    assert_eq!(v["reject"]["code"], "E_LOOKAHEAD", "{v}");
    let (_, v) = check_json(&write(dir.path(), "b.json", BAD_IR), None);
    assert_eq!(v["errors"][0]["reason"], "E_LOOKAHEAD", "{v}");
    // Plain text form.
    let (code, out) = run(&["check", dir.path().join("u.qp").to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(out.contains("E_UNKNOWN_OP 1:5"), "{out}");
    // A missing file is a usage error.
    let (code, _) = run(&["check", dir.path().join("nope.qp").to_str().unwrap()]);
    assert_eq!(code, 2);
}

/// The sample bar series file and its bar times.
fn sample() -> (String, Vec<i64>) {
    let data = root().join("fixtures/bars/sample.json");
    let bars: Value = serde_json::from_str(&std::fs::read_to_string(&data).unwrap()).unwrap();
    let times = bars["data"]["bars"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["t"].as_i64().unwrap())
        .collect();
    (data.to_str().unwrap().to_owned(), times)
}

#[test]
fn run_full_lists_series_and_events_bar_by_bar() {
    let dir = tempfile::tempdir().unwrap();
    let (d, times) = sample();
    let d = d.as_str();
    let f = write(dir.path(), "a.qp", SERIES_EVENT);
    let (code, out) = run(&["run", "--json", "--full", "--data", d, f.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["kind"], "program_run");
    // A closed window: every bar of the file, the last one too.
    assert_eq!(v["bars"], times.len());
    assert_eq!(v["to_t"], *times.last().unwrap());
    let s = &v["outputs"]["rsi_line"];
    assert_eq!(s["kind"], "series");
    assert_eq!(s["values"].as_array().unwrap().len(), times.len());
    assert!(s["values"][0].is_null() && s["values"][times.len() - 1].is_f64());
    let e = &v["outputs"]["up"];
    let ev: Vec<usize> = serde_json::from_value(e["bars"].clone()).unwrap();
    let et: Vec<i64> = serde_json::from_value(e["t"].clone()).unwrap();
    assert_eq!(ev.iter().map(|&i| times[i]).collect::<Vec<_>>(), et);
    assert!(ev.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn run_full_lists_signal_records_and_state_keys() {
    let dir = tempfile::tempdir().unwrap();
    let (d, times) = sample();
    let d = d.as_str();
    // Signal records (PR §6 form + bar indexes) and state keys with counts.
    let f = write(dir.path(), "s.qp", SIGNAL_STATE);
    let (code, out) = run(&["run", "--json", "--full", "--data", d, f.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(out.trim()).unwrap();
    let recs = v["outputs"]["rsi_dip"]["records"].as_array().unwrap();
    for r in recs {
        let bar = usize::try_from(r["bar"].as_u64().unwrap()).unwrap();
        assert_eq!(r["born_t"], times[bar], "{r}");
        assert_eq!(r["mode"], "replay");
        assert_eq!(r["label"], "simulated");
        if let Some(x) = r["exit_bar"].as_u64() {
            assert!(x > bar as u64, "{r}");
        }
    }
    let st = &v["outputs"]["zone"];
    let keys = st["keys"].as_array().unwrap();
    assert_eq!(keys.len(), times.len());
    let counted: u64 = st["counts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["count"].as_u64().unwrap())
        .sum();
    assert_eq!(counted, keys.iter().filter(|k| !k.is_null()).count() as u64);
}

#[test]
fn run_without_full_is_the_try_summary_and_errors_do_not_run() {
    let dir = tempfile::tempdir().unwrap();
    let (d, _) = sample();
    let d = d.as_str();
    let f = write(dir.path(), "s.qp", SIGNAL_STATE);
    // Without --full: the try summary.
    let (code, out) = run(&["run", "--json", "--data", d, f.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["kind"], "program_try");
    assert_eq!(v["signal"][0]["name"], "rsi_dip");
    assert_eq!(v["state"][0]["name"], "zone");
    // A program with errors does not run (exit 1, the check answer); bad data is exit 2.
    let f = write(dir.path(), "u.qp", UNKNOWN_OP);
    let (code, out) = run(&["run", "--json", "--data", d, f.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(out.contains("E_UNKNOWN_OP"), "{out}");
    let (code, _) = run(&["run", "--data", f.to_str().unwrap(), f.to_str().unwrap()]);
    assert_eq!(code, 2);
}

#[test]
fn explain_prints_the_canonical_text() {
    let dir = tempfile::tempdir().unwrap();
    let f = write(dir.path(), "a.json", IR);
    let (code, out) = run(&["explain", "--json", f.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}");
    let v: Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["kind"], "program_explain");
    assert!(v["text"].as_str().unwrap().contains("cross_above("), "{v}");
    assert_eq!(v["cost"].as_array().unwrap().len(), 4);
    let (code, out) = run(&["explain", f.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(out.contains("event up ="), "{out}");
    let r = write(dir.path(), "r.json", REJECT_IR);
    let (code, _) = run(&["explain", r.to_str().unwrap()]);
    assert_eq!(code, 1, "a refusal has nothing to explain");
}

/// VERIFY 16: the MCP `program_check` answer (`structuredContent.data`) of a session with
/// default parameters equals `qp program check --json` on the same input.
#[test]
#[ignore = "needs qp-program (rl1/program)"]
fn cli_check_equals_mcp_program_check() {
    use qp_mcp::{surfaces, Config, Hub, McpServer, NotifyMode, Options};
    let r = root();
    let init = qp_session::read_json(&r.join("fixtures/protocol/init/thyao.json")).unwrap();
    let hub = Hub::shared(qp_session::Session::new(init, &r).unwrap());
    let surface = serde_json::from_value(json!({"kind": "headless"})).unwrap();
    let mut s = McpServer::open(
        hub,
        "m",
        Config::new("scout", surface),
        surfaces::by_name("thin").unwrap(),
        Options {
            notify: NotifyMode::Spec,
            common_tools: true,
            log: None,
            capsule_format: None,
        },
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    for (i, (name, content, lang, _)) in CASES.into_iter().enumerate() {
        let f = write(dir.path(), name, content);
        let (_, cli) = check_json(&f, lang);
        let program: Value = if name.ends_with(".json") {
            serde_json::from_str(content).unwrap()
        } else {
            json!(content)
        };
        let mut args = json!({"program": program});
        if let Some(l) = lang {
            args["lang"] = json!(l);
        }
        let out = s.handle_message(&json!({"jsonrpc":"2.0","id":i,"method":"tools/call",
            "params":{"name":"program_check","arguments":args,
            "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}));
        let mcp = &out[0]["result"]["structuredContent"]["data"];
        assert_eq!(&cli, mcp, "{name}");
    }
}

#[test]
fn oversized_files_and_broken_json_are_coded_errors() {
    let dir = tempfile::tempdir().unwrap();
    // The cap: max(program.text.max_source_chars × 4, program.max_ir_bytes) = 262144.
    let big = format!("# {}\nseries x = close\n", "a".repeat(262_144));
    let f = write(dir.path(), "big.qp", &big);
    let (code, v) = check_json(&f, None);
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["errors"][0]["reason"], "E_TOO_LARGE", "{v}");
    assert_eq!(v["errors"][0]["code"], "limit", "{v}");
    assert_eq!(v["errors"][0]["expected"][0], "262144", "{v}");
    // Starts like JSON but nests deeper than the parser takes: E_PARSE with a hint.
    let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
    let f = write(dir.path(), "deep.json", &deep);
    let (code, v) = check_json(&f, None);
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["errors"][0]["reason"], "E_PARSE", "{v}");
    assert!(v["errors"][0]["hint"].is_string(), "{v}");
    let f = write(dir.path(), "trunc.json", "{\"version\":\"qp-ir/0\",");
    let (code, v) = check_json(&f, None);
    assert_eq!(code, 1);
    assert_eq!(v["errors"][0]["reason"], "E_PARSE", "{v}");
    // `run` stops the same way before reading data-dependent work.
    let (d, _) = sample();
    let (code, out) = run(&["run", "--json", "--data", &d, f.to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(out.contains("E_PARSE"), "{out}");
}
