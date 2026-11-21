//! R5B/V: the comment validator (PLAN §2.1).

use qp_capsule::comment::{
    advice_hits, direction_words, markers, number_matches, numbers, validate, withheld_matches,
    Check, CitedDirection, CitedValue, CommentDraft, Direction, Evidence, ExternalQuote, Num,
    NumKind, Reason, Strictness,
};
use qp_capsule::Lang;
use qp_schema::protocol::Language;

const ALL: [Strictness; 3] = [
    Strictness::Cites,
    Strictness::CitesNumbers,
    Strictness::PerSentence,
];

fn ev_with(values: &[f64], withheld: &[f64]) -> Evidence {
    Evidence {
        known_ids: ["f1", "f2", "cap1"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        hidden_ids: ["lic1"].iter().map(|s| (*s).to_owned()).collect(),
        values: values
            .iter()
            .enumerate()
            .map(|(i, &v)| CitedValue {
                id: "f1".to_owned(),
                key: format!("k{i}"),
                value: v,
                decimals: 4,
            })
            .collect(),
        withheld: withheld.to_vec(),
        symbols: vec!["SYN03".to_owned(), "ISCTR".to_owned(), "XU100".to_owned()],
        directions: vec![],
    }
}

fn ev(values: &[f64]) -> Evidence {
    ev_with(values, &[])
}

fn tr(text: &str) -> CommentDraft {
    CommentDraft {
        lang: Language::Tr,
        text: text.to_owned(),
        cites: vec!["f1".to_owned()],
        external: vec![],
    }
}

fn en(text: &str) -> CommentDraft {
    CommentDraft {
        lang: Language::En,
        ..tr(text)
    }
}

fn codes(r: Result<(), Vec<Reason>>) -> Vec<(Check, String)> {
    r.err()
        .unwrap_or_default()
        .into_iter()
        .map(|r| (r.check, r.code))
        .collect()
}

fn ok(d: &CommentDraft, e: &Evidence) -> bool {
    validate(d, e, Strictness::CitesNumbers).is_ok()
}

fn vals(text: &str, lang: Lang) -> Vec<f64> {
    numbers(text, lang, &["SYN03".to_owned()])
        .iter()
        .map(|n| n.value)
        .collect()
}

fn num(text: &str, lang: Lang) -> Num {
    numbers(text, lang, &[]).remove(0)
}

// ---------------------------------------------------------------- 1. citations

#[test]
fn clean_comment_passes_every_strictness() {
    let e = ev(&[44.3187, 72.4]);
    let d = tr("SYN03 kapanışı 44,32 oldu [1]. RSI 72,4 seviyesinde [1].");
    for s in ALL {
        assert_eq!(validate(&d, &e, s), Ok(()), "{s:?}");
    }
}

#[test]
fn clean_english_comment_passes() {
    let e = ev(&[44.3187, 72.4]);
    let d = en("SYN03 closed at 44.32 [1]. RSI is at 72.4 [1].");
    for s in ALL {
        assert_eq!(validate(&d, &e, s), Ok(()), "{s:?}");
    }
}

#[test]
fn no_cites_is_rejected() {
    let mut d = tr("Fiyat yükseldi.");
    d.cites.clear();
    assert_eq!(
        codes(validate(&d, &ev(&[]), Strictness::Cites)),
        vec![(Check::Cites, "no_cites".to_owned())]
    );
}

#[test]
fn unknown_id_is_rejected() {
    let mut d = tr("Fiyat yükseldi.");
    d.cites.push("nope".to_owned());
    let r = validate(&d, &ev(&[]), Strictness::Cites).unwrap_err();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].check, Check::Cites);
    assert_eq!(r[0].code, "unknown_id");
    assert_eq!(r[0].token.as_deref(), Some("nope"));
}

#[test]
fn hidden_id_is_a_licence_reason() {
    let mut d = tr("Fiyat yükseldi.");
    d.cites = vec!["lic1".to_owned()];
    assert_eq!(
        codes(validate(&d, &ev(&[]), Strictness::Cites)),
        vec![(Check::License, "not_visible".to_owned())]
    );
}

#[test]
fn marker_out_of_range_is_rejected() {
    let d = tr("Fiyat yükseldi [2].");
    assert_eq!(
        codes(validate(&d, &ev(&[]), Strictness::Cites)),
        vec![(Check::Cites, "bad_marker".to_owned())]
    );
    let d0 = tr("Fiyat yükseldi [0].");
    assert!(validate(&d0, &ev(&[]), Strictness::Cites).is_err());
}

#[test]
fn markers_are_parsed_and_not_numbers() {
    assert_eq!(markers("RSI 70 [1, 2] ve [3]."), vec![1, 2, 3]);
    assert_eq!(vals("RSI 70 [1, 2] ve [3].", Lang::Tr), vec![70.0]);
}

