# R5B — Alt Görevler

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T0 | Hazırlık: ana akış değişiklikleri; RP4 bittiyse hızlandırılmış benzetim, bitmediyse R4 yeniden oynatması oturum kaynağı; Matriks MCP değişkenlerinin **varlık** denetimi (değer yazdırmaz); MCP yapılandırmasının ortam değişkenine başvuran biçimi (güncel belgeden) | `scripts/`, `.mcp.json` (gizli değer içermez) | — |
| T1 | Araştırma: güncel MCP şartnamesi (araç, kaynak, abonelik ve bildirim, yetkilendirme), ajan araç tasarımı üzerine güncel kanıt (az ve görev düzeyi araçlar, kod kipi, cevap biçimi), dış içerikten talimat saldırıları ve savunmaları | `rounds/R5B/RESEARCH.md` | — |
| H | Görev kümesi (40+), doğru cevap üretici (motordan), puanlama kuralı, bozuk yorum kümesi, gömülü talimat örnekleri; **alternatiflerden önce commit** | `rounds/R5B/experiments/harness/` | T1 |
| T2 | `qp mcp serve`: oturuma bağlanma, okuyucu kimliği, stdio + HTTP, yalnız yerel varsayılanı, bildirim yolu | `crates/qp-mcp/`, `crates/qp-cli/` | — |
| T3 | Niyet: şema, eşleşme, yaşam döngüsü, sınırlar; protokol eki | `crates/qp-schema/`, `crates/qp-session/`, `docs/PROTOCOL.md` | — |
| T4 | Yorum ve doğrulayıcı; reddetme nedenleri; saklama ve köken | `crates/qp-session/`, `crates/qp-capsule/` (dil taramaları), `crates/qp-schema/` | — |
| T5 | Öneri yolu: bekleyen öneriler, onay / ret, ekransız görünüm kurma | `crates/qp-session/`, `crates/qp-grammar/` | — |
| M28a–d | Araç yüzeyi alternatifleri, ayrı alt ajanlar, ayrı dallar | `crates/qp-mcp/` | H, T2 |
| M29 | Kapsülün ajana veriliş biçimi | `crates/qp-mcp/` | H |
| M30 | Doğrulayıcı sıkılığı | T4 üstünde | H, T4 |
| R1–R3 | Başvuru ajanları: Yorumcu, Nöbetçi, Soru cevaplayan (yönergeler ve küçük sürücü) | `agents/` (yeni klasör) | seçilen yüzey, T3–T5 |
| M31 | Nöbetçi süzgeci | `agents/`, `crates/qp-session/` | R2 |
| T6 | Gösterimde yorum ve öneri izi | `crates/qp-grammar/`, `shells/web/` | T4, T5 |
| T7 | `docs/AGENTS.md`; Arkin için deneme | `docs/` | hepsi |
| A | Denetim: bağımsız alt ajan; doğrulayıcıyı kendi bozuk yorumlarıyla sınar; lisans girişimsizliğini ajan yolunda dener; görev kümesinin doğru cevaplarının motordan geldiğini doğrular; gizli bilgi taraması | `rounds/R5B/AUDIT.md` | hepsi |

## Notlar

- **Ajan neyi yazamaz:** olgu, bulgu, kapsül, karne, hikâye durumu. Bunun testi vardır
  (ajan kimliğiyle gelen böyle bir komut reddedilir).
- **Sayı denetimi nasıl:** yorumdaki her sayı (birim ve yerel biçim normalleştirilerek)
  atıf yapılan kanıtın değerleri, kapsülün yapı taşları ya da "dış kaynak" diye işaretli
  alıntılar arasında aranır; yuvarlama kuralı açık yazılır (ör. gösterilen basamağa kadar).
  Türetilmiş sayı (ör. iki değerin farkı) için kural ölçülerek belirlenir: ya motor
  türetilmiş değeri de verir ya da ajan yalnız verileni yazar.
- **Niyet eşleşmesi** seçicileri (R2: anlamla başvuru) ve altı anlam türünü kullanır;
  serbest metin niyeti ajanın işidir (metni yapılandırılmış niyete çevirir), motor yalnız
  yapılandırılmış niyeti bilir.
- **Model boyları:** ölçüm küçük, orta ve büyük modelle; alt ajan başlatırken model
  seçilebiliyorsa onunla. Maliyeti ve çağrı sayısını kaydet; üst sınır yaklaşık 60 dolar
  karşılığı ölçüm; aşacaksa dur ve bana sor.
- **Matriks MCP:** yalnız Yorumcu ve Soru cevaplayan ölçümlerinde ve ölçülü (önbellek:
  aynı sorgu ölçüm içinde bir kez). Sonuçları depoya yazma. Erişilemiyorsa bağlamsız
  ölçümle devam et ve ölçüt 9'u açık bırak; uydurma.
- **Gerçek veri ve dil modeli:** gerçek veriyle yapılan ajan koşularının dökümleri
  `fixtures/local/` altında kalır. Depoya giren her örnek sentetik oturumdandır.
- **Alt ajanların birbirini görmemesi:** her ajan yalnız kendi `scratchpad/agents/<görev>/`
  klasörüne yazar (R5 dersi).
- **Disk (K56, K68):** ajan dökümleri hızla büyür; ölçüm bitince ham dökümler özet tabloya
  indirgenir ve silinir.
- **GitHub ve ana akış:** `CLAUDE.md` kuralları geçerli.
