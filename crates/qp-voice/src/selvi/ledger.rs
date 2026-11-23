//! Selvi's track-record (ledger) line.

use super::*;

impl<'a> Cx<'a> {
    // -----------------------------------------------------------------------
    // Ledger
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_lines)]
    pub(super) fn ledger(&self) -> Bi {
        let l = self.story.ledger_at_birth;
        let (tr_past, en_past, tr_neg, tr_if, en_if) = match self.dir {
            Direction::Up => ("yükseldi", "rose", "yükselmedi", "yükselirse", "rises"),
            Direction::Down => ("düştü", "fell", "düşmedi", "düşerse", "falls"),
            Direction::Neutral => (
                "bir yöne kırıldı",
                "broke out",
                "bir yöne kırılmadı",
                "bir yöne kırılırsa",
                "breaks out",
            ),
        };
        let ok = |b: &Bi| b.fits(MAX_LEDGER);
        if self.red {
            let b = if l.n == 0 {
                bi(
                    format!("Henüz benzeri yok, fiyat {tr_if} tutmuş sayılacak."),
                    format!("No past cases yet, it counts if the price {en_if}."),
                )
            } else if l.n < LEDGER_MIN_N {
                bi(
                    "Az örnek: geçmişte yalnız birkaç benzeri var.",
                    "Few past cases: only a handful so far.",
                )
            } else {
                let (tr_q, en_q) = if 10 * l.realized >= 6 * l.n {
                    ("çoğunda", "most")
                } else if 10 * l.realized >= 4 * l.n {
                    ("yaklaşık yarısında", "about half of the")
                } else {
                    ("azında", "few")
                };
                bi(
                    format!("Geçmişteki benzerlerinin {tr_q} fiyat sonradan {tr_past}."),
                    format!("In {en_q} past cases the price {en_past} afterwards."),
                )
            };
            return first_fit(vec![b, bi("Benzerleri var.", "Past cases exist.")], ok);
        }
        let h = self.span32(self.story.horizon_bars).trading();
        if l.n == 0 {
            return first_fit(
                vec![
                    bi(
                        format!(
                            "Henüz benzeri yok, fiyat {} içinde {tr_if} tutmuş sayılacak.",
                            h.tr
                        ),
                        format!(
                            "No past cases yet, it counts if the price {en_if} within {}.",
                            h.en
                        ),
                    ),
                    bi(
                        format!(
                            "Henüz benzeri yok, {} içinde {tr_if} tutmuş sayılacak.",
                            h.tr
                        ),
                        format!(
                            "No past cases yet, it counts if it {en_if} within {}.",
                            h.en
                        ),
                    ),
                    bi(
                        format!("Henüz benzeri yok, {} içinde {tr_if} tutacak.", h.tr),
                        format!("No past cases yet; counts if it {en_if} within {}.", h.en),
                    ),
                    bi("Henüz benzeri yok.", "No past cases yet."),
                ],
                ok,
            );
        }
        let n = Word::number(&l.n.to_string());
        let r = Word::number(&l.realized.to_string());
        let (tr_hit, tr_verb) = if l.realized == 0 {
            ("hiçbirinde (0)".to_owned(), tr_neg)
        } else {
            (r.poss3_loc(), tr_past)
        };
        let thin = l.n < LEDGER_MIN_N;
        let (tr_head, tr_head_short) = if thin {
            ("Az örnek: ", "Az örnek: ")
        } else {
            ("Geçmişte ", "")
        };
        let en_full = if thin {
            format!(
                "Few past cases: the price {en_past} within {} in {} of {}.",
                h.en, l.realized, l.n
            )
        } else {
            format!(
                "The price {en_past} within {} in {} of {} past cases.",
                h.en, l.realized, l.n
            )
        };
        let en_short = if thin {
            format!(
                "Few past cases: it {en_past} within {} in {} of {}.",
                h.en, l.realized, l.n
            )
        } else {
            format!(
                "It {en_past} within {} in {} of {} past cases.",
                h.en, l.realized, l.n
            )
        };
        first_fit(
            vec![
                bi(
                    format!(
                        "{tr_head}{} benzerinin {tr_hit} fiyat {} içinde {tr_verb}.",
                        n.written, h.tr
                    ),
                    en_full,
                ),
                bi(
                    format!(
                        "{tr_head}{} benzerinin {tr_hit} {} içinde {tr_verb}.",
                        n.written, h.tr
                    ),
                    en_short.clone(),
                ),
                bi(
                    format!(
                        "{tr_head_short}{} benzerinin {tr_hit} {} içinde {tr_verb}.",
                        n.written, h.tr
                    ),
                    en_short.clone(),
                ),
                bi(
                    format!(
                        "{tr_head_short}{} benzerinde {} içinde {} kez {tr_past}.",
                        n.written, h.tr, l.realized
                    ),
                    en_short,
                ),
            ],
            ok,
        )
    }
}
