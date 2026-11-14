# RP1 / T1 — Ölçüm tabanı ve başlangıç ölçümleri

Commit: `caab6e3` (dal `rp1/t1-baseline`), hesap kodu R1 ile aynı (`f5c7d2c`); yalnızca ölçüm
düzeneği eklendi.

## Düzenek

- **Yük profilleri** `crates/qp-compute/src/profiles.rs` (`#[doc(hidden)]`): her profil girdiyi,
  parametreleri ve istenen çıktıları sabitler ve **bütün çıktıların özetini** verir; bir
  iyileştirme hem ölçülür hem bit-eşitliği özetle sınanır.
  - **A — tek seri, çok parametre:** 10.000 bar × 1.000 varyasyon, beş hesap (R1 `bench_grid`).
  - **B — çok seri, tek hesap:** 500 sembol × 5 zaman dilimi (10.000 / 5.000 / 2.000 / 1.000 /
    500 bar) = 2.500 seri, 9,25 M bar; her hesap varsayılan parametreyle.
  - **C — zincirli hesaplar:** her aileden 1.000 varyasyon, 10.000 bar: `ema>rsi`
    (10 × 100), `rsi>bollinger` (10 × 10 × 10), `sma>ema>rsi` (10 × 10 × 10); `Chain` anlamı.
  - **D — canlı güncelleme:** 500 hesaplık banka (her yerleşik hesaptan 100), 10.000 bar
    geçmiş; 2.000 tik (`update_last`, hepsinde) ve 1.000 yeni bar (`push`); bir de "attach"
    (bankayı kur + geçmişi koş).
