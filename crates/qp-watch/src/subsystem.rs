//! The watch subsystem: its ops, and its [`Phase::AfterData`] collect step.

use qp_subsystem::{Moment, OpHandler, Phase, SessionCtx, Subsystem};

use crate::matching::{match_watches, watch_lines, Matched};
use crate::ops::{ADD_WATCH, REMOVE_WATCH};

/// The watch subsystem (`izleme niyeti`): register it with the authority.
pub struct Watch;

/// The watch subsystem, as the composition registers it.
pub static WATCH: Watch = Watch;

/// The watch ops, in this order.
static OPS: &[&dyn OpHandler] = &[&ADD_WATCH, &REMOVE_WATCH];

/// The matches of the current look, between its collect ([`Moment::Look`]) and the line
/// its entry was written at ([`Moment::Written`]): the authority's world's and the public
/// world's.
#[derive(Debug, Default)]
pub(crate) struct Pending {
    full: Vec<Matched>,
    public: Vec<Matched>,
}

impl Subsystem for Watch {
    fn name(&self) -> &'static str {
        "watch"
    }
    fn init(&self, ctx: &mut SessionCtx<'_>) {
        ctx.slots.put(Pending::default());
    }
    fn phases(&self) -> &'static [Phase] {
        &[Phase::AfterData]
    }
    fn ops(&self) -> &'static [&'static dyn OpHandler] {
        OPS
    }
    fn collect(&self, _phase: Phase, at: &Moment<'_>, ctx: &mut SessionCtx<'_>) {
        match *at {
            // (R5B, §25.5) Watches, in the look's engine entry: the authority's world first
            // (a new look), then the public world's.
            Moment::Look { events, public } => {
                let (changes, found) = match_watches(ctx.view.document(), ctx.view.clock(), events);
                ctx.out.amend(changes);
                let Some(p) = ctx.slots.get_mut::<Pending>() else {
                    return;
                };
                if public {
                    p.public = found;
                } else {
                    *p = Pending {
                        full: found,
                        public: Vec::new(),
                    };
                }
            }
            // The look's entry was written: its `intent_matched` events, at this line.
            Moment::Written { seq, public } => {
                let Some(p) = ctx.slots.get_mut::<Pending>() else {
                    return;
                };
                let p = std::mem::take(p);
                if p.full.is_empty() && p.public.is_empty() {
                    return;
                }
                for l in watch_lines(ctx.view, &p.full, seq, &p.public, public) {
                    ctx.out.say(l);
                }
            }
            _ => {}
        }
    }
}
