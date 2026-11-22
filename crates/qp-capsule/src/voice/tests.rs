//! Tests of the voice registry and the licence guard (moved out of `voice/mod.rs`).

use super::*;
use crate::Lang;

#[test]
fn registry_is_well_formed() {
    assert_eq!(VOICES[0], DEFAULT_VOICE);
    assert!(is_voice(PRESENTED_VOICE));
    assert_ne!(PRESENTED_VOICE, DEFAULT_VOICE);
    assert!(!is_voice("tunca"), "tunca was eliminated (M17)");
    assert!(!is_voice("r3"));
    assert_eq!(names(), ["r4", "marla", "selvi"]);
}

#[test]
fn unnumber_leaves_no_digit_but_the_label() {
    assert_eq!(
        unnumber(
            "SYN04 20 günlük ortalamanın altında; RSI 70'in üstü",
            "SYN04",
            Lang::Tr
        ),
        "SYN04 yirmi günlük ortalamanın altında; RSI yetmişin üstü"
    );
    assert_eq!(
        unnumber(
            "SYN04 kept falling below its 20-day average",
            "SYN04",
            Lang::En
        ),
        "SYN04 kept falling below its twenty-day average"
    );
    let t = unnumber("1234 ve 3,5", "", Lang::Tr);
    assert!(!t.bytes().any(|b| b.is_ascii_digit()), "{t}");
}
