# R4 — Evre 1 bağımsız denetimi (VERIFY-E1)

Denetlenen: `r4/int` 6b0c3f1 (T2 yeniden oynatma, T3 K42/süre ifadesi/ergonomi, T4 etkileşim).
Denetçi, işi yapan ajanlardan ayrı bir alt ajan; kod değiştirilmedi. Denemeler kendi
senaryolarımla, ayrı bir sonda crate'inde (`verify-e1/probe/`, çalışma alanına üye değil, depo
crate'lerine yol bağımlılığıyla) ve `qp serve` / `qp replay` üzerinden yapıldı. Ham çıktılar
`rounds/R4/experiments/verify-e1/` altında (`out-*.txt`, `leafdiff.txt`, `ergonomics.txt`,
`verify-summary.txt`, `replay-local-stats.txt`). Yeniden üretme:
`CARGO_TARGET_DIR=<worktree>/target cargo run --release --manifest-path rounds/R4/experiments/verify-e1/probe/Cargo.toml --bin <elapsed|replay|k42|a22|validity|interact|edges>`.

## Özet tablo

| # | Konu | Sonuç | Kanıt (kısa) |
|---|------|-------|--------------|
| 1a | T2: aynı veri + hız + denetim → bayt eşit akış (kendi dizim: adım, tarihe ileri, 5×tek adım, hız, sürdür, düzensiz pump, duraklat, izleyicinin `forbidden`'ı, zamanla geri seek, yeniden `hello` + kitle, ileri seek) | PASS | 257 satır / 388 254 B iki koşu eşit; bildirim 0; kayıt `(adım)` = `[0,3,99,…,20,31]` |
| 1b | Pump'ın duvar saati parçalanması içeriği değiştirmiyor | PASS | `[0,400,800,1200,2000]` ile `[0,133,1999,2000]` akışları eşit |
| 1c | Hız içeriği değiştirmiyor | PASS | 1, 37, 900 adım/sn → 150 adımın bildirimleri, doküman ve günlük özeti aynı (`9dfc59fd…`) |
| 1d | Geri seek = baştan kurma | PASS | 180→37 (adım), 250→tarih: doküman ve günlük eşit; sonraki 60 adımın bildirimleri eşit; tarihe git o günün hiçbir adımını almamış |
| 1e | Otorite + yansı (kendi düzenim: 8 enstrüman, tohum 3, RSI(9), 12 koşul, yansının verisi 0 ve **4 adım geride**) | PASS | 180 adım, her adımda karşılaştırma, fark 0; gösterilen 631 / 591 iddia, ihlal 0; iki koşu eşit |
| 1f | `qp replay` yerel gerçek veriyle | PASS | 10 enstrüman, 2025-04-28…07-31, saatlik yol, seek'li denetimler: iki koşu bayt eşit, `synthetic:false`, bildirim 0 (yalnız toplu sayı) |
| 1g | Denetim kenar durumları | PASS | geçersiz tarih/hız/n `invalid`, iki alansız seek `invalid`, bilinmeyen alan `schema`, `hello` öncesi `handshake`, aralık dışı seek kırpılıyor |
| 2a | K42/A23 karne yönü | PASS | 846 doğumda anahtar `<tür>/<yön>/…`; `ledger_at_birth` aynı anahtarlı, doğumdan **önce** kapananlardan bağımsız yeniden hesapla 846/846 eşit; 806 doğumda yönsüz sayım farklı olurdu |
| 2b | İleriye bakma (kendi bozmam: kesim barından sonraki her barı aynala + zikzak) | PASS | kesime kadarki 1562 olay aynı; 12 anahtarın karnesi aynı; kesim öncesi doğumların karne/önem/kademesi aynı; sonrası farklı |
| 2c | K42/A22 olumsuz geçiş bağımsızlığı (protokol düzeyi, yeniden oynatmada) | PASS | `important`: 252 geçişten 39 teslim, eksik 0, fazla 0; seviye üstü 20 pozisyon doğumu gelmedi, 15 olumsuz geçişin 15'i geldi. `balanced` da aynı |
| 2d | Geçerli olmayan kapsül: öğe, ayrıntı, işaret, TR/EN cümle | PASS (+ bulgu B2, B3) | 40 anlık görüntü: `no_longer` 42, `valid` 132, `closed` 2642; 303 ayrıntıda işaret = öğe; uyuşmazlık 0 |
| 2e | Süre ifadesi, kendi örneklerim (günlük + 60 dk, hafta sonu + tatil) | **FAIL** (B1) | 24 örnekten 13 doğru; 10 yanlışın hepsi takvimde olmayan gerçek tatil (2025'in bütün tatilleri, 2026 Kurban, 15 Temmuz). İşlevin mantığı doğru, `XIST` takvim verisi eksik |
| 2f | Hassasiyet benzetimi | PASS | sentetik ve gerçek ikişer koşu bayt eşit ve depodakiyle aynı; `important` doğumlarında pozisyon/niyet payı 0 (906 / 1934 doğum) |
| 2g | Değişen eski dökümler/altınlar T3 listesiyle örtüşüyor mu | PASS (+ B5) | Kendi yaprak farkım: her değişen yaprak K42 alanı ya da yalnız karne yan cümlesi; istisnalar sıra/önem (38, 41) ve not metinleri (41). `fixtures/golden` yalnız c2, c3 (+ yeni g2) |
| 3a | Piksel ↔ bar | PASS | pinch: x'in altındaki bar 4/4'te aynı ya da ±1; genişlik round(59/2)=30; drag: parmağın altındaki bar 5/5'te aynı (±1), viewport = `pan(round(−dx/slot))` 7/7 |
| 3b | Bakış durumu günlüğe/dokümana girmiyor | PASS | 1500 karışık girdi (drag, pinch, tap_at, open/close, look): günlük 2→2, doküman özeti aynı, `changes` abonesine 0 satır; `screen.view_state` var |
| 3c | `shared` kutuları gerçekten o öğeler mi | PASS | yeni sahnede her kutunun merkezine `tap_at`: kapsül 7/7, grafik 4/4 doğru öğe; kapsül→grafik `before` kutuları = önceki geçişin `after` kutuları; geri kapatınca aynı |
| 3d | Kanıta inme yetkili/yetkisiz | PASS | `arkin`: `values` + `evidence_ref`; `can`: `requires: bist/THYAO` + `rests_on`, değer yok, sunumda `evidence: {}`; ikisi de `capsule` sorgusunun `evidence`'ı ile aynı |
| 4 | Ergonomi (`qp serve`, kendi mesajlarım) | PASS | `capsule` seçici: `latest`/`first`/`stage`/`open`, `ambiguous` (aday listesiyle), `not_found`, `did_you_mean`; `capsules` `latest` + `born_since_bar` (+`open`); bulgu seçicisinde `open`/`stage`; §20.3 kitle biçimi, `set_sensitivity: null`, bilinmeyen alan `schema`, kişi kendi kitlesini `document`'tan okuyor, sahip başkasınınkini görmüyor. Hepsi elle hesapladığım beklenenle aynı |
| 5a | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | PASS | 16/16 PASS, çıkış 0 (browser PASS; conform 55 × 3; digest `7b1e99d40889f83c`) |
| 5b | Eski dökümler 01–37 `main` ile bayt eşit | PASS | 37 dosya `cmp` eşit |
| 5c | `fixtures/golden/` değişenler yalnız T3 listesi | PASS | c2, c3 değişti; g2-light/dark yeni |
| 5d | Hesap özeti | PASS | `fixtures/compute` değişmedi; üç hedef `7b1e99d40889f83c` |
| 5e | `git ls-files \| grep fixtures/local` | PASS | boş |
| 5f | Gizli bilgi taraması; eski round klasörleri | PASS | `main`'e göre değişen dosyalarda anahtar/belirteç kalıbı yok; `rounds/R0–R3`, `RP*`, `NORTH_STAR.md` değişmemiş |

## Bulgular

### B1 — Yüksek: `XIST` takviminde yalnız beş örnek 2026 tatili var; süre ifadesi gerçek veride yanlış

`qp_calendar::builtin("XIST")` tatil listesi: 2026-01-01, 03-20, 04-23, 05-01, 05-19 ve bir yarım
gün (2026-03-19). Kod yorumu: "R1 scope: mechanism, not full data". 2025 ve öncesinin hiçbir
tatili, 2026 Kurban Bayramı (27–29 Mayıs), 15 Temmuz, 29 Ekim yok. `elapsed` işlevinin kuralları
doğru, ama takvim eksik olduğu için gerçek tarihlerde işlem günü sayısı yanlış çıkıyor. T3'ün
VERIFY 12 testleri tam olarak listedeki üç tatili (23 Nisan, 1 Mayıs, 19 Mayıs 2026) seçtiği için
geçiyor.

Elle yazdığım doğru cevaplarla karşılaştırma (`out-elapsed.txt`; 24 örnekten 13 doğru):

| Periyot | Olay → şimdi | Doğru (gerçek BIST takvimi) | İşlevin çıktısı |
|---|---|---|---|
| 1d | 2026-05-26 → 06-01 | salı günü | 4 işlem günü önce |
| 1d | 2026-05-25 → 06-02 | 3 işlem günü önce | geçen hafta |
| 1d | 2026-07-14 → 07-16 | salı günü | 2 işlem günü önce |
| 1d | 2025-04-30 → 05-02 | çarşamba günü | 2 işlem günü önce |
| 1d | 2025-03-28 → 04-02 | cuma günü | 3 işlem günü önce |
| 1d | 2025-06-04 → 06-12 | 4 işlem günü önce | geçen hafta |
| 1d | 2025-10-27 → 11-04 | 5 işlem günü önce | geçen hafta |
| 60m | 2026-05-26 12:00 → 06-01 10:00 | salı 12:00'de | 4 işlem günü önce |
| 60m | 2025-04-30 17:00 → 05-02 10:00 | çarşamba 17:00'de | 2 işlem günü önce |
| 60m | 2025-06-04 16:00 → 06-10 10:00 | 2 işlem günü önce | 4 işlem günü önce |

Takvim içindeki tatillerle ve hafta sonuyla bütün örnekler doğru (Ramazan 2026 + yarım gün,
23 Nisan, 19 Mayıs, cuma→pazartesi, aynı gün saat, ≥ 20 işlem günü tarih, yıl gösterimi).
Bağımsız destek: yerel gerçek veride 2025-04-28…07-31 arasında 64 işlem günü var, gerçek BIST
takvimiyle aynı; yerleşik takvim aynı aralıkta 66 der.

Bugün `elapsed` oturumda kullanılmıyor (yalnız `realize::parts`), ama M8 yüzü bunu gerçek veriyle
gösterecek ve gösterim 2018→bugün aralığını oynatıyor; GOAL K43 "takvime uygun süre" ve VERIFY 12
"takvime göre doğru" istiyor. Öneri: XIST'in 2018–2026 tatil ve yarım günlerini takvim verisine
eklemek (ya da gerçek veride işlem günlerini veriden türetmek) ve VERIFY 12 testine takvim
dışından seçilmiş bir örnek eklemek. Önem Yüksek: kullanıcının göreceği metinde yanlış bilgi
olur, testler bunu yakalamıyor.

Yeniden üretme: `cargo run --release --manifest-path rounds/R4/experiments/verify-e1/probe/Cargo.toml --bin elapsed`.

### B2 — Orta: kapanmış (`closed`) kapsülün cümlesi "Hâlâ geçerli / It still holds" diyebiliyor

Gerçekleşmiş (`realized`) bir kırılımın öğesi `validity: "closed"`, işareti kapanmış (✓) çiziliyor,
ama cümlesi: "… Beklenen sonuç 8 bar önce gerçekleşti. Hâlâ geçerli. …" / "… The expected outcome
came true 8 bars ago. It still holds." Yeniden oynatmada (6 enstrüman, tohum 9, 40 anlık görüntü)
1190 `realized` öğenin 594'ü böyle. Transcript 52/53'teki `cap.THYAO.breakout.down.124` de aynı
(sunumun metni dahil). A28'e göre "durum koşulu hâlâ tutuyor" anlamında savunulabilir; ama GOAL'ün
"geçerlilik ilk bakışta ayrılır; işaret ve cümle aynı şeyi söyler" ölçüsünde kapanmış bir kapsül
için "Hâlâ geçerli" kelimesi `closed` işaretiyle çelişik okunur. T3'ün testi (`capsule_k42.rs`)
ve benim `no_longer` denetimim bu durumu kapsamıyor (yalnız `no_longer` ⇒ "artık", `valid` ⇒ "artık"
yok). M8 başlığı kurulurken kapanmış kapsülde "hâlâ geçerli" yerine koşul dilinin seçilmesi
gerekir. Yeniden üretme: `--bin validity` (sayım satırı `closed(realized)+sentence says still valid`)
ve `--bin interact` (E satırları).

### B3 — Düşük: `validity: "valid"` alanı "izlenen koşulu yok" durumunu da kapsıyor; cümle "bilinmiyor" diyor

Aynı koşuda 132 `valid` öğenin 34'ünün cümlesi "Hâlâ geçerli olup olmadığı bilinmiyor" /
"Whether it still holds is unknown". Protokol alanını okuyan bir ajan "geçerli" öğrenir, cümle
"bilinmiyor" der. T3 bunu "Önerilen karar T3-1" olarak açıkça yazmış (dördüncü bir `unknown`
değeri yok). Karar açık kalmalı; M8 ve ajan yazabilirlik ölçümünde dikkate alınmalı.

### B4 — Düşük: T2.md ve T4.md'de döküm numaraları bütünleştirme sonrası güncellenmemiş

T2.md "Yeni: dökümler 45, 46", tabloda "45: durum…, 46: kaynaksız…" diyor; depoda bunlar
`49-replay-control`, `50-replay-unsupported` (45–48 T3'ün). T4.md "49/49 döküm üç taşımada" diyor;
bugün 55. Belge doğruluğu; PROTOCOL §4.6.1, §20.2 doğru numaraları veriyor.

### B5 — Düşük: T3.md'de döküm 41'in farkı eksik anlatılmış

T3 tablosu 41 için "kademe 3 doğum bildirimi kalktı … liste sırası" diyor. Kendi farkımda adım 12'nin
`capsules` (+`audience`) cevabında `cap.THYAO.extreme.down.126` (kademe 3) **listeden çıkmış**
(4 → 3 öğe), yalnız sıra değişmemiş. Bu, A22 + "sorgu hikâyeyi şimdiki aşamasıyla süzer" (T3-4)
kuralının doğru sonucu; listede neden olarak yazılmalıydı. Döküm 38'deki 2. sıra değişimi ise
önemle tam açıklanıyor (aynı küme, önem 1,656/1,636 → 1,630/1,664).

### B6 — Bilgi

- `capsule_perf_equality.rs` R3 özetleri K42 koduyla yeniden kaydedildi (T3.md'de yazılı); test artık
  R3 önbellek düzeltmesine eşitliği değil, yalnız gerilemeyi yakalıyor.
- `T3/sensitivity-real.json` sentetik kitlenin izleme listesi olarak gerçek sembol adlarını taşıyor
  (fiyat ya da seri yok). R3'teki `sensitivity-real.json` ile aynı uygulama; "toplu istatistik"
  kuralını ihlal etmiyor, not olarak.
- Yeniden oynatmada `seek` sonrasında `reset` alanı yoksa `null`/yok (isteğe bağlı alan); belgeyle
  uyumlu.
- Yetkisiz okuyucunun açılma sunumunda olgu `strength` ve geometri bar aralıkları görünüyor,
  değerler (`evidence`) boş. R3 `present` davranışı; K33 açısından T5/K34 ile gözden geçirilebilir.
- VERIFY 7'nin "sahne özetleri" kısmı bu denetimde ayrıca ölçülmedi (demo dokümanında görünüm
  yok); mesaj akışı, doküman ve günlük bayt eşitliği ölçüldü.

## Notlar (yöntem)

- Elle cevaplar gerçek BIST takviminden yazıldı: 2025 — 1 Ocak, Ramazan B. 30 Mart–1 Nisan,
  23 Nisan, 1 Mayıs, 19 Mayıs, Kurban B. 5 Haziran yarım gün + 6–9 Haziran, 15 Temmuz, 30 Ağustos
  (cumartesi), 28 Ekim yarım gün + 29 Ekim; 2026 — 1 Ocak, 19 Mart yarım gün + 20–22 Mart,
  23 Nisan, 1 Mayıs, 19 Mayıs, 26 Mayıs yarım gün + 27–30 Mayıs, 15 Temmuz, 30 Ağustos (pazar),
  28 Ekim yarım gün + 29 Ekim. Yarım günler işlem günü sayıldı.
- Yansı denetimindeki gösterim kuralı (iddia otoritece ileri sürülmüş, kesin etiketli, bar
  yansının kendi verisinde kapanmış) RP2/R3 ölçütüyle aynı; farklı olan senaryo: yansının verisi
  4 adım gecikmeli.
- Hassasiyet benzetimi çıktıları depodaki dosyaların üzerine yazıldı ve `git status` değişiklik
  göstermedi (koşular depodakiyle bayt eşit).
- Gerçek veriden yalnız toplu sayılar kaydedildi (`replay-local-stats.txt`); hiçbir çıktı dosyası
  sembol, fiyat ya da ekran görüntüsü içermiyor.
