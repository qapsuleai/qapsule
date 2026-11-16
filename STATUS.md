# Durum

Bu dosya, Claude Code oturumları ile ana akış oturumu arasındaki köprüdür.
Her round sonunda güncellenir.

## Şu an

- **Aktif round:** yok. R4 tamamlandı (koşullu kabul; koşullar karşılandı, `rounds/R4/REPORT.md` "Denetim").
- **Son güncelleme:** 2026-11-16
- **Sıradaki adım:** Arkin gösterimi açar ve telefonda ölçer (`rounds/R4/REPORT.md` son bölüm, `rounds/R4/DEVICE.md` §4); insan testi v2 (`rounds/R4/human-test/`); R4 önerilen kararları (`DECISIONS.md` "Önerilen (R4)") ana akışta kesinleşir; R5 tasarımı ana akış oturumunda.

## Round geçmişi

| Round | Sonuç | Rapor |
|-------|-------|-------|
| — | Temel dokümanlar yazıldı | — |
| R0 | Tamamlandı: 8/8 kabul ölçütü, VERIFY 13/13. Şema v0, `qp` CLI, deterministik referans çizici (tiny-skia + fontdue), wasm + tarayıcı 0 px fark, A1 → K12. | `rounds/R0/REPORT.md`, `rounds/R0/AUDIT.md` |
| R1 | Tamamlandı: 10/10 kabul ölçütü (ölçüt 5'te uyarı: süre hedefleri M4 Max'te tutuyor, GitHub Linux koşucusunda RSI, MACD ve Bollinger yerelde, MACD WASM'da aşıyor). Depo GitHub'da, CI yeşil. `qp-compute` (Study, 5 indikatör, toplu = akışkan = düzelt bit-eşit, tarama), `qp-source`, `qp-calendar` (K16), `qp compute`. Hesap özeti aarch64 = x86_64 = wasm32. | `rounds/R1/REPORT.md`, `rounds/R1/AUDIT.md`, `rounds/R1/BENCH.md` |
| RD1 | Tamamlandı: kabul ölçütleri 10 ✅ / 3 ⚠️; bağımsız denetim koşullu kabul, dört koşul giderildi. Alan protokolü `qp/0` (`docs/PROTOCOL.md`; 8 mesaj türü), tipler `qp-schema::protocol`, referans otorite `qp-session`, `qp serve`, `qp conform`, MCP uyarlayıcısı `qp-mcp` (araçlar şemadan). 17 uygunluk dökümü inproc/serve/mcp'de geçiyor; yeniden oynatma bayt-eşit. Beş çelişkili karar ölçüldü: K21-1 koru, K21-2 koru (inceltmelerle), K21-3 incelt, JSON kanonik, anlamsal değişim. Ajan yazabilirlik: v1 15/20 → v2 14/20 ilk deneme (sonunda 17 → 18); kimlikler kopyalanamayınca 6/20 ilk, 19/20 sonunda, düzeltme 13/14. Alt ajanlar bu round'da çalıştı. | `rounds/RD1/REPORT.md`, `rounds/RD1/RESEARCH.md`, `rounds/RD1/AUDIT.md` |
| RP1 | Tamamlandı: 7/7 kabul ölçütü, bağımsız denetim "kabul". 8 yönde 21 yaklaşım (9 alındı). SoA şerit çekirdekleri, ızgarada ortak alt hesap paylaşımı, zincirler için hesap grafiği, seri şeritleri, commit-only attach, paralel indirgeme taraması, kanıtlı geç başlangıç (yeni yöntem). K17 hedefleri beş indikatörde GitHub koşucusunda da tutuyor (en kötü RSI 42,2 ms yerel, 57,3 ms WASM). Bit-eşitlik ve hesap özeti değişmedi. | `rounds/RP1/REPORT.md`, `rounds/RP1/RESEARCH.md`, `rounds/RP1/AUDIT.md` |
| R2 | Tamamlandı: kabul ölçütleri 12 ✅ / 2 ⚠️ (6: kökensiz sabit için bazı panellerde aday yok; 7: ekranlı bakış gelecek yuvaları göstermiyor); bağımsız denetim koşullu kabul, dört koşul giderildi. Gramer v0 (`docs/GRAMMAR.md`; `qp-schema::grammar`, `::scene`), `qp-grammar` derleyicisi, çizici yalnız sahne tarifinden (hesapsız), altı görünüm + iki derinlik görünümü yalnız protokol dökümlerinden (V0 R0 altınla bayt-eşit), köken zorunlu (`no_origin`), kimlikli gramer işlemleri (18 eşzamanlı durumda yanlış 0), `screen`/`hit` sahne tarifinden, anlamla başvuru: seçici nesneler (T9: ilk denemede kimliksiz 19/20; B 10/20; yalnız kimlik 2/20; A v2 sonunda 20/20), 100k bar kare 2,7 ms (LOD), 27 döküm × 3 taşıma, WASM +%25 (595 KB gzip). | `rounds/R2/REPORT.md`, `rounds/R2/AUDIT.md`, `rounds/R2/ergonomics/RESULTS.md` |
| RP2 | Tamamlandı: kabul ölçütleri 11 ✅ / 1 ⚠️ (7: seçilen tasarımın kendini onarma yolu `qp/0`'da yok, A20); bağımsız son denetim koşullu kabul, iki koşul giderildi. Veri bağı (`source.kind = "bind"`, kalıcı çapa, veri yetkisi; PROTOCOL §18): 10.000 barlı bağlı dokümanın anlık görüntüsü 2.744 B / gzip 985 B (RD1 750 KB / 211 KB), cihazda hazır olma 0,50 ms. Alan düzeyinde veri olayları (`qp-source`: `FeedStore`, `SourceProvider`; Matriks eşlemesi uygulanmadan). Artımlı canlı yol: tikte doküman kopyası yok, artımlı = baştan bayt düzeyinde (özellik testi), tik 85 µs → 2,0 µs (belge içi, N=5) / 0,76 µs (bağlı); hedef ≤ 3× çekirdek 54/54 durumda (en kötü 2,66; yalın paydayla N=500 `all` 5,06). `view` konusu: etkilenmeyene mesaj yok. Dondurulmuş kanıt (`evidence_ref`, `verify_evidence`). Tutarlılık: dört yaklaşım ayrı dallarda, bağımsız düzenekle; hepsi yanlış/erken 0; B seçildi (§19: iddia kanıtı, `claims` konusu, yansı kendi verisinde yeniden sınar); yansı yerel ↔ WASM özetleri her adımda aynı. K25 (belirsiz sunum, gramer katman adları, geleceğe bakış) ve R2 kalanları. 36 döküm × 3 taşıma; hesap özeti ve altın görseller değişmedi. | `rounds/RP2/REPORT.md`, `rounds/RP2/RESEARCH.md`, `rounds/RP2/AUDIT.md` |
| R3 | Tamamlandı: kabul ölçütleri 18 ✅ (11: AUD2 önce 49 → 53 kötüleşmişti; yansı boşluk onarımı ve otorite yeniden çözüm taramasıyla 0'a indi, denetçi doğruladı); bağımsız son denetim koşullu kabul → üç yeniden denetimden sonra **kabul** (K1: lisanslı akış gücü açık bulgunun `confidence`'ı ile yetkisize gidiyordu — girişimsizlik denemesiyle bulundu; yapı gereği giderildi: yalnız açık olgularla ikinci motor ve ayrı sıra numaralı "açık dünya"; yeniden denetimlerde bulunan iki kanal ve güvenlik incelemesinin bulguları da giderildi; K2 kapanış belgeleri). Olgu dedektörleri (`qp-facts`, 7 dedektör, altı tür; toplu = akışkan), kapsül motoru (`qp-capsule`: hikâye, aday, karne ileriye bakmadan, cümle TR/EN, hassasiyet, mühür), otoritede motor ve PROTOCOL §19.9 (itiraz, kesin bar hijyeni), §20 (kapsül); gramer `glyph` işareti ve kapsül görünümleri (yeni altınlar g1, c2–c5). M1–M7: düzenek önce, 16 alternatif ayrı ajanlarla; seçimler siluet (insan testine kadar), şablon dilbilgisi, teyit birleşimi, tür+yön, oynaklık bariyeri, mühür, itiraz+hijyen. Gerçek veri (5 sembol, günlük 2018– ve 60 dk son iki yıl) yalnız yerelde. Ajan yazabilirlik C01–C20: son 19–20/20, ilk deneme 11–12/20. Performans: tik 2,27 µs/olay (RP2 bağlı N=5'in 2,9×'i), kapanış 48 ms, bağlanma 133 ms (1.000 bar, 30 enstrüman). 44 döküm × 3 taşıma; eski altınlar ve hesap özeti değişmedi. `main`'e birleşti (`86dd8f3`, CI 43788530692 yeşil); ölçüt 11 düzeltmesiyle son hali `6834fd0`, CI 43794279925 yeşil (Linux x86_64). | `rounds/R3/REPORT.md`, `rounds/R3/AUDIT.md`, `rounds/R3/experiments/`, `rounds/R3/human-test/`, `rounds/R3/ergonomics/RESULTS.md` |
| R4 | Tamamlandı: kabul ölçütleri 14 ✅ / 1 ⚠️ (6: yüz M8 ile seçildi, ama kapsül derinliğindeki ayrıntı cümlesi R3 cümlesi kaldı — bar sayısı, iç terim; değiştirmek var olan dökümleri değiştirirdi, R5'e, A33); bağımsız son denetim **koşullu kabul**, engelleyici yok, dört koşul karşılandı. Yeniden oynatma kaynağı (`qp replay`, PROTOCOL §21; iki koşu bayt eşit), tek komutluk gösterim (`scripts/demo.sh`, `qp demo`: SSE + POST, yalnız yerel köken, telefon için `--lan`), ince web kabuğu (beyin Rust/WASM `ShellCore`; mimari testi JS'te sıfır aritmetik; kabuksuz sürüş bayt eşit), etkileşim çekirdekte (`tap_at`/`drag`/`pinch`, bakış durumu günlüğe girmez), K42 (karne yönü, olumsuz geçiş, geçerlilik; `important` doğumlarında pozisyon payı %57–71 → %0), XIST takvimi 2018–2027. M8–M11 (düzenek önce, 9 alternatif, körlemeli persona + eleştirmen ajanları; ön eleme): yüz `vadrel` (persona 0,768; R3 kartı 0,141), akış `elikov`, geçiş `zumlak` (zayıf), sonuç işareti `tg5`; ilk ikiler gösterimde ayardan. Bağlanma 140 → 86 ms; ilk kapsül 121 ms masaüstü / 460 ms 4× kısıt; WASM gzip 1,29 MB. Cihaz ölçümü masaüstü + 4×/6×, K17/K45 önerisi (telefonda doğrulanacak). İnsan testi v2 paketi hazır. Ajan yazabilirlik ilk deneme 12 → 14–17/20. 58 döküm × 3 taşıma; var olan altınlar ve hesap özeti değişmedi (K42 gereği değişenler listeli). | `rounds/R4/REPORT.md`, `rounds/R4/AUDIT.md`, `rounds/R4/DEVICE.md`, `docs/FACE.md`, `rounds/R4/experiments/`, `rounds/R4/human-test/` |

## Bilinen durumlar

- Eski prototip `../Services/quantex-charts-core` ve `../Docs/q-charts` altında, yalnızca
  referans olarak duruyor. Testleri bu çalışma sırasında çalıştırılıp doğrulanmadı.
- Eski repoda `ts/.env` içinde bir Matriks token'ı var ve eski `.gitignore` bunu
  dışlamıyor. Yeni repoya hiçbir gizli bilgi taşınmayacak.
- Eski klasörde git geçmişi yok.

- Depo: https://github.com/qapsuleai/qapsule (`origin` = `git@github-qapsule:qapsuleai/qapsule.git`). CI her gönderimde `verify.sh` çalıştırıyor (Linux x86_64, tarayıcı zorunlu); altın görseller ve hesap özeti gerçek x86_64'te eşit.
- `scripts/verify.sh` tek komutluk doğrulama girişi; 11 adım (R1'de `compute` ve `digest` eklendi). Tarayıcı ve wasm özet adımı `QP_VERIFY_BROWSER=1` ile zorunlu, `=0` ile kapalı.
- `scripts/qapsule-gh.env` `.gitignore`'daki `*.env` kuralı yüzünden izlenmiyor; temiz klonda `qgh`/`preflight` çalışmaz (gizli bilgi içermiyor, istisna önerisi R1 raporunda).
- Tarama süre hedefleri (K17) RP1 sonrası GitHub koşucusunda da tutuyor; koşucu CPU'su çalışmadan çalışmaya değişiyor (AMD EPYC 9V74 / 7763, Intel Xeon 8573C), ölçümler CPU modeline göre okunmalı. `sweep_bench` CI'da profil ve şerit tablosunu basıyor.
- RD1'de alt ajanlar sorunsuz başlatıldı (context-mode engeli bu oturumda yoktu); bütün alt görevler, deneyler, ölçüm ve denetim alt ajanlarla yapıldı.
- `verify.sh` 15 adım (RD1'de `protocol` ve `conform`, R2'de `views`, RP2'de `replica` eklendi; `conform` dökümleri inproc, `qp serve` ve MCP üzerinden koşar; R3 sonunda 44 döküm, 12 görünüm; `digest` adımı R3 hesaplarının ayrı özetini de denetler).
- R3'ten beri gerçek geçmiş barlar `scripts/fetch-bars.sh` ile `fixtures/local/` altına çekilebilir (`docs/DATA_ACCESS.md`); depoya girmez.
- RP2'den beri canlı yol artımlı: bar yazısı, veri olayı ve motor girdisi dokümanı kopyalamaz (tik 0,76–20 µs, 10k bar, 5–500 hesap). Diğer komutlar hâlâ dokümanı kopyalar.
- R1'de alt görevler hook kısıtı nedeniyle alt ajanlara verilemedi; denetim yine bağımsız alt ajanla yapıldı. RP1'de de aynı: yalnızca E1 deneyi ve denetim alt ajanla (hook'un ürettiği girdiyle tek başına yeniden gönderilince) başlatılabildi.
- R4'ten beri `verify.sh` 18 adım (`replay`, `shell`, `demo` eklendi; `demo` sahne görüntülerini bayt eşit, sayfa görüntülerini `shells/web/tests/pagecmp.mjs` toleransıyla — ≤ 64 px, ≤ 2 seviye — karşılaştırır); 58 döküm × 3 taşıma. Gösterim: `scripts/demo.sh` (`shells/web/README.md`).

## Arkin'den beklenenler

- **R4:** gösterimi açıp telefonda ölçmek (`rounds/R4/REPORT.md` son bölüm); insan testi v2 (5–10 kişi, ~20 dk; `rounds/R4/human-test/GUIDE.md`) — A21 ve yüz seçimini kapatır, M9–M11'i onaylar ya da değiştirir; önerilen kararlar Ö39–Ö51, açık A31–A33; R4 açık soruları (`rounds/R4/REPORT.md`).

- R3 önerilen karar satırları (`DECISIONS.md` Ö29–Ö41) ve açık kararlar A21–A30; R3 açık soruları
  (`rounds/R3/REPORT.md`): mühürün "yetkisize lisanslı hikâye yalnız mühür iletileriyle" kuralı ile
  K34 gerilimi; pozisyon/niyet geçişinin `important`'ı doldurması; açık hikâyede "artık geçerli
  değil" cümlesi; kapanmış hikâyelerin arşivlenmesi; insan testi cevaplarının depoya girmesi.
- İnsan testi (5–10 kişi, ~16 dk): `rounds/R3/human-test/GUIDE.md`; sonuç alfabe seçimini (A21) kapatır.
- `fixtures/local/symbols.txt`'teki `MIGRS` büyük olasılıkla `MGROS` (her istekte 503).

- RP2 önerilen karar satırları (`DECISIONS.md` Ö18–Ö28) ve açık kararlar A15 (kanıt değerleri ve veri lisansı), A16 (canlı barda geçici iddia), A17 (kesin bar kaynağı), A18 (paylaşılan depo, geçmiş düzeltmede kısmi yeniden hesap), A19 (kriptografik özet), A20 (otoritenin yanlış kesin iddiasının onarımı); RP2 açık soruları (`rounds/RP2/REPORT.md`).

- R2 önerilen karar satırları (`DECISIONS.md` Ö10–Ö17) ve açık kararlar A13 (WASM boyutu), A14; R2 açık soruları (`rounds/R2/REPORT.md`).

- K14 (bar hizası borsaya göre, gösterim kullanıcıya göre) onayı; R1'de bu şekilde uygulandı (günlük bar `t` = seans açılışının UTC zamanı).
- Süre hedeflerinin hangi donanıma göre olduğu (R1 raporu, açık soru 4).
- R6 öncesi: Matriks günlük barlarının taşıdığı zaman (gece yarısı mı, seans açılışı mı).
- RP1 açık soruları (`rounds/RP1/REPORT.md`): `unsafe_code = "forbid"` istisnası (x86 AVX2), tamsayı kuruş toplamı (SMA tanımı), sorgu biçimli hesap API'si ve K13 ilişkisi, önbellek bellek politikası; önerilen `DECISIONS.md` satırları.
- RD1 önerilen karar satırları (`DECISIONS.md` Ö1–Ö9) ve açık kararlar A10–A12; RD1 açık soruları (`rounds/RD1/REPORT.md`): bar verisi biçimi, anlamsal referans, devir protokolü, MCP olay kanalı, öğe başına `base`.
- Agent alt görevleri için: context-mode hook'u Agent çağrılarını değiştirdiği için auto mode onları denetleyemiyor; hook'un Agent'ı muaf tutması ya da R2'de auto mode dışında onay.
- A2: Qapsule adı için marka ön taraması ve alan adı kararı.
- A5 (ajan ortamı) kararı.
- R6 öncesi: NATS akışlarından birkaç örnek mesaj (konu adı ve içerik biçimi).
- Eski `ts/.env` içindeki token'ın iptali ve yenilenmesi (eski `.gitignore` onu dışlamıyor).
- R6 öncesi: Matriks protobuf (`.proto`) tanımlarının kaynağı.
