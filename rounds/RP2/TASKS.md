# RP2 — Alt Görevler

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T1 | Ölçüm tabanı: bugünkü oturumun tik maliyeti (5/50/500 hesap, yerel ve WASM), anlık görüntü boyutu | `rounds/RP2/BASELINE.md` | — |
| T2 | Veri bağı tasarımı: sözlük, şema, şartname; dondurulmuş kanıt başvurusu; hata türleri | `docs/`, `crates/qp-schema/` | — |
| T3 | Veri kaynağı olayları (alan düzeyinde güncelleme) ve sentetik kaynak; veri sağlayıcı arayüzü | `crates/qp-source/` | T2 |
| T4 | Oturumda veri bağı çözümleme; anlık görüntünün barsız hali | `crates/qp-session/` | T2, T3 |
| T5 | Artımlı canlı yol; etkilenenleri bulma; özellik testleri; ölçüm | `crates/qp-session/`, gerekirse `crates/qp-grammar/` | T4 |
| T6 | Tutarlılık araştırması: literatür (akış işleme, filigran, olay zamanı; dağıtık veritabanlarında okuma tutarlılığı) ve aday yaklaşımlar | `rounds/RP2/RESEARCH.md` | — |
| T7a–c | Tutarlılık yaklaşımları: her biri ayrı dalda, ayrı alt ajanla, birbirini görmeden; ortak senaryo düzeneğiyle ölçüm | `rounds/RP2/experiments/`, dal başına kod | T5, T6 |
| T8 | Seçilen yaklaşımın bütünleştirilmesi; otorite ↔ yansı (yerel ↔ WASM) testi | `crates/qp-session/`, `crates/qp-wasm/` | T7 |
| T9 | K25 kararları ve R2 kalanları | `crates/qp-session/`, `crates/qp-grammar/`, `docs/` | T5 |
| T10 | Dökümler, şartname son hali | `fixtures/protocol/conformance/`, `docs/PROTOCOL.md` | hepsi |
| T11 | Denetim: bağımsız alt ajan; ölçümleri yeniden üretir, tutarlılık senaryolarını kendi yazdığı bir düzenle dener | `rounds/RP2/AUDIT.md` | hepsi |

## Notlar

- **Tutarlılık senaryo düzeneği** T7'den önce, yaklaşımlardan bağımsız yazılır; üç
  yaklaşım aynı düzenekle ölçülür. Düzenek belirleyicidir: gecikme ve sıra bozulması
  tohumlu bir zamanlayıcıdan gelir, gerçek saatten değil.
- **Alternatifler gerçek olmalı:** her yaklaşım savunucusu tarafından yazılmış gibi en iyi
  haliyle kurulur.
- **Bilinen yöntemlerle sınırlı değilsin (K18).** Akış işleme ve dağıtık sistem
  literatürü başlangıç noktasıdır; finansal bulgunun "kanıtıyla birlikte doğru zamanda
  gösterilmesi" ihtiyacını karşılamıyorsa yenisini tasarla.
- **Uyumluluk:** protokol hâlâ `qp/0` ve yayımlanmadı. K23 ve K25 gereği davranışı
  değişen dökümler güncellenebilir; her biri raporda gerekçesiyle listelenir. Gerekçesiz
  döküm değişikliği yapılmaz.
- **Test verisi:** belge içi bar taşıyan örnekler bir kaynak türü olarak çalışmaya devam
  eder; altın görsellerin dayandığı dökümler bu yüzden değişmeden çizilebilmelidir.
- **Güvenlik:** veri bağı dış girdidir; aralık ve boyut sınırları, yol hapsi ve yetki
  denetimi şartnamede ve uygulamada yer alır.
- **GitHub:** `CLAUDE.md` kuralları geçerli (`scripts/qgh`, `scripts/preflight.sh`).
