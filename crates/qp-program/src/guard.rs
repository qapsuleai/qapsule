//! The program-series guards of the `Check` phase (PS §1.1, §1.2), over every op of a
//! command: a program series is written only by the program ops (`E_PROGRAM_SERIES`), and
//! an event series enters a condition only as a `threshold` series (`E_EVENT_OPERAND`).

use qp_lang::Reason;
use qp_plane::error::ptr;
use qp_schema::protocol::{Op, ProtocolError};
use qp_schema::{Condition, Document};

use crate::inst::{claims_program, is_event_series, is_program_series};
use crate::refuse::one;

/// The first guard an op of `ops` breaks against `doc`.
///
/// # Errors
/// `E_PROGRAM_SERIES` or `E_EVENT_OPERAND` at the op's field.
#[allow(clippy::result_large_err)]
pub(crate) fn guards(doc: &Document, ops: &[Op]) -> Result<(), ProtocolError> {
    for (i, op) in ops.iter().enumerate() {
        let p = ptr("/ops", i);
        let direct = |path: &str, id: &str, how: &str| -> ProtocolError {
            let mut e = one(
                Reason::ProgramSeries,
                path,
                format!("`{id}` is a program series: {how}"),
            );
            e.hint = Some(
                "Program series change only through add_program, set_program_params and remove_program."
                    .to_owned(),
            );
            e
        };
        match op {
            Op::PutSeries { series } => {
                if claims_program(series) || is_program_series(doc, &series.id) {
                    let path = ptr(&p, "series");
                    return Err(direct(&path, &series.id, "put_series cannot write it"));
                }
            }
            Op::SetParams { series, .. } if is_program_series(doc, series) => {
                return Err(direct(
                    &ptr(&p, "series"),
                    series,
                    "its values change with set_program_params",
                ));
            }
            Op::Remove { id } if is_program_series(doc, id) => {
                return Err(direct(
                    &ptr(&p, "id"),
                    id,
                    "it goes with its instance (remove_program)",
                ));
            }
            Op::AddIntent {
                trigger: Some(c), ..
            } => event_operand(doc, c, &ptr(&p, "trigger"))?,
            Op::AddFinding { invalidation, .. } => {
                event_operand(doc, invalidation, &ptr(&p, "invalidation"))?
            }
            Op::UpdateFinding {
                invalidation: Some(c),
                ..
            } => event_operand(doc, c, &ptr(&p, "invalidation"))?,
            _ => {}
        }
    }
    Ok(())
}

/// An event series as a `compare` operand (PS §1.2: its last evaluable value is the last
/// closed bar's, compared with the other series' live bar).
#[allow(clippy::result_large_err)]
fn event_operand(doc: &Document, c: &Condition, p: &str) -> Result<(), ProtocolError> {
    let Condition::Compare { left, right, .. } = c else {
        return Ok(());
    };
    for (field, id) in [("left", left), ("right", right)] {
        if is_event_series(doc, id) {
            let mut e = one(
                Reason::EventOperand,
                &ptr(p, field),
                format!(
                    "`{id}` is an event series: it enters a condition only as a threshold series"
                ),
            );
            e.hint = Some(format!(
                "Use {{\"kind\":\"threshold\",\"series\":\"{id}\",\"op\":\"ge\",\"value\":1}}."
            ));
            return Err(e);
        }
    }
    Ok(())
}
