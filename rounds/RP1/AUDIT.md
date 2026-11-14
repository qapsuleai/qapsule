# RP1 — Bağımsız denetim

Denetçi: ayrı alt ajan (işi yapmadı). Denetlenen commit: `4628343` (dal `rp1/integrate`).
Çalışma yeri: temiz klon (scratchpad). Kod düzeltilmedi. Klona eklenen deneme testi
commit edilmedi ve sonra silindi.

Makine: Apple M4 Max, macOS. Ölçüm sırasında yük ortalaması **7,8–9,0** (`uptime`).
RESEARCH.md ≈ 4 diyor; benim ölçümlerim daha yüklü makinede yapıldı.

## 1. VERIFY 1 — `QP_VERIFY_BROWSER=1 scripts/verify.sh`

Çıkış kodu **0**. 11 adımın hepsi PASS: fmt, clippy, test, schema, fixtures, golden,
wasm, hygiene, browser (headless Chromium), **compute**, **digest**.
`digest` adımı: arm64, x86_64 (Rosetta), wasm32 (Node 24.3.0) = `7b1e99d40889f83c`. ✅

## 2. VERIFY 2 — hesap özeti

- `fixtures/compute/digest.txt` = `7b1e99d40889f83c` (R1 değeri). Üç hedefte de aynı çıktı.
- `git diff f5c7d2c 4628343 -- fixtures/` boş. ✅

## 3. VERIFY 3 — dört profil yeniden ölçüldü

Komutlar görevde verilen biçimde çalıştı (yerel 11 koşu, wasm32-wasip1 Node 24.3.0 11 koşu).
"Sonra" = RESEARCH.md bölüm 4. Fark = benim / RESEARCH − 1.

| Profil / hesap | M4 RESEARCH | M4 denetim | fark | WASM RESEARCH | WASM denetim | fark |
|---|---:|---:|---:|---:|---:|---:|
| A sma | 4,34 | 4,49 | +3% | 10,89 | 11,87 | +9% |
| A ema | 3,80 | 4,02 | +6% | 10,43 | 11,08 | +6% |
| A rsi | 8,97 | 9,47 | +6% | 18,03 | 19,21 | +7% |
| A macd | 7,50 | 8,42 | +12% | 9,54 | 10,42 | +9% |
| A bollinger | 8,56 | 9,20 | +7% | 11,08 | 11,39 | +3% |
| B sma | 6,28 | 6,46 | +3% | 11,39 | 12,70 | +12% |
| B ema | 5,40 | 5,84 | +8% | 12,26 | 13,54 | +10% |
| B rsi | 18,52 | 21,04 | +14% | 31,98 | 33,27 | +4% |
| B macd | 12,41 | 12,93 | +4% | 18,31 | 19,31 | +5% |
| B bollinger | 32,37 | 33,92 | +5% | 40,92 | 42,34 | +3% |
| C ema>rsi | 11,56 | 11,77 | +2% | 21,43 | 21,74 | +1% |
| C rsi>bollinger | 14,35 | 15,11 | +5% | 17,48 | 18,21 | +4% |
| C sma>ema>rsi | 16,02 | 17,81 | +11% | 30,82 | 32,71 | +6% |
| D tik µs | 1,75 | 1,79 | +2% | 4,17 | 4,25 | +2% |
| D yeni bar µs | 2,67 | 2,83 | +6% | 6,00 | 6,17 | +3% |
| D attach (extend) ms | 13,78 | 14,64 | +6% | 18,11 | 20,54 | +13% |

Hepsi ±%20 içinde. Hepsi biraz yavaş; yük iki kat olduğu için bu beklenir. ✅

Özetler: yerel ve wasm çıktısında yazılan **her özet** BASELINE.md'deki özetle aynı
(A: `d9a393c2…`, `f9be271e…`, `9515526d…`, `d5a6c407…`, `7e683c08…`; B: `4deca6b3…`,
`858efc3a…`, `23d1bb67…`, `25dca317…`, `2aee14ef…`; C: `d60c4040…`, `51d10314…`,
`8c536220…`; D: `4db86a72…`). ✅

Ayrıca `rounds/RP1/raw/final-*.txt` ham dosyalarını RESEARCH tablolarıyla karşılaştırdım:
M4, Rosetta ve WASM sayıları birebir aynı.

