# RD2 — Bağımsız son denetim (D)

Denetçi: bağımsız denetçi alt ajanı (işi yapmadı), 2026-11-23. Denetlenen durum: dal
`rd2/presented` = `b5098203` (RD2'nin bütünü: kapanış belgeleri `0bd70994` ve gösterim
varsayılanı geçişi `0abb1ba4` dahil), worktree `.claude/worktrees/rd2-pres`. Round başı
`9aee506c`. Derleme klasörü özel: `target/wt/audit-final` (denetim sonunda silindi). Karalama
yalnız oturum klasöründe, sonra silindi. Galeri çıktısı oturum klasörüne yazıldı ve silindi.
Denetçi yalnız bu dosyayı yazdı; push/birleştirme yok; süreç öldürülmedi.

## Hüküm: **KOŞULLU KABUL**

Engelleyici bulgu yok. `verify.sh` benim koşumumda **20/20 PASS** (`design` dahil, 1.022 s,
`verify: OK`); `arch-check` 0 hata, `qp-session` 31.999 (round boyunca `crates/qp-session`
diff'i boş), `qp-grammar` 20.964 ≤ 21.000, taban çizgisi yalnız sıkılaştı; var olan altın,
döküm ve hesap özeti değişmedi (yalnız 9 yeni `rd2-*` altın); protokol/şema değişmedi, MCP
yüzeyinde yalnız A62'nin kaldırdığı yol gitti; Arkin'in iki kararı `DECISION.md`'de aynen ve
tarihli. Kabulü koşullu yapanlar: (1) GOAL 3'te `design-check` P9'u ölçmüyor (`n.a.`; galeri
düzen dökümünden) ve P8'in sahne yarısı boş (çizilen 0, fikstürde işaret yok) — REPORT bunu
dürüstçe söylüyor ama CONTRACT'ta "Arkin onaylı gerekçe" olarak yazılı değil; (2) kapanış
temizliği henüz yapılmadı (iki worktree, `target/` 64 GB, artık `target/wt/rd2-polish` 12 GB) —
bu sıra gereği denetimden sonra ana oturumda; (3) küçük belge tutarsızlıkları (B4–B7).

## 1. VERIFY.md (15 satır)

| # | Hüküm | Kanıt |
|---|---|---|
| 1 | PASS | `CARGO_TARGET_DIR=target/wt/audit-final scripts/verify.sh`: 20 adımın hepsi PASS, `verify: OK`, çıkış 0. `design`: "non-legacy looks pass P1-P13 and H1-H3; contract tables equal the look record". `demo`: 98 görsel ×2, `rd2` altınları dahil (`screens.mjs:101–106`). `test` 555 s, `wasm` ham 2.345.901 / gzip 769.894 B, `digest` arm64 = x86_64 = wasm32 = `7b1e99d40889f83c`, `conform` 70 × 3 taşıma. |
| 2 | PASS | `scripts/arch-check`: "0 error(s); 11 named exception(s), 0 awaiting approval; within the baseline: … file.soft 36, fn.lines 177 …". `--table`: `qp-session` 31.999 (tavan 31.999; `git diff ec1c7c89 HEAD -- crates/qp-session` boş), `qp-grammar` 20.964 (tavan 21.000), `qp-capsule` 13.230. `git diff 9aee506c -- scripts/arch-baseline.json`: yalnız sıkılaşma (file.soft 37 → 36, `scene.rs` çıktı; `compile` 654 → 564). `arch-config.json`: yalnız İ3–İ5'e `max` mandalı (GOAL 0'ın istediği; K89). |
| 3 | PASS | `git diff --name-status 9aee506c HEAD -- fixtures/golden fixtures/compute fixtures/protocol fixtures/views`: 9 satırın hepsi `A` (`fixtures/golden/demo/rd2-*`); değişen/silinen var olan dosya yok. A62'nin kaldırdığı `mcp/adapter.jsonl` RG1 tutanağına aitti, bu klasörlerde değil (REPORT §1'de adıyla). |
| 4 | PASS | `qp mcp --help`: yalnız `serve`; `qp mcp --init x --principal arkin` → "unexpected argument '--init'". `human_decision.rs` 5/5 (`the_rd1_adapter_entry_point_no_longer_exists`, `no_served_surface_offers_a_decision_tool_even_to_a_person` …); `qp-cli` `the_rd1_adapter_is_removed` ok. `qp-mcp/src` değişikliği yalnız yorum. |
| 5 | PASS | E1 (`experiments/E1/RESULTS.md` §4): kabuk paketi gzip 2.431.842 B < 2.506.216 (−%2,97); round başına göre +%0,95, varsayılan modül +%3,09 (hedef ≤ +%5). Kabuk paketini yeniden ölçmedim (pahalı); varsayılan modül benim koşumumda gzip 769.894 B (E1: 769.827; gri düzeltmesi + varsayılan geçişiyle +67 B). |
| 6 | PASS | `git branch --list 'recovered/*' \| wc -l` = 0; `ls rounds/R3/experiments/harness/target` → yok. `r5/m16-bertan`, `r5/m16-kivra` duruyor. |
| 7 | PASS | `scripts/design-check` çıkış 0: `tavek` (dense) ve `tavek-tel` (touch) iki temada PASS; P1 0,012; P2 5/6; P4 51/5; P5 evet/6; P6 0; P7 hepsi; P8 20 (kayıt) / 0 (çizilen); P10 13/11/11; P11 0; P12 6/8; P13 1,39/3,29; H1 5,27/5,26; H2 3,04/3,06; H3 11,3/10,7. **P9 `n.a.`** (galeriden; bkz. B1). |
| 8a | PASS | ADOPTED ilk commit `9e7c5bd4` 2026-11-22 20:52; yön dallarının ilk yön commit'leri 22:30–22:38 (`7b6d05ca`, `193147c0`, `6d4f17f5`); `9e7c5bd4` üç yön dalının da atası. Kopya taraması: RD2'nin eklediği 3.401 ayrı kod/CSS/HTML satırı (≥ 30 karakter) sekiz başvuru projesinin 7.511 kaynak dosyasıyla satır satır: 19 eşleşme, hepsi genel CSS deyimi (`font-variant-numeric: tabular-nums;`, `justify-content: space-between;`, `grid-template-columns: 1fr 1fr;` …). `tavek.rs`'in 52 rengi hex olarak başvuru kaynaklarında: yalnız `#ffffff`. Değer tablosu kopyası yok. |
| 8b | PASS | `.claude/skills/qapsule-design/SKILL.md` (11,4 KB) izleniyor. Taze ajan sınaması `61323e9c` (Y22 maketi + `system.css`); `system.css`'te renk kodu 0, `design-check` yeşil, REPORT §6'da bulunan dört boşluk ve beceriye işlenişi. |
| 8 | PASS | `scripts/design-contract --check`: "generated tables equal the look record (difference 0)". |
| 9 | PASS | `scripts/design-gallery --looks tavek --anchors c1,c3 --out <oturum>` (31 s): Ç1 2880×1800 (1440×900 CSS, DPR 2), Ç3 1170×2532 (390×844 CSS, DPR 3), `tavek` ve `oltan` aynı fikstür (`fixtures/design/desk.json`) ve aynı boyut; `compare.html` etiketleri "1440×900 CSS, DPR 2", "390×844 CSS, DPR 3"; başvuru ekranları yanda, yerinde okunuyor (OpenChart `dashboard-preview.png` "3836×2102 px (≈ DPR 2,66)", OpenTerminalUI `research-mobile.png` "390×844 (DPR 1); kısmi" — farkı dürüstçe etiketli). `layout-tavek.json` 5/3/7. Çıktı silindi. Üç yönlü galeri `rd2/gallery-all` dalında (main'e girmez). |
| 10 | PASS | `DECISION.md` §1 ara karar 2026-11-22, §2 son kabul 2026-11-23; ikisi de "aynen" alıntı ve oturumun okuması ayrı başlıkta. |
| 11 | PASS | Y01–Y37'nin her biri `docs/design/surfaces/*.md`'de başlıklı; her bölümde durum tablosu, **Köken**, **Veri durumu**, **Masaüstü / telefon** var (bölüm bölüm tarandı, eksik 0). |
| 12 | PASS | Kademe 1 (15) ve kademe 2 (15) maketlerinin hepsi `docs/design/mockups/surfaces/*/Y??.html`; + Y22 (beceri sınaması). Kademe 1'de iki profil başlığı (Masaüstü/Telefon) var; örnek: Y31'in şartnamedeki 10 durumunun hepsi makette. Renk taraması (`#hex`, `rgb(`, `hsl(`) `docs/design/mockups` altında 0; grup CSS'lerinde `var(--qp-…)` 65–212 kullanım. |
| 13 | PASS | E1: R5 düzeneği `compute` 5/5; bütün öğeler ilk ölçümde **332/312 eşik altı ile KALDI** (gri rol), düzeltmeden sonra 0 (en kötü metin 5,48, metin dışı 3,30/3,04); K52 `tavek-tel` JIT açık 1/1/3 ms, ilk kapsül 215 ms, JIT kapalı uzun kare %0,8; K17 en kötü 9,36 / 18,59 ms, özet aynı. Düzeltme sonrası `design-check` H1/H2 artık her zemini ölçüyor ve benim koşumumda tutuyor. |
| 14 | PASS | RD2'nin eklediği görseller: 9 `rd2-*` altın + `rounds/RD2/experiments/E1/before-after.png` (799.129 B ≤ 1 MB, sentetik SYN20). `git ls-files fixtures/local` boş. Başvuru görseli izlenmiyor (galeri `file://…/refs` yerinde okuyor). `--ignored` özetinde beklenmeyen yok (`target/web-spike` 1,7 MB worktree'de — `shells/web-spike/run.sh` yazar, worktree ile gider). |
| 15 | KOŞULLU | `git worktree list`: ana + `rd2-gal` + `rd2-pres` (kapanış temizliği bu denetimden sonra; ana oturum doğrulayacak). `du -sh target` = **64 GB** (benim 14 GB'lık `wt/audit-final` dahil; silindi) — içinde artık `target/wt/rd2-polish` **12 GB** (K87: iş bitince silinir); ≤ 6 GB için `clean.sh --deep` gerekli. REPORT, STATUS, ROADMAP güncel; küçük tutarsızlıklar B4–B6. |

## 2. GOAL.md kabul ölçütleri (15)

| # | Hüküm | Kanıt |
|---|---|---|
| 1 | PASS | A62 (`911a957c`) + test; İ7 (`4bab81b9`) gzip 2.570.017 → 2.408.899 (denetçi ön iş: 2.408.953); 24 dal ve R3 derlemesi silindi; mandal `01b63fe1` ve öz-sınama (`verify` adım 19). `audit-prework.md` KABUL. |
| 2 | PASS | ADOPTED (78 kart, yönlerden önce), CONTRACT (tablolar kayıttan, fark 0); kopya taraması 0 (8a); beceri yazıldı ve sınandı (8b). |
| 3 | KOŞULLU | `design-check` `verify.sh` adımı ve `tavek`/`tavek-tel` geçiyor. Ama P9 `design-check`'te `n.a.` (galeri dökümünden: 5/3/7) ve P8'in "çizilen" değeri 0 (fikstürde işaret yok; yalnız kayıt değeri ölçülüyor). REPORT §10 ve CONTRACT satır 66 bunu söylüyor; "Arkin'in onayıyla sözleşmede gerekçeli" değil (B1). |
| 4 | PASS | Üç yön (`sakel`/`tavek`/`ravek`), dört çapa, iki tema, aynı veri/boyut/DPR, `oltan` ve başvuru yanda; ara karar aynen `DECISION.md` §1. Ajan eleştirmeni kullanılmadı (REPORT §3, §8; E1 eleştirmen aşamalarını koşmadı). |
| 5 | PASS | `look/tavek.rs` (`tavek` Dense, `tavek-tel` Touch), iki tema `design-check`'te; `css_vars` RD2 tokenları. |
| 6 | PASS | `compile/alphabet/`: P4–P8 sahneden ölçülüyor, P6 = 0 (masaüstü + telefon + seviyeli sahne). P8'in çizilen yarısı boş (B1). |
| 7 | PASS (not) | `zemre` yüzü, akış, açılmış kapsül ve `korda` `tavek` değerleriyle; yeni altınlar `rd2-feed-*`, `rd2-closed-*`, `rd2-capsule-*`, `rd2-chart-*` bunu sabitliyor. Not: `korda` etiket yerleşimi kısalıyor (CONTRACT "Bilinen sınır"). |
| 8 | PASS | VERIFY 11. |
| 9 | PASS | VERIFY 12. Kademe 2'de telefon profili çoğunda yok (kesme sırası 3; REPORT §11'de yazılı). |
| 10 | PASS | VERIFY 13; ilk geçişteki başarısızlık ve düzeltme açıkça raporlanmış. |
| 11 | PASS | VERIFY 5, 13. |
| 12 | PASS | VERIFY 3; yeni altınlar REPORT §9'da listeli (9 dosya). Var olan altına dayanan sınamalar `?look=oltan`'a sabitlendi (`screens.mjs` p4, `settings.mjs`, `viewer_m15.rs`); hesap özeti `7b1e99d40889f83c` aynı. |
| 13 | PASS | VERIFY 2. |
| 14 | PASS (not) | `rounds/RD2/human-test/` (GUIDE, SCREENS, FORM + form.csv, RESULTS, screens.sh, blind.html). Son kabul `DECISION.md` §2: soru "profesyonel ve güvenilir bulup kabul ediyor musunuz"; cevap aynen "Kabul, varsayılanı değiştir". Not: Arkin'in P15 formu (`RESULTS.md` §1) boş; öbür kullanıcılar sonradan (GOAL buna izin veriyor) (B7). |
| 15 | KOŞULLU | Bu dosya; REPORT/STATUS/ROADMAP yazıldı; kapanış temizliği bekliyor (VERIFY 15). |

