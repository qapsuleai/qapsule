//! `qp program check | run | explain <file>` (RL1, PS §3.3): the program queries without a
//! session. The file is text or IR JSON: a file that starts with `{` or `[` is JSON (if
//! it does not parse, the answer is the coded `E_PARSE` with a hint; it is never read as
//! text), anything else is text. The file's size is checked before it is read
//! ([`file_cap`]); a larger file is the coded `E_TOO_LARGE`. Limits are the parameter
//! registry's defaults, text limits its `program.text.*` keys, so `qp program check`
//! answers exactly what `check_program` answers in a session with default parameters (VERIFY 16).
//!
//! Exit codes: 0 — the answer was printed (`check`: a program that checks, or a coded
//! refusal; `run` / `explain`: it ran); 1 — the program has errors (the `program_check`
//! answer with them is printed; `run` / `explain` of a refusal too); 2 — usage, I/O or
//! data errors (message on stderr).

mod full;

use std::path::{Path, PathBuf};

use qp_lang::{check, Binding, CheckCtx, CheckReport, Lang, Limit, Limits, Purpose, Reason};
use qp_lang_text::{lower_with, SourceMap, TextLimits};
use qp_schema::program::{Program, ProgramDoc};
use qp_schema::protocol::{Language, ProgramError, QueryResult};
use serde_json::Value;

use crate::Outcome;

/// `qp program` subcommands.
#[derive(Debug, clap::Subcommand)]
pub(crate) enum ProgramCommand {
    /// Check a program (`check_program`): every error at once, warnings, cost, outputs,
    /// the IR of a text. Exit 0 when it checks or is a coded refusal (`reject`), 1 when
    /// it has errors.
    Check(ProgramArgs),
    /// Run a program over a closed window of bars (`try_program`'s core; every bar,
    /// the last one too, is closed). Prints the try summary, or with `--full` every
    /// output bar by bar.
    Run(RunArgs),
    /// Explain a program (`explain_program`): canonical text, identities, per node cost.
    Explain(ProgramArgs),
}

/// Arguments of `check` and `explain`.
#[derive(Debug, clap::Args)]
pub(crate) struct ProgramArgs {
    /// Print the protocol answer as JSON (`program_check` / `program_explain`).
    #[arg(long)]
    json: bool,
    /// Message language: `tr` or `en` (default `en`).
    #[arg(long, value_parser = ["tr", "en"])]
    lang: Option<String>,
    /// Program file: text or IR JSON.
    file: PathBuf,
}

/// Arguments of `run`.
#[derive(Debug, clap::Args)]
pub(crate) struct RunArgs {
    /// Print JSON (`program_try`, or `program_run` with `--full`).
    #[arg(long)]
    json: bool,
    /// Every output bar by bar: series `values`, event `bars` and `t`, signal `records`
    /// (with `bar`, `exit_bar`), state `keys` and `counts`.
    #[arg(long)]
    full: bool,
    /// Message language: `tr` or `en` (default `en`).
    #[arg(long, value_parser = ["tr", "en"])]
    lang: Option<String>,
    /// Bar series file (`fixtures/bars` format: a `Series` with embedded bars).
    #[arg(long)]
    data: PathBuf,
    /// Program file: text or IR JSON.
    file: PathBuf,
}

pub(crate) fn program_cmd(command: &ProgramCommand) -> Result<Outcome, String> {
    match command {
        ProgramCommand::Check(a) => check_cmd(a),
        ProgramCommand::Run(a) => full::run_cmd(a),
        ProgramCommand::Explain(a) => explain_cmd(a),
    }
}

/// The limits in force: the parameter registry's defaults (PR §8.4).
pub(crate) fn limits() -> Limits {
    Limits::from_params(|key| qp_schema::params::by_key(key).and_then(|d| d.default.as_f64()))
}

/// The text language's limits (`program.text.*`).
fn text_limits(l: &Limits) -> TextLimits {
    let n = |k| usize::try_from(l.get(k)).unwrap_or(usize::MAX);
    TextLimits {
        max_source_chars: n(Limit::TextMaxSourceChars),
        max_token_chars: n(Limit::TextMaxTokenChars),
        max_depth: n(Limit::TextMaxDepth),
    }
}

fn language(lang: Option<&str>) -> (Lang, Language) {
    match lang {
        Some("tr") => (Lang::Tr, Language::Tr),
        _ => (Lang::En, Language::En),
    }
}

/// The largest program file read, in bytes: `max(program.text.max_source_chars × 4,
/// program.max_ir_bytes)` (a UTF-8 character is at most 4 bytes). Checked on the file's
/// size before reading, and the read is bounded by it.
pub(crate) fn file_cap(l: &Limits) -> u64 {
    l.get(Limit::TextMaxSourceChars)
        .saturating_mul(4)
        .max(l.get(Limit::MaxIrBytes))
}

