//! (R5B/T2, T5) Tools every surface gets from `qp mcp serve` (after the surface's own):
//!
//! - `render` — a view drawn to PNG by the reference renderer (`qp-render`), returned as
//!   MCP `image` content: the agent sees the chart it built, headless.
//! - `notifications` — the endpoint's pending `qp/0` notifications (the same as reading
//!   the `qapsule://notifications` resource), so that a client that cannot listen does
//!   not miss them.
//!
//! A surface that defines a tool of the same name wins (its tool is listed and called).

use serde::Deserialize;
use serde_json::{json, Value};

use crate::surfaces::{input_schema, Annotations, Ctx, ToolDef};
use crate::RpcError;

/// Largest frame side `render` draws.
pub const MAX_SIDE: u32 = 4096;
/// Smallest frame side `render` draws.
pub const MIN_SIDE: u32 = 64;

/// `render` arguments.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenderArgs {
    /// View id (see `query_list` with collection `views`).
    pub view: String,
    /// Frame width in pixels (default 1200, 64..=4096).
    #[serde(default)]
    pub width: Option<u32>,
    /// Frame height in pixels (default 600, 64..=4096).
    #[serde(default)]
    pub height: Option<u32>,
    /// Look (design record) to draw with, e.g. `r4` (the default).
    #[serde(default)]
    pub look: Option<String>,
    /// A pending proposal id: draws its preview (`docs/PROTOCOL.md` §25.6) — the
    /// proposal's operations applied on a copy of the document; nothing is written.
    #[serde(default)]
    pub proposal: Option<String>,
}

/// `notifications` arguments (none).
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NotificationsArgs {}

/// The common tools.
#[must_use]
pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "render".to_owned(),
            description: "Draws a view of the document to a PNG image (headless) and returns \
                it as image content with a one-line summary. Refused when the view shows \
                data you are not entitled to."
                .to_owned(),
            input_schema: input_schema::<RenderArgs>(),
            annotations: Annotations::READ,
            output_schema: None,
        },
        ToolDef {
            name: "notifications".to_owned(),
            description: "Takes your pending qp/0 notifications (subscription `notify` \
                messages, oldest first; each is returned once). Same as reading the \
                qapsule://notifications resource."
                .to_owned(),
            input_schema: input_schema::<NotificationsArgs>(),
            annotations: Annotations::WRITE,
            output_schema: None,
        },
    ]
}

/// Calls a common tool; `None` when `name` is not one.
pub fn call(ctx: &mut Ctx<'_>, name: &str, args: &Value) -> Option<Result<Value, RpcError>> {
    match name {
        "render" => Some(render(ctx, args)),
        "notifications" => Some(
            serde_json::from_value::<NotificationsArgs>(args.clone())
                .map_err(|e| RpcError::new(-32602, format!("tool `notifications`: {e}")))
                .map(|_| {
                    let v = notifications_json(ctx);
                    json!({
                        "resultType": "complete",
                        "content": [{"type": "text", "text": v.to_string()}],
                        "structuredContent": v,
                        "isError": false,
                    })
                }),
        ),
        _ => None,
    }
}

/// The pending notifications as one JSON object (taken: each is returned once).
pub fn notifications_json(ctx: &mut Ctx<'_>) -> Value {
    let items: Vec<Value> = ctx
        .take_notifications()
        .into_iter()
        .map(|q| json!({"order": q.order, "msg": q.msg}))
        .collect();
    json!({"notifications": items, "dropped": ctx.dropped_notifications()})
}

fn tool_error(text: String, reply: Option<Value>) -> Value {
    let mut r = json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "isError": true,
    });
    if let Some(e) = reply {
        r["structuredContent"] = e;
    }
    r
}

