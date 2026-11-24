//! (RF1 S2) The session as the document provider's headline source
//! ([`qp_ops::HeadlineSource`]): a reader's headline pieces and voice surfaces, over the
//! session's document and the reader's projection (inside [`Session::as_reader`]).

use super::*;

impl qp_ops::HeadlineSource for Session {
    fn headline_parts(&self, finding: &str, lang: Language) -> Option<qp_grammar::HeadlineParts> {
        let c = self.doc.capsules.iter().find(|c| c.finding == finding)?;
        self.reader_parts(
            &self.doc,
            c,
            self.proj.as_deref(),
            lang,
            &mut qp_capsule::sentence::StudyMemo::default(),
        )
    }

    fn voice(
        &self,
        finding: &str,
        lang: Language,
        voice: &str,
        clock: bool,
    ) -> Option<qp_schema::protocol::CapsuleVoice> {
        let c = self.doc.capsules.iter().find(|c| c.finding == finding)?;
        self.reader_voice(
            &self.doc,
            c,
            self.proj.as_deref(),
            lang,
            voice,
            clock,
            &mut qp_capsule::sentence::StudyMemo::default(),
        )
    }
}

impl Session {
    /// (RF1 S3.3) The headline pieces of the capsule of finding `finding` as `principal`
    /// reads it in `lang` (the headline source inside its `as_reader`): the headline
    /// pieces test (`qp-replayhost` `tests/headparts.rs`, over the demo replay).
    #[doc(hidden)]
    pub fn reader_parts_of(
        &mut self,
        principal: &str,
        lang: Language,
        finding: &str,
    ) -> Option<qp_grammar::HeadlineParts> {
        use qp_ops::HeadlineSource as _;
        self.as_reader(principal, lang, |s| s.headline_parts(finding, lang))
    }
}
