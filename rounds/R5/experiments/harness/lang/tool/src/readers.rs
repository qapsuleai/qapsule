//! Blind readers and critics (SPEC §3–§4): reader-set selection, prompt pages, the
//! deterministic scorer, and the critic pair plan and scorer. No language name ever
//! reaches a prompt: languages are blind codes; `key.json` (code → name) stays in the run
//! directory and is never copied to a reader's directory.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::key::{key, Key};
use crate::lang::{Lang, Language, Surfaces};
use crate::situ::Situation;
use crate::stats::{blind_code, bradley_terry, bt_bootstrap, corr, wilson, Duel, Rng};
use crate::text::{has_close, numbers, sentences};

/// Reader-set criteria (SPEC §3.2), in order. Each picks the first situation that
/// matches and was not picked yet; `None` fields match anything.
pub struct Crit {
    /// Label.
    pub label: &'static str,
    /// World.
    pub world: Option<&'static str>,
    /// Detector.
    pub detector: Option<&'static str>,
    /// Direction.
    pub direction: Option<&'static str>,
    /// Stage(s).
    pub stages: &'static [&'static str],
    /// Validity.
    pub validity: Option<&'static str>,
    /// Redacted reader.
    pub redacted: bool,
    /// Ledger bucket.
    pub ledger: Option<&'static str>,
}

/// SPEC §3.2 reader set S12.
pub const S12: &[Crit] = &[
    Crit {
        label: "kırılım yukarı, doğdu, geçerli",
        world: Some("syn-d"),
        detector: Some("breakout.range"),
        direction: Some("up"),
        stages: &["born"],
        validity: Some("valid"),
        redacted: false,
        ledger: None,
    },
    Crit {
        label: "kırılım aşağı, açık, geçerli",
        world: Some("syn-d"),
        detector: Some("breakout.range"),
        direction: Some("down"),
        stages: &[],
        validity: Some("valid"),
        redacted: false,
        ledger: None,
    },
    Crit {
        label: "aşırılık (RSI), artık geçerli değil",
        world: Some("syn-d"),
        detector: Some("extreme.rsi"),
        direction: None,
        stages: &[],
        validity: Some("no_longer"),
        redacted: false,
        ledger: None,
    },
    Crit {
        label: "sıkışma, sürüyor",
        world: Some("syn-d"),
        detector: Some("squeeze.bandwidth"),
        direction: None,
        stages: &[],
        validity: Some("valid"),
        redacted: false,
        ledger: None,
    },
    Crit {
        label: "ortalama dizilimi, kapandı: gerçekleşti",
        world: Some("syn-d"),
        detector: Some("continuation.ma_stack"),
        direction: None,
        stages: &["realized"],
        validity: None,
        redacted: false,
        ledger: None,
    },
    Crit {
        label: "uyumsuzluk dönüşü, kapandı: tutmadı",
        world: Some("syn-d"),
        detector: Some("reversal.rsi_divergence"),
        direction: None,
        stages: &["invalidated"],
        validity: None,
        redacted: false,
        ledger: None,
    },
    Crit {
        label: "hacim, kapandı: süresi doldu",
        world: Some("syn-d"),
        detector: Some("flow.volume_surge"),
        direction: None,
        stages: &["expired"],
        validity: None,
        redacted: false,
        ledger: None,
    },
    Crit {
        label: "geri çekildi (kurgu)",
        world: Some("syn-d"),
        detector: None,
        direction: None,
        stages: &["retracted"],
        validity: None,
        redacted: false,
        ledger: None,
    },
    Crit {
        label: "kurumsal akış, yetkisiz okur",
        world: None,
        detector: Some("flow.institutional"),
        direction: None,
        stages: &[],
        validity: None,
        redacted: true,
        ledger: None,
    },
    Crit {
        label: "az örnekli karne",
        world: Some("syn-d"),
        detector: None,
        direction: None,
        stages: &["born"],
        validity: Some("valid"),
        redacted: false,
        ledger: Some("thin"),
    },
    Crit {
        label: "saatlik, geçerli",
        world: Some("syn-h"),
        detector: None,
        direction: None,
        stages: &["born", "strengthened", "confirmed"],
        validity: Some("valid"),
        redacted: false,
        ledger: None,
    },
    Crit {
        label: "teyit edilmiş (iki aile), açık",
        world: Some("syn-d"),
        detector: None,
        direction: None,
        stages: &["confirmed"],
        validity: None,
        redacted: false,
        ledger: None,
    },
];

