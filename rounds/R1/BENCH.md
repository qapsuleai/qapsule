# R1 / T3 — Parametre taraması ölçümü

## Düzenek

- **İş:** her indikatör için 10.000 bar × 1.000 parametre kümesi, tek çağrı
  (`qp_compute::sweep::sweep_each`), **tek iş parçacığı**.
- **Girdi:** `sweep::bench_closes(10_000, 20_261_114)` — tamsayı LCG ile deterministik rastgele
  yürüyüş, iki ondalıklı fiyatlar (88,41–317,19).
- **Parametre ızgarası** (`sweep::bench_grid`, 1.000 küme):
  - `sma`, `ema`, `rsi`: periyot 2…1001
  - `macd`: fast 2…11 × slow fast+1, +3, …, +19 × signal 2…11
  - `bollinger`: periyot 2…101 × k 1,00, 1,25, …, 3,25
- **Çıktı:** her varyasyonun tüm çıktıları (MACD 3, Bollinger 4 sütun) her bar için yazılır;
  tampon varyasyonlar arasında yeniden kullanılır. Ölçüm, çıktı yazımını da içerir.
- **Yerel:** `cargo run --release -p qp-compute --example sweep_bench -- --reps 21`; bir ısınma
  koşusu, sonra 21 koşunun en küçük / medyan / en büyük süresi.
- **WASM:** `scripts/wasm-sweep-bench.sh` — `qp-wasm` wasm-pack ile `--target nodejs
  --profile wasm` derlenir, Node 24.3.0 (V8 13.6) içinde `sweep_bench` çağrısı JS tarafında
  `performance.now()` ile ölçülür (15 koşu).
- **Makine:** Apple M4 Max (aarch64), macOS; makine ölçüm sırasında başka işlerle yüklüydü
  (load average ≈ 3,8), sayılar buna göre okunmalı.

## Sonuç (medyan, ms)

| İndikatör | Yerel aarch64 | Hedef (yerel) | WASM (Node/V8) | Hedef (WASM) | x86_64 (Rosetta 2, bilgi) |
|-----------|--------------:|:-------------:|---------------:|:------------:|--------------------------:|
| SMA       | **14,2** | < 50 ✅ | **16,4** | < 150 ✅ | 15,6 |
| EMA       | **8,1**  | < 50 ✅ | **27,3** | < 150 ✅ | 21,8 |
| RSI       | **22,4** | < 50 ✅ | **58,6** | < 150 ✅ | 27,6 |
| MACD      | **41,2** | < 50 ✅ | **81,5** | < 150 ✅ | 71,0 |
| Bollinger | **44,4** | < 50 ✅ | **71,4** | < 150 ✅ | 64,2 |

Ham satırlar (en küçük / medyan / en büyük):

```
yerel aarch64 (varsayılan şeritler), 21 koşu
sma             1000     13.34     14.22     14.58  d9a393c2f397ae9e
ema             1000      7.83      8.08     12.16  f9be271e91103539
rsi             1000     21.75     22.38     24.80  9515526d5288453b
macd            1000     40.39     41.15     42.95  d5a6c407256992e6
bollinger       1000     43.57     44.39     50.31  7e683c082fb95dc9

wasm32, Node v24.3.0 (V8 13.6.233.10), 15 koşu
sma            1000     16.20     16.38     16.59  d9a393c2f397ae9e
ema            1000     26.56     27.30     27.51  f9be271e91103539
rsi            1000     58.04     58.61     62.48  9515526d5288453b
macd           1000     80.59     81.46     82.06  d5a6c407256992e6
bollinger      1000     70.37     71.36     74.88  7e683c082fb95dc9

x86_64-apple-darwin (Rosetta 2), 11 koşu
sma             1000     15.18     15.61     16.02  d9a393c2f397ae9e
ema             1000     21.53     21.83     22.07  f9be271e91103539
rsi             1000     27.35     27.62     28.32  9515526d5288453b
macd            1000     69.44     71.00     72.09  d5a6c407256992e6
bollinger       1000     63.81     64.18     64.68  7e683c082fb95dc9
```

Referans makinede (M4 Max) hedeflerin hepsi tutuyor. **Sınırda olanlar:** yerel MACD (41 ms)
ve Bollinger (44 ms; en büyük koşu 50,3 ms) hedefin %80–90'ında.

### Gerçek Linux x86_64: GitHub Actions koşucusu (bilgi amaçlı, kapı değil)