## 3. Bulgular

- **B1 (GOAL 3, orta).** `design-check` P9'u ölçmüyor (`n.a.` her görünüşte; değer
  `design-gallery`'nin `layout-<look>.json`'undan) ve P8'in sahne ölçümü boş (`record / drawn`
  = 20 / 0; `fixtures/design/desk.json`'da Glyph yok). `audit-cand.md` koşul 2 REPORT'a not
  düşülerek karşılandı ama ölçüm eklenmedi. CONTRACT'ın "Ölçüm" bölümü P8 için "sahnede çizilmiş
  en yüksek işaret" diyor, gerçekte 0 ölçülüyor ve bu orada yazmıyor.
- **B2 (bilgi, dürüstlük iyi).** REPORT başarısızlıkları saklamıyor: E1 ilk geçiş 332/312 eşik
  altı (§10), `viewer_m15` 505.752 B > 400.000 ve `oltan`'a sabitleme (§9, §12), P8 çizilen 0
  (§10), alt ajanın `pkill -f "qp demo"` ile 12 saatlik dış `qp demo --web shells/web --mcp`
  sürecini kapatması (§14; STATUS'ta da), kesilen/kalan işler (§11, §12). Not: `viewer_m15`
  sabitlemesi, yeni varsayılanın trafik bütçesini (açılmış kapsül ≤ 400.000 B) bugün hiçbir
  sınamanın tutmadığı anlamına geliyor — yeni varsayılan `tavek-tel` bu bütçeyi **aşıyor**.
  Gerileme bir karar satırı istemeli (R7/RF1'de bütçe ya da yük düzeltmesi).
- **B3 (bilgi).** Gösterim varsayılanı geçişi (`0abb1ba4`) var olan sınamaların beklentisini
  değiştirdi (`s_int.rs`, `shell_s1.rs`: beklenen yüz artık `companion_face(phone_profile(…))`).
  Bu, Ö69/Ö79 kalıbındaki "varsayılanı soran sınama yeni varsayılanı bekler" değişikliği;
  altın ve döküm değişmedi. Kabul edilebilir.
- **B4 (belge).** `REPORT.md` başlığı hâlâ "rapor (yazılıyor)".
- **B5 (belge).** `STATUS.md` satır 10–11 eski: "Sıradaki adım: RD2'yi başlat …" ve K81 sırası
  (RL1 → R7), satır 8 ise "RD2 tamamlandı, sıradaki RF1" diyor.
- **B6 (belge).** `docs/decisions/rounds/RD2.md` A66 `qp-grammar` 20.951 diyor; ölçülen 20.964
  (REPORT ve STATUS doğru). STATUS/DECISION "30 yüzey maketi" diyor; dosya sayısı 31 (30 + Y22
  sınaması) — tutarlı ama belirtilmeli.
- **B7 (GOAL 14, küçük).** `human-test/RESULTS.md` §1'de Arkin'in P15 cevapları (5 sn bakış,
  E/H, güven 1–5) boş; son kabul cümlesi `DECISION.md`'de. GOAL "Arkin'in kendi cevabıyla
  kapanır" şartı karşılanıyor; form doldurulursa insan testi verisi tam olur.
- **B8 (disk, kapanış).** `target/wt/rd2-polish` 12 GB ve `rd2-polish-scratch` artık (K87);
  `target/` 64 GB (benim klasörüm silinince ≈ 50 GB) > 6 GB bütçesi.
- **B9 (bilgi).** `audit-c2a` koşul 1 (`display_tz` yoksa borsa takvimi) kısmen: gramer saat
  dilimini yalnız görünümden alıyor, dokümanınki ve borsa takvimi oturumdan geçmiyor (REPORT
  §12'de açık yazılı; protokol/oturum işi olduğu için bu round'da yapılamazdı).

BRIEF kuralları: ajan eleştirmeni puanı seçimde kullanılmadı (PASS); DPR kuralları galeride
uygulanıyor ve etiketli (PASS); `../refs/src` kodu kopyalanmadı (PASS, 8a); R7 davranışı
kurulmadı — kod değişikliği gramer/görünüş/kabuk değerleri, `qp render --cursor`, A62 ve
sınamalarla sınırlı; çizim aracı, izleme görevi, niyet kartı davranışı yok, yalnız maket ve
şartname (PASS). Protokol/şema: `git diff 9aee506c HEAD -- crates/qp-schema docs/PROTOCOL.md
fixtures/schema` boş.

## 4. Koşullar

1. **K1 (B1):** ya fikstüre bir Glyph işareti eklenip P8'in çizilen değeri ölçülsün ve P9
   `design-check`'e (ya da galeri dökümünün `verify`'da okunmasına) bağlansın, ya da CONTRACT
   "Ölçüm" bölümüne bugünkü sınır (P8 yalnız kayıt, P9 galeriden) yazılıp Arkin'in onayı
   (karar satırı ya da A satırı) alınsın. Birleştirmeye engel değil; RF1/R7'nin ilk gramer
   işinden önce.
2. **K2 (VERIFY 15, B8):** kapanış temizliği: `rd2-gal` ve `rd2-pres` worktree'leri kaldırılsın,
   `target/wt/rd2-polish*` silinsin, `scripts/clean.sh` (gerekirse `--deep`) ile `target/` ≤ 6 GB;
   sonuç REPORT "Disk" bölümüne üç listeyle.
3. **K3 (B4–B6):** REPORT başlığı, STATUS satır 10–11 ve A66 sayısı düzeltilsin (ana akış
   belgesi; tek "ana akış:" commit'i yeter).
4. **K4 (B2):** `viewer_m15` trafik bütçesinin yeni varsayılanda aşıldığı bir A satırı olarak
   kayda geçsin (bugün yalnız REPORT §12'de).

## 5. RF1 ve R7 için notlar

- **RF1:** `qp-grammar` 20.964 / 21.000 — 36 satır pay; İ4 (kayıt enjeksiyonu, `faces`, `open`,
  `stories`, `glyph`, `look` ayrı crate) RF1'in ilk işi olmalı, yoksa R7'nin grafik öğeleri
  (sol uç hapı, kanıt parantezi, niyet hapı, lejantın imleci izlemesi) sığmaz. İ6 fonksiyon boyu
  istisnalarının mandalı yok (`prepare` 891, `expand_op` 910 satırlık dosyalar). `look.rs`
  "Adding a look" notu eskimiş (`look/<ad>.rs` + `registry.rs`).
- **R7:** `viewer_m15` bütçesi `tavek-tel` için (505.752 B); maket kalan grafik öğeleri;
  ajan paneli yoğunluğu (Arkin'in notu, A65 kod düzeyinde inceleme); kademe 2 maketlerinde
  telefon profili; `korda` etiket yerleşimi; renk taraması `color-mix` ve opaklığı yakalamıyor;
  yüzey sayfaları için ekran görüntüsü aracı yok (galeri yalnız çapaları çekiyor).
- `display_tz` ve borsa takvimi oturumdan gramere geçmeli (B9).

## 6. Denetçinin izi

- Komutlar: `scripts/verify.sh` (tam), `scripts/arch-check` (+ `--table`),
  `scripts/design-check`, `scripts/design-contract --check`,
  `scripts/design-gallery --looks tavek --anchors c1,c3 --out <oturum>`, `qp mcp --help`,
  `git diff`/`git log` sorguları, başvuru kaynaklarına karşı satır ve renk taraması (Python,
  `-I`, oturum klasöründe).
- Silinen: `target/wt/audit-final` (14 GB), oturum klasöründeki galeri çıktısı, tarama dosyaları
  ve `verify.log`.
