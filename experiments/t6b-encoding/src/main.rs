//! RD1/T6b: protocol encodings on the real qp-schema types.
//!
//! Output: markdown-ish tables on stdout (also written raw to `raw/t6b/` by the caller).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

mod schema {
    pub use qp_schema::protocol::*;
    pub use qp_schema::*;
}
#[path = "common.rs"]
mod common;

use common::*;
use schema::{Bar, ChangeOp, Document, Event, QueryResult, SeriesData, SeriesValues, ServerMsg};

type R<T> = Result<T, String>;

fn e<E: std::fmt::Display>(x: E) -> String {
    let s = x.to_string();
    s.chars().take(110).collect()
}

// ---------------------------------------------------------------------------
// Codecs on the SAME qp-schema types
// ---------------------------------------------------------------------------

struct Codec {
    name: &'static str,
    enc: fn(&ServerMsg) -> R<Vec<u8>>,
    dec: fn(&[u8]) -> R<ServerMsg>,
    enc_doc: fn(&Document) -> R<Vec<u8>>,
    dec_doc: fn(&[u8]) -> R<Document>,
}

macro_rules! serde_codec {
    ($name:expr, |$v:ident| $enc:expr, |$b:ident| $dec:expr) => {
        Codec {
            name: $name,
            enc: |$v: &ServerMsg| $enc,
            dec: |$b: &[u8]| $dec,
            enc_doc: |$v: &Document| $enc,
            dec_doc: |$b: &[u8]| $dec,
        }
    };
}

fn cbor_ciborium_enc<T: Serialize>(v: &T) -> R<Vec<u8>> {
    let mut out = Vec::new();
    ciborium::into_writer(v, &mut out).map_err(e)?;
    Ok(out)
}

fn same_type_codecs() -> Vec<Codec> {
    vec![
        serde_codec!(
            "json (float_roundtrip)",
            |v| serde_json::to_vec(v).map_err(e),
            |b| serde_json::from_slice(b).map_err(e)
        ),
        serde_codec!("cbor ciborium", |v| cbor_ciborium_enc(v), |b| {
            ciborium::from_reader(b).map_err(e)
        }),
        serde_codec!(
            "cbor minicbor-serde",
            |v| minicbor_serde::to_vec(v).map_err(e),
            |b| minicbor_serde::from_slice(b).map_err(e)
        ),
        serde_codec!(
            "msgpack rmp (struct-map)",
            |v| rmp_serde::to_vec_named(v).map_err(e),
            |b| rmp_serde::from_slice(b).map_err(e)
        ),
        serde_codec!(
            "msgpack rmp (struct-array)",
            |v| rmp_serde::to_vec(v).map_err(e),
            |b| rmp_serde::from_slice(b).map_err(e)
        ),
        serde_codec!(
            "postcard (same types)",
            |v| postcard::to_allocvec(v).map_err(e),
            |b| postcard::from_bytes(b).map_err(e)
        ),
    ]
}

// ---------------------------------------------------------------------------
// Mirror types for postcard: externally tagged enums, no skipped fields.
// Only the hot-path shapes; conversion cost is INCLUDED in timing.
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
enum MServerMsg {
    Result {
        id: String,
        seq: u64,
        data: MQueryResult,
    },
    Notify {
        sub: String,
        event: MEvent,
    },
}
#[derive(Serialize, Deserialize)]
enum MQueryResult {
    Values {
        series: String,
        from_bar: u64,
        t: Vec<i64>,
        outputs: BTreeMap<String, Vec<Option<f64>>>,
    },
}
#[derive(Serialize, Deserialize)]
enum MEvent {
    Change {
        seq: u64,
        at: i64,
        author: String,
        approved_by: Option<String>,
        cause: Option<String>,
        changes: Vec<MChangeOp>,
    },
}
#[derive(Serialize, Deserialize)]
enum MChangeOp {
    AppendBars { series: String, bars: Vec<Bar> },
    SetLastBar { series: String, bar: Bar },
}

