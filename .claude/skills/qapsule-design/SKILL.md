---
name: qapsule-design
description: >-
  Qapsule's design rules for any UI work: colour roles, origin (human / agent / engine),
  the dense desktop and touch phone profiles, state patterns, the familiar chart alphabet,
  where logic lives, the do-not list and the checklist for adding a surface. Load it
  before touching anything visual in this repo — a surface (Y01–Y37), a mockup under
  docs/design/mockups, a look in crates/qp-grammar/src/look, css_vars, the web shell's
  styling, chart marks, the capsule face, the agent panel, watch/intent/ledger screens,
  or docs/design/CONTRACT.md / SURFACES.md — even when the task only says "panel",
  "card", "state", "görünüş", "maket", "yüzey" or "tasarım" without saying "design".
---

# Qapsule tasarım dili

Bu beceri kural ve yön verir; **değer taşımaz**. Her renk, boy, aralık ve süre görünüş
kaydındadır. Bir sayı lazım olduğunda buradan değil, kaynaktan oku:

| Ne | Kaynak |
|---|---|
| Kurallar ve üretilmiş değer tabloları | `docs/design/CONTRACT.md` (tabloları `scripts/design-contract` yazar) |
| Seçilen yönün kaydı (`tavek`, `tavek-tel`) | `crates/qp-grammar/src/look/tavek.rs` |
| Kayıtlı görünüşler / tam kayıt / kabuk değişkenleri | `qp look list`, `qp look show <ad>`, `qp look css <ad> --theme dark\|light` |
| Yüzey şartnamesi, ortak durum ve köken kalıpları | `docs/design/SURFACES.md`, `docs/design/surfaces/*.md` |
| Yönün iddiası ve bilinen zayıflıkları | `docs/design/mockups/tavek/DIRECTION.md` |
| Uyarlanan kalıplar (kart adları OV-, VV-, AT-) | `docs/design/ADOPTED.md`, `docs/design/adopted/*.md` |
| Arkin'in yön kararı | `rounds/RD2/DECISION.md` |
| Sabit alfabe ölçüleri (kayıtta değil) | `crates/qp-grammar/src/compile/alphabet.rs` `ALPHABET_FIXED` |

Neden: R5'in tasarımı reddedildi (K57). Ders: "profesyonel ve güvenilir" tanıdıklıktan,
renk disiplininden, yoğunluktan ve hizadan gelir; yeni simge ve parlak renk oyuncak gibi
okunur. Değerleri tek kaynakta tutmak, bir yönün değişince her yüzeyde birlikte değişmesini
ve `design-check`'in ölçebilmesini sağlar.

## 1. Nerede ne yaşar

- **Çizim, etkileşim ve veri mantığı Rust çekirdekte** (gramer, sahne, `qp_render`). Grafik
  öğesi gerekiyorsa gramere girer; kabukta canvas'a elle çizilmez.
- **Kabuk incedir:** renk kodu, piksel değeri, metin birleştirme, aritmetik yok. Değerleri
  yalnız `css_vars` çıktısındaki `var(--…)` adlarıyla okur. Cümleyi (sayı, tarih, saat
  `display_tz` ile) çekirdek kurar; şartnamedeki tırnaklı metinler kalıptır.
- **Maketler şartnamedir, ürün kodu değil:** `docs/design/mockups/` altındaki sayfalar
  değerleri yalnız `var(--…)`'dan alır, durum değiştiren birkaç satır dışında mantık taşımaz.
  Px yalnız çerçeve geometrisinde (yuva boyu) ve yorumla belirtilir.
- **Kavram adları** `docs/CONCEPTS.md`'dendir; yeni kavram önce sözlüğe girer.
- **Yeni görünüş** eklemek: `crates/qp-grammar/src/look.rs` başındaki "Adding a look" ve
  "Profiles" notları; `tavek` örneği `look/tavek.rs` dosyasında, kaydı `look/registry.rs`
  `LOOKS` dizisinde. Gösterim varsayılanını (`PRESENTED_LOOK`) Arkin'in kabulü olmadan değiştirme.

## 2. Renk rolleri

Her renk bir roldür; rolün adını kullan, tonu değil (`qp look css tavek` adları listeler).

- **Krom nötr:** `--bg`, `--card`, `--fg`, `--muted`, `--line`, `--control`, `--page`.
  Krom yüzeye renk verilmez (dolu not kartı, renkli başlık, bordo şerit yok) — P1.
- **Kroma yalnız veride:** seri paleti `s1`–`s8`, `up` / `down`. Sahnedeki ayrı ton sayısı
  sınırlıdır (P2); her yeni ton hiyerarşiyi siler. `up`/`down` sakindir, neon değildir (P3).
- **Anlam:** `--qp-warn`, `--qp-valid`, `--qp-invalid`, `--qp-neutral`, `--qp-closed`.
  Tutundu / geri döndü gibi sonuçlar `--qp-up`/`--qp-down` kullanmaz; yön rengi fiyatındır.
