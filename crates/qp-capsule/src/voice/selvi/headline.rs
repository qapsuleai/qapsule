//! Selvi's headline: what changed, per kind, with when and whether it still holds.

use super::*;

impl<'a> Cx<'a> {
    // -----------------------------------------------------------------------
    // Headline
    // -----------------------------------------------------------------------

    fn validity_part(&self) -> Bi {
        match (self.parts.validity, self.parts.ending) {
            (Validity::Valid, _) => bi("Hâlâ geçerli.", "Still holds."),
            (Validity::NoLonger, _) => bi("Artık geçerli değil.", "No longer holds."),
            (Validity::Closed, Some(Ending::Realized)) => {
                bi("Kapandı: gerçekleşti.", "Closed: played out.")
            }
            (Validity::Closed, Some(Ending::Failed)) => {
                bi("Kapandı: tutmadı.", "Closed: did not play out.")
            }
            (Validity::Closed, Some(Ending::Expired)) => {
                bi("Kapandı: süresi doldu.", "Closed: ran out of time.")
            }
            (Validity::Closed, Some(Ending::Retracted)) => {
                bi("Kapandı: geri çekildi.", "Closed: withdrawn.")
            }
            (Validity::Closed, None) => bi("Kapandı.", "Closed."),
        }
    }

    /// The short form of the validity part: a closed capsule's ending alone ("Tutmadı.").
    fn validity_short(&self) -> Bi {
        match (self.parts.validity, self.parts.ending) {
            (Validity::Closed, Some(Ending::Realized)) => bi("Gerçekleşti.", "Played out."),
            (Validity::Closed, Some(Ending::Failed)) => bi("Tutmadı.", "Did not play out."),
            (Validity::Closed, Some(Ending::Expired)) => bi("Süresi doldu.", "Ran out of time."),
            (Validity::Closed, Some(Ending::Retracted)) => bi("Geri çekildi.", "Withdrawn."),
            _ => self.validity_part(),
        }
    }

    pub(super) fn headline(&self) -> Bi {
        let (v, vs) = (self.validity_part(), self.validity_short());
        let mains = self.headline_mains();
        let mut cands: Vec<Bi> = Vec::new();
        for m in &mains {
            cands.push(Bi::join(&[m.clone(), v.clone()]));
            if vs != v {
                cands.push(Bi::join(&[m.clone(), vs.clone()]));
            }
        }
        first_fit(cands, |b| b.fits(self.room))
    }

    /// Main sentences, most preferred first (each ends with a full stop).
    fn headline_mains(&self) -> Vec<Bi> {
        let i = self.inst.as_str();
        let w = self.when();
        let mut out: Vec<Option<Bi>> = match self.det {
            "breakout.range" => self.hl_breakout(i, &w),
            "extreme.rsi" => self.hl_extreme(i, &w),
            "continuation.ma_stack" => self.hl_ma(i, &w),
            "reversal.rsi_divergence" => self.hl_divergence(i, &w),
            "squeeze.bandwidth" => self.hl_squeeze(i, &w),
            "flow.volume_surge" => self.hl_volume(i, &w),
            "flow.institutional" => self.hl_institutional(i, &w),
            _ => Vec::new(),
        };
        out.push(Some(self.hl_generic(i, &w)));
        out.into_iter()
            .flatten()
            .filter(|b| !self.red || !has_digit(b, &self.inst))
            .collect()
    }

    fn hl_generic(&self, i: &str, w: &Bi) -> Bi {
        let k = self.story.kind;
        bi(
            format!(
                "{i} {}: {}.",
                w.tr,
                selvi_glossary::plain_event(k, self.dir, Lang::Tr)
            ),
            format!(
                "{i}: {}, {}.",
                selvi_glossary::plain_event(k, self.dir, Lang::En),
                w.en
            ),
        )
    }

    fn hl_breakout(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let span = self.span(RangeBreakout::WINDOW);
        let (u, a) = (span.short_gen(), span.adj());
        let l = self.num(self.lead, "level", Cat::Price);
        if up(self.dir) {
            vec![
                l.as_ref().map(|l| {
                    bi(
                        format!(
                            "{i} {} son {} en yükseği olan {} aştı.",
                            w.tr,
                            u.tr,
                            l.w().acc()
                        ),
                        format!(
                            "{i} broke above its prior {} high of {} {}.",
                            a.en, l.en, w.en
                        ),
                    )
                }),
                Some(bi(
                    format!("{i} {} son {} en yükseğini aştı.", w.tr, u.tr),
                    format!("{i} broke above its prior {} high {}.", a.en, w.en),
                )),
                Some(bi(
                    format!("{i} {} son zirvesini aştı.", w.tr),
                    format!("{i} broke above its recent high {}.", w.en),
                )),
            ]
        } else {
            vec![
                l.as_ref().map(|l| {
                    bi(
                        format!(
                            "{i} {} son {} en düşüğü olan {} altına indi.",
                            w.tr,
                            u.tr,
                            l.w().gen()
                        ),
                        format!(
                            "{i} broke below its prior {} low of {} {}.",
                            a.en, l.en, w.en
                        ),
                    )
                }),
                Some(bi(
                    format!("{i} {} son {} en düşüğünün altına indi.", w.tr, u.tr),
                    format!("{i} broke below its prior {} low {}.", a.en, w.en),
                )),
                Some(bi(
                    format!("{i} {} son en düşüğünün altına indi.", w.tr),
                    format!("{i} broke below its recent low {}.", w.en),
                )),
            ]
        }
    }

