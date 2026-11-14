# Durum

Bu dosya, Claude Code oturumları ile ana akış oturumu arasındaki köprüdür.
Her round sonunda güncellenir.

## Şu an

- **Aktif round:** yok. R1 tamamlandı; R2 tasarımı bekliyor.
- **Son güncelleme:** 2026-11-14
- **Sıradaki adım:** Ana akışta `rounds/R1/REPORT.md` ve `AUDIT.md` okunur, R2 dosyaları yazılır.

## Round geçmişi

| Round | Sonuç | Rapor |
|-------|-------|-------|
| — | Temel dokümanlar yazıldı | — |
| R0 | Tamamlandı: 8/8 kabul ölçütü, VERIFY 13/13. Şema v0, `qp` CLI, deterministik referans çizici (tiny-skia + fontdue), wasm + tarayıcı 0 px fark, A1 → K12. | `rounds/R0/REPORT.md`, `rounds/R0/AUDIT.md` |
| R1 | Tamamlandı: 10/10 kabul ölçütü (ölçüt 5'te uyarı: süre hedefleri M4 Max'te tutuyor, GitHub Linux koşucusunda RSI, MACD ve Bollinger yerelde, MACD WASM'da aşıyor). Depo GitHub'da, CI yeşil. `qp-compute` (Study, 5 indikatör, toplu = akışkan = düzelt bit-eşit, tarama), `qp-source`, `qp-calendar` (K16), `qp compute`. Hesap özeti aarch64 = x86_64 = wasm32. | `rounds/R1/REPORT.md`, `rounds/R1/AUDIT.md`, `rounds/R1/BENCH.md` |

## Bilinen durumlar

- Eski prototip `../Services/quantex-charts-core` ve `../Docs/q-charts` altında, yalnızca
  referans olarak duruyor. Testleri bu çalışma sırasında çalıştırılıp doğrulanmadı.
- Eski repoda `ts/.env` içinde bir Matriks token'ı var ve eski `.gitignore` bunu
  dışlamıyor. Yeni repoya hiçbir gizli bilgi taşınmayacak.
- Eski klasörde git geçmişi yok.

- Depo: https://github.com/qapsuleai/qapsule (`origin` = `git@github-qapsule:qapsuleai/qapsule.git`). CI her gönderimde `verify.sh` çalıştırıyor (Linux x86_64, tarayıcı zorunlu); altın görseller ve hesap özeti gerçek x86_64'te eşit.
- `scripts/verify.sh` tek komutluk doğrulama girişi; 11 adım (R1'de `compute` ve `digest` eklendi). Tarayıcı ve wasm özet adımı `QP_VERIFY_BROWSER=1` ile zorunlu, `=0` ile kapalı.
- `scripts/qapsule-gh.env` `.gitignore`'daki `*.env` kuralı yüzünden izlenmiyor; temiz klonda `qgh`/`preflight` çalışmaz (gizli bilgi içermiyor, istisna önerisi R1 raporunda).
- Tarama süre hedefleri GitHub koşucusunda tutmuyor (RSI/MACD/Bollinger yerel, MACD WASM); bit-eşit ortak alt hesap paylaşımı önerisi `rounds/R1/BENCH.md`'de.
- R1'de alt görevler hook kısıtı nedeniyle alt ajanlara verilemedi; denetim yine bağımsız alt ajanla yapıldı.

## Arkin'den beklenenler

- K14 (bar hizası borsaya göre, gösterim kullanıcıya göre) onayı; R1'de bu şekilde uygulandı (günlük bar `t` = seans açılışının UTC zamanı).
- Süre hedeflerinin hangi donanıma göre olduğu (R1 raporu, açık soru 4).
- `scripts/qapsule-gh.env` için `.gitignore` istisnası kararı.
- R6 öncesi: Matriks günlük barlarının taşıdığı zaman (gece yarısı mı, seans açılışı mı).
- Agent alt görevleri için: context-mode hook'u Agent çağrılarını değiştirdiği için auto mode onları denetleyemiyor; hook'un Agent'ı muaf tutması ya da R2'de auto mode dışında onay.
- A2: Qapsule adı için marka ön taraması ve alan adı kararı.
- A4 (hukuk) ve A5 (ajan ortamı) kararları.
- R6 öncesi: NATS akışlarından birkaç örnek mesaj (konu adı ve içerik biçimi).
- Eski `ts/.env` içindeki token'ın iptali ve yenilenmesi (eski `.gitignore` onu dışlamıyor).
- R6 öncesi: Matriks protobuf (`.proto`) tanımlarının kaynağı.