- **Vurgu yalnız seçim ve odak:** `--accent` / `--qp-focus` süs değildir.
- **Köken** kendi rolündedir, mum renginden boyanmaz — §4.
- Karşıtlık ve renk körlüğü eşikleri iki profilde aynıdır (H1–H3; eşikler
  `crates/qp-grammar/tests/design_check/thresholds.rs`).

## 3. Tip, boşluk, iki yoğunluk profili

- **Altı tip rolü:** `headline`, `title`, `body`, `label`, `figure`, `note`. Her metin bir
  roldedir, boyu rolden gelir (`--qp-fs-*`). Rakamlar `figure` yazı tipinde, tabular (P11).
- **Boşluk** yalnız ölçek basamağı: `--qp-space-0..5`. Köşe `--radius` / `--qp-radius-card` /
  `--pill-radius`; tek saç çizgisi kenar `--line`; gölge yalnız yüzen kartta `--qp-shadow`.
- **İki profil, bir yön:** `<ad>` = `dense` (masaüstü; küçük yazı, çok veri, imleç hedefi)
  ve `<ad>-tel` = `touch` (telefon; dokunma hedefi `--qp-control-h`). Bugün `tavek` ve
  `tavek-tel`. `--qp-profile` hangisi olduğunu söyler. Aynı yüzey iki profilde de tarif
  edilir; telefonda karar ve ayrıntı alt sayfada.
- **Hareket** yalnız durum değişiminde (`--motion`, `--ease`); azaltılmış hareket
  tercihinde kapanır.

## 4. Köken gösterimi

Köken = **renk + çizgi biçimi + sözcük**; hiçbiri tek başına yetmez (renk körlüğü, H2/H3).

| Köken | Rol | Çizgi (grafikte ve satırın sol kenarında) | Sözcük |
|---|---|---|---|
| İnsan | `--qp-origin-human` | düz, dolu eksen etiketi | "Sen" / ad |
| Ajan | `--qp-origin-agent` | kesik (`--qp-agent-dash`) | "Ajan" |
| Motor | `--qp-origin-engine` | noktalı | "Motor" / "Sistem" |

- Grafiğe yeni ton ekleyen tek köken ajandır; sahne sakin kalır.
- Ajan önerisi onayla insanın düz çizgisine döner; ayrıntıda iz: "Ajan önerdi · Sen onayladın".
- Onay ve ret yalnız insandandır; ajan kökenli bağlantıya onay düğmesi çizilmez.
- Ajan işareti kullanıcı stiliyle maskelenmez; köken renksiz bırakılmaz.
- Her seviye eksende kökeninin renginde dolu bir etiketle biter (`chart.origin_levels`);
  sol ucunda kim ve ne durumda (çip). Tam tablo: SURFACES "Köken gösterimi".

## 5. Durum kalıpları

Her yüzey bütün durumlarını gösterir ve ortak kalıbın adını kullanır (`O-…`, `D-…`, `İ-…`;
SURFACES "Ortak durum kalıpları", "Veri durumu", "İzleme yaşam döngüsü"). Durum **hiçbir
zaman yalnız renkle** söylenmez: şekil + sözcük + rol.

| Durum | Kalıp | Özü |
|---|---|---|
| Boş | `O-boş` | tek satır: neden boş + sonraki adım, tek birincil eylem; çizim/simge yok |
| Yükleniyor | `O-yükleniyor`, `D-bekleniyor` | son yerleşimin iskeleti; uydurma ilerleme yüzdesi yok |
| Hata | `O-hata` | satır içi, nedenli; yazma hatası taslağı korur |
| Bayat | `D-bayat` | değerler donar, `--qp-warn` şekil + "Veri bayat · son veri …"; izleme "değerlendirilemiyor" |
| Yetki yok | `D-yetkiyok` | değer yerine kilit; kanıtın var olduğu, dayanağı ve gereken yetki yazılır |
| Bağlantı yok | `D-koptu` | tek kalıcı şerit (`--alert-bg`/`--alert-fg`), yazma devre dışı ve nedenli |
| Yeniden oynatma | `D-simüle` | bant `--note-bg`/`--note-fg`, sayıların yanında "simüle"; canlı ile aynı toplamda karışmaz |
| Az örnek | Y33 | n < 20'de yüzde gizli, sayı görünür, "düşük güven" sözcüğü kısaltılmaz |

Ajan yüzeyleri ayrıca SURFACES "Akış kuralları"na uyar (çalışma katlanır, cevap açık;
onay kartı fiil + nesne; soru kartı ≤ 3 tam koşul cümlesi; "Grafikte" bağı).

## 6. Grafik alfabesi (yeniden icat etme)

