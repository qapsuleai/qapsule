# RD1 — Tasarım: Alan Protokolü

Bu bir tasarım ve araştırma round'udur. Çıktısı çalışan bir ürün parçası değil, sonraki
bütün round'ların üzerine oturacağı sözleşmedir: motor, arayüz ve ajanların birbiriyle
nasıl konuştuğu.

## Neden

Görünümü JSON olarak tarif etmek yalnızca bir anın fotoğrafıdır. Oysa sistem sürekli
konuşur: kullanıcı kaydırır, fiyat akar, ajan önerir, kapsül kendini geçersiz ilan eder,
saat yalnızca bir işaret gösterebildiğini söyler. Gramer bu konuşmanın kelime dağarcığıdır;
bu round konuşmanın kendisini tasarlar.

`NORTH_STAR.md` ilke 2 ve AI-native testinin 4. ve 5. soruları bu round'un ölçüsüdür:
tek komut yolu; insan girdisi ajanın okuduğu aynı dokümana yazılır; ürün API'dir.

## Çalışma kararları (K21)

1. Dil modeli çizim döngüsünde yer almaz. Ajan anlam düzeyinde konuşur; motordaki
   deterministik **sunucu** (presenter) bunu gramere çevirir. Ajan grameri doğrudan da
   yazabilir. — *sınanacak*
2. Belge başına tek otorite ve sıralı işlem günlüğü; her yazı kökeniyle kaydedilir. — *sınanacak*
3. Motor hem sunucuda hem cihazda çalışır; doküman devredilebilir. — *sınanacak*
4. Arayüz ve ajan tavizsiz aynı protokolü kullanır. — **kesin**
5. Ajanlar varsayılan olarak öneri yazar; yetkiyle doğrudan yazar. — **kesin**
6. Protokol zaman çizelgesi ve dikkat yönlendirmesi taşır.

"Sınanacak" kararlar için alternatifler kurulur ve ölçülür. Kanıt aksini gösterirse karar
değiştirilmez; bulgu ve öneri rapora yazılır, karar ana akışta verilir.

## Protokolün taşıması gerekenler (alt sınır)

| Tür | Anlamı |
|-----|--------|
| Komut | Dokümanı değiştirme isteği. İnsan ve ajan aynısını gönderir. |
| Değişim | Dokümanın yalnızca değişen kısmı; sıralı ve yeniden oynatılabilir. |
| Sorgu | Anlamsal okuma: ekranda ne var, bir noktada ne var, bir hesabın bir aralıktaki değeri (K20). |
| Abonelik | Bir bulgu, bölge ya da koşul değiştiğinde haber. |
| Girdi | Dokunma, kaydırma, yakınlaştırma; etkileşim çekirdekte yaşar. |
| Öneri | Onay bekleyen yazı; kabul, ret, süre aşımı. |
| Dikkat | İki yönlü: kullanıcının baktığı yer; ajanın baktırmak istediği yer ve sırası. |
| Yetenek | Yüzeyin kendini tanıtması (saat, telefon, masaüstü, ses, ekransız); motor derinliği buna göre seçer. |

Ek olarak: sürümleme, hata modeli (ajanın hatadan düzeltme yapabileceği kadar bilgi),
yetki ve denetim izi, kimlik (kim yazdı), zaman (canlı güncelleme ve anlatım çizelgesi).

## Kapsam

1. Araştırma: var olan protokoller ve 2026–2030 bakışı.
2. Şartname v0: `docs/PROTOCOL.md`.
3. Mesaj tipleri (tek kaynak kuralı), JSON Schema, örnekler, uygunluk dökümleri.
4. Referans oturum: komut → değişim, sorgu, abonelik, öneri, günlükten yeniden oynatma.
5. Taşıma bağımsızlığı kanıtı: süreç içi ve satır tabanlı akış (`qp serve`).
6. Çelişkili kararların alternatifleriyle ölçümü.
7. Ajan yazabilirlik ölçümü.
8. Yetenek uzlaşması ve anlatım çizelgesi.
9. Gömme: MCP uyarlayıcısı kanıtı; A2UI ve MCP Apps için analiz.
10. R2'nin yeniden yazımı için girdi.

## Kapsam dışı

