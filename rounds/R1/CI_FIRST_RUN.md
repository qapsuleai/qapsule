# R1 / T0 — CI'ın ilk gerçek çalışması

- **Tarih:** 2026-11-14
- **Ön kontrol:** `scripts/preflight.sh` → `preflight: TAMAM` (origin, SSH takma adı
  `github-qapsule` → `qapsuleai`, `qgh` hesabı `qapsuleai`, depoya yazma izni, commit e-postası).
- **Gönderim:** `git push -u origin main` (uzak depo boştu; yeni dal `main`). Başka dal, depo
  ya da hesap işlemi yapılmadı.
- **Çalışma:** https://github.com/qapsuleai/qapsule/actions/runs/43611236862
- **Commit:** `35befe0` (R1 dokümanları + `qgh`/`preflight`; kod R0 sonundaki ile aynı)
- **Koşucu:** GitHub barındırılan `ubuntu-latest` → imaj `ubuntu-26.04` (sürüm 20261103.629),
  x86_64. Chromium 151.0.7922.34 (Playwright).
- **Süre:** 15:38:11Z → 15:43:23Z (≈5 dk 12 sn; `verify.sh` 175 sn, soğuk önbellek).
- **Sonuç:** `success`. İzleme `scripts/qgh run watch 43611236862 --exit-status` ile yapıldı.

## `scripts/verify.sh` özeti (CI günlüğünden)

| # | Adım | Sonuç | Süre | Not |
|---|------|-------|------|-----|
| 1 | fmt | PASS | 0 s | |
| 2 | clippy | PASS | 24 s | |
| 3 | test | PASS | 38 s | `qp-render` altın görsel testleri dahil |
| 4 | schema | PASS | 85 s | 9 şema, deterministik |
| 5 | fixtures | PASS | 0 s | 2 örnek geçerli |
| 6 | golden | PASS | 0 s | `qp render` çıktısı `fixtures/golden/sample.png` ile **bayt düzeyinde aynı** |
| 7 | wasm | PASS | 13 s | ham 1.017.631 B, gzip -9 367.758 B |
| 8 | hygiene | PASS | 0 s | |
| 9 | browser | PASS | 15 s | Headless Chromium: canvas ↔ altın **0 px**, stres wasm ↔ altın **0 px** |

## Değerlendirme

- **Altın görseller gerçek x86_64/Linux üzerinde bayt-eşit.** K12'deki "gerçek x86/Linux
  donanımında ilk ölçüm" açık riski kapandı; fark çıkmadığı için kök neden araştırması
  ve K12 güncellemesi gerekmedi.
- **Tarayıcı adımı CI'da zorunlu ve geçti** (`QP_VERIFY_BROWSER=1`): dört karşılaştırmanın
  hepsi 0 farklı piksel.
- **Küçük gözlem:** Linux'ta ham wasm 1.017.631 B, macOS'ta 1.016.781 B (+850 B). gzip
  boyutları da 405 B farklı. Bu fark derlenmiş koda değil, büyük olasılıkla wasm içine gömülen
  panik konumu dizgelerine (kayıt defteri ve kaynak yolları, `$HOME` farkı) aittir; işlevsel
  bir etkisi yok. Boyut izlemesinde ±1 KB'lık bu oynaklık göz önünde tutulmalı.
- Bir uyarı: Node `punycode` kullanımdan kaldırma uyarısı (Playwright bağımlılığından);
  işlevsel değil.

Sonraki her `main` gönderiminde CI yeniden çalışır; R1 kapanışındaki son çalışma
`rounds/R1/REPORT.md` içinde.
