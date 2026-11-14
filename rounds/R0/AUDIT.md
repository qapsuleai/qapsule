# R0 — Bağımsız Denetim (T7)

Denetçi: ayrı alt ajan (işi yapan ajanlardan bağımsız). Hiçbir şey düzeltilmedi; yalnızca ölçüldü.
Bu dosya dışında asıl depoya yazılmadı, commit yapılmadı.

## Ortam

| Öğe | Değer |
|-----|-------|
| Klonlanan commit | `2c3d0490f1a2ac94acdf35817ec9f75e91e6b38f` (`main`, "Merge T6: scripts/verify.sh and CI wiring") |
| Klon yeri | `<scratchpad>/audit` — taze `git clone`, `target/` ve `node_modules/` yok |
| Asıl depo `git status` | temiz (denetim başında) |
| OS | macOS 27.0.1 (26A434), Apple Silicon (aarch64) |
| rustc / cargo | 1.92.0 (ded5c06cf 2025-12-08) / 1.92.0 |
| Kurulu hedefler | aarch64-apple-darwin, wasm32-unknown-unknown, x86_64-apple-darwin, x86_64-unknown-linux-gnu |
| node | v24.3.0 |
| wasm-pack | 0.14.0 (önceden kurulu, `~/.cargo/bin`) |
| Playwright / Chromium | 1.62.0 / headless shell 151.0.7922.34 (kullanıcı önbelleğinde önceden kurulu) |

Not: "temiz kopya" depo için geçerli; araç zinciri (wasm-pack, Playwright Chromium, cargo kayıt önbelleği) makinede hazırdı.

## VERIFY.md adımları

