# RP3/S denetimi (bağımsız denetçi)

Tarih: 2026-11-17. İncelenen: `rp3/s-paint` @ 6c3c0b2 (taban `main` 0512196), `rp3/https-host`
@ 21f086e (taban 0512196). Dallar değiştirilmedi. Derleme scratchpad'deki ayrı çalışma
ağaçlarında yapıldı (işten sonra kaldırıldı). Portlar: 8500 (paint-paths), 8501 (ölçüm);
verify'ın kendi demo portları çakışmadı.

## Sonuç

| Madde | Karar |
|---|---|
| a. Kapsam ve parmak izi | **PASS** |
| b. Mimari | **PASS** (bir not) |
| c. `QP_VERIFY_BROWSER=1 scripts/verify.sh` | **PASS — 18/18** |
| d. VERIFY 5/8: paint-paths, Chromium + WebKit | **PASS** |
| e. VERIFY 4/7: PHONE ölçümü tekrarı | **PASS** (±%20 içinde) |
| f. VERIFY 6 (M14) sayaçları | **PASS** |
| g. https-host | **PASS**; iki dal temiz birleşiyor |

Birleştirme koşulu yok. Notlar aşağıda.

## a. Kapsam

- `git diff 0512196..rp3/s-paint --stat`: 48 dosya; `fixtures/**` içinde hiç değişiklik yok
  (altın görseller, `fixtures/protocol/**`, `fixtures/compute/digest.txt` aynı). Hesap
  özeti verify 12. adımda `arm64, x86_64, wasm32: 7b1e99d40889f83c` = `fixtures/compute/digest.txt`.
- `digests()` (`shell.rs:2336`), `drive.rs` ve `view_digest` hâlâ `scene_digest` (JSON üzerinde
  FNV, `qp-session/src/viewer.rs:54`) kullanıyor. `scene_fingerprint` yalnız `shown_key`'de
  geçiyor; o da yalnız `face_changed` / `scene_changed` içinde ("değişti mi" kısa devresi,
  `face_shown` / `scene_shown`). Dışarıya hiçbir özet olarak çıkmıyor.
- `scene_fingerprint` (`qp-render/src/retained.rs:184`) `width`, `height` ve `background`
  alanlarını, sonra `SceneDesc::prims()` (under → panel katmanları → over, boyama sırası)
  ile gelen her ilkelin `key()` değerini ve ilkel sayısını özetliyor. `key()`'i `qp-schema`
  `Prim` ile karşılaştırdım: Rect (x, y, w, h, color), Polyline (points bit bit, width, color),
  Fill (points, color), Circle (cx, cy, r, color) ve Text (x, baseline, px, text, color,
  align) alanlarının hepsi giriyor. `Align` yalnız Left/Right olduğu için bir bool yetiyor.
  `SceneDesc`'in öteki alanları (`view`, `x`, `bars`, `lod`, `panels[].rect/y/items/...`,
  `emphasis`) piksele ancak ilkellerin üzerinden ulaşıyor; çizici bunları okumuyor (cpu.rs'de
  kırpma yok). Aygıt ölçeği (`scale`) sahne tarifinde yok. Ama ölçek değişince
  `face_shown.clear()` ve `scene_shown = None` çalışıyor (`shell.rs:1244–1247`); eski JSON
  özetinde de durum aynıydı, gerileme yok. **Eksik alan bulunmadı.** Test:
  `scene_fingerprint_flips_on_any_pixel_field` geçiyor.
- Bilinçli davranış değişikliği: JSON özeti, piksel dışı alanlar değişince de "değişti"
  diyordu. Parmak izi demiyor (S.md §6.1: `ref` idle yüz boyaması n 12 → 4). Bu doğru
  davranış. Taplar ve okumalar sahneyi çekirdekte yeniden kurduğu için verilen piksellere
  bağımlı değil. paint-paths bunu doğruluyor.
- Kenar not: `DefaultHasher` (SipHash 1-3, sabit anahtarlar) sürümler arasında kararlılık
  garantisi vermiyor. Burada yalnız aynı süreç içinde karşılaştırıldığı için sorun yok;
  özet olarak dışarı verilmemeli. Kod yorumu bunu söylüyor.

## b. Mimari

