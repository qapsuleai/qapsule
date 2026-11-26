//! Programs in a session (RL1, `docs/PROGRAMS-SESSION.md` §1–§3): the subsystem that puts
//! compute-language programs (`qp-ir/0`, `qp-lang`) into a document.
//!
//! - `add_program` → one compute series per `series`/`event` output (`<instance>.<output>`,
//!   `SeriesSource::Study { study: "program", program: Some(ProgramRef) }`, the canonical
//!   IR inline, `since_t` = the expansion's clock, owner = the person who decided), with
//!   `show` one layer per output; `set_program_params` → the same series with new values,
//!   `instance_id` and `since_t`; `remove_program` → the `remove` rule per output. Signal
//!   and state outputs are try-only (`E_TRY_ONLY`). Changes are existing `ChangeOp`s:
//!   undo and log replay need no code here.
//! - Policy (PS §2.2): `write`; an agent only proposes add (a new instance) and set, and
//!   never removes; the person decides.
//! - [`qp_subsystem::Phase::Check`]: program series are written only by the program ops
//!   (`E_PROGRAM_SERIES`), an event series enters a condition only as a `threshold`
//!   series (`E_EVENT_OPERAND`), and the program ops of the command (or of the proposal
//!   it accepts) are re-checked under the host's limits with the owner's live budget.
//! - Queries `check_program`, `try_program`, `explain_program` (IR, text or instance).
//!
//! Knows no authority: only the contract (`qp-subsystem`). Values of program series are
//! computed by the data plane (`qp-plane`), not here.

mod asked;
mod budget;
mod check;
mod expand;
mod guard;
mod inst;
mod ops;
mod query;
mod refuse;
mod show;
mod subsystem;

pub use subsystem::{Programs, PROGRAM};
