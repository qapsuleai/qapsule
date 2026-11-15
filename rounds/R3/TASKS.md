# R3 — Alt Görevler

Bu round büyüktür; iki evrede yürür. Birinci evre makineyi kurar, ikinci evre ölçümle
tasarımı seçer. İkinci evrenin düzenekleri birinci evre bitmeden yazılabilir.

## Evre 1 — makine

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| T1 | Sözlük ve tasarım: `docs/CAPSULE.md`'yi uygulanabilir ayrıntıya indir (çerçeveyi değiştirmeden); şema taslağı; yirmi ajan görevi | `docs/`, `rounds/R3/tasks/` | — |
| T2 | Veri: rejimli sentetik üreteç; sentetik lisanslı akış beslemesi; `scripts/fetch-bars.sh` (`docs/DATA_ACCESS.md`'ye göre) ve `fixtures/local/` okuyucusu. İlk çekimde zaman birimi, `time` alanının anlamı, hız sınırı ve düzeltilmiş/düzeltilmemiş farkı küçük isteklerle doğrulanır ve `docs/DATA_ACCESS.md`'ye yazılır | `crates/qp-source/`, `scripts/`, `fixtures/`, `docs/DATA_ACCESS.md` | — |
| T3 | `qp-facts`: dedektör arayüzü ve altı türün dedektörleri; toplu = akışkan | `crates/qp-facts/`, gerekiyorsa `qp-compute`'a küçük ek hesap | T1 |
| T4 | Bulgu, kanıt aileleri, teyit; yaşam döngüsü durum makinesi; aday | `crates/qp-session/` ya da `crates/qp-capsule/` (karar raporda) | T3 |
| T5 | Karne; sonuçlandırma; ileriye bakma testi | T4 ile aynı crate | T4 |
| T6 | Protokol eki: kapsül sorguları ve abonelikleri, itiraz, geri çekme, hassasiyet, sunucu olgusu işareti | `crates/qp-schema/`, `docs/PROTOCOL.md` | T1 |
| T7 | Gramer eki: işaret türü; kapsül görünümü; derinlik kuralları; kanıt geometrisi | `crates/qp-schema/`, `crates/qp-grammar/`, `crates/qp-render/`, `docs/GRAMMAR.md` | T1 |

## Evre 2 — ölçüm ve seçim

| # | Görev | Yazdığı yer | Bağımlılık |
|---|-------|-------------|------------|
| H | Düzenekler: M1–M7 için ortak ölçüm düzenekleri, alternatiflerden önce ve bağımsız | `rounds/R3/experiments/harness/` | T2 |
| M1a–c | İşaret alfabeleri: üç tasarım, üç alt ajan, birbirini görmeden | dal başına, `rounds/R3/experiments/M1/` | T7, H |
| M2a–b | Cümle üretimi | dal başına | T4, H |
| M3a–c | Önem sıralaması | dal başına | T4, T5, H |
| M4a–b | Tema kimliği | dal başına | T4, H |
| M5a–b | Sonuç tanımı | dal başına | T5, H |
| M6a–b | Sunucu olgusu güven yolu | dal başına | T6, H |
| M7 | İtiraz ve geri çekme; RP2 düzeneğiyle | dal | T6, H |
| S | Seçimlerin bütünleştirilmesi; altın görseller; dökümler; hassasiyet benzetimi | bütünleştirme dalı | M1–M7 |
| E | Ajan yazabilirlik (kapsül görevleri) | `rounds/R3/ergonomics/` | S |
| HT | İnsan testi paketi | `rounds/R3/human-test/` | S |
| A | Denetim: bağımsız alt ajan; ölçümleri yeniden üretir; kendi geri çıkarma saldırısını ve kendi ileriye bakma testini yazar; görselleri gözle inceler | `rounds/R3/AUDIT.md` | hepsi |

## Notlar

- **Alternatifler gerçek olmalı.** Her biri savunucusu tarafından yazılmış gibi kurulur.
  Zayıf kurulmuş bir alternatifi yenmek kanıt değildir.
- **Düzenek önce.** Hiçbir ölçüm düzeneği, ölçtüğü alternatifleri gördükten sonra yazılmaz.
- **Bilinen yöntemlerle sınırlı değilsin (K18).** Özellikle işaret alfabesi ve sunucu
  olgusu güven yolunda yeni bir şey tasarlaman bekleniyor.
- **Dedektörler sade kalsın.** Amaç formasyon kütüphanesi değil, altı türün uçtan uca
  çalışması. Dedektör var olan beş hesap ve fiyat/hacim ilkelerinden kurulur; gereken
  küçük ek hesaplar (ör. kayan en yüksek/en düşük) `qp-compute`'a aynı kurallarla eklenir
  ve hesap özeti genişletilir, var olan özet değişmez.
- **Dil modeli nerede var, nerede yok.** Ürünün içinde yok: kapsül deterministik motordan
  çıkar. M2'nin ikinci alternatifi dil modelini yalnızca karşılaştırma için dener. M1 ve
  M2'de okuyucu olarak, ajan yazabilirlikte kullanıcı olarak yer alır.
- **Dürüst sınırlar.** Görsel okuyan alt ajan insan gözünün yerine geçmez; sentetik veride
  karne bir başarı oranı kanıtı değildir. İkisi de raporda açıkça yazılır.
- **Gerçek veri (varsa)** yalnızca `fixtures/local/` altından okunur; içeriği, türevleri
  ve ondan üretilen görseller commit edilmez. Rapora yalnızca toplu istatistik girer.
- **Görseller:** var olan altın görseller güncellenmez; yenileri eklenir.
- **GitHub ve ana akış:** `CLAUDE.md` kuralları geçerli.