- `node shells/web/arch-check.mjs` sonucu: 0 ihlal (main.js ve 6 boyama dosyası); öz sınama 22/22.
- `eval`, `Function(`, `new Function`, dizgeyle `setTimeout`, dinamik `import(` yok.
- `canvas.js`: akış `ops.values()` yineleyicisiyle okunuyor. Komut başına tek bir tuval çağrısı
  var. Dizgeler çekirdeğin tablosundan geliyor, yalnız özelliğe atanıyor (`fillStyle`, `font`,
  `textAlign`), kod olarak değerlendirilmiyor. `ops[0] === INIT && ops[5] === CLIP` sabit dizinli
  bir okuma. Yalnız ölçüm açıklığını (`canvas_part`) seçiyor, alan mantığı değil.
- `cpu.js`: yanıt dizisinin sabit dizinleri (`r[0]…r[10]`) okunup doğrudan
  `putImageData`'ya veriliyor. Aritmetik yok.
- `gpu.js`: `GPUTextureUsage.A | B` bayrak birleştirmesi (API düzeyinde, alan mantığı değil) ve
  `v.byteLength > vbuf.size` karşılaştırması var (ARCH.md'de gerekçelendirilmiş). **Not:** WGSL
  gölgelendirici dizgesinde `floor(uv)` ve `color * a` geçiyor. Bu, GPU'da genel bir doku
  örnekleme ve renk çarpma işi; alan mantığı değil, denetleyici JS dışını taramıyor. Kabul
  edilebilir ama ARCH.md'de açıkça anılması iyi olur (Öneri).
- Yeni izinli çekirdek yöntemlerinin hepsi (M14: 6, M12b: 5, M12a: 6, M12c: 9) ARCH.md'de
  gerekçeli tablolarda var. `arch-check.mjs`'teki liste bunlarla birebir uyuşuyor.

## c. verify (birleştirme kapısı)

`QP_VERIFY_BROWSER=1 scripts/verify.sh`, `rp3/s-paint` @ 6c3c0b2, temiz scratch çalışma ağacı:
**18/18 PASS, `verify: OK`, çıkış 0**, toplam 748 s. Öne çıkanlar: 3 test 329 s;
12 digest `7b1e99d40889f83c`; 15 replica viewer lag 0/25; 17 shell; 18 demo: paint-paths
(Chromium) her yol = ref, paint-canvas 16 görüntü tolerans içinde, `?paint=ref` ile
26 görüntü × 2 altın görsellere eşit.

## d. VERIFY 5/8 — `tests/paint-paths.mjs --browsers chromium,webkit --port 8500`

- Chromium: ref, canvas ve cpu kendi yollarıyla açılıyor. gpu `{"fallback":"unsupported","used":"ref"}`
  döndürüyor (başsız kabukta WebGPU bağdaştırıcısı yok), yedek raporlanıyor. Her yolda yüz ve
  sahne boyanıyor.
- WebKit: dört yol da kendisiyle açılıyor (gpu WebKit'te gerçekten `gpu`).
- Her iki motorda canvas, cpu ve gpu = ref: digests, feed, open ve 10 yetke cevabı
  (tap_at/hit, screen) bayt düzeyinde aynı. Varsayılan (parametresiz) = `canvas`.
  Sonuç `paint-paths: ok`.

## e. VERIFY 4/7 — PHONE ölçüm tekrarı

`RUNS=3 ORDERS=forward PORT=8501 PATHS=ref,canvas measure.sh <out> phone` (WebKit 26.5,
`__XPC_JSC_useJIT=false`, DPR 3, 402×874), ardından `breakdown.py`. 17:26–17:30 arasında
3 koşu, 0 hata. Yük (`uptime` 1 dk): 5,18 → 5,98 → 8,00 → 6,13.

| Ölçü (3 koşu ortancası) | Rapor (S.md §6.1, B) | Denetim | Fark |
|---|---|---|---|
| canvas `chart_ticks` geçiş p50/p95 | 7/10 | 8/11 | +%14 / +%10 |
| canvas `chart_ticks` tik+geçiş / çerç. | 7,3 | 8,06 (koşular 8,16 · 8,07 · 7,38) | +%10 |
| ref `chart_ticks` tik+geçiş / çerç. | 9,4 | 9,58 (9,58 · 9,65 · 8,71) | +%2 |
| canvas `idle` tik+geçiş | 1,3 | 1,28 | −%2 |
| ref `idle` tik+geçiş | 5,6 | 5,36 | −%4 |
| değişiklik sınaması / çerç. (canvas, chart) | 0,7 | 0,77 | +%10 |
| sahne kurma / çerç. (canvas, chart) | 3,2 | 3,52 | +%10 |

Hepsi ±%20 içinde, VERIFY 4 PASS. Not: ref `chart_ticks` geçiş p95 tek koşuda 49'a çıktı
(öteki koşular 34 ve 9; ortanca 34, raporda 10). p95 gürültülü ve yükten etkileniyor. Kararı
ortalama (tik+geçiş) belirliyor; rapor da bunu kullanıyor. S.md'nin "PHONE'da p95 hedefi
(≤ 8 ms) tutmuyor (1 + 10/11)" saptaması denetimde de geçerli. Ölçüt 2/3 için **kısmi**
tablosu doğru okunuyor.

**Dikkat (yan etki):** Ölçümün çıktısını scratchpad'deki `meas/` klasörüne yazdım. Bu klasörde
S ajanının 15:34–16:26 ham verileri de duruyormuş (aynı scratchpad yolu). Benim koşularım
`meas/phone/forward-{1,2,3}/` altındaki `compare-webkit.json`, `log.txt` ve `load-*.txt`
dosyalarının üzerine yazdı. `reverse-*`, `phone_jit`, `cdp4*` ve `tables.md`'ye
dokunulmadı. S.md'deki sayılar dalda commit edilmiş durumda (S.md, S/tables.md); kayıp yalnız
o üç koşunun scratch ham dosyaları.

## f. VERIFY 6 (M14)

- Denetim koşularındaki sayaçlar (PHONE, viewer kipi): `passes_per_data_frame` her yol ve
  evrede 1,000–1,032; S'nin ters sıra koşularında en çok 1,065. Hepsi ≤ 1,1.
  `faces_skipped_hidden` `chart_ticks`'te (katman akışı örterken) 160, `idle`'da (ekran dışı
  yüzler) 108, yani > 0.
- `crates/qp-wasm/tests/freq_m14.rs`: aynı ekran karesindeki iki veri çerçevesi tek geçiş
  (`passes_per_data_frame` 0,5); veri olayı olmayan çerçeve boyama istemiyor; yalnız ekrandaki
  yüzler boyanıyor ve atlananlar sayılıyor (≥ 1); açık katmanda `faces_to_paint()` boş;
  özetteki kip (secure_context, webgpu, jit) doğru. verify 3. adımda geçti.
- Kod: `faces_to_paint` örtülüyken hepsini atlıyor. Katman kapanınca `force=false` ile
  boyanıyor, anahtar değiştiyse yeniden boyanıyor. Görünür olan yüz IntersectionObserver'da
  `force=true` ile boyanıyor. Yolların kendi yeniden boyama istekleri bir sonraki geçişe
  erteleniyor (`repaintWanted`). Kare başına tek geçiş yapı gereği sağlanıyor.

## g. https-host

- `cargo test -p qp-cli --test demo_security` (`rp3/https-host`): 2/2 ok. Bunlar
  `demo_rejects_foreign_hosts_origins_and_secrets` (eski, değişmedi) ve
  `https_host_is_accepted_only_when_named`.
- Gate farkını okudum. `https_hosts` yalnız `--https-host` verilince doluyor (`AD` ve `AD:443`;
  kırpılıp küçük harfe çevriliyor). Bayrak yokken `host_ok` ve `origin_ok` eskisiyle aynı
  çalışıyor, genişleme yok; test bayraksız durumda vekil adını ve `https://127.0.0.1:P`
  kökenini 403 ile sınıyor. Bayrak varken bu Host için yalnız `https://AD` kökeni kabul
  ediliyor. `http://AD` 403, `https://evil.example` 403, yanlış belirteç 403, yabancı Host 403.
  Yerel köken eskisi gibi çalışıyor. PROTOCOL §21.4 notu kodla uyumlu.
- Küçük not (engel değil): `--https-host ""` verilirse `https_hosts` boş kalıyor ama
  `urls`'e `https:///` basılıyor (yalnız görüntü; güvenliği etkilemiyor).
- Birlikte birleşme: `git merge-tree --write-tree rp3/s-paint rp3/https-host` çakışmasız
  (ağaç 51a71cc). Bu ağaçtan geçici bir commit nesnesi oluşturup (ref yok) scratch'te açtım.
  `cargo fmt --check` temiz, `cargo clippy -p qp-cli --all-targets -D warnings` temiz,
  `demo_security` 2/2.

## Öneriler (kapsam dışı)

- ARCH.md: gpu yolundaki WGSL gölgelendiricinin (doku örnekleme × renk) "alan mantığı değil"
  olduğunu bir satırla anmak; arch-check JS dışını taramıyor.
- `--https-host` için boş ad reddedilmeli (görüntü düzeltmesi).
- p95 ölçütleri için koşu sayısını artırmak (3 koşuda tek bir uç değer ortancayı oynatıyor).
