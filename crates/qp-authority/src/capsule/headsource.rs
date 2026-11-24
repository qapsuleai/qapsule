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