fn matches(c: &Crit, s: &Situation, k: &Key) -> bool {
    c.world.is_none_or(|w| w == s.world)
        && c.detector.is_none_or(|d| d == s.detector)
        && c.direction.is_none_or(|d| d == k.direction)
        && (c.stages.is_empty() || c.stages.contains(&k.stage.as_str()))
        && c.validity.is_none_or(|v| v == k.validity)
        && c.redacted == s.redacted
        && c.ledger.is_none_or(|l| l == s.ledger_bucket())
}

/// The reader set: `(criterion label, situation index)`; a criterion with no match is
/// relaxed (stage, validity, ledger dropped) and marked `*`.
#[must_use]
pub fn reader_set(sits: &[Situation]) -> Vec<(String, usize)> {
    let keys: Vec<Key> = sits.iter().map(key).collect();
    let mut out: Vec<(String, usize)> = Vec::new();
    for c in S12 {
        let taken = |i: usize, out: &Vec<(String, usize)>| out.iter().any(|(_, j)| *j == i);
        // Prefer a stated ledger (counts, then thin, then empty) so the ledger question
        // discriminates; then generation order.
        let rank = |i: usize| match sits[i].ledger_bucket() {
            "counts" => 0,
            "thin" => 1,
            _ => 2,
        };
        // …and a detector the set does not hold yet (spread over the seven).
        let used = |i: usize| {
            out.iter()
                .filter(|(_, j)| sits[*j].detector == sits[i].detector)
                .count()
        };
        let mut order: Vec<usize> = (0..sits.len()).collect();
        order.sort_by_key(|&i| (used(i), rank(i), i));
        let pick = order
            .iter()
            .copied()
            .find(|&i| !taken(i, &out) && matches(c, &sits[i], &keys[i]));
        let pick = pick.map(|i| (c.label.to_owned(), i)).or_else(|| {
            let r = Crit {
                stages: &[],
                validity: None,
                ledger: None,
                ..*c
            };
            order
                .iter()
                .copied()
                .find(|&i| !taken(i, &out) && matches(&r, &sits[i], &keys[i]))
                .map(|i| (format!("{} *", c.label), i))
        });
        if let Some(p) = pick {
            out.push(p);
        }
    }
    out
}

/// Persona prompt texts (SPEC §3.1), verbatim.
#[must_use]
pub fn persona(code: &str, lang: Lang) -> &'static str {
    match (code, lang) {
        ("F", Lang::Tr) => "Yirmi yaşındasın. Hisse senedi alıp satıyorsun ama teknik analiz bilmiyorsun; grafik, indikatör, RSI, ortalama gibi terimler sana bir şey ifade etmiyor. Telefonuna bir bildirim geldi; her metne yalnızca birkaç saniye bakacaksın. İlk bakışta ne anladığını cevapla; geri dönüp uzun uzun inceleme.",
        ("H", Lang::Tr) => "Kırk beş yaşındasın. Yıllardır borsada işlem yapıyorsun; terminal ekranlarına, teknik analize ve indikatörlere alışkınsın; ayrıntıya güvenirsin. Telefonunda bir piyasa uygulamasına bakıyorsun. Okuyup cevapla.",
        ("F", Lang::En) => "You are twenty. You buy and sell shares but know no technical analysis; words like chart, indicator, RSI or moving average mean nothing to you. A notification arrived on your phone; you will glance at each text for only a few seconds. Answer with what you understood at first sight; do not go back and study it.",
        (_, Lang::En) => "You are forty-five. You have traded for years; you are used to terminal screens, technical analysis and indicators, and you trust detail. You are looking at a market app on your phone. Read and answer.",
        (_, Lang::Tr) => "",
    }
}

/// S1 options (event categories), SPEC §3.3.
pub const S1: &[(&str, &str, &str)] = &[
    ("A", "breakout_up", "Fiyat bir sınırın üstüne çıktı"),
    ("B", "breakout_down", "Fiyat bir sınırın altına indi"),
    ("C", "reversal_up", "Düşüş yukarı dönüyor olabilir"),
    ("D", "reversal_down", "Yükseliş aşağı dönüyor olabilir"),
    ("E", "trend_up", "Yükseliş sürüyor"),
    ("F", "trend_down", "Düşüş sürüyor"),
    (
        "G",
        "squeeze",
        "Fiyat dar bir aralığa sıkıştı, yön belli değil",
    ),
    ("H", "extreme_up", "Fiyat kısa sürede çok yükseldi"),
    ("I", "extreme_down", "Fiyat kısa sürede çok düştü"),
    ("J", "flow_in", "Alışılmadık güçlü alım (para girişi) oldu"),
    (
        "K",
        "flow_out",
        "Alışılmadık güçlü satış (para çıkışı) oldu",
    ),
    ("L", "", "Bilmiyorum"),
];
/// S3 options.
pub const S3: &[(&str, &str)] = &[
    ("A", "Bugün"),
    ("B", "Dün"),
    ("C", "2–4 işlem günü önce"),
    ("D", "Bir haftaya yakın ya da daha önce"),
    ("E", "Birkaç saat önce"),
    ("F", "Bilmiyorum"),
];
/// S4 options.
pub const S4: &[(&str, &str, &str)] = &[
    ("A", "valid", "Evet, hâlâ geçerli"),
    (
        "B",
        "no_longer",
        "Hayır, artık geçerli değil (ama henüz kapanmadı)",
    ),
    ("C", "closed:realized", "Kapandı: gerçekleşti"),
    ("D", "closed:failed", "Kapandı: tutmadı"),
    ("E", "closed:expired", "Kapandı: süresi doldu"),
    ("F", "closed:retracted", "Geri çekildi (veri hatalıydı)"),
    ("G", "", "Bilmiyorum"),
];
/// S6 options.
pub const S6: &[(&str, &str)] = &[
    ("A", "Almamı"),
    ("B", "Satmamı"),
    ("C", "Beklememi"),
    ("D", "Hiçbir şey; yalnız bilgi veriyor"),
    ("E", "Bilmiyorum"),
];

