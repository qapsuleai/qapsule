# Karar Günlüğü — Önerilen (RD2) — Arkin'in onayını bekliyor (2026-11-23)

<!-- dizin: DECISIONS.md; RD2 tasarım turunun önerdiği satırlar -->

RD2 bu satırları **önerir**; kesinleştirmez. Arkin'in iki kararı (yön `tavek`, son kabul ve
gösterim varsayılanı) `rounds/RD2/DECISION.md`'de kendi sözleriyle. Kanıt `rounds/RD2/REPORT.md`,
`AUDIT.md`, `experiments/`.

| # | Önerilen karar | Kanıt |
|---|----------------|-------|
| Ö109 | **Görsel dil `tavek`** (yön B: anlam grafiğin üstünde, grafiğin dilbilgisiyle): `tavek` (yoğun, masaüstü) ve `tavek-tel` (dokunma, telefon) görünüş kayıtları gösterim varsayılanıdır; protokol varsayılanı `r4` kalır. `oltan` seçilebilir kalır (K57'de reddedildi). | DECISION.md §1–§2; design-check P1–P13, H1–H3; E1. |
| Ö110 | **Profil = ayrı görünüş kaydı:** bir yön `<ad>` (Dense) ve `<ad>-tel` (Touch) olarak kaydolur; şema ve protokol değişmez; telefon yüzeyi `-tel` kaydını kullanır. | `look/chart.rs`, `look/registry.rs`; GRAMMAR §9a. |
| Ö111 | **Tasarım sözleşmesi** `docs/design/CONTRACT.md`: kural başına bir cümle; değer tabloları görünüş kaydından üretilir (`scripts/design-contract`, fark 0 denetimi `design-check`'te). Kayıtta olmayan sabit alfabe ölçüleri adıyla listelenir ve testle kodla eşitlenir. | CONTRACT; `ALPHABET_FIXED`. |
| Ö112 | **`scripts/design-check`** (`verify.sh` adımı `design`): P1–P13 (VISUAL §5) ve H1–H3 (R5 sert eşikleri, her zemine karşı) kayıt ve sahneden ölçülür; yalnız eski olmayan görünüşler düşürür. Eşikler başlangıç değerleridir; ara karardan sonra gözden geçirildi ve aynı kaldı. | design_check testleri; `a_bad_non_legacy_look_fails`. |
| Ö113 | **Ortak grafik alfabesi gramerdedir** (`compile/alphabet/`, `ChartLook` ile açılır): sağ fiyat ekseni 40–60 px (1–1,5–2–2,5–3–4–5–7,5 merdiveni), iki düzeyli zaman ekseni (gösterim saat dilimiyle), çakışma önleme, son değer etiketi + noktalı değer çizgisi, imleç + eksen hapları (`CompileOptions::cursor`), çekirdeğin kurduğu OHLC lejantı, soluk hacim, işaret ≤ 28 px. Eski görünüşler bayt eşit. | C2a, C2a-fix; audit-c2a. |
| Ö114 | **Köken rolleri ve biçimi:** insan / ajan / motor üç renk rolü (`--qp-origin-*`) ve çizgi biçimi (düz / kesik / noktalı); `tavek`'te yatay seviyeler köken renginde, fiyat ekseninde etiketle biter (öncelik: son değer etiketi > görünüm sırası). Köken, seviyenin dayandığı olgunun yazarından (onaylanmış öneride onaylayan). | C2b; `tests/design_check/levels.rs`. |
| Ö115 | **`qp-grammar` → `qp-calendar` bağımlılığı** (çekirdek katmana aşağı; gösterim saat dilimi için). Dış bağımlılık yok. | C2a-fix; ENGINEERING §5 "yeni bağımlılık karar satırı ister". |
| Ö116 | **Yüzey şartnamesi ve maketler:** `docs/design/SURFACES.md` (+ `surfaces/`) Y01–Y37'nin tek kaynağıdır; maketler `docs/design/mockups/` altında yalnız `css_vars` ile (renk kodu taraması 0), mantık taşımaz. | M1–M3; denetim. |
| Ö117 | **Proje becerisi `qapsule-design`** (`.claude/skills/`, `.gitignore` istisnası): arayüz işi yapan her alt ajan önce onu yükler; değer taşımaz, kayda ve CONTRACT'a işaret eder. | B; taze ajan sınaması (Y22). |
| Ö118 | **Uyarlama envanteri** `docs/design/ADOPTED.md` (78 kart) RD2 sonrası arayüz işinde başvurudur; yeni uyarlama yeni kart ister. | U. |

## Açık (Arkin'e)

| # | Soru | Öneri |
|---|------|-------|
| A65 | Arkin'in ara karar notu: OpenChart ve OpenTerminalUI **kodu üzerinden** inceleme ve uyarlama. K83 bugün kod kopyalamayı yasaklıyor. MIT/Apache projelerinde "kod okunur, davranış şartnameye yazılır, bizim kurallarımızla yeniden kurulur" mu, yoksa sınırlı kod uyarlaması (atıf ve lisans dosyasıyla) mı? | Temiz oda korunsun; ama okuyucu ajanlar kodu derinlemesine okuyup davranış şartnamesi (akış, durum makinesi, ölçü mantığı) çıkarsın; R7'nin ajan paneli ve çizim akışı için ilk iş. |
| A66 | `qp-grammar` 20.964 / 21.000: sol uç hapı, kanıt parantezi, niyet hapı, lejantın imleci izlemesi, `korda` etiket yerleşimi gramer satırı ister. | RF1'de kayıt enjeksiyonu ve tasarım klasörlerinin ayrı crate'e çıkması (İ4) önce; sonra R7. |
| A67 | Fiyat ekseni adım merdiveni 7,5 ve 1,5 içeriyor (yalnız 1/2/2,5/5 ile örnek veride 35 px). Kabul mü, yoksa P4 eşiği 35–60 px'e mi insin? | Merdiven kalsın; gerçek veride ayrıca bakılsın. |
| A68 | SURFACES "Arkin'e sorular" (S1–S25; en etkilileri S14 ilk gün ajan basamağı "Öner", S4 "kırıldı" = kendi periyodunda bar kapanışı, S16 kilit ekranında rakam yok). | Şartnamedeki önerili cevaplar kabul; R7/RS1'de ölçülsün. |
| A69 | Yeni gösterim varsayılanıyla (`tavek-tel`) açılmış kapsülün görüntüleyici trafiği 505.752 B; `viewer_m15` sınırı 400.000 B. Sınama `oltan`'a sabitlendiği için bu bütçeyi bugün yeni varsayılanda hiçbir sınama tutmuyor. | R7 başında ölçülsün: ya bütçe `tavek-tel` için gerekçeyle güncellensin ya da açılmış kapsülün sahne boyu küçültülsün; o zamana kadar sınama `oltan`'da kalır. |