#[test]
fn external_quote_needs_a_source() {
    let mut d = tr("Haber akışı sakin.");
    d.external = vec![ExternalQuote {
        source: " ".to_owned(),
        quote: "Nothing new.".to_owned(),
        values: vec![],
    }];
    assert_eq!(
        codes(validate(&d, &ev(&[]), Strictness::Cites)),
        vec![(Check::Cites, "external_no_source".to_owned())]
    );
}

// ---------------------------------------------------------------- 2. numbers: rounding

#[test]
fn rounding_to_written_decimals() {
    let e = ev(&[44.3187]);
    assert!(ok(&tr("Kapanış 44,32."), &e));
    assert!(ok(&tr("Kapanış 44,3."), &e));
    assert!(ok(&tr("Kapanış 44."), &e));
    assert!(ok(&tr("Kapanış 44,319."), &e));
    assert!(!ok(&tr("Kapanış 44,4."), &e));
    assert!(!ok(&tr("Kapanış 44,31."), &e));
    assert!(!ok(&tr("Kapanış 45."), &e));
}

#[test]
fn rounding_half_up_boundary() {
    let e = ev(&[2.345]);
    assert!(ok(&tr("Değer 2,35."), &e));
    assert!(!ok(&tr("Değer 2,34."), &e));
    let e = ev(&[0.125]);
    assert!(ok(&en("Value 0.13."), &e));
    assert!(!ok(&en("Value 0.12."), &e));
}

#[test]
fn rounding_decimal_half_stored_below() {
    // 1.005 is stored as 1.00499999…; a person rounds it to 1.01.
    let e = ev(&[1.005]);
    assert!(ok(&en("Value 1.01."), &e));
    assert!(!ok(&en("Value 1.00."), &e));
}

#[test]
fn rounding_half_away_from_zero_for_negatives() {
    let e = ev(&[-2.345]);
    assert!(ok(&tr("Değişim -2,35."), &e));
    assert!(!ok(&tr("Değişim -2,34."), &e));
}

#[test]
fn shown_precision_padding_counts() {
    let mut e = ev(&[44.3187]);
    e.values[0].decimals = 2;
    // the engine shows 44,32; "44,320" is that value padded.
    assert!(ok(&tr("Kapanış 44,320."), &e));
    assert!(!ok(&tr("Kapanış 44,330."), &e));
}

#[test]
fn explicit_sign_must_agree() {
    let e = ev(&[1.2]);
    assert!(!ok(&tr("Değişim -1,2."), &e));
    assert!(ok(&tr("Değişim +1,2."), &e));
    let e = ev(&[-1.2]);
    assert!(ok(&tr("Değişim −1,2."), &e), "unicode minus");
    assert!(ok(&tr("Değişim -1,2."), &e));
    assert!(!ok(&tr("Değişim +1,2."), &e));
}

#[test]
fn unsigned_number_matches_magnitude() {
    let e = ev(&[-1.2]);
    assert!(ok(&tr("Fiyat 1,2 geriledi."), &e));
}

#[test]
fn hyphen_is_not_a_minus() {
    let n = numbers("5-10 arası", Lang::Tr, &[]);
    assert_eq!(n.len(), 2);
    assert_eq!(n[1].negative, None);
    assert!((n[1].value - 10.0).abs() < 1e-12);
    assert_eq!(n[1].raw, "10");
}

// ---------------------------------------------------------------- 2. numbers: forms

#[test]
fn percent_matches_ratio_or_points() {
    let e = ev(&[0.0123]);
    assert!(ok(&tr("Getiri %1,23."), &e));
    assert!(ok(&tr("Getiri 1,23%."), &e));
    assert!(ok(&en("Return 1.23%."), &e));
    assert!(ok(&tr("Getiri %1,2."), &e));
    assert!(!ok(&tr("Getiri %1,24."), &e));
    assert!(!ok(&tr("Getiri 1,23."), &e), "a plain number is not ×100");
    let e = ev(&[12.5]);
    assert!(ok(&tr("Pay %12,5."), &e));
    assert!(ok(&tr("Pay %12.5."), &e));
}

#[test]
fn negative_percent_forms() {
    let e = ev(&[-0.0123]);
    assert!(ok(&tr("Getiri -%1,23."), &e));
    assert!(ok(&tr("Getiri %-1,23."), &e));
    assert!(ok(&en("Return -1.23%."), &e));
    let n = num("-%1,23", Lang::Tr);
    assert_eq!(n.kind, NumKind::Percent);
    assert_eq!(n.negative, Some(true));
}

#[test]
fn multiples_use_the_value_directly() {
    let e = ev(&[2.4987]);
    assert!(ok(&tr("Hacim ortalamanın 2,5 katı."), &e));
    assert!(ok(&tr("Hacim 2,5 kat arttı."), &e));
    assert!(ok(&en("Volume 2.5x average."), &e));
    assert!(ok(&en("Volume 2.5 times average."), &e));
    assert!(!ok(&en("Volume 2.6x average."), &e));
    assert_eq!(num("2.5x", Lang::En).kind, NumKind::Multiple);
    assert_eq!(num("2,5 katı", Lang::Tr).kind, NumKind::Multiple);
}

