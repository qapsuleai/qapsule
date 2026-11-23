//! The compiler's design call points through the public design functions: feed face
//! headlines (every face design), open capsule text fields (`ask/…`, `open.*`), glossary
//! terms (voice terms and a design's own entries) and story charts (every story design).

use qp_grammar::open::nobra::ASKS;
use qp_grammar::open::{self, Check, DeviceCheck, OpenCtx, OpenTap, TextInput};
use qp_grammar::stories::{self, StoryInput};
use qp_grammar::{depth, faces, DataProvider as _, GrammarError};
use qp_grammar::{CompileOptions, MemProvider};
use qp_schema::capsule::{LedgerStat, Trust, Validity};
use qp_schema::protocol::{Language, OpenDepth, StoryRow};
use qp_schema::scene::SceneDesc;

use crate::fixture::{self, compile_in, field_text, look, looks, shape, texts};
use crate::snap::{debug_digest, err_line, scene_digest, Book};
use crate::support::view;

const W: u32 = 390;

/// A face scene of `finding` by design `face` in `look_name`, compiled with the face set.
pub fn face_scene(
    p: &MemProvider,
    face: &str,
    look_name: &str,
    finding: &str,
) -> Result<SceneDesc, GrammarError> {
    let l = look(look_name);
    let v = faces::face_view_in(
        l,
        Some(face),
        finding,
        120,
        p,
        &format!("capsule-{finding}"),
    )?;
    let h = faces::height_for(l, Some(face), p.voice(finding).is_some());
    compile_in(&v, p, look_name, W, h, Some(face))
}

#[test]
fn charac_face_headlines() {
    let mut b = Book::new("faces");
    let tr = fixture::provider(Language::Tr);
    let en = fixture::provider(Language::En);
    let mut faces_list = faces::names();
    faces_list.push("card");
    for face in &faces_list {
        for look_name in looks() {
            for f in [
                "fnd-brk",
                "fnd-parts",
                "fnd-sq",
                "fnd-real",
                "fnd-lock",
                "fnd-rsi",
            ] {
                let all_looks = matches!(f, "fnd-brk" | "fnd-parts");
                if !all_looks && !matches!(look_name, "r4" | "tavek") {
                    continue;
                }
                let case = format!("{face}.{look_name}.{f}");
                match face_scene(&tr, face, look_name, f) {
                    Ok(s) => {
                        let h = field_text(&s, "headline").unwrap_or_else(|| "-".into());
                        b.put(
                            &case,
                            &scene_digest(&s),
                            &format!("{} headline={h}", shape(&s)),
                        );
                    }
                    Err(e) => b.note(&case, &err_line(&e)),
                }
            }
        }
        for f in ["fnd-brk", "fnd-parts"] {
            let case = format!("{face}.r4.{f}.en");
            match face_scene(&en, face, "r4", f) {
                Ok(s) => {
                    let h = field_text(&s, "headline").unwrap_or_else(|| "-".into());
                    b.put(
                        &case,
                        &scene_digest(&s),
                        &format!("{} headline={h}", shape(&s)),
                    );
                }
                Err(e) => b.note(&case, &err_line(&e)),
            }
        }
    }
    // The templates themselves, and the face text a compile takes (`headline_text`).
    for face in &faces_list {
        let t = faces::headline_template(Some(face)).unwrap();
        for (n, hp) in [
            ("tr", fixture::parts(Language::Tr, false)),
            ("en", fixture::parts(Language::En, false)),
            ("tr.redacted", fixture::parts(Language::Tr, true)),
        ] {
            b.note(
                &format!("template.{face}.{n}"),
                &t.map_or_else(|| "none".to_owned(), |t| t(&hp)),
            );
        }
        for f in ["fnd-brk", "fnd-parts", "fnd-sq", "fnd-rsi", "nope"] {
            b.note(
                &format!("headline_text.{face}.{f}"),
                &format!("{:?}", faces::headline_text(t, f, &tr)),
            );
        }
        for look_name in looks() {
            let l = look(look_name);
            b.note(
                &format!("height.{face}.{look_name}"),
                &format!(
                    "{} {} voiced={}",
                    faces::height_in(l, Some(face)),
                    faces::height(Some(face)),
                    faces::height_for(l, Some(face), true)
                ),
            );
        }
    }
    b.note(
        "unknown",
        &err_line(&faces::by_name(Some("nope")).unwrap_err()),
    );
    b.note(
        "registry",
        &format!(
            "{:?} default={} presented={}",
            faces::names(),
            faces::DEFAULT_FACE,
            faces::PRESENTED_FACE
        ),
    );
    b.check();
}

