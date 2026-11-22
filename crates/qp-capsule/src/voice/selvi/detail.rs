//! Selvi's detail (S1): why — the trust line, the threshold and the per-kind first sentence.

use super::*;

impl<'a> Cx<'a> {
    // -----------------------------------------------------------------------
    // Detail
    // -----------------------------------------------------------------------

    pub(super) fn detail(&self) -> Bi {
        let s1s = self.d_threshold();
        let s2 = self.d_latest();
        let s3s = self.d_state();
        let s3 = s3s.first().cloned().unwrap_or_default();
        let st = self.d_trust();
        let mut cands: Vec<Bi> = Vec::new();
        let opt = |b: &Option<Bi>| b.clone().unwrap_or_default();
        // Most complete first. What changed since birth (S2) is kept before the long
        // forms: the line and the state fall back to their compact forms first; then
        // the trust note goes (never the redacted reader's), then S2.
        for with2 in [true, false] {
            for s1 in &s1s {
                for s3v in &s3s {
                    for with_t in [true, false] {
                        if self.red && !with_t {
                            continue;
                        }
                        let parts = [
                            s1.clone(),
                            if with2 { opt(&s2) } else { Bi::default() },
                            s3v.clone(),
                            if with_t { opt(&st) } else { Bi::default() },
                        ];
                        cands.push(Bi::join(&parts));
                    }
                }
            }
        }
        let ok = |b: &Bi| {
            b.fits(MAX_DETAIL)
                && sentence_count(&b.tr) <= MAX_DETAIL_SENTENCES
                && sentence_count(&b.en) <= MAX_DETAIL_SENTENCES
        };
        if let Some(b) = cands.iter().find(|b| ok(b)) {
            return b.clone();
        }
        let s3 = &s3;
        // Last resort: the threshold and the state, the threshold in its short form.
        first_fit(
            vec![
                Bi::join(&[self.d_threshold_short(), s3.clone()]),
                s3.clone(),
            ],
            ok,
        )
    }

    /// The trust note for the open capsule: the redacted reader always learns which
    /// entitlement opens the figures and who verified the claim; an entitled reader of a
    /// server fact learns that the server verified it.
    fn d_trust(&self) -> Option<Bi> {
        let server = self.story.trust == Trust::Server
            || self.lead.meaning.as_ref().is_some_and(|m| m.server);
        if self.red {
            let (tr_feed, en_feed) = self.feed_name();
            let verified_tr = if server {
                ", bunu sunucu doğruladı"
            } else {
                ""
            };
            let verified_en = if server {
                ", and the server verified it"
            } else {
                ""
            };
            return Some(bi(
                format!("Rakamlar {tr_feed} verisi yetkisiyle açılır{verified_tr}."),
                format!("The figures open with a {en_feed} data entitlement{verified_en}."),
            ));
        }
        server.then(|| bi("Bunu sunucu doğruladı.", "The server verified this."))
    }

    fn feed_name(&self) -> (&'static str, &'static str) {
        let ent = self
            .lead
            .meaning
            .as_ref()
            .and_then(|m| m.license.entitlement.as_deref())
            .unwrap_or("");
        if ent.starts_with("flow") {
            ("kurum akışı", "broker-flow")
        } else {
            ("lisanslı", "licensed")
        }
    }

    /// S1: the line, with its name, value and parameter — the full sentence, then its
    /// compact form (both carry the name, the value and the parameter).
    fn d_threshold(&self) -> Vec<Bi> {
        let w = self.when();
        let got = match self.det {
            "breakout.range" => self.d1_breakout(&w),
            "extreme.rsi" => self.d1_extreme(&w),
            "continuation.ma_stack" => self.d1_ma(&w),
            "reversal.rsi_divergence" => self.d1_divergence(),
            "squeeze.bandwidth" => self.d1_squeeze(&w),
            "flow.volume_surge" => self.d1_volume(&w),
            "flow.institutional" => self.d1_institutional(&w),
            _ => None,
        };
        let mut out: Vec<Bi> = got.into_iter().collect();
        out.extend(self.d1_compact());
        if out.is_empty() {
            out.push(self.d_threshold_short());
        }
        out
    }

