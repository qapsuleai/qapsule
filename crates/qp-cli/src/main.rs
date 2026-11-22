//! `qp` command line: a thin shell over `qp-schema`, `qp-source`, `qp-compute`,
//! `qp-render`, `qp-session` and `qp-mcp`.
//!
//! It only parses arguments, reads/writes files and calls the libraries.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use qp_host::{demo, live, mcp_serve, sim};

mod cmd;

use cmd::*;

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
        /// With `init.replay` (docs/PROTOCOL.md §21): never read local data (a `local`
        /// source falls back to the synthetic market). Conformance runs use it.
        #[arg(long)]
        no_local_data: bool,
    },
    /// Run an MCP server over stdin/stdout (newline-delimited JSON-RPC, MCP stdio
    /// transport), speaking `qp/0` as one principal (rounds/RD1/EMBEDDING.md §1); or,
    /// with `serve`, the Qapsule MCP server for agents (R5B: stdio or HTTP, a running
    /// session, agent principals only, tool surfaces).
    Mcp(McpArgs),
    /// Print a plain-text semantic summary of a view, read from its compiled scene
    /// (panels, scales and visible domains, layers with data, origins and last values,
    /// emphasis). Headless: nothing is rendered.
    Explain(SceneArgs),
    /// Print the compiled scene description (`SceneDesc` JSON) of a view.
    Scene(SceneArgs),
    /// (R5/T3) The looks (design records): list them, show one as JSON, or its CSS
    /// custom properties.
    Look {
        #[command(subcommand)]
        command: LookCommand,
    },
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
        /// (RP3/M15, §22) The viewer variant: the device opens the capsule and holds only
        /// that instrument's slice, claims and data (a second instrument stays on the
        /// authority); writes the outcome instead of a tape.
        #[arg(long)]
        viewer: bool,
    },
    /// Replay historical bars like a live feed (docs/PROTOCOL.md §21): local real data
    /// (`fixtures/local/bars`, or `--local-bars`) or the synthetic market, through the
    /// reference authority with the capsule engine on every instrument. Writes one JSON
    /// line per message the authority sends a watching connection (owner `arkin`,
    /// subscribed to `capsules` and `changes`) and per replay result; after a backward
    /// seek (a new authority, `reset: true`) it connects again the same way and goes on;
    /// two runs with the same arguments write the same bytes.
    Replay(ReplayArgs),
    /// (R4/T5) The local demo: replay host + authority + the web shell on one origin
    /// (`scripts/demo.sh`; `shells/web/README.md`).
    Demo(demo::DemoArgs),
    /// (RP4/T4) Accelerated simulation: cached historical trades (`fixtures/local/trades`)
    /// through the live path (`LiveFeed` with trades → live host → capsule engine) on a
    /// simulated exchange clock, `--speed N` or `max`, over several days (XIST calendar).
    /// Prints counts and a digest; `--out` (under `fixtures/local/`) gets the event
    /// stream. Same cache and arguments → byte-equal stream (`docs/matriks/LIVE.md` §8).
    Sim(sim::SimArgs),
    /// (R4/T5) Drive the demo session with protocol messages only (no shell) and print
    /// the document and scene digests the web shell reports (parity check).
    Drive(demo::DriveArgs),
    /// (R6) The live Matriks connection: record raw messages, inspect a raw log
    /// (`docs/matriks/LIVE.md`). Values come from the environment (`scripts/live.sh`).
    Live {
        #[command(subcommand)]
        command: live::LiveCommand,
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

#[derive(Debug, clap::Args)]
struct ReplayArgs {
    /// Session init with `init.replay` (default: the demo init built from the flags).
    #[arg(long)]
    init: Option<PathBuf>,
    /// `synthetic` or `local` (falls back to synthetic, and says so, without data).
    #[arg(long, default_value = "synthetic")]
    source: String,
    /// Local bars directory (default: `$QAPSULE_LOCAL_BARS` or `fixtures/local/bars`).
    #[arg(long)]
    local_bars: Option<PathBuf>,
    /// Number of instruments.
    #[arg(long, default_value_t = 30)]
    instruments: u32,
    /// Synthetic market: trading days.
    #[arg(long, default_value_t = 260)]
    days: u32,
    /// Synthetic market: seed.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Feed interval.
    #[arg(long, default_value = "1d")]
    interval: String,
    /// Finer interval that moves a bar inside its day (`1h`; `none` for the OHLC path).
    #[arg(long, default_value = "1h")]
    fine: String,
    /// Bars of history before the first replayed bar (without `--from`).
    #[arg(long, default_value_t = 120)]
    history: u32,
    /// First replayed day (`YYYY-MM-DD`).
    #[arg(long)]
    from: Option<String>,
    /// Last replayed day (`YYYY-MM-DD`).
    #[arg(long)]
    to: Option<String>,
    /// Steps to take (default: all).
    #[arg(long)]
    steps: Option<u64>,
    /// Recorded controls: a JSON array of `{"at": <step>, "control": <ReplayControl>}`,
    /// applied when the replay has taken `at` steps (before the next one).
    #[arg(long)]
    controls: Option<PathBuf>,
    /// What to write: `messages` (default), `events` (the data events, `{"t","event"}`)
    /// or `none`.
    #[arg(long, default_value = "messages")]
    emit: String,
    /// Print a timing summary to stderr (steps, events, ms per step, per virtual day).
    #[arg(long)]
    stats: bool,
    /// Write here instead of stdout.
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Directory that document paths in `--init` are relative to.
    #[arg(long, default_value = ".")]
    root: PathBuf,
    /// (R6/T5) Replay a Matriks raw log (`*.qpraw`, under `fixtures/local/`) through the
    /// live feed (`qp_matriks::LiveFeed`, the same pipeline as the live connector): one
    /// JSON line per output (data event, clock, connection, phase, final bar). The other
    /// flags above do not apply; see `--intervals`, `--method`, `--volume-field`, `--time`.
    #[arg(long)]
    raw: Option<PathBuf>,
    #[command(flatten)]
    feed: Box<live::FeedArgs>,
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

/// `qp mcp`: the RD1 adapter (flags) or `qp mcp serve` (R5B).
#[derive(Debug, clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
struct McpArgs {
    #[command(subcommand)]
    command: Option<McpCommand>,
    /// Session init (`SessionInit` JSON).
    #[arg(long)]
    init: Option<PathBuf>,
    /// Principal the adapter speaks as (sent in its `hello`).
    #[arg(long)]
    principal: Option<String>,
    /// Surface kind: desktop, phone, watch, voice or headless.
    #[arg(long, default_value = "headless")]
    surface: String,
    /// Directory that document paths in the init are relative to (and confined to).
    #[arg(long, default_value = ".")]
    root: PathBuf,
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    /// (R5B/T2) The Qapsule MCP server for agents over a running session: stdio
    /// (default) or Streamable HTTP (`--http`, this machine only unless `--listen` with
    /// `--token-env`); the principal must be an agent; `--surface` picks the tool surface.
    Serve(mcp_serve::ServeArgs),
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
    /// (R5/T3) Look to compile with (`qp look list`; default `r4`).
    #[arg(long)]
    look: Option<String>,
    /// (R5/T3) Write each text's type role into the scene (`role`; for measurement).
    #[arg(long)]
    roles: bool,
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
    /// (R5/T3) Look to draw with (`qp look list`; default `r4`).
    #[arg(long)]
    look: Option<String>,
    /// (RD2/G) Device pixel scale: the PNG is `width·N × height·N` (same layout, sharper;
    /// the comparison gallery uses 2 for desktop, 3 for phone). 1 is the reference path.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=3))]
    scale: u32,
    /// (RD2/G) Scheme override, `dark` or `light` (default: the view's theme).
    #[arg(long)]
    theme: Option<String>,
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

/// (R5/T3) `qp look`: the design records (`qp_grammar::look`).
#[derive(Debug, Subcommand)]
enum LookCommand {
    /// The registered looks, one per line: name, default scheme, note (default first).
    List,
    /// One look's whole record as JSON (fonts, type roles, colour roles of both schemes,
    /// spacing, surface, glyph, motion).
    Show {
        /// Look name.
        name: String,
        /// Pretty-print the JSON.
        #[arg(long)]
        pretty: bool,
    },
    /// The shell chrome's CSS custom properties of a look in a scheme, one per line.
    Css {
        /// Look name.
        name: String,
        /// `dark` or `light` (default: the look's own).
        #[arg(long)]
        theme: Option<String>,
    },
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
        Command::Demo(args) => demo::demo(&args).map(|()| Outcome::Ok),
        Command::Sim(args) => sim::sim(&args).map(|()| Outcome::Ok),
        Command::Drive(args) => demo::drive(&args).map(|()| Outcome::Ok),
        Command::Live { command } => live::run(&command).map(|()| Outcome::Ok),
        Command::Render(args) => match &args.input {
            Some(input) => render(input, &args),
            None => render_view(&args),
        },
        Command::Serve {
            init,
            root,
            batch_end,
            host_frames,
            no_local_data,
        } => {
            let init: qp_schema::protocol::SessionInit = qp_session::read_json(&init)?;
            if init.replay.is_some() {
                let dir = if no_local_data {
                    PathBuf::new()
                } else {
                    qp_source::local_bars_dir()
                };
                let host = qp_session::ReplayHost::with_local_dir(init, &root, &dir)?;
                serve_replay(host, batch_end, host_frames)
            } else {
                serve(init, &root, batch_end, host_frames)
            }
        }
        Command::Mcp(McpArgs {
            command: Some(McpCommand::Serve(args)),
            ..
        }) => mcp_serve::serve(&args).map(|()| Outcome::Ok),
        Command::Mcp(McpArgs {
            command: None,
            init,
            principal,
            surface,
            root,
        }) => {
            let (Some(init), Some(principal)) = (init, principal) else {
                return Err("qp mcp needs --init and --principal (or: qp mcp serve)".to_owned());
            };
            mcp(&init, &principal, &surface, &root)
        }
        Command::Conform {
            inputs,
            transport,
            root,
        } => conform(&inputs, &transport, &root),
        Command::ReplicaTape { lag, out, viewer } => {
            if viewer {
                viewer_tape(lag, &out)
            } else {
                replica_tape(lag, &out)
            }
        }
        Command::Replay(args) => replay_cmd(&args),
        Command::Explain(args) => scene_cmd(&args, true),
        Command::Scene(args) => scene_cmd(&args, false),
        Command::Look { command } => look_cmd(&command),
    }
}

fn render(input: &Path, args: &RenderArgs) -> Result<Outcome, String> {
    let text = std::fs::read_to_string(input)
        .map_err(|e| format!("cannot read {}: {e}", input.display()))?;
    let bars = qp_render::parse_bars(&text).map_err(|e| format!("{}: {e}", input.display()))?;
    let scene = r0_scene(&bars, args.width, args.height)?;
    write_png(&scene, args.scale, &args.out)
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