CI çalışması https://github.com/qapsuleai/qapsule/actions/runs/43614904957 (commit `96023c0`).
Koşucu `ubuntu-latest` (ubuntu-26.04), paylaşımlı sanal çekirdek; işlemci modeli günlüğe
basılmadı. Yerel 7 koşu, WASM Node 22.23.4 (V8 12.4) 15 koşu, medyan ms:

| İndikatör | Linux x86_64 yerel | Hedef < 50 | WASM (Node 22) | Hedef < 150 |
|-----------|-------------------:|:----------:|---------------:|:-----------:|
| SMA       | 17,5 | ✅ | 26,9 | ✅ |
| EMA       | 21,5 | ✅ | 37,3 | ✅ |
| RSI       | 62,8 | ❌ | 118,3 | ✅ |
| MACD      | 69,4 | ❌ | **156,6** | ❌ |
| Bollinger | 72,5 | ❌ | 120,0 | ✅ |

Özetler (son sütun) burada da aarch64 ile birebir aynı.

**Tutmayanlar ve nedenleri:** CI koşucusunda RSI, MACD ve Bollinger yerel 50 ms'yi, MACD ise
WASM 150 ms'yi aşıyor. Hedefler test olarak gevşetilmedi.

1. Koşucu çekirdeği M4 Max'ten belirgin biçimde yavaş. SMA ve EMA gibi basit hesaplar bile
   1,2–2,7 kat yavaş; paylaşımlı sanal çekirdek ve bilinmeyen model.
2. Şerit sayıları yalnızca Apple M4 üzerinde ölçülerek seçildi. x86_64 temel hedefinde
   (SSE2) 16 vektör yazmacı var, M4'te 32. 4 şeritli RSI ve 2 şeritli MACD/Bollinger
   x86'da yazmaç taşmasına girebilir. Rosetta ölçümü gerçek x86'yı temsil etmiyor.
3. MACD ve Bollinger ızgaralarında iş tekrar ediyor:
   - MACD'de her (fast, slow) çifti için 10 signal değeri var; fast/slow EMA'ları 10 kez
     hesaplanıyor.
   - Bollinger'de her periyot için 10 k değeri var; SMA ve std 10 kez hesaplanıyor.

**Öneriler (sırayla):**

- **(a) Ortak alt hesap paylaşımı (bit-eşit):**
  - MACD'de macd çizgisi (fast, slow) başına bir kez hesaplanır, yalnızca signal EMA'sı
    varyasyon başına çalışır.
  - Bollinger'de middle/std periyot başına bir kez hesaplanır, k en sonda uygulanır.
  - Tek çalıştırmayla aynı işlemler aynı sırada yapıldığından sonuçlar bit-eşit kalır.
  - Beklenen kazanç: MACD'de işin ~2/3'ü, Bollinger'de ~9/10'u ortadan kalkar.
- **(b) Hedef mimariye göre şerit ayarı** (`cfg(target_arch)`), x86_64 için gerçek
  donanımda ölçülerek.
- **(c)** CI ölçüm adımına `lscpu` çıktısı eklenmeli; sayılar donanıma bağlanabilsin.

## Bit-eşitlik

- **Tarama = tek tek çalıştırma**, bit düzeyinde. `tests/sweep.rs`, ızgaradan seçilen
  varyasyonlarda bunu 1, 2, 3, 4 ve 8 şeritte ve varsayılan şeritte sınar. Tarama
  çekirdekleri, `Study` arayüzünün kullandığı `peek1`/`commit1` fonksiyonlarını aynen çağırır.
- **Mimariler arası:** yukarıdaki beş özet aarch64, x86_64 ve wasm32'de birebir aynı. Toplam
  hesap özeti `cross_target_digest()` = `7b1e99d40889f83c`; bu da üç hedefte aynı (T7,
  `verify.sh`).

## İyileştirmeler ve etkileri (yerel medyan, ms)

| Adım | SMA | EMA | RSI | MACD | Bollinger | Not |
|------|----:|----:|----:|-----:|----------:|-----|
| 1. Düz uygulama (`peek1`+`commit1`, halkada `%`) | 13,1 | 13,6 | 38,6 | 32,3 | 119,8 | |
| 2. Halkada `%` yok; Bollinger tek halka; RSI dallanmasız | 17,4 | 13,9 | 37,2 | 32,1 | 120,4 | Özetler değişmedi |
| 3. Sütun dilimlerine doğrudan yazım | 15,2 | 17,9 | 25,2 | 46,7 | 127,9 | 8 şeritle; şeritler aşağıda |
| 4. Şerit sayısı hesap başına ölçülerek seçildi | 15,1 | 9,1 | 23,7 | 42,4 | 72,7 | |
| 5. Bollinger: dolu pencerede `1/p` ile çarpma | 15,1 | 9,8 | 23,6 | 42,4 | 59,1 | Bollinger özeti değişti (yeni tanım) |
| 6. Bollinger: pencere ortalaması SMA toplamından; düz pencere sayacı | 14,2 | 8,1 | 22,4 | 41,2 | 44,4 | Son hâl |

