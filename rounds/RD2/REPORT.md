# RD2 — Tasarım turu: rapor (yazılıyor)

Round başı: `9aee506c` (ana akış commit'i; RG1 sonu `ec1c7c89`).

## 0. Round başı notları

- **Ana akış belgeleri** `9aee506c` ile kaydedildi (CLAUDE.md, ROADMAP, STATUS, ENGINEERING,
  kararlar K88–K89, research, `rounds/RD2/`). `NORTH_STAR.md` değişikliği yoktu.
- **Alt ajanlar başlatılabiliyor.** İlk deneme `isolation: worktree` ile düştü (oturumun çalışma
  dizini `../refs`'e kaymıştı, git deposu değil); worktree'ler elle `.claude/worktrees/rd2-*`
  altında açıldı, alt ajanlar oraya yönlendirildi. Ortak derleme klasörü `target/` (K56).
- **Oturumdaki tasarım becerileri** (round başı listesi):
  `frontend-design:frontend-design` (maket ve sözleşme işçiliği), `data:data-visualization` ve
  `dataviz` (grafik ve veri gösterimi, renk), `anthropic-skills:skill-creator` ve
  `superpowers:writing-skills` (proje becerisi), `artifact-design` (yalnız karşılaştırma sayfası
  biçimi için), `data:create-viz`, `data:build-dashboard` (kullanılmadı). Öncelik: depo kuralları >
  uyarlama envanteri ve sözleşme > beceri önerileri (BRIEF).

## 1. Ön işler (Ö1–Ö3) — main `0e0f53df`, denetim KABUL (`experiments/audit-prework.md`)

- **A62 (Ö1, `911a957c`):** eski `qp mcp --init/--principal/--surface/--root` bağdaştırıcısı
  kaldırıldı (`McpArgs`, `cmd/serve.rs::mcp()`, test `mcp_stdio_smoke` → `the_rd1_adapter_is_removed`).
  `qp mcp` artık yalnız `serve`. `human_decision.rs` iki yeni test: hiçbir sunulan yüzey (thin, task,
  query, mixed) ne ajana ne insana karar aracı vermiyor; `McpServer::connect`/`with_decisions`
  yalnız uygunluk koşucusundan çağrılıyor. MCP üzerinden uygunluk 70/70 süreç içinde sürüyor.
  **Kaldırılan döküm:** RG1 tutanağındaki `mcp/adapter.jsonl` artık üretilmiyor (`rg1-snapshot.sh`).
  Kalan eski çağrı: `rounds/R5B/experiments/harness/driver/test_qmcp.py` (deney, verify koşmuyor).
- **İ7 (Ö2, `4bab81b9`):** `fonts::FILES`, `cpu::FONT_BYTES` → `static`. Kabuk paketi
  (`--features replica`) gzip 2.570.017 → **2.408.899 B** (RG1 tabanı 2.506.216'nın %3,9 altında;
  denetçi yeniden derledi: 2.408.953); ham 9.101.210 → 8.731.379; brotli ≈ aynı. Varsayılan
  `qp_wasm.wasm` değişmedi.
- **Silmeler (Ö3):** 24 `recovered/worktree-agent/*` dalı (silmeden önce `recovered.md` ile
  karşılaştırıldı; uçlar raporun ekinde) silindi, `r5/m16-bertan`, `r5/m16-kivra` kaldı;
  `rounds/R3/experiments/harness/target/` (140 MB, izlenmiyordu) silindi.
- **İstisna mandalı (Ö3, `01b63fe1`):** `crate.lines` istisnası `max` taşır; aşılırsa hata. Tavanlar
  qp-session 31.999, qp-grammar 21.000 (K89), qp-capsule 13.230. Öz-sınamada üç yeni durum.
  Not: İ6 (fonksiyon boyu) istisnalarının tavanı yok; RF1'e öneri.

## 2. Sıra notu: ortak alfabe yönlerden önce

TASKS C2'yi (alışılmış grafik öğeleri) ara karardan sonraya koyuyor. Uygulamada C2'nin **ortak**
kısmı (P4–P8: fiyat ekseni yoğunluğu, iki düzeyli zaman, çakışma önleme, son değer + değer çizgisi,
imleç hapları, OHLC lejantı, hacim, işaret boyu) yönlerden önce, `rd2/c2a-alphabet` dalında kuruldu.
Gerekçe: üç yön aynı uyarlanmış alfabeyi paylaşır (GOAL §2) ve karşılaştırma başvuru ekranıyla
yan yana yapılır (P14); R5 düzeyindeki grafikle (tek eksen etiketi, hacim yok) yapılan bir
karşılaştırma VISUAL §3.2/6'daki karıştırıcı etkeni tekrarlardı. Kural korunur: alfabe görünüş
kaydının `ChartLook` alanlarıyla açılır, var olan görünüşler (r4, ornek, oltan, zemre) bayt eşit,
gösterim varsayılanı değişmez; **seçilen yön** ancak Arkin'in kararından sonra çekirdeğe işlenir.

## 3. Üç yön ve ara teslim (K1)

| Yön | Görünüş | Dal / commit | Odak | design-check (2 profil × 2 tema) | qp-grammar |
|---|---|---|---|---|---|
| A | `sakel`, `sakel-tel` | `rd2/y-a` `6d4f17f5` | sözleşmeye sadık, Qapsule katmanı en sessiz | P1–P13, H1–H3 PASS (P9 6/3/7) | +289 |
| B | `tavek`, `tavek-tel` | `rd2/y-b` `193147c0` | anlam grafiğin üstünde, grafiğin dilbilgisiyle | PASS (P9 5/3/7) | +285 |
| C | `ravek`, `ravek-tel` | `rd2/y-c` `7b6d05ca` | anlam akışta; sicil tek bakışta | PASS (P9 5/3/7) | +264 |

Üç yön aynı tabandan (`rd2/c2a-alphabet`), birbirini görmeden üç alt ajanla kuruldu. Galeri dalı
`rd2/gallery-all` (yalnız karşılaştırma; main'e girmez). Galeri: sentetik veri
`rounds/RD2/experiments/gallery/out/` (ignored), gerçek veri (GARAN, günlük) depo dışında
oturum klasöründe; aynı veri, aynı boyut, masaüstü DPR 2, telefon DPR 3, başvuru ekranı yanda,
`oltan` (R5) aynı düzenekte. Ajan eleştirmeni kullanılmadı; kusur taraması gözle yapıldı.

**Ara karar (Arkin, aynen; `DECISION.md` §1):** "En mantıklısı tavek gibi duruyor. ama basvurulardaki
openchart'ın inceleme ve akış şemaları hala daha iyi gozukuyor. yani orneklendirmelerimizde halen
ciddi eksikler var. geliştirmelerinin ileriki aşamalarında openchart ve openterminal tarafındaki
kodlar üzerinden de inceleme ve uyarlama dah aiyi olabilir." → seçilen yön **`tavek`**; elenen
yönlerin kodu `rd2/y-a` (`sakel`) ve `rd2/y-c` (`ravek`) dallarında, worktree'leri kaldırıldı.

## 4. Seçilen yönün çekirdeğe işlenmesi (C1–C3) ve cilalama

- **C1:** `tavek` (yoğun, masaüstü) ve `tavek-tel` (dokunma, telefon) görünüş kaydı, iki tema;
  `css_vars` RD2 tokenlarıyla (köken rolleri, ajan kesik çizgisi, tip ölçeği, kart yarıçapı, profil).
  CONTRACT değer tabloları kayıttan üretiliyor (`scripts/design-contract`, fark 0). Eşikler ara
  karardan sonra gözden geçirildi ve aynı bırakıldı (Arkin değişiklik istemedi).
- **C2 (ortak alfabe, `compile/alphabet/`):** sağ fiyat ekseni (hedef aralık 40–60 px; adım merdiveni
  1–1,5–2–2,5–3–4–5–7,5: yalnız 1/2/2,5/5 örnek veride 35 px veriyordu, gerekçe CONTRACT'ta), iki
  düzeyli zaman ekseni (gösterim saat dilimiyle), çakışma önleme (P6 = 0), son değer etiketi + noktalı
  değer çizgisi, imleç + fiyat/tarih hapları (`CompileOptions::cursor`, `qp render --cursor`; şema
  değişmedi), çekirdeğin kurduğu OHLC lejantı (O H L C; Türkçe A Y D K CONCEPTS'te olmadığı için
  alınmadı), soluk hacim, işaret ≤ 28 px. **C2b (`tavek`'e özgü):** her yatay seviye köken renginde
  ve çizgi biçiminde (insan düz, ajan kesik, motor noktalı) çizilir ve fiyat ekseninde etiketle
  biter; öncelik: son değer etiketi > görünüm sırasıyla seviye etiketleri.
- **C3:** `tavek` eşlik eden yüzü `zemre` (cümle + karne + fiyat şeridi; `vadrel` bu görünüşte
  grafiğini kaybediyordu, `oltan`'ın 120 px işareti reddedilmişti); yüz, akış, açılmış kapsül ve
  `korda` görünüş değerleriyle aynı dilde. Web kabuğu değerleri yalnız `css_vars` ile alıyor
  (`arch-check.mjs` yeşil). **Gösterim varsayılanı (`PRESENTED_LOOK`) değişmedi** — son kabule bağlı.
- **Cilalama:** P2 seviyeli sahnede de ölçülüyor (5/6 ≤ 6), güven işareti kendi satırında, telefonda
  oynatma çubuğu sığıyor, tasarım fikstürü SYN20 adıyla (`fixtures/design/docs/syn20.json`).
- **Grafik üstünde maket kalanlar** (gramer bütçesi bitti): seviyenin sol uç hapı (kim + durum),
  kırılım çentiği, kanıt bandı ve zaman ekseni parantezi, niyet hapı, `korda`'da doğumdan sağa
  sığmayan etiket.
- `shells/web/style.css`'te `.trust`'a eklenen zemin (`--qp-scene-bg`) `oltan`'da da görünüyor
  (altın ve tarayıcı adımları yeşil; denetim koşulu 3).

## 5. Yüzey şartnamesi ve maketler (M1–M3)

- `docs/design/SURFACES.md` + `docs/design/surfaces/{chart,agent,watch,ledger,phone,system}.md`:
  Y01–Y37'nin her biri: ne, kademe, yerleşim, bütün durumlar (tablo), köken, veri durumu, akış,
  masaüstü/telefon, uyarlanan kart, token rolleri. 22 varsayım (S1–S4, S7–S15, S16–S25) "Arkin'e
  sorular"da önerili cevaplarıyla.
- Maketler `docs/design/mockups/surfaces/{chart,agent,watch,system}/`: **kademe 1 tam** (Y01, Y04,
  Y05, Y06, Y07, Y09, Y11, Y12, Y18, Y25, Y26, Y27, Y31, Y32, Y33 — şartnamedeki bütün durumlar, iki
  profil; Y26/Y27 yalnız telefon yüzeyi), **kademe 2 ana durum + durum listesi** (Y02, Y03, Y08, Y10,
  Y13, Y14, Y16, Y17, Y20, Y21, Y23, Y24, Y34, Y35, Y36; masaüstü), **kademe 3 yalnız şartname**
  (Y15, Y19, Y28, Y29, Y30, Y37) + **Y22** (beceri sınaması olarak maketlendi). Her ana durumun
  yanında numaralı akış şeridi (Arkin'in "akış şemaları" notu). Grafikler çekirdekten (`qp render`),
  bindirmeler `qp scene` konumlarından. Renk taraması (`#…`, `rgb(`, `hsl(`) 0.
- **Başvuruya göre kalan açıklar** (alt ajanlar yan yana karşılaştırdı): yoğun profilde ajan paneli
  OpenChart'tan sıkışık okunuyor; besteci model/çalışma alanı seçicisi taşımıyor (şartnamede yok);
  araç sonucu kartının karşılığı yok (kanıt Y16'da açılıyor); maketler durum sayfaları, OpenChart
  hero'su gibi dolu çalışma ekranı değil; telefon sekme çubuğunda simge yok; Ç3 küçük önizlemede RSI
  lejantı çizginin üstünde (çekirdeğin küçük hücre davranışı).

## 6. Proje becerisi (B)

`.claude/skills/qapsule-design/SKILL.md` (≈ 170 satır; `skill-creator` yönergesiyle: tetikleyen
açıklama, "neden"li kurallar, değer taşımayan gövde). Kaynak tablosu (CONTRACT, `look/tavek.rs`,
`qp look show/css`, SURFACES, ADOPTED, DECISION), nerede ne yaşar, renk ve köken rolleri, iki profil,
durum kalıpları, grafik alfabesi, yapılmayacaklar, yeni yüzey kontrol listesi. `.gitignore`:
`/.claude/*` + `!/.claude/skills/`. **Sınama:** bağlamsız taze bir alt ajan yalnız beceriyi okuyarak
Y22'yi maketledi: `design-check` PASS, sözleşme farkı 0, renk taraması 0. Bulduğu dört boşluk
(klasör kuralı, kademe 3 sonrası, kesik çizgi tokeninin CSS'te kullanımı, galerinin yalnız çapaları
çekmesi) beceriye işlendi; iki öneri (ortak sayfa CSS'i, yüzey ekran görüntüsü aracı) "Öneriler"de.

## 7. İnsan testi paketi v4 (H)

`rounds/RD2/human-test/`: `GUIDE.md` (5 saniyelik bakış, "profesyonel araç mı" E/H, "güvenir miyim"
1–5, başvuru yanda, üç sabit sıra, kör kodlar), `SCREENS.md`, `FORM.md` + `form.csv`, `RESULTS.md`
(önce Arkin), `screens.sh` (görselleri adsız kodlarla depo dışına kopyalar) ve `blind.html` (sıralı,
5 sn sonra gizlenen gösterim). Kişi başı ≈ 10 dk.

## 8. Becerilerin kullanımı

| Beceri | Nerede |
|---|---|
| `frontend-design` | G (maket iskeleti), Y-a/b/c (yön maketleri), M2/M3 (yüzey maketleri): hizalama, tipografi ritmi, yoğunluk — yön için değil |
| `data:data-visualization` / `dataviz` | C2a (eksen yoğunluğu, etiket biçimi, renk ölçülülüğü), yönlerin renk kararları |
| `anthropic-skills:skill-creator` | B (proje becerisi) |
| `superpowers:*` | süreç (alt ajan dağıtımı); depo kuralları önce: worktree/derleme düzeni CLAUDE.md'deki gibi, push/PR yok |
| Ajan eleştirmeni | kullanılmadı; kusur taraması gözle ve denetçiyle |

## 9. Son kabul (K2) ve gösterim varsayılanı

Arkin (aynen; `DECISION.md` §2): **"Kabul, varsayılanı değiştir"**. Uygulama (`0abb1ba4`):
`PRESENTED_LOOK = "tavek"`; kabuk her zaman telefon yüzeyi olduğu için `phone_profile()` ile
`tavek-tel` + eşlik eden yüz `zemre` açılır (açıkça `?look=tavek` yazan okur masaüstü kaydını alır).
Protokol varsayılanı `r4`, şema, MCP, `qp-session` değişmedi. Eski varsayılana dayanan sınamalar
`?look=oltan`'a sabitlendi (`screens.mjs` v2/p4, `tests/settings.mjs` 4–5, `viewer_m15.rs`);
varsayılanı soran sınamalar `tavek-tel`/`zemre` bekler. **Yeni altınlar** (yalnız yeni görünüm):
`fixtures/golden/demo/rd2-*` (9 dosya: feed-tr-dark yüz 0–2, feed-en-light yüz 0–1, closed-tr-light,
capsule-tr-dark, chart-tr-dark, chart-en-light).
**Açık konu:** `viewer_m15` trafik sınırı (açılmış kapsül ≤ 400.000 B) yeni varsayılanla 505.752 B;
sınama `oltan`'a sabitlendi, bütçe `tavek-tel` için yeniden bakılmalı (R7/RF1 notu).

## 10. Ölçümler

**P1–P13 ve H1–H3** (`scripts/design-check`, iki tema; P3 yalnız koyu, P9 galeri dökümünden):

| # | `tavek` (masaüstü) | `tavek-tel` (telefon) | eşik |
|---|---|---|---|
| P1 krom kroması | 0,012 | 0,012 | ≤ 0,02 |
| P2 sahne tonu (desk / seviyeli) | 5 / 6 | 5 / 6 | ≤ 6 |
| P3 up/down (koyu) S, L | up 44/59, down 60/59 | aynı | S ≤ 65, L 40–60 |
| P4 fiyat etiketi aralığı / 300 px'te etiket | 51 px / 5 | 52 / 5 | 40–60 / ≥ 4 |
| P5 iki düzeyli zaman / 390 px'te etiket | evet / 6 | evet / 6 | evet / ≥ 4 |
| P6 çakışma + taşma | 0 | 0 | 0 |
| P7 son değer / değer çizgisi / imleç+hap / lejant | var / var / var / OHLC | var / var / var / OHLC (telefonda C kısa) | hepsi |
| P8 veri üstü işaret | 20 px (kayıt) | 28 px | ≤ 28 |
| P9 bölge / grafik bölmesi / lejant sayısı | 5 / 3 / 7 | — | ≥ 4 / 2 / 5 |
| P10 gövde / grafik / en küçük | 13 / 11 / 11 | gövde ≥ 15, en küçük 13, denetim 44 | profil |
| P11 tablo rakamı | 0 sapma | 0 | 0 |
| P12 denetim / kart yarıçapı | 6 / 8 | aynı | ≤ 8 / ≤ 12 |
| P13 kenar rolü / çizgi:zemin / denetim | 1 / 1,39 / 3,29 (koyu) | aynı | 1 / 1,2–1,6 / ≥ 3 |
| H1 metin, her zemin | 5,27 / 5,26 | aynı | ≥ 4,5 |
| H2 metin dışı, her zemin (gri dahil) | 3,04 / 3,06 | aynı | ≥ 3 |
| H3 anlam + köken ΔE00, 5 koşul | 11,3 / 10,7 | aynı | ≥ 10 |

P8'in "çizilen" değeri 0: fikstürde işaret (glyph) yok; P8 bugün kaydın değerini ölçüyor
(denetim koşulu 2).

**E1** (`experiments/E1/RESULTS.md`, bir kez): R5 düzeneği `compute` beş kontrolde geçti
(metin 5,08, kapanmış işaret 3,07, en küçük 11/13, telefonda 44 px altı hedef 0, ΔE00 en küçük
11,45). **Bütün öğeler (S/CONTRAST yöntemi) ilk ölçümde düştü:** gri rol 332/312 öğede 3:1'in
altında (en kötü 2,41); düzeltmeden sonra eşik altı **0**, en kötü metin 5,48, metin dışı 3,30 /
3,04. **K52** (`calibrate.mjs` PHONE, `tavek-tel`): JIT açık tik/yüz/sahne p95 1/1/3 ms, ilk kapsül
215 ms; JIT kapalı uzun kare %0,8 (sınır %10) — tutuyor. **K17:** hesap kodu değişmedi; en kötü yerel
ortanca 9,36 ms, WASM 18,59 ms; özet `7b1e99d40889f83c` aynı. **WASM:** kabuk paketi gzip 2.431.842 B
(RG1 tabanının %2,97 altında, round başına göre +%0,95; hedef ≤ +%5 tutuyor), varsayılan modül +%3,09.

**Önce / sonra görseli:** `rounds/RD2/experiments/E1/before-after.png` (R5 `oltan` → `tavek`, Ç1 ve Ç3,
sentetik veri).

**Yapı:** qp-grammar test dışı 19.330 → 20.964 (tavan 21.000); qp-session 31.999 (değişmedi); arch-check
0 hata; taban çizgisi yalnız sıkılaştı (file.soft 37 → 36, `compile` 654 → 564).

## 11. Kesilenler

GOAL kesme sırasından **hiçbir madde kesilmedi** (kademe 3 şartname olarak zaten kapsamdaydı; açık
tema maketleri ve kademe 2'de telefon profili de kısmen var). Bütçe yüzünden yapılmayanlar:
- Grafik üstünde seviyenin sol uç hapı, kırılım çentiği, kanıt bandı/parantezi, niyet hapı çekirdekte
  değil, maket + şartnamede (qp-grammar tavanı).
- Kademe 2 maketlerinde telefon profili (kesme sırası 3) çoğunda yok; yalnız masaüstü.
- `korda` etiket yerleşimi (doğumdan sağa sığmayan etiket kısalıyor) — gramer işi.
- İmleç varken gösterge lejantlarının imleç barını izlemesi.

## 12. R7 ve RF1 için fark edilenler

- **RF1:** qp-grammar tavanı (≈ 36 satır kaldı) R7'yi engeller; İ4 (kaydın enjeksiyonu, tasarım
  klasörlerinin — faces, open, stories, glyph, look — ayrı crate'e çıkması) RF1'in ilk işi olmalı.
  İ6 fonksiyon boyu istisnalarının tavanı yok (mandal yalnız `crate.lines`'ta). `look.rs` "Adding a look"
  notu "yalnız bu dosyada" diyor, gerçekte `look/<ad>.rs` + `registry.rs`.
- **R7:** maket kalan grafik öğeleri (sol uç hapı, kanıt parantezi, niyet hapı) çekirdeğe; lejantın
  imleci izlemesi; ajan paneli yoğunluğu başvuruya göre sıkışık (Arkin'in notu); besteci model/çalışma
  alanı seçicisi ve araç sonucu kartı şartnamede yok; seviye etiketlerinin kesik/noktalı çerçeveli
  hali; `viewer_m15` trafik bütçesi `tavek-tel` için; telefon sekme simgeleri.
- **Ortak:** maket grupları ~200 satırlık sayfa iskeletini kopyalıyor (ortak `sheet.css` önerisi);
  yüzey sayfaları için ekran görüntüsü aracı yok (galeri yalnız çapaları çekiyor); renk taraması
  `color-mix` yüzdelerini ve opaklıkları yakalamıyor.
- Grammar `display_tz`'yi yalnız görünümden alıyor; dokümanın saat dilimi ve borsa takvimi oturumdan
  geçirilmeli (yoksa UTC).

## 13. Öneriler (kapsam dışı; yapılmadı)

1. A65: kod düzeyinde inceleme (Arkin'in notu) — temiz oda içinde, derin okuma + davranış şartnamesi.
2. `design-gallery`'ye yüzey sayfaları kipi ve ortak sayfa CSS'i.
3. CONTRACT'a `strip_alpha`, `focus_row_alpha`, `grey_plate_alpha_max`, `closed_label` tabloları
   (bugün yalnız düzyazıda; denetim notu b).
4. P8 için fikstüre bir işaret (glyph) ekle; seviyeli sahneyi de ölç.
5. `--real` galeri çıktısı varsayılan olarak `out/` altında kalsın (özel `--out` ignore dışına düşebilir).
6. `docs/CONCEPTS.md`: *çizim*, *hazırlık görevi*, *kurumsal işlem*, *sağlık* (SURFACES S20).
7. `rounds/R5B/experiments/harness/driver/test_qmcp.py` eski `qp mcp --init` yolunu çağırıyor.

## 14. Arkin'e sorular

- A65–A68 (`docs/decisions/rounds/RD2.md`), Ö109–Ö118 onayı.
- SURFACES "Arkin'e sorular" S1–S25 (en etkilileri: S14 ilk gün ajan basamağı "Öner"; S4 "kırıldı" =
  çizimin periyodunda bar kapanışı; S16 kilit ekranında lisanslı rakam yok; S21 grafikte yalnız seçili
  ya da durumu değişen seviyenin hapı).
- Bir olay: gri düzeltmesini yapan alt ajan `pkill -f "qp demo"` çalıştırdı ve bu oturuma ait olmayan,
  12 saattir açık `target/release/qp demo --web shells/web --mcp` sürecini kapattı (büyük olasılıkla
  `qapsule-demo` MCP sunucusu). Yeniden açmak: depo kökünden aynı komut.

## 15. İnsan testini nasıl yürüteceksin

`rounds/RD2/human-test/GUIDE.md`: `scripts/design-gallery --looks tavek`, sonra
`rounds/RD2/human-test/screens.sh` (görselleri adsız kodlarla `~/qp-ht-v4/` altına kopyalar),
`blind.html` ile her ekran 5 saniye; "profesyonel araç mı" (E/H), "güvenir miyim" (1–5), üç açık soru;
telefon görselleri (K5–K8, B3, B4) telefona. En az iki işlem yapan kullanıcı; kişi başı ≈ 10 dk;
sonuçlar `human-test/RESULTS.md` (Arkin önce, kör değil, ayrı raporlanır).
