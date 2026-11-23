//! `expand_op`, document arms: series, studies, views, layers by index, view range,
//! facts, findings, intents, narratives, presenter views, removal, bars. One success and
//! at least one error per arm (`ops-doc` book).

use serde_json::json;

use super::opcase::case;
use super::snap::Book;

const LAST_T: i64 = 1_759_881_600_000;
const DAY: i64 = 86_400_000;

fn bar(t: i64) -> serde_json::Value {
    json!({"t":t,"o":1.0,"h":2.0,"l":0.5,"c":1.5,"v":10.0})
}

#[test]
fn charac_session_ops_series_and_views() {
    let mut b = Book::new("ops-series-views");
    // put_series
    case(&mut b, "put_series.ok.inline_bars", &[], json!([{"op":"put_series","series":{"id":"q",
        "source":{"kind":"inline"},"data":{"kind":"bars","bars":[bar(1), bar(2)]}}}]));
    case(&mut b, "put_series.ok.inline_points", &[], json!([{"op":"put_series","series":{"id":"q",
        "source":{"kind":"inline"},"data":{"kind":"points","points":[{"t":1,"v":1.0},{"t":2,"v":null}]}}}]));
    case(&mut b, "put_series.ok.study", &[], json!([{"op":"put_series","series":{"id":"e9",
        "source":{"kind":"study","study":"ema","inputs":["px"],"params":{"period":9}}}}]));
    case(&mut b, "put_series.ok.replace", &[], json!([{"op":"put_series","series":{"id":"rsi14",
        "source":{"kind":"study","study":"rsi","inputs":["px"],"params":{"period":7}}}}]));
    case(&mut b, "put_series.err.duplicate", &[], json!([{"op":"put_series","series":{"id":"main",
        "source":{"kind":"inline"},"data":{"kind":"bars","bars":[bar(1)]}}}]));
    case(&mut b, "put_series.err.source", &[], json!([{"op":"put_series","series":{"id":"f",
        "source":{"kind":"file","path":"/etc/passwd"}}}]));
    case(&mut b, "put_series.err.no_data", &[], json!([{"op":"put_series","series":{"id":"q",
        "source":{"kind":"inline"}}}]));
    case(&mut b, "put_series.err.bad_bars", &[], json!([{"op":"put_series","series":{"id":"q",
        "source":{"kind":"inline"},"data":{"kind":"bars","bars":[{"t":1,"o":1.0,"h":0.5,"l":2.0,"c":1.0,"v":1.0}]}}}]));
    case(&mut b, "put_series.err.points_order", &[], json!([{"op":"put_series","series":{"id":"q",
        "source":{"kind":"inline"},"data":{"kind":"points","points":[{"t":2,"v":1.0},{"t":1,"v":1.0}]}}}]));
    case(&mut b, "put_series.err.study_output", &[], json!([{"op":"put_series","series":{"id":"e9",
        "source":{"kind":"study","study":"macd","inputs":["px"],"output":"histo"}}}]));
    case(&mut b, "put_series.err.study_data", &[], json!([{"op":"put_series","series":{"id":"e9",
        "source":{"kind":"study","study":"ema","inputs":["px"]},"data":{"kind":"bars","bars":[bar(1)]}}}]));
    case(&mut b, "put_series.err.bad_id", &[], json!([{"op":"put_series","series":{"id":"a b",
        "source":{"kind":"inline"},"data":{"kind":"bars","bars":[bar(1)]}}}]));
    // add_study
    case(&mut b, "add_study.ok.plain", &[], json!([{"op":"add_study","study":"sma","inputs":["px"],"params":{"period":5}}]));
    case(&mut b, "add_study.ok.named", &[], json!([{"op":"add_study","id":"s5","study":"sma","inputs":["px"],"params":{"period":5}}]));
    case(&mut b, "add_study.ok.show_panel", &[], json!([{"op":"add_study","study":"ema","inputs":["px"],"params":{"period":9},"show":{"view":"main","panel":0}}]));
    case(&mut b, "add_study.ok.show_new_panel", &[], json!([{"op":"add_study","study":"rsi","inputs":["px"],"params":{"period":7},"show":{"view":"main"}}]));
    case(&mut b, "add_study.ok.show_with", &[], json!([{"op":"add_study","study":"rsi","inputs":["px"],"params":{"period":7},"show":{"view":"main","with":"rsi14"}}]));
    case(&mut b, "add_study.err.input_kind", &[], json!([{"op":"add_study","study":"sma","inputs":["rsi14"]}]));
    case(&mut b, "add_study.err.unknown", &[], json!([{"op":"add_study","study":"emaa","inputs":["px"]}]));
    case(&mut b, "add_study.err.param", &[], json!([{"op":"add_study","study":"ema","inputs":["px"],"params":{"period":0}}]));
    case(&mut b, "add_study.err.unknown_param", &[], json!([{"op":"add_study","study":"ema","inputs":["px"],"params":{"perod":3}}]));
    case(&mut b, "add_study.err.input", &[], json!([{"op":"add_study","study":"ema","inputs":["pxx"]}]));
    case(&mut b, "add_study.err.show_view", &[], json!([{"op":"add_study","study":"ema","inputs":["px"],"show":{"view":"mian"}}]));
    case(&mut b, "add_study.err.id_taken", &[], json!([{"op":"add_study","id":"main","study":"ema","inputs":["px"]}]));
    // set_params
    case(&mut b, "set_params.ok", &[], json!([{"op":"set_params","series":"rsi14","params":{"period":10}}]));
    case(&mut b, "set_params.ok.partial", &[], json!([{"op":"set_params","series":"macd","params":{"fast":10}}]));
    case(&mut b, "set_params.err.not_study", &[], json!([{"op":"set_params","series":"px","params":{"period":10}}]));
    case(&mut b, "set_params.err.unknown_param", &[], json!([{"op":"set_params","series":"rsi14","params":{"perid":10}}]));
    case(&mut b, "set_params.err.bad_value", &[], json!([{"op":"set_params","series":"rsi14","params":{"period":-1}}]));
    case(&mut b, "set_params.err.missing", &[], json!([{"op":"set_params","series":"rsi","params":{"period":10}}]));
    // put_view
    case(&mut b, "put_view.ok.new", &[], json!([{"op":"put_view","view":{"id":"v2","x":{"kind":"time"},
        "y":[{"kind":"linear"}],"layers":[{"mark":"line","series":"px","panel":0,"encoding":{"y":"c"}}]}}]));
    case(&mut b, "put_view.ok.replace", &[], json!([{"op":"put_view","view":{"id":"main","x":{"kind":"time"},
        "y":[{"kind":"linear"}],"layers":[{"mark":"candle","series":"px","panel":0}]}}]));
    case(&mut b, "put_view.err.duplicate", &[], json!([{"op":"put_view","view":{"id":"px","x":{"kind":"time"},
        "y":[{"kind":"linear"}],"layers":[]}}]));
    case(&mut b, "put_view.err.layer_series", &[], json!([{"op":"put_view","view":{"id":"v2","x":{"kind":"time"},
        "y":[{"kind":"linear"}],"layers":[{"mark":"line","series":"nope","panel":0}]}}]));
    // add_layer / remove_layer (by index; deprecated forms)
    case(&mut b, "add_layer.ok.end", &[], json!([{"op":"add_layer","view":"main","layer":{"mark":"line","series":"bb20","panel":0,"encoding":{"y":"upper"}}}]));
    case(&mut b, "add_layer.ok.index", &[], json!([{"op":"add_layer","view":"main","index":1,"layer":{"mark":"line","series":"px","panel":0,"encoding":{"y":"c"}}}]));
    case(&mut b, "add_layer.err.index", &[], json!([{"op":"add_layer","view":"main","index":9,"layer":{"mark":"line","series":"px","panel":0}}]));
    case(&mut b, "add_layer.err.view", &[], json!([{"op":"add_layer","view":"mian","layer":{"mark":"line","series":"px","panel":0}}]));
    case(&mut b, "add_layer.err.panel", &[], json!([{"op":"add_layer","view":"main","layer":{"mark":"line","series":"px","panel":7}}]));
    case(&mut b, "remove_layer.ok", &[], json!([{"op":"remove_layer","view":"main","index":4}]));
    case(&mut b, "remove_layer.err.index", &[], json!([{"op":"remove_layer","view":"main","index":9}]));
    case(&mut b, "remove_layer.err.view", &[], json!([{"op":"remove_layer","view":"nope","index":0}]));
    // set_view_range
    case(&mut b, "set_view_range.ok", &[], json!([{"op":"set_view_range","view":"main","from_bar":10,"to_bar":50}]));
    case(&mut b, "set_view_range.err.past", &[], json!([{"op":"set_view_range","view":"main","from_bar":10,"to_bar":999}]));
    case(&mut b, "set_view_range.err.order", &[], json!([{"op":"set_view_range","view":"main","from_bar":60,"to_bar":50}]));
    case(&mut b, "set_view_range.err.view", &[], json!([{"op":"set_view_range","view":"nope","from_bar":1,"to_bar":2}]));
    b.check();
}