fn open_scene(
    p: &MemProvider,
    name: &str,
    look_name: &str,
    finding: &str,
    ask: Option<&str>,
    check: Option<Check>,
    lang: Language,
) -> Result<SceneDesc, GrammarError> {
    let l = look(look_name);
    let h = open::height_in(l, Some(name)).unwrap_or(640);
    let ctx = OpenCtx {
        ask,
        lang,
        check,
        frame: [W, h],
    };
    let v = open::capsule_view(
        Some(name),
        l,
        finding,
        120,
        p,
        &format!("capsule-{finding}"),
        &ctx,
    )?;
    compile_in(&v, p, look_name, W, h, None)
}

fn open_line(b: &mut Book, case: &str, r: Result<SceneDesc, GrammarError>) {
    match r {
        Ok(s) => b.put(case, &scene_digest(&s), &shape(&s)),
        Err(e) => b.note(case, &err_line(&e)),
    }
}

#[test]
fn charac_open_designs() {
    let mut b = Book::new("open");
    let tr = fixture::provider(Language::Tr);
    let en = fixture::provider(Language::En);
    for name in open::names() {
        for look_name in looks() {
            for f in [
                "fnd-brk",
                "fnd-parts",
                "fnd-sq",
                "fnd-lock",
                "fnd-div",
                "fnd-rsi",
            ] {
                if f != "fnd-brk" && !matches!(look_name, "r4" | "tavek") {
                    continue;
                }
                let r = open_scene(&tr, name, look_name, f, None, None, Language::Tr);
                open_line(&mut b, &format!("{name}.{look_name}.{f}"), r);
            }
        }
        let r = open_scene(&en, name, "r4", "fnd-brk", None, None, Language::En);
        open_line(&mut b, &format!("{name}.r4.fnd-brk.en"), r);
        let dev = tr.clone().with_check("fnd-brk", DeviceCheck::Device);
        let r = open_scene(&dev, name, "r4", "fnd-brk", None, None, Language::Tr);
        open_line(&mut b, &format!("{name}.r4.fnd-brk.device"), r);
        b.note(
            &format!("{name}.meta"),
            &format!(
                "styled={} hint={:?} height={:?} scene_text={:?}",
                open::is_styled(Some(name)),
                open::hint(Some(name), OpenDepth::Capsule, Language::Tr),
                open::height_in(look("r4"), Some(name)),
                open::by_name(Some(name)).scene_text.map(|f| f()),
            ),
        );
    }
    // nobra: every question opened, every device check.
    for ask in ASKS {
        let r = open_scene(&tr, "nobra", "r4", "fnd-brk", Some(ask), None, Language::Tr);
        open_line(&mut b, &format!("nobra.ask.{ask}"), r);
    }
    for c in Check::ALL {
        let r = open_scene(&tr, "nobra", "r4", "fnd-brk", Some("who"), c, Language::Tr);
        open_line(&mut b, &format!("nobra.check.{}", Check::name(c)), r);
    }
    // The text column (tesir) and the text rows (nobra).
    let facts = fixture::facts();
    let v = fixture::voice(Language::Tr);
    for (n, voice, fi, check) in [
        ("voiced", Some(&v), 0, None),
        ("plain", None, 0, Some(DeviceCheck::Device)),
        ("locked", None, 3, None),
    ] {
        let t = TextInput {
            lang: Language::Tr,
            headline: "SAMPLE",
            voice,
            facts: std::slice::from_ref(&facts[fi]),
            trust: Trust::Server,
            check,
            ledger: LedgerStat {
                n: 30,
                realized: 18,
            },
            validity: Validity::Valid,
        };
        for name in open::names() {
            if let Some(col) = open::by_name(Some(name)).text {
                b.note(&format!("text.{name}.{n}"), &col(&t));
            }
        }
    }
    for item in fixture::feed_items().iter().take(3) {
        for lang in [Language::Tr, Language::En] {
            let rows = open::text_rows(Some("nobra"), OpenDepth::Capsule, item, lang);
            b.put(
                &format!("text_rows.{}.{lang:?}", item.finding),
                &debug_digest(&rows),
                &format!("{} rows", rows.len()),
            );
        }
    }
    // A feed item names no evidence families: with more than one confirmation the rows
    // count them in words (`families_of`'s "independent families" branch).
    for conf in [2, 3] {
        let mut item = fixture::feed_items()[0].clone();
        item.confirmations = conf;
        for lang in [Language::Tr, Language::En] {
            let rows = open::text_rows(Some("nobra"), OpenDepth::Capsule, &item, lang);
            let counted: Vec<String> = rows
                .iter()
                .map(|r| format!("{r:?}"))
                .filter(|r| r.contains("bağımsız kanıt") || r.contains("independent evidence"))
                .collect();
            b.put(
                &format!("text_rows.confirmations{conf}.{lang:?}"),
                &debug_digest(&rows),
                &format!("{} rows; {}", rows.len(), counted.join(" ")),
            );
        }
    }
    b.check();
}

