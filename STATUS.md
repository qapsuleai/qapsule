# Durum

Bu dosya, Claude Code oturumları ile ana akış oturumu arasındaki köprüdür.
Her round sonunda güncellenir.

## Şu an

- **Aktif round:** RP2 (koşulmayı bekliyor). R3 ana akışta tasarlanıyor.
- **Son güncelleme:** 2026-11-15
- **Sıradaki adım:** `scripts/preflight.sh`; `rounds/RP2/PROMPT.md` yeni bir Claude Code oturumuna verilir.

## Round geçmişi

| Round | Sonuç | Rapor |
|-------|-------|-------|
| — | Temel dokümanlar yazıldı | — |
| R0 | Tamamlandı: 8/8 kabul ölçütü, VERIFY 13/13. Şema v0, `qp` CLI, deterministik referans çizici (tiny-skia + fontdue), wasm + tarayıcı 0 px fark, A1 → K12. | `rounds/R0/REPORT.md`, `rounds/R0/AUDIT.md` |
| R1 | Tamamlandı: 10/10 kabul ölçütü (ölçüt 5'te uyarı: süre hedefleri M4 Max'te tutuyor, GitHub Linux koşucusunda RSI, MACD ve Bollinger yerelde, MACD WASM'da aşıyor). Depo GitHub'da, CI yeşil. `qp-compute` (Study, 5 indikatör, toplu = akışkan = düzelt bit-eşit, tarama), `qp-source`, `qp-calendar` (K16), `qp compute`. Hesap özeti aarch64 = x86_64 = wasm32. | `rounds/R1/REPORT.md`, `rounds/R1/AUDIT.md`, `rounds/R1/BENCH.md` |
| RD1 | Tamamlandı: kabul ölçütleri 10 ✅ / 3 ⚠️; bağımsız denetim koşullu kabul, dört koşul giderildi. Alan protokolü `qp/0` (`docs/PROTOCOL.md`; 8 mesaj türü), tipler `qp-schema::protocol`, referans otorite `qp-session`, `qp serve`, `qp conform`, MCP uyarlayıcısı `qp-mcp` (araçlar şemadan). 17 uygunluk dökümü inproc/serve/mcp'de geçiyor; yeniden oynatma bayt-eşit. Beş çelişkili karar ölçüldü: K21-1 koru, K21-2 koru (inceltmelerle), K21-3 incelt, JSON kanonik, anlamsal değişim. Ajan yazabilirlik: v1 15/20 → v2 14/20 ilk deneme (sonunda 17 → 18); kimlikler kopyalanamayınca 6/20 ilk, 19/20 sonunda, düzeltme 13/14. Alt ajanlar bu round'da çalıştı. | `rounds/RD1/REPORT.md`, `rounds/RD1/RESEARCH.md`, `rounds/RD1/AUDIT.md` |
| RP1 | Tamamlandı: 7/7 kabul ölçütü, bağımsız denetim "kabul". 8 yönde 21 yaklaşım (9 alındı). SoA şerit çekirdekleri, ızgarada ortak alt hesap paylaşımı, zincirler için hesap grafiği, seri şeritleri, commit-only attach, paralel indirgeme taraması, kanıtlı geç başlangıç (yeni yöntem). K17 hedefleri beş indikatörde GitHub koşucusunda da tutuyor (en kötü RSI 42,2 ms yerel, 57,3 ms WASM). Bit-eşitlik ve hesap özeti değişmedi. | `rounds/RP1/REPORT.md`, `rounds/RP1/RESEARCH.md`, `rounds/RP1/AUDIT.md` |
| R2 | Tamamlandı: kabul ölçütleri 12 ✅ / 2 ⚠️ (6: kökensiz sabit için bazı panellerde aday yok; 7: ekranlı bakış gelecek yuvaları göstermiyor); bağımsız denetim koşullu kabul, dört koşul giderildi. Gramer v0 (`docs/GRAMMAR.md`; `qp-schema::grammar`, `::scene`), `qp-grammar` derleyicisi, çizici yalnız sahne tarifinden (hesapsız), altı görünüm + iki derinlik görünümü yalnız protokol dökümlerinden (V0 R0 altınla bayt-eşit), köken zorunlu (`no_origin`), kimlikli gramer işlemleri (18 eşzamanlı durumda yanlış 0), `screen`/`hit` sahne tarifinden, anlamla başvuru: seçici nesneler (T9: ilk denemede kimliksiz 19/20; B 10/20; yalnız kimlik 2/20; A v2 sonunda 20/20), 100k bar kare 2,7 ms (LOD), 27 döküm × 3 taşıma, WASM +%25 (595 KB gzip). | `rounds/R2/REPORT.md`, `rounds/R2/AUDIT.md`, `rounds/R2/ergonomics/RESULTS.md` |

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
- `verify.sh` 14 adım (RD1'de `protocol` ve `conform`, R2'de `views` eklendi; `conform` dökümleri inproc, `qp serve` ve MCP üzerinden koşar; 27 döküm).
- Referans otorite (`qp-session`) her komutta dokümanı kopyalıyor; canlı yolda yavaş (T6d: 84 µs/tik, artımlı yol 0,18 µs). Öneri olarak duruyor.
- R1'de alt görevler hook kısıtı nedeniyle alt ajanlara verilemedi; denetim yine bağımsız alt ajanla yapıldı. RP1'de de aynı: yalnızca E1 deneyi ve denetim alt ajanla (hook'un ürettiği girdiyle tek başına yeniden gönderilince) başlatılabildi.

## Arkin'den beklenenler

- R2 önerilen karar satırları (`DECISIONS.md` Ö10–Ö17) ve açık kararlar A13 (WASM boyutu), A14; R2 açık soruları (`rounds/R2/REPORT.md`).

- K14 (bar hizası borsaya göre, gösterim kullanıcıya göre) onayı; R1'de bu şekilde uygulandı (günlük bar `t` = seans açılışının UTC zamanı).
- Süre hedeflerinin hangi donanıma göre olduğu (R1 raporu, açık soru 4).
- R6 öncesi: Matriks günlük barlarının taşıdığı zaman (gece yarısı mı, seans açılışı mı).
- RP1 açık soruları (`rounds/RP1/REPORT.md`): `unsafe_code = "forbid"` istisnası (x86 AVX2), tamsayı kuruş toplamı (SMA tanımı), sorgu biçimli hesap API'si ve K13 ilişkisi, önbellek bellek politikası; önerilen `DECISIONS.md` satırları.
- RD1 önerilen karar satırları (`DECISIONS.md` Ö1–Ö9) ve açık kararlar A10–A12; RD1 açık soruları (`rounds/RD1/REPORT.md`): bar verisi biçimi, anlamsal referans, devir protokolü, MCP olay kanalı, öğe başına `base`.
- Agent alt görevleri için: context-mode hook'u Agent çağrılarını değiştirdiği için auto mode onları denetleyemiyor; hook'un Agent'ı muaf tutması ya da R2'de auto mode dışında onay.
- A2: Qapsule adı için marka ön taraması ve alan adı kararı.
- A4 (hukuk) ve A5 (ajan ortamı) kararları.
- R6 öncesi: NATS akışlarından birkaç örnek mesaj (konu adı ve içerik biçimi).
- Eski `ts/.env` içindeki token'ın iptali ve yenilenmesi (eski `.gitignore` onu dışlamıyor).
- R6 öncesi: Matriks protobuf (`.proto`) tanımlarının kaynağı.
