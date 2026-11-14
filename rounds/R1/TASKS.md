# R1 — Alt Görevler

T0 ve T1 önce; T2, T4, T6 paralel; T3, T5, T7 onlardan sonra; T8 en sonda.

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T0 | Uzak depo: `scripts/preflight.sh` geçtikten sonra `main`'i `origin`'e gönder, CI'ın ilk çalışmasını `scripts/qgh run watch` ile izle, sonucu kaydet. Fark çıkarsa kök nedeni araştır | `.github/`, `rounds/R1/CI_FIRST_RUN.md` | — |
| T1 | Şema: tek `Bar`/`Series` tipi, `null` ısınma kuralı, `Instrument`, `Calendar`, `display_tz`; doğrulama fonksiyonu kütüphaneye; `fixtures/bars` şemaya bağlanır | `crates/qp-schema/`, `fixtures/` | — |
| T2 | `qp-compute` çekirdeği: `Study` arayüzü (çok girdi, çok çıktı, toplu/akışkan/düzelt), beş indikatör, elle hesaplanmış örnekler, eski sonuçlarla karşılaştırma, özellik testleri | `crates/qp-compute/` | T1 |
| T3 | Parametre taraması ve ölçüm düzeneği; yerel ve WASM ölçümleri | `crates/qp-compute/` (sweep, benches), `rounds/R1/BENCH.md` | T2 |
| T4 | `SeriesSource` arayüzü, dosya ve sentetik kaynak | `crates/qp-compute/src/source/` ya da ayrı crate (karar raporda) | T1 |
| T5 | `qp compute` komutu; `qp schema validate` kütüphaneyi çağırır | `crates/qp-cli/` | T2, T4 |
| T6 | Takvim: saat dilimli bar sınırları, BIST ve NYSE örnekleri, WASM boyut ölçümü | `crates/qp-compute/src/calendar/` ya da ayrı crate | T1 |
| T7 | Bütünleştirme: çizici `qp-schema` barlarını ve `qp-compute` SMA'sını kullanır; `qp-wasm` hesap bağlaması; mimariler arası özet testi; `verify.sh` güncellemesi | `crates/qp-render/`, `crates/qp-wasm/`, `scripts/` | T2, T3 |
| T8 | Denetim: bağımsız alt ajan, temiz kopya | `rounds/R1/AUDIT.md` | hepsi |

## Notlar

- **Eski kod kâhin olarak:** `../Services/quantex-charts-core/core/src/indicator/` salt
  okunur. Eski kod ısınma barlarını kırpıyor ve `f32` bar kullanıyor; karşılaştırırken
  hizayı ve tipi buna göre eşle. Eski kodun kendisi yanlışsa (elle hesapla çelişiyorsa)
  elle hesap esas alınır ve fark rapora yazılır.
- **"Son barı düzelt":** canlı barda her fiyat değişiminde tüm seriyi yeniden hesaplamak
  kabul edilmez. Hesabın durumunu bir bar geri alabilmesi ya da son bar öncesi durumu
  saklaması gerekir; yaklaşım T2'de seçilir ve gerekçesi yazılır.
- **Tarama:** bellek yerleşimi ve tek geçişte çok parametre gibi iyileştirmeler serbest,
  ama çıktı tek tek çalıştırmayla bit-eşit kalmalı. İş parçacığı kullanımı isteğe bağlı;
  hedefler tek çekirdek içindir.
- **Takvim kütüphanesi:** saat dilimi verisi WASM boyutunu büyütebilir. En az iki
  seçeneği ölç, seçimi `DECISIONS.md`'ye yaz.
- **Altın görseller:** T7 sonunda bayt düzeyinde aynı kalmalı. Değişirse güncelleme
  yapma; farkı ve nedenini rapora yaz.
