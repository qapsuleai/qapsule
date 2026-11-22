//! `qp scene`, `qp explain`, `qp look` and `qp render` (RG1: split out of `main.rs`).

use crate::*;

/// A session from `--init` (or the transcript's init), after the transcript's messages,
/// and the first principal of its init.
fn scene_session(src: &ViewSource) -> Result<(qp_session::Session, String), String> {
    // `send.msg` is forwarded raw, as the conformance runner does (selectors allowed).
    let transcript: Option<qp_session::conform::RunTranscript> = src
        .transcript
        .as_deref()
        .map(qp_session::conform::read_transcript)
        .transpose()?;
    let init: qp_schema::protocol::SessionInit = match (&src.init, &transcript) {
        (Some(p), _) => qp_session::read_json(p)?,
        (None, Some(t)) => qp_session::conform::load_init(&t.init, &src.root)?.init,
        (None, None) => return Err("give --init or --transcript".to_owned()),
    };
    let principal = init
        .principals
        .first()
        .map(|p| p.id.clone())
        .unwrap_or_default();
    let mut session = qp_session::Session::new(init, &src.root)?;
    for step in transcript.iter().flat_map(|t| &t.steps) {
        if let Some(line) = &step.send {
            let text = serde_json::to_string(&line.msg).map_err(|e| e.to_string())?;
            session.handle_raw(&line.conn, &text);
        } else if let Some(raw) = &step.send_raw {
            session.handle_raw(&raw.conn, &raw.line);
        }
    }
    Ok((session, principal))
}

/// Connection the CLI opens to ask the presenter.
const PRESENT_CONN: &str = "qp-cli-present";

/// The view the presenter returns for `finding` on a `surface` connection opened as
/// `principal`: `capsule_view` (capsule depth) or `view` (chart depth).
fn presented_view(
    session: &mut qp_session::Session,
    principal: &str,
    finding: &str,
    surface: &str,
) -> Result<qp_schema::grammar::View, String> {
    let hello = serde_json::json!({"type":"hello","protocol":"qp/0","principal":principal,
        "surface":{"kind":surface}});
    let first =
        |outs: Vec<qp_schema::protocol::OutboundLine>| -> Result<serde_json::Value, String> {
            let o = outs
                .into_iter()
                .find(|o| o.conn == PRESENT_CONN)
                .ok_or("no answer")?;
            serde_json::to_value(&o.msg).map_err(|e| e.to_string())
        };
    let w = first(session.handle_raw(PRESENT_CONN, &hello.to_string()))?;
    if w["type"] != "welcome" {
        return Err(format!(
            "hello as `{principal}` on `{surface}` refused: {w}"
        ));
    }
    let q = serde_json::json!({"type":"query","id":"present","query":{"kind":"present","finding":finding}});
    let r = first(session.handle_raw(PRESENT_CONN, &q.to_string()))?;
    let p = &r["data"]["presentation"];
    if r["type"] != "result" || p.is_null() {
        return Err(format!("present `{finding}` failed: {r}"));
    }
    let v = if p["capsule_view"].is_object() {
        &p["capsule_view"]
    } else if p["view"].is_object() {
        &p["view"]
    } else {
        return Err(format!(
            "the presentation of `{finding}` on `{surface}` (depth {}) has no view",
            p["depth"]
        ));
    };
    serde_json::from_value(v.clone()).map_err(|e| e.to_string())
}

/// Compiles the view `src` names at `width` x `height`; also returns the view (for
/// `explain`). `Err(Ok(e))` is a protocol error to print as JSON, `Err(Err(m))` a usage
/// or I/O error.
fn compile_source(
    src: &ViewSource,
    width: u32,
    height: u32,
    look: Option<&str>,
    roles: bool,
) -> Result<
    (qp_schema::grammar::View, qp_schema::scene::SceneDesc),
    Result<Box<qp_schema::protocol::ProtocolError>, String>,
> {
    let (mut session, principal) = scene_session(src).map_err(Err)?;
    let mut opts = qp_session::CompileOptions::new(width, height);
    opts.look = look.map(str::to_owned);
    opts.roles = roles;
    match (&src.view, &src.present) {
        (Some(view), _) => {
            let scene = session.scene(view, &opts).map_err(|e| Ok(Box::new(e)))?;
            let v = session
                .document()
                .views
                .iter()
                .find(|v| &v.id == view)
                .cloned()
                .ok_or_else(|| Err(format!("no view `{view}`")))?;
            Ok((v, scene))
        }
        (None, Some(finding)) => {
            let v = presented_view(&mut session, &principal, finding, &src.surface).map_err(Err)?;
            let scene = qp_grammar::compile(&v, &session.provider(), &opts)
                .map_err(|e| Err(e.to_string()))?;
            Ok((v, scene))
        }
        (None, None) => Err(Err("give --view or --present".to_owned())),
    }
}

fn print_protocol_error(e: &qp_schema::protocol::ProtocolError) -> Outcome {
    eprintln!(
        "qp: {}",
        serde_json::to_string(e).unwrap_or_else(|_| e.message.clone())
    );
    Outcome::Invalid
}

pub(crate) fn scene_cmd(args: &SceneArgs, explain: bool) -> Result<Outcome, String> {
    let (view, scene) = match compile_source(
        &args.source,
        args.width,
        args.height,
        args.look.as_deref(),
        args.roles,
    ) {
        Ok(x) => x,
        Err(Ok(e)) => return Ok(print_protocol_error(&e)),
        Err(Err(m)) => return Err(m),
    };
    if explain {
        print!("{}", qp_session::explain_scene(&view, &scene));
    } else {
        let text = if args.pretty {
            serde_json::to_string_pretty(&scene)
        } else {
            serde_json::to_string(&scene)
        }
        .map_err(|e| e.to_string())?;
        println!("{text}");
    }
    Ok(Outcome::Ok)
}

/// (R5/T3) `qp look`.
pub(crate) fn look_cmd(c: &LookCommand) -> Result<Outcome, String> {
    use qp_grammar::look;
    match c {
        LookCommand::List => {
            for l in look::LOOKS {
                println!("{}\t{}\t{}", l.name, l.scheme, l.note);
            }
        }
        LookCommand::Show { name, pretty } => {
            let l = look::by_name(Some(name)).map_err(|e| e.to_string())?;
            let v = l.to_json();
            let text = if *pretty {
                serde_json::to_string_pretty(&v)
            } else {
                serde_json::to_string(&v)
            }
            .map_err(|e| e.to_string())?;
            println!("{text}");
        }
        LookCommand::Css { name, theme } => {
            let l = look::by_name(Some(name)).map_err(|e| e.to_string())?;
            l.theme(theme.as_deref()).map_err(|e| e.to_string())?;
            for (k, v) in look::css_vars(l, theme.as_deref()) {
                println!("{k}: {v};");
            }
        }
    }
    Ok(Outcome::Ok)
}

pub(crate) fn render_view(args: &RenderArgs) -> Result<Outcome, String> {
    let scene = match compile_source(
        &args.source,
        args.width,
        args.height,
        args.look.as_deref(),
        false,
    ) {
        Ok((_, scene)) => scene,
        Err(Ok(e)) => return Ok(print_protocol_error(&e)),
        Err(Err(m)) => return Err(m),
    };
    let png = qp_render::render_scene_png(&scene).map_err(|e| e.to_string())?;
    std::fs::write(&args.out, png)
        .map_err(|e| format!("cannot write {}: {e}", args.out.display()))?;
    Ok(Outcome::Ok)
}