    /// The compact form of S1 (numbers only; `None` for a redacted reader).
    fn d1_compact(&self) -> Option<Bi> {
        let f = self.lead;
        Some(match self.det {
            "breakout.range" => {
                let s = self.span(RangeBreakout::WINDOW);
                let l = self.num(f, "level", Cat::Price)?;
                let c = self.num(f, "close", Cat::Price)?;
                let (tr, en) = if up(self.dir) {
                    ("en yükseği", "high")
                } else {
                    ("en düşüğü", "low")
                };
                bi(
                    format!(
                        "Önceki {} {tr} {}, kapanış {}.",
                        s.trading_gen().tr,
                        l.tr,
                        c.tr
                    ),
                    format!("Prior {} {en} {}, close {}.", s.adj().en, l.en, c.en),
                )
            }
            "extreme.rsi" => {
                let v = self.num(f, "value", Cat::Osc)?;
                let l = self.rsi_line()?;
                bi(
                    format!("RSI {} oldu, çizgi {}.", v.tr, l.tr),
                    format!("RSI reached {}, the line is {}.", v.en, l.en),
                )
            }
            "continuation.ma_stack" => {
                let a = self.num(f, "sma20", Cat::Price)?;
                let b = self.num(f, "sma50", Cat::Price)?;
                let (af, asl) = (
                    self.span(MaStack::FAST).adj(),
                    self.span(MaStack::SLOW).adj(),
                );
                bi(
                    format!(
                        "{} ortalama {}, {} ortalama {} idi.",
                        capitalize(&af.tr),
                        a.tr,
                        asl.tr,
                        b.tr
                    ),
                    format!(
                        "The {} average was {} and the {} {}.",
                        af.en, a.en, asl.en, b.en
                    ),
                )
            }
            "reversal.rsi_divergence" => {
                let p1 = self.num(f, "p1_price", Cat::Price)?;
                let p2 = self.num(f, "p2_price", Cat::Price)?;
                let r1 = self.num(f, "p1_rsi", Cat::Osc)?;
                let r2 = self.num(f, "p2_rsi", Cat::Osc)?;
                let (tr, en) = if up(self.dir) {
                    ("dip", "low")
                } else {
                    ("tepe", "high")
                };
                bi(
                    format!(
                        "Önceki {tr} {} (RSI {}), yeni {tr} {} (RSI {}).",
                        p1.tr, r1.tr, p2.tr, r2.tr
                    ),
                    format!(
                        "Previous {en} {} (RSI {}), new {en} {} (RSI {}).",
                        p1.en, r1.en, p2.en, r2.en
                    ),
                )
            }
            "squeeze.bandwidth" => {
                let lo = self.num(f, "lower", Cat::Price)?;
                let hi = self.num(f, "upper", Cat::Price)?;
                let u = self.span(BandSqueeze::LOOKBACK).short_gen();
                bi(
                    format!("Bant {}–{}, son {} en darı.", lo.tr, hi.tr, u.tr),
                    format!("Band {}–{}, the narrowest in {}.", lo.en, hi.en, u.en),
                )
            }
            "flow.volume_surge" => {
                let r = self.num(f, "ratio", Cat::Osc)?;
                let m = Num::new(VolumeSurge::MULT, Cat::Osc);
                let a = self.span(VolumeSurge::WINDOW).adj();
                bi(
                    format!(
                        "Hacim {} ortalamanın {} katıydı, eşik {} kat.",
                        a.tr, r.tr, m.tr
                    ),
                    format!(
                        "Volume was {} times the {} average, the mark {}×.",
                        r.en, a.en, m.en
                    ),
                )
            }
            "flow.institutional" => {
                let net = self.num(f, "net", Cat::Count)?;
                let avg = self.num(f, "avg5", Cat::Count)?;
                let m = Num::new(InstitutionalFlow::MULT, Cat::Osc);
                let a = self.span(InstitutionalFlow::WINDOW).adj();
                let (tr, en) = if up(self.dir) {
                    ("alımı", "buying")
                } else {
                    ("satışı", "selling")
                };
                bi(
                    format!("En çok işlem yapan kurumun net {tr} {} TL, {} ortalama {} TL, eşik {} kat.", net.tr, a.tr, avg.tr, m.tr),
                    format!("Top institution net {en} TRY {}, {} average TRY {}, mark {}×.", net.en, a.en, avg.en, m.en),
                )
            }
            _ => return None,
        })
    }

