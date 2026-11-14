# R0 — Rapor

- **Durum:** Tamamlandı. 8 kabul ölçütünün 8'i karşılandı, `VERIFY.md` içindeki 13 adımın 13'ü geçti.
- **Tarih:** 2026-11-14
- **Denetlenen commit:** `2c3d049` (`main`)
- **Denetim kaydı:** `rounds/R0/AUDIT.md`. Denetimi, işi yapan ajanlardan bağımsız bir denetçi alt ajan temiz klonda yaptı.

## Ne yapıldı

| # | Alt görev | Kim | Çıktı |
|---|-----------|-----|-------|
| T1 | Depo ve iskelet | Orkestratör | Hazırlananlar: `git init`, workspace `Cargo.toml` ve 4 crate (`qp-schema`, `qp-render`, `qp-cli`, `qp-wasm`). Ayrıca `rust-toolchain.toml` (Rust 1.92.0, rustfmt, clippy, wasm32 hedefi), `rustfmt.toml`, `clippy.toml`, `.gitignore` (`.env*`, anahtarlar, `*token*`, `.claude/`) ve CI iskeleti. |
| T2 | Şema v0 | Alt ajan | Rust tipleri `crates/qp-schema` içinde: `Series`, `SeriesSource`, `Fact`, `Finding`, `Capsule`, `Glyph`, `View`, `Depth`, `Document`, `Command`, `Intent`, `Ledger`, `Provenance`. JSON Schema schemars 1.x ile draft 2020-12 olarak üretiliyor; 9 kök tip var. Örnekler `fixtures/schema/` altında: THYAO verisi, RSI(14)'ün 70'i kesmesi olgusu, bir Capsule ve bir Document. 16 test: gidiş-dönüş, geçerlilik, determinizm, proptest. |
| T3 | Çizim denemesi | Alt ajan | Çizici tiny-skia ve fontdue kullanıyor; gömülü font IBM Plex Mono (OFL). Deterministik çizilenler: eksenler, mumlar, SMA çizgisi, metin. `fixtures/bars/sample.json` 200 bardan oluşuyor; sabit tohumlu LCG ile üretildi. İki altın görsel var: `sample.png` ve stres sahnesi `stress.png`. Diğer parçalar: `qp-wasm` (wasm-bindgen), `shells/web-spike/` (Playwright ile tarayıcı karşılaştırması), üç yığın arasında ölçüm ve `crates/qp-render/SPIKE.md`. |
| T4 | Eski kod envanteri | Alt ajan | `docs/LEGACY.md` ve `docs/MATRIKS_PROTOCOL.md` yazıldı. İkisinde de gizli bilgi yok; eski klasörlere yazılmadı. |
| T5 | `qp` komut satırı | Alt ajan | Komutlar: `qp schema export --out`, `qp schema validate --type [--schema-dir]` ve `qp render -o [--width --height]`. 7 uçtan uca test yazıldı. |
| T6 | Doğrulama düzeneği | Alt ajan | `scripts/verify.sh` 9 adım çalıştırıyor: fmt, clippy, test, schema, fixtures, golden, wasm, hygiene, browser. Bash 3.2 uyumlu ve sonunda PASS/FAIL/SKIP tablosu basıyor. `.github/workflows/ci.yml` bu betiği çağırıyor; tarayıcı adımı CI'da zorunlu. |
| T7 | Denetim | Bağımsız denetçi alt ajan | Temiz klonda 13 adım ve 8 ölçüt çalıştırıldı (`rounds/R0/AUDIT.md`). |
| — | A1 kararı | Orkestratör | `DECISIONS.md` içinde K12 olarak kapatıldı. |

Her alt görev kendi worktree'si ve dalında çalıştı, sonra `main`'e `--no-ff` ile birleştirildi. Tek çakışma `Cargo.lock` içindeydi. T3'ün kilidi esas alındı; böylece altın görseli etkileyen `png`/`fdeflate` sürümleri sabit kaldı. Birleştirme sonrası testler yeşil.

## Ölçümler

Tüm ölçümler Apple M4 Max üzerinde yapıldı. x86_64 satırları Rosetta 2 altında çalıştı.

| Ölçüm | Değer |
|-------|-------|
| `cargo test --workspace` | 35 test, 0 hata |
| `scripts/verify.sh`, soğuk `target/` | 63–66 s, 9/9 PASS |
| `scripts/verify.sh`, sıcak `target/` | yaklaşık 9 s |
| İlk kare, yerel (font, JSON, sahne, raster, PNG) | 10–13 ms |
| Kararlı kare, yerel (1200x600) | 0,75 ms |
| 100.000 bar, yerel aarch64 | 38 ms (denetçi yeniden ölçtü: 37,83 ms) |
| 100.000 bar, x86_64 (Rosetta) | 67 ms |
| 100.000 bar, tarayıcı (wasm) | 80 ms |
| Tarayıcı: wasm init / ilk kare / ikinci kare | 6 / 13 / 3,2 ms |
| WASM (wasm-pack, `--profile wasm`): ham / gzip -9 | 959.199 B / 362.694 B |
| WASM, wasm-opt -Oz sonrası: ham / gzip | 829.736 B / 330.965 B |
| WASM (`cargo build --profile wasm`, `verify.sh` 7. adım): ham / gzip | 1.016.781 B / 368.163 B |
| Tarayıcı ↔ ekransız fark | **0 piksel**. Canvas piksel okuması, canvas ekran görüntüsü, wasm RGBA ve stres sahnesinde aynı sonuç; PNG baytları da aynı. |
| aarch64 ↔ x86_64 ↔ wasm32 | Bit-eşit. FNV özetleri tüm hedeflerde aynı: `1fc9e1e055002efd` / `a718ba2b22abb123`. |

