# R1 — Hesap Çekirdeği

## Hedef

Ajanın "hesaplayarak düşünebileceği" deterministik, akışkan ve hızlı hesap çekirdeğini
kurmak; R0'da bırakılan sapmaları kapatmak; kodu ilk kez Linux üzerinde doğrulamak.

## Kapsam

1. Uzak depo ve CI'ın ilk gerçek çalışması (Linux x86_64).
2. Şema düzeltmeleri: tek bar/seri tipi, ısınma kuralı (K13), Enstrüman ve Takvim (K14),
   doğrulamanın `qp-schema` kütüphanesine taşınması.
3. `qp-compute`: `Study` arayüzü ve eski beş indikatörün yeniden yazımı.
4. Parametre taraması ve ölçüm.
5. `SeriesSource` arayüzü, dosya ve sentetik kaynak, `qp compute` komutu.
6. Takvime göre bar hizası.
7. Çizicinin hesap çekirdeğine bağlanması.
8. Hesap çıktılarının mimariler arası bit eşitliği.

## Kapsam dışı

- Görsel gramer, GPU çizimi, LOD (R2). Olgu dedektörleri ve kapsül davranışı (R3).
- Canlı veri bağlayıcıları (R6). Beş indikatörün ötesinde yeni indikatör.
- Tatil takvimlerinin tam verisi; yalnızca mekanizma ve birkaç örnek gün.

## Tasarım kuralları

- Tüm değerler `f64`. Eski `f32` tipleri kopyalanmaz.
- Bir hesap birden çok girdi alanı (açılış, yüksek, düşük, kapanış, hacim ya da başka
  bir hesabın çıktısı) okuyabilir ve birden çok adlandırılmış çıktı üretebilir.
- Çıktı uzunluğu her zaman girdi bar sayısına eşittir; ısınma dönemi `null` (K13).
- Üç çalışma biçimi aynı sonucu verir: toplu, bar bar akışkan ve "son barı düzelt"
  (canlı barda fiyat değiştikçe son değerin yeniden hesaplanması). Yeni bar O(1).
- Hesapta LLM, ağ, saat ve rastgelelik yok. Sonuç donanımdan bağımsız bit-eşit olmalı;
  bunu bozan derleyici davranışlarından (örneğin birleşik çarp-topla) kaçınılır ve
  testle kanıtlanır.
- Bar hizası enstrümanın takvimine göredir; gösterim saat dilimi hesaba girmez (K14).

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | Depo uzak sunucuda; CI Linux x86_64 üzerinde `scripts/verify.sh` çalıştırdı. Altın görseller ve tarayıcı adımı dahil sonuç raporlandı. Fark çıktıysa nedeni bulundu ve giderildi ya da K12 güncellendi. Depo adresi verilmediyse bu ölçüt "yapılamadı" olarak raporlanır, round'un geri kalanı sürer. |
| 2 | `qp-render` kendi bar tipini kullanmıyor; `fixtures/bars` biçimi üretilen JSON Schema ile doğrulanıyor; şema doğrulaması `qp-schema` içinde, CLI ince. |
| 3 | SMA, EMA, RSI, MACD (3 çıktı) ve Bollinger (4 çıktı) `Study` olarak yazılmış. Her biri için: elle hesaplanmış küçük bir örnekle ve eski kodun toplu sonuçlarıyla karşılaştırma (tolerans 1e-9, sapma varsa nedeni açıklanmış). |
| 4 | Özellik testleri: toplu = akışkan = "son barı düzelt"; çıktı uzunluğu = bar sayısı; ısınma değerleri `null`, sonrası sonlu. |
| 5 | Parametre taraması: tek çağrıda N parametre kümesi. Ölçüm raporlanmış: 10.000 bar × 1.000 varyasyon, beş indikatörün her biri için, yerel ve WASM. Hedef: yerel tek çekirdekte 50 ms'nin, WASM'da 150 ms'nin altı. Tutmayan varsa nedeni ve önerisi yazılmış. |
| 6 | `SeriesSource` arayüzü ile dosya ve sentetik kaynak çalışıyor; `qp compute` bir doküman ya da seri dosyasından hesap çıktısını JSON olarak veriyor. |
| 7 | Takvim: BIST ve NYSE için gün içi, günlük, haftalık ve aylık bar sınırları doğru; yaz saati geçiş günleri ve yarım gün örneği testte. Saat dilimi verisinin WASM boyutuna etkisi ölçülmüş. |
| 8 | Çizicideki SMA `qp-compute` çıktısından geliyor; altın görseller bayt düzeyinde değişmedi (değiştiyse nedeni açıklanmış ve onay için rapora yazılmış, kendiliğinden güncellenmemiş). |
| 9 | Hesap çıktılarının özeti aarch64, x86_64 ve wasm32 üzerinde aynı; test `verify.sh` içinde. |
| 10 | `REPORT.md`, `AUDIT.md` yazılmış; `STATUS.md`, `ROADMAP.md`, gerekiyorsa `docs/CONCEPTS.md` güncellenmiş. |