    /// The shortest statement of the line (no number when redacted).
    fn d_threshold_short(&self) -> Bi {
        let side_up = up(self.dir);
        let red = self.red;
        let b = |tr: String, en: String| bi(tr, en);
        match self.det {
            "breakout.range" if !red => {
                let s = self.span(RangeBreakout::WINDOW);
                let l = self.num(self.lead, "level", Cat::Price);
                let lv = l.map(|l| (l.tr, l.en)).unwrap_or_default();
                if side_up {
                    b(
                        format!("Çizgi önceki {} en yükseği, {}.", s.trading_gen().tr, lv.0),
                        format!("The line is the prior {} high, {}.", s.adj().en, lv.1),
                    )
                } else {
                    b(
                        format!("Çizgi önceki {} en düşüğü, {}.", s.trading_gen().tr, lv.0),
                        format!("The line is the prior {} low, {}.", s.adj().en, lv.1),
                    )
                }
            }
            "flow.institutional" if red => {
                let (tr, en) = if side_up {
                    ("alımı", "buying")
                } else {
                    ("satışı", "selling")
                };
                b(
                    format!("En çok net işlem yapan kurumun net {tr} son günlerdeki olağan düzeyinin çok üstündeydi."),
                    format!("The top institution's net {en} was far beyond its recent usual level."),
                )
            }
            _ => {
                let e = selvi_glossary::plain_event(self.story.kind, self.dir, Lang::Tr);
                let ee = selvi_glossary::plain_event(self.story.kind, self.dir, Lang::En);
                b(format!("Olan: {e}."), format!("What happened: {ee}."))
            }
        }
    }

    fn d1_breakout(&self, w: &Bi) -> Option<Bi> {
        let s = self.span(RangeBreakout::WINDOW);
        let l = self.num(self.lead, "level", Cat::Price)?;
        let c = self.num(self.lead, "close", Cat::Price)?;
        let wc = w.cap();
        Some(if up(self.dir) {
            bi(
                format!(
                    "{} kapanış {} oldu, önceki {} en yükseği {} idi.",
                    wc.tr,
                    c.tr,
                    s.trading_gen().tr,
                    l.tr
                ),
                format!(
                    "{}, it closed at {}, above the prior {} high of {}.",
                    wc.en,
                    c.en,
                    s.adj().en,
                    l.en
                ),
            )
        } else {
            bi(
                format!(
                    "{} kapanış {} oldu, önceki {} en düşüğü {} idi.",
                    wc.tr,
                    c.tr,
                    s.trading_gen().tr,
                    l.tr
                ),
                format!(
                    "{}, it closed at {}, below the prior {} low of {}.",
                    wc.en,
                    c.en,
                    s.adj().en,
                    l.en
                ),
            )
        })
    }

    fn d1_extreme(&self, w: &Bi) -> Option<Bi> {
        let v = self.num(self.lead, "value", Cat::Osc)?;
        let l = self.rsi_line()?;
        let wc = w.cap();
        Some(if up(self.dir) {
            bi(
                format!("{} RSI {} oldu ve {} çizgisini geçti.", wc.tr, v.tr, l.tr),
                format!("{}, RSI reached {}, past the {} line.", wc.en, v.en, l.en),
            )
        } else {
            bi(
                format!(
                    "{} RSI {} oldu ve {} çizgisinin altına indi.",
                    wc.tr, v.tr, l.tr
                ),
                format!("{}, RSI fell to {}, below the {} line.", wc.en, v.en, l.en),
            )
        })
    }

    fn d1_ma(&self, w: &Bi) -> Option<Bi> {
        let c = self.num(self.lead, "close", Cat::Price)?;
        let f = self.num(self.lead, "sma20", Cat::Price)?;
        let s = self.num(self.lead, "sma50", Cat::Price)?;
        let af = self.span(MaStack::FAST).adj();
        let asl = self.span(MaStack::SLOW).adj();
        let (tr_d, en_d) = if up(self.dir) {
            ("yukarı", "upward")
        } else {
            ("aşağı", "downward")
        };
        let wc = w.cap();
        Some(bi(
            format!(
                "{} fiyat {}, {} ortalama {}, {} ortalama {} idi: sıra {tr_d} doğru.",
                wc.tr, c.tr, af.tr, f.tr, asl.tr, s.tr
            ),
            format!(
                "{}, the price was {}, with the {} average at {} and the {} at {}, lined up {en_d}.",
                wc.en, c.en, af.en, f.en, asl.en, s.en
            ),
        ))
    }

