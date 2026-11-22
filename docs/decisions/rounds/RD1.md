# Karar Günlüğü — RD1 kararları — kabul edildi (2026-11-15, K22); Ö9 K23 ile geçersiz

<!-- dizin: DECISIONS.md; bu dosyadaki satırlar RG1 bölünmesinde aynen taşındı -->

## RD1 kararları — kabul edildi (2026-11-15, K22); Ö9 K23 ile geçersiz

RD1 bu satırları **önerir**; K21'deki "sınanacak" kararlar round içinde değiştirilmedi.
Kanıt `rounds/RD1/REPORT.md` ve `rounds/RD1/experiments/`.

| # | Önerilen karar | Kanıt |
|---|----------------|-------|
| Ö1 | **K21-1 korunur:** dil modeli çizim döngüsünde yer almaz; ajan anlam düzeyinde yazar (`show_finding`), deterministik sunucu görünümü kurar. Ajanın grameri doğrudan yazması (`put_view`) yeni görünüm türleri için açık kalır; bulgu başına bir kez yazılır ve belgede saklanır. | T6e: sunucu 125 ns / 6,6 µs, tek çıktı; 10 model örneği 10/10 farklı, ~9 s, olguya dayanmayan öğeler; ölçekte Sonnet ~10–25 bin $/gün. |
| Ö2 | **K21-2 korunur:** belge başına tek otorite ve sıralı günlük. İnceltmeler: katmanlara kimlik ve katman başına çakışma denetimi; alan düzeyinde `base`; gönderenin kendi girdileri çakışma sayılmaz; yeniden bağlanma değişim akışı ya da anlık görüntüyle. | T6a: 30 koşuda otorite 0 anlamsal bozulma; Automerge/Loro 1.000 işlemlik çevrimdışı pencerede 33–50 sessiz bozulma; Loro geri alması başka yazarın değerini 10k işlemde 7–8 kez ezdi. CRDT'ler yeniden bağlanma boyutu ve geçmiş boyutunda üstün. |
| Ö3 | **K21-3 inceltilir:** motor sunucuda ve cihazda aynı kodla çalışır (bit-eşit); **varsayılan** otorite sunucuda, cihaz hesapları kendisi yapan bir yansıdır. Tam devir yalnız başka yazar ve canlı ajan yokken, süreli kira ile. | T6d: yerel = WASM özetleri; B+ 15 KB/s, 0,24 µs/tik sunucu işi; epoch kuralı bölünmeyi birleştirmiyor. |
| Ö4 | **Kodlama: JSON kanonik**, `float_roundtrip` zorunlu; ikili biçim eklenmez. Sıcak yolda akış sıkıştırması. | T6b: deflate ile tik 33–35 B tüm biçimlerde; ikinci serde biçimi WASM'ı ~2× büyütüyor; `float_roundtrip` kapalıyken MACD değerlerinin %11'i bozuluyor. |
| Ö5 | **Değişim biçimi anlamsal:** istek `Op`, günlük ve tel `ChangeOp`; JSON Patch yalnız dışa aktarım/hata ayıklama. | T6c: diff hesabı ~2.400× pahalı; eşzamanlılıkta 0/18 yanlış (`base` ile) vs 10/18. |
| Ö6 | Protokol mesaj tipleri `qp-schema::protocol`'da (tek şema kaynağı); referans otorite `qp-session`, MCP uyarlayıcısı `qp-mcp` (araçlar şemadan üretilir). | RD1 uygulaması; `verify.sh` 13/13. |
| Ö7 | Katmanlar R2'de kararlı kimlik taşır; katman işlemleri kimlikle. | T6c: `base`'siz yanlış sonuçların hepsi indeksli `remove_layer`'dan. |
| Ö8 | Anlamsal referans (kimlik yerine anlamla başvuru) ve `welcome`'da doküman özeti protokolün bir sonraki tasarım adımıdır. | Ajan yazabilirlik v2r: ilk deneme 6/20 (kimlik keşfi), düzeltme 13/14. |
| Ö9 | Bar verisi için JSON içinde sütunlu biçim değerlendirilir (doküman şemasını ve `qp-render`'ı etkiler; ayrı karar). | T6b: gzip'te ikili biçimlerden küçük, çözme 2,6× hızlı. |

| # | Açık karar | Ne zaman | Not |
|---|------------|----------|-----|
| A10 | Devir protokolünün tamamı: kira, fencing, `(epoch, seq)` sürekliliği, parçalı anlık görüntü | R4 öncesi (cihaz) | T6d "Öneriler". |
| A11 | Öğe başına `base` (çevrimdışı biriken komutların kendi kendisiyle çakışmaması) | `qp/1` | T6d: tek `base` ile 20 komuttan 1'i, öğe başına 19'u uygulandı. |
| A12 | MCP olay kanalı: `notifications/message` (2026-07-28'de kullanımdan kalktı) yerine `subscriptions/listen` + `log` | MCP barındırıcısıyla ilk gerçek kullanımda | `crates/qp-mcp/NOTES.md`. |
