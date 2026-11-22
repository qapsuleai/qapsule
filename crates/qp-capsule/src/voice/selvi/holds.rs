//! Selvi's detail (S3): does it still hold, when the outcome is due, or how it closed.

use super::*;

impl<'a> Cx<'a> {
    /// S3: does it still hold, and when is the outcome due — or how it closed. The
    /// full form first, then the compact one.
    pub(super) fn d_state(&self) -> Vec<Bi> {
        match self.parts.validity {
            Validity::Closed => vec![self.d_closed()],
            Validity::NoLonger => vec![self.d_no_longer()],
            Validity::Valid => {
                let mut v = vec![self.d_holds()];
                v.extend(self.d_holds_compact());
                v
            }
        }
    }

    /// The holding condition alone (no "when is the result due").
    fn d_holds_compact(&self) -> Option<Bi> {
        let side_up = up(self.dir);
        let (ts, es) = if side_up {
            ("üstünde", "above")
        } else {
            ("altında", "below")
        };
        Some(match self.det {
            "breakout.range" => {
                let l = self.num(self.lead, "level", Cat::Price)?;
                bi(
                    format!("Kapanış {} {ts} kaldıkça geçerli.", l.w().gen()),
                    format!("It holds while the close stays {es} {}.", l.en),
                )
            }
            "extreme.rsi" => {
                let l = self.rsi_line()?;
                bi(
                    format!("RSI {} {ts} kaldıkça geçerli.", l.w().gen()),
                    format!("It holds while RSI stays {es} {}.", l.en),
                )
            }
            "continuation.ma_stack" => {
                let a = self.span(MaStack::FAST).adj();
                bi(
                    format!("Kapanış {} ortalamanın {ts} kaldıkça geçerli.", a.tr),
                    format!("It holds while the close stays {es} the {} average.", a.en),
                )
            }
            "squeeze.bandwidth" => bi(
                "Bant yeniden açılana kadar geçerli.",
                "It holds until the band widens again.",
            ),
            _ => return None,
        })
    }

    fn due(&self) -> Bi {
        if self.red {
            return bi(
                "sonuç önümüzdeki günlerde belli olur",
                "the result is due in the coming days",
            );
        }
        let r = self.span32(self.remaining()).trading();
        bi(
            format!("sonuç en geç {} içinde belli olur", r.tr),
            format!("the result is due within {}", r.en),
        )
    }

    fn d_holds(&self) -> Bi {
        let due = self.due();
        let side_up = up(self.dir);
        let plain = || {
            bi(
                format!("{}.", capitalize(&due.tr)),
                format!("{}.", cap_en(&due.en)),
            )
        };
        match self.det {
            "breakout.range" => {
                let Some(l) = self.num(self.lead, "level", Cat::Price) else {
                    return plain();
                };
                let (ts, es) = if side_up {
                    ("üstünde", "above")
                } else {
                    ("altında", "below")
                };
                bi(
                    format!("Kapanış {} {ts} kaldıkça geçerli, {}.", l.w().gen(), due.tr),
                    format!(
                        "It holds while the close stays {es} {}, and {}.",
                        l.en, due.en
                    ),
                )
            }
            "extreme.rsi" => {
                let Some(l) = self.rsi_line() else {
                    return plain();
                };
                let (ts, es) = if side_up {
                    ("üstünde", "above")
                } else {
                    ("altında", "below")
                };
                let v0 = self.num(self.lead, "value", Cat::Osc).map(|v| v.tr);
                match self
                    .current(Cat::Osc)
                    .filter(|c| Some(&c.tr) != v0.as_ref())
                {
                    Some(c) => bi(
                        format!(
                            "RSI {} {ts} kaldıkça geçerli, şu an {} ve {}.",
                            l.w().gen(),
                            c.tr,
                            due.tr
                        ),
                        format!(
                            "It holds while RSI stays {es} {}, now at {}, and {}.",
                            l.en, c.en, due.en
                        ),
                    ),
                    None => bi(
                        format!("RSI {} {ts} kaldıkça geçerli, {}.", l.w().gen(), due.tr),
                        format!("It holds while RSI stays {es} {}, and {}.", l.en, due.en),
                    ),
                }
            }
            "continuation.ma_stack" => {
                let a = self.span(MaStack::FAST).adj();
                let (ts, es) = if side_up {
                    ("üstünde", "above")
                } else {
                    ("altında", "below")
                };
                match self.current(Cat::Price) {
                    Some(c) => bi(
                        format!("Kapanış {} ortalamanın {ts} kaldıkça geçerli, son kapanış {} ve {}.", a.tr, c.tr, due.tr),
                        format!("It holds while the close stays {es} the {} average, last at {}, and {}.", a.en, c.en, due.en),
                    ),
                    None => bi(
                        format!("Kapanış {} ortalamanın {ts} kaldıkça geçerli, {}.", a.tr, due.tr),
                        format!("It holds while the close stays {es} the {} average, and {}.", a.en, due.en),
                    ),
                }
            }
            "squeeze.bandwidth" => bi(
                format!("Bant yeniden açılana kadar geçerli, {}.", due.tr),
                format!("It holds until the band widens again, and {}.", due.en),
            ),
            _ => plain(),
        }
    }

