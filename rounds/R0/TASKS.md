# R0 — Alt Görevler

T1 önce biter; T2, T3, T4 paralel yürür; T5 ve T6 en sonda.

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T1 | Depo ve iskelet: `git init`, workspace `Cargo.toml`, boş crate'ler (`qp-schema`, `qp-render`, `qp-cli`, `qp-wasm`), `rustfmt`/`clippy` ayarları, `.gitignore`, CI iş akışı | kök, `crates/*/Cargo.toml`, `.github/` | — |
| T2 | Şema v0: `docs/CONCEPTS.md` içindeki kavramların Rust tipleri (serde), JSON Schema üretimi, gidiş-dönüş ve geçerlilik testleri, örnek Kapsül ve Doküman | `crates/qp-schema/`, `fixtures/schema/` | T1 |
| T3 | Çizim denemesi: örnek bar verisi, ekransız PNG çıktısı, deterministik referans çizici, WASM derlemesi ve test sayfası. En az iki çizim yığını seçeneğini küçük ölçekte dene, ölç, A1'i kapat | `crates/qp-render/`, `crates/qp-wasm/`, `fixtures/bars/`, `fixtures/golden/`, `shells/web-spike/` | T1 |
| T4 | Eski kod envanteri: `../Services/quantex-charts-core` ve `../Docs/q-charts` okunur. `docs/LEGACY.md` ve `docs/MATRIKS_PROTOCOL.md` yazılır. Kod taşınmaz, yalnızca envanter çıkarılır | `docs/` | T1 |
| T5 | `qp` komut satırı: `schema export`, `render` | `crates/qp-cli/` | T2, T3 |
| T6 | Doğrulama düzeneği: `scripts/verify.sh` (biçim, lint, test, altın görsel, wasm derleme), CI'a bağlanması | `scripts/`, `.github/` | T5 |
| T7 | Denetim: ayrı bir alt ajan `VERIFY.md` adımlarını temiz kopyada çalıştırır, kabul ölçütlerini tek tek işaretler | `rounds/R0/REPORT.md` | T6 |

## T3 için notlar

- Altın görsel testi için çizim sonucu makineden makineye değişmemeli. GPU çıktısı
  sürücüye göre farklılaşabilir; bu yüzden deterministik bir referans yol gerekir.
  Nasıl sağlanacağı denemenin konusudur.
- Çizilecek şey basit: eksenler, mum çubukları, bir çizgi serisi, kısa bir metin etiketi.
  Metin çizimi denemeye dahildir; sonradan en çok sorun çıkaran kısım odur.
- Ölçülecekler: ilk kare süresi, 100 bin bar çizim süresi, WASM dosya boyutu,
  ekransız ve tarayıcı çıktısı arasındaki piksel farkı.

## T4 için notlar

- `ts/.env` okunmaz.
- Envanterde özellikle bakılacaklar: viewport, LOD ve hit-test matematiği; özellik
  testleri; benchmark düzeneği; MQTT v3.1 bağlantı kuralları, kimlik doğrulama biçimi,
  konu yapısı, REST uç nokta eşlemesi, zaman damgası birimleri.