#[test]
fn charac_session_ops_findings() {
    let mut b = Book::new("ops-findings");
    // add_fact
    case(&mut b, "add_fact.ok", &[], json!([{"op":"add_fact","kind":"note","series":"px","bar":10,"text":"t","evidence":{"c":1.0}}]));
    case(&mut b, "add_fact.ok.named_study", &[], json!([{"op":"add_fact","id":"f1","kind":"note","series":"rsi14","bar":150,"text":"t"}]));
    case(&mut b, "add_fact.err.bar", &[], json!([{"op":"add_fact","kind":"note","series":"px","bar":999,"text":"t"}]));
    case(&mut b, "add_fact.err.series", &[], json!([{"op":"add_fact","kind":"note","series":"pxx","bar":1,"text":"t"}]));
    case(&mut b, "add_fact.err.evidence", &[], json!([{"op":"add_fact","kind":"note","series":"rsi14","bar":150,"text":"t","evidence":{"value":10.0}}]));
    // add_finding
    case(&mut b, "add_finding.ok", &[], json!([{"op":"add_finding","claim":"c","facts":["fact-rsi-cross70"],"confidence":0.5,
        "invalidation":{"kind":"threshold","series":"rsi14","op":"cross_below","value":60.0}}]));
    case(&mut b, "add_finding.ok.capsule", &[], json!([{"op":"add_finding","id":"f9","claim":"c","facts":["fact-rsi-cross70"],"confidence":0.5,
        "invalidation":{"kind":"threshold","series":"px","op":"lt","value":100.0},"capsule":"Başlık"}]));
    case(&mut b, "add_finding.err.no_facts", &[], json!([{"op":"add_finding","claim":"c","facts":[],"confidence":0.5,
        "invalidation":{"kind":"threshold","series":"rsi14","op":"lt","value":60.0}}]));
    case(&mut b, "add_finding.err.fact", &[], json!([{"op":"add_finding","claim":"c","facts":["fact-rsi-cross7"],"confidence":0.5,
        "invalidation":{"kind":"threshold","series":"rsi14","op":"lt","value":60.0}}]));
    case(&mut b, "add_finding.err.confidence", &[], json!([{"op":"add_finding","claim":"c","facts":["fact-rsi-cross70"],"confidence":1.5,
        "invalidation":{"kind":"threshold","series":"rsi14","op":"lt","value":60.0}}]));
    case(&mut b, "add_finding.err.condition", &[], json!([{"op":"add_finding","claim":"c","facts":["fact-rsi-cross70"],"confidence":0.5,
        "invalidation":{"kind":"threshold","series":"rsi","op":"lt","value":60.0}}]));
    // update_finding
    case(&mut b, "update_finding.ok.claim", &[], json!([{"op":"update_finding","finding":"fnd-overbought","claim":"yeni"}]));
    case(&mut b, "update_finding.ok.all", &[], json!([{"op":"update_finding","finding":"fnd-overbought","claim":"yeni","confidence":0.9,
        "invalidation":{"kind":"threshold","series":"px","op":"lt","value":100.0}}]));
    case(&mut b, "update_finding.err.nothing", &[], json!([{"op":"update_finding","finding":"fnd-overbought"}]));
    case(&mut b, "update_finding.err.missing", &[], json!([{"op":"update_finding","finding":"fnd-over","claim":"x"}]));
    case(&mut b, "update_finding.err.confidence", &[], json!([{"op":"update_finding","finding":"fnd-overbought","confidence":-0.1}]));
    // approve / reject
    case(&mut b, "approve_finding.ok", &[], json!([{"op":"approve_finding","finding":"fnd-overbought","note":"iyi"}]));
    case(&mut b, "approve_finding.err.missing", &[], json!([{"op":"approve_finding","finding":"nope"}]));
    case(&mut b, "reject_finding.ok", &[], json!([{"op":"reject_finding","finding":"fnd-overbought"}]));
    case(&mut b, "reject_finding.err.missing", &[], json!([{"op":"reject_finding","finding":"fnd-overbougt"}]));
    // add_intent / set_intent_active
    case(&mut b, "add_intent.ok.plain", &[], json!([{"op":"add_intent","text":"izle"}]));
    case(&mut b, "add_intent.ok.trigger", &[], json!([{"op":"add_intent","id":"i1","text":"izle",
        "trigger":{"kind":"threshold","series":"rsi14","op":"gt","value":70.0}}]));
    case(&mut b, "add_intent.err.trigger", &[], json!([{"op":"add_intent","text":"izle",
        "trigger":{"kind":"threshold","series":"rsx","op":"gt","value":70.0}}]));
    case(&mut b, "add_intent.err.id_taken", &[], json!([{"op":"add_intent","id":"px","text":"izle"}]));
    let mk = json!([{"op":"add_intent","id":"i1","text":"izle"}]);
    case(&mut b, "set_intent_active.ok", &[mk.clone()], json!([{"op":"set_intent_active","intent":"i1","active":false}]));
    case(&mut b, "set_intent_active.err.missing", &[mk], json!([{"op":"set_intent_active","intent":"i2","active":false}]));
    // add_narrative
    let step = |at: i64| json!({"at_ms":at,"target":{"view":"main"},"say":"s"});
    case(&mut b, "add_narrative.ok", &[], json!([{"op":"add_narrative","about":"fnd-overbought","steps":[step(0), step(500)]}]));
    case(&mut b, "add_narrative.err.empty", &[], json!([{"op":"add_narrative","about":"fnd-overbought","steps":[]}]));
    case(&mut b, "add_narrative.err.order", &[], json!([{"op":"add_narrative","about":"fnd-overbought","steps":[step(500), step(500)]}]));
    case(&mut b, "add_narrative.err.about", &[], json!([{"op":"add_narrative","about":"nope","steps":[step(0)]}]));
    case(&mut b, "add_narrative.err.target", &[], json!([{"op":"add_narrative","about":"fnd-overbought",
        "steps":[{"at_ms":0,"target":{"view":"nope"},"say":"s"}]}]));
    // show_finding
    case(&mut b, "show_finding.ok", &[], json!([{"op":"show_finding","finding":"fnd-overbought"}]));
    case(&mut b, "show_finding.ok.named", &[], json!([{"op":"show_finding","finding":"fnd-overbought","view":"pres"}]));
    case(&mut b, "show_finding.err.missing", &[], json!([{"op":"show_finding","finding":"nope"}]));
    case(&mut b, "show_finding.err.view_taken", &[], json!([{"op":"show_finding","finding":"fnd-overbought","view":"px"}]));
    b.check();
}