    pub(super) fn rsi_line(&self) -> Option<Num> {
        let fallback = if up(self.dir) {
            RsiExtreme::UPPER
        } else {
            RsiExtreme::LOWER
        };
        if self.red {
            return None;
        }
        Some(Num::new(
            self.ev(self.lead, "level").unwrap_or(fallback),
            Cat::Osc,
        ))
    }

    fn hl_extreme(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let l = self.rsi_line();
        let il = self.inst_loc();
        let Some(l) = l else {
            return Vec::new();
        };
        if up(self.dir) {
            vec![
                Some(bi(
                    format!(
                        "{i} kısa sürede çok yükseldi: RSI {} {} üstüne çıktı.",
                        w.tr,
                        l.w().gen()
                    ),
                    format!("{i} rose fast: its RSI went above {} {}.", l.en, w.en),
                )),
                Some(bi(
                    format!("{il} RSI {} {} üstüne çıktı.", w.tr, l.w().gen()),
                    format!("{i}'s RSI went above {} {}.", l.en, w.en),
                )),
                Some(bi(
                    format!("{il} RSI {} {} aştı.", w.tr, l.w().acc()),
                    format!("{i}'s RSI topped {} {}.", l.en, w.en),
                )),
            ]
        } else {
            vec![
                Some(bi(
                    format!(
                        "{i} kısa sürede çok düştü: RSI {} {} altına indi.",
                        w.tr,
                        l.w().gen()
                    ),
                    format!("{i} fell fast: its RSI went below {} {}.", l.en, w.en),
                )),
                Some(bi(
                    format!("{il} RSI {} {} altına indi.", w.tr, l.w().gen()),
                    format!("{i}'s RSI went below {} {}.", l.en, w.en),
                )),
            ]
        }
    }

    fn hl_ma(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        // (R5/m17-int, S1) The event first and plain: the price is in a fall / kept falling
        // ("düşüşte" / "kept falling"), the average after it as where the price stands —
        // never "altına yerleşti", which readers took for a threshold crossing.
        let a = self.span(MaStack::FAST).adj();
        let s20 = self.num(self.lead, "sma20", Cat::Price);
        let (tr_state, tr_side, en_verb) = if up(self.dir) {
            ("yükselişte", "üstünde", "kept rising above")
        } else {
            ("düşüşte", "altında", "kept falling below")
        };
        vec![
            s20.as_ref().map(|s| {
                bi(
                    format!(
                        "{i} {} {tr_state}: {} ortalama ({}) {tr_side}.",
                        w.tr, a.tr, s.tr
                    ),
                    format!("{i} {en_verb} {} average ({}) {}.", a.en, s.en, w.en),
                )
            }),
            Some(bi(
                format!("{i} {} {tr_state}: {} ortalama {tr_side}.", w.tr, a.tr),
                format!("{i} {en_verb} {} average {}.", a.en, w.en),
            )),
            Some(bi(
                format!("{i} {} {tr_state}, ortalamanın {tr_side}.", w.tr),
                format!("{i} {en_verb} its average {}.", w.en),
            )),
        ]
    }

    fn hl_divergence(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let p2 = self.num(self.lead, "p2_price", Cat::Price);
        if up(self.dir) {
            vec![
                p2.as_ref().map(|p| {
                    bi(
                        format!(
                            "{i} {} {} ile daha derin bir dip yaptı ama RSI yukarıda kaldı.",
                            w.tr, p.tr
                        ),
                        format!(
                            "{i} made a lower low at {} {}, but its RSI stayed higher.",
                            p.en, w.en
                        ),
                    )
                }),
                p2.as_ref().map(|p| {
                    bi(
                        format!(
                            "{i} {} {} ile daha derin dip yaptı, RSI yukarıda kaldı.",
                            w.tr, p.tr
                        ),
                        format!(
                            "{i} made a lower low at {} {}, but RSI stayed higher.",
                            p.en, w.en
                        ),
                    )
                }),
                Some(bi(
                    format!(
                        "{i} {} daha derin bir dip yaptı ama RSI yukarıda kaldı.",
                        w.tr
                    ),
                    format!("{i} made a lower low {}, but its RSI stayed higher.", w.en),
                )),
                Some(bi(
                    format!("{i} {} dip yaptı, RSI yukarıda kaldı.", w.tr),
                    format!("{i} made a low {}, its RSI stayed higher.", w.en),
                )),
            ]
        } else {
            vec![
                p2.as_ref().map(|p| {
                    bi(
                        format!(
                            "{i} {} {} ile daha yüksek bir tepe yaptı ama RSI aşağıda kaldı.",
                            w.tr, p.tr
                        ),
                        format!(
                            "{i} made a higher high at {} {}, but its RSI stayed lower.",
                            p.en, w.en
                        ),
                    )
                }),
                p2.as_ref().map(|p| {
                    bi(
                        format!(
                            "{i} {} {} ile daha yüksek tepe yaptı, RSI aşağıda kaldı.",
                            w.tr, p.tr
                        ),
                        format!(
                            "{i} made a higher high at {} {}, but RSI stayed lower.",
                            p.en, w.en
                        ),
                    )
                }),
                Some(bi(
                    format!(
                        "{i} {} daha yüksek bir tepe yaptı ama RSI aşağıda kaldı.",
                        w.tr
                    ),
                    format!("{i} made a higher high {}, but its RSI stayed lower.", w.en),
                )),
                Some(bi(
                    format!("{i} {} tepe yaptı, RSI aşağıda kaldı.", w.tr),
                    format!("{i} made a high {}, its RSI stayed lower.", w.en),
                )),
            ]
        }
    }