**Esas alınacak WASM boyutu:** wasm-pack paketi, yani gzip ile 363 KB (wasm-opt sonrası 331 KB). `verify.sh` içindeki cargo çıktısı wasm-bindgen'den geçmemiş ham modüldür; yalnızca gerilemeyi izlemek için kullanılır. Gömülü font bu boyutun yaklaşık 135 KB'ı.

Alternatif yığınların ölçümleri (vello_cpu 0.4, vello 0.12 GPU) `crates/qp-render/SPIKE.md` ve K12 içinde.

## Kabul ölçütleri

| # | Ölçüt | Sonuç | Kanıt |
|---|-------|-------|-------|
| 1 | `scripts/verify.sh` temiz kopyada tek komutla yeşil | ✅ | AUDIT adım 1: çıkış kodu 0, 9/9 PASS, tarayıcı adımı da çalıştı. |
| 2 | Şema gidiş-dönüş; örnek Kapsül ve Doküman üretilen şemaya göre geçerli | ✅ | AUDIT adım 4–6. Ters denemeler de beklenen gibi çıkış kodu 1 verdi. |
| 3 | `qp render` iki çalıştırmada aynı ve altınla eşleşiyor | ✅ | AUDIT adım 7–8: `cmp` ve SHA-256 aynı. |
| 4 | Aynı kod wasm32'de tarayıcıda çiziyor; fark ölçülmüş | ✅ | AUDIT adım 9–10: 0 px fark. Denetçi ekran görüntüsünü kendisi açıp inceledi. |
| 5 | A1 kanıtıyla kapatılmış | ✅ | `DECISIONS.md` K12. Denetçi özetleri ve 100k bar süresini yeniden üretti. |
| 6 | `docs/LEGACY.md` ve `docs/MATRIKS_PROTOCOL.md` yazılmış; gizli bilgi yok | ✅ | AUDIT ölçüt 6, adım 11. |
| 7 | Depoda `.env` veya token yok; `.gitignore` dışlıyor | ✅ | AUDIT adım 11. |
| 8 | `REPORT.md` yazılmış, `STATUS.md` güncellenmiş | ✅ | Bu dosya, `STATUS.md` ve `ROADMAP.md`. |

## Sapmalar

1. **Renderer'ın kendi bar tipi var.** `qp-render` kendi `BarsFile`/`Bar` tipini tanımlıyor; `fixtures/bars` biçimi de üretilen JSON Schema kapsamında değil. T2 ve T3 paralel yürüdüğü için bilinçli bir R0 kısayolu. Yine de "şemanın tek kaynağı `qp-schema`" kuralından küçük bir sapma. R1'de `qp-schema::Series` ile birleştirilmeli.
2. **SMA20 çizicinin içinde.** Hesap `crates/qp-render/src/scene.rs` içinde yalnızca görsel yer tutucu olarak duruyor. f64 ve deterministik, ama hesap çekirdeğine ait. R1'de `qp-compute`'a taşınmalı.
3. **CLI'da kapsam dışı bir komut var.** `qp schema validate` kapsamda yoktu. VERIFY 6. adımın ekransız ve dışa aktarılmış şemaya karşı çalıştırılabilmesi için eklendi. Doğrulama mantığı şu an CLI'da; `qp-schema`'ya taşınması önerilir.
4. **Fazladan altın görsel ve özellikler.** Brief'te olmayan bir stres altın görseli (`fixtures/golden/stress.png`) eklendi. `qp-render` ve `qp-wasm`'a isteğe bağlı `simd` ve `spike-*` özellikleri de eklendi. Varsayılan derleme ve testler bunlara bağlı değil.
5. **Sözlükte olmayan tip adları.** `Mark`, `Layer`, `Scale`, `Author`, `Condition`, `Op`, `Point`, `SeriesData`, `LedgerEntry`, `Outcome` `docs/CONCEPTS.md` içinde yok. Bunlar yardımcı tiplerdir. "Çizimler" (`Drawing`) sözlükte kod adı olmadığı için şemaya hiç eklenmedi.
6. **VERIFY 12. adım uyarlandı.** `../Docs` ve `../Services` git deposu olmadığından `git status` yerine round öncesi alınan değişiklik zamanı ve boyut listesiyle karşılaştırıldı. Sonuç: 302 dosya, değişiklik yok.

## Açık sorular

