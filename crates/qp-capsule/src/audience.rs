//! Sensitivity filter (`docs/CAPSULE.md` §9.8, K31; R4: K42/A22).

use qp_schema::capsule::{Audience, Sensitivity, Stage, Story};

/// Whether `story`, as of its current stage, reaches `audience`: [`admits_at`] with
/// `story.stage` (an engine event's story snapshot carries the transition's stage).
#[must_use]
pub fn admits(audience: &Audience, story: &Story, intents_on: &[String]) -> bool {
    admits_at(audience, story, story.stage, intents_on)
}

/// Whether a transition to `stage` of `story` reaches `audience`.
///
/// - (K42/A22) A **negative transition** ([`Stage::is_negative`]: `weakened`,
///   `invalidated`, `retracted`) of a story about an instrument the person holds a
///   position in (non-zero quantity) or has an active intent on (`intents_on`) always
///   passes, whatever the level.
/// - Everything else, births included (and every transition of a story the person is not
///   involved in), goes through the tier filter of the effective sensitivity:
///   `important` = tier 1, `balanced` = tiers 1–2, `all` = every tier.
///
/// Pure; never changes the audience (the system does not change the level by itself).
/// Watch-list selection belongs to the subscription layer, not to this filter.
#[must_use]
pub fn admits_at(audience: &Audience, story: &Story, stage: Stage, intents_on: &[String]) -> bool {
    if stage.is_negative() && relevant(audience, story, intents_on) {
        return true;
    }
    match audience.effective() {
        Sensitivity::Important => story.tier <= 1,
        Sensitivity::Balanced => story.tier <= 2,
        Sensitivity::All => true,
    }
}

/// Whether `story` is about the person's position (non-zero quantity) or one of the
/// active intents' instruments.
#[must_use]
pub fn relevant(audience: &Audience, story: &Story, intents_on: &[String]) -> bool {
    audience
        .positions
        .get(&story.instrument)
        .is_some_and(|q| *q != 0.0)
        || intents_on.iter().any(|s| s == &story.instrument)
}
