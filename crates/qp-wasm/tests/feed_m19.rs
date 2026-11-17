//! R5/M19 (K53): the presentation's feed rule and orders, and the empty feed, in the shell
//! core — the core writes the empty feed's text (the shell only paints header rows).
#![cfg(feature = "replica")]

use qp_capsule::feedrule::{FeedRule, HideRule};
use qp_session::ReplayHost;
use qp_wasm::drive::parity_host;
use qp_wasm::shell::ShellCore;
use serde_json::{json, Value};

fn page(host: &mut ReplayHost, setup: impl FnOnce(&mut ShellCore)) -> ShellCore {
    let data = host.init().data.clone().expect("data");
    let evs = host.recorded_events().to_vec();
    let mut core = ShellCore::new("arkin", "tr", &data, &evs).expect("core");
    setup(&mut core);
    pump(&mut core, host);
    core
}

fn pump(core: &mut ShellCore, host: &mut ReplayHost) {
    for _ in 0..200 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            for l in host.handle_raw("w", &m.to_string()) {
                if l.conn == "w" {
                    let msg = serde_json::to_value(&l.msg).expect("msg");
                    core.on_frame(&json!({ "msg": msg }).to_string())
                        .expect("frame");
                }
            }
        }
    }
}

fn rows(core: &mut ShellCore) -> Vec<Value> {
    core.feed_json().as_array().cloned().unwrap_or_default()
}

#[test]
fn the_presentation_host_has_the_m19_rule_and_the_choices_list_it() {
    let host = parity_host();
    assert_eq!(
        host.init().feed_rule.as_ref().map(|f| f.rule.as_str()),
        Some(qp_capsule::feedrule::PRESENTED_FEED_RULE)
    );
    let ch = ShellCore::choices_json();
    assert_eq!(ch["feed_rule"][0], "rate55", "{ch}");
    assert_eq!(ch["feed_order"], json!(["grouped", "live_new"]), "{ch}");
    for o in ["grouped", "live_new"] {
        assert!(ch["feed"].as_array().unwrap().contains(&json!(o)));
    }
}

#[test]
fn feed_order_picks_the_layout() {
    let mut host = parity_host();
    let core = page(&mut host, |c| {
        c.set_look(&json!({"feed": "vakiro", "feed_order": "grouped"}));
    });
    let look = serde_json::to_value(core.look()).unwrap();
    assert_eq!(look["feed"], "grouped", "{look}");
    let core = page(&mut host, |c| c.set_look(&json!({"feed_order": "nope"})));
    let look = serde_json::to_value(core.look()).unwrap();
    assert_eq!(look["feed"], qp_grammar::feeds::PRESENTED_FEED, "{look}");
}

#[test]
fn an_empty_feed_says_so_and_why() {
    let mut host = parity_host();
    // A rule that hides every story with at least one closed case: what is left in the
    // feed has none (few cases are never hidden).
    host.set_principal_coin_flip_rule(
        "arkin",
        Some(FeedRule {
            hide: HideRule::Rate { min_n: 1, tau: 2.0 },
            exempt_negative: true,
        }),
    );
    let mut core = page(&mut host, |c| {
        c.set_audience(&json!({"group":"future","sensitivity":"important"}));
    });
    let symbols: Vec<String> = core.instruments();
    let mut said = None;
    for sym in &symbols {
        core.set_audience(&json!({"group":"future","sensitivity":"important","watch":[sym]}));
        pump(&mut core, &mut host);
        let r = rows(&mut core);
        if r.iter().all(|x| x.get("id").is_none()) {
            said = Some((sym.clone(), r));
            break;
        }
    }
    let (sym, r) = said.expect("some instrument's feed is empty under this rule");
    assert_eq!(r.len(), 2, "{r:?}");
    assert_eq!(r[0]["header"], "Şu an anlatacak bir şey yok.");
    let why = r[1]["header"].as_str().unwrap().to_owned();
    assert!(
        why.starts_with("Karnesi yazı turaya yakın ")
            && why.ends_with("kapsül gizlendi; en açık hassasiyette (Hepsi) işaretli görünür."),
        "{why}"
    );
    if why.contains("yazı turaya yakın") {
        // At the most open sensitivity the hidden ones are in the feed, marked.
        core.set_audience(&json!({"group":"future","sensitivity":"all","watch":[sym]}));
        pump(&mut core, &mut host);
        let r = rows(&mut core);
        assert!(r.iter().any(|x| x.get("id").is_some()), "{r:?}");
        assert!(
            r.iter().filter(|x| x.get("id").is_some()).any(|x| x["mark"]
                .as_str()
                .is_some_and(|m| m.ends_with("karnesi yazı tura"))),
            "{r:?}"
        );
    }
}