/// Reads at most `cap` bytes of `path`; a larger file is the coded `E_TOO_LARGE`.
fn read_capped(path: &Path, cap: u64) -> Result<Result<String, ProgramError>, String> {
    use std::io::Read as _;
    let shown = path.display();
    let too_large = |n: String| {
        let mut e = Reason::TooLarge.error(
            Some(String::new()),
            format!("the program file is {n} bytes; at most {cap} are read"),
        );
        e.expected = vec![cap.to_string()];
        e.hint = Some(
            "the cap is max(program.text.max_source_chars × 4, program.max_ir_bytes)".to_owned(),
        );
        e
    };
    let meta = std::fs::metadata(path).map_err(|e| format!("cannot read {shown}: {e}"))?;
    if meta.len() > cap {
        return Ok(Err(too_large(meta.len().to_string())));
    }
    let f = std::fs::File::open(path).map_err(|e| format!("cannot read {shown}: {e}"))?;
    let mut bytes = Vec::new();
    f.take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read {shown}: {e}"))?;
    if bytes.len() as u64 > cap {
        return Ok(Err(too_large(format!("more than {cap}"))));
    }
    String::from_utf8(bytes)
        .map(Ok)
        .map_err(|e| format!("{shown}: not UTF-8: {e}"))
}

/// A program file, read: the IR as the checker reads it, and for a text its source map
/// and lowered program.
pub(crate) struct Loaded {
    raw: Value,
    map: Option<SourceMap>,
    lowered: Option<Program>,
}

/// Reads `path`: IR JSON or text. `Ok(Err(errors))`: the text did not lower.
fn load(
    path: &Path,
    lang: Language,
    l: &Limits,
) -> Result<Result<Loaded, Vec<ProgramError>>, String> {
    let text = match read_capped(path, file_cap(l))? {
        Ok(t) => t,
        Err(e) => return Ok(Err(vec![e])),
    };
    let head = text.trim_start();
    if head.starts_with('{') || head.starts_with('[') {
        return match serde_json::from_str::<Value>(&text) {
            Ok(raw) => Ok(Ok(Loaded {
                raw,
                map: None,
                lowered: None,
            })),
            Err(e) => {
                let mut err = Reason::Parse.error(
                    Some(String::new()),
                    format!("the file starts like JSON but is not valid JSON: {e}"),
                );
                err.hint = Some(
                    "an IR file must be one JSON object (nesting at most 128 deep); \
                     a text program must not start with `{` or `[`"
                        .to_owned(),
                );
                Ok(Err(vec![err]))
            }
        };
    }
    let lowered = match lower_with(&text, lang, &text_limits(l)) {
        Ok(x) => x,
        Err(errors) => return Ok(Err(errors)),
    };
    let program = match lowered.doc() {
        Ok(ProgramDoc::Program(p)) => Some(p),
        _ => None,
    };
    Ok(Ok(Loaded {
        raw: lowered.ir,
        map: Some(lowered.map),
        lowered: program,
    }))
}

/// The checker's report on `p`, its text errors mapped to source positions.
fn report(p: &Loaded, binding: Option<Binding>, lang: Lang, l: Limits) -> CheckReport {
    let ctx = CheckCtx {
        limits: l,
        binding,
        params: None,
        lang,
        purpose: Purpose::Check,
    };
    let mut rep = check(&p.raw, &ctx);
    if let Some(map) = &p.map {
        map.attach(&mut rep.errors);
        map.attach(&mut rep.warnings);
    }
    rep
}

/// The `program_check` answer of text that did not lower (as `check_program` gives it).
fn lowering_answer(errors: Vec<ProgramError>, l: Limits) -> QueryResult {
    let mut ans = CheckReport {
        errors,
        truncated: false,
        warnings: Vec::new(),
        cost: None,
        outputs: Vec::new(),
        params: Vec::new(),
        program_id: None,
        instance_id: None,
        reject: None,
        checked: None,
        limits: l,
    }
    .to_answer();
    ans.ok = false;
    QueryResult::ProgramCheck(Box::new(ans))
}

/// What a command does with a program file: checked (with its report), or the
/// `program_check` answer to print instead (text errors, check errors, a refusal).
pub(crate) enum Prepared {
    /// It checks.
    Checked(Box<CheckReport>),
    /// It does not (text or check errors, or a refusal): the answer to print.
    Answer(Box<QueryResult>),
}

/// Loads and checks a program file.
pub(crate) fn prepare(
    path: &Path,
    lang: Option<&str>,
    binding: Option<Binding>,
) -> Result<Prepared, String> {
    let l = limits();
    let (lang, language) = language(lang);
    let p = match load(path, language, &l)? {
        Ok(p) => p,
        Err(errors) => return Ok(Prepared::Answer(Box::new(lowering_answer(errors, l)))),
    };
    let rep = report(&p, binding, lang, l);
    if rep.checked.is_some() {
        return Ok(Prepared::Checked(Box::new(rep)));
    }
    let mut ans = rep.to_answer();
    ans.ir = p.lowered;
    Ok(Prepared::Answer(Box::new(QueryResult::ProgramCheck(
        Box::new(ans),
    ))))
}