| # | Komut / kontrol | Beklenen | Gerçek çıktı (özet) | Sonuç |
|---|-----------------|----------|---------------------|-------|
| 1 | `scripts/verify.sh` | Çıkış kodu 0 | rc=0, `verify: OK`, toplam 63 s (soğuk `target/`). 9 adım PASS: fmt 0 s, clippy 6 s, test 12 s, schema 34 s (9 şema, iki dışa aktarım aynı), fixtures 0 s (2 geçerli), golden 0 s (altınla bayt eşit), wasm 4 s, hygiene 0 s, browser 7 s (0 px). Hiçbir adım SKIP olmadı. | PASS |
| 2 | `cargo fmt --all -- --check` | Fark yok | rc=0, çıktı yok | PASS |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | Uyarı yok | rc=0 | PASS |
| 4 | `cargo test --workspace` | Tüm testler geçer | 35 test geçti, 0 başarısız (qp-cli, qp-render, qp-schema; qp-wasm'da test yok) | PASS |
| 5 | `cargo run -p qp-cli -- schema export --out <dizin>` | JSON Schema dosyaları üretilir | `/tmp/schema` yerine `<scratchpad>/sx/schema` kullanıldı. rc=0; 9 dosya: Capsule, Command, Document, Fact, Finding, Intent, Ledger, Series, View `.schema.json` | PASS |
| 6 | `fixtures/schema/*.example.json` üretilen şemaya göre | Geçerli | `qp schema validate … --type Capsule/Document --schema-dir <5'in çıktısı>` → ikisi de `valid`, rc=0. Negatif kontrol: Capsule örneğine `bogus_field` eklenince rc=1 ("Additional properties are not allowed"); Document örneği `--type Capsule` ile rc=1. Doğrulayıcı gerçekten doğruluyor. | PASS |
| 7 | `qp render fixtures/bars/sample.json -o a.png` iki kez | Birebir aynı | release `target/release/qp`; `cmp a.png b.png` → aynı. 29.850 bayt, 0,10 s | PASS |
| 8 | 7'nin çıktısı ↔ `fixtures/golden/` | Eşleşir | `cmp a.png fixtures/golden/sample.png` → aynı; SHA-256 `ed0672fb…6e10` ikisinde de | PASS |
| 9 | WASM derlemesi | Başarılı; boyut | `cargo build -p qp-wasm --target wasm32-unknown-unknown --profile wasm` rc=0: ham 1.016.781 B, gzip -9 368.163 B. wasm-pack (`--no-opt`) paketi `qp_wasm_bg.wasm`: ham 959.199 B, gzip 362.694 B; `wasm-opt -Oz` 829.736 B, -Oz+gzip 330.965 B | PASS |
| 10 | Test sayfası başsız tarayıcıda, ekran görüntüsü | Çizim var; fark ölçülür | `shells/web-spike/run.sh` rc=0. 1200x600; canvas↔altın 0 px, ekran görüntüsü↔altın 0 px, wasm RGBA↔altın 0 px, stres sahnesi↔altın 0 px, wasm PNG baytları = altın. `target/web-spike/canvas-screenshot.png` üretildi; denetçi gözle baktı: koyu zemin, ~200 yeşil/kırmızı mum, turuncu SMA20 çizgisi, sağda fiyat ekseni (90–130), altta tarih ekseni (2025-02…2025-10), başlık "SAMPLE 1d  C 124.69", "SMA20 124.97", kırmızı son fiyat etiketi. `fixtures/golden/sample.png` görsel olarak aynı. | PASS |
| 11 | `git ls-files \| grep -i -E '\.env$\|token\|secret'` | Sonuç yok | Çıktı yok (rc=1). Ek `git grep` (JWT `eyJ…`, `password/api_key/secret = "…"`) → sonuç yok. Geniş `token/password/bearer` araması yalnızca belge metni buldu (kurallar, LEGACY risk tablosu, protokol açıklaması `Authorization: jwt <token>` yer tutucusu); gerçek değer yok. `.gitignore`: `.env`, `.env.*`, `*.env`, `*.pem`, `*.key`, `*.p12`, `secrets/`, `*token*` | PASS |
| 12 | `../Docs`, `../Services` değişmedi | Değişiklik yok | İkisi de git deposu değil (`git status` → "not a git repository"); bu yüzden stat tabanlı kontrol. Round öncesi taban çizgisi (14:26:53; ilk commit 14:27) ile aynı komutla yeniden üretilen liste (302 satır) `cmp` ile **birebir aynı**. Taban dosyasından yeni (`-newer`) dosya/dizin sayısı 0. En yeni eski dosya 00:30. `.env` içeriği açılmadı. | PASS |
| 13 | `DECISIONS.md` A1 | Kanıtıyla kapatılmış | A1 "KAPANDI", K12'ye taşındı; denenen seçenekler (tiny-skia+fontdue, vello_cpu, vello GPU), ölçümler ve seçim yazılı; kanıt `crates/qp-render/SPIKE.md` depoda. Yeniden üretildi: `bench` örneği aarch64'te `sample_rgba_fnv 1fc9e1e055002efd`, `bars100k_rgba_fnv a718ba2b22abb123`, 100k bar raster 37,83 ms (SPIKE: 38); x86_64 (Rosetta) aynı FNV özetleri; tarayıcı 0 px ve wasm 959.199 B SPIKE ile aynı. | PASS |

## Kabul ölçütleri (GOAL.md)

| # | Ölçüt | Sonuç | Kanıt |
|---|-------|-------|-------|
| 1 | `scripts/verify.sh` temiz kopyada tek komutla yeşil | PASS | Adım 1: taze klonda rc=0, 63 s, 9/9 PASS (browser dahil). |
| 2 | Şema JSON gidiş-dönüş; örnek Kapsül ve Doküman var ve üretilen şemaya göre geçerli | PASS | `qp-schema` testleri (roundtrip, proptest, validation) geçti; adım 5–6 ile `capsule.example.json`, `document.example.json` dışa aktarılan şemaya göre geçerli; negatif testler reddediyor. JSON Schema elle yazılmamış (depoda `*.schema.json` yok, `schemars` ile üretiliyor). |
| 3 | `qp render … -o out.png` iki çalıştırmada aynı ve altınla eşleşiyor | PASS | Adım 7–8: bayt eşit, SHA-256 aynı. (Bkz. Bulgu 1: altın aynı koddan üretildi.) |
| 4 | Aynı kod `wasm32`'ye derleniyor, test sayfası aynı veriyi çiziyor, fark ölçülüp raporlanmış | PASS | Adım 9–10: canvas / ekran görüntüsü / wasm RGBA ↔ altın 0 px; SPIKE.md'de raporlanmış. (Bkz. Bulgu 2: tarayıcıda rasterleştirme yine wasm CPU koduyla.) |
| 5 | A1 kanıtıyla kapatılmış | PASS | Adım 13; sayılar depoda ve kısmen yeniden üretildi. |
| 6 | `docs/LEGACY.md` (taşınacak/taşınmayacak + nedenler), `docs/MATRIKS_PROTOCOL.md` (gizli bilgi yok) | PASS | LEGACY.md (211 satır): genel yapı, "Taşınacaklar" tablosu (kaynak → hedef crate → round → neden), eski 5 indikatör, "Taşınmayacaklar" tablosu nedenleriyle, benchmark referansı, viewport/LOD matematiği, 16 maddelik risk tablosu. MATRIKS_PROTOCOL.md (429 satır): ortamlar, kimlik doğrulama biçimi, discovery, topach, REST, MQTT kuralları, konu ağacı, protobuf mesajları, saat dilimi, yeniden bağlanma, açık sorular; kaynak/güven etiketli. Gizli bilgi taraması temiz: yalnızca herkese açık host adları (`api.matriksdata.com`, `rt.radix.matriksdata.com`) ve `<token>` yer tutucuları; JWT, parola, IP yok. `.env` dosyalarının açılmadığı belgelenmiş. |
| 7 | Depoda `.env` / token yok; `.gitignore` dışlıyor | PASS | Adım 11. |
| 8 | `rounds/R0/REPORT.md` yazılmış, `STATUS.md` güncellenmiş | BEKLİYOR (orkestratör) | Denetim anında `REPORT.md` yok; `STATUS.md` en son T1 commit'inde değişmiş. Denetimden sonra orkestratör yazacak. |

## Ölçülen sayılar

| Ölçüm | Değer |
|-------|-------|
| `verify.sh` toplam (soğuk `target/`) | 63 s (user 204 s, %347 CPU) |
| Test sayısı | 35 geçti / 0 başarısız |
| `qp render` (release) | 0,10 s duvar saati; PNG 29.850 B |
| Yerel bench (aarch64, tiny-skia+fontdue) | ilk kare 18,33 ms; sample raster medyan 0,85 ms; PNG kodlama 3,52 ms; 100k bar sahne 1,02 ms + raster 37,83 ms |
| x86_64 (Rosetta) determinizm | FNV özetleri aarch64 ile aynı |
| WASM (cargo, `--profile wasm`) | 1.016.781 B ham / 368.163 B gzip -9 |
| WASM (wasm-pack paketi) | 959.199 B ham / 362.694 B gzip; -Oz 829.736 B / 330.965 B gzip |
| Tarayıcı ↔ altın | canvas 0 px, ekran görüntüsü 0 px, wasm RGBA 0 px, stres 0 px, max kanal farkı 0 |
| Tarayıcı süreleri (ms) | başlatma 92, sayfa→bitti 176, wasm init 6,6, ilk kare 12,5, ikinci kare 3,2, putImageData 1,3, render_png 14,5, 100k bar 79,2, stres 34,9 |
| Eski klasörler | 302 dosya, taban çizgisiyle birebir aynı |

## Bulgular / riskler

1. **Altın görsel aynı koddan üretilmiş (kendini doğrulama riski).** `QP_UPDATE_GOLDEN=1` ile yazılıyor; bayt eşitliği yalnızca determinizmi kanıtlar, doğruluğu değil. Denetçi bağımsız kontrol yaptı: son kapanış 124,69 (veri), son 20 kapanışın ortalaması 124,9705 → etiket "SMA20 124.97" doğru; veri aralığı 89,39–130,68 eksen 90–130 ile uyumlu; görüntü gözle anlamlı bir mum grafiği. Test, altın eksikse otomatik yazmıyor (hata veriyor) — iyi.
2. **Tarayıcı karşılaştırması tarayıcının kendi rasterleştirmesini sınamıyor.** Sayfada çizimi wasm içindeki aynı CPU çizici yapıyor, tarayıcı yalnızca `putImageData` ile kopyalıyor. 0 px sonucu bu yüzden beklenen; ölçüt 4'ü karşılıyor ama GPU/WebGPU yolu (vello MSAA, K12) yalnızca spike düzeyinde ve otomatik doğrulamada yok. R2'de toleranslı GPU testi gerekecek.
3. **`verify.sh` tarayıcı adımı araç yoksa SKIP olup yine "OK" diyor.** `node` veya `wasm-pack` bulunmayan makinede ölçüt 4 sessizce atlanır. CI `QP_VERIFY_BROWSER=1` ile zorluyor, ama depoda git remote yok; **CI hiç çalışmadı**. Gerçek Linux/x86 üzerindeki ilk determinizm ölçümü (K12'deki açık risk) hâlâ yapılmadı.
4. **Kapsam sınırı: SMA20 hesabı çizicide.** `crates/qp-render/src/scene.rs:270-281` içinde 20 periyotluk SMA hesaplanıp çiziliyor. Küçük ve deterministik (f64), ama indikatör hesabı R1 hesap çekirdeğinin işi; "hesap çekirdekte" mimarisine göre R1'de `qp-compute`'e taşınmalı.
5. **Şemanın tek kaynağı kuralına sapma (küçük).** `qp-render/src/bars.rs` kendi `Bar`/`BarsFile` tipini tanımlıyor (`qp-schema::Bar` ile aynı alanlar, f64), `qp-render` `qp-schema`'ya bağlı değil. `fixtures/bars/sample.json` biçimi üretilen JSON Schema kapsamında değil. R1/R2'de birleştirilmeli.
6. **Sözlük:** Şemada `Mark`, `Layer`, `Scale`, `Author`, `Condition`, `Op` gibi tipler `docs/CONCEPTS.md`'de ayrı satır olarak yok (yalnızca `View` açıklamasında "işaret, ölçek, katman" geçiyor). "Yeni kavram önce sözlüğe" kuralı açısından sınırda.
7. **İki farklı "wasm boyutu" sayısı var:** `verify.sh` cargo çıktısını (1.016.781 B / 368.163 B gzip), SPIKE.md ve `run.sh` wasm-pack paketini (959.199 B / 362.694–362.710 B gzip) raporluyor. REPORT'ta hangisinin esas alındığı belirtilmeli. SPIKE'taki gzip sayısı (362.710) ölçülen (362.694) ile 16 B farklı — önemsiz.
8. **Altın PNG baytları kodlayıcı sürümüne bağlı** (`png`/`fdeflate`; `Cargo.lock` sabitliyor). SPIKE.md bunu belirtiyor; test uyuşmazlıkta piksel farkını ayrıca yazıyor.
9. **Kural kontrolleri temiz:** fiyatlar `f64` (şema ve `bars.rs`); `f32` yalnızca piksel/font koordinatlarında. Rastgelelik yalnızca sabit tohumlu LCG (sentetik bar ve stres sahnesi) — deterministik, hesap kodu değil. Ağ, saat (`SystemTime`/`Instant`) kullanımı ürün kodunda yok. Şema tipleri yalnızca veri; kapsül davranışı, gramer veya hesap çekirdeği uygulanmamış.
10. **Yazma alanları:** Commit'ler TASKS.md'deki alanlara uyuyor (T2 → `crates/qp-schema`, `fixtures/schema`; T3 → `crates/qp-render`, `crates/qp-wasm`, `fixtures/bars|golden`, `shells/web-spike`; T4 → `docs/`; T5 → `crates/qp-cli`; T6 → `scripts/`, `.github/`; ortak `Cargo.lock`). `NORTH_STAR.md` yalnızca ilk commit'te var, değiştirilmemiş. `../Docs`, `../Services` dokunulmamış (adım 12). `verify.sh` sonrası klonda `git status` temiz.
11. **Eski depo hijyeni (kapsam dışı, bilgi):** LEGACY.md eski `ts/.env` dosyasının `.gitignore` ile dışlanmadığını ve demo'nun token'ı `sessionStorage`'a yazdığını not ediyor; eski token'ın iptali önerilmiş. Yeni depoya sızıntı yok.
12. **Ortam notu:** wasm-pack ve Playwright Chromium makinede önceden kuruluydu; tamamen boş bir makinede `run.sh` Chromium'u indirmeye çalışır (ağ gerekir).
