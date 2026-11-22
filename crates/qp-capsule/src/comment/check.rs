//! The draft check ([`validate`]): every claim of a comment draft against the evidence.

use super::*;

fn reason(
    check: Check,
    code: &str,
    token: Option<String>,
    sentence: Option<usize>,
    hint: &str,
) -> Reason {
    Reason {
        check,
        code: code.to_owned(),
        token,
        sentence,
        hint: hint.to_owned(),
    }
}

/// The written form of an `f64` as a [`Num`] (unsigned), for comparing an external
/// value with withheld values at the precision it is given with.
fn num_of_value(v: f64) -> Num {
    let s = format!("{}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let decimals = u8::try_from(frac.len()).unwrap_or(u8::MAX);
    let scaled = if frac.len() <= 18 {
        format!("{int}{frac}").parse::<u128>().ok()
    } else {
        None
    };
    Num {
        raw: s.clone(),
        negative: None,
        scaled,
        decimals,
        value: v.abs(),
        kind: NumKind::Plain,
    }
}

fn other(lang: Lang) -> Lang {
    match lang {
        Lang::Tr => Lang::En,
        Lang::En => Lang::Tr,
    }
}

/// (R5B, M28 usability) What is removed from the text before the number and direction
/// scans: the instrument symbols and every id the draft may name (known, hidden or
/// cited) — "cap.SYN02.breakout.up.151" carries neither the number 151 nor the word
/// "up". Only id-shaped tokens (a letter and a separator, 5+ chars) are removed, so a
/// short or numeric id never masks a real number.
fn text_ids(d: &CommentDraft, ev: &Evidence) -> Vec<String> {
    let id_like = |x: &&String| {
        x.chars().count() >= 5
            && x.chars().any(char::is_alphabetic)
            && x.chars().any(|c| matches!(c, '.' | '-' | '_' | ':'))
    };
    let mut v: Vec<String> = ev.symbols.clone();
    v.extend(
        ev.known_ids
            .iter()
            .chain(&ev.hidden_ids)
            .chain(&d.cites)
            .filter(id_like)
            .cloned(),
    );
    v.sort();
    v.dedup();
    v
}

/// Validates a comment draft against the reader-projected evidence (PLAN §2.1).
///
/// # Errors
///
/// Every reason the draft is rejected, in a fixed order: citations, external quotes,
/// then sentence by sentence (advice, markers, numbers, per-sentence citation).
pub fn validate(d: &CommentDraft, ev: &Evidence, s: Strictness) -> Result<(), Vec<Reason>> {
    let lang = lang_of(d.lang);
    let strip = text_ids(d, ev);
    let numbers_on = matches!(s, Strictness::CitesNumbers | Strictness::PerSentence);
    let mut out = Vec::new();

    // 1. citations
    if d.cites.is_empty() {
        out.push(reason(
            Check::Cites,
            "no_cites",
            None,
            None,
            "Cite at least one fact, finding or capsule id the comment rests on.",
        ));
    }
    for id in &d.cites {
        if ev.known_ids.contains(id) {
            continue;
        }
        if ev.hidden_ids.contains(id) {
            out.push(reason(
                Check::License,
                "not_visible",
                Some(id.clone()),
                None,
                "This id cannot be cited for this reader; cite only ids the reader can see.",
            ));
        } else {
            out.push(reason(
                Check::Cites,
                "unknown_id",
                Some(id.clone()),
                None,
                "This id does not exist; cite ids returned by the Qapsule tools.",
            ));
        }
    }

    // external quotes
    for q in &d.external {
        if q.source.trim().is_empty() {
            out.push(reason(
                Check::Cites,
                "external_no_source",
                Some(q.quote.clone()),
                None,
                "Name the external source of every quote.",
            ));
        }
        let quote_nums: Vec<Num> = numbers(&q.quote, lang, &ev.symbols)
            .into_iter()
            .chain(numbers(&q.quote, other(lang), &ev.symbols))
            .collect();
        for &v in &q.values {
            let as_num = num_of_value(v);
            let leaked = ev.withheld.iter().any(|&w| withheld_matches(&as_num, w));
            if leaked {
                out.push(reason(
                    Check::License,
                    "licensed_external",
                    Some(format!("{v}")),
                    None,
                    "This value is licensed data the reader may not see; it cannot be passed as an external value.",
                ));
            } else if numbers_on && !quote_nums.iter().any(|t| number_matches(t, v, None)) {
                out.push(reason(
                    Check::Number,
                    "external_value_not_in_quote",
                    Some(format!("{v}")),
                    None,
                    "Every external value must appear in its quote text.",
                ));
            }
        }
    }

    // per sentence
    let ext_values: Vec<f64> = d.external.iter().flat_map(|q| q.values.clone()).collect();
    for (i, sent) in sentences(&d.text).iter().enumerate() {
        for h in advice_hits(sent, &ev.symbols) {
            out.push(reason(
                Check::Advice,
                "advice_word",
                Some(h),
                Some(i),
                "Describe what happened; do not advise (no buy/sell, opportunity, target price, should).",
            ));
        }
        let mut valid_markers = 0usize;
        for m in markers(sent) {
            if m == 0 || m > d.cites.len() {
                out.push(reason(
                    Check::Cites,
                    "bad_marker",
                    Some(format!("[{m}]")),
                    Some(i),
                    "A marker [n] must point to the n-th id in cites (1-based).",
                ));
            } else {
                valid_markers += 1;
            }
        }
        let nums = numbers(sent, lang, &strip);
        for n in &nums {
            let visible = ev
                .values
                .iter()
                .any(|cv| number_matches(n, cv.value, Some(cv.decimals)));
            if visible {
                continue;
            }
            if ev.withheld.iter().any(|&w| withheld_matches(n, w)) {
                out.push(reason(
                    Check::License,
                    "licensed_value",
                    Some(n.raw.clone()),
                    Some(i),
                    "This number is licensed data the reader may not see; remove it.",
                ));
            } else if n.scaled.is_none() {
                // fail-closed: a number that cannot be read exactly never passes
                out.push(reason(
                    Check::Number,
                    "unsupported_number",
                    Some(n.raw.clone()),
                    Some(i),
                    "This number is too long to check; write it at the precision of its source.",
                ));
            } else if numbers_on && !ext_values.iter().any(|&v| number_matches(n, v, None)) {
                out.push(reason(
                    Check::Number,
                    "unsupported_number",
                    Some(n.raw.clone()),
                    Some(i),
                    "Every number must be a value of a cited item or of a named external quote, written at its precision; derived numbers are not allowed.",
                ));
            }
        }
        if s == Strictness::PerSentence && !nums.is_empty() && valid_markers == 0 {
            out.push(reason(
                Check::Sentence,
                "uncited_sentence",
                None,
                Some(i),
                "A sentence with a number must carry a citation marker such as [1].",
            ));
        }
    }

    // 5b. (R5B audit B1) licence, fail-closed: a licensed value must not get through hidden
    //     in a "date" or "time", split across digit groups, or written in words.
    //     For a writer with withheld numbers in scope, a hiding form whose value the
    //     reader does not see is refused whether it is licensed or made up: the writer
    //     cannot tell the two apart (both are told `unsupported_number`, §25.4).
    if !ev.withheld.is_empty() && !out.iter().any(|r| r.code == "licensed_value") {
        let hidden = sentences(&d.text).iter().enumerate().find_map(|(i, sent)| {
            raw_numbers(sent, lang, &strip)
                .into_iter()
                .find(|n| {
                    !ev.values
                        .iter()
                        .any(|cv| number_matches(n, cv.value, Some(cv.decimals)))
                })
                .map(|n| (i, n))
        });
        if let Some((i, n)) = hidden {
            let licensed = ev.withheld.iter().any(|&w| withheld_matches(&n, w));
            out.push(if licensed {
                reason(
                    Check::License,
                    "licensed_value",
                    Some(n.raw),
                    Some(i),
                    "This number is licensed data the reader may not see; remove it.",
                )
            } else {
                reason(
                    Check::Number,
                    "unsupported_number",
                    Some(n.raw),
                    Some(i),
                    "A date or time here must not carry numbers the reader cannot see; leave it out.",
                )
            });
        }
    }
    // (every writer alike, so it tells nothing about what is withheld)
    if let Some(w) = spelled_decimal(&d.text, lang) {
        out.push(reason(
            Check::Number,
            "spelled_number",
            Some(w),
            None,
            "Write numbers in digits so they can be checked against the evidence.",
        ));
    }

    // 6. direction
    if numbers_on {
        let dirs: Vec<Direction> = ev
            .directions
            .iter()
            .filter(|c| c.direction != Direction::Neutral && d.cites.contains(&c.id))
            .map(|c| c.direction)
            .collect();
        let want = dirs
            .first()
            .copied()
            .filter(|w| dirs.iter().all(|x| x == w));
        if let Some(want) = want {
            let (up, down) = direction_words(&d.text, lang, &strip);
            let opposite = match want {
                Direction::Up if up.is_empty() => down.first(),
                Direction::Down if down.is_empty() => up.first(),
                _ => None,
            };
            if let Some(word) = opposite {
                let sentence = sentences(&d.text).iter().position(|s| {
                    let (u, w) = direction_words(s, lang, &strip);
                    u.contains(word) || w.contains(word)
                });
                out.push(reason(
                    Check::Direction,
                    "wrong_direction",
                    Some(word.clone()),
                    sentence,
                    "The text states the opposite direction of the cited items; describe the direction they show.",
                ));
            }
        }
    }

    if out.is_empty() {
        Ok(())
    } else {
        Err(out)
    }
}