Raporlanmamış iki gerileme gördüm (yalnızca bilgi, hedef dışı yollar):
- WASM'da D "attach (push)" yolu: taban 51,11 ms → final 60,28 ms (benim ölçüm 62,19).
  ≈ +%18–22. Yerelde aynı yol değişmedi (26,28 → 26,68). RESEARCH yalnızca `extend`
  satırını veriyor.
- Rosetta x86_64'te B Bollinger: 61,93 → 123,65 ms (2 kat yavaş). CI'da (gerçek x86)
  aynı satır 113,99 → 66,51 iyileşmiş; büyük olasılıkla Rosetta etkisi. Yine de raporda yok.

## 4. VERIFY 4 — CI ölçümü

`scripts/qgh run view` ile günlükler okundu.

- Çalışma 43625622117: `headSha` = `0cd37add…`, dal `rp1/integrate`, 5 deneme, hepsi
  `success`, hepsinde `verify: OK` ve digest `x86_64, wasm32: 7b1e99d40889f83c`.
- Makine satırları: deneme 1 ve 3 **AMD EPYC 7763**, deneme 2, 4, 5 **AMD EPYC 9V74**.
  RESEARCH ile aynı.
- Taban çalışması 43618730149: `headSha` = `caab6e3…` (dal `rp1/t1-baseline`), 9V74,
  sayılar BASELINE.md ile aynı (A: 17,44 / 21,59 / 62,87 / 69,07 / 71,98; WASM 26,44 /
  37,63 / 118,46 / 159,91 / 120,71).
- `git diff --stat 0cd37ad 4628343`: yalnızca `rounds/RP1/RESEARCH.md` (+243). Ölçülen
  commit ile denetlenen commit arasında kod farkı yok.

K17 tablosu (bölüm 5) günlüklerle karşılaştırıldı. "En kötü" değerler doğru:

| Hesap | Yerel en kötü (günlük) | WASM en kötü (günlük) |
|---|---|---|
| SMA | 15,03 (d1, 7763) | 21,86 (d1) |
| EMA | 5,29 (d1) | 25,70 (d1) |
| RSI | 42,15 (d1) | 57,34 (d1) |
| MACD | 13,20 (d1) | 29,24 (d1) |
| Bollinger | 14,27 (d3) | 27,21 (d1) |

Hepsi < 50 / < 150. ✅

Bölüm 4'teki "CI 9V74 sonra" sütunu üç 9V74 denemesinin medyanı; A, B, C ve WASM A
satırlarını tek tek kontrol ettim, hepsi doğru. 7763 sütunu da doğru.

Küçük hatalar:
- **D satırında CI "sonra" değerleri medyan değil, en küçük değer.** Yeni bar: raporda
  5,18 (9V74) ve 7,07 / 7,15 (7763); günlükteki medyanlar 5,38 ve 7,49 / 7,45. Attach
  (extend): raporda 17,25 ve 20,07 / 19,77; medyanlar 17,33 ve 20,11 / 19,81. Tik satırı
  doğru (medyan). Sonuç değişmiyor; "yeni bar 5,18 → 5,18" aslında 5,18 → 5,38 (+%4).
- "Çalışmalar arası yayılım 9V74'te %2'nin altında" cümlesi genel olarak doğru değil:
  B Bollinger 60,79 / 67,46 / 66,51 (≈ %11). A satırları için doğru.
- Taban CI ölçümü tek çalışma (n = 1); final 5 çalışma. TASKS "en az beş çalıştırma"
  diyor; taban için uygulanmamış.

## 5. VERIFY 5 — yaklaşım kaydı

RESEARCH.md bölüm 3: 21 yaklaşım, 8 yön. Her satırda hipotez, ölçüm (ya da gerekçe) ve
karar var. Ölçümle desteklenen farklı yön sayısı 7 (paylaşım, bellek, SIMD/şerit, iş
parçacığı, artımlı/geç başlangıç, parametre uzayı, içerik adresli önbellek); GPU yalnızca
analiz, #11 denenmedi (gerekçe ölçüm). ≥ 6 koşulu sağlanıyor. ✅

