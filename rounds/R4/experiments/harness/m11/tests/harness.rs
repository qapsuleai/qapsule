//! Harness self-tests (no reader is called): sheets are byte-identical per seed; the
//! measurement is deterministic and sees a style that does not separate realized from
//! failed as 0; prompts of every registered style pass the forbidden-word check; scoring
//! is a pure function of the raw files and applies the selection rule. Writes only under
//! Cargo's test temp directory.

use std::fs;
use std::path::{Path, PathBuf};

use m11::{blind, distinct, io, score, sheets};
use qp_grammar::glyph::outcome::{self, OutcomeStyle};
use qp_grammar::glyph::{Alphabet, GlyphBox};
use qp_grammar::Theme;
use qp_schema::capsule::{GlyphPhase, GlyphSpec};
use qp_schema::scene::Prim;

fn root(name: &str) -> PathBuf {
    let p = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&p);
    p
}

fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push((
                    p.strip_prefix(dir).unwrap().display().to_string(),
                    fs::read(&p).unwrap(),
                ));
            }
        }
    }
    out.sort();
    out
}

/// A test style that draws `failed` exactly like `realized` (no outcome distinction).
struct Same;

impl OutcomeStyle for Same {
    fn name(&self) -> &'static str {
        "kopya"
    }
    fn draw(&self, a: &dyn Alphabet, spec: &GlyphSpec, bx: GlyphBox, t: &Theme) -> Vec<Prim> {
        let mut s = spec.clone();
        if s.phase == GlyphPhase::Failed {
            s.phase = GlyphPhase::Realized;
        }
        a.draw(&s, bx, t)
    }
    fn legend(&self, _: &str) -> &'static str {
        "Köşe işareti kapandığını gösterir."
    }
}

fn r3() -> &'static dyn OutcomeStyle {
    outcome::by_name(m11::REFERENCE).unwrap()
}

#[test]
fn sheets_are_byte_identical_and_blind() {
    let (a, b) = (root("sa"), root("sb"));
    for r in [&a, &b] {
        let w = sheets::write_sheet(
            r3(),
            "siluet",
            7,
            blind::DEFAULT_SALT,
            &sheets::PERSONAS,
            &sheets::CONDITIONS,
            r,
        )
        .unwrap();
        assert_eq!(w.prompts.len(), 4);
        sheets::write_sheet(
            r3(),
            "rozet",
            7,
            blind::DEFAULT_SALT,
            &["F"],
            &["nolegend"],
            r,
        )
        .unwrap();
    }
    let (fa, fb) = (files(&a), files(&b));
    assert_eq!(fa, fb);
    // 2 png + 2 keys + key.json + 5 prompts
    assert_eq!(fa.len(), 10);
    for (name, bytes) in &fa {
        // No style name in any path a reader could see (screens, prompts).
        if name.contains("screens") || name.contains("prompts") {
            assert!(!name.contains("r3"), "{name}");
        }
        if name.contains("prompts") {
            let t = String::from_utf8(bytes.clone()).unwrap();
            assert!(blind::forbidden_hits(&t, &[]).is_empty(), "{name}");
        }
    }
    let idx: serde_json::Value =
        serde_json::from_slice(&fs::read(a.join("raw/key.json")).unwrap()).unwrap();
    assert_eq!(idx.as_object().unwrap().len(), 2);
    // Another seed changes the sheet.
    let c = root("sc");
    let w8 = sheets::write_sheet(
        r3(),
        "siluet",
        8,
        blind::DEFAULT_SALT,
        &["F"],
        &["nolegend"],
        &c,
    )
    .unwrap();
    let w7 = blind::code("r3", "siluet", 7, blind::DEFAULT_SALT);
    assert_ne!(
        fs::read(&w8.image).unwrap(),
        fs::read(a.join(format!("raw/screens/{w7}.png"))).unwrap()
    );
}

#[test]
fn every_registered_prompt_is_clean() {
    let r = root("clean");
    for name in outcome::names() {
        let st = outcome::by_name(name).unwrap();
        for al in m11::alphabets::ALPHABETS {
            sheets::write_sheet(
                st,
                al,
                1,
                blind::DEFAULT_SALT,
                &sheets::PERSONAS,
                &sheets::CONDITIONS,
                &r,
            )
            .unwrap();
        }
    }
}

#[test]
fn forbidden_legend_is_refused() {
    struct Leaky;
    impl OutcomeStyle for Leaky {
        fn name(&self) -> &'static str {
            "leaky"
        }
        fn draw(&self, a: &dyn Alphabet, s: &GlyphSpec, bx: GlyphBox, t: &Theme) -> Vec<Prim> {
            a.draw(s, bx, t)
        }
        fn legend(&self, _: &str) -> &'static str {
            "Bu varyant onay işaretini büyütür."
        }
    }
    let r = root("leaky");
    let e = sheets::write_sheet(&Leaky, "siluet", 1, 11, &["F"], &["legend"], &r).unwrap_err();
    assert!(e.contains("varyant"), "{e}");
}

#[test]
fn measure_is_deterministic_and_sees_no_distinction_as_zero() {
    let a = io::json(&distinct::measure(r3()).unwrap()).unwrap();
    let b = io::json(&distinct::measure(r3()).unwrap()).unwrap();
    assert_eq!(a, b);
    let r = distinct::measure(r3()).unwrap();
    assert_eq!(r.alphabets.len(), 2);
    for ar in r.alphabets.values() {
        assert_eq!(ar.conditions.len(), 30);
        assert_eq!(ar.out_of_box_prims, 0);
        assert!(ar.main.pair.d > 0.0);
    }
    let z = distinct::measure(&Same).unwrap();
    assert!(z.main.pair.d.abs() < 1e-12);
}

