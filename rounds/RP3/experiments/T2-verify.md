# RP3/T2 — bağımsız denetim

Dal `rp3/t2-harness` (1a6137f), taban 6877bda. Denetçi işi yazan ajan değil. Dala dokunulmadı.
Kullanılan port: 8500–8501.

**Karar: PASS**

| # | Denetim | Sonuç |
|---|---------|-------|
| 1 | Kapsam (`git diff 6877bda..rp3/t2-harness --stat`) | 18 dosya: yalnız `shells/web/**`, `crates/qp-wasm/**`, `rounds/RP3/experiments/T2.md`. Altın görsel, döküm ve hesap crate'lerinde değişiklik yok. |
| 1 | arch-check kuralları | `FORBIDDEN` regex'leri, R1 ve R4 denetimleri aynı kaldı. Değişenler: (a) `ALLOWED_CORE` listesine 9 yöntem eklendi, her biri ARCH.md'de gerekçeli. (b) R6: `main.js` için yalnız `./paint/index.js` ekleniyor, `paint/*.js` için yalnız kardeş `./x.js` kabul ediliyor. (c) Kural sıkılaştı: her yerde dinamik `import()` yasak. (d) `paint/*.js` dosyaları aynı `checkJs` ile taranıyor, 8 yeni bozuk örnek eklendi. Gevşetilen kural yok. `node shells/web/arch-check.mjs`: 0 ihlal (main.js ve 3 boyama dosyası), öz-sınama 22/22. |
| 2 | fmt / clippy / test | `cargo fmt --check` temiz. `clippy -p qp-wasm --all-targets -D warnings` hem özelliksiz hem `--features replica` ile temiz. `cargo test -p qp-wasm --features replica`: hepsi geçti (1+3+2+9+4+3+1). |
| 3 | `paint/ref.js` ile R4 boyaması | `put` fonksiyonu 6877bda `main.js`'tekiyle bayt bayt aynı: boşsa false döner, yoksa `width`/`height` atanır ve `putImageData` çağrılır. `paintFace` → `core.face_rgba(id, force)` ve `faceSize.px`. `paintScene` → `core.scene_rgba(force)` ve `open.px`. `metric_span`/`first_capsule` çağrıları `main.js`'te aynı yerde duruyor. Varsayılan yolda davranış aynı. |
| 4 | `compare.mjs` (Chromium ve WebKit, varsayılan 8000 ms evre) | Port 8500 için betiğin geçici kopyası kullanıldı, mantık değişmedi. Chromium 68,1 s (120 Hz), WebKit 68,0 s (60 Hz); iki tarayıcı toplam 140 s, çıkış kodu 0. İkisi de < 180 s. Her tarayıcıda tek özet var: `paths` = `ref` ve `fixture`, her yolda 4 evre (idle, scroll, chart_ticks, drag), `fallback` boş. Elle hareket gerekmedi. Yedeğe düşme (`paint=nope` için `unknown`, `paint=fixture` için `unsupported`) doğrulandı. |
| 5 | Ölçüt 9a (`metrics_t2.rs`) | `ios_60hz` serisi 16/17/17 ms. Sonuç: `refresh.hz` 60, `long` 0, `very_long` 0. Gerçek düşmeler sayılıyor: 200 karede 24 uzun (0,12), 4 çok uzun (0,02). 120 Hz algılanıyor; az karede 60 Hz varsayılıp bu söyleniyor. Eşik `LONG_FACTOR` = 1,5 × yenileme aralığı (`shell.rs`). DPR 0,8 için uyarı üç kodla geliyor: `dpr_below_1`, `dpr_fraction`, `scale_differs`. |
| 6 | Açılış hatasının kök nedeni | 6877bda `main.js` `start()` her başlangıç çerçevesinde `core = new Shell(...)` kuruyor ve hemen ardından `metric_mark('start_frame', performance.now())` çağırıyor. Sonraki bir başlangıç çerçevesi işaretleri dakikalar sonra yeniden alıyor. Kök neden iddiası doğru. Düzeltme: `next.metric_inherit(core)` ve ilk işaretin korunması. Test `the_opening_timeline_survives_a_later_start_frame`. |

## Notlar (engel değil)

- Yol başına yaklaşık 34 s sürüyor (4 × 8 s evre ve geçişler). 3 gerçek yol yaklaşık 102 s, 4 yol yaklaşık 136 s tutar. 5 ve daha fazla yolda 3 dakika sınırı aşılır; evre süresi ya da yol sayısı buna göre ayarlanmalı.
- `compare.mjs` portu sabit kodlanmış (`8420 + k`). Paralel denetimlerin çakışmaması için bir `--port` seçeneği yararlı olur.
- `metrics_d.rs` içinde alan adı `long_16_7` → `long` olarak değişti ve beklenen değer yeni kurala göre gerekçelendirildi. Bu değişiklik kapsam içinde.
