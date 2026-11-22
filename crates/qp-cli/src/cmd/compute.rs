//! `qp compute`: bars in, study values out (RG1: split out of `main.rs`).

use crate::*;

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

#[allow(clippy::expect_used)]
pub(crate) fn compute(args: &ComputeArgs) -> Result<Outcome, String> {
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
