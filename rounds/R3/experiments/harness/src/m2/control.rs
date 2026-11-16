//! **Control — not a candidate.** A deliberately trivial realizer for testing the harness
//! itself: it prints machine codes, the same in both languages, e.g.
//! `event breakout up stage=born bars_ago=3 holds=na`. It has no tense, so the time
//! classifier must flag it, and it prints digits, so the leakage check must flag every
//! redacted situation. Do not start a candidate from it.

use qp_capsule::{Lang, Realizer, SentenceInput};

use super::situations::{dir_name, stage_name, time_name};

/// The control realizer.
#[derive(Debug, Clone, Copy, Default)]
pub struct Control;

impl Realizer for Control {
    fn name(&self) -> &'static str {
        "control"
    }

    fn sentence(&self, inp: &SentenceInput<'_>, _lang: Lang) -> String {
        let s = inp.story;
        format!(
            "{} {} {} stage={} bars_ago={} holds={}",
            time_name(s.time),
            s.kind.name(),
            dir_name(s.direction),
            stage_name(s.stage),
            inp.now_bar.saturating_sub(inp.lead.bar),
            match inp.still_holds {
                Some(true) => "yes",
                Some(false) => "no",
                None => "na",
            }
        )
    }
}
