# R2 — Alt Görevler

T1 önce; T2 ve T3 ardından; T4–T8 paralel yürüyebilir; T9 ölçüm ve düzeltme; T11 en sonda.

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T1 | Gramer tasarımı: önce altı görünümün ve yirmi görevin taslağı, sonra tipler, `docs/GRAMMAR.md`, sözlük | `docs/`, `crates/qp-schema/`, `rounds/R2/tasks/` | — |
| T2 | Protokol eki: kimlikli gramer işlemleri, kullanımdan kalkan indeksli alanlar, `welcome`/`screen` doküman özeti; şartname ve örnekler | `crates/qp-schema/src/protocol.rs`, `docs/PROTOCOL.md`, `fixtures/protocol/` | T1 |
| T3 | `qp-grammar`: Görünüm + veri → sahne tarifi; doğrulama ve hata iletileri | `crates/qp-grammar/` | T1 |
| T4 | Referans çizici sahne tarifinden çizer; altı görünüm ve derinlik altın görselleri | `crates/qp-render/`, `fixtures/golden/`, `fixtures/views/` | T3 |
| T5 | Oturum: gramer işlemleri, köken denetimi, sunucunun gramer kurallarına taşınması, derinlik indirgemesi | `crates/qp-session/` | T2, T3 |
| T6 | Anlamsal okuma: `screen` ve nokta sorgusu sahne tarifinden; `qp explain` | `crates/qp-grammar/` (hit), `crates/qp-session/`, `crates/qp-cli/` | T3 |
| T7 | Görüş alanı ve LOD; 100k bar ölçümü | `crates/qp-grammar/` (viewport, lod) | T3 |
| T8a, T8b | Anlamla başvuru: iki tasarım, ayrı dallarda, ayrı alt ajanlarla, birbirini görmeden | dal başına `crates/qp-session/`, `rounds/R2/experiments/` | T5 |
| T9 | Ajan yazabilirlik ölçümü (gramer görevleri), tasarım düzeltmesi, yeniden ölçüm; T8'in iki tasarımı bu düzenekle karşılaştırılır | `rounds/R2/ergonomics/` | T5, T6, T8 |
| T10 | Uygunluk dökümleri; MCP araçlarının yeniden üretimi; R1/RD1 denetim kalanları | `fixtures/protocol/conformance/`, `crates/qp-mcp/`, `crates/qp-compute/` | T5 |
| T11 | Denetim: bağımsız alt ajan, temiz kopya. Altın görselleri gözle inceler, sayısal noktaları veriyle karşılaştırır; yalnızca şartnameyi okuyarak yeni bir görünüm kurar | `rounds/R2/AUDIT.md` | hepsi |

## Notlar

- **T1 bu round'un en önemli görevi.** Gramer kalıcı çekirdektir. Var olan gramerleri
  (RD1 araştırmasındaki Vega-Lite notları dahil) incele, ama kopyalama: seans boşlukları,
  canlı son bar, çok panel, `null` ısınma, köken ve derinlik burada birinci sınıftır.
  Bilinen yöntemlerle sınırlı değilsin (K18).
- **Sınav protokolden geçer.** Altı görünüm elle yazılmış JSON dosyasından değil,
  bir protokol mesaj dizisinden (döküm) kurulur. Böylece "ajan bunu kurabilir mi"
  sorusu baştan cevaplanır.
- **Sahne tarifi tek doğruluk kaynağıdır:** çizici ve anlamsal okuma aynısını okur.
- **Ölçüm düzeneği (T9):** RD1'in düzeneğini kullan (`rounds/RD1/ergonomics/harness`),
  iki düzeltmeyle: kimlikler her koşuda kopyalanamaz; alt ajanların yalnızca şartname ve
  şemayı görmesi dosya erişimiyle sağlanır (RD1 denetim koşulu O3).
- **Veri (K23):** doküman içinde bar taşıyan test verileri bu round'da olduğu gibi kalır;
  gramer seriye her zaman başvuruyla ulaşır, böylece RP2'de veri bağına geçiş grameri
  etkilemez. Protokolden büyük bar dizileri geçirme.
- **Altın görseller:** var olanlar güncellenmez; yeni görünümler için yenileri eklenir.
- **Hesap:** çizicide ya da gramerde hesap kodu olmaz; değerler `qp-compute`'tan gelir.
- **GitHub:** `CLAUDE.md` kuralları geçerli (`scripts/qgh`, `scripts/preflight.sh`).
