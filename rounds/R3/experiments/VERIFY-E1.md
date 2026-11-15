# R3 — Evre 1 bağımsız denetimi (VERIFY-E1)

Denetçi: bağımsız alt ajan (işi yapan değil). Dal `r3/verify-e1`, taban `r3/int` @ `aea776c`.
Kod düzeltilmedi; yalnız bu dosya ve `rounds/R3/experiments/verify-e1/` (denetçi betikleri) yazıldı.
`.env.local` açılmadı. Gerçek veri depoya kopyalanmadı (geçici dosyalar depo dışında, iş bitince silindi).

**Karar: Evre 1 kabul edilebilir.** Dokuz kontrolün dokuzu PASS; engelleyici bulgu yok. Düşük önemli
üç bulgu (N1–N3) ve beş bilgi notu (N4–N8) aşağıda.

Okunanlar: `CLAUDE.md`, `rounds/R3/GOAL.md`, `rounds/R3/VERIFY.md`, `docs/CAPSULE.md` §9,
`docs/CONCEPTS.md` "Kapsül (R3)", `experiments/T2.md`, `T3.md`, `T45.md`, `docs/DATA_ACCESS.md`.

## Düzenek

`rounds/R3/experiments/verify-e1/` — çalışma alanı dışı ayrı bir Cargo paketi (`[workspace]` kendi
kökü; `verify.sh`'ı etkilemez) ve iki Python betiği:

| Dosya | Ne yapar |
|---|---|
| `src/lib.rs` | `synthetic_market` + `synthetic_flow`; akışı denetçi bozar (her 17. barda nokta boş, her 29. barda `net` yok) |
| `src/bin/export.rs` | barlar, akış (`net`) ve `qp-facts` çıktısını JSON'a döker |
| `recompute.py` | yedi dedektörün **bağımsız** yeniden hesabı (T3 tablosundan; `qp-facts` çağrılmaz), iki aritmetik kipi: `naive` (denetçinin kendi aritmetiği: pencere toplamı her bar sıfırdan, std iki geçişli) ve `engine` (T3 "Denetçiye not"undaki işlem sırası) |
| `src/bin/stream.rs` | toplu = canlı düzeltmeli akış, kendi verisi ve kendi gürültüsüyle |
| `src/bin/capsule.rs` | ileriye bakma (4 karıştırma × 4 kesim × 2 yapılandırma), aday saflığı, geri çekme |
| `src/bin/realdata.rs` | gerçek veride toplu tetik sayıları (yalnız toplam) |
| `src/bin/export_local.rs` | gerçek veriyi **depo dışı** geçici dosyaya döker (yalnız `recompute.py` için; `/rounds/` içine yazmayı reddeder) |
| `window_start.py` | sonda: `ma_stack` kararı serinin başladığı bara duyarlı mı |

Çalıştırma: `CARGO_TARGET_DIR=<geçici> cargo run --release --manifest-path rounds/R3/experiments/verify-e1/Cargo.toml --bin <ad>`;
`python3 -I rounds/R3/experiments/verify-e1/recompute.py <json> engine|naive`.
Sentetik JSON'lar belirleyicidir, commit edilmedi (`export <çıktı> 4242 4 1500` ve `export <çıktı> 99 8 1200`).

## Sonuç tablosu

| # | Adım | Sonuç | Kanıt |
|---|---|---|---|
| 1a | `cargo fmt --all -- --check` | PASS | çıkış 0 |
| 1b | `cargo clippy --workspace --all-targets -- -D warnings` | PASS | çıkış 0 |
| 1c | `cargo test --workspace` | PASS | 85 test ikilisi, 388 geçti, 0 kaldı, 2 yok sayıldı |
| 1d | `QP_VERIFY_BROWSER=0 scripts/verify.sh` | PASS | çıkış 0, `verify: OK`; 14 PASS, 1 SKIP (browser, istenen); ayrıntı §1 |
| 1e | Hesap özeti | PASS | `cargo run --release -p qp-compute --example digest` → `7b1e99d40889f83c`; `-- r3` → `914fcfdb9f653a6d` |
| 2 | VERIFY 6 ön kontrolü: 7 dedektör bağımsız hesap | PASS (not: N2, N3) | aşağıda §2 |
| 3 | VERIFY 5 ön kontrolü: toplu = akışkan | PASS | testler `to_bits` ile karşılaştırıyor; kendi kontrolüm 16 723 olgu, 0 fark |
| 4 | İleriye bakma | PASS (not: N4) | 32 karşılaştırma, 0 fark; sonuç kuralında 68 655 çağrıda 0 sızıntı |
| 5 | Aday saflığı (VERIFY 9 ön kontrolü) | PASS | 4 896 `candidates` çağrısı, 8 613 aday; anlık görüntü 245/245 eşit; adaylı tam koşu = adaysız koşu (bayt) |
| 6 | Geri çekme karnede yok; önceki doğumların karnesi değişmez | PASS (not: N5) | aşağıda §6 |
| 7 | Hijyen | PASS (not: N7) | `git ls-files \| grep fixtures/local` boş; belirteç taraması boş; betik incelendi |
| 8 | Gerçek veri, yalnız toplu | PASS | aşağıda §8 |
| 9 | Kavram koddan önce | PASS (not: N1, N8) | `0ad3a4e` (sözlük) hem `924a804` (motor) hem `0ea493d` (dedektörler) için ata |

## 1. Derleme ve testler

`cargo fmt`, `cargo clippy … -D warnings`, `cargo test --workspace` ayrı ayrı: üçü de çıkış 0
(test: 85 test ikilisi, 388 geçti, 0 kaldı, 2 yok sayıldı).

`QP_VERIFY_BROWSER=0 scripts/verify.sh` → çıkış 0, `verify: OK` (247 s):

| # | Adım | Sonuç | Not |
|---|---|---|---|
| 1 | fmt | PASS | |
| 2 | clippy | PASS | |
| 3 | test | PASS | |
| 4 | schema | PASS | 28 şema, belirleyici |
| 5 | fixtures | PASS | 4 örnek geçerli |
| 6 | golden | PASS | altın görselle bayt eşit |
| 7 | views | PASS | 8 görünüm ×2, altınlarla bayt eşit |
| 8 | wasm | PASS | ham 1 889 334 B, gzip -9 598 451 B |
| 9 | hygiene | PASS | |
| 10 | browser | SKIP | `QP_VERIFY_BROWSER=0` (istenen) |
| 11 | compute | PASS | |
| 12 | digest | PASS | arm64 ve x86_64 (Rosetta): `7b1e99d40889f83c`; wasm32 atlandı (tarayıcı kapalı) |
| 13 | protocol | PASS | 61 örnek |
| 14 | conform | PASS | 36 döküm × inproc, serve, mcp |
| 15 | replica | PASS | yerel otorite = yerel yansı (lag 0, 25); wasm32 atlandı |

Özetler elle: `cargo run --release -p qp-compute --example digest` → `7b1e99d40889f83c` (değişmedi);
`… --example digest -- r3` → `914fcfdb9f653a6d` (T3'teki değerle aynı). wasm32 özetleri bu koşuda
ölçülmedi (tarayıcı adımı kapalı).

## 2. Dedektörlerin bağımsız hesabı (VERIFY 6 ön kontrolü)

Karşılaştırma anahtarı `(bar, kod)`; yön ve kenar ayrıca, kanıtta her anahtar, güç. Üç veri kümesi:
sentetik tohum 4242 (4 enstrüman × 1500 gün), sentetik tohum 99 (8 × 1200), gerçek veri (6 sembol,
günlük ve saatlik; yalnız toplu sonuç).

**`engine` kipi** (motorun işlem sırası): yedi dedektörün tamamında **karar farkı 0**; altısında
kanıt ve güç **bit düzeyinde aynı**. `squeeze.bandwidth`'te kararlar aynı, değerlerde küçük fark:

| Veri | Dedektör | Motor | Denetçi | Karar farkı | Kanıt maks. mutlak / göreli fark | Bit eşit |
|---|---|---|---|---|---|---|
| s4242 | `continuation.ma_stack` | 749 | 749 | 0 | 0 | evet |
| s4242 | `reversal.rsi_divergence` | 121 | 121 | 0 | 0 | evet |
| s4242 | `breakout.range` | 862 | 862 | 0 | 0 | evet |
| s4242 | `squeeze.bandwidth` | 67 | 67 | 0 | 2,2e-11 / 6,2e-11 | hayır |
| s4242 | `extreme.rsi` | 342 | 342 | 0 | 0 | evet |
| s4242 | `flow.volume_surge` | 163 | 163 | 0 | 0 | evet |
| s4242 | `flow.institutional` | 974 | 974 | 0 | 0 | evet |
| s99 | `continuation.ma_stack` | 1101 | 1101 | 0 | 0 | evet |
| s99 | `reversal.rsi_divergence` | 225 | 225 | 0 | 0 | evet |
| s99 | `breakout.range` | 1462 | 1462 | 0 | 0 | evet |
| s99 | `squeeze.bandwidth` | 114 | 114 | 0 | 2,3e-11 / 1,2e-10 | hayır |
| s99 | `extreme.rsi` | 580 | 580 | 0 | 0 | evet |
| s99 | `flow.volume_surge` | 267 | 267 | 0 | 0 | evet |
| s99 | `flow.institutional` | 1594 | 1594 | 0 | 0 | evet |
| gerçek 1day | 7 dedektör | — | — | 0 | yalnız squeeze: 2,4e-10 / 1,1e-11 | squeeze hariç evet |
| gerçek 1hour | 7 dedektör | — | — | 0 | yalnız squeeze: 3,7e-9 / 7,5e-11 | squeeze hariç evet |

`squeeze` farkının kaynağı: benim Bollinger'im pencereyi her bar sıfırdan dolaşıyor (iki geçiş ya da
taze Welford); motor kayan pencereli Welford kullanıyor. Kesin (rasyonel) hesapla karşılaştırınca
motorun `bw` göreli hatası en çok **4,8e-11**, iki geçişli hesabınki **5,0e-16** (s4242). Bkz. N2.

**`naive` kipi** (denetçinin kendi aritmetiği): yalnız `ma_stack`'te kenar barı kayıyor —
s4242: 2 (aynı olay, bar 803 ↔ 804, bir enstrüman); s99: 4; gerçek 1day: 4; gerçek 1hour: 6. Hepsi
"kapanış = SMA20" eşitlik durumu: örnek (s4242, SYN04, bar 803) kesin hesapla `c − sma20 = −1,8e-15`,
yani ondalıkta tam eşitlik; motorun kayan toplamı (`sma20 = 52.76000000000005`) kesin sonuçla aynı
tarafa, benim saf toplamım öbür tarafa düşüyor. Değer farkları ≤ 1e-14 göreli. Bkz. N3.

Isınma, kenar sırası, yön, kanıt anahtarları (fazla/eksik anahtar yok), `flow.institutional`'ın
eksik gözlemi atlaması (pencere eski 5 **gözlem**, şimdiki hariç) T3 tablosuyla birebir.

## 3. Toplu = akışkan (VERIFY 5 ön kontrolü)

- `crates/qp-facts/tests/detectors.rs::batch_equals_streaming_equals_revised`: 3 tohum × 2000 bar × 7
  dedektör; `sig()` her kanıt değerini ve gücü `to_bits()` ile, artı kod/tür/yön/zaman/kenar/geometri/
  koşulu karşılaştırıyor — gerçekten bit düzeyinde. Not: `run_batch` tanım gereği `push` döngüsü
  olduğundan "toplu = saf akış" kendiliğinden doğru; anlamlı olan "düzeltmeli" kolu (0–3 gürültülü
  `update_last` + son hal), o da var.
- Kendi kontrolüm (`stream.rs`): rejimli sentetik piyasa (testlerdeki `test_bars` değil), tohum 77 ve
  2027, 10 enstrüman × 1500 gün, yedi dedektör. Her bar önce uç değerli (±%20–25 kapanış, 0–8× hacim,
  akış silinmiş ya da ×±20) bir sürümle `push`, 0–6 gürültülü `update_last`, son hal; %20 olasılıkla
  son halden sonra bir gürültü daha ve son hal yeniden. Sonuç: **16 723 olgu karşılaştırıldı, 276 573
  ara (aday) tespit görüldü, uyuşmayan (enstrüman, dedektör) çifti 0**.

## 4. İleriye bakma

`crates/qp-capsule/tests/pipeline.rs::look_ahead_shuffling_the_future_does_not_change_the_past`:
4 kesim × 3 tohum; `t ≥ T` barlarını başka bir yürüyüşle değiştiriyor, geleceğin gerçekten değiştiğini
doğruluyor (`assert_ne!` olaylar) ve `t < T` olaylarını, `T` öncesi doğumların `ledger_at_birth`'ini,
her anahtar için `ledger_stat(key, T)`'yi ve `T` öncesi karne satırlarını JSON bayt eşitliğiyle
karşılaştırıyor. Test sonuç kuralı `bars.len() == now + 1` değilse panikliyor. Gerçekten geleceği
bozup geçmişi karşılaştırıyor. Eksik: sahte dedektörlerle çalışıyor, gerçek `qp-facts` ile değil (N4).

Kendi kontrolüm (`capsule.rs`, gerçek yedi dedektör, 5 enstrüman × 700 gün, akış dahil):

- Yapılandırmalar: (a) `scaffold::config(10)`; (b) denetçinin kuralları — `KindTheme` + **kendi sonuç
  kuralım** (doğum kapanışına göre ±%3 → gerçekleşti/geçersiz, ufuk 10; `bars.len() != now+1` sayılır)
  + **kendi önem kuralım** (karneye ve sayaçlara duyarlı, karnede sızıntı olaylara yansısın diye).
  Not: iskeletin `FixedHorizon`'ı hiç karar vermez (karne 741 satır, 0 gerçekleşen) — bu yüzden (b).
  (b)'de karne 1119 satır, 532 gerçekleşen; tüm aşamalar görülüyor.
- Karıştırmalar: `truncate` (T sonrası hiç yok), `extreme_bars` (T sonrası barlar ×3/÷2,5 sıçramalı,
  hacim ×1000), `future_flow` (yalnız T sonrası akış ×−100·u), `truncate+append` (kesip 60 uç bar ve
  ±5e11 `net` ekle). Kesimler bar 150, 333, 520, 690.
- Karşılaştırılan: `t < T` olguları, `t < T` olayları, `T` öncesi doğumların `ledger_at_birth`'i,
  `recorded_at < T` karne satırları, her anahtar için `ledger_stat(key, T)` (n>0 olanlar).
- Sonuç: **2 yapılandırma × 4 kesim × 4 karıştırma = 32 karşılaştırmanın 32'si aynı**; her
  karıştırmanın geleceği gerçekten değiştirdiği doğrulandı (hiçbiri etkisiz değil). Sonuç kuralı
  68 655 kez çağrıldı, **0 sızıntı** (`bars.len() == now + 1` hep doğru).
- İlk koşuda `truncate` bir "fark" gösterdi; sebep benim düzeneğimdi (yalnız gelecekte görülen bir
  karne anahtarı `n = 0` ile listeye giriyordu), motorla ilgisi yok; düzeltildi.

## 5. Aday saflığı

`Engine::candidates` `&self` alıyor; `qp-capsule` ve `qp-facts` kaynaklarında `Cell`/`RefCell`/
`Mutex`/`unsafe`/`static mut` yok — tip düzeyinde durumu değiştiremez. Test:
`lifecycle.rs::candidates_are_pure_and_never_in_ledger` (anlık görüntü JSON'u önce/sonra eşit).

Kendi kontrolüm (`capsule.rs` B): 4 enstrüman × 600 gün, gerçek dedektörler, denetçi kuralları. Her
barda 1–3 gürültülü açık bar sürümü (`push`/`update_last` → geçici olgu → `to_fact` → `candidates`),
sonra son hal `update_last` → `on_bar`. **4 896 çağrı, 8 613 aday hikâye** (would-be: doğdu 7469,
teyit 456, güçlendi 166, zayıfladı 522; hepsi `Stage::Candidate`). Her 20. çağrıda anlık görüntü
önce/sonra: **245/245 eşit**. Adaylı tam koşunun son anlık görüntüsü, adaysız `simulate_engine` ile
**bayt eşit**; karne 672 = 672 satır, olay 1836 = 1836.

## 6. Geri çekme

Testler: `retracted_open_story_never_in_ledger`, `retracting_a_closed_story_does_not_change_earlier_births`.

Kendi kontrolüm (`capsule.rs` C): 5 × 600, koşunun ortasında (global adım 1500) en eski
gerçekleşmiş ve geçersiz iki kapanmış hikâye ile iki açık hikâye geri çekildi.

- Geri çekme anında karne satır sayısı 417 → 417 (yeni satır yok); `Stage::Retracted` aşamalı satır **0**.
- Kapanmış iki hikâyenin var olan satırı silinmedi, `retracted_at = t` ile işaretlendi; `ledger()`
  görünümünde geri çekilen kapsüllerin satırı **0**. Açıkken geri çekilen iki hikâyenin koşu sonunda
  da hiç satırı yok.
- Aynı kapsülü ikinci kez geri çekmek reddedildi (`None`).
- Geri çekmeden önceki **423 doğumun** `ledger_at_birth`'i geri çekmesiz koşuyla **aynı**; ayrıca
  son karne kayıtlarından `ledger_stat(key, t_doğum)` ile yeniden hesaplanınca **0 uyuşmazlık**.
- Sonraki doğumlar geri çekilen satırı görmüyor (hizalanabilen 4 doğumun 2'sinde `n` daha düşük, hiçbirinde
  yüksek değil). Geçmişte 4 `Retracted` olayı var.

## 7. Hijyen

- `git ls-files | grep fixtures/local` → boş; `.gitignore`'da `fixtures/local/`.
- `git grep -nIE "jwt [A-Za-z0-9]|eyJ[A-Za-z0-9_-]{10,}"` → boş. Ek tarama (`MATRIKS_TOKEN=` değerli,
  `Bearer …`, `api_key=…`) yalnız `rounds/R3/PROMPT.md:7`'deki yer tutucuyu (`<belirteç>`) buldu.
- `scripts/fetch-bars.sh`: `set -euo pipefail` + açıkça `set +x`; belirteç hiçbir yerde yazdırılmıyor
  (yalnız "MATRIKS_TOKEN is not set" mesajı); `Authorization` başlığı bash yerleşiği `printf` ile
  `curl -K -`'ye stdin'den veriliyor → süreç argümanlarında yok. `-D` yalnız **yanıt** başlıklarını
  saklıyor; `--keep-raw` de yanıt başlığı/gövdesi kopyalıyor (belirteç içermez). Python yardımcısı
  belirteci kullanmıyor. Not N7.

## 8. Gerçek veri (yalnız toplu)

`QAPSULE_LOCAL_BARS=<ana çalışma ağacı>/fixtures/local/bars`, `load_local_bars` (düzeltilmiş), akışsız.

| Periyot | Sembol | Toplam bar | ma_stack | rsi_divergence | breakout | squeeze | extreme.rsi | volume_surge | institutional |
|---|---|---|---|---|---|---|---|---|---|
| 1day | 6 | 13 314 | 1459 | 245 | 1915 | 194 | 857 | 256 | 0 |
| 1hour | 6 | 23 898 | 2561 | 409 | 2694 | 344 | 1790 | 988 | 0 |

Altı türün altısı da gerçek veride tetikliyor (akış türü `volume_surge` ile). Her dedektör 6/6
sembolde ≥1 kez; tek istisna `volume_surge` günlükte 5/6 (bir sembolde 0). `institutional` gerçek
akış olmadığından 0 (beklenen). Aynı veride bağımsız hesap (§2): `engine` kipinde 0 karar farkı.

## 9. Kavram koddan önce

`git log`: `0ad3a4e` (R3/T1: sözlük "Kapsül (R3)", CAPSULE §9, şema ve sözleşme iskeletleri) →
`924a804` (T45 motor) → `0ea493d` (T3 dedektörler). `git merge-base --is-ancestor` her iki uygulama
için de doğru. Sözlük, uygulamadan önce; sözleşme iskeletiyle (trait'ler, şema tipleri) aynı commit'te (N8).

## Bulgular (önem sırasıyla)

Engelleyici (FAIL) bulgu yok.

| # | Önem | Bulgu | Öneri |
|---|---|---|---|
| N2 | Düşük | Bollinger bant genişliği (`qp-compute`'un var olan hesabı; kayan Welford) kesin hesaba göre **4,8e-11** göreli hataya kadar çıkıyor (gerçek saatlikte bağımsız hesapla 7,5e-11 fark); iki geçişli hesap 5e-16. T3'ün "~1e-13 göreli" notu Bollinger için iyimser. Testlerde `squeeze` kararı hiç değişmedi, ama eşiğe yakın bir karar ulp'tan çok daha geniş bir bantta değişebilir ve bağımsız bir uygulayıcı değerleri bit düzeyinde tutturamaz. Hesap özetiyle donmuş olduğu için değiştirmek kapsam dışı. | T3/CAPSULE notunu düzelt (≈1e-10); VERIFY 6'da squeeze için "değerler ≤1e-9 göreli" kabul ölçüsü yaz; ileride özet sürümüyle birlikte daha kararlı bir pencere varyansı düşünülebilir. |
| N3 | Düşük | `ma_stack` kesin karşılaştırmaları (`c > sma20` …) kuruşa yuvarlı fiyatlarda sık görülen **eşitlik** durumlarında kayan toplamın yuvarlama hatasıyla karar veriyor: farklı ama makul bir aritmetik 2–6 kenarı bir bar kaydırıyor (gerçek veride 4 günlük, 6 saatlik). Belirleyici; serinin başlangıç barına duyarlılık sondası (`window_start.py`, ofset 1/137/500) 0 fark verdi. Yine de VERIFY 6'nın "aynı bar" şartı ancak motorun işlem sırası taklit edilirse sağlanıyor. | Dedektörün v2'sinde eşitlik kuralı (ör. `c > f + ε·atr` ya da açık "eşitlik = durum yok") ya da T3'e "eşitlikte karar işlem sırasına bağlıdır" notu. |
| N1 | Düşük (belge) | `docs/CONCEPTS.md` "Olgu dedektörü" satırı "açık barda `peek` aday üretir, kapanışta `push` kesin olgu" diyor; kodda `peek` yok: `push` (bekleyeni kesinleştir, yeni barı bekleyen yap, tespit döndür) ve `update_last`. Sözlük ile kod ayrışmış. | Sözlüğü `push`/`update_last` ve "bekleyen bar" ile güncelle (T3 de öneriyor). |
| N4 | Bilgi | `pipeline.rs` ileriye bakma testi sahte dedektörlerle; gerçek dedektörlerle bir test yok (benim kontrolüm geçti). İskeletin `FixedHorizon`'ı hiç karar vermediğinden, iskeletle yapılan bir ileriye bakma kontrolü karnenin gerçekleşen tarafını sınamaz. | M5 seçildikten sonra ileriye bakma testini gerçek dedektörler + seçilen sonuç kuralıyla tekrarla (VERIFY 8). |
| N5 | Bilgi | Kapanmış hikâyenin geri çekilmesi satırı silmiyor, `retracted_at` ile işaretliyor (T45 gerekçeli; doğru davranış). Bu anlam `docs/CAPSULE.md` §9.3'te yok (yalnız "kapanır, karne yok"). Ayrıca `counts()` `retracted_at >= before_t` ise saydığından, geri çekmeyle **aynı `t`**'de ama ondan sonra işlenen bir doğum (ör. başka enstrüman) satırı hâlâ sayar. | §9.3'e "kapandıktan sonra geri çekme: satır işaretlenir, zaman noktası kuralı" ekle; aynı-`t` durumunu açıkça tanımla. |
| N6 | Bilgi | Ek R3 özeti (`914fcfdb9f653a6d`) `verify.sh`'ta denetlenmiyor (yalnız testte sabit). | T3 önerisi: `digest` adımına `-- r3` eklensin. |
| N7 | Bilgi (hijyen) | `fetch-bars.sh`, `.env.local`'ı `set -a` ile kaynakladığı için o dosyadaki **tüm** değişkenleri alt süreçlere (curl, python3) dışa aktarıyor. `--out` depo içinde izlenen bir yere verilirse koruma yok. | Yalnız `MATRIKS_TOKEN`'ı oku/dışa aktar; `--out`/raw yolunun `fixtures/local/` altında olduğunu denetle. |
| N8 | Bilgi | Sözlük, sözleşme iskeletiyle aynı commit'te (`0ad3a4e`); uygulamalardan önce. | — |
