use std::collections::BTreeMap;

use qp_session::ReplayHost;
use serde_json::{json, Value};

use super::{
    cand_label, candidates, tr, Build, CandOut, Layout, Shot, ShotOut, TopicOut, IMAGE_SCALE,
    PRINCIPAL, STEP, TOPICS,
};
use crate::shell::ShellCore;

/// Delivers what `core` queues to `host` on `conn` and the answers back, until idle.
fn pump(core: &mut ShellCore, host: &mut ReplayHost, conn: &str) {
    for _ in 0..1000 {
        let out = core.take_outbox();
        if out.is_empty() {
            break;
        }
        for m in out {
            for l in host.handle_raw(conn, &m.to_string()) {
                if l.conn == conn {
                    core.on_message(&l.msg);
                }
            }
        }
    }
}

struct Ctx {
    host: ReplayHost,
    lang: String,
    theme: String,
    next: u32,
    images: BTreeMap<String, Vec<u8>>,
}

impl Ctx {
    /// A new core on its own connection, at the host's moment, in the gallery's
    /// language and theme, painting at [`IMAGE_SCALE`].
    fn core(&mut self) -> (ShellCore, String) {
        self.next += 1;
        let conn = format!("g{}", self.next);
        let data = self.host.init().data.clone().unwrap_or_default();
        let evs = self.host.recorded_events();
        let mut core = ShellCore::new(PRINCIPAL, &self.lang, &data, &evs).expect("core");
        core.set_replay(self.host.state());
        core.set_dpr(IMAGE_SCALE);
        pump(&mut core, &mut self.host, &conn);
        core.set_style("", "", &self.theme);
        pump(&mut core, &mut self.host, &conn);
        (core, conn)
    }

    fn png(&mut self, name: String, rgba: &[u8], (w, h, k): (u32, u32, u32)) -> ShotOut {
        let png = qp_render::encode_png(rgba, w * k, h * k).unwrap_or_default();
        let src = format!("/gallery/img/{name}");
        self.images.insert(name, png);
        ShotOut {
            kind: "img".to_owned(),
            src: Some(src),
            css: Some([w, h]),
            text: None,
        }
    }
}

fn text(kind: &str, t: &str) -> ShotOut {
    ShotOut {
        kind: kind.to_owned(),
        src: None,
        css: None,
        text: Some(t.to_owned()),
    }
}

/// The feed's first capsule with a face scene under the presentation's settings: the
/// capsule every candidate opens.
fn reference_capsule(core: &mut ShellCore) -> Option<String> {
    core.feed_json().as_array().and_then(|rows| {
        rows.iter()
            .find(|r| r["scene"].as_bool() == Some(true))
            .and_then(|r| r["id"].as_str().map(str::to_owned))
    })
}

#[allow(clippy::too_many_lines)]
fn shots(
    cx: &mut Ctx,
    core: &mut ShellCore,
    conn: &str,
    prefix: &str,
    open: Option<&str>,
    list: &[Shot],
) -> (Vec<ShotOut>, String) {
    let lang = cx.lang.clone();
    let mut out = Vec::new();
    let mut depth = String::new();
    let mut n = 0usize;
    for s in list {
        match *s {
            Shot::Feed(k) => {
                let rows = core.feed_json();
                let rows = rows.as_array().cloned().unwrap_or_default();
                for r in rows.iter().take(k) {
                    if let Some(h) = r["header"].as_str() {
                        out.push(text("head", h));
                        continue;
                    }
                    let id = r["id"].as_str().unwrap_or("");
                    let px = if r["scene"].as_bool() == Some(true) {
                        core.face(id)
                    } else {
                        None
                    };
                    match px {
                        Some(scene) => {
                            let rgba = core.face_rgba(id, true);
                            let dims = (scene.width, scene.height, core.scale());
                            let shot = cx.png(format!("{prefix}-{n}.png"), &rgba, dims);
                            n += 1;
                            out.push(shot);
                        }
                        None => out.push(text("text", r["text"].as_str().unwrap_or(""))),
                    }
                }
                let (n_feed, hidden) = (core.feed().len(), core.coin_flip_hidden());
                out.push(text(
                    "text",
                    &if tr(&lang) {
                        format!("Akışta {n_feed} kapsül; yazı tura kuralının gizlediği: {hidden}.")
                    } else {
                        format!("{n_feed} capsules in the feed; hidden by the coin-flip rule: {hidden}.")
                    },
                ));
                depth = if tr(&lang) {
                    format!("akışın ilk {k} satırı")
                } else {
                    format!("the feed's first {k} rows")
                };
            }
            Shot::Face => {
                let Some(id) = open else { continue };
                if let Some(scene) = core.face(id) {
                    let rgba = core.face_rgba(id, true);
                    let dims = (scene.width, scene.height, core.scale());
                    let shot = cx.png(format!("{prefix}-{n}.png"), &rgba, dims);
                    n += 1;
                    out.push(shot);
                }
            }
            Shot::Capsule | Shot::Chart | Shot::Evidence => {
                let Some(id) = open else { continue };
                let to = match *s {
                    Shot::Capsule => "capsule",
                    Shot::Chart => "chart",
                    _ => "evidence",
                };
                if core.open_json()["depth"] == "sentence" {
                    core.open_capsule(id);
                    pump(core, &mut cx.host, conn);
                }
                if to != "capsule" {
                    core.open_to(to);
                    pump(core, &mut cx.host, conn);
                }
                let o = core.open_json();
                if let Some(t) = o["text"].as_str() {
                    // The open layer's title once (the depths below share it).
                    if !out
                        .iter()
                        .any(|x| x.kind == "head" && x.text.as_deref() == Some(t))
                    {
                        out.push(text("head", t));
                    }
                }
                if let Some(scene) = core.scene() {
                    let rgba = core.scene_rgba(true);
                    let dims = (scene.width, scene.height, core.scale());
                    let shot = cx.png(format!("{prefix}-{n}.png"), &rgba, dims);
                    n += 1;
                    out.push(shot);
                }
                if to == "evidence" {
                    for row in core.evidence_json().as_array().cloned().unwrap_or_default() {
                        out.push(text("text", &evidence_text(&row)));
                    }
                }
                depth = format!(
                    "{} {}",
                    o["capsule"].as_str().unwrap_or(id),
                    o["depth"].as_str().unwrap_or("")
                );
            }
        }
    }
    if depth.is_empty() {
        if let Some(id) = open {
            depth = format!("{id} {}", if tr(&lang) { "yüz" } else { "face" });
        }
    }
    (out, depth)
}