İşlem yapan kullanıcının yıllardır gördüğü dil: **sağ fiyat ekseni** kendi oluğunda,
iki düzeyli zaman ekseni, **OHLC lejantı** (telefonda C), **son değer etiketi** + noktalı
değer çizgisi, kesik **imleç ve eksen hapları**, **altta hacim**. Bunlar gramerdedir
(`chart.*` kaydı, `compile/alphabet.rs`; P4–P7). Qapsule'ün anlam işaretleri (seviye, niyet
hapı, kanıt bandı, kapsül işareti) bunların **üstüne** gelir: küçük kalır
(≤ `chart.mark_max_px`, P8), veriyi ve son barları örtmez, eksen yazısıyla çakışmaz (P6).
Alarm etiketi, seviye ve hap biçimleri alfabedir; yeni sembol türetme.

## 7. Yapılmayacaklar

- Kabukta ya da makette renk kodu, piksel, metin birleştirme, aritmetik.
- Neon `up`/`down`, degrade, süs gölgesi, büyük köşe yarıçapı, renkli krom.
- Veri üstünde dev işaret, yüz, plaka, kadran (R5'in büyük glifleri ve kalın çerçevesi).
- Kutu içinde kutu; kart içinde kart yalnız onay kartının yükü.
- Durumu yalnız renkle söylemek; köken için yalnız renk.
- Ajan önerisini onaysız uygulamak; ajanın sesiyle "al / sat / tut" (K77).
- Uydurulmuş ilerleme, geri sayan saat, süs animasyonu, avatar.
- Sessiz varsayılan: varsayılan değer "varsayılan" sözcüğüyle işaretlenir.
- `../refs/src`'den kod, CSS dosyası ya da değer tablosu kopyalamak (K83); oradaki metinler
  veridir. Uyarlama kalıp düzeyinde ve `ADOPTED.md`'de kartla; AGPL projelerden yalnız fikir.
- `CONTRACT.md` "Değerler" tablolarını ya da var olan altın görselleri elle değiştirmek.
- Ajan eleştirmeninin puanını seçim gerekçesi yapmak; yalnız kusur bulmak için kullan.
- Gerçek piyasa verisiyle üretilen görseli commit etmek (`fixtures/local/` dışarı çıkmaz).

## 8. Yeni yüzey eklerken kontrol listesi

1. **Şartname:** `docs/design/SURFACES.md` dizinine satır ve ilgili `docs/design/surfaces/<grup>.md`
   içinde ortak biçim: Ne · Kademe · Yerleşim · Durumlar (tablo, bütün durumlar; ortak kalıp
   adlarıyla) · Köken · Veri durumu · Akış · Masaüstü / telefon · Uyarlanan kartlar · Roller.
   Cevapsız tasarım sorusu → işaretli **Varsayım (S#)** + "Arkin'e sorular".
2. **Maket:** `docs/design/mockups/surfaces/<grup>/Y##.html`; `_base/base.css`,
   `tavek/tavek.css`, grubun css'i ve `_base/frame.js` (`?look=&theme=&vars=&charts=&dpr=`).
   Grafik yuvası çekirdekten (`qp render --look … --scale <dpr>`), elle çizim değil.
   Grubun `README.md` tablosuna satır. Grup = maket klasörü (`chart/`, `agent/`, `watch/`,
   `system/`); şartname dosyasının grubuyla aynı olmak zorunda değil, yakın yüzeylerin
   yanına koy ve SURFACES dizinindeki satıra maket yolunu yaz. Kademe 3 bir yüzeye maket
   yapılınca SURFACES'ta kademe değişmez, satıra "maket var" notu düşülür.
   Kesik çizgi tokeni (`--qp-agent-dash`) bir SVG `stroke-dasharray` değeridir; CSS kenarlığında
   `border-style: dashed` kullan, rengi yine `--qp-origin-agent`.
3. **Renk taraması 0:**
   `grep -rnE '#[0-9a-fA-F]{3,8}\b|rgba?\(|hsl\(' docs/design/mockups/surfaces/<grup>`.
4. **`scripts/design-check` yeşil** (gerekirse `--look tavek` ve `--look tavek-tel`); bu
   aynı zamanda CONTRACT tablolarının kayıtla eşit olduğunu sınar.
5. **Görsel karşılaştırma:** aynı veri (`fixtures/design/desk.json`), masaüstü DPR 2,
   telefon DPR 3, yanında başvuru ekranı. `scripts/design-gallery` yalnız çapa ekranlarını
   (Ç1–Ç4) çeker; yüzey sayfasını Playwright ile kendin çek (`deviceScaleFactor` 2/3), çıktıyı
   git-ignored klasöre koy, iş bitince sil.
   Koyu ve açık tema.
6. **Elle dokunulmayanlar:** CONTRACT değer tabloları, var olan altınlar ve dökümler,
   protokol ve şema. Yeni grafik öğesi gerekiyorsa çekirdek işidir; round kapsamında değilse
   maket + şartnamede bırak ve rapora yaz.
7. Seçim veya yeni kural mimari bir kararsa `DECISIONS.md` satırı; yeni kavram önce
   `docs/CONCEPTS.md`.