/// Series ids a view's layers draw (from the `get` reply of the view).
fn layer_series(item: &Value) -> Vec<String> {
    let mut out: Vec<String> = item
        .get("layers")
        .and_then(Value::as_array)
        .map(|ls| {
            ls.iter()
                .filter_map(|l| l.get("series").and_then(Value::as_str).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out.dedup();
    out
}

#[allow(clippy::too_many_lines)]
fn render(ctx: &mut Ctx<'_>, args: &Value) -> Result<Value, RpcError> {
    let a: RenderArgs = serde_json::from_value(args.clone())
        .map_err(|e| RpcError::new(-32602, format!("tool `render`: {e}")))?;
    let side = |v: Option<u32>, d: u32, name: &str| -> Result<u32, RpcError> {
        let v = v.unwrap_or(d);
        if (MIN_SIDE..=MAX_SIDE).contains(&v) {
            Ok(v)
        } else {
            Err(RpcError::new(
                -32602,
                format!("tool `render`: `{name}` {v} is not in {MIN_SIDE}..={MAX_SIDE}"),
            ))
        }
    };
    let width = side(a.width, qp_render::DEFAULT_WIDTH, "width")?;
    let height = side(a.height, qp_render::DEFAULT_HEIGHT, "height")?;
    // The view as this principal may read it (the authority's `get`, permission-checked).
    let got = ctx.query(json!({"kind": "get", "id": a.view}));
    if got.get("type").and_then(Value::as_str) != Some("result") {
        return Ok(tool_error(format!("render: {got}"), Some(got)));
    }
    let item = got.get("data").cloned().unwrap_or(Value::Null);
    if item.get("collection").and_then(Value::as_str) != Some("views") {
        return Ok(tool_error(
            format!("render: `{}` is not a view", a.view),
            None,
        ));
    }
    let view = item.get("item").cloned().unwrap_or(Value::Null);
    let view_id = view
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or(&a.view)
        .to_owned();
    // Entitlement (K33): every series the view draws must be readable by this principal
    // (asked through the protocol, so the authority's rule decides). The scene is then
    // compiled in the reader's projection (`on_behalf`, public world; §25.7).
    for s in layer_series(&view) {
        let r = ctx.query(json!({"kind": "study", "series": s, "reduce": "last"}));
        let code = r
            .pointer("/error/code")
            .and_then(Value::as_str)
            .unwrap_or("");
        if r.get("type").and_then(Value::as_str) == Some("error")
            && matches!(code, "no_entitlement" | "forbidden")
        {
            return Ok(tool_error(
                format!("render: series `{s}` of view `{view_id}` is not readable by you"),
                Some(r),
            ));
        }
    }
    let mut opts = qp_session::CompileOptions::new(width, height);
    opts.look.clone_from(&a.look);
    let scene = {
        let conn = ctx.conn.to_owned();
        let mut hub = ctx.hub().borrow_mut();
        hub.session_mut()
            .scene_as_reader(&conn, &view_id, a.proposal.as_deref(), &opts)
    };
    let scene = match scene {
        Ok(s) => s,
        Err(e) => {
            let v = serde_json::to_value(&e).unwrap_or(Value::Null);
            return Ok(tool_error(format!("render: {v}"), Some(v)));
        }
    };
    let png = match qp_render::render_scene_png(&scene) {
        Ok(p) => p,
        Err(e) => return Ok(tool_error(format!("render: {e}"), None)),
    };
    let layers = view
        .get("layers")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let text = format!(
        "view `{view_id}` drawn at {width}x{height} (look {}, {layers} layer(s), {} PNG bytes)",
        a.look.as_deref().unwrap_or("r4"),
        png.len()
    );
    Ok(json!({
        "resultType": "complete",
        "content": [
            {"type": "image", "data": base64(&png), "mimeType": "image/png"},
            {"type": "text", "text": text},
        ],
        "isError": false,
    }))
}

/// Standard base64 (RFC 4648, padded).
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= c.len() {
                out.push(char::from(A[((n >> (18 - 6 * i)) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::base64;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        for (i, o) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(i.as_bytes()), o);
        }
    }
}