    fn d1_divergence(&self) -> Option<Bi> {
        let p1 = self.num(self.lead, "p1_price", Cat::Price)?;
        let p2 = self.num(self.lead, "p2_price", Cat::Price)?;
        let r1 = self.num(self.lead, "p1_rsi", Cat::Osc)?;
        let r2 = self.num(self.lead, "p2_rsi", Cat::Osc)?;
        Some(if up(self.dir) {
            bi(
                format!(
                    "Önceki dipte fiyat {} ve RSI {} idi, yeni dipte fiyat {} ve RSI {}: düşüş hız kesiyor.",
                    p1.tr, r1.tr, p2.tr, r2.tr
                ),
                format!(
                    "At the previous low the price was {} with RSI at {}, at the new low {} with RSI at {}: the fall is losing pace.",
                    p1.en, r1.en, p2.en, r2.en
                ),
            )
        } else {
            bi(
                format!(
                    "Önceki tepede fiyat {} ve RSI {} idi, yeni tepede fiyat {} ve RSI {}: yükseliş hız kesiyor.",
                    p1.tr, r1.tr, p2.tr, r2.tr
                ),
                format!(
                    "At the previous high the price was {} with RSI at {}, at the new high {} with RSI at {}: the rise is losing pace.",
                    p1.en, r1.en, p2.en, r2.en
                ),
            )
        })
    }

    fn d1_squeeze(&self, w: &Bi) -> Option<Bi> {
        let lo = self.num(self.lead, "lower", Cat::Price)?;
        let hi = self.num(self.lead, "upper", Cat::Price)?;
        let u = self.span(BandSqueeze::LOOKBACK).short_gen();
        let wc = w.cap();
        Some(bi(
            format!(
                "{} bant {} ile {} arasına daraldı, bu son {} en darı.",
                wc.tr, lo.tr, hi.tr, u.tr
            ),
            format!(
                "{}, the band narrowed to {}–{}, its narrowest in {}.",
                wc.en, lo.en, hi.en, u.en
            ),
        ))
    }

    fn d1_volume(&self, w: &Bi) -> Option<Bi> {
        let v = self.num(self.lead, "volume", Cat::Count)?;
        let a = self.num(self.lead, "vol_avg", Cat::Count)?;
        let m = Num::new(VolumeSurge::MULT, Cat::Osc);
        let ad = self.span(VolumeSurge::WINDOW).adj();
        let wc = w.cap();
        Some(bi(
            format!(
                "{} {} adet hisse el değiştirdi, {} ortalama {} adet, eşik bunun {} katı.",
                wc.tr, v.tr, ad.tr, a.tr, m.tr
            ),
            format!(
                "{}, {} shares changed hands against a {} average of {}, and the mark is {} times that.",
                wc.en, v.en, ad.en, a.en, m.en
            ),
        ))
    }

    fn d1_institutional(&self, w: &Bi) -> Option<Bi> {
        let net = self.num(self.lead, "net", Cat::Count)?;
        let avg = self.num(self.lead, "avg5", Cat::Count)?;
        let m = Num::new(InstitutionalFlow::MULT, Cat::Osc);
        let s = self.span(InstitutionalFlow::WINDOW);
        let (tr, en) = if up(self.dir) {
            ("alımı", "buying")
        } else {
            ("satışı", "selling")
        };
        let wc = w.cap();
        Some(bi(
            format!(
                "{} en çok net işlem yapan kurumun net {tr} {} TL oldu, son {} ortalaması {} TL, eşik bunun {} katı.",
                wc.tr, net.tr, s.short_gen().tr, avg.tr, m.tr
            ),
            format!(
                "{}, the top institution's net {en} was TRY {} against a {} average of TRY {}, and the mark is {} times that.",
                wc.en, net.en, s.adj().en, avg.en, m.en
            ),
        ))
    }
}