- Görsel gramerin tasarımı (R2). Bu round'da Görünüm, var olan v0 tipiyle opak bir yük olarak taşınır.
- Etkileşimin uygulanması, GPU, anlam işaretleri, kapsül davranışı, canlı veri bağlayıcıları.
- Ağ sunucusu, kimlik doğrulama altyapısı, kalıcı depolama. Yalnızca modelleri tasarlanır.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | `rounds/RD1/RESEARCH.md`: ajan–araç, ajan–arayüz, editör–sunucu, çok kullanıcılı doküman ve veri akışı protokollerinden en az sekizi incelenmiş (ör. MCP, MCP Apps, A2UI, AG-UI, LSP, bir CRDT ve bir OT sistemi, bir bildirimsel görselleştirme veri akışı). Her biri için: neyi çözüyor, bizde neyin karşılığı, ne alınır, ne alınmaz. Güncel belgelere bakılmış, bellekten yazılmamış. |
| 2 | "2026–2030" bölümü: protokolün dayandığı varsayımlar tek tek yazılmış (ör. modelin cihaza inmesi, ajanların arayüz üretmesi, ekranların çeşitlenmesi, çok ajanlı çalışma). Her varsayım için: gerçekleşmezse ya da tersine dönerse protokolün hangi kısmı değişir. |
| 3 | `docs/PROTOCOL.md` v0: sekiz mesaj türü, durum modeli, otorite, köken, yetki, yetenek, sürümleme ve hata modeli. Her tür en az bir örnekle. |
| 4 | Mesaj tipleri Rust'ta tek kaynaktan; JSON Schema üretiliyor; her tür için `fixtures/protocol/` altında geçerli ve geçersiz örnekler. |
| 5 | Uygunluk dökümleri: dilden bağımsız, "girdi mesajları → beklenen çıktı mesajları" biçiminde en az on iki senaryo. İleride yazılacak her kabuk ve her taşıma aynı dökümlerle sınanabilir. |
| 6 | Referans oturum dökümlerin hepsini geçiyor. İşlem günlüğünü baştan oynatmak dokümanı bayt düzeyinde aynı üretiyor. Geri alma çalışıyor. Öneri kabul/ret karneye ve kökene işleniyor. |
| 7 | Aynı dökümler iki taşımada geçiyor: süreç içi çağrı ve `qp serve` (standart girdi/çıktı üzerinden satır tabanlı). |
| 8 | Çelişkili kararlar ölçülmüş, her biri için alternatif kurulmuş: (a) eşzamanlılık — sıralı günlük ile en az bir CRDT yaklaşımı; senaryo: bir insan ve iki ajan aynı dokümana aynı anda yazıyor, biri çevrimdışı kalıp dönüyor; ölçüler: yakınsama, mesaj boyutu, kod karmaşıklığı, köken ve geri almanın korunması. (b) kodlama — JSON ile en az bir ikili biçim; sıcak yollar (seri parçaları, saniyede 60 değişim) için boyut ve süre, yerel ve WASM. (c) değişim biçimi — genel doküman farkı ile anlamsal işlemler. (d) devir — 10.000 barlı bir kapsül dokümanının sunucudan cihaza aktarımı: boyut, süre, cihazda canlı sürdürme. (e) sunucu ile ajanın grameri doğrudan yazması — gecikme, belirleyicilik ve tahmini model maliyeti. |
| 9 | Ajan yazabilirlik ölçümü: en az yirmi doğal dil görevi (ör. "RSI ekle", "son yarım saatte ne değişti", "bu bulguyu reddet ve nedenini yaz", "saat için en kısa halini ver"). Uygulamayı görmemiş, yalnızca `docs/PROTOCOL.md` ve şemayı okuyan taze alt ajanlar mesaj dizilerini üretiyor; diziler referans oturumda çalıştırılıyor. Ölçüler: ilk denemede başarı, hata iletisinden sonra düzeltme başarısı, mesaj ve belirteç sayısı. Başarısızlıklardan en az bir tasarım değişikliği çıkarılmış ve ölçüm tekrarlanmış; önce/sonra tablosu var. |
| 10 | Yetenek uzlaşması gösterilmiş: aynı bulgu için saat, telefon ve masaüstü yüzey bildirimleri üç farklı derinlikte cevap alıyor. Anlatım çizelgesi şartnamede ve en az bir dökümde. |
| 11 | MCP uyarlayıcısı: protokolün komut ve sorguları şemadan üretilen MCP araçları olarak sunuluyor; en az üç döküm MCP üzerinden geçiyor. A2UI ve MCP Apps için: protokol bunların üzerinde nasıl taşınır, yazılı analiz. |
| 12 | `rounds/RD1/R2_INPUT.md`: gramerin protokole karşı karşılaması gerekenler; R2 için önerilen kapsam; ana akışın vermesi gereken kararlar. |
| 13 | `REPORT.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md`, `docs/CONCEPTS.md`, `docs/ARCHITECTURE.md` güncel; önerilen karar satırları `DECISIONS.md`'ye yazılmış. |