**Şerit:** `L` varyasyon aynı döngüde birlikte adımlanır. Her şeridin aritmetiği tek
çalıştırmanınkiyle aynıdır, yalnızca komutlar iç içe geçer. Böylece işlemci, döngüde taşınan
bağımlılık zincirlerini (bölme, çarpma, toplama) örtüştürebilir. Ölçüm, medyan ms:

| Şerit | SMA | EMA | RSI | MACD | Bollinger |
|------:|----:|----:|----:|-----:|----------:|
| 1 | **14,1** | 20,8 | 58,3 | 45,3 | **44,6** |
| 2 | 16,2 | **8,8** | 31,6 | **41,0** | 56,4 |
| 4 | 15,7 | 9,3 | **22,9** | 90,7 | 138,2 |
| 8 | 27,1 | 12,2 | 24,5 | 127,1 | 163,5 |

Seçim: SMA 1, EMA 2, RSI 4, MACD 2, Bollinger 1. Çok çıktılı hesaplarda 4–8 şerit yazmaç
taşmasıyla yavaşlıyor. WASM için ayrı ayar yapılmadı; aynı şeritler kullanıldı.

## Doğruluk bulgusu: kayan varyans ve düz pencereler

Bollinger'in kayan varyansı için üç yöntem karşılaştırıldı. Ölçüm, 10.000 barlık ölçüm
serisinde periyot 2, 5, 20, 50 ve 101 ile, her pencere için iki geçişli hesaba karşı yapıldı
(`tests/sweep.rs::rolling_variance_accuracy_on_long_series`):

| Yöntem | Azami mutlak std hatası | Not |
|--------|------------------------:|-----|
| Ders kitabı kayan Welford (ortalama ve M2 kayar) | 3,5e-6 | Düz pencerede gerçek varyans 0; ~1e-15'lik M2 artığı `sqrt` ile ~1e-7–1e-6'ya büyüyor |
| **Seçilen:** M2 Welford ile; pencere ortalaması SMA kayan toplamından; düz pencere sayacı | **1,0e-9** (fiyata göre 1,1e-11) | Düz pencerede M2 tam 0'a sıfırlanır, birikmiş artık da temizlenir |

Eski kodun test kümesinde ders kitabı Welford yeterli görünüyordu; açığı uzun seri ve düz
pencere testi ortaya çıkardı. Fiyatı günlerce değişmeyen sığ hisselerde bu durum gerçekten
görülür. Düz pencerede bant genişliği artık **tam 0** (`flat_windows_are_exact_at_real_price_levels`).
Orta bant (SMA) ise kayan toplam yüzünden ~1e-13 kayabilir; eski kodda da aynısı var.

## WASM boyutu

`qp-wasm`'a `compute`, `sweep_bench`, `sweep_digest` ve `compute_digest` bağlamaları eklendi
(`cargo build --profile wasm`, ham modül): T1 sonrası 1.140.525 B (gzip 397.903 B) →
1.320.781 B (gzip 441.978 B), yani gzip ile +44 KB. Şerit kopyalarının payı yalnızca ~9 KB
ham; geri kalanı hesap kodu ve parametre JSON çözümlemesi. Takvim/saat dilimi verisinin
etkisi T6'da ayrıca ölçüldü.

## Öneriler

- Hedefe en yakın iki indikatör MACD ve Bollinger. Pay gerekirse ilk seçenek, aynı periyodu
  paylaşan varyasyonlarda ortak kısmı bir kez hesaplamak. Bollinger'de k değişirken SMA ve
  std aynı kalır; ızgarada 10 k değeri var, yani SMA/std işinin 9/10'u tekrar ediliyor.
  Bunun için çıktıyı `(periyot, k)` yerine `periyot` başına üretip k'yi en son uygulayan bir
  çekirdek gerekir; sonuç yine bit-eşit kalır. Ölçülerek R2'de ya da ihtiyaç doğunca yapılır.
- WASM'da `simd128` ve wasm'a özel şerit ayarı denenmedi.
