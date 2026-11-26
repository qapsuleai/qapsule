//! Program instances in a document (PS §1.1): a program output series carries its
//! `ProgramRef` inline; an instance is the set of series with the same `instance`.

use qp_schema::program::{OutputKind, ProgramRef, PROGRAM_STUDY};
use qp_schema::{Document, Series, SeriesSource};

/// The program behind series `s`, if it is a program output series.
pub(crate) fn program_of(s: &Series) -> Option<&ProgramRef> {
    match &s.source {
        SeriesSource::Study {
            program: Some(p), ..
        } => Some(p),
        _ => None,
    }
}

/// Whether `s` is (or claims to be) a program series: `study: "program"` or a `program`.
pub(crate) fn claims_program(s: &Series) -> bool {
    match &s.source {
        SeriesSource::Study { study, program, .. } => study == PROGRAM_STUDY || program.is_some(),
        _ => false,
    }
}

/// Whether series `id` of `doc` is a program series.
pub(crate) fn is_program_series(doc: &Document, id: &str) -> bool {
    doc.series.iter().any(|s| s.id == id && claims_program(s))
}

/// The output series of instance `name`, in document order.
pub(crate) fn outputs_of<'d>(doc: &'d Document, name: &str) -> Vec<&'d Series> {
    doc.series
        .iter()
        .filter(|s| program_of(s).is_some_and(|p| p.instance == name))
        .collect()
}

/// Every instance name of `doc`, in document order, each once.
pub(crate) fn instance_names(doc: &Document) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for p in doc.series.iter().filter_map(program_of) {
        if !out.contains(&p.instance.as_str()) {
            out.push(&p.instance);
        }
    }
    out
}

/// The output name of program series `s`.
pub(crate) fn output_name(s: &Series) -> Option<&str> {
    match &s.source {
        SeriesSource::Study { output, .. } => output.as_deref(),
        _ => None,
    }
}

/// The bar series a program series reads.
pub(crate) fn input_of(s: &Series) -> Option<&str> {
    match &s.source {
        SeriesSource::Study { inputs, .. } => inputs.first().map(String::as_str),
        _ => None,
    }
}

/// Whether series `id` of `doc` is an event series (an `event` output of a program).
pub(crate) fn is_event_series(doc: &Document, id: &str) -> bool {
    let Some(s) = doc.series.iter().find(|s| s.id == id) else {
        return false;
    };
    let (Some(p), Some(o)) = (program_of(s), output_name(s)) else {
        return false;
    };
    p.ir.outputs
        .iter()
        .any(|out| out.name == o && matches!(out.kind, OutputKind::Event { .. }))
}