fn fake_reader(root: &Path, code: &str, persona: &str, cond: &str, k: usize, answer: &str) {
    let run = root
        .join("raw/readers")
        .join(format!("{persona}_{cond}_{code}_r{k}"));
    fs::create_dir_all(&run).unwrap();
    fs::write(
        run.join("meta.json"),
        format!(
            "{{\"experiment\": \"m11\", \"code\": \"{code}\", \"persona\": \"{persona}\", \"condition\": \"{cond}\", \"seed\": {k}, \"reader\": \"r{k}\", \"model\": \"test\", \"alphabet\": \"siluet\"}}"
        ),
    )
    .unwrap();
    let wrapped = serde_json::json!({"type": "result", "result": answer});
    fs::write(run.join("answer.json"), wrapped.to_string()).unwrap();
}

fn perfect(key: &sheets::Key) -> String {
    let m: serde_json::Map<String, serde_json::Value> = key
        .items
        .iter()
        .map(|i| {
            let a = match i.outcome {
                sheets::Outcome::Realized => "gerçekleşti",
                sheets::Outcome::Failed => "tutmadı",
                sheets::Outcome::Open => "açık",
            };
            (i.label.clone(), a.into())
        })
        .collect();
    format!("```json\n{}\n```", serde_json::Value::Object(m))
}

#[test]
fn score_is_pure_and_applies_the_rule() {
    let r = root("score");
    distinct::run(r3(), &r).unwrap();
    distinct::run(&Same, &r).unwrap();
    let key = |code: &str| -> sheets::Key {
        serde_json::from_slice(&fs::read(r.join(format!("raw/keys/{code}.json"))).unwrap()).unwrap()
    };
    // Reference: F perfect, H all "bilmiyorum".
    let c = sheets::write_sheet(r3(), "siluet", 1, 11, &sheets::PERSONAS, &["nolegend"], &r)
        .unwrap()
        .code;
    fake_reader(&r, &c, "F", "nolegend", 1, &perfect(&key(&c)));
    fake_reader(&r, &c, "H", "nolegend", 1, "{\"A1\": \"bilmiyorum\"}");
    // A style with no distinction: perfect readers, but eliminated by rule 1.
    let c2 = sheets::write_sheet(&Same, "siluet", 1, 11, &sheets::PERSONAS, &["nolegend"], &r)
        .unwrap()
        .code;
    fake_reader(&r, &c2, "F", "nolegend", 1, &perfect(&key(&c2)));
    fake_reader(&r, &c2, "H", "nolegend", 1, &perfect(&key(&c2)));
    let s = score::run(&r).unwrap();
    let rf = &s.styles["r3"];
    assert!((rf.cells["F/nolegend"].accuracy - 1.0).abs() < 1e-12);
    assert!(rf.cells["H/nolegend"].accuracy.abs() < 1e-12);
    assert!((rf.weighted["nolegend"] - 0.6667).abs() < 1e-9);
    assert!(s.styles["kopya"]
        .excluded
        .as_deref()
        .unwrap()
        .starts_with("rule 1"));
    assert!(s.selected.is_none());
    let first = fs::read(r.join("raw/scores.json")).unwrap();
    score::run(&r).unwrap();
    assert_eq!(first, fs::read(r.join("raw/scores.json")).unwrap());

    // Two eligible styles (distinct reports made above the reference): rule 2 and rule 3.
    let rep: distinct::Report =
        serde_json::from_slice(&fs::read(r.join("raw/r3/distinct.json")).unwrap()).unwrap();
    let fake = |name: &str, bump: f64, acc_f: &str| {
        let mut x = rep.clone();
        x.style = name.into();
        x.main.pair.d += bump;
        for ar in x.alphabets.values_mut() {
            ar.main.pair.d += bump;
        }
        io::write_json(&r.join(format!("raw/{name}/distinct.json")), &x).unwrap();
        let k = sheets::Key {
            code: format!("c0{name}"),
            style: name.into(),
            ..key(&c)
        };
        io::write_json(&r.join(format!("raw/keys/{}.json", k.code)), &k).unwrap();
        fake_reader(&r, &k.code, "F", "nolegend", 1, acc_f);
        fake_reader(&r, &k.code, "H", "nolegend", 1, &perfect(&k));
    };
    let p = perfect(&key(&c));
    fake("ba", 0.5, &p);
    fake("bb", 1.0, &p); // same accuracy → tie → higher ΔE wins
    let s = score::run(&r).unwrap();
    assert_eq!(s.selected.as_deref(), Some("bb"), "{:?}", s.rule);
    assert!(s.rule.iter().any(|l| l.starts_with("rule 3")));
    // Make `ba`'s F reader clearly better than `bb`'s: rule 2.
    fs::remove_dir_all(r.join("raw/readers/F_nolegend_c0bb_r1")).unwrap();
    fake_reader(&r, "c0bb", "F", "nolegend", 1, "{}");
    let s = score::run(&r).unwrap();
    assert_eq!(s.selected.as_deref(), Some("ba"), "{:?}", s.rule);
    assert!(s.rule.iter().any(|l| l.starts_with("rule 2")));
}
