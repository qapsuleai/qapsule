//! `qp` command line: a thin shell over `qp-schema`, `qp-source`, `qp-compute`,
//! `qp-render`, `qp-session` and `qp-mcp`.
//!
//! It only parses arguments, reads/writes files and calls the libraries.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "qp", version, about = "Qapsule command line", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// JSON Schema export and validation.
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    /// Run a study (sma, ema, rsi, macd, bollinger) over a bar series and print the
    /// result as `StudyOutput` JSON: one value per bar, warmup `null` (K13).
    Compute(ComputeArgs),
    /// Render to PNG with the reference renderer: a bars JSON file as the R0 chart
    /// (candles + SMA(20)), or a view of a session built from an init/transcript
    /// (`--view`), or the view of a presentation (`--present`, `--surface`).
    Render(RenderArgs),
    /// Run the protocol authority (`qp/0`) over stdin/stdout: one JSON line per message,
    /// `{"conn":…,"msg":…}` in and out (docs/PROTOCOL.md §3).
    Serve {
        /// Session init (`SessionInit` JSON).
        #[arg(long)]
        init: PathBuf,
        /// Directory that document paths in the init are relative to (and confined to).
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// After the outputs of each input line, write `{"batch_end":true}`.
        #[arg(long)]
        batch_end: bool,
        /// Accept host frames `{"host":{"data":<DataEvent>}}` (data events on the host's
        /// path, docs/PROTOCOL.md §3, §18.4). Only for a stream that belongs to the host;
        /// without this flag such a line is a framing error.
        #[arg(long)]
        host_frames: bool,
    },
    /// Run an MCP server over stdin/stdout (newline-delimited JSON-RPC, MCP stdio
    /// transport), speaking `qp/0` as one principal (rounds/RD1/EMBEDDING.md §1).
    Mcp {
        /// Session init (`SessionInit` JSON).
        #[arg(long)]
        init: PathBuf,
        /// Principal the adapter speaks as (sent in its `hello`).
        #[arg(long)]
        principal: String,
        /// Surface kind: desktop, phone, watch, voice or headless.
        #[arg(long, default_value = "headless")]
        surface: String,
        /// Directory that document paths in the init are relative to (and confined to).
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Print a plain-text semantic summary of a view, read from its compiled scene
    /// (panels, scales and visible domains, layers with data, origins and last values,
    /// emphasis). Headless: nothing is rendered.
    Explain(SceneArgs),
    /// Print the compiled scene description (`SceneDesc` JSON) of a view.
    Scene(SceneArgs),
    /// Authority ↔ replica check (docs/PROTOCOL.md §19.6): run the reference authority
    /// and a replica on one tick stream (replica data `--lag` events behind), check the
    /// display rule with an independent oracle, and write the replica's input tape with
    /// the digests after every step (replayed by the WASM replica in Node).
    ReplicaTape {
        /// Replica data delay in events.
        #[arg(long, default_value_t = 0)]
        lag: usize,
        /// Output tape (JSON).
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Run protocol conformance transcripts (docs/PROTOCOL.md §15).
    Conform {
        /// Transcript files or directories of `*.json` transcripts.
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Transport: `inproc` (default), `serve` (spawns `qp serve`) or `mcp` (one MCP
        /// adapter per connection over an in-process session).
        #[arg(long, default_value = "inproc")]
        transport: String,
        /// Repository root (paths in transcripts are relative to it).
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

/// Which view of which session: the session starts from `--init` (or the transcript's
/// init) and runs the transcript's messages; the view is a document view (`--view`) or
/// the view the presenter returns for a finding on a surface (`--present`, `--surface`).
#[derive(Debug, clap::Args)]
struct ViewSource {
    /// Session init (`SessionInit` JSON). Default: the transcript's init.
    #[arg(long)]
    init: Option<PathBuf>,
    /// A conformance transcript whose messages are sent first (its expectations are not
    /// checked), e.g. one that builds the view with grammar operations.
    #[arg(long)]
    transcript: Option<PathBuf>,
    /// View id.
    #[arg(long)]
    view: Option<String>,
    /// Instead of `--view`: present this finding (`present` query) on `--surface` and
    /// use the presentation's view (`capsule_view` at capsule depth, `view` at chart
    /// depth).
    #[arg(long, conflicts_with = "view")]
    present: Option<String>,
    /// Surface kind for `--present`: desktop, phone, watch, voice or headless.
    #[arg(long, default_value = "desktop", requires = "present")]
    surface: String,
    /// Directory that paths in the init and transcript are relative to (and confined to).
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

#[derive(Debug, clap::Args)]
struct SceneArgs {
    #[command(flatten)]
    source: ViewSource,
    /// Frame width in pixels.
    #[arg(long, default_value_t = qp_session::HIT_WIDTH)]
    width: u32,
    /// Frame height in pixels.
    #[arg(long, default_value_t = qp_session::HIT_HEIGHT)]
    height: u32,
    /// Pretty-print the JSON (`qp scene`).
    #[arg(long)]
    pretty: bool,
}

#[derive(Debug, clap::Args)]
struct RenderArgs {
    /// Input bars JSON file, rendered as the R0 chart (candles + SMA(20)).
    #[arg(required_unless_present_any = ["init", "transcript"], conflicts_with_all = ["init", "transcript", "view", "present"])]
    input: Option<PathBuf>,
    #[command(flatten)]
    source: ViewSource,
    /// Output PNG file.
    #[arg(short, long)]
    out: PathBuf,
    /// Image width in pixels.
    #[arg(long, default_value_t = qp_render::DEFAULT_WIDTH)]
    width: u32,
    /// Image height in pixels.
    #[arg(long, default_value_t = qp_render::DEFAULT_HEIGHT)]
    height: u32,
}

#[derive(Debug, clap::Args)]
#[allow(clippy::struct_excessive_bools)]
struct ComputeArgs {
    /// Input: a bar series file (`Series` with embedded bars) or a `Document` (then pick
    /// the series with `--series`; default: the first series that has bars).
    #[arg(required_unless_present = "list")]
    input: Option<PathBuf>,
    /// Study name.
    #[arg(long, required_unless_present = "list")]
    study: Option<String>,
    /// Parameter `name=value` (repeatable); missing ones take their defaults.
    #[arg(long = "param", value_name = "NAME=VALUE")]
    params: Vec<String>,
    /// Bar field the study reads.
    #[arg(long, default_value = "close")]
    field: String,
    /// Series id inside a document input.
    #[arg(long)]
    series: Option<String>,
    /// Parameter sweep: a JSON file with an array of parameter objects; prints a JSON
    /// array with one `StudyOutput` per set (all bit-identical to single runs).
    #[arg(long, conflicts_with = "params")]
    sweep: Option<PathBuf>,
    /// Write the JSON here instead of stdout.
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Pretty-print the JSON.
    #[arg(long)]
    pretty: bool,
    /// List the studies with their outputs and default parameters, then exit.
    #[arg(long)]
    list: bool,
}

#[derive(Debug, Subcommand)]
enum SchemaCommand {
    /// Write `<Name>.schema.json` for every root type into a directory.
    Export {
        /// Output directory (created if missing).
        #[arg(long)]
        out: PathBuf,
    },
    /// Validate a JSON instance against a root schema.
    Validate {
        /// JSON instance file.
        file: PathBuf,
        /// Root type name (e.g. Capsule, Document).
        #[arg(long = "type")]
        type_name: String,
        /// Read `<DIR>/<Name>.schema.json` instead of the built-in schema.
        #[arg(long)]
        schema_dir: Option<PathBuf>,
    },
}

/// Outcome of a command that ran to completion without an I/O or usage error.
enum Outcome {
    Ok,
    Invalid,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(Outcome::Ok) => ExitCode::SUCCESS,
        Ok(Outcome::Invalid) => ExitCode::from(1),
        Err(msg) => {
            eprintln!("qp: {msg}");
            ExitCode::from(2)
        }
    }
}

fn run(command: Command) -> Result<Outcome, String> {
    match command {
        Command::Schema {
            command: SchemaCommand::Export { out },
        } => schema_export(&out),
        Command::Schema {
            command:
                SchemaCommand::Validate {
                    file,
                    type_name,
                    schema_dir,
                },
        } => schema_validate(&file, &type_name, schema_dir.as_deref()),
        Command::Compute(args) => compute(&args),
        Command::Render(args) => match &args.input {
            Some(input) => render(input, &args.out, args.width, args.height),
            None => render_view(&args),
        },
        Command::Serve {
            init,
            root,
            batch_end,
            host_frames,
        } => serve(&init, &root, batch_end, host_frames),
        Command::Mcp {
            init,
            principal,
            surface,
            root,
        } => mcp(&init, &principal, &surface, &root),
        Command::Conform {
            inputs,
            transport,
            root,
        } => conform(&inputs, &transport, &root),
        Command::ReplicaTape { lag, out } => replica_tape(lag, &out),
        Command::Explain(args) => scene_cmd(&args, true),
        Command::Scene(args) => scene_cmd(&args, false),
    }
}

/// `qp replica-tape`: runs the check, writes the tape, prints a summary; invalid if a
/// display-rule violation was found or (without lag) a step's digests differ.
fn replica_tape(lag: usize, out: &std::path::Path) -> Result<Outcome, String> {
    let o = qp_session::tape::run(lag)?;
    let text = serde_json::to_string(&o.tape).map_err(|e| e.to_string())?;
    std::fs::write(out, text).map_err(|e| format!("{}: {e}", out.display()))?;
    let last = o.tape.steps.last().map(|s| &s.expect);
    println!(
        "lag {lag}: {} steps, {} with replica digests != authority, {} violations, final claims {} (shown final at end {}), display counts final/provisional/pending/disputed {:?}; last doc {} view {}",
        o.tape.steps.len(),
        o.differing_steps,
        o.violations.len(),
        o.final_claims,
        o.shown_final,
        o.shown_counts,
        last.map_or("", |e| e.doc.as_str()),
        last.map_or("", |e| e.view.as_str()),
    );
    for v in o.violations.iter().take(10) {
        eprintln!("violation at step {}: {}", v.step, v.what);
    }
    let caught_up = last.is_some_and(|e| e.doc == e.authority_doc && e.view == e.authority_view);
    let ok = o.violations.is_empty()
        && caught_up
        && o.shown_final == o.final_claims
        && (lag > 0 || o.differing_steps == 0);
    Ok(if ok { Outcome::Ok } else { Outcome::Invalid })
}

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
) -> Result<
    (qp_schema::grammar::View, qp_schema::scene::SceneDesc),
    Result<Box<qp_schema::protocol::ProtocolError>, String>,
> {
    let (mut session, principal) = scene_session(src).map_err(Err)?;
    let opts = qp_session::CompileOptions::new(width, height);
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

fn scene_cmd(args: &SceneArgs, explain: bool) -> Result<Outcome, String> {
    let (view, scene) = match compile_source(&args.source, args.width, args.height) {
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

fn render_view(args: &RenderArgs) -> Result<Outcome, String> {
    let scene = match compile_source(&args.source, args.width, args.height) {
        Ok((_, scene)) => scene,
        Err(Ok(e)) => return Ok(print_protocol_error(&e)),
        Err(Err(m)) => return Err(m),
    };
    let png = qp_render::render_scene_png(&scene).map_err(|e| e.to_string())?;
    std::fs::write(&args.out, png)
        .map_err(|e| format!("cannot write {}: {e}", args.out.display()))?;
    Ok(Outcome::Ok)
}

fn parse_param(s: &str) -> Result<(String, f64), String> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| format!("--param `{s}`: expected NAME=VALUE"))?;
    let v: f64 = v
        .trim()
        .parse()
        .map_err(|_| format!("--param `{s}`: value is not a number"))?;
    Ok((k.trim().to_owned(), v))
}

/// Loads the bars of a series file or of one series of a document.
fn load_bars(input: &Path, series: Option<&str>) -> Result<Vec<qp_schema::Bar>, String> {
    let text = std::fs::read_to_string(input)
        .map_err(|e| format!("cannot read {}: {e}", input.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("invalid JSON in {}: {e}", input.display()))?;
    let mut source: Box<dyn qp_source::SeriesSource> = if value.get("schema_version").is_some() {
        let doc: qp_schema::Document = serde_json::from_value(value)
            .map_err(|e| format!("{}: invalid document: {e}", input.display()))?;
        let base = input.parent().unwrap_or_else(|| Path::new("."));
        let chosen = match series {
            Some(id) => doc
                .series
                .iter()
                .find(|s| s.id == id)
                .ok_or_else(|| format!("{}: no series `{id}`", input.display()))?,
            None => doc
                .series
                .iter()
                .find(|s| {
                    s.bars().is_some() || matches!(s.source, qp_schema::SeriesSource::File { .. })
                })
                .ok_or_else(|| format!("{}: no series with bars", input.display()))?,
        };
        if chosen.bars().is_some() {
            Box::new(
                qp_source::MemorySource::from_series(chosen.clone())
                    .map_err(|e| format!("{}: {e}", input.display()))?,
            )
        } else {
            qp_source::open_binding(chosen, base).map_err(|e| e.to_string())?
        }
    } else {
        if series.is_some() {
            return Err("--series is only valid for a document input".to_owned());
        }
        Box::new(qp_source::FileSource::open(input).map_err(|e| e.to_string())?)
    };
    qp_source::collect_bars(source.as_mut()).map_err(|e| e.to_string())
}

fn compute(args: &ComputeArgs) -> Result<Outcome, String> {
    if args.list {
        let list: Vec<serde_json::Value> = qp_compute::STUDY_NAMES
            .iter()
            .map(|name| {
                let params = qp_compute::default_params(name).unwrap_or_default();
                let study = qp_compute::create(name, &params, qp_compute::Field::Close)
                    .expect("defaults are valid");
                serde_json::json!({
                    "study": name,
                    "outputs": study.outputs(),
                    "defaults": params,
                    "warmup": study.warmup(),
                })
            })
            .collect();
        return emit(&serde_json::Value::Array(list), args);
    }
    let (Some(input), Some(study)) = (args.input.as_deref(), args.study.as_deref()) else {
        return Err("INPUT and --study are required".to_owned());
    };
    let field = qp_compute::Field::parse(&args.field).map_err(|e| e.to_string())?;
    let bars = load_bars(input, args.series.as_deref())?;
    if let Some(sweep) = &args.sweep {
        let text = std::fs::read_to_string(sweep)
            .map_err(|e| format!("cannot read {}: {e}", sweep.display()))?;
        let sets: Vec<std::collections::BTreeMap<String, f64>> = serde_json::from_str(&text)
            .map_err(|e| {
                format!(
                    "{}: expected an array of parameter objects: {e}",
                    sweep.display()
                )
            })?;
        let column = field.column(&bars);
        let t: Vec<i64> = bars.iter().map(|b| b.t).collect();
        let mut outs = Vec::with_capacity(sets.len());
        let mut effective = Vec::with_capacity(sets.len());
        for p in &sets {
            effective.push(
                qp_compute::create(study, p, field)
                    .map_err(|e| e.to_string())?
                    .params(),
            );
        }
        qp_compute::sweep::sweep_each(study, &sets, &column, |vi, cols| {
            outs.push(qp_schema::StudyOutput {
                study: study.to_owned(),
                params: effective[vi].clone(),
                t: t.clone(),
                outputs: cols
                    .names
                    .iter()
                    .enumerate()
                    .map(|(j, n)| ((*n).to_owned(), cols.to_options(j)))
                    .collect(),
                warmup: cols
                    .names
                    .iter()
                    .zip(&cols.warmup)
                    .map(|(n, w)| ((*n).to_owned(), u32::try_from(*w).unwrap_or(u32::MAX)))
                    .collect(),
            });
        })
        .map_err(|e| e.to_string())?;
        let value = serde_json::to_value(&outs).map_err(|e| e.to_string())?;
        return emit(&value, args);
    }
    let params = args
        .params
        .iter()
        .map(|s| parse_param(s))
        .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
    let out = qp_compute::compute_bars(study, &params, field, &bars).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(&out).map_err(|e| e.to_string())?;
    emit(&value, args)
}

fn emit(value: &serde_json::Value, args: &ComputeArgs) -> Result<Outcome, String> {
    let mut text = if args.pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|e| e.to_string())?;
    text.push('\n');
    match &args.out {
        Some(path) => std::fs::write(path, text)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?,
        None => print!("{text}"),
    }
    Ok(Outcome::Ok)
}

fn schema_export(out: &Path) -> Result<Outcome, String> {
    let written = qp_schema::export_schemas(out)
        .map_err(|e| format!("cannot export schemas to {}: {e}", out.display()))?;
    for path in written {
        println!("{}", path.display());
    }
    Ok(Outcome::Ok)
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("invalid JSON in {}: {e}", path.display()))
}

fn schema_validate(
    file: &Path,
    type_name: &str,
    schema_dir: Option<&Path>,
) -> Result<Outcome, String> {
    let instance = read_json(file)?;
    let result = match schema_dir {
        Some(dir) => {
            let schema = read_json(&dir.join(format!("{type_name}.schema.json")))?;
            qp_schema::validate_with_schema(&schema, &instance)
        }
        None => {
            if qp_schema::json_schema(type_name).is_none() {
                return Err(format!(
                    "unknown type `{type_name}`; expected one of: {}",
                    qp_schema::root_type_names().join(", ")
                ));
            }
            qp_schema::validate(type_name, &instance)
        }
    };
    match result {
        Ok(()) => {
            println!("valid");
            Ok(Outcome::Ok)
        }
        Err(errors) => {
            for err in &errors {
                println!("error: {err}");
            }
            println!("invalid ({} error(s))", errors.len());
            Ok(Outcome::Invalid)
        }
    }
}

fn render(input: &Path, out: &Path, width: u32, height: u32) -> Result<Outcome, String> {
    let text = std::fs::read_to_string(input)
        .map_err(|e| format!("cannot read {}: {e}", input.display()))?;
    let bars = qp_render::parse_bars(&text).map_err(|e| format!("{}: {e}", input.display()))?;
    let scene = r0_scene(&bars, width, height)?;
    let png = qp_render::render_scene_png(&scene).map_err(|e| e.to_string())?;
    std::fs::write(out, png).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    Ok(Outcome::Ok)
}

/// The R0 chart (candles + SMA(20) of close, one linear panel) of a bar series: study
/// values from the compute core, layout from the grammar compiler; the renderer only
/// rasterizes the resulting scene.
fn r0_scene(
    series: &qp_schema::Series,
    width: u32,
    height: u32,
) -> Result<qp_schema::scene::SceneDesc, String> {
    const R0_VIEW: &str = r#"{"id":"r0","x":{"kind":"time"},
      "panels":[{"id":0,"y":{"kind":"linear"}}],
      "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
                {"id":"sma","mark":"line","series":"sma20","panel":0}]}"#;
    let bars = series.bars().ok_or("series has no embedded bars")?.to_vec();
    let params = std::collections::BTreeMap::from([("period".to_owned(), 20.0)]);
    let line = qp_compute::compute_bars("sma", &params, qp_compute::Field::Close, &bars)
        .map_err(|e| e.to_string())?
        .outputs
        .into_values()
        .next()
        .unwrap_or_default();
    let provider = qp_grammar::MemProvider::new()
        .with_bars("px", bars, series.symbol(), series.interval.as_deref())
        .with_values("sma20", "px", vec![("sma", line)], "sma", &["sma"])
        .with_label("sma20", "SMA20");
    let view = serde_json::from_str(R0_VIEW).map_err(|e| e.to_string())?;
    qp_grammar::compile(
        &view,
        &provider,
        &qp_grammar::CompileOptions::new(width, height),
    )
    .map_err(|e| e.to_string())
}

fn serve(init: &Path, root: &Path, batch_end: bool, host_frames: bool) -> Result<Outcome, String> {
    use std::io::{BufRead, Write};
    let init: qp_schema::protocol::SessionInit = qp_session::read_json(init)?;
    let mut session = qp_session::Session::new(init, root)?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let is_host = host_frames
            && serde_json::from_str::<serde_json::Value>(&line)
                .ok()
                .is_some_and(|v| v.get("host").is_some());
        let lines = if is_host {
            match session.handle_host_frame(&line) {
                Ok(o) => {
                    // Notices for the host (§18.4: a gap, a binding that became stale),
                    // before the connections' notifications.
                    for n in &o.notices {
                        writeln!(out, "{}", n.frame()).map_err(|e| format!("stdout: {e}"))?;
                    }
                    o.lines
                }
                Err(e) => {
                    let text = serde_json::json!({"host": {"error": e}}).to_string();
                    writeln!(out, "{text}").map_err(|e| format!("stdout: {e}"))?;
                    Vec::new()
                }
            }
        } else {
            session.handle_frame(&line)
        };
        for o in lines {
            let text = serde_json::to_string(&o).map_err(|e| e.to_string())?;
            writeln!(out, "{text}").map_err(|e| format!("stdout: {e}"))?;
        }
        if batch_end {
            writeln!(out, "{{\"batch_end\":true}}").map_err(|e| format!("stdout: {e}"))?;
        }
        out.flush().map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(Outcome::Ok)
}

fn mcp(init: &Path, principal: &str, surface: &str, root: &Path) -> Result<Outcome, String> {
    use std::io::{BufRead, Write};
    let init: qp_schema::protocol::SessionInit = qp_session::read_json(init)?;
    let session = qp_session::Session::new(init, root)?;
    let surface: qp_schema::protocol::Surface =
        serde_json::from_value(serde_json::json!({ "kind": surface }))
            .map_err(|e| format!("--surface `{surface}`: {e}"))?;
    let config = qp_mcp::Config::new(principal, surface);
    let mut server = qp_mcp::McpServer::connect(qp_mcp::Hub::shared(session), "mcp", config)
        .map_err(|e| format!("hello refused: {e}"))?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("stdin: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        for o in server.handle_line(&line) {
            writeln!(out, "{o}").map_err(|e| format!("stdout: {e}"))?;
        }
        out.flush().map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(Outcome::Ok)
}

fn conform(inputs: &[PathBuf], transport: &str, root: &Path) -> Result<Outcome, String> {
    use qp_session::conform::{collect_files, run_file, TransportKind};
    let kind = match transport {
        "inproc" => Some(TransportKind::InProc),
        "serve" => Some(TransportKind::Serve(
            std::env::current_exe().map_err(|e| format!("cannot locate qp: {e}"))?,
        )),
        "mcp" => None,
        other => {
            return Err(format!(
                "unknown transport `{other}`; expected inproc, serve or mcp"
            ))
        }
    };
    let files = collect_files(inputs)?;
    if files.is_empty() {
        return Err("no transcripts found".to_owned());
    }
    let mut failed = 0;
    for f in &files {
        let (o, carried) = match &kind {
            Some(k) => (run_file(f, root, k), String::new()),
            None => {
                let (o, s) = qp_mcp::conform::run_file(f, root);
                let note = format!(
                    " (mcp: {} hello, {} tool call(s), {} direct)",
                    s.hellos, s.tool_calls, s.direct
                );
                (o, note)
            }
        };
        match &o.failure {
            None => println!("PASS {}{carried}", o.name),
            Some(why) => {
                failed += 1;
                println!("FAIL {}: {why}", o.name);
            }
        }
    }
    println!(
        "{} transcript(s), {} passed, {failed} failed ({transport})",
        files.len(),
        files.len() - failed
    );
    Ok(if failed == 0 {
        Outcome::Ok
    } else {
        Outcome::Invalid
    })
}
