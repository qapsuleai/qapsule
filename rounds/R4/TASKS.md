# R4 — Alt Görevler

## Evre 1 — zemin

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T1 | Sözlük ve tasarım çerçevesi: yeniden oynatma, yüz, başlık/ayrıntı cümlesi, süre ifadesi; persona tanımları ve görevleri; puanlama ölçütleri (alternatiflerden önce) | `docs/`, `rounds/R4/experiments/harness/` | — |
| T2 | Yeniden oynatma kaynağı ve denetimi (hız, duraklat, adım, tarihe git); gerekirse ek periyotların çekimi | `crates/qp-source/`, `scripts/` | — |
| T3 | K42 uygulaması: karne yönü, hassasiyet geçişi, geçerlilik koşulu ayrımı; R3 ergonomi önerileri | `crates/qp-capsule/`, `crates/qp-session/`, `crates/qp-schema/`, `docs/PROTOCOL.md` | — |
| T4 | Etkileşim: girdi mesajları, bakış durumu, isabet; derinlikler arası açılma komutları | `crates/qp-session/`, `crates/qp-grammar/` | — |
| T5 | Kabuk iskeleti: sahne boyama (WASM), protokol bağlantısı (yansı), girdi iletme; mimari testi; ölçüm ekranı; `scripts/demo.sh` | `shells/web/`, `crates/qp-wasm/`, `scripts/` | T2, T4 |
| T6 | Açılış süresi ve paket boyutu çalışması | `crates/qp-session/`, `crates/qp-wasm/` | T5 |

## Evre 2 — tasarım, ölçüm, seçim

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| M8a–c | Kapsülün yüzü: üç tasarım, üç alt ajan, birbirini görmeden. Her biri gramer kurallarıyla ve şablon cümleleriyle; kabukta özel çizim yok | dal başına; `rounds/R4/experiments/M8/` | T1, T3, T5 |
| M9a–b | Akış düzeni | dal başına | T5 |
| M10a–b | Açılma geçişi | dal başına | T4, T5 |
| M11a–b | Sonuç işaretleri | dal başına | T1 |
| P | Persona okuma ve eleştirmen koşuları: taze alt ajanlar, tasarım adları gizli | `rounds/R4/experiments/` | M8–M11 |
| S | Seçimlerin bütünleştirilmesi; `docs/FACE.md`; altın görseller; ekran görüntüsü testleri; dökümler | bütünleştirme dalı | P |
| D | Cihaz ölçümü: masaüstü ve kısıtlanmış işlemci; Arkin için telefon yönergesi | `rounds/R4/DEVICE.md` | S |
| E | Ajan yazabilirlik yeniden ölçümü | `rounds/R4/ergonomics/` | T3 |
| HT | İnsan testi v2 paketi | `rounds/R4/human-test/` | S |
| A | Denetim: bağımsız alt ajan. Gösterimi kendisi açar ve bir kullanıcı gibi kullanır; kabuğun inceliğini kendi yöntemiyle sınar (kabuksuz sürüş); persona koşularından üçünü kendi taze ajanlarıyla tekrarlar; sayfanın dış ağa çıkmadığını doğrular | `rounds/R4/AUDIT.md` | hepsi |

## Notlar

- **Tasarım gramerde yaşar.** Kapsülün yüzü, akış ve geçiş, gramer ve derinlik
  kurallarıyla ifade edilir; kabuk bunları boyar. Bir tasarım yalnızca kabukta özel kodla
  mümkün oluyorsa ya grameri genişlet (ek olarak, gerekçesiyle) ya da tasarımı ele.
  Böylece aynı yüz yarın yerel iOS/Android kabuğunda ve ekransız çıktıda da aynı olur.
- **Düzen öğeleri** (liste kaydırma, sekme, ayar sayfası) kabuğa aittir; alan bilgisi
  taşımazlar. Sınırı T1'de yaz ve mimari testine koy.
- **Alternatifler gerçek olmalı;** düzenek ve puanlama ölçütü önce.
- **Bilinen kalıplarla sınırlı değilsin (K18).** Finans uygulamalarının alışılmış kart
  düzenini kopyalamak yerine "tek bakışta anlam" hedefinden tasarla. Var olan iyi
  örnekleri (hava durumu, sağlık halkaları, navigasyon yönergeleri gibi başka alanlardan)
  incele.
- **Persona ajanları** tasarımın adını, yapanı ve önceki turların sonucunu görmez;
  yalnızca ekran görüntüsünü ve soruyu görür. Dosya erişimi R2/R3 düzeneğindeki gibi
  kısıtlanır.
- **Gerçek veri:** gösterim ve ölçümler yerel veriyle koşar; depoya yalnızca sentetik
  veriyle üretilmiş görseller ve toplu istatistik girer. Sayfa dış ağa çıkmaz.
- **Yeniden oynatma gün içi akıcılık için** daha ince periyot gerekebilir (ör. birkaç
  sembolde 1 ya da 5 dakikalık barlar); `docs/DATA_ACCESS.md` sınırlarına uyarak çek.
- **Erişilebilirlik:** renk tek başına anlam taşımaz (yön ve geçerlilik şekille de
  okunur); karşıtlık eşiği ölçülür.
- **GitHub ve ana akış:** `CLAUDE.md` kuralları geçerli.