/// The two exposure conditions (SPEC §3.4).
pub const CONDS: [&str; 2] = ["yuz", "acik"];

/// The texts of a situation in a condition, neutral plain text.
#[must_use]
pub fn shown(s: &Surfaces, cond: &str) -> String {
    let mut t = String::new();
    if !s.ledger.is_empty() {
        let _ = writeln!(t, "{}", s.ledger);
    }
    let _ = writeln!(t, "{}", s.headline);
    if cond == "acik" {
        let _ = writeln!(t, "{}", s.detail);
        for g in &s.glossary {
            let _ = writeln!(t, "({}: {})", g.term, g.text);
        }
    }
    t
}

fn options<T: AsRef<str>>(o: impl IntoIterator<Item = (T, T)>) -> String {
    o.into_iter()
        .map(|(a, b)| format!("{}) {}", a.as_ref(), b.as_ref()))
        .collect::<Vec<_>>()
        .join("  ")
}

/// The reader prompt (TR), SPEC §3.3; `items` = `(number, text)`.
#[must_use]
pub fn reader_prompt(persona_code: &str, cond: &str, items: &[(usize, String)]) -> String {
    let mut p = String::new();
    let _ = writeln!(p, "{}", persona(persona_code, Lang::Tr));
    let _ = writeln!(p);
    let _ = writeln!(
        p,
        "Aşağıda bir piyasa bilgi uygulamasının telefonda gösterdiği {} kısa metin var; her biri ayrı bir hisse hakkında.{}",
        items.len(),
        if cond == "yuz" {
            " Her metin, bildirimde ilk görünen kısımdır."
        } else {
            " Her metin, bildirime dokununca açılan ekranın tamamıdır; parantez içindekiler terime dokununca açılan açıklamalardır."
        }
    );
    let _ = writeln!(p);
    for (n, t) in items {
        let _ = writeln!(p, "[{n}]\n{t}");
    }
    let _ = writeln!(p, "Her metin için şu soruları yalnız metne dayanarak cevapla. Seçmeli sorularda yalnız harfi yaz. Açık sorularda EN ÇOK BİR CÜMLE yaz; metinde yoksa \"yok\" yaz. Tahmin etme.");
    let _ = writeln!(
        p,
        "S1 Ne oldu? {}",
        options(S1.iter().map(|(a, _, b)| (*a, *b)))
    );
    let _ = writeln!(
        p,
        "S2 Hangi eşik ya da sınır söz konusu ve değeri kaç? (bir cümle)"
    );
    let _ = writeln!(
        p,
        "S3 Anlatılan şey ne zaman oldu? {}",
        options(S3.iter().copied())
    );
    let _ = writeln!(
        p,
        "S4 Hâlâ geçerli mi? {}",
        options(S4.iter().map(|(a, _, b)| (*a, *b)))
    );
    let _ = writeln!(
        p,
        "S5 Geçmişte benzerleri kaç örnekte kaç kez tuttu? (bir cümle; ör. \"10 örnekte 7\")"
    );
    let _ = writeln!(
        p,
        "S6 Bu metin sana ne yapmanı söylüyor? {}",
        options(S6.iter().copied())
    );
    let _ = writeln!(p);
    let _ = writeln!(
        p,
        "Cevabını yalnız bir JSON dizisi olarak yaz, başka hiçbir şey yazma:"
    );
    let _ = writeln!(p, "[{{\"no\": 1, \"S1\": \"…\", \"S2\": \"…\", \"S3\": \"…\", \"S4\": \"…\", \"S5\": \"…\", \"S6\": \"…\"}}, …]");
    p
}