Deney notlarının hepsinde "Hipotez(ler) (denemeden önce yazıldı)" başlığı var (E1: "written
before implementing"). Ancak git geçmişi bunu kanıtlamıyor: her not dosyası sonuçlarla
birlikte tek commit'te geliyor (E2 hariç iki commit; ikisi de sonuçlu). "Önce yazıldı"
beyanı doğrulanamıyor. Literatür bölümü de "modelin bilgisinden, ağdan doğrulanmadı"
diye açıkça işaretli.

## 6. VERIFY 6 — değişen dosyalar

`git diff --stat f5c7d2c 4628343`: 48 dosya; yalnızca `crates/qp-compute/**` ve
`rounds/RP1/**`. Başka crate, `fixtures/`, `.github/`, `scripts/`, `Cargo.lock`, durum
dosyası değişmemiş. ✅

R1 testleri `crates/qp-compute/tests/{hand,legacy,modes,chain,sweep}.rs` f5c7d2c'ye göre
**değişmemiş** (diff 0 satır). ✅

## 7. Kural kontrolleri

- `unsafe`: `src/` içinde yok; çalışma alanında `unsafe_code = "forbid"` duruyor. ✅
- `mul_add` / FMA: `src/` içinde yalnızca yorumlarda geçiyor; kodda yok. ✅
- Saat / rastgelelik: `Instant` yalnızca `benches/driver/mod.rs` içinde. `src/` içinde
  saat, `rand`, `HashMap`/`RandomState` yok. `bench_closes` tamsayı LCG (deterministik).
  `par.rs` iş parçacığı kullanıyor ama sonuçları sırayla topluyor; sonuç iş parçacığı
  sayısından bağımsız (test `par.rs` geçiyor). ✅
- `NORTH_STAR.md` değişmemiş. ✅
- Gizli bilgi: diff'te token / anahtar / parola deseni yok. ✅ (Not: `raw/base-wasm.txt`
  içinde yerel scratchpad yolu yazıyor; zararsız.)