#[test]
fn thousands_and_decimals_tr_and_en() {
    assert_eq!(vals("1.234,5", Lang::Tr), vec![1234.5]);
    assert_eq!(vals("1,234.5", Lang::En), vec![1234.5]);
    // both separators: the last one is the decimal mark, whatever the language.
    assert_eq!(vals("1,234.5", Lang::Tr), vec![1234.5]);
    assert_eq!(vals("1.234,5", Lang::En), vec![1234.5]);
    assert_eq!(vals("1.234.567", Lang::Tr), vec![1_234_567.0]);
    assert_eq!(vals("1,234,567", Lang::En), vec![1_234_567.0]);
    let e = ev(&[1234.5]);
    assert!(ok(&tr("Kapanış 1.234,5."), &e));
    assert!(ok(&en("Close 1,234.5."), &e));
}

#[test]
fn ambiguous_single_separator_follows_language() {
    // TR: '.' + three digits is a thousands separator; EN: the decimal mark.
    assert_eq!(vals("1.234", Lang::Tr), vec![1234.0]);
    assert_eq!(vals("1.234", Lang::En), vec![1.234]);
    // EN: ',' + three digits is a thousands separator; TR: the decimal mark.
    assert_eq!(vals("1,234", Lang::En), vec![1234.0]);
    assert_eq!(vals("1,234", Lang::Tr), vec![1.234]);
    // the other language's decimal mark when not three digits follow
    assert_eq!(vals("12.5", Lang::Tr), vec![12.5]);
    assert_eq!(vals("12,5", Lang::En), vec![12.5]);
    // a leading zero is never a thousands group
    assert_eq!(vals("0.123", Lang::Tr), vec![0.123]);
    assert_eq!(num("1.234", Lang::Tr).decimals, 0);
    assert_eq!(num("1.234", Lang::En).decimals, 3);
}

#[test]
fn times_are_not_numbers() {
    assert!(vals("Saat 14:30'da ve 09:05:10'da", Lang::Tr).is_empty());
    assert!(ok(&tr("Kırılım 14:30'da geldi."), &ev(&[])));
}

#[test]
fn tr_dates_are_not_numbers() {
    assert!(vals("12 Kas", Lang::Tr).is_empty());
    assert!(vals("12 Kasım'da", Lang::Tr).is_empty());
    assert!(vals("12 KASIM 2026", Lang::Tr).is_empty());
    assert!(vals("Kasım 2026 içinde", Lang::Tr).is_empty());
    assert!(vals("21.11.2026 ve 2026-11-21", Lang::Tr).is_empty());
    assert!(vals("2026'da ve 2026 yılında", Lang::Tr).is_empty());
    assert!(ok(&tr("12 Kas 14:30'da kırılım oldu."), &ev(&[])));
}

#[test]
fn en_dates_are_not_numbers() {
    assert!(vals("12 Nov", Lang::En).is_empty());
    assert!(vals("Nov 12, 2026", Lang::En).is_empty());
    assert!(vals("12th of November 2026", Lang::En).is_empty());
    assert!(vals("2026-11-21 and 11/21/2026", Lang::En).is_empty());
    assert!(vals("in 2026", Lang::En).is_empty());
    assert!(vals("12 May 2026", Lang::En).is_empty());
}

#[test]
fn en_may_without_year_is_a_modal_not_a_month() {
    assert_eq!(vals("the ratio of 12 may rise", Lang::En), vec![12.0]);
}

#[test]
fn bare_year_is_a_number() {
    assert_eq!(vals("seviye 2026 oldu", Lang::Tr), vec![2026.0]);
}

#[test]
fn symbols_are_removed_first() {
    assert!(vals("SYN03 ve SYN03'te", Lang::Tr).is_empty());
    let e = ev(&[]);
    assert!(ok(&tr("ISCTR ve XU100 yükseldi."), &e));
    assert!(ok(&en("SYN03 rose."), &e));
}

#[test]
fn integer_counts_must_be_values() {
    let e = ev(&[10.0, 7.0]);
    assert!(ok(&tr("Benzer 10 örnekte 7 kez yükseldi."), &e));
    let e = ev(&[10.0]);
    let r = validate(
        &tr("Benzer 10 örnekte 7 kez yükseldi."),
        &e,
        Strictness::CitesNumbers,
    )
    .unwrap_err();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].code, "unsupported_number");
    assert_eq!(r[0].token.as_deref(), Some("7"));
    assert_eq!(r[0].sentence, Some(0));
}