    fn hl_squeeze(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let u = self.span(BandSqueeze::LOOKBACK).short_gen();
        let lo = self.num(self.lead, "lower", Cat::Price);
        let hi = self.num(self.lead, "upper", Cat::Price);
        vec![
            lo.as_ref().zip(hi.as_ref()).map(|(lo, hi)| {
                bi(
                    format!(
                        "{i} {} {} en dar aralığına sıkıştı: bant {}–{}.",
                        w.tr, u.tr, lo.tr, hi.tr
                    ),
                    format!(
                        "{i} hit its tightest band in {} {}: {}–{}.",
                        u.en, w.en, lo.en, hi.en
                    ),
                )
            }),
            Some(bi(
                format!("{i} {} {} en dar bant aralığına sıkıştı.", w.tr, u.tr),
                format!("{i} hit its tightest band in {} {}.", u.en, w.en),
            )),
            Some(bi(
                format!("{i} {} en dar bant aralığına sıkıştı.", w.tr),
                format!("{i} hit its tightest band {}.", w.en),
            )),
        ]
    }

    fn hl_volume(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let a = self.span(VolumeSurge::WINDOW).adj();
        let r = self.num(self.lead, "ratio", Cat::Osc);
        let (tr_v, en_v) = match self.dir {
            Direction::Up => ("yükseldi", "rose"),
            Direction::Down => ("düştü", "fell"),
            Direction::Neutral => ("yatay kapandı", "closed flat"),
        };
        vec![
            r.as_ref().map(|r| {
                bi(
                    format!(
                        "{i} {} {} ortalamanın {} katı hacimle {tr_v}.",
                        w.tr, a.tr, r.tr
                    ),
                    format!(
                        "{i} {en_v} on {} times its {} average volume {}.",
                        r.en, a.en, w.en
                    ),
                )
            }),
            r.as_ref().map(|r| {
                bi(
                    format!("{i} {} ortalamanın {} katı hacimle {tr_v}.", w.tr, r.tr),
                    format!("{i} {en_v} on {} times its average volume {}.", r.en, w.en),
                )
            }),
            Some(bi(
                format!("{i} {} çok yüksek hacimle {tr_v}.", w.tr),
                format!("{i} {en_v} on very high volume {}.", w.en),
            )),
            Some(bi(
                format!("{i} {} yüksek hacimle {tr_v}.", w.tr),
                format!("{i} {en_v} on high volume {}.", w.en),
            )),
        ]
    }

    fn hl_institutional(&self, i: &str, w: &Bi) -> Vec<Option<Bi>> {
        let a = self.span(InstitutionalFlow::WINDOW).adj();
        let il = self.inst_loc();
        let (tr_v, en_v) = if up(self.dir) {
            ("aldı", "bought")
        } else {
            ("sattı", "sold")
        };
        let m = (!self.red).then(|| Num::new(InstitutionalFlow::MULT, Cat::Osc));
        vec![
            m.as_ref().map(|m| {
                bi(
                    format!(
                        "Kurumlar {} {il} {} ortalamanın {} katından fazla net {tr_v}.",
                        w.tr, a.tr, m.tr
                    ),
                    format!(
                        "Institutions net-{en_v} {i} at over {}× their {} average {}.",
                        m.en, a.en, w.en
                    ),
                )
            }),
            m.as_ref().map(|m| {
                bi(
                    format!(
                        "Kurumlar {} {il} ortalamanın {} katından fazla net {tr_v}.",
                        w.tr, m.tr
                    ),
                    format!(
                        "Institutions net-{en_v} {i} at over {}× their average {}.",
                        m.en, w.en
                    ),
                )
            }),
            Some(bi(
                format!("Kurumlar {} {il} her zamankinden çok net {tr_v}.", w.tr),
                format!("Institutions net-{en_v} {i} far more than usual {}.", w.en),
            )),
            Some(bi(
                format!("Kurumlar {} {il} yoğun net {tr_v}.", w.tr),
                format!("Institutions net-{en_v} {i} heavily {}.", w.en),
            )),
        ]
    }
}