- **Sürücü** `crates/qp-compute/benches/driver/mod.rs`: zaman ölçümü kütüphane dışında
  (hesap kodu saat okumaz). `examples/sweep_bench.rs` (CI'ın çalıştırdığı) ve
  `benches/profiles.rs` (`cargo bench -p qp-compute --bench profiles -- --only a,b,c,d`)
  ikisi de bunu kullanır. Her satır: en küçük / medyan / en büyük ve özet. Makine satırı CPU
  modelini basar (R1 önerisi (c): CI'da `lscpu` yerine `/proc/cpuinfo`).
- **WASM** `crates/qp-compute/benches/wasi-run.mjs`: aynı örnek `wasm32-wasip1` için
  `--profile wasm` (opt-level s, fat LTO; `qp-wasm` ile aynı profil) derlenip Node'un
  `node:wasi` modülünde koşar. `qp-wasm`'a dokunulmadı. A profilinde wasm-pack yoluyla
  (R1) aynı sayıları veriyor (SMA 16,2 / 16,4; RSI 58,7 / 58,6; MACD 81,6 / 81,5 ms).
  Bunun için `qp-compute`'un `proptest` geliştirme bağımlılığında `fork` özelliği kapatıldı
  (wasip1'de derlenmiyor; testler kullanmıyordu).
- **Sabitlenmiş özetler** `tests/profiles.rs`: B (50 seri), C (2.000 bar, üç aile) alt kümeleri
  ve D'nin tam özeti; profilleri kullanan bir yol bit değiştirirse test kırılır.
- **Gürültü:** her ölçüm medyan ve yayılımla (en küçük–en büyük) verilir; CI'da 7 koşu.

## Başlangıç ölçümleri (taban `caab6e3`)

Referans makine Apple M4 Max (12P+4E), macOS; ölçüm sırasında makine yüklüydü (yük ortalaması
≈ 4). x86_64 yerelde Rosetta 2 ile (yalnızca bilgi). CI: GitHub `ubuntu-latest`,
**AMD EPYC 9V74 (Zen 4), 4 vCPU**, çalışma
[43618730149](https://github.com/qapsuleai/qapsule/actions/runs/43618730149). Ham çıktılar
`rounds/RP1/raw/`.

### A — medyan ms (en küçük–en büyük)

| Hesap | aarch64 (11) | x86_64 Rosetta (11) | WASM Node 24 (11) | CI x86_64 (7) | CI WASM Node 22 (15) |
|-------|-------------:|--------------------:|------------------:|--------------:|---------------------:|
| SMA | 15,07 (14,68–15,11) | 15,54 | 15,62 | 17,44 | 26,44 |
| EMA | 9,62 (8,81–20,65) | 22,35 | 26,20 | 21,59 | 37,63 |
| RSI | 24,73 (24,64–24,98) | 27,77 | 56,02 | **62,87** | 118,46 |
| MACD | 41,40 (41,32–41,66) | 72,56 | 75,70 | **69,07** | **159,91** |
| Bollinger | 46,27 (45,76–48,13) | 64,43 | 69,74 | **71,98** | 120,71 |

K17 dizüstü hedefi: yerel < 50 ms, WASM < 150 ms. CI'da RSI, MACD, Bollinger yerelde; MACD
WASM'da aşıyor (R1 ile aynı tablo).

### B — 2.500 seri, medyan ms

| Hesap | aarch64 | x86_64 Rosetta | WASM Node 24 | CI x86_64 |
|-------|--------:|---------------:|-------------:|----------:|
| SMA | 14,43 | 16,41 | 15,36 | 17,75 |
| EMA | 25,93 | 41,20 | 17,61 | 40,35 |
| RSI | 61,99 | 40,90 | 61,43 | 98,27 |
| MACD | 44,16 | 67,52 | 68,85 | 88,98 |
| Bollinger | 42,84 | 61,93 | 65,29 | 113,99 |

### C — zincirler, 1.000 varyasyon, medyan ms

| Aile | aarch64 | x86_64 Rosetta | WASM Node 24 | CI x86_64 |
|------|--------:|---------------:|-------------:|----------:|
| ema>rsi | 99,95 | 151,74 | 189,54 | 200,66 |
| rsi>bollinger | 175,84 | 248,15 | 340,67 | 335,97 |
| sma>ema>rsi | 161,71 | 238,27 | 379,57 | 300,34 |

### D — 500 hesap, işlem başına

| Ölçüm | aarch64 | x86_64 Rosetta | WASM Node 24 | CI x86_64 |
|-------|--------:|---------------:|-------------:|----------:|
| tik, medyan (p99) µs | 1,79 (2,17) | 2,71 (2,83) | 4,25 (5,42) | 3,50 (3,87) |
| yeni bar, medyan (p99) µs | 2,67 (3,33) | 3,83 (4,00) | 5,12 (6,62) | 5,18 (5,77) |
| attach (kur + 10k bar) ms | 26,28 | 38,12 | 51,11 | 51,82 |

### Özetler (her hedefte aynı)

A: SMA `d9a393c2f397ae9e`, EMA `f9be271e91103539`, RSI `9515526d5288453b`, MACD
`d5a6c407256992e6`, Bollinger `7e683c082fb95dc9` (R1 ile aynı). B: `4deca6b358b56f6c`,
`858efc3a7829a6ab`, `23d1bb6754b8ccae`, `25dca317a156c644`, `2aee14ef015f0228`. C:
`d60c4040e327df75`, `51d10314ec252891`, `8c536220fb4398a5`. D: `4db86a7246f733bf`.
`cross_target_digest` = `7b1e99d40889f83c`.

## İlk gözlemler (deneylere yön veren)

1. **D'nin tik yolu sınır değil** (500 hesap 2–5 µs). D'de sınır attach (26–52 ms).
2. **B'de şerit yok:** tek parametreli çağrılar şerit kullanamıyor; RSI B 62 ms (CI 98).
3. **C'de iş tekrar ediyor:** ilk aşamanın 10 farklı parametresi 1.000 kez hesaplanıyor; her
   varyasyon `dyn Study` + `Option` ile bar bar.
4. **x86'da RSI bölme çıkışıyla sınırlı** olmalı: 1.000 × 10.000 × 4 bölme = 40 M bölme ≈
   Zen 4'te ~50 ms (ölçülen 62,9).
