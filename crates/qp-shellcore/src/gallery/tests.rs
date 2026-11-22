use super::*;

fn layout() -> Layout {
    let cand = |id: &str| CandOut {
        id: id.to_owned(),
        label: id.to_owned(),
        content: "adım 120 / 2080 · x".to_owned(),
        shots: vec![],
    };
    Layout {
        lang: "tr".to_owned(),
        theme: "dark".to_owned(),
        moment: "adım 120 / 2080 · 2025-06-02 10:00".to_owned(),
        topics: vec![
            TopicOut {
                key: "voice".to_owned(),
                label: "Dil".to_owned(),
                hint: String::new(),
                cands: vec![cand("marla"), cand("selvi")],
            },
            TopicOut {
                key: "look".to_owned(),
                label: "Görünüş".to_owned(),
                hint: String::new(),
                cands: vec![cand("r4"), cand("ornek")],
            },
        ],
    }
}

#[test]
fn summary_is_deterministic_and_complete() {
    let mut g = GalleryCore::new("", "", "", "", "1763460000000");
    g.load(&serde_json::to_string(&layout()).unwrap()).unwrap();
    g.set_device("Mozilla/5.0 (iPhone; CPU iPhone OS 27_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.0 Mobile/15E148 Safari/604.1", 402.0, 874.0, 3.0);
    g.pick("voice", "marla");
    g.note("voice", "marla", "  kısa\nve net  ");
    g.note("look", "ornek", "sıcak");
    g.pick("look", "nope");
    let s = g.summary();
    assert_eq!(
        s,
        "R5 galeri · voice=marla (not: kısa ve net) · look=—\n\
         notlar: look/ornek: sıcak\n\
         an: adım 120 / 2080 · 2025-06-02 10:00 · tr · koyu\n\
         cihaz: iPhone iOS 27.0 · Safari · 402×874 @3 · tarih: 2025-11-18 13:00 (İstanbul)"
    );
    // Restored from the saved state: the same summary.
    let mut h = GalleryCore::new(&g.state(), "", "", "", "1763460000000");
    h.load(&serde_json::to_string(&layout()).unwrap()).unwrap();
    h.set_device("Mozilla/5.0 (iPhone; CPU iPhone OS 27_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/27.0 Mobile/15E148 Safari/604.1", 402.0, 874.0, 3.0);
    assert_eq!(h.summary(), s);
    // One pick per topic; a second tap takes it back.
    h.pick("voice", "selvi");
    assert_eq!(h.marks()["voice/selvi"], json!(true));
    assert_eq!(h.marks()["voice/marla"], json!(false));
    h.pick("voice", "selvi");
    assert_eq!(h.marks()["voice/selvi"], json!(false));
}

#[test]
fn pick_parameter_and_share_link() {
    let mut g = GalleryCore::new("", "en", "light", "voice.selvi,look.zzz,bad", "");
    g.load(&serde_json::to_string(&layout()).unwrap()).unwrap();
    assert_eq!(g.share(), "gallery.html?pick=voice.selvi");
    assert_eq!(g.layout_url(), "/gallery/layout?lang=en&theme=light");
    assert!(g
        .summary()
        .starts_with("R5 gallery · voice=selvi · look=—\nmoment: "));
    assert!(g.summary().ends_with("device: — · date: —"));
    g.toggle_lang();
    assert_eq!(g.lang(), "tr");
}

#[test]
fn notes_are_one_short_line() {
    assert_eq!(clean_note("a\tb\n\nc "), "a b c");
    assert_eq!(clean_note(&"x".repeat(500)).chars().count(), NOTE_MAX);
}

#[test]
fn candidates_follow_the_registry_and_finalists() {
    let ch = json!({"voice": ["r4", "marla", "selvi"]});
    let voice = TOPICS.iter().find(|t| t.key == "voice").unwrap();
    let story = TOPICS.iter().find(|t| t.key == "story").unwrap();
    assert_eq!(candidates(voice, &ch).len(), 3);
    assert!(candidates(story, &ch).is_empty());
}

/// The host's build: every shown topic has at least two candidates, all with the same
/// content (moment, capsule, depth), and the images exist.
#[test]
fn build_shows_the_same_content_per_topic() {
    let b = build("tr", "dark");
    let keys: Vec<&str> = b.layout.topics.iter().map(|t| t.key.as_str()).collect();
    for k in ["voice", "look", "feed_order", "feed_rule"] {
        assert!(keys.contains(&k), "{k} missing: {keys:?}");
    }
    for t in &b.layout.topics {
        assert!(t.cands.len() >= 2, "{}", t.key);
        let c0 = &t.cands[0].content;
        assert!(c0.contains("120"), "{c0}");
        for c in &t.cands {
            assert_eq!(&c.content, c0, "{} / {}", t.key, c.id);
            assert!(
                c.shots.iter().any(|s| s.kind == "img"),
                "{} / {}",
                t.key,
                c.id
            );
            for s in c.shots.iter().filter(|s| s.kind == "img") {
                let name = s
                    .src
                    .as_deref()
                    .unwrap()
                    .trim_start_matches("/gallery/img/");
                assert!(b.images[name].starts_with(b"\x89PNG"), "{name}");
            }
        }
    }
    // Each candidate's own setting shows: within a topic that paints the open capsule,
    // no two candidates have identical shots (voice: the title/texts; look: pixels).
    for t in b
        .layout
        .topics
        .iter()
        .filter(|t| t.key == "voice" || t.key == "look")
    {
        for (i, a) in t.cands.iter().enumerate() {
            for z in &t.cands[i + 1..] {
                let px = |c: &CandOut| -> Vec<Vec<u8>> {
                    c.shots
                        .iter()
                        .map(|s| match &s.src {
                            Some(src) => b.images[src.trim_start_matches("/gallery/img/")].clone(),
                            None => s.text.clone().unwrap_or_default().into_bytes(),
                        })
                        .collect()
                };
                assert_ne!(px(a), px(z), "{}: {} = {}", t.key, a.id, z.id);
            }
        }
    }
    // Deterministic: a second build is identical.
    let again = build("tr", "dark");
    assert_eq!(again.layout, b.layout);
    assert_eq!(again.images, b.images);
}