fn check_cmd(a: &ProgramArgs) -> Result<Outcome, String> {
    let l = limits();
    let (lang, language) = language(a.lang.as_deref());
    let answer = match load(&a.file, language, &l)? {
        Err(errors) => lowering_answer(errors, l),
        Ok(p) => {
            let rep = report(&p, None, lang, l);
            let mut ans = rep.to_answer();
            ans.ir = p.lowered;
            ans.warnings.extend(live_budget_warning(&rep));
            QueryResult::ProgramCheck(Box::new(ans))
        }
    };
    let QueryResult::ProgramCheck(c) = &answer else {
        return Err("internal: not a check answer".to_owned());
    };
    let ok = c.ok;
    if a.json {
        print_json(&answer)?;
    } else {
        print!("{}", check_text(c));
    }
    Ok(if ok { Outcome::Ok } else { Outcome::Invalid })
}

/// The warning `check_program` gives in a session (PR §11.3) when the program's live cost
/// alone is over the per-owner live limit (`program.live.doc.max_cost_per_bar`, the
/// registry's default here): it checks, but `add_program` would refuse it (`E_BUDGET`).
/// Same bytes as the session's (VERIFY 16).
fn live_budget_warning(rep: &CheckReport) -> Option<ProgramError> {
    let live = rep.cost.as_ref()?.live_per_bar;
    let limit = rep.limits.get(Limit::LiveDocMaxCostPerBar);
    (live > limit).then(|| ProgramError {
        code: None,
        reason: "W_LIVE_BUDGET".to_owned(),
        message: format!(
            "live cost {live} per bar is over the per-owner live limit {limit}: add_program refuses it"
        ),
        at: None,
        span: None,
        expected: vec![limit.to_string()],
        did_you_mean: Vec::new(),
        hint: Some("Lower windows or outputs; try_program still runs it.".to_owned()),
    })
}

/// A check answer as plain text: the verdict, then one line per error and warning.
pub(crate) fn check_text(c: &qp_schema::program::ProgramCheck) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    if let Some(r) = &c.reject {
        let _ = writeln!(s, "reject {}: {}", r.code, r.reason);
    } else if c.ok {
        let _ = writeln!(s, "ok {}", c.program_id.as_deref().unwrap_or_default());
        for o in &c.outputs {
            let kind = serde_json::to_value(o.kind).unwrap_or(Value::Null);
            let _ = writeln!(s, "output {} {}", o.name, kind.as_str().unwrap_or("?"));
        }
    } else {
        let _ = writeln!(s, "errors {}", c.errors.len());
    }
    for e in c.errors.iter().chain(&c.warnings) {
        let at = match (&e.span, &e.at) {
            (Some(sp), _) => format!("{}:{}", sp.line, sp.col),
            (None, Some(at)) => at.clone(),
            (None, None) => "-".to_owned(),
        };
        let _ = write!(s, "{} {at}: {}", e.reason, e.message);
        if !e.did_you_mean.is_empty() {
            let _ = write!(s, " (did you mean {})", e.did_you_mean.join(", "));
        }
        s.push('\n');
    }
    s
}

fn explain_cmd(a: &ProgramArgs) -> Result<Outcome, String> {
    let rep = match prepare(&a.file, a.lang.as_deref(), None)? {
        Prepared::Checked(rep) => rep,
        Prepared::Answer(ans) => return print_refused(&ans, a.json),
    };
    let answer = full::explain(&rep)?;
    if a.json {
        print_json(&answer)?;
    } else if let QueryResult::ProgramExplain(x) = &answer {
        print!("{}", x.text);
        if !x.text.ends_with('\n') {
            println!();
        }
        println!("# program_id {}", x.program_id);
        for c in &x.cost {
            println!("# {} {} {} w={}", c.id, c.op, c.clock, c.w);
        }
    }
    Ok(Outcome::Ok)
}

/// Prints the `program_check` answer of a program that cannot run; exit 1.
pub(crate) fn print_refused(ans: &QueryResult, json: bool) -> Result<Outcome, String> {
    if json {
        print_json(ans)?;
    } else if let QueryResult::ProgramCheck(c) = ans {
        print!("{}", check_text(c));
    }
    Ok(Outcome::Invalid)
}

/// One JSON line on stdout.
pub(crate) fn print_json(v: &QueryResult) -> Result<(), String> {
    print_value(&serde_json::to_value(v).map_err(|e| e.to_string())?)
}

/// One JSON line on stdout.
pub(crate) fn print_value(v: &Value) -> Result<(), String> {
    println!("{v}");
    Ok(())
}