#[test]
fn derived_numbers_are_rejected() {
    // 44,32 − 40,00 = 4,32 is not an engine value.
    let e = ev(&[44.32, 40.0]);
    assert_eq!(
        codes(validate(
            &tr("Fiyat 40'tan 44,32'ye, 4,32 yükseldi."),
            &e,
            Strictness::CitesNumbers
        )),
        vec![(Check::Number, "unsupported_number".to_owned())]
    );
}

#[test]
fn cites_strictness_does_not_check_numbers() {
    let d = tr("Fiyat 99,99 oldu.");
    assert!(validate(&d, &ev(&[]), Strictness::Cites).is_ok());
    assert!(validate(&d, &ev(&[]), Strictness::CitesNumbers).is_err());
    assert!(validate(&d, &ev(&[]), Strictness::PerSentence).is_err());
}

#[test]
fn external_value_supports_a_number() {
    let mut d = tr("Haberde net kârın 1.250 milyon olduğu yazıyor.");
    d.external = vec![ExternalQuote {
        source: "matriks:news/1".to_owned(),
        quote: "Net kâr 1.250 milyon TL oldu.".to_owned(),
        values: vec![1250.0],
    }];
    assert_eq!(validate(&d, &ev(&[]), Strictness::CitesNumbers), Ok(()));
}

#[test]
fn external_value_must_be_in_its_quote() {
    let mut d = tr("Haberde net kâr 1.250 milyon.");
    d.external = vec![ExternalQuote {
        source: "matriks:news/1".to_owned(),
        quote: "Şirket kâr açıkladı.".to_owned(),
        values: vec![1250.0],
    }];
    assert_eq!(
        codes(validate(&d, &ev(&[]), Strictness::CitesNumbers)),
        vec![(Check::Number, "external_value_not_in_quote".to_owned())]
    );
    // the plain citation level does not read quotes
    assert!(validate(&d, &ev(&[]), Strictness::Cites).is_ok());
}

#[test]
fn external_quote_in_the_other_language_is_read() {
    let mut d = tr("Habere göre gelir 1.234,5 milyon.");
    d.external = vec![ExternalQuote {
        source: "reuters".to_owned(),
        quote: "Revenue was 1,234.5 million.".to_owned(),
        values: vec![1234.5],
    }];
    assert!(validate(&d, &ev(&[]), Strictness::CitesNumbers).is_ok());
}

// ---------------------------------------------------------------- 3. advice

#[test]
fn advice_tr_hits() {
    let s: Vec<String> = vec![];
    assert_eq!(
        advice_hits("Şimdi al, fırsatı kaçırma", &s),
        vec!["al", "fırsatı", "kaçırma"]
    );
    assert_eq!(
        advice_hits("Hedef fiyat 50; zarar kes 40'ta", &s),
        vec!["hedef fiyat", "zarar kes"]
    );
    assert_eq!(advice_hits("Hemen SAT", &s), vec!["sat"]);
    assert_eq!(advice_hits("ALINMALI", &s), vec!["alınmalı"]);
    assert_eq!(advice_hits("bunu tavsiye ederiz", &s), vec!["tavsiye"]);
}

#[test]
fn advice_tr_suffix_rule_avoids_false_friends() {
    let s: Vec<String> = vec![];
    assert!(advice_hits("Büyük alım geldi; satış sürdü; alt sınır.", &s).is_empty());
    assert!(advice_hits("ALIM baskısı ve SATIŞLAR", &s).is_empty());
    assert!(advice_hits("Alıcılar ve satıcılar dengede", &s).is_empty());
}

#[test]
fn advice_en_hits_and_non_hits() {
    let s: Vec<String> = vec![];
    assert_eq!(
        advice_hits("Buy now: a rare opportunity", &s),
        vec!["buy", "opportunity"]
    );
    assert_eq!(
        advice_hits("You should set a stop-loss; target price 50", &s),
        vec!["should", "target price", "stop-loss"]
    );
    assert_eq!(advice_hits("Don't miss it; go long", &s).len(), 2);
    assert!(advice_hits("Heavy buying and selling; sellers left", &s).is_empty());
}

#[test]
fn advice_is_rejected_at_every_strictness() {
    let d = tr("Bu bir fırsat.");
    for s in ALL {
        let r = validate(&d, &ev(&[]), s).unwrap_err();
        assert_eq!(r.len(), 1, "{s:?}");
        assert_eq!(r[0].check, Check::Advice);
        assert_eq!(r[0].code, "advice_word");
        assert_eq!(r[0].token.as_deref(), Some("fırsat"));
        assert_eq!(r[0].sentence, Some(0));
    }
}

// ---------------------------------------------------------------- 4. licence

#[test]
fn licensed_value_in_text_is_rejected_at_every_strictness() {
    let e = ev_with(&[], &[123.456]);
    let d = tr("Son fiyat 123,46 [1].");
    for s in ALL {
        assert_eq!(
            codes(validate(&d, &e, s)),
            vec![(Check::License, "licensed_value".to_owned())],
            "{s:?}"
        );
    }
}

