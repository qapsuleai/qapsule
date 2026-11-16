//! Reader prompts (TR). Persona descriptions verbatim from SPEC §1; the context sentence
//! "Bu bir piyasa bilgi uygulamasının ekran görüntüsü." is given, "Uygulama tavsiye
//! vermez." is **not** (M8 S5 measures it). `@DIR@` is the only placeholder: the reader's
//! private directory (substituted by `run_readers.sh`). No design name, no experiment
//! vocabulary (checked by [`crate::blind::check`]).

use crate::s8;

/// Persona codes.
pub const PERSONAS: [&str; 2] = ["F", "H"];

/// SPEC §1 persona description (verbatim).
#[must_use]
pub fn persona(p: &str) -> &'static str {
    match p {
        "F" => "Yirmi yaşındasın. Hisse senedi alıp satıyorsun ama teknik analiz bilmiyorsun; grafik, indikatör, mum gibi terimler sana bir şey ifade etmiyor. Telefonuna bir bildirim geldi; ekrana yalnızca birkaç saniye bakacaksın. İlk bakışta ne anladığını cevapla; uzun uzun inceleme.",
        _ => "Yıllardır borsada işlem yapıyorsun; teknik analize, mum grafiklerine ve indikatörlere alışkınsın. Telefonunda bir piyasa uygulamasına bakıyorsun. Gördüğünü okuyup cevapla.",
    }
}

/// The context sentence both personas get.
pub const CONTEXT: &str = "Bu bir piyasa bilgi uygulamasının ekran görüntüsü.";

