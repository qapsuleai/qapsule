//! R6 VERIFY 3: two separate processes try to connect to the same endpoint at the same time
//! (mock broker on 127.0.0.1). The test binary re-runs itself as the two child processes.
//! Exactly one connects; the other does not start and says why (lock holder's PID).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use qp_live::auth::{AuthClient, AuthPolicy};
use qp_live::conn::{ConnState, ConnectOptions, LiveConnection};
use qp_live::mock::{mock_config_urls, BrokerScript, MockBroker, MockTokenService, RecordingSink};

const CHILD: &str = "QP_LIVE_TEST_CHILD";
const TOKEN_URL: &str = "QP_LIVE_TEST_TOKEN_URL";
const BROKER_URL: &str = "QP_LIVE_TEST_BROKER_URL";
const DIR: &str = "QP_LIVE_TEST_DIR";

fn result_file(dir: &Path, role: &str) -> PathBuf {
    dir.join(format!("result-{role}"))
}

fn wait_file(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// Child role. Does nothing unless started by the parent test with `QP_LIVE_TEST_CHILD`.
#[test]
fn child_process_entry() {
    let Ok(role) = std::env::var(CHILD) else {
        return;
    };
    let dir = PathBuf::from(std::env::var(DIR).unwrap());
    let cfg = mock_config_urls(
        &std::env::var(TOKEN_URL).unwrap(),
        &std::env::var(BROKER_URL).unwrap(),
    )
    .unwrap();
    let auth = Arc::new(AuthClient::new(&cfg, AuthPolicy::default()));
    let mut opts = ConnectOptions::new(vec!["mx/timestamp".into()]);
    opts.lock_dir = dir.join("locks");
    opts.poll_interval = Duration::from_millis(20);
    let out = result_file(&dir, &role);
    let tmp = dir.join(format!("tmp-{role}"));
    match LiveConnection::open(
        &cfg,
        cfg.market_endpoint(),
        auth,
        opts,
        RecordingSink::new(),
    ) {
        Ok(conn) => {
            let st = conn.wait_for(Duration::from_secs(10), |s| {
                matches!(s, ConnState::Connected { .. }) || s.is_final()
            });
            let line = match st {
                Some(ConnState::Connected { .. }) => {
                    format!("CONNECTED pid={}", std::process::id())
                }
                other => format!("FAILED {other:?}"),
            };
            std::fs::write(&tmp, &line).unwrap();
            std::fs::rename(&tmp, &out).unwrap();
            // Hold the connection until the parent has seen the other child's result.
            wait_file(&dir.join("release"), Duration::from_secs(30));
            conn.close().unwrap();
        }
        Err(e) => {
            println!("refused: {e}");
            std::fs::write(&tmp, format!("REFUSED {e}")).unwrap();
            std::fs::rename(&tmp, &out).unwrap();
        }
    }
}

#[test]
fn second_process_is_refused_by_machine_lock() {
    if std::env::var(CHILD).is_ok() {
        return;
    }
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());
    let dir = tempfile::tempdir().unwrap();
    let exe = std::env::current_exe().unwrap();
    let spawn = |role: &str| {
        Command::new(&exe)
            .args([
                "child_process_entry",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD, role)
            .env(TOKEN_URL, tokens.url())
            .env(BROKER_URL, broker.url("t-twoproc"))
            .env(DIR, dir.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    // Both start at the same time.
    let a = spawn("a");
    let b = spawn("b");
    let (pa, pb) = (a.id(), b.id());
    assert!(wait_file(
        &result_file(dir.path(), "a"),
        Duration::from_secs(30)
    ));
    assert!(wait_file(
        &result_file(dir.path(), "b"),
        Duration::from_secs(30)
    ));
    let ra = std::fs::read_to_string(result_file(dir.path(), "a")).unwrap();
    let rb = std::fs::read_to_string(result_file(dir.path(), "b")).unwrap();
    std::fs::write(dir.path().join("release"), "").unwrap();
    let (oa, ob) = (a.wait_with_output().unwrap(), b.wait_with_output().unwrap());
    assert!(oa.status.success() && ob.status.success());

    let (winner_pid, refused, refused_stdout) =
        match (ra.starts_with("CONNECTED"), rb.starts_with("CONNECTED")) {
            (true, false) => (pa, rb, String::from_utf8_lossy(&ob.stdout).to_string()),
            (false, true) => (pb, ra, String::from_utf8_lossy(&oa.stdout).to_string()),
            _ => panic!("expected exactly one connection: a={ra:?} b={rb:?}"),
        };
    println!("refused child: {refused}");
    assert!(refused.starts_with("REFUSED"), "{refused}");
    assert!(refused.contains("locked by another process"), "{refused}");
    assert!(refused.contains(&format!("pid {winner_pid}")), "{refused}");
    // The reason is on the refused process's own output too.
    assert!(
        refused_stdout.contains("locked by another process"),
        "{refused_stdout}"
    );

    let st = broker.stats();
    assert_eq!(st.sessions_accepted, 1);
    assert_eq!(st.connections_total, 1);
    assert_eq!(st.max_concurrent, 1);
    assert_eq!(
        tokens.requests(),
        1,
        "the refused process never asked for a token"
    );
}

/// A52 (K64): after this process halted on two unexplained server closes, a **new
/// process** does not open the endpoint either (the `.halted` marker; no token request,
/// no connection) until a person's reset with a note; then it connects.
#[test]
fn new_process_is_refused_by_the_unexplained_close_halt_until_reset() {
    if std::env::var(CHILD).is_ok() {
        return;
    }
    let tokens = MockTokenService::start_default(3600);
    let broker = MockBroker::start(BrokerScript::default());
    let dir = tempfile::tempdir().unwrap();
    let url = broker.url("t-a52-proc");
    let cfg = mock_config_urls(&tokens.url(), &url).unwrap();
    let auth = Arc::new(AuthClient::new(&cfg, AuthPolicy::default()));
    let clock = qp_live::halt::ManualClock::new();
    let mut opts = ConnectOptions::new(vec!["mx/timestamp".into()]);
    opts.lock_dir = dir.path().join("locks");
    opts.poll_interval = Duration::from_millis(20);
    opts.backoff.initial = Duration::from_millis(50);
    opts.clock = clock.clock();
    std::fs::create_dir_all(&opts.lock_dir).unwrap();
    let conn = LiveConnection::open(
        &cfg,
        cfg.market_endpoint(),
        auth,
        opts,
        RecordingSink::new(),
    )
    .unwrap();
    let wait = Duration::from_secs(10);
    let session = |n: u32| move |s: &ConnState| matches!(s, ConnState::Connected { session } if *session >= n);
    assert!(conn.wait_for(wait, session(1)).is_some());
    broker.drop_connections();
    assert!(conn.wait_for(wait, session(2)).is_some());
    clock.advance(Duration::from_secs(120));
    broker.drop_connections();
    assert!(conn
        .wait_for(wait, |s| matches!(s, ConnState::Halted(_)))
        .is_some());
    let key = conn.key().clone();
    let _ = conn.close();
    let (connections, token_requests) = (broker.stats().connections_total, tokens.requests());

    let exe = std::env::current_exe().unwrap();
    let run = |role: &str| {
        let child = Command::new(&exe)
            .args([
                "child_process_entry",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD, role)
            .env(TOKEN_URL, tokens.url())
            .env(BROKER_URL, &url)
            .env(DIR, dir.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        assert!(wait_file(
            &result_file(dir.path(), role),
            Duration::from_secs(30)
        ));
        std::fs::write(dir.path().join("release"), "").unwrap();
        assert!(child.wait_with_output().unwrap().status.success());
        let r = std::fs::read_to_string(result_file(dir.path(), role)).unwrap();
        let _ = std::fs::remove_file(dir.path().join("release"));
        r
    };
    let r = run("after-halt");
    assert!(r.starts_with("REFUSED"), "{r}");
    assert!(r.contains("unexplained server close"), "{r}");
    assert_eq!(broker.stats().connections_total, connections);
    assert_eq!(tokens.requests(), token_requests, "no token request");

    assert!(qp_live::halt::reset_halt(&dir.path().join("locks"), &key, "").is_err());
    assert!(
        qp_live::halt::reset_halt(&dir.path().join("locks"), &key, "test: person checked").is_ok()
    );
    let r = run("after-reset");
    assert!(r.starts_with("CONNECTED"), "{r}");
}