fn to_mirror(m: &ServerMsg) -> R<MServerMsg> {
    Ok(match m {
        ServerMsg::Result {
            id,
            seq,
            data: QueryResult::Values { values },
        } => MServerMsg::Result {
            id: id.clone(),
            seq: *seq,
            data: MQueryResult::Values {
                series: values.series.clone(),
                from_bar: values.from_bar,
                t: values.t.clone(),
                outputs: values.outputs.clone(),
            },
        },
        ServerMsg::Notify {
            sub,
            event:
                Event::Change {
                    seq,
                    at,
                    author,
                    approved_by,
                    cause,
                    changes,
                },
        } => {
            let changes = changes
                .iter()
                .map(|c| match c {
                    ChangeOp::AppendBars { series, bars } => Ok(MChangeOp::AppendBars {
                        series: series.clone(),
                        bars: bars.clone(),
                    }),
                    ChangeOp::SetLastBar { series, bar } => Ok(MChangeOp::SetLastBar {
                        series: series.clone(),
                        bar: bar.clone(),
                    }),
                    _ => Err("mirror: unsupported change op".to_string()),
                })
                .collect::<R<Vec<_>>>()?;
            MServerMsg::Notify {
                sub: sub.clone(),
                event: MEvent::Change {
                    seq: *seq,
                    at: *at,
                    author: author.clone(),
                    approved_by: approved_by.clone(),
                    cause: cause.clone(),
                    changes,
                },
            }
        }
        _ => return Err("mirror: unsupported message".into()),
    })
}

fn from_mirror(m: MServerMsg) -> ServerMsg {
    match m {
        MServerMsg::Result {
            id,
            seq,
            data:
                MQueryResult::Values {
                    series,
                    from_bar,
                    t,
                    outputs,
                },
        } => ServerMsg::Result {
            id,
            seq,
            data: QueryResult::Values {
                values: SeriesValues {
                    series,
                    from_bar,
                    t,
                    outputs,
                },
            },
        },
        MServerMsg::Notify {
            sub,
            event:
                MEvent::Change {
                    seq,
                    at,
                    author,
                    approved_by,
                    cause,
                    changes,
                },
        } => ServerMsg::Notify {
            sub,
            event: Event::Change {
                seq,
                at,
                author,
                approved_by,
                cause,
                changes: changes
                    .into_iter()
                    .map(|c| match c {
                        MChangeOp::AppendBars { series, bars } => {
                            ChangeOp::AppendBars { series, bars }
                        }
                        MChangeOp::SetLastBar { series, bar } => {
                            ChangeOp::SetLastBar { series, bar }
                        }
                    })
                    .collect(),
            },
        },
    }
}

fn no_doc_enc(_: &Document) -> R<Vec<u8>> {
    Err("n/a (no Document mirror)".into())
}
fn no_doc_dec(_: &[u8]) -> R<Document> {
    Err("n/a".into())
}