/// An evidence row as the core formatted it: `fact · name value · …`.
fn evidence_text(row: &Value) -> String {
    let mut parts = vec![row["fact"].as_str().unwrap_or("").to_owned()];
    for v in row["values"].as_array().cloned().unwrap_or_default() {
        let a = v.as_array().cloned().unwrap_or_default();
        let name = a.first().and_then(Value::as_str).unwrap_or("");
        let val = a.get(1).and_then(Value::as_str).unwrap_or("");
        parts.push(format!("{name} {val}"));
    }
    if let Some(r) = row["requires"].as_str() {
        parts.push(r.to_owned());
    }
    parts.join(" · ")
}

/// Builds the gallery for `lang` (`tr`, `en`) and `theme` (`dark`, `light`): the demo's
/// synthetic, paused world at [`STEP`]; per topic and candidate a fresh core with the
/// presentation's settings plus the candidate's, painted at the topic's depth.
///
/// # Panics
/// Never for the built-in synthetic world.
#[must_use]
pub fn build(lang: &str, theme: &str) -> Build {
    let lang = if lang == "en" { "en" } else { "tr" };
    let theme = if theme == "light" { "light" } else { "dark" };
    let mut host = crate::drive::parity_host();
    host.set_record_events(true);
    let mut cx = Ctx {
        host,
        lang: lang.to_owned(),
        theme: theme.to_owned(),
        next: 0,
        images: BTreeMap::new(),
    };
    // The moment: a seeker core moves the paused host to STEP.
    let (mut seeker, conn) = cx.core();
    seeker.seek(&STEP.to_string());
    pump(&mut seeker, &mut cx.host, &conn);
    let (mut reference, _) = cx.core();
    let open = reference_capsule(&mut reference);
    let st = reference.status_json();
    let step_w = if tr(lang) { "adım" } else { "step" };
    let moment = format!(
        "{step_w} {} · {}",
        st["step"].as_str().unwrap_or(""),
        st["time"].as_str().unwrap_or("")
    );
    let choices = ShellCore::choices_json();
    let mut topics = Vec::new();
    for t in TOPICS {
        let names = candidates(t, &choices);
        if names.len() < 2 {
            continue;
        }
        let mut cands = Vec::new();
        for c in &names {
            if t.key == "feed_rule" {
                let rule = qp_capsule::feedrule::FeedRule::named(c, true);
                cx.host.set_principal_coin_flip_rule(PRINCIPAL, rule);
            }
            let (mut core, conn) = cx.core();
            core.set_look(&json!({ t.key: c }));
            pump(&mut core, &mut cx.host, &conn);
            core.requery_feed();
            pump(&mut core, &mut cx.host, &conn);
            let prefix = format!("{lang}-{theme}-{}-{c}", t.key);
            let (shots, depth) =
                shots(&mut cx, &mut core, &conn, &prefix, open.as_deref(), t.shots);
            let s = core.status_json();
            let content = format!(
                "{step_w} {} · {} · {depth}",
                s["step"].as_str().unwrap_or(""),
                s["time"].as_str().unwrap_or("")
            );
            cands.push(CandOut {
                id: c.clone(),
                label: cand_label(t.key, c, lang),
                content,
                shots,
            });
            if t.key == "feed_rule" {
                cx.host.clear_principal_coin_flip_rule(PRINCIPAL);
            }
        }
        let i = usize::from(!tr(lang));
        topics.push(TopicOut {
            key: t.key.to_owned(),
            label: t.label[i].to_owned(),
            hint: t.hint[i].to_owned(),
            cands,
        });
    }
    Build {
        layout: Layout {
            lang: lang.to_owned(),
            theme: theme.to_owned(),
            moment,
            topics,
        },
        images: cx.images,
    }
}