#[test]
fn charac_session_ops_remove_and_bars() {
    let mut b = Book::new("ops-remove-bars");
    // remove
    let uncap = json!([{"op":"remove","id":"cap-overbought"}]);
    case(&mut b, "remove.ok.finding", &[uncap.clone()], json!([{"op":"remove","id":"fnd-overbought"}]));
    case(&mut b, "remove.ok.series_layers", &[], json!([{"op":"remove","id":"macd"}]));
    case(&mut b, "remove.err.in_use_series", &[], json!([{"op":"remove","id":"rsi14"}]));
    case(&mut b, "remove.err.in_use_finding", &[], json!([{"op":"remove","id":"fnd-overbought"}]));
    case(&mut b, "remove.ok.view", &[], json!([{"op":"remove","id":"main"}]));
    case(&mut b, "remove.ok.capsule", &[], json!([{"op":"remove","id":"cap-overbought"}]));
    case(&mut b, "remove.err.in_use_fact", &[], json!([{"op":"remove","id":"fact-rsi-cross70"}]));
    case(&mut b, "remove.ok.fact", &[uncap, json!([{"op":"remove","id":"fnd-overbought"}])], json!([{"op":"remove","id":"fact-rsi-cross70"}]));
    case(&mut b, "remove.err.missing", &[], json!([{"op":"remove","id":"fnd-overbougt"}]));
    // append_bars
    case(&mut b, "append_bars.ok", &[], json!([{"op":"append_bars","series":"px","bars":[bar(LAST_T + DAY), bar(LAST_T + 2 * DAY)]}]));
    case(&mut b, "append_bars.err.order", &[], json!([{"op":"append_bars","series":"px","bars":[bar(LAST_T)]}]));
    case(&mut b, "append_bars.err.empty", &[], json!([{"op":"append_bars","series":"px","bars":[]}]));
    case(&mut b, "append_bars.err.study", &[], json!([{"op":"append_bars","series":"rsi14","bars":[bar(LAST_T + DAY)]}]));
    case(&mut b, "append_bars.err.bad_bar", &[], json!([{"op":"append_bars","series":"px",
        "bars":[bar(LAST_T + DAY), {"t":LAST_T + 2 * DAY,"o":1.0,"h":0.1,"l":2.0,"c":1.0,"v":1.0}]}]));
    // update_last_bar
    case(&mut b, "update_last_bar.ok", &[], json!([{"op":"update_last_bar","series":"px","bar":{"t":LAST_T,"o":125.26,"h":127.0,"l":124.47,"c":126.5,"v":1_500_000.0}}]));
    case(&mut b, "update_last_bar.err.time", &[], json!([{"op":"update_last_bar","series":"px","bar":bar(LAST_T + DAY)}]));
    case(&mut b, "update_last_bar.err.study", &[], json!([{"op":"update_last_bar","series":"macd","bar":bar(LAST_T)}]));
    case(&mut b, "update_last_bar.err.bad_bar", &[], json!([{"op":"update_last_bar","series":"px",
        "bar":{"t":LAST_T,"o":1.0,"h":0.1,"l":2.0,"c":1.0,"v":1.0}}]));
    // several ops in one command, the second failing: nothing is logged
    case(&mut b, "multi.err.second", &[], json!([{"op":"add_intent","text":"x"},{"op":"remove","id":"nope"}]));
    case(&mut b, "multi.ok", &[], json!([{"op":"add_study","study":"ema","inputs":["px"]},{"op":"add_intent","text":"x"},
        {"op":"set_params","series":"rsi14","params":{"period":21}}]));
    b.check();
}
