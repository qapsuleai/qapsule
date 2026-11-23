//! Selvi's detail (S2): the latest fact that joined, said as a clause about a fact.

use super::*;

impl<'a> Cx<'a> {
    /// S2: the latest fact that joined, if it is not the lead (and not the exit the state
    /// sentence already tells).
    pub(super) fn d_latest(&self) -> Option<Bi> {
        if !self.latest_is_new()
            || (self.parts.validity == Validity::NoLonger && self.latest_is_exit())
        {
            return None;
        }
        if self.red {
            return None;
        }
        let m = self.latest.meaning.as_ref()?;
        let against = opposite(self.dir, m.direction);
        let also = !against && m.edge != Edge::Exit;
        let c = self.clause(self.latest, also)?;
        let t = self.ago(self.latest.t);
        let tc = t.cap();
        Some(if against {
            bi(
                format!("{} bunun tersine {}.", tc.tr, c.tr),
                format!("Against it, {} {}.", c.en, t.en),
            )
        } else {
            bi(
                format!("{} {}.", tc.tr, c.tr),
                format!("{} {}.", cap_en(&c.en), t.en),
            )
        })
    }

    #[allow(clippy::too_many_lines)]
    /// A clause about fact `f` (no time, no full stop). `also`: it adds to the story.
    pub(super) fn clause(&self, f: &Fact, also: bool) -> Option<Bi> {
        let m = f.meaning.as_ref()?;
        let u = up(m.direction);
        let exit = m.edge == Edge::Exit;
        // A licensed fact's figures are shown only when the reader holds the same
        // entitlement as for the lead (entitled to the lead's licensed figures).
        let licensed = m.license.class == LicenseClass::Licensed;
        let lead_lic = self.lead.meaning.as_ref().map(|lm| &lm.license);
        let figures = !self.red
            && (!licensed
                || lead_lic.is_some_and(|ll| {
                    ll.class == LicenseClass::Licensed && ll.entitlement == m.license.entitlement
                }));
        let da = |w: &str| -> String {
            if !also {
                return String::new();
            }
            // "RSI" is read "ar-es-ay" here: "RSI da".
            let last = if w == "rsi" {
                'a'
            } else {
                w.chars()
                    .rev()
                    .find(|c| "aıoueiöü".contains(*c))
                    .unwrap_or('e')
            };
            let a = qp_capsule::realize::morph::a2(last);
            format!(" d{a}")
        };
        let ae = if also { " also" } else { "" };
        Some(match m.detector.as_str() {
            "breakout.range" => {
                let s = self.span(RangeBreakout::WINDOW);
                let l = self.num(f, "level", Cat::Price)?;
                if u {
                    bi(
                        format!(
                            "fiyat{} önceki {} en yükseğini ({}) aştı",
                            da("fiyat"),
                            s.short_gen().tr,
                            l.tr
                        ),
                        format!(
                            "the price{ae} broke its prior {} high ({})",
                            s.adj().en,
                            l.en
                        ),
                    )
                } else {
                    bi(
                        format!(
                            "fiyat{} önceki {} en düşüğünün ({}) altına indi",
                            da("fiyat"),
                            s.short_gen().tr,
                            l.tr
                        ),
                        format!(
                            "the price{ae} broke below its prior {} low ({})",
                            s.adj().en,
                            l.en
                        ),
                    )
                }
            }
            "extreme.rsi" => {
                let l = Num::new(
                    self.ev(f, "level").unwrap_or(if u {
                        RsiExtreme::UPPER
                    } else {
                        RsiExtreme::LOWER
                    }),
                    Cat::Osc,
                );
                match (exit, u) {
                    (false, true) => bi(
                        format!("RSI{} {} üstüne çıktı", da("rsi"), l.w().gen()),
                        format!("RSI{ae} went above {}", l.en),
                    ),
                    (false, false) => bi(
                        format!("RSI{} {} altına indi", da("rsi"), l.w().gen()),
                        format!("RSI{ae} went below {}", l.en),
                    ),
                    (true, true) => bi(
                        format!("RSI {} altına döndü", l.w().gen()),
                        format!("RSI went back below {}", l.en),
                    ),
                    (true, false) => bi(
                        format!("RSI {} üstüne döndü", l.w().gen()),
                        format!("RSI went back above {}", l.en),
                    ),
                }
            }
            "continuation.ma_stack" => {
                let a = self.span(MaStack::FAST).adj();
                match (exit, u) {
                    (false, _) => {
                        let s = self.num(f, "sma20", Cat::Price)?;
                        let (ts, es) = if u {
                            ("üstüne", "above")
                        } else {
                            ("altına", "below")
                        };
                        bi(
                            format!(
                                "fiyat{} {} ortalamanın ({}) {ts} yerleşti",
                                da("fiyat"),
                                a.tr,
                                s.tr
                            ),
                            format!("the price{ae} settled {es} its {} average ({})", a.en, s.en),
                        )
                    }
                    (true, true) => bi(
                        format!("fiyat {} ortalamanın altına indi", a.tr),
                        format!("the price fell back below its {} average", a.en),
                    ),
                    (true, false) => bi(
                        format!("fiyat {} ortalamanın üstüne çıktı", a.tr),
                        format!("the price rose back above its {} average", a.en),
                    ),
                }
            }
            "reversal.rsi_divergence" => {
                let again = if also { " yine" } else { "" };
                let again_en = if also { " again" } else { "" };
                bi(
                    format!("fiyatla RSI{again} ters yönlere gitti"),
                    format!("price and RSI moved apart{again_en}"),
                )
            }
            "squeeze.bandwidth" => {
                let u120 = self.span(BandSqueeze::LOOKBACK).short_gen();
                if exit {
                    let lo = self.num(f, "lower", Cat::Price)?;
                    let hi = self.num(f, "upper", Cat::Price)?;
                    bi(
                        format!("bant yeniden genişledi ({}–{})", lo.tr, hi.tr),
                        format!("the band widened again ({}–{})", lo.en, hi.en),
                    )
                } else {
                    bi(
                        format!("bant{} son {} en darına indi", da("bant"), u120.tr),
                        format!("the band{ae} hit its narrowest in {}", u120.en),
                    )
                }
            }
            "flow.volume_surge" => {
                let r = self.num(f, "ratio", Cat::Osc)?;
                let a = self.span(VolumeSurge::WINDOW).adj();
                bi(
                    format!(
                        "hacim{} {} ortalamanın {} katına çıktı",
                        da("hacim"),
                        a.tr,
                        r.tr
                    ),
                    format!("volume{ae} reached {} times its {} average", r.en, a.en),
                )
            }
            "flow.institutional" if !figures => {
                let (tv, ev) = if u {
                    ("aldı", "bought")
                } else {
                    ("sattı", "sold")
                };
                bi(
                    format!("kurumlar{} her zamankinden çok net {tv}", da("kurumlar")),
                    format!("institutions{ae} net-{ev} far more than usual"),
                )
            }
            "flow.institutional" => {
                let (tv, ev) = if u {
                    ("aldı", "bought")
                } else {
                    ("sattı", "sold")
                };
                let a = self.span(InstitutionalFlow::WINDOW).adj();
                let net = self.ev(f, "net")?;
                let avg = self.ev(f, "avg5").filter(|v| *v > 0.0)?;
                let r = Num::new(net.abs() / avg, Cat::Osc);
                bi(
                    format!(
                        "kurumlar{} {} ortalamanın {} katı net {tv}",
                        da("kurumlar"),
                        a.tr,
                        r.tr
                    ),
                    format!(
                        "institutions{ae} net-{ev} {} times their {} average",
                        r.en, a.en
                    ),
                )
            }
            _ => return None,
        })
    }
}