    fn d_no_longer(&self) -> Bi {
        let side_up = up(self.dir);
        if self.latest_is_exit() {
            if let Some(c) = self.clause(self.latest, false) {
                let t = self.ago(self.latest.t);
                return bi(
                    format!("{} {}.", t.cap().tr, c.tr),
                    format!("{} {}.", cap_en(&c.en), t.en),
                );
            }
        }
        let (ts, es) = if side_up {
            ("altına", "below")
        } else {
            ("üstüne", "above")
        };
        match self.det {
            "breakout.range" => {
                let l = self.num(self.lead, "level", Cat::Price);
                match (l, self.current(Cat::Price)) {
                    (Some(l), Some(c)) => bi(
                        format!("Son kapanış {} ile {} {ts} döndü.", c.tr, l.w().gen()),
                        format!("The latest close, {}, is back {es} {}.", c.en, l.en),
                    ),
                    (Some(l), None) => bi(
                        format!("Son kapanış {} {ts} döndü.", l.w().gen()),
                        format!("The latest close is back {es} {}.", l.en),
                    ),
                    _ => bi(
                        "Son kapanış çizginin öbür yanına döndü.",
                        "The latest close is back across the line.",
                    ),
                }
            }
            "extreme.rsi" => {
                let l = self.rsi_line();
                match (l, self.current(Cat::Osc)) {
                    (Some(l), Some(c)) => bi(
                        format!("RSI şimdi {} ile {} {ts} döndü.", c.tr, l.w().gen()),
                        format!("RSI is now back {es} {}, at {}.", l.en, c.en),
                    ),
                    (Some(l), None) => bi(
                        format!("RSI {} {ts} döndü.", l.w().gen()),
                        format!("RSI is back {es} {}.", l.en),
                    ),
                    _ => bi(
                        "RSI çizginin öbür yanına döndü.",
                        "RSI is back across its line.",
                    ),
                }
            }
            "continuation.ma_stack" => {
                let a = self.span(MaStack::FAST).adj();
                match self.current(Cat::Price) {
                    Some(c) => bi(
                        format!("Son kapanış {} ile {} ortalamanın {ts} geçti.", c.tr, a.tr),
                        format!(
                            "The latest close, {}, crossed back {es} the {} average.",
                            c.en, a.en
                        ),
                    ),
                    None => bi(
                        format!("Son kapanış {} ortalamanın {ts} geçti.", a.tr),
                        format!("The latest close crossed back {es} the {} average.", a.en),
                    ),
                }
            }
            "squeeze.bandwidth" => bi("Bant yeniden genişledi.", "The band widened again."),
            _ => bi(
                "Dayandığı koşul bozuldu.",
                "The condition it rests on broke.",
            ),
        }
    }

    fn d_closed(&self) -> Bi {
        let mv = self.moved();
        let mc = mv.cap();
        let neutral = self.dir == Direction::Neutral;
        let (tr_past, en_past) = match self.dir {
            Direction::Up => ("yükseldi", "rose"),
            Direction::Down => ("düştü", "fell"),
            Direction::Neutral => ("bir yöne yeterince açıldı", "moved far enough one way"),
        };
        let h = self.span32(self.story.horizon_bars).trading();
        let (h_tr, h_en) = if self.red {
            ("beklenen süre".to_owned(), "the set time".to_owned())
        } else {
            (h.tr.clone(), h.en.clone())
        };
        match self.parts.ending {
            Some(Ending::Realized) if neutral => bi(
                format!("{} fiyat {tr_past} ve gerçekleşti.", mc.tr),
                format!("The price {en_past} {}, so it played out.", mv.en),
            ),
            Some(Ending::Realized) => bi(
                format!("{} fiyat yeterince {tr_past} ve gerçekleşti.", mc.tr),
                format!(
                    "The price {en_past} far enough {}, so it played out.",
                    mv.en
                ),
            ),
            Some(Ending::Failed) if neutral => bi(
                format!(
                    "Fiyat {h_tr} içinde yeterince hareket etmedi, {} tutmadı.",
                    mv.tr
                ),
                format!(
                    "The price did not move far enough within {h_en}, so it did not play out {}.",
                    mv.en
                ),
            ),
            Some(Ending::Failed) => bi(
                format!("{} fiyat ters yöne yeterince gitti ve tutmadı.", mc.tr),
                format!(
                    "The price went far enough the other way {}, so it did not play out.",
                    mv.en
                ),
            ),
            Some(Ending::Expired) => bi(
                format!(
                    "{} içinde yeterli hareket olmadı, {} süresi doldu.",
                    capitalize(&h_tr),
                    mv.tr
                ),
                format!(
                    "No large enough move came within {h_en}, so it ran out of time {}.",
                    mv.en
                ),
            ),
            Some(Ending::Retracted) => bi(
                format!(
                    "Dayandığı veri {} düzeltildi, bu yüzden geri çekildi ve sayılmadı.",
                    mv.tr
                ),
                format!(
                    "The data behind it was corrected {}, so it was withdrawn and not counted.",
                    mv.en
                ),
            ),
            None => bi(
                format!("{} kapandı.", mc.tr),
                format!("It closed {}.", mv.en),
            ),
        }
    }
}