- `Cargo.lock` değişmemiş. `Cargo.toml` değişikliği: `proptest` için `default-features =
  false` (`fork` kapandı; wasip1 derlemesi için, BASELINE'da gerekçeli) ve `[[bench]]
  profiles`. Yeni bağımlılık yok. ✅
- `DECISIONS.md` güncellenmemiş. CLAUDE.md "mimari seçimde DECISIONS.md'ye satır ekle"
  diyor. RP1 yeni genel API'ler (`late::*`, `par::sweep_reduce_par`, `graph`,
  `ResultCache`, `Study::extend`) ve hedefe göre şerit sayısı gibi mimari seçimler ekledi.
  Satır yok. ⚠️

## 8. Bit-eşitlik

- `cargo test -p qp-compute` (aarch64, debug): tüm test dosyaları geçti.
- `cargo test -p qp-compute --release --target x86_64-apple-darwin` (Rosetta): tüm test
  dosyaları geçti.
- CI'da (gerçek Linux x86_64) `verify` adımı testleri koşuyor; 5 denemede de OK.

### Kod okuması

**`lanes.rs`.** Her çekirdek ısınmayı skaler `peek1`/`commit1` ile koşuyor, sonra kararlı
durumun işlemlerini aynı sırayla uyguluyor. Satır satır karşılaştırdım:
- EMA, MACD: `x*a + prev*(1-a)` aynı. MACD başlangıcı `slow+signal-1`; `fast < slow`
  `Macd::new` tarafından zorunlu, bu yüzden fast EMA'nın tohumlanmamış olma durumu yok.
- SMA: `s = sum + x; v = s/pf; sum = s - x[i-(p-1)]` = `step_steady` ile aynı.
- RSI: `split` ve `(avg*pm1 + g)/pf` aynı. `rsi_from_avg` seçimli biçim eski dallı biçimle
  NaN dahil eşdeğer (`avg_loss == 0` → düz değer; değilse formül; NaN'da da aynı dal).
- Bollinger: `add`, `track_run`, `step_steady`, `value` ve M2 çıkarma aynı sırada. `sqrt`
  ve bölme önce hesaplanıp seçiliyor; seçim bitleri kopyaladığı için sonuç aynı. `run`
  sayacının `f64` olması 2^53'e kadar tam. Period 1'de M2'nin NaN olması zararsız (bir
  sonraki adımda düz dal seçiliyor).

**`sweep/share.rs`.** Paylaşım anahtarı parametre bitleri; eşit anahtar ⇒ eşit periyot,
yani yanlış paylaşım yok. MACD çizgisi (önbellekli EMA'lar ya da `macd_line`) ve sinyal
şeritleri tek çalıştırmanın işlemleri. Bollinger `step_std` + `bands`, `peek1`+`commit1`
ile aynı işlemler (`std_of` = `value`'daki std). RSI kazanç/kayıp önceden `split` ile
bölünüyor; `step_split` ve `rsi_from_avg_pass` eski yolla aynı. E6'nın "sıfırlamama"
değişikliğinde her sütunun her hücresinin yazıldığını kontrol ettim (`[..w]` NaN,
`[w..]` hesap).

**`late.rs` (sandviç kanıtı).**
- Monotonluk: adım `fl(fl(x·a) + fl(s·b))`, `b = fl(1-a) ≥ 0`. Çarpma ve toplama
  yuvarlamada monoton; bileşimi de. RSI'de `fl(fl(s·(p-1)) + g)/p` aynı. Doğru.
- ±0: `L` ile `U` bit eşit `+0` iken gerçek durum `-0` olabilir mi diye baktım. İşlemler
  `total_cmp` sırasında (−0 < +0) da monoton; başlangıçta `L < T < U` kesin (sınırlar
  `MIN_POSITIVE` kadar genişletiliyor). Bu yüzden bit eşitlik gerçek durumu da belirliyor.
  Kanıt bu açıdan sağlam.
- Sınırlar: EMA için `[min, max]` göreli 1e-6 genişletilmiş. Bu aralık adım altında
  korunuyor, şart `1e-6 · a > ~3u`, yani periyot ≲ 3e9. Yorum "≤ 1e9" diyor; genel
  `ema_tail` bunu doğrulamıyor (pratikte erişilemez, bkz. bulgular). RSI'de alt sınır 0
  tam, üst sınır en büyük kazanç/kayıp genişletilmiş. Tohum (SMA ortalaması) hatası
  için ayrıca `p² · u < 1` gerekiyor; `p < 1e8` için sağlanıyor.
- Sonlu olmayan girdi: geçmişte NaN/inf varsa (`finite = false`) doğrudan tam koşuya
  düşüyor. Geçmişteki farklar taşarsa (inf) sınır inf oluyor ve hiç eşitlenmiyor → tam
  koşu. `from` sonrasındaki NaN/inf aynı ham işlemlerden geçiyor.
- API riski: `ema_tail_with` / `rsi_tail_with` başka bir seriden üretilmiş `Bounds`
  kabul ediyor. O durumda "her zaman doğru" garantisi bozulur (kullanım hatası; belgede
  "precomputed bounds" diyor ama seriyle bağ yok).

### Kırma denemesi (commit edilmedi)

Klona `tests/audit_adv.rs` ekledim. Seriler: rastgele yürüyüş, hep `-0.0`, `±0`
karışık, negatif alt-normal sayılar, `±1e300`, `±0.9·f64::MAX`, ortada NaN, ortada
`+inf`, sonda NaN, `1e15` sonra `1e-300` mertebesi, negatif fiyatlar; ayrıca
`-1e-310` sonra uzun `-0.0` kuyruğu (gerçek EMA durumunu `-0`'a sürmek için) ve
yürüyüşten sonra `±0` kuyruğu.

- `sweep_each_lanes` (şerit 1/2/4/8/varsayılan), sırasız ve tekrarlı ızgaralar
  (period 1 dahil; MACD paylaşımlı ve paylaşımsız; Bollinger period 1–20, k 0,1–3,5),
  ayrıca `sweep_series_each` (B yolu): tek çalıştırmayla **ham bit karşılaştırması
  (NaN bitleri dahil) — 0 fark**.
- `ema_tail` / `rsi_tail`, periyot 1–30, `from` 0…n: 840 durum, 356'sı kanıtlandı,
  geri kalanı tam koşuya düştü; **0 fark**.
- aarch64 debug ve x86_64 release (Rosetta) iki hedefte de geçti.

Kıramadım.

Kapsam notu: `cross_target_digest` yalnızca `sweep_each` (sıralı ızgara) ve akış yolunu
özetliyor. Yeni `late`, `par`, `graph`, `extend` yolları wasm32'de test edilmiyor (B, C, D
profil özetleri wasm benchmark çıktısında tabanla aynı, bu dolaylı kanıt). `late` için
wasm32 kanıtı yok.

## 9. GOAL kabul ölçütleri

| # | Durum | Kanıt |
|---|---|---|
| 1 | ✅ | RESEARCH bölüm 3: 21 yaklaşım, 8 yön (7'si ölçümlü), işe yaramayanlar dahil; her biri hipotez + ölçüm + karar. |
| 2 | ✅ | CI 43625622117 deneme 1–5 (`0cd37ad`): en kötü yerel RSI 42,15 < 50, en kötü WASM RSI 57,34 < 150; beş hesap da tutuyor, iki CPU tipinde. |
| 3 | ✅ | Bölüm 4: A–D için M4, CI (9V74 önce/sonra + 7763), WASM tabloları; yeniden ölçüm ±%20 içinde. (D satırında CI değerleri medyan değil en küçük — küçük hata.) |
| 4 | ✅ | R1 testleri değişmemiş ve geçiyor; digest `7b1e99d40889f83c` aarch64, Rosetta, wasm32, CI Linux ×5; profil özetleri tabanla aynı. |
| 5 | ✅ | Bölüm 6: FMA, karşılıklı çarpma, önek toplamı, f32, tamsayı kuruş, GPU, `unsafe` AVX2; bedel ve karar ana akışa bırakılmış; hiçbiri çekirdekte değil. |
| 6 | ✅ | Bölüm 7: en yavaş = x86 RSI (bölme çıkışı), çıktı yazımı, B Bollinger, uzun geçmiş, WASM'da iş parçacığı yok; her biri için gereken mimari değişiklik yazılı. |
| 7 | ⏳ | `REPORT.md`, `STATUS.md`, `ROADMAP.md` henüz yok/güncellenmemiş (ana akış denetimden sonra yazacak). `AUDIT.md` bu dosya. |

## Bulgular

### Yüksek
- Yok.

### Orta
- **M1.** `DECISIONS.md` güncellenmemiş. RP1 yeni genel API'ler (`late`, `par`, `graph`,
  `ResultCache`, `Study::extend`) ve mimari seçimler (hedefe göre şerit sayısı, paylaşım
  yolu, E2.2 `unsafe` kararı ana akışa) ekledi. CLAUDE.md'ye göre satır gerekir.

### Düşük
- **L1.** RESEARCH bölüm 4, D satırı: CI "sonra" değerleri (yeni bar, attach) medyan değil
  en küçük. Doğru medyanlar: yeni bar 5,38 (9V74), 7,49 / 7,45 (7763); attach 17,33,
  20,11 / 19,81.
- **L2.** "9V74 çalışmaları arası yayılım %2'nin altında" genel değil; B Bollinger ≈ %11.
- **L3.** Raporlanmamış gerilemeler: WASM D attach (push) 51,1 → 60,3 ms (+%18; benim
  ölçüm 62,2); Rosetta B Bollinger 61,9 → 123,7 ms. Hedef yolları değil, ama yazılmalı.
- **L4.** "Hipotez denemeden önce yazıldı" beyanı git geçmişiyle doğrulanamıyor (not ve
  sonuç aynı commit'te).
- **L5.** `late::*_tail_with` başka seriden `Bounds` kabul ediyor; o durumda doğruluk
  garantisi yok. `Bounds` seriye bağlanmalı ya da belgede açıkça yazmalı.
- **L6.** `late.rs` genişletme payı periyot ≲ 3e9 (ve tohum için ≲ 1e8) varsayıyor;
  genel `ema_tail`/`rsi_tail` periyodu doğrulamıyor. Pratikte erişilemez (eşitlenme
  ~19·p bar ister), ama varsayım kodda zorlanmıyor.
- **L7.** `late`, `par`, `graph`, `extend` wasm32'de test edilmiyor; cross-target digest
  bunları kapsamıyor.
- **L8.** Taban CI ölçümü tek çalışma; final beş çalışma.
- **L9.** Denetim ölçümleri yük ≈ 8 altında yapıldı (RESEARCH ≈ 4); sayılar yine ±%20 içinde.

## Karar

**Kabul.** VERIFY 1–6 geçti, bit-eşitlik korunuyor ve kırılamadı, K17 hedefleri CI'da beş
hesapta tutuyor; yalnızca belge düzeyinde küçük düzeltmeler ve `DECISIONS.md` satırları
gerekli. Ölçüt 7 ana akışın REPORT/STATUS/ROADMAP yazmasını bekliyor.
