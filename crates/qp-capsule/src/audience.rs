//! Sensitivity filter (`docs/CAPSULE.md` §9.8, K31).

use qp_schema::capsule::{Audience, Sensitivity, Story};

/// Whether `story` reaches `audience`.
///
/// - A story of an instrument the person holds a position in (non-zero quantity) or has
///   an active intent on (`intents_on`) always passes, whatever the level and stage.
/// - Otherwise the tier filter of the effective sensitivity: `important` = tier 1,
///   `balanced` = tiers 1–2, `all` = every tier.
///
/// Pure; never changes the audience (the system does not change the level by itself).
/// Watch-list selection belongs to the subscription layer, not to this filter.
#[must_use]
pub fn admits(audience: &Audience, story: &Story, intents_on: &[String]) -> bool {
    let held = audience
        .positions
        .get(&story.instrument)
        .is_some_and(|q| *q != 0.0);
    if held || intents_on.iter().any(|s| s == &story.instrument) {
        return true;
    }
    match audience.effective() {
        Sensitivity::Important => story.tier <= 1,
        Sensitivity::Balanced => story.tier <= 2,
        Sensitivity::All => true,
    }
}