fn options(o: &[&str]) -> String {
    o.iter()
        .map(|x| format!("\"{x}\""))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// M8 persona prompt for one page image with `n` labelled cards.
#[must_use]
pub fn m8_persona(p: &str, image: &str, n: usize) -> String {
    let labels: Vec<&str> = crate::m8::LABELS[..n.min(crate::m8::LABELS.len())].to_vec();
    format!(
        "{persona}\n\n{CONTEXT}\n\nGörüntü: @DIR@/{image} (Read aracıyla aç). Ekranda alt alta {n} kart var; \
her kartın sol üst köşesinde bir harf var ({labs}).\n\n\
Her harf için aşağıdaki altı soruyu cevapla. Her soruda yalnız verilen seçeneklerden birini, tırnak içindeki \
metni aynen yazarak seç.\n\n\
S1 Ne oldu? (yön): {s1}\n\
S1b Ne oldu? (ne tür): {s1b}\n\
S2 Ne zaman? {s2}\n\
S3 Hâlâ geçerli mi? {s3}\n\
S4 Ne kadar güvenilir? {s4}\n\
S5 Bana ne yapmamı söylüyor? {s5}\n\n\
Cevabını yalnız bir JSON nesnesi olarak ver, başka hiçbir şey yazma:\n\
{{{example}, \"not\": \"(isteğe bağlı, kısa)\"}}\n",
        persona = persona(p),
        labs = labels.join(", "),
        s1 = options(&s8::S1),
        s1b = options(&s8::S1B),
        s2 = options(&s8::S2),
        s3 = options(&s8::S3),
        s4 = options(&s8::S4),
        s5 = options(&s8::S5),
        example = labels
            .iter()
            .map(|l| format!("\"{l}\": {{\"S1\": \"…\", \"S1b\": \"…\", \"S2\": \"…\", \"S3\": \"…\", \"S4\": \"…\", \"S5\": \"…\"}}"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// SPEC §2(c) critic criteria (verbatim).
pub const CRITERIA: [&str; 8] = [
    "Tek bakış: kapalı yüz tek bir mesaj veriyor mu?",
    "Hiyerarşi: önce ne okunacağı belli mi?",
    "Geçerlilik: sürüyor / artık geçerli değil / kapandı ilk bakışta ayrılıyor mu; işaret ve cümle aynı şeyi mi söylüyor?",
    "Dürüstlük: güvenilirlik (karne, az örnek) görünür mü, kısaltılmış ama gizlenmemiş mi?",
    "Gündelik dil: iç terim, jargon, \"bar\" yok mu; zaman gündelik mi?",
    "Erişilebilirlik: renk tek başına anlam taşımıyor mu (gri kopya verilir)?",
    "Tavsiye değil: ekran bir eylem önermiyor mu?",
    "Estetik ve özgünlük: finans uygulaması kalıbının kopyası değil, \"tek bakışta anlam\" hedefinden mi?",
];

/// M8 critic prompt: `designs` = (letter, image names).
#[must_use]
pub fn m8_critic(designs: &[(String, Vec<String>)]) -> String {
    let mut s = String::from(
        "Bir piyasa bilgi uygulamasının akış ekranı için birkaç tasarımı değerlendiren deneyimli bir arayüz \
tasarımcısısın. Her tasarımda aynı durumlar gösterilir: her ekranda alt alta kartlar var, her kart bir piyasa \
olayını kapalı hâlde (tek bakışta) anlatır; kartların sol üstündeki harfler yalnız konum etiketidir. \
Uygulama yatırım tavsiyesi vermemelidir.\n\nGörüntüler (Read aracıyla aç; hepsi @DIR@ dizininde):\n",
    );
    for (l, files) in designs {
        s.push_str(&format!(
            "- Tasarım {l}: {}\n",
            files
                .iter()
                .map(|f| format!("@DIR@/{f}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    s.push_str("\n(\"-gri\" ile biten dosyalar aynı ekranın gri tonlamalı kopyasıdır.)\n\nHer tasarıma aşağıdaki sekiz ölçütün her biri için 1 (çok kötü) ile 5 (çok iyi) arasında bir tam sayı puan ver, sonra tasarımları en iyiden en kötüye sırala:\n");
    for (i, c) in CRITERIA.iter().enumerate() {
        s.push_str(&format!("{}. {c}\n", i + 1));
    }
    let letters: Vec<&str> = designs.iter().map(|(l, _)| l.as_str()).collect();
    s.push_str(&format!(
        "\nCevabını yalnız bir JSON nesnesi olarak ver, başka hiçbir şey yazma:\n\
{{\"puanlar\": {{{}}}, \"sira\": [{}], \"gerekce\": \"(kısa)\"}}\n",
        letters
            .iter()
            .map(|l| format!("\"{l}\": {{\"1\": 0, \"2\": 0, \"3\": 0, \"4\": 0, \"5\": 0, \"6\": 0, \"7\": 0, \"8\": 0}}"))
            .collect::<Vec<_>>()
            .join(", "),
        letters
            .iter()
            .map(|l| format!("\"{l}\""))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    s
}

/// M9 G2 "which" options.
pub const G2_WHICH: [&str; 5] = ["{X}", "{Y}", "ikisinde de", "hiçbirinde", "bilmiyorum"];
/// M9 G2 "what" options.
pub const G2_WHAT: [&str; 5] = [
    "zayıfladı",
    "tutmadı, geçersiz oldu",
    "geri çekildi",
    "olumsuz bir şey yok",
    "bilmiyorum",
];
/// M9 G1 directions.
pub const G1_DIRS: [&str; 3] = ["yukarı", "aşağı", "yönsüz"];

/// M9 persona prompt: `screens` image names in order, the two held instruments, the
/// instrument list of G1's options.
#[must_use]
pub fn m9_persona(p: &str, screens: &[String], held: [&str; 2], instruments: &[String]) -> String {
    let which: Vec<String> = G2_WHICH
        .iter()
        .map(|w| w.replace("{X}", held[0]).replace("{Y}", held[1]))
        .collect();
    let g1: Vec<String> = instruments
        .iter()
        .flat_map(|i| G1_DIRS.iter().map(move |d| format!("{i} {d}")))
        .collect();
    format!(
        "{persona}\n\n{CONTEXT}\n\nAşağıdaki görüntüler aynı ekranın aşağı kaydırılmış hâlleridir, sırayla: {list}. \
(Read aracıyla aç; ekran 1 en üsttür.) Kartların sol üstündeki kısa etiketler kartları adlandırmak içindir.\n\n\
Uygulamaya en son bir önceki işlem gününde, aşağı yukarı bu saatte bakmıştın; şimdi yeniden açtın.\n\n\
G1 Son baktığından beri ne değişti? Değişen her enstrümanı ve değişimin yönünü şu seçeneklerden seç \
(birden çok seçebilirsin, hiçbiri de olabilir): {g1}\n\
G2 Elindeki hisseler: {x} ve {y}. Bunlarda olumsuz bir şey var mı? Hangisinde: {which}. Ne: {what}\n\
G3 En güvenilir kart hangisi? (etiketini yaz)\n\n\
Her görev için ayrıca cevabı hangi ekranda bulduğunu (ekran numaraları) ve cevap için bir kartı açman \
(dokunup ayrıntısına bakman) gerekir miydi, gerekirdiyse hangilerini (etiketler) yaz.\n\n\
Cevabını yalnız bir JSON nesnesi olarak ver, başka hiçbir şey yazma:\n\
{{\"G1\": {{\"cevap\": [\"…\"], \"ekran\": [1], \"acmam_gerekirdi\": []}}, \
\"G2\": {{\"hangisi\": \"…\", \"ne\": \"…\", \"ekran\": [1], \"acmam_gerekirdi\": []}}, \
\"G3\": {{\"cevap\": \"…\", \"ekran\": [1], \"acmam_gerekirdi\": []}}, \"not\": \"(isteğe bağlı, kısa)\"}}\n",
        persona = persona(p),
        list = screens
            .iter()
            .enumerate()
            .map(|(i, s)| format!("ekran {} = @DIR@/{s}", i + 1))
            .collect::<Vec<_>>()
            .join(", "),
        g1 = options(&g1.iter().map(String::as_str).collect::<Vec<_>>()),
        x = held[0],
        y = held[1],
        which = options(&which.iter().map(String::as_str).collect::<Vec<_>>()),
        what = options(&G2_WHAT),
    )
}

/// M10 G2 options.
pub const M10_G2: [&str; 3] = ["evet", "hayır", "emin değilim"];
/// M10 region labels.
pub const M10_REGIONS: [&str; 4] = ["A", "B", "C", "D"];

/// M10 persona prompt: `seqs` = per sequence its four images (capsule, two in-between
/// frames, chart).
#[must_use]
pub fn m10_persona(p: &str, seqs: &[[String; 4]]) -> String {
    let mut s = format!(
        "{persona}\n\n{CONTEXT}\n\nAşağıda {n} kısa dizi var. Her dizide önce bir kartın açılmış hâlini, sonra \
ekranın geçiş sırasındaki iki ara karesini, sonra açılan grafik ekranını görürsün (Read aracıyla sırayla aç):\n",
        persona = persona(p),
        n = seqs.len()
    );
    for (i, q) in seqs.iter().enumerate() {
        s.push_str(&format!(
            "- Dizi {}: kart @DIR@/{}, ara kare 1 @DIR@/{}, ara kare 2 @DIR@/{}, grafik @DIR@/{}\n",
            i + 1,
            q[0],
            q[1],
            q[2],
            q[3]
        ));
    }
    let ex: Vec<String> = (1..=seqs.len())
        .map(|i| format!("\"{i}\": {{\"G1\": \"…\", \"G2\": \"…\", \"G3\": \"…\"}}"))
        .collect();
    s.push_str(&format!(
        "\nGrafik ekranında A, B, C, D harfli dört işaret var. Her dizi için:\n\
G1 Grafikte işaretli yer az önceki kartın söylediği şeyin neresi? {g1}\n\
G2 Grafik ekranında az önceki kartla aynı şeye mi bakıyorsun? {g2}\n\
G3 Grafikte kartın iddiası hâlâ geçerli mi? {g3}\n\n\
Cevabını yalnız bir JSON nesnesi olarak ver (anahtarlar dizi numaraları), başka hiçbir şey yazma:\n\
{{{ex}, \"not\": \"(isteğe bağlı, kısa)\"}}\n",
        g1 = options(&M10_REGIONS),
        g2 = options(&M10_G2),
        g3 = options(&s8::S3),
        ex = ex.join(", ")
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_are_clean() {
        let a = m8_persona("F", "0123abcd.png", 6);
        crate::blind::check("m8", &a).unwrap();
        assert!(a.contains(persona("F")));
        assert!(!a.contains("tavsiye vermez"));
        let c = m8_critic(&[("K".into(), vec!["K-1.png".into()])]);
        crate::blind::check("critic", &c).unwrap();
        let m = m9_persona(
            "H",
            &["a.png".into()],
            ["SYN01", "SYN02"],
            &["SYN01".into()],
        );
        crate::blind::check("m9", &m).unwrap();
        let t = m10_persona("F", &[["a".into(), "b".into(), "c".into(), "d".into()]]);
        crate::blind::check("m10", &t).unwrap();
    }
}