1. **Gerçek x86/Linux determinizmi ölçülmedi.** CI iş akışı yazıldı ama depoda git remote yok, CI hiç çalışmadı. İlk gerçek x86_64/Linux altın görsel ölçümü ilk push ile gelecek. Depo nereye konacak?
2. **Altın görsel aynı koddan üretildi.** Bayt eşitliği değişmezliği kanıtlar, doğruluğu değil. Denetçi son kapanışı (124,69) ve SMA20 değerini elle doğruladı. R2'de gramerle birlikte görsel doğruluk için bağımsız bir kontrol gerekir: göz onayı ya da analitik piksel testleri.
3. **Tarayıcı testi tarayıcının kendi rasterleştirmesini sınamıyor.** wasm aynı CPU kodunu çalıştırıyor, sonuç canvas'a `putImageData` ile kopyalanıyor. GPU yolu (vello/WebGPU) otomatik doğrulamada yok. R2'de toleranslı bir GPU testi eklenmeli.
4. **Tarayıcı adımı sessizce atlanabiliyor.** `verify.sh` içinde `node` veya `wasm-pack` yoksa tarayıcı adımı SKIP olup yine OK sayılıyor. CI'da `QP_VERIFY_BROWSER=1` ile zorunlu. Yerelde de zorunlu olsun mu?
5. **Font alt kümelemesi yapılmadı.** IBM Plex Mono OFL lisansında "Plex" ayrılmış ad olduğu için alt küme yeniden adlandırma gerektirir. Bu yüzden tam font gömüldü (135 KB).
6. **`.proto` dosyaları eksik.** Matriks protobuf tanımları eski klasörlerde yok. R6 için kaynak gerekiyor; ayrıntı `docs/MATRIKS_PROTOCOL.md` sonunda.
7. **Eski repoda gerçek token olabilir.** `../Services/quantex-charts-core/ts/.env` muhtemelen gerçek bir token içeriyor ve eski `.gitignore` onu dışlamıyor. Dosya açılmadı. Token'ın iptal edilip yenilenmesi önerilir; karar Arkin'de.

## Öneriler

### R1 için

1. **Bar ve seri tipini birleştir.** `qp-render::BarsFile`/`Bar` ile `qp-schema::Series`/`Bar` tek tip olsun; `fixtures/bars` biçimi de üretilen şemaya bağlansın.
2. **Eski 5 indikatörü akışkan `Study` olarak yeniden yaz.** İndikatörler: SMA(20), EMA(20, SMA tohumlu), RSI(14, Wilder), MACD(12,26,9; 3 çıktı), Bollinger(20, 2σ, popülasyon std; 4 çıktı). Hepsi eski `core/src/indicator/` altında. Eski toplu sonuçlar test kâhini olarak kullanılsın. Eski `OHLCV` tipi `f32`; kopyalanmaz, yeniden yazılır.
3. **Isınma dönemi çıktısının kuralına karar ver.** Eski kod baştaki değerleri kırpıyor; NaN ya da `null` doldurma seçenekleri var. Karar şemaya yazılsın.
4. **Çizicideki SMA20'yi `qp-compute` çıktısına bağla.**
5. **Borsa saat dilimini sözlüğe ekle.** BIST İstanbul saatinde çalışıyor; eski kod barları UTC'ye hizalıyor. Önce `docs/CONCEPTS.md`'ye girsin.
6. **Şema doğrulamasını taşı.** Doğrulama `qp-schema` kütüphanesine geçsin (`validate(name, &Value)`), CLI ince kalsın.
7. **proptest'i CI'da tekrarlanabilir yap.** Sabit tohum ya da `PROPTEST_CASES` kullanılsın.

### R2 ve sonrası

- **R2:**
  - vello + WebGPU, yalnızca MSAA kipinde. Önce CPU karesi gösterilsin, GPU hazır olunca ona geçilsin.
  - Eski kodun viewport, LOD (200/1000/5000/10000) ve hit-test matematiği taşınsın; bunlar için gidiş-dönüş özellik testleri yazılsın.
  - 100k bar için LOD.
  - Metin şekillendirme (kerning) ihtiyacı değerlendirilsin.
- **R6:**
  - Matriks bağlayıcısının crate adı ve konumu `docs/ARCHITECTURE.md`'ye eklensin.
  - `.proto` dosyaları bulunsun ve tipler `prost` ile üretilsin.
  - Token alma ve yenileme akışı netleşsin.
  - Eski koddaki hatalar tekrarlanmasın: MQTT keepalive birimi, kapalı TLS doğrulaması, yeniden bağlanırken eski istemcinin kapatılmaması.
- **Hijyen:** Depo uzak sunucuya konunca CI'ın ilk çalışması izlensin. Linux altın görseli tutmazsa hangi kısmın kaydığı (kodlayıcı mı, raster mi) test çıktısındaki piksel farkı istatistiğinden görülür.
- **Sözlük:** `Drawing` kavramı ve yardımcı tip adlarının (`Mark`, `Layer`, `Scale` vb.) sözlüğe girip girmeyeceği ana akışta karara bağlansın.
