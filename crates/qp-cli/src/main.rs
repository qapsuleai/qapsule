//! `qp` command line: a thin shell over `qp-schema` and `qp-render`.
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
    /// Render a bars JSON file to PNG with the reference renderer.
    Render {
        /// Input bars JSON file.
        input: PathBuf,
        /// Output PNG file.
        #[arg(short, long)]
        out: PathBuf,
        /// Image width in pixels.
        #[arg(long, default_value_t = qp_render::DEFAULT_WIDTH)]
        width: u32,
        /// Image height in pixels.
        #[arg(long, default_value_t = qp_render::DEFAULT_HEIGHT)]
        height: u32,
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
        Command::Render {
            input,
            out,
            width,
            height,
        } => render(&input, &out, width, height),
    }
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
    let png = qp_render::render_png(&bars, width, height).map_err(|e| e.to_string())?;
    std::fs::write(out, png).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    Ok(Outcome::Ok)
}