#[test]
fn licensed_value_passed_as_external_is_rejected() {
    let e = ev_with(&[], &[123.456]);
    let mut d = tr("Habere göre son fiyat 123,46.");
    d.external = vec![ExternalQuote {
        source: "blog".to_owned(),
        quote: "Fiyat 123,46.".to_owned(),
        values: vec![123.46],
    }];
    for s in ALL {
        let c = codes(validate(&d, &e, s));
        assert!(
            c.contains(&(Check::License, "licensed_external".to_owned())),
            "{s:?}: {c:?}"
        );
        assert!(
            c.contains(&(Check::License, "licensed_value".to_owned())),
            "{s:?}: {c:?}"
        );
    }
}

#[test]
fn licensed_external_value_alone_is_rejected() {
    // the value is only in the external list, not in the text
    let e = ev_with(&[], &[123.456]);
    let mut d = tr("Habere bakıldı.");
    d.external = vec![ExternalQuote {
        source: "blog".to_owned(),
        quote: "Fiyat 123,456.".to_owned(),
        values: vec![123.456],
    }];
    assert_eq!(
        codes(validate(&d, &e, Strictness::Cites)),
        vec![(Check::License, "licensed_external".to_owned())]
    );
}

#[test]
fn licensed_percent_is_rejected() {
    let e = ev_with(&[], &[0.0345]);
    assert_eq!(
        codes(validate(&tr("Getiri %3,45."), &e, Strictness::Cites)),
        vec![(Check::License, "licensed_value".to_owned())]
    );
}

#[test]
fn a_visible_value_equal_to_a_withheld_one_passes() {
    let e = ev_with(&[70.0], &[70.0]);
    assert!(validate(&tr("RSI 70."), &e, Strictness::Cites).is_ok());
}

// ---------------------------------------------------------------- 5. per sentence

#[test]
fn per_sentence_requires_a_marker_where_numbers_are() {
    let e = ev(&[72.4]);
    let d = tr("RSI 72,4 oldu. Fiyat yükseldi.");
    assert!(validate(&d, &e, Strictness::CitesNumbers).is_ok());
    let r = validate(&d, &e, Strictness::PerSentence).unwrap_err();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].check, Check::Sentence);
    assert_eq!(r[0].code, "uncited_sentence");
    assert_eq!(r[0].sentence, Some(0));
    let d = tr("RSI 72,4 oldu [1]. Fiyat yükseldi.");
    assert!(validate(&d, &e, Strictness::PerSentence).is_ok());
}

#[test]
fn per_sentence_invalid_marker_does_not_count() {
    let e = ev(&[72.4]);
    let c = codes(validate(
        &tr("RSI 72,4 oldu [5]."),
        &e,
        Strictness::PerSentence,
    ));
    assert_eq!(
        c,
        vec![
            (Check::Cites, "bad_marker".to_owned()),
            (Check::Sentence, "uncited_sentence".to_owned())
        ]
    );
}

#[test]
fn sentence_index_points_at_the_offending_sentence() {
    let e = ev(&[72.4]);
    let r = validate(
        &tr("RSI 72,4 oldu [1]. Fiyat 55 oldu [1]. Bitti."),
        &e,
        Strictness::PerSentence,
    )
    .unwrap_err();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].sentence, Some(1));
    assert_eq!(r[0].token.as_deref(), Some("55"));
}

// ---------------------------------------------------------------- determinism, wire form

#[test]
fn deterministic_same_input_same_output() {
    let mut e = ev_with(&[44.3187, 10.0], &[123.456]);
    e.symbols.push("SYN0".to_owned());
    let mut d = tr("SYN03 al! 44,32 [1]; 123,46 ve 99 [4]. 10 örnekte 7.");
    d.cites.push("nope".to_owned());
    d.external = vec![ExternalQuote {
        source: String::new(),
        quote: "x".to_owned(),
        values: vec![5.0],
    }];
    let a = validate(&d, &e, Strictness::PerSentence);
    for _ in 0..5 {
        let b = validate(&d, &e, Strictness::PerSentence);
        assert_eq!(a, b);
        assert_eq!(
            serde_json::to_string(&a.clone().unwrap_err()).unwrap(),
            serde_json::to_string(&b.unwrap_err()).unwrap()
        );
    }
    assert!(a.unwrap_err().len() >= 6);
}