#[test]
fn charac_open_fields_and_terms() {
    let mut b = Book::new("open-fields");
    let tr = fixture::provider(Language::Tr);
    // Every text field the open designs' views read, resolved directly.
    let mut fields: Vec<String> = Vec::new();
    let mut layer_ids: Vec<String> = vec!["headline".into(), "ledger".into(), "detail".into()];
    for name in open::names() {
        let ctx = OpenCtx {
            ask: Some("who"),
            lang: Language::Tr,
            check: None,
            frame: [W, 640],
        };
        let v = open::capsule_view(Some(name), look("r4"), "fnd-brk", 120, &tr, "v", &ctx).unwrap();
        for l in &v.layers {
            if let Some(t) = &l.encoding.text {
                if !fields.contains(t) {
                    fields.push(t.clone());
                }
            }
            if let Some(id) = &l.id {
                if !layer_ids.contains(id) {
                    layer_ids.push(id.clone());
                }
            }
        }
    }
    for f in ["fnd-brk", "fnd-parts", "fnd-lock", "fnd-rsi"] {
        for k in &fields {
            let a = open::field_text(k, f, &tr);
            let t = open::text(k, f, &tr);
            if a.is_some() || t.is_some() {
                b.note(&format!("field.{f}.{k}"), &format!("ask={a:?} open={t:?}"));
            }
        }
        for id in &layer_ids {
            b.note(
                &format!("term.{f}.{id}"),
                &format!(
                    "voice_terms_in={} layer_term={:?}",
                    open::voice_terms_in(id),
                    open::layer_term(id, f, &tr)
                ),
            );
        }
    }
    // A capsule text layer with voice terms, by layer id: the terms and their underlines.
    for id in [
        "detail",
        "headline",
        "ledger",
        "open-trust",
        "open-basis",
        "note",
    ] {
        for field in ["detail", "ledger", "open.trust"] {
            let v = view(&format!(
                r#"{{"id":"v","x":{{"kind":"time"}},"panels":[{{"id":0,"y":{{"kind":"linear"}}}}],
                "layers":[{{"id":"{id}","mark":"text","finding":"fnd-brk","panel":0,
                  "encoding":{{"x":{{"panel":0,"px":4}},"x2":{{"panel":1,"px":-4}},"y":{{"panel":0.2}},"text":"{field}"}}}}]}}"#
            ));
            let case = format!("terms.{id}.{field}");
            match compile_in(&v, &tr, "r4", W, 300, None) {
                Ok(s) => b.put(
                    &case,
                    &scene_digest(&s),
                    &format!("{:?} {}", s.terms, texts(&s).join(" / ")),
                ),
                Err(e) => b.note(&case, &err_line(&e)),
            }
        }
    }
    b.check();
}