// ---------------------------------------------------------------------------
// JSON with bars as columns (struct-of-arrays inside JSON)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct BarsSoa {
    t: Vec<i64>,
    o: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    c: Vec<f64>,
    v: Vec<f64>,
}
impl BarsSoa {
    fn from_bars(b: &[Bar]) -> Self {
        BarsSoa {
            t: b.iter().map(|x| x.t).collect(),
            o: b.iter().map(|x| x.o).collect(),
            h: b.iter().map(|x| x.h).collect(),
            l: b.iter().map(|x| x.l).collect(),
            c: b.iter().map(|x| x.c).collect(),
            v: b.iter().map(|x| x.v).collect(),
        }
    }
    fn into_bars(self) -> R<Vec<Bar>> {
        let n = self.t.len();
        if [
            self.o.len(),
            self.h.len(),
            self.l.len(),
            self.c.len(),
            self.v.len(),
        ]
        .iter()
        .any(|&k| k != n)
        {
            return Err("soa length mismatch".into());
        }
        Ok((0..n)
            .map(|i| Bar {
                t: self.t[i],
                o: self.o[i],
                h: self.h[i],
                l: self.l[i],
                c: self.c[i],
                v: self.v[i],
            })
            .collect())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum SoaMsg {
    Notify { sub: String, event: SoaEvent },
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum SoaEvent {
    Change {
        seq: u64,
        at: i64,
        author: String,
        changes: Vec<SoaChange>,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum SoaChange {
    AppendBars { series: String, bars: BarsSoa },
    SetLastBar { series: String, bar: Bar },
}

fn json_soa_enc(m: &ServerMsg) -> R<Vec<u8>> {
    match m {
        ServerMsg::Notify {
            sub,
            event:
                Event::Change {
                    seq,
                    at,
                    author,
                    approved_by: None,
                    cause: None,
                    changes,
                },
        } => {
            let changes = changes
                .iter()
                .map(|c| match c {
                    ChangeOp::AppendBars { series, bars } => Ok(SoaChange::AppendBars {
                        series: series.clone(),
                        bars: BarsSoa::from_bars(bars),
                    }),
                    ChangeOp::SetLastBar { series, bar } => Ok(SoaChange::SetLastBar {
                        series: series.clone(),
                        bar: bar.clone(),
                    }),
                    _ => Err("unsupported".to_string()),
                })
                .collect::<R<Vec<_>>>()?;
            let s = SoaMsg::Notify {
                sub: sub.clone(),
                event: SoaEvent::Change {
                    seq: *seq,
                    at: *at,
                    author: author.clone(),
                    changes,
                },
            };
            serde_json::to_vec(&s).map_err(e)
        }
        // Values are already columnar in JSON: same bytes as plain JSON.
        ServerMsg::Result { .. } => serde_json::to_vec(m).map_err(e),
        _ => Err("unsupported".into()),
    }
}
fn json_soa_dec(b: &[u8]) -> R<ServerMsg> {
    if let Ok(SoaMsg::Notify {
        sub,
        event:
            SoaEvent::Change {
                seq,
                at,
                author,
                changes,
            },
    }) = serde_json::from_slice::<SoaMsg>(b)
    {
        let changes = changes
            .into_iter()
            .map(|c| match c {
                SoaChange::AppendBars { series, bars } => Ok(ChangeOp::AppendBars {
                    series,
                    bars: bars.into_bars()?,
                }),
                SoaChange::SetLastBar { series, bar } => Ok(ChangeOp::SetLastBar { series, bar }),
            })
            .collect::<R<Vec<_>>>()?;
        return Ok(ServerMsg::Notify {
            sub,
            event: Event::Change {
                seq,
                at,
                author,
                approved_by: None,
                cause: None,
                changes,
            },
        });
    }
    serde_json::from_slice(b).map_err(e)
}

/// Document with bars moved out into a column object: `{"doc": <doc without bars>, "bars": {id: soa}}`.
#[derive(Serialize)]
struct SoaDocRef<'a> {
    doc: &'a Document,
    bars: BTreeMap<String, BarsSoa>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SoaDoc {
    doc: Document,
    bars: BTreeMap<String, BarsSoa>,
}

/// The document with every bar series emptied (bars stay a `bars` kind with `[]`).
fn strip_bars(d: &Document) -> (Document, Vec<(String, &[Bar])>) {
    let mut cols = Vec::new();
    let mut doc = Document {
        series: Vec::new(),
        ..d.clone_shallow()
    };
    for s in &d.series {
        match &s.data {
            Some(SeriesData::Bars { bars }) => {
                cols.push((s.id.clone(), bars.as_slice()));
                let mut s2 = s.clone_without_data();
                s2.data = Some(SeriesData::Bars { bars: Vec::new() });
                doc.series.push(s2);
            }
            _ => doc.series.push(s.clone()),
        }
    }
    (doc, cols)
}

trait CloneShallow {
    fn clone_shallow(&self) -> Self;
}
impl CloneShallow for Document {
    fn clone_shallow(&self) -> Self {
        Document {
            schema_version: self.schema_version.clone(),
            id: self.id.clone(),
            title: self.title.clone(),
            display_tz: self.display_tz.clone(),
            series: Vec::new(),
            views: self.views.clone(),
            facts: self.facts.clone(),
            findings: self.findings.clone(),
            capsules: self.capsules.clone(),
            intents: self.intents.clone(),
            ledger: self.ledger.clone(),
            narratives: self.narratives.clone(),
            proposals: self.proposals.clone(),
        }
    }
}
trait CloneNoData {
    fn clone_without_data(&self) -> Self;
}
impl CloneNoData for schema::Series {
    fn clone_without_data(&self) -> Self {
        schema::Series {
            id: self.id.clone(),
            instrument: self.instrument.clone(),
            interval: self.interval.clone(),
            source: self.source.clone(),
            data: None,
        }
    }
}

fn put_bars(doc: &mut Document, id: &str, bars: Vec<Bar>) -> R<()> {
    let s = doc
        .series
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or("unknown series")?;
    s.data = Some(SeriesData::Bars { bars });
    Ok(())
}

fn json_soa_enc_doc(d: &Document) -> R<Vec<u8>> {
    let (doc, cols) = strip_bars(d);
    let bars = cols
        .into_iter()
        .map(|(id, b)| (id, BarsSoa::from_bars(b)))
        .collect();
    serde_json::to_vec(&SoaDocRef { doc: &doc, bars }).map_err(e)
}
fn json_soa_dec_doc(b: &[u8]) -> R<Document> {
    let SoaDoc { mut doc, bars } = serde_json::from_slice(b).map_err(e)?;
    for (id, soa) in bars {
        put_bars(&mut doc, &id, soa.into_bars()?)?;
    }
    Ok(doc)
}

// ---------------------------------------------------------------------------
// Columnar binary frame ("qcol"): f64/i64 little-endian blocks, 8-byte aligned so a
// JS shell can view them as Float64Array / BigInt64Array without copying.
// Null in `values` = validity bitmap (Arrow-style), not NaN.
// ---------------------------------------------------------------------------

#[path = "qcol.rs"]
mod qcol;

mod qcol_doc {
    use super::*;
    use qcol::{read_bars, write_bars, Rd, W};

    /// Snapshot: [u32 json_len][json of doc without bars][pad8][u32 n_series]{str id, bars}*
    pub fn enc_doc(d: &Document) -> R<Vec<u8>> {
        let (doc, cols) = strip_bars(d);
        let js = serde_json::to_vec(&doc).map_err(e)?;
        let mut w = W(Vec::with_capacity(
            js.len() + 64 + cols.iter().map(|c| c.1.len() * 48).sum::<usize>(),
        ));
        w.u32(js.len() as u32);
        w.0.extend_from_slice(&js);
        w.u32(cols.len() as u32);
        for (id, bars) in cols {
            w.str(&id);
            write_bars(&mut w.0, bars);
        }
        Ok(w.0)
    }
    pub fn dec_doc(b: &[u8]) -> R<Document> {
        let mut r = Rd(b, 0);
        let n = r.u32()? as usize;
        let mut doc: Document = serde_json::from_slice(r.take(n)?).map_err(e)?;
        for _ in 0..r.u32()? {
            let id = r.str()?;
            let bars = read_bars(&mut r)?;
            put_bars(&mut doc, &id, bars)?;
        }
        r.end()?;
        Ok(doc)
    }
}

fn all_codecs() -> Vec<Codec> {
    let mut v = same_type_codecs();
    v.push(Codec {
        name: "postcard (mirror types)",
        enc: |m| postcard::to_allocvec(&to_mirror(m)?).map_err(e),
        dec: |b| Ok(from_mirror(postcard::from_bytes(b).map_err(e)?)),
        enc_doc: no_doc_enc,
        dec_doc: no_doc_dec,
    });
    v.push(Codec {
        name: "json SoA bars",
        enc: json_soa_enc,
        dec: json_soa_dec,
        enc_doc: json_soa_enc_doc,
        dec_doc: json_soa_dec_doc,
    });
    v.push(Codec {
        name: "qcol (LE f64 blocks)",
        enc: qcol::enc,
        dec: qcol::dec,
        enc_doc: qcol_doc::enc_doc,
        dec_doc: qcol_doc::dec_doc,
    });
    v
}

// ---------------------------------------------------------------------------
// Compression (native only)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod compress {
    use std::io::Write;

    pub fn gzip(b: &[u8]) -> usize {
        let mut z = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(b).unwrap();
        z.finish().unwrap().len()
    }
    pub fn brotli(b: &[u8], q: u32) -> usize {
        let mut out = Vec::new();
        {
            let mut w = brotli::CompressorWriter::new(&mut out, 4096, q, 22);
            w.write_all(b).unwrap();
        }
        out.len()
    }
    /// WebSocket permessage-deflate with context takeover: one raw-deflate stream, a sync
    /// flush after every message (RFC 7692 strips the trailing 4 bytes 00 00 ff ff).
    /// Returns average compressed bytes per message.
    pub fn ws_deflate_stream(msgs: &[Vec<u8>]) -> f64 {
        let mut c = flate2::Compress::new(flate2::Compression::default(), false);
        let mut total = 0usize;
        for m in msgs {
            let mut out = Vec::with_capacity(m.len() * 2 + 64);
            let before = c.total_out();
            c.compress_vec(m, &mut out, flate2::FlushCompress::Sync)
                .unwrap();
            total += (c.total_out() - before) as usize - 4;
        }
        total as f64 / msgs.len() as f64
    }
}

// ---------------------------------------------------------------------------
// Runs
// ---------------------------------------------------------------------------

fn fmt_us(ns: f64) -> String {
    if ns < 10_000.0 {
        format!("{:.2}", ns / 1000.0)
    } else {
        format!("{:.0}", ns / 1000.0)
    }
}

struct Row {
    case: &'static str,
    codec: &'static str,
    bytes: usize,
    enc_ns: f64,
    dec_ns: f64,
    gz: Option<usize>,
    br: Option<usize>,
    note: String,
}

fn bench_msg(case: &'static str, c: &Codec, m: &ServerMsg, rows: &mut Vec<Row>) {
    let mut note = String::new();
    let bytes = match (c.enc)(m) {
        Ok(b) => b,
        Err(err) => {
            rows.push(Row {
                case,
                codec: c.name,
                bytes: 0,
                enc_ns: f64::NAN,
                dec_ns: f64::NAN,
                gz: None,
                br: None,
                note: format!("encode error: {err}"),
            });
            return;
        }
    };
    match (c.dec)(&bytes) {
        Ok(back) => {
            let mm = bit_mismatches(&msg_bits(m), &msg_bits(&back));
            if &back != m || mm != 0 {
                note = format!("ROUND-TRIP MISMATCH ({mm} bit diffs)");
            }
        }
        Err(err) => {
            let enc_ns = median_ns(|| {
                std::hint::black_box((c.enc)(std::hint::black_box(m)).unwrap());
            });
            rows.push(Row {
                case,
                codec: c.name,
                bytes: bytes.len(),
                enc_ns,
                dec_ns: f64::NAN,
                gz: None,
                br: None,
                note: format!("decode error: {err}"),
            });
            return;
        }
    }
    let enc_ns = median_ns(|| {
        std::hint::black_box((c.enc)(std::hint::black_box(m)).unwrap());
    });
    let dec_ns = median_ns(|| {
        std::hint::black_box((c.dec)(std::hint::black_box(&bytes)).unwrap());
    });
    #[cfg(not(target_arch = "wasm32"))]
    let (gz, br) = (
        Some(compress::gzip(&bytes)),
        Some(compress::brotli(&bytes, 6)),
    );
    #[cfg(target_arch = "wasm32")]
    let (gz, br) = (None, None);
    rows.push(Row {
        case,
        codec: c.name,
        bytes: bytes.len(),
        enc_ns,
        dec_ns,
        gz,
        br,
        note,
    });
}

fn bench_doc(case: &'static str, c: &Codec, d: &Document, rows: &mut Vec<Row>) {
    let bytes = match (c.enc_doc)(d) {
        Ok(b) => b,
        Err(err) => {
            rows.push(Row {
                case,
                codec: c.name,
                bytes: 0,
                enc_ns: f64::NAN,
                dec_ns: f64::NAN,
                gz: None,
                br: None,
                note: err,
            });
            return;
        }
    };
    let note = match (c.dec_doc)(&bytes) {
        Ok(back) => {
            let mm = bit_mismatches(&doc_bits(d), &doc_bits(&back));
            if &back != d || mm != 0 {
                format!("ROUND-TRIP MISMATCH ({mm} bit diffs)")
            } else {
                String::new()
            }
        }
        Err(err) => {
            rows.push(Row {
                case,
                codec: c.name,
                bytes: bytes.len(),
                enc_ns: f64::NAN,
                dec_ns: f64::NAN,
                gz: None,
                br: None,
                note: format!("decode error: {err}"),
            });
            return;
        }
    };
    let enc_ns = median_ns(|| {
        std::hint::black_box((c.enc_doc)(std::hint::black_box(d)).unwrap());
    });
    let dec_ns = median_ns(|| {
        std::hint::black_box((c.dec_doc)(std::hint::black_box(&bytes)).unwrap());
    });
    #[cfg(not(target_arch = "wasm32"))]
    let (gz, br) = (
        Some(compress::gzip(&bytes)),
        Some(compress::brotli(&bytes, 6)),
    );
    #[cfg(target_arch = "wasm32")]
    let (gz, br) = (None, None);
    rows.push(Row {
        case,
        codec: c.name,
        bytes: bytes.len(),
        enc_ns,
        dec_ns,
        gz,
        br,
        note,
    });
}

fn print_rows(rows: &[Row]) {
    println!("| case | codec | bytes | enc µs | dec µs | gzip-6 B | brotli-6 B | note |");
    println!("|---|---|---:|---:|---:|---:|---:|---|");
    for r in rows {
        let o = |x: Option<usize>| x.map(|v| v.to_string()).unwrap_or_else(|| "-".into());
        let t = |x: f64| {
            if x.is_nan() {
                "-".to_string()
            } else {
                fmt_us(x)
            }
        };
        let b = if r.bytes == 0 {
            "-".to_string()
        } else {
            r.bytes.to_string()
        };
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            r.case,
            r.codec,
            b,
            t(r.enc_ns),
            t(r.dec_ns),
            o(r.gz),
            o(r.br),
            r.note
        );
    }
}

fn exactness(codecs: &[Codec]) {
    let vals = exactness_values(100_000);
    let (a, v) = exactness_msgs(&vals);
    println!(
        "\n## f64 exactness ({} values: {} tricky + random finite bit patterns + price-like)\n",
        vals.len(),
        tricky_values().len()
    );
    println!("| codec | append_bars bit diffs | values bit diffs | tricky-only diffs |");
    println!("|---|---:|---:|---:|");
    let tricky = tricky_values();
    let (ta, tv) = exactness_msgs(&tricky);
    for c in codecs {
        let run = |m: &ServerMsg| -> String {
            match (c.enc)(m).and_then(|b| (c.dec)(&b)) {
                Ok(back) => bit_mismatches(&msg_bits(m), &msg_bits(&back)).to_string(),
                Err(err) => format!("err: {}", err.chars().take(50).collect::<String>()),
            }
        };
        let tr = {
            let x = run(&ta);
            let y = run(&tv);
            format!("{x}/{y}")
        };
        println!("| {} | {} | {} | {} |", c.name, run(&a), run(&v), tr);
    }
}

/// Schema evolution probes: what happens when a sender adds a field the receiver does not know.
fn evolution() {
    #[derive(Serialize)]
    struct BarX {
        t: i64,
        o: f64,
        h: f64,
        l: f64,
        c: f64,
        v: f64,
        x: f64,
    }
    #[derive(Serialize)]
    struct BarMissing {
        t: i64,
        o: f64,
        h: f64,
        l: f64,
        c: f64,
    }
    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct BarLoose {
        t: i64,
        o: f64,
        h: f64,
        l: f64,
        c: f64,
        v: f64,
    }
    let bx = BarX {
        t: 1,
        o: 2.0,
        h: 3.0,
        l: 1.0,
        c: 2.5,
        v: 10.0,
        x: 99.0,
    };
    let bm = BarMissing {
        t: 1,
        o: 2.0,
        h: 3.0,
        l: 1.0,
        c: 2.5,
    };
    println!("\n## Schema evolution probe (Bar has deny_unknown_fields)\n");
    println!("| codec | extra field -> Bar (deny) | extra field -> Bar w/o deny | missing field -> Bar |");
    println!("|---|---|---|---|");
    fn show<T: std::fmt::Debug>(r: R<T>) -> String {
        match r {
            Ok(v) => format!("OK {:?}", v).chars().take(60).collect(),
            Err(x) => format!("error: {}", x.chars().take(60).collect::<String>()),
        }
    }
    macro_rules! probe {
        ($name:expr, $enc:expr, $dec:path) => {{
            let a: R<Bar> = $enc(&bx).and_then(|b: Vec<u8>| $dec(&b).map_err(e));
            let b: R<BarLoose> = $enc(&bx).and_then(|b: Vec<u8>| $dec(&b).map_err(e));
            let c: R<Bar> = $enc(&bm).and_then(|b: Vec<u8>| $dec(&b).map_err(e));
            println!("| {} | {} | {} | {} |", $name, show(a), show(b), show(c));
        }};
    }
    fn jenc<T: Serialize>(v: &T) -> R<Vec<u8>> {
        serde_json::to_vec(v).map_err(e)
    }
    fn cenc<T: Serialize>(v: &T) -> R<Vec<u8>> {
        cbor_ciborium_enc(v)
    }
    fn menc<T: Serialize>(v: &T) -> R<Vec<u8>> {
        minicbor_serde::to_vec(v).map_err(e)
    }
    fn renc<T: Serialize>(v: &T) -> R<Vec<u8>> {
        rmp_serde::to_vec_named(v).map_err(e)
    }
    fn penc<T: Serialize>(v: &T) -> R<Vec<u8>> {
        postcard::to_allocvec(v).map_err(e)
    }
    fn cdec<T: for<'a> Deserialize<'a>>(
        b: &[u8],
    ) -> Result<T, ciborium::de::Error<std::io::Error>> {
        ciborium::from_reader(b)
    }
    probe!("json", jenc, serde_json::from_slice);
    probe!("cbor ciborium", cenc, cdec);
    probe!("cbor minicbor-serde", menc, minicbor_serde::from_slice);
    probe!("msgpack rmp (struct-map)", renc, rmp_serde::from_slice);
    probe!("postcard", penc, postcard::from_bytes);
}

fn main() {
    let c = corpus();
    let codecs = all_codecs();
    println!("# T6b encoding bench — platform: {}", platform());
    println!(
        "corpus: {} bars; RSI nulls {}; ticks per second {}\n",
        N_BARS,
        match &c.values_rsi {
            ServerMsg::Result {
                data: QueryResult::Values { values },
                ..
            } => values.outputs["rsi"].iter().filter(|x| x.is_none()).count(),
            _ => 0,
        },
        TICKS_PER_SEC
    );

    // Warm-up pass (CPU frequency, caches, allocator); results discarded.
    let mut warm = Vec::new();
    for cd in &codecs {
        bench_msg("warmup", cd, &c.ticks_set_last[0], &mut warm);
        bench_msg("warmup", cd, &c.values_rsi, &mut warm);
    }
    let mut rows = Vec::new();
    for cd in &codecs {
        bench_msg("tick set_last_bar", cd, &c.ticks_set_last[30], &mut rows);
    }
    for cd in &codecs {
        bench_msg("tick append_bars(1)", cd, &c.ticks_append1[30], &mut rows);
    }
    for cd in &codecs {
        bench_msg("values RSI 10k", cd, &c.values_rsi, &mut rows);
    }
    for cd in &codecs {
        bench_msg("values MACD(3) 10k", cd, &c.values_macd, &mut rows);
    }
    for cd in &codecs {
        bench_msg("append_bars 10k", cd, &c.append_10k, &mut rows);
    }
    for cd in &codecs {
        bench_doc("snapshot Document 10k", cd, &c.document, &mut rows);
    }
    println!("## Sizes and times (median; per message)\n");
    print_rows(&rows);

    // Live rate: 60 msgs/s.
    println!("\n## Live tick stream: 60 messages/second\n");
    println!("| stream | codec | avg B/msg | B/s raw | ws-deflate B/msg (context takeover) | B/s ws-deflate | enc+dec µs/s (CPU) |");
    println!("|---|---|---:|---:|---:|---:|---:|");
    for (name, msgs) in [
        ("set_last_bar", &c.ticks_set_last),
        ("append_bars(1)", &c.ticks_append1),
    ] {
        for cd in &codecs {
            let enc: Vec<Vec<u8>> = match msgs.iter().map(|m| (cd.enc)(m)).collect::<R<Vec<_>>>() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if enc
                .iter()
                .zip(msgs.iter())
                .any(|(b, m)| (cd.dec)(b).map(|x| &x != m).unwrap_or(true))
            {
                println!("| {name} | {} | decode fails | | | | |", cd.name);
                continue;
            }
            let avg = enc.iter().map(|b| b.len()).sum::<usize>() as f64 / enc.len() as f64;
            let cpu = median_ns(|| {
                for m in msgs.iter() {
                    let b = (cd.enc)(m).unwrap();
                    std::hint::black_box((cd.dec)(&b).unwrap());
                }
            });
            #[cfg(not(target_arch = "wasm32"))]
            let ws = compress::ws_deflate_stream(&enc);
            #[cfg(target_arch = "wasm32")]
            let ws = f64::NAN;
            println!(
                "| {name} | {} | {:.1} | {:.0} | {:.1} | {:.0} | {:.1} |",
                cd.name,
                avg,
                avg * TICKS_PER_SEC as f64,
                ws,
                ws * TICKS_PER_SEC as f64,
                cpu / 1000.0
            );
        }
    }

    exactness(&codecs);
    evolution();

    // Samples for the report (native only).
    #[cfg(not(target_arch = "wasm32"))]
    {
        let dir = std::env::args().nth(1);
        if let Some(dir) = dir {
            let m = &c.ticks_set_last[30];
            std::fs::write(
                format!("{dir}/sample_tick_set_last.json"),
                serde_json::to_vec_pretty(m).unwrap(),
            )
            .unwrap();
            std::fs::write(
                format!("{dir}/sample_tick_set_last.compact.json"),
                serde_json::to_vec(m).unwrap(),
            )
            .unwrap();
            std::fs::write(
                format!("{dir}/sample_tick_set_last.cbor"),
                cbor_ciborium_enc(m).unwrap(),
            )
            .unwrap();
            std::fs::write(
                format!("{dir}/sample_tick_set_last.msgpack"),
                rmp_serde::to_vec_named(m).unwrap(),
            )
            .unwrap();
            std::fs::write(
                format!("{dir}/sample_tick_set_last.qcol"),
                qcol::enc(m).unwrap(),
            )
            .unwrap();
        }
        // Full payloads for the JS-side check (`js-parse.mjs`); large, not committed.
        if let Some(pdir) = std::env::args().nth(2) {
            for (name, m) in [
                ("tick", &c.ticks_set_last[30]),
                ("rsi", &c.values_rsi),
                ("macd", &c.values_macd),
                ("append10k", &c.append_10k),
            ] {
                std::fs::write(
                    format!("{pdir}/{name}.json"),
                    serde_json::to_vec(m).unwrap(),
                )
                .unwrap();
                std::fs::write(format!("{pdir}/{name}.qcol"), qcol::enc(m).unwrap()).unwrap();
            }
            std::fs::write(
                format!("{pdir}/doc.json"),
                serde_json::to_vec(&c.document).unwrap(),
            )
            .unwrap();
        }
    }
}