/// Critic criteria (SPEC §4.1), verbatim in the prompt.
pub const CRITERIA: &[&str] = &[
    "Ne olduğunu somut söylüyor mu: hangi eşik, adıyla ve değeriyle?",
    "Ne zaman olduğu ve hâlâ geçerli olup olmadığı ilk okumada anlaşılıyor mu?",
    "Geçmişte kaç örnekte kaç kez tuttuğunu ve neyin beklendiğini dürüstçe söylüyor mu?",
    "İç terim, jargon, belirsiz ifade yok mu (\"önemli bir eşik\" gibi)?",
    "Bir eylem (al, sat, bekle) önermiyor mu?",
    "Kısa ve hızlı okunuyor mu; ilk satır yeterli mi?",
    "Kısa metin ile açılan ayrıntı birbiriyle çelişmiyor mu?",
];

/// The critic prompt for one pair and order; `items` = `(number, text A, text B)`.
#[must_use]
pub fn critic_prompt(items: &[(usize, String, String)]) -> String {
    let mut p = String::new();
    let _ = writeln!(p, "Bir piyasa bilgi uygulamasının telefonda gösterdiği metinleri değerlendiriyorsun. Uygulama tavsiye vermez; her metin bir hissede olan bir şeyi anlatır. Aşağıda {} durum var; her durumda aynı olayın iki ayrı yazılışı (Metin A ve Metin B) verildi. Hangisinin kim tarafından ya da ne zaman yazıldığını bilmiyorsun.", items.len());
    let _ = writeln!(p, "Ölçütler (sırası önem sırası değildir):");
    for (i, c) in CRITERIA.iter().enumerate() {
        let _ = writeln!(p, "{}. {c}", i + 1);
    }
    let _ = writeln!(p);
    for (n, a, b) in items {
        let _ = writeln!(p, "[{n}]\nMetin A:\n{a}Metin B:\n{b}");
    }
    let _ = writeln!(p, "Her durum için önce kendi gözlemini EN ÇOK BİR CÜMLE yaz, sonra ölçütlere göre bütün olarak daha iyi olanı seç (\"A\" ya da \"B\"; eşitlik yok).");
    let _ = writeln!(
        p,
        "Cevabını yalnız bir JSON dizisi olarak yaz, başka hiçbir şey yazma:"
    );
    let _ = writeln!(p, "[{{\"no\": 1, \"gozlem\": \"…\", \"secim\": \"A\"}}, …]");
    p
}

/// Meta of one reader run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderMeta {
    /// Blind code of the language.
    pub code: String,
    /// Persona.
    pub persona: String,
    /// Condition.
    pub cond: String,
    /// Page (1, 2).
    pub page: usize,
    /// Reader number.
    pub reader: usize,
    /// Seed of the item order.
    pub seed: u64,
    /// Model alias.
    pub model: String,
    /// Item number → situation id.
    pub items: Vec<(usize, String)>,
}

/// Meta of one critic run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticMeta {
    /// Code shown as "Metin A".
    pub a: String,
    /// Code shown as "Metin B".
    pub b: String,
    /// Repetition (seed index).
    pub rep: usize,
    /// Order 1 or 2 (2 = the same items with A and B swapped).
    pub order: usize,
    /// Seed of the item order.
    pub seed: u64,
    /// Model alias.
    pub model: String,
    /// Item number → situation id.
    pub items: Vec<(usize, String)>,
    /// Character counts of the texts shown, per item `(A, B)` (length covariate, U2).
    pub lens: Vec<(usize, usize)>,
}

fn write(path: &Path, s: &str) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(path, s)
}