#[test]
fn reasons_serialize_in_snake_case() {
    let r = validate(&tr("Fiyat 5 [1]."), &ev(&[]), Strictness::PerSentence).unwrap_err();
    let j = serde_json::to_value(&r).unwrap();
    assert_eq!(j[0]["check"], "number");
    assert_eq!(j[0]["code"], "unsupported_number");
    assert_eq!(j[0]["token"], "5");
    assert_eq!(j[0]["sentence"], 0);
    assert!(j[0]["hint"].as_str().unwrap().len() > 10);
    assert_eq!(
        serde_json::to_value(Strictness::PerSentence).unwrap(),
        "per_sentence"
    );
    assert_eq!(
        serde_json::to_value(Strictness::CitesNumbers).unwrap(),
        "cites_numbers"
    );
    assert_eq!(serde_json::to_value(Check::License).unwrap(), "license");
    let none = validate(&tr("x"), &ev(&[]), Strictness::Cites);
    assert!(none.is_ok());
}

#[test]
fn draft_round_trips_through_json() {
    let j = r#"{"lang":"en","text":"RSI 72.4 [1].","cites":["f1"],
        "external":[{"source":"s","quote":"q 1","values":[1.0]}]}"#;
    let d: CommentDraft = serde_json::from_str(j).unwrap();
    assert_eq!(d.lang, Language::En);
    let back: CommentDraft = serde_json::from_str(&serde_json::to_string(&d).unwrap()).unwrap();
    assert_eq!(back, d);
    let bad = r#"{"lang":"en","text":"x","cites":[],"extra":1}"#;
    assert!(serde_json::from_str::<CommentDraft>(bad).is_err());
}

#[test]
fn number_matches_direct() {
    let n = num("44,32", Lang::Tr);
    assert!(number_matches(&n, 44.3187, None));
    assert!(number_matches(&n, -44.3187, None));
    assert!(!number_matches(&n, 44.3149, None));
    assert!(!number_matches(&n, f64::NAN, None));
}

// ---------------------------------------------------------------- advice in both languages

#[test]
fn advice_scans_both_languages() {
    let s: Vec<String> = vec![];
    assert_eq!(advice_hits("Bence buy fırsatı", &s), vec!["fırsatı", "buy"]);
    assert_eq!(advice_hits("Time to AL now", &s), vec!["al"]);
    assert!(advice_hits("Strong alım and satış flows", &s).is_empty());
    assert_eq!(
        codes(validate(
            &tr("Şimdi buy zamanı."),
            &ev(&[]),
            Strictness::Cites
        )),
        vec![(Check::Advice, "advice_word".to_owned())]
    );
    assert_eq!(
        codes(validate(&en("Time to al."), &ev(&[]), Strictness::Cites)),
        vec![(Check::Advice, "advice_word".to_owned())]
    );
    assert!(validate(
        &en("Heavy alım and satış today."),
        &ev(&[]),
        Strictness::Cites
    )
    .is_ok());
}

// ---------------------------------------------------------------- direction

fn ev_dir(dirs: &[(&str, Direction)]) -> Evidence {
    let mut e = ev(&[]);
    e.directions = dirs
        .iter()
        .map(|(id, d)| CitedDirection {
            id: (*id).to_owned(),
            direction: *d,
        })
        .collect();
    e
}

fn dir_codes(d: &CommentDraft, e: &Evidence) -> Vec<(Check, String)> {
    codes(validate(d, e, Strictness::CitesNumbers))
}

#[test]
fn direction_matching_text_passes() {
    let e = ev_dir(&[("f1", Direction::Up)]);
    assert!(dir_codes(&tr("SYN03 yükseldi."), &e).is_empty());
    assert!(dir_codes(&en("SYN03 rose."), &e).is_empty());
    let e = ev_dir(&[("f1", Direction::Down)]);
    assert!(dir_codes(&tr("Fiyat geriledi ve aşağı indi."), &e).is_empty());
    assert!(dir_codes(&en("The price fell."), &e).is_empty());
}

#[test]
fn opposite_direction_is_rejected_tr() {
    let e = ev_dir(&[("f1", Direction::Up)]);
    let r = validate(
        &tr("Hacim sakin. SYN03 düştü."),
        &e,
        Strictness::CitesNumbers,
    )
    .unwrap_err();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].check, Check::Direction);
    assert_eq!(r[0].code, "wrong_direction");
    assert_eq!(r[0].token.as_deref(), Some("düştü"));
    assert_eq!(r[0].sentence, Some(1));
    let e = ev_dir(&[("f1", Direction::Down)]);
    assert_eq!(
        dir_codes(&tr("Fiyat yukarı tırmandı."), &e),
        vec![(Check::Direction, "wrong_direction".to_owned())]
    );
}

#[test]
fn opposite_direction_is_rejected_en() {
    let e = ev_dir(&[("f1", Direction::Down)]);
    assert_eq!(
        dir_codes(&en("The stock climbed higher."), &e),
        vec![(Check::Direction, "wrong_direction".to_owned())]
    );
    let e = ev_dir(&[("f1", Direction::Up)]);
    assert_eq!(
        dir_codes(&en("It dropped and lost ground."), &e),
        vec![(Check::Direction, "wrong_direction".to_owned())]
    );
}

