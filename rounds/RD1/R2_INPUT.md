# RD1 → R2 girdisi

R2 (görsel gramer v0 ve headless çizim) RD1'den sonra yeniden yazılacak. Bu belge,
gramerin protokole karşı karşılaması gerekenleri, R2 için önerilen kapsamı ve ana akışın
vermesi gereken kararları toplar. Dayanak: `docs/PROTOCOL.md` (`qp/0`), `RESEARCH.md`,
`experiments/`, `ergonomics/`.

## 1. Gramerin protokole karşı karşılaması gerekenler

RD1'de Görünüm (`View`) v0 tipiyle **opak bir yük** olarak taşındı. R2'nin gramerinin
protokolde şu sözleşmeleri tutması gerekir:

| # | Gereksinim | Neden (protokoldeki karşılığı) |
|---|------------|--------------------------------|
| G1 | **Kararlı, adreslenebilir parçalar.** Gramerdeki her katman, ölçek ve işaret dizi sırasıyla değil kimlikle adreslenebilmeli (`layer.id`). | v0'da `add_layer`/`remove_layer` dizi indeksiyle çalışıyor; eşzamanlı yazıda indeks kayar (T6c bulgusu, JSON Patch'in de zayıflığı). `insert_layer`/`delete_layer` değişimleri kimliğe geçmeli. |
| G2 | **Anlamsal işlemler gramer düzeyinde de olmalı.** "RSI'yı RSI(14)'ün paneline koy" (`show.with`) gibi işlemler grameri bilmeden yazılabilmeli; gramerin tam ağacını yazmak isteğe bağlı kalmalı. | K21-1: ajan anlam düzeyinde konuşur, sunucu gramere çevirir. Ajan yazabilirlik ölçümünde anlamsal işlemler daha yüksek ilk deneme başarısı verdi (bkz. `ergonomics/RESULTS.md`). |
| G3 | **Her işaret bir olguya ya da seriye bağlanır** (ilke 6). Gramer düğümleri `series`/`fact`/`finding` kimliklerine başvurur; başvuru otoritede denetlenir (`not_found`, `in_use`). | Silmede bağımlılık (`remove` → `in_use`), sunumda kanıt (`present.facts`). |
| G4 | **Derinlik başına alt küme.** Gramer, aynı bulgunun `sentence`, `capsule`, `chart` derinliklerinde neyi çizdiğini tanımlamalı; yüzey profili (`max_points`, `max_glyphs`, `series_data`) gramerin indirgeme kuralı olmalı. | Yetenek uzlaşması (§10). R2'de sunucu (presenter) gramer kurallarına taşınır. |
| G5 | **Deterministik kurulum.** Aynı doküman → aynı görünüm baytları (sunucu) ve aynı çizim (altın görsel). Gramer içinde saat, rastgelelik, yerel ayar bağımlılığı yok; biçimlendirme (`display_tz`) yalnız gösterimde. | Yeniden oynatma bayt eşitliği; T6e belirleyicilik ölçümü. |
| G6 | **Görünür aralık iki düzeyde.** Kalıcı aralık (`x.domain`, komutla) ile geçici bakış (varlık durumu, girdiyle) gramerde ayrı kavramlar olmalı. | §4.6, §7. Girdi günlüğü şişirmez. |
| G7 | **Hedeflenebilirlik.** Dikkat yönergesi ve anlatım adımı `Target {view, panel, series, item, from_bar, to_bar}` ile işaret eder; gramer bu hedefi çizebilmeli (vurgu, odak, karartma). | §4.8, §11. |
| G8 | **Anlamsal okuma.** `screen` sorgusu görünümü panel/seri/işaret olarak özetler; gramerin her düğümü bu özete indirgenebilmeli (ekransız çalışır, ilke 9). | AI-native testi 1 ve 4. |
| G9 | **Şema tek kaynak.** Gramer tipleri `qp-schema`'da; JSON Schema ve MCP araçları ondan üretilir; gramerin her yapı taşı için geçerli/geçersiz örnek ve uygunluk dökümü. | `fixtures/protocol/`, `qp-mcp` araç üretimi. |
| G10 | **Ek değişiklikle büyüme.** Yeni işaret türü = yeni enum değeri (ek); var olan alanın anlamı değişmez. Otorite tanımadığını reddeder (`schema` + `did_you_mean`). | §12 sürümleme. |

## 2. R2 için önerilen kapsam

1. Gramer v0 tipleri (`qp-schema`): katman kimlikleri (G1), işaret/ölçek/kodlama, panel,
   derinlik başına indirgeme (G4).
2. Protokol `qp/0` eki: `insert_layer`/`delete_layer` kimlikle; `add_layer` `index` yerine
   `after`/`before` kimliği. (Kırıcı değil: yeni alanlar; eski indeks alanı v0'da kalır,
   `qp/1`'de kalkar.)
3. Sunucuyu (presenter) gramer kurallarına taşımak: `show_finding` ve `present` aynı
   gramerle; altın görseller derinlik başına (yeni görünüm → yeni altın görsel).
4. Headless çizim: `View` → PNG, `qp-render` üzerinden; tarayıcı eşitliği (R0 düzeneği).
5. Uygunluk dökümleri: gramer işlemleri için en az altı yeni döküm; MCP üzerinden üçü.
6. Ajan yazabilirlik: gramer işlemleri için yeni görev kümesi; aynı düzenek
   (`ergonomics/harness`).

Kapsam dışında kalması önerilen: etkileşimin uygulanması (girdi R4), GPU yolu, anlam
işaretlerinin tasarımı (R3).

## 3. Ana akışın vermesi gereken kararlar

(RD1 ölçümlerinden sonra güncellendi; `REPORT.md` "Önerilen karar satırları" ile aynı.)

| # | Karar | Seçenekler | RD1 önerisi |
|---|-------|------------|-------------|
| D1 | K21-1 (dil modeli çizim döngüsünde değil, sunucu) | koru / değiştir | T6e'ye göre |
| D2 | K21-2 (tek otorite + sıralı günlük) | koru / CRDT / hibrit | T6a'ya göre |
| D3 | K21-3 (motor iki uçta, devir) | koru / yalnız sunucu | T6d'ye göre |
| D4 | Kodlama | yalnız JSON / JSON + pazarlıklı ikili | T6b'ye göre |
| D5 | Değişim biçimi | anlamsal / genel fark | T6c'ye göre |
| D6 | Mesaj tiplerinin yeri | `qp-schema::protocol` (RD1) / ayrı `qp-protocol` crate | RD1: `qp-schema` (tek şema kaynağı kuralı) |
| D7 | Katman adresleme | indeks / kimlik | kimlik (G1) |
| D8 | Bar numarası | 0 tabanlı sıra (v0) / zaman / ikisi | ergonomi sonucuna göre |