/// Writes every reader and critic run directory of a plan (SPEC §3.5): prompts, metas,
/// `key.json` (code → name), `set.json` (reader set). Returns the run count.
///
/// # Errors
/// File system errors.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn plan(
    run: &Path,
    sits: &[Situation],
    langs: &[Box<dyn Language>],
    personas: &[&str],
    readers: usize,
    reps: usize,
    seed: u64,
    model: &str,
) -> std::io::Result<usize> {
    let set = reader_set(sits);
    let codes: Vec<String> = langs.iter().map(|l| blind_code(l.name(), seed)).collect();
    let key_map: BTreeMap<&str, &str> = codes
        .iter()
        .map(String::as_str)
        .zip(langs.iter().map(|l| l.name()))
        .collect();
    write(
        &run.join("key.json"),
        &serde_json::to_string_pretty(&key_map).unwrap_or_default(),
    )?;
    let set_json: Vec<Value> = set
        .iter()
        .map(|(l, i)| serde_json::json!({"label": l, "situation": sits[*i].id, "cell": crate::situ::cell(&sits[*i])}))
        .collect();
    write(
        &run.join("set.json"),
        &serde_json::to_string_pretty(&set_json).unwrap_or_default(),
    )?;
    // surfaces: [lang][set item]
    let surf: Vec<Vec<Surfaces>> = langs
        .iter()
        .map(|l| {
            set.iter()
                .map(|(_, i)| l.surfaces(&sits[*i], Lang::Tr))
                .collect()
        })
        .collect();
    let mut count = 0;
    let half = set.len().div_ceil(2);
    let pages: Vec<Vec<usize>> = vec![(0..half).collect(), (half..set.len()).collect()];
    for (li, code) in codes.iter().enumerate() {
        for p in personas {
            for cond in CONDS {
                for (pi, page) in pages.iter().enumerate() {
                    for r in 1..=readers {
                        let s = seed
                            .wrapping_mul(1_000_003)
                            .wrapping_add(crate::stats::fnv(&format!("{code}{p}{cond}{pi}{r}")));
                        let mut order = page.clone();
                        Rng::new(s).shuffle(&mut order);
                        let items: Vec<(usize, String)> = order
                            .iter()
                            .enumerate()
                            .map(|(n, &j)| (n + 1, shown(&surf[li][j], cond)))
                            .collect();
                        let meta = ReaderMeta {
                            code: code.clone(),
                            persona: (*p).to_owned(),
                            cond: cond.to_owned(),
                            page: pi + 1,
                            reader: r,
                            seed: s,
                            model: model.to_owned(),
                            items: order
                                .iter()
                                .enumerate()
                                .map(|(n, &j)| (n + 1, sits[set[j].1].id.clone()))
                                .collect(),
                        };
                        let dir = run
                            .join("readers")
                            .join(format!("{code}_{p}_{cond}_p{}_r{r}", pi + 1));
                        write(&dir.join("prompt.txt"), &reader_prompt(p, cond, &items))?;
                        write(
                            &dir.join("meta.json"),
                            &serde_json::to_string_pretty(&meta).unwrap_or_default(),
                        )?;
                        count += 1;
                    }
                }
            }
        }
    }
    // Critics: every pair, `reps` seeds, both orders (U1, U11).
    for x in 0..langs.len() {
        for y in (x + 1)..langs.len() {
            for rep in 1..=reps {
                let s = seed
                    .wrapping_mul(7_919)
                    .wrapping_add(crate::stats::fnv(&format!("{}{}{rep}", codes[x], codes[y])));
                let mut order: Vec<usize> = (0..set.len()).collect();
                Rng::new(s).shuffle(&mut order);
                for (o, (a, b)) in [(x, y), (y, x)].into_iter().enumerate() {
                    let items: Vec<(usize, String, String)> = order
                        .iter()
                        .enumerate()
                        .map(|(n, &j)| {
                            (
                                n + 1,
                                shown(&surf[a][j], "acik"),
                                shown(&surf[b][j], "acik"),
                            )
                        })
                        .collect();
                    let meta = CriticMeta {
                        a: codes[a].clone(),
                        b: codes[b].clone(),
                        rep,
                        order: o + 1,
                        seed: s,
                        model: model.to_owned(),
                        items: order
                            .iter()
                            .enumerate()
                            .map(|(n, &j)| (n + 1, sits[set[j].1].id.clone()))
                            .collect(),
                        lens: items
                            .iter()
                            .map(|(_, ta, tb)| (ta.chars().count(), tb.chars().count()))
                            .collect(),
                    };
                    let dir = run.join("critics").join(format!(
                        "{}_{}_k{rep}_o{}",
                        codes[x],
                        codes[y],
                        o + 1
                    ));
                    write(&dir.join("prompt.txt"), &critic_prompt(&items))?;
                    write(
                        &dir.join("meta.json"),
                        &serde_json::to_string_pretty(&meta).unwrap_or_default(),
                    )?;
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

/// The words a prompt must never contain (SPEC §3.6): language names, design names,
/// "old/new/reference" words.
#[must_use]
pub fn forbidden_words(langs: &[Box<dyn Language>]) -> Vec<String> {
    let mut w: Vec<String> = langs.iter().map(|l| l.name().to_owned()).collect();
    for x in [
        "bozuk",
        "r3",
        "r4",
        "vadrel",
        "fotolay",
        "tavrik",
        "eski",
        "başvuru",
        "referans",
        "reference",
        "variant",
        "alternatif",
        "bugünkü dil",
        "yeni dil",
        "harness",
        "düzenek",
    ] {
        w.push(x.to_owned());
    }
    w.sort();
    w.dedup();
    w
}

/// Forbidden words found in a prompt (whole words, case-insensitive).
#[must_use]
pub fn check_prompt(prompt: &str, words: &[String]) -> Vec<String> {
    let low = crate::text::lower(prompt, Lang::Tr);
    words
        .iter()
        .filter(|w| {
            regex::Regex::new(&format!(
                r"\b{}\b",
                regex::escape(&crate::text::lower(w, Lang::Tr))
            ))
            .is_ok_and(|r| r.is_match(&low))
        })
        .cloned()
        .collect()
}

/// Extracts the model's text from `claude -p --output-format json` stdout and parses the
/// first JSON array in it.
#[must_use]
pub fn answer_array(raw: &str) -> Option<Vec<Value>> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let text = v.get("result")?.as_str()?;
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    serde_json::from_str::<Vec<Value>>(&text[start..=end]).ok()
}

/// Cost of one run (`total_cost_usd`).
#[must_use]
pub fn cost(raw: &str) -> f64 {
    serde_json::from_str::<Value>(raw)
        .ok()
        .and_then(|v| v.get("total_cost_usd").and_then(Value::as_f64))
        .unwrap_or(0.0)
}

/// A free answer cut to its first sentence and 200 characters (SPEC §3.3).
#[must_use]
pub fn cut(s: &str) -> String {
    let first = sentences(s).into_iter().next().unwrap_or_default();
    first.chars().take(200).collect()
}

fn letter(v: &Value, q: &str) -> String {
    v.get(q)
        .and_then(Value::as_str)
        .map(|s| {
            s.trim()
                .chars()
                .next()
                .map(|c| c.to_ascii_uppercase().to_string())
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

fn free(v: &Value, q: &str) -> String {
    cut(v.get(q).and_then(Value::as_str).unwrap_or(""))
}

fn when_letters(days: i64, intraday: bool) -> Vec<&'static str> {
    let mut v = vec![match days {
        0 => "A",
        1 => "B",
        2..=4 => "C",
        _ => "D",
    }];
    if intraday && days == 0 {
        v.push("E");
    }
    v
}

/// Per-question correctness of one answered item (`None` = not scored), SPEC §3.3.
#[must_use]
pub fn score_item(k: &Key, ans: &Value) -> BTreeMap<&'static str, Option<bool>> {
    let mut m = BTreeMap::new();
    let s1 = S1
        .iter()
        .find(|(_, c, _)| *c == k.event)
        .map_or("", |(l, _, _)| *l);
    m.insert("S1", Some(letter(ans, "S1") == s1));
    let s2 = free(ans, "S2");
    let n2 = numbers(&s2, Lang::Tr);
    m.insert(
        "S2",
        Some(if k.redacted {
            !s2.bytes().any(|b| b.is_ascii_digit())
        } else {
            k.threshold.values.iter().any(|v| has_close(&n2, *v))
        }),
    );
    let mut ok3 = when_letters(k.born_days, k.intraday);
    ok3.extend(when_letters(k.moved_days, k.intraday));
    m.insert("S3", Some(ok3.contains(&letter(ans, "S3").as_str())));
    let s4 = S4
        .iter()
        .find(|(_, c, _)| *c == k.validity_class())
        .map_or("", |(l, _, _)| *l);
    m.insert("S4", Some(letter(ans, "S4") == s4));
    let s5 = free(ans, "S5");
    let n5 = numbers(&s5, Lang::Tr);
    m.insert(
        "S5",
        if k.redacted {
            None
        } else if k.ledger_n == 0 {
            let l = crate::text::lower(&s5, Lang::Tr);
            Some(
                n5.iter().all(|x| *x == 0.0)
                    && (n5.contains(&0.0)
                        || ["henüz", "hiç", "örnek yok", "karne yok", "karnesi yok"]
                            .iter()
                            .any(|w| l.contains(w))),
            )
        } else {
            Some(
                has_close(&n5, f64::from(k.ledger_n))
                    && has_close(&n5, f64::from(k.ledger_realized)),
            )
        },
    );
    m.insert("S6", Some(letter(ans, "S6") == "D"));
    m
}

/// Chance level of each question (SPEC §3.3): choices 1/k (the "bilmiyorum" option not
/// counted); free numeric answers ≈ 0.
pub const CHANCE: &[(&str, f64)] = &[
    ("S1", 1.0 / 11.0),
    ("S2", 0.0),
    ("S3", 1.0 / 5.0),
    ("S4", 1.0 / 6.0),
    ("S5", 0.0),
    ("S6", 1.0 / 4.0),
];

/// Accumulated counts.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Tally {
    /// Correct.
    pub k: f64,
    /// Scored.
    pub n: f64,
}

/// Scores of one run directory.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Scores {
    /// code → persona → condition → question → tally.
    pub readers: BTreeMap<String, BTreeMap<String, BTreeMap<String, BTreeMap<String, Tally>>>>,
    /// code → advice reads (S6 = A or B) / items.
    pub advice_reads: BTreeMap<String, Tally>,
    /// Unparsed answers.
    pub unparsed: Vec<String>,
    /// Reader runs answered.
    pub reader_runs: usize,
    /// Critic runs answered.
    pub critic_runs: usize,
    /// Total cost (USD).
    pub cost_usd: f64,
    /// Per code: primary (weighted), low, high.
    pub primary: BTreeMap<String, (f64, f64, f64)>,
    /// BT: code → (log strength, low, high).
    pub bt: BTreeMap<String, (f64, f64, f64)>,
    /// Pair conflict rates: "a|b" → (conflicts, units).
    pub conflicts: BTreeMap<String, (usize, usize)>,
    /// Correlation between "longer text" and "won" over decided units (U2).
    pub length_corr: f64,
    /// Share of decided units won by the longer text.
    pub longer_won: (usize, usize),
    /// Bootstrap seed.
    pub bootstrap_seed: u64,
}

fn dirs(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(p)
        .map(|r| {
            r.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Scores a run (SPEC §3.7, §4.2).
///
/// # Errors
/// Never fails on missing answers (they are skipped); file errors on `situations` only.
#[allow(clippy::too_many_lines)]
pub fn score(run: &Path, sits: &[Situation], bootstrap_seed: u64) -> Scores {
    let keys: BTreeMap<String, Key> = sits.iter().map(|s| (s.id.clone(), key(s))).collect();
    let mut sc = Scores {
        bootstrap_seed,
        ..Scores::default()
    };
    for d in dirs(&run.join("readers")) {
        let Ok(raw) = std::fs::read_to_string(d.join("answer.json")) else {
            continue;
        };
        let Ok(meta) = std::fs::read_to_string(d.join("meta.json"))
            .map_err(|_| ())
            .and_then(|m| serde_json::from_str::<ReaderMeta>(&m).map_err(|_| ()))
        else {
            continue;
        };
        sc.cost_usd += cost(&raw);
        let Some(arr) = answer_array(&raw) else {
            sc.unparsed.push(d.display().to_string());
            continue;
        };
        sc.reader_runs += 1;
        for (n, sid) in &meta.items {
            let Some(ans) = arr
                .iter()
                .find(|a| a.get("no").and_then(Value::as_u64) == u64::try_from(*n).ok())
            else {
                continue;
            };
            let Some(k) = keys.get(sid) else { continue };
            for (q, ok) in score_item(k, ans) {
                if let Some(ok) = ok {
                    let t = sc
                        .readers
                        .entry(meta.code.clone())
                        .or_default()
                        .entry(meta.persona.clone())
                        .or_default()
                        .entry(meta.cond.clone())
                        .or_default()
                        .entry(q.to_owned())
                        .or_default();
                    t.n += 1.0;
                    if ok {
                        t.k += 1.0;
                    }
                }
            }
            let a = sc.advice_reads.entry(meta.code.clone()).or_default();
            a.n += 1.0;
            if matches!(letter(ans, "S6").as_str(), "A" | "B") {
                a.k += 1.0;
            }
        }
    }
    // Primary: persona-weighted (F 2, H 1) accuracy over all scored questions and both
    // conditions; interval = the same weighting of the per-persona Wilson bounds.
    for (code, per) in &sc.readers {
        let mut parts = Vec::new();
        for (p, w) in [("F", 2.0), ("H", 1.0)] {
            if let Some(c) = per.get(p) {
                let (k, n) = c
                    .values()
                    .flat_map(|q| q.values())
                    .fold((0.0, 0.0), |(k, n), t| (k + t.k, n + t.n));
                if n > 0.0 {
                    let (l, h) = wilson(k, n);
                    parts.push((w, k / n, l, h));
                }
            }
        }
        let ws: f64 = parts.iter().map(|x| x.0).sum();
        if ws > 0.0 {
            let m = |f: fn(&(f64, f64, f64, f64)) -> f64| {
                parts.iter().map(|x| x.0 * f(x)).sum::<f64>() / ws
            };
            sc.primary
                .insert(code.clone(), (m(|x| x.1), m(|x| x.2), m(|x| x.3)));
        }
    }
    // Critics: per (pair, rep, situation) the two orders; same winner = win, split = tie.
    let mut by_unit: BTreeMap<(String, String, usize, String), Vec<(String, usize)>> =
        BTreeMap::new();
    let mut lens: BTreeMap<(String, String, usize, String), (usize, usize)> = BTreeMap::new();
    for d in dirs(&run.join("critics")) {
        let Ok(raw) = std::fs::read_to_string(d.join("answer.json")) else {
            continue;
        };
        let Ok(meta) = std::fs::read_to_string(d.join("meta.json"))
            .map_err(|_| ())
            .and_then(|m| serde_json::from_str::<CriticMeta>(&m).map_err(|_| ()))
        else {
            continue;
        };
        sc.cost_usd += cost(&raw);
        let Some(arr) = answer_array(&raw) else {
            sc.unparsed.push(d.display().to_string());
            continue;
        };
        sc.critic_runs += 1;
        let (x, y) = if meta.order == 1 {
            (meta.a.clone(), meta.b.clone())
        } else {
            (meta.b.clone(), meta.a.clone())
        };
        for (i, (n, sid)) in meta.items.iter().enumerate() {
            let Some(ans) = arr
                .iter()
                .find(|a| a.get("no").and_then(Value::as_u64) == u64::try_from(*n).ok())
            else {
                continue;
            };
            let pick = letter(ans, "secim");
            let winner = match pick.as_str() {
                "A" => meta.a.clone(),
                "B" => meta.b.clone(),
                _ => continue,
            };
            let u = (x.clone(), y.clone(), meta.rep, sid.clone());
            by_unit
                .entry(u.clone())
                .or_default()
                .push((winner, meta.order));
            if let Some(&(la, lb)) = meta.lens.get(i) {
                let (lx, ly) = if meta.order == 1 { (la, lb) } else { (lb, la) };
                lens.insert(u, (lx, ly));
            }
        }
    }
    let mut codes: Vec<String> = sc.readers.keys().cloned().collect();
    for (x, y, _, _) in by_unit.keys() {
        for c in [x, y] {
            if !codes.contains(c) {
                codes.push(c.clone());
            }
        }
    }
    codes.sort();
    let idx = |c: &str| codes.iter().position(|x| x == c).unwrap_or(0);
    let mut units: Vec<Vec<Duel>> = Vec::new();
    let (mut x_win, mut longer, mut decided) = (Vec::new(), Vec::new(), (0usize, 0usize));
    for ((x, y, _, _), ws) in &by_unit {
        if ws.len() < 2 {
            continue;
        }
        let c = sc.conflicts.entry(format!("{x}|{y}")).or_insert((0, 0));
        c.1 += 1;
        let sa = if ws[0].0 == ws[1].0 {
            if ws[0].0 == *x {
                1.0
            } else {
                0.0
            }
        } else {
            c.0 += 1;
            0.5
        };
        units.push(vec![Duel {
            a: idx(x),
            b: idx(y),
            sa,
        }]);
    }
    // Length covariate (U2): over decided units, is the winner the longer text?
    for ((x, y, rep, sid), ws) in &by_unit {
        if ws.len() < 2 || ws[0].0 != ws[1].0 {
            continue;
        }
        if let Some(&(lx, ly)) = lens.get(&(x.clone(), y.clone(), *rep, sid.clone())) {
            if lx == ly {
                continue;
            }
            let x_won = ws[0].0 == *x;
            let x_longer = lx > ly;
            x_win.push(if x_won { 1.0 } else { 0.0 });
            longer.push(if x_longer { 1.0 } else { 0.0 });
            decided.1 += 1;
            if x_won == x_longer {
                decided.0 += 1;
            }
        }
    }
    sc.length_corr = corr(&x_win, &longer);
    sc.longer_won = decided;
    if !units.is_empty() {
        let all: Vec<Duel> = units.iter().flatten().copied().collect();
        let s = bradley_terry(codes.len(), &all);
        let ci = bt_bootstrap(codes.len(), &units, 1000, bootstrap_seed);
        for (i, c) in codes.iter().enumerate() {
            sc.bt.insert(c.clone(), (s[i], ci[i].0, ci[i].1));
        }
    }
    sc
}

/// Whether two intervals overlap ("ayırt edilemedi").
#[must_use]
pub fn overlap(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 <= b.1 && b.0 <= a.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_are_blind() {
        let langs = crate::lang::registry();
        let words = forbidden_words(&langs);
        let p = reader_prompt("F", "yuz", &[(1, "SYN03: son 20 günün en yükseği".into())]);
        assert!(
            check_prompt(&p, &words).is_empty(),
            "{:?}",
            check_prompt(&p, &words)
        );
        assert!(!check_prompt("bu r4 dilidir", &words).is_empty());
        let c = critic_prompt(&[(1, "a\n".into(), "b\n".into())]);
        assert!(check_prompt(&c, &words).is_empty());
    }

    #[test]
    fn cut_and_answer() {
        assert_eq!(cut("10 örnekte 7. Ama az."), "10 örnekte 7.");
        let raw = r#"{"result":"```json\n[{\"no\":1,\"S1\":\"a\"}]\n```","total_cost_usd":0.01}"#;
        assert_eq!(answer_array(raw).unwrap().len(), 1);
        assert!((cost(raw) - 0.01).abs() < 1e-12);
    }
}