#[test]
fn mixed_or_no_direction_passes() {
    let e = ev_dir(&[("f1", Direction::Up)]);
    assert!(dir_codes(&tr("Önce düştü, sonra yükseldi."), &e).is_empty());
    assert!(dir_codes(&tr("Fiyat yatay seyretti."), &e).is_empty());
    assert!(dir_codes(&en("It fell early, then rose."), &e).is_empty());
}

#[test]
fn mixed_or_neutral_evidence_skips_direction() {
    let up_down = ev_dir(&[("f1", Direction::Up), ("f2", Direction::Down)]);
    let mut d = tr("Fiyat düştü.");
    d.cites.push("f2".to_owned());
    assert!(dir_codes(&d, &up_down).is_empty());
    let neutral = ev_dir(&[("f1", Direction::Neutral)]);
    assert!(dir_codes(&tr("Fiyat düştü."), &neutral).is_empty());
    // a direction of an item that is not cited does not count
    let uncited = ev_dir(&[("f2", Direction::Up)]);
    assert!(dir_codes(&tr("Fiyat düştü."), &uncited).is_empty());
}

#[test]
fn direction_is_not_checked_at_cites_strictness() {
    let e = ev_dir(&[("f1", Direction::Up)]);
    assert!(validate(&tr("Fiyat düştü."), &e, Strictness::Cites).is_ok());
    assert!(validate(&tr("Fiyat düştü."), &e, Strictness::PerSentence).is_err());
}

#[test]
fn direction_words_suffix_rule() {
    let s: Vec<String> = vec![];
    let (u, d) = direction_words(
        "artık düşük hacim, düşünce, gerilim, inci, çıkarım",
        Lang::Tr,
        &s,
    );
    assert!(u.is_empty(), "{u:?}");
    assert!(d.is_empty(), "{d:?}");
    let (u, d) = direction_words("ARTTI, inen, indi, iniş, kaybetti, zayıfladı", Lang::Tr, &s);
    assert_eq!(u, vec!["arttı"]);
    // (M30 fix) "zayıfladı" is a stage word, not a direction.
    assert_eq!(d.len(), 4, "{d:?}");
    let (u, d) = direction_words("aşağı yönlü kırılım güçlendi", Lang::Tr, &s);
    assert!(u.is_empty(), "{u:?}");
    assert_eq!(d, vec!["aşağı"]);
    let (u, d) = direction_words("rising, gains; declined, loses", Lang::En, &s);
    assert_eq!(u.len(), 2);
    assert_eq!(d.len(), 2);
}

// ---------------------------------------------------------------- fail-closed, percent words

#[test]
fn percent_words_are_percent_signs() {
    let e = ev(&[0.0123]);
    assert!(ok(&tr("Getiri yüzde 1,23."), &e));
    assert!(ok(&tr("Getiri 1,23 puan."), &e));
    assert!(ok(&en("Return 1.23 percent."), &e));
    assert!(ok(&en("Return 1.23 per cent."), &e));
    assert!(ok(&en("Return 1.23 pct."), &e));
    assert_eq!(num("yüzde 1,23", Lang::Tr).kind, NumKind::Percent);
}

#[test]
fn licence_cannot_be_dodged_by_how_a_percent_is_written() {
    let e = ev_with(&[], &[0.0123]);
    for t in [
        "Getiri yüzde 1,23.",
        "Getiri 1,23 puan.",
        "Getiri 1,23.",
        "Getiri %1,23.",
    ] {
        assert_eq!(
            codes(validate(&tr(t), &e, Strictness::Cites)),
            vec![(Check::License, "licensed_value".to_owned())],
            "{t}"
        );
    }
    // a withheld percentage written as a ratio
    let e = ev_with(&[], &[12.5]);
    assert_eq!(
        codes(validate(&en("Share 0.125."), &e, Strictness::Cites)),
        vec![(Check::License, "licensed_value".to_owned())]
    );
    assert!(withheld_matches(&num("1,23", Lang::Tr), 0.0123));
    assert!(withheld_matches(&num("-1,23", Lang::Tr), 0.0123));
}

#[test]
fn overlong_number_is_rejected_not_skipped() {
    let long = format!("Değer 7,{}1 [1].", "1".repeat(30));
    for s in ALL {
        assert_eq!(
            codes(validate(&tr(&long), &ev(&[7.1]), s)),
            vec![(Check::Number, "unsupported_number".to_owned())],
            "{s:?}"
        );
    }
}

#[test]
fn overlong_number_is_still_licence_checked() {
    let e = ev_with(&[], &[123.456]);
    let t = format!("Fiyat 123,456{}1.", "0".repeat(30));
    assert_eq!(
        codes(validate(&tr(&t), &e, Strictness::Cites)),
        vec![(Check::License, "licensed_value".to_owned())]
    );
    // trailing zeros only: read exactly, still the withheld value
    let t = format!("Fiyat 123,456{}.", "0".repeat(30));
    assert_eq!(
        codes(validate(&tr(&t), &e, Strictness::Cites)),
        vec![(Check::License, "licensed_value".to_owned())]
    );
}