fn story_scene(
    d: &stories::StoryDesign,
    look_name: &str,
    p: &MemProvider,
    inp: &StoryInput<'_>,
    h: u32,
) -> Result<SceneDesc, GrammarError> {
    let l = look(look_name);
    let v = depth::chart_view_in(l, "fnd-brk", p, "present-fnd-brk")?;
    match d.compile {
        Some(c) => {
            let mut o = CompileOptions::new(W, h);
            o.look = Some(look_name.into());
            c(&v, p, &o, inp)
        }
        None => {
            let ph = (d.price_height)(l, h);
            let s = compile_in(&v, p, look_name, W, ph, None)?;
            Ok((d.extend)(l, None, s, h, inp))
        }
    }
}

#[test]
fn charac_story_charts() {
    let mut b = Book::new("stories");
    let p = fixture::provider(Language::Tr);
    let rows = fixture::story_rows(9);
    let fact_of = |f: &str| f.ends_with('2').then(|| "f-brk".to_owned());
    let stamp = |r: &StoryRow| Some(format!("b{}", r.born_bar));
    for d in stories::STORIES {
        for look_name in looks() {
            for (n, focus, hide, lang, stamped) in [
                ("focus", Some("fnd-s7"), false, Language::Tr, false),
                ("nofocus.hide", None, true, Language::Tr, false),
                (
                    "closedfocus.en.stamp",
                    Some("fnd-s1"),
                    false,
                    Language::En,
                    true,
                ),
            ] {
                if n != "focus" && !matches!(look_name, "r4" | "tavek") {
                    continue;
                }
                let inp = StoryInput {
                    rows: &rows,
                    focus,
                    hide_closed: hide,
                    lang,
                    fact_of: &fact_of,
                    stamp_of: if stamped { Some(&stamp) } else { None },
                };
                for h in [420, 640] {
                    let case = format!("{}.{look_name}.{n}.{h}", d.name);
                    match story_scene(d, look_name, &p, &inp, h) {
                        Ok(s) => b.put(&case, &scene_digest(&s), &shape(&s)),
                        Err(e) => b.note(&case, &err_line(&e)),
                    }
                }
            }
            b.note(
                &format!("{}.{look_name}.price_height", d.name),
                &format!(
                    "{} {} {}",
                    (d.price_height)(look(look_name), 300),
                    (d.price_height)(look(look_name), 420),
                    (d.price_height)(look(look_name), 900)
                ),
            );
        }
        b.note(
            &format!("{}.meta", d.name),
            &format!(
                "note={} compiles={} tap(None,focus)={:?} tap(None,None)={:?}",
                d.note,
                d.compile.is_some(),
                (d.tap)(None, Some("fnd-s7")),
                (d.tap)(None, None)
            ),
        );
    }
    // No rows at all.
    for d in stories::STORIES {
        let inp = StoryInput {
            rows: &[],
            focus: None,
            hide_closed: false,
            lang: Language::Tr,
            fact_of: &|_| None,
            stamp_of: None,
        };
        let case = format!("{}.r4.empty", d.name);
        match story_scene(d, "r4", &p, &inp, 420) {
            Ok(s) => b.put(&case, &scene_digest(&s), &shape(&s)),
            Err(e) => b.note(&case, &err_line(&e)),
        }
    }
    b.note(
        "registry",
        &format!(
            "{:?} default={} presented={} by_name(None)={:?} by_name(r4)={:?}",
            stories::names(),
            stories::DEFAULT_STORY,
            stories::PRESENTED_STORY,
            stories::by_name(None).map(|d| d.name),
            stories::by_name(Some("r4")).map(|d| d.name)
        ),
    );
    b.note(
        "open.tap",
        &format!(
            "{:?} {:?}",
            open::tap(Some("nobra"), OpenDepth::Capsule, None, None, None),
            open::tap(None, OpenDepth::Sentence, None, None, None) == OpenTap::Nothing
        ),
    );
    b.check();
}
