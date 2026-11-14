//! Size probe: one exported function that encodes and decodes a `ServerMsg` with every
//! enabled codec. JSON (ser+de of `ServerMsg`) is the baseline unless `json-none`.
#![allow(clippy::all)]

mod schema {
    pub use qp_schema::protocol::*;
    pub use qp_schema::*;
}
use schema::{Bar, ChangeOp, Event, ServerMsg};

pub type R<T> = Result<T, String>;
pub fn e<E: core::fmt::Display>(x: E) -> String {
    x.to_string()
}

#[cfg(feature = "qcol")]
#[path = "../../src/qcol.rs"]
#[allow(dead_code)]
mod qcol;

fn msg(n: u32) -> ServerMsg {
    let f = n as f64;
    ServerMsg::Notify {
        sub: "s".into(),
        event: Event::Change {
            seq: n as u64,
            at: n as i64,
            author: "feed".into(),
            approved_by: None,
            cause: None,
            changes: vec![ChangeOp::AppendBars {
                series: "px".into(),
                bars: vec![Bar {
                    t: n as i64,
                    o: f,
                    h: f,
                    l: f,
                    c: f,
                    v: f,
                }],
            }],
        },
    }
}

#[cfg(feature = "postcard-mirror")]
mod mirror {
    use super::*;
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize)]
    pub enum M {
        Notify {
            sub: String,
            seq: u64,
            at: i64,
            author: String,
            series: String,
            bars: Vec<Bar>,
        },
    }
    pub fn to(m: &ServerMsg) -> Option<M> {
        match m {
            ServerMsg::Notify {
                sub,
                event:
                    Event::Change {
                        seq,
                        at,
                        author,
                        changes,
                        ..
                    },
            } => match &changes[0] {
                ChangeOp::AppendBars { series, bars } => Some(M::Notify {
                    sub: sub.clone(),
                    seq: *seq,
                    at: *at,
                    author: author.clone(),
                    series: series.clone(),
                    bars: bars.clone(),
                }),
                _ => None,
            },
            _ => None,
        }
    }
}

#[no_mangle]
pub extern "C" fn run(n: u32) -> u32 {
    let m = msg(n);
    let mut total = 0u32;
    #[cfg(not(feature = "json-none"))]
    {
        let b = serde_json::to_vec(&m).unwrap();
        let back: ServerMsg = serde_json::from_slice(&b).unwrap();
        total += b.len() as u32 + (back == m) as u32;
    }
    #[cfg(feature = "cbor-ciborium")]
    {
        let mut b = Vec::new();
        ciborium::into_writer(&m, &mut b).unwrap();
        let back: ServerMsg = ciborium::from_reader(&b[..]).unwrap();
        total += b.len() as u32 + (back == m) as u32;
    }
    #[cfg(feature = "cbor-minicbor")]
    {
        let b = minicbor_serde::to_vec(&m).unwrap();
        let back: ServerMsg = minicbor_serde::from_slice(&b).unwrap();
        total += b.len() as u32 + (back == m) as u32;
    }
    #[cfg(feature = "msgpack")]
    {
        let b = rmp_serde::to_vec_named(&m).unwrap();
        let back: ServerMsg = rmp_serde::from_slice(&b).unwrap();
        total += b.len() as u32 + (back == m) as u32;
    }
    #[cfg(feature = "postcard-mirror")]
    {
        let b = postcard::to_allocvec(&mirror::to(&m).unwrap()).unwrap();
        let back: mirror::M = postcard::from_bytes(&b).unwrap();
        total += b.len() as u32 + matches!(back, mirror::M::Notify { .. }) as u32;
    }
    #[cfg(feature = "qcol")]
    {
        let b = qcol::enc(&m).unwrap();
        let back = qcol::dec(&b).unwrap();
        total += b.len() as u32 + (back == m) as u32;
    }
    total
}