// ------------------------------------------------- R5B/M28 usability (ids, flow nouns)

#[test]
fn ids_named_in_the_text_carry_no_number_and_no_direction() {
    let mut e = ev_dir(&[("cap.SYN02.breakout.down.151", Direction::Down)]);
    e.known_ids.insert("cap.SYN02.breakout.down.151".to_owned());
    e.known_ids.insert("cap.SYN02.breakout.up.88".to_owned());
    let mut d = en("cap.SYN02.breakout.down.151 [1] shows the price falling; cap.SYN02.breakout.up.88 is older.");
    d.cites = vec!["cap.SYN02.breakout.down.151".to_owned()];
    for s in ALL {
        assert_eq!(validate(&d, &e, s), Ok(()), "{s:?}");
    }
    // a real number next to the id is still checked
    let mut d2 = d.clone();
    d2.text = "cap.SYN02.breakout.down.151 [1] fell 151 points.".to_owned();
    assert_eq!(
        codes(validate(&d2, &e, Strictness::CitesNumbers)),
        vec![(Check::Number, "unsupported_number".to_owned())]
    );
}

#[test]
fn short_or_numeric_ids_never_mask_numbers() {
    let mut e = ev(&[]);
    e.known_ids.insert("12".to_owned());
    let d = tr("SYN03 12 puan değişti [1].");
    assert_eq!(
        codes(validate(&d, &e, Strictness::CitesNumbers)),
        vec![(Check::Number, "unsupported_number".to_owned())]
    );
}

#[test]
fn money_outflow_is_not_an_up_word() {
    let (up, down) = direction_words("yabancı para çıkışı sürdü, fiyat düştü", Lang::Tr, &[]);
    assert!(up.is_empty(), "{up:?}");
    assert_eq!(down, vec!["düştü".to_owned()]);
    // the verb still is
    let (up, _) = direction_words("fiyat yukarı çıktı", Lang::Tr, &[]);
    assert_eq!(up.len(), 2, "{up:?}");
}

// --------------------------------------------- R5B audit B1/B4 (masks, raw scan, words)

#[test]
fn out_of_range_times_and_dates_are_numbers() {
    assert!(!vals("saat 52:87", Lang::Tr).is_empty());
    assert!(!vals("47 Kas", Lang::Tr).is_empty());
    assert!(!vals("52.87.2026", Lang::Tr).is_empty());
    // real ones stay masked
    assert!(vals("saat 14:30", Lang::Tr).is_empty());
    assert!(vals("21.11.2026", Lang::Tr).is_empty());
}

#[test]
fn a_withheld_value_cannot_hide_in_a_time_a_date_or_words() {
    let e = ev_with(&[], &[5.23]);
    for text in [
        "Ortalama (5:23) [1].",
        "Ortalama 5/23 [1].",
        "Ortalama beş virgül yirmi üç [1].",
    ] {
        let r = codes(validate(&tr(text), &e, Strictness::Cites));
        assert!(
            r.iter()
                .any(|(c, k)| *c == Check::License || k == "spelled_number"),
            "{text}: {r:?}"
        );
    }
    // a decimal in words is refused for every writer (no oracle on what is withheld)
    let en = codes(validate(
        &en("Average five point two three [1]."),
        &ev(&[]),
        Strictness::Cites,
    ));
    assert_eq!(en, vec![(Check::Number, "spelled_number".to_owned())]);
    // a real clock time beside it is fine when nothing is withheld
    assert!(ok(&tr("Saat 14:30'da kırılım oldu [1]."), &ev(&[])));
}

#[test]
fn unmasked_numbers_read_times_and_dates_as_digits() {
    use qp_capsule::comment::unmasked_numbers;
    let v: Vec<f64> = unmasked_numbers("(5:23) ve 21.11.2026 [1]", Lang::Tr, &[])
        .iter()
        .map(|n| n.value)
        .collect();
    assert!(v.contains(&5.0) && v.contains(&23.0), "{v:?}");
    assert!(unmasked_numbers("[1] [2, 3]", Lang::Tr, &[]).is_empty());
}

#[test]
fn numbers_in_words_are_caught_in_either_casing() {
    use qp_capsule::comment::{number_word_run, spelled_decimal};
    for t in [
        "BEŞ VİRGÜL YİRMİ ÜÇ",
        "BEŞ VIRGÜL YIRMI ÜÇ",
        "on beş buçuk",
        "five dot two three",
        "Five Comma Two",
    ] {
        assert!(spelled_decimal(t, Lang::Tr).is_some(), "{t}");
    }
    assert!(number_word_run("ortalama beş yirmi üç").is_some());
    assert!(number_word_run("Five Twenty Three").is_some());
    assert!(number_word_run("bir hikâye doğdu").is_none());
}
