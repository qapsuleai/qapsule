# E3 — İş parçacıkları

Dal: `rp1/e3-threads` (taban `caab6e3`). Yürüten: orkestratör.

## Hipotezler (denemeden önce yazıldı)

**H3.1.** Taramanın varyasyonları (A) ve B'nin serileri bağımsız; çekirdek sayısıyla neredeyse
doğrusal ölçeklenir. M4 Max'te 12 performans çekirdeği → 8–10 kat; 4 vCPU'lu CI'da ≈ 3,5 kat.

**H3.2.** Sınır bellek bant genişliği olur: profil A çıktıları sütun olarak yazıyor (Bollinger
1.000 × 4 × 10k × 8 B = 320 MB/tarama). Çıktıyı yazmayıp varyasyon başına indirgeyen bir
biçimde ölçek daha iyi olur.

**H3.3.** Determinizm: her varyasyonun sonucu yalnızca kendi girdisine bağlı; iş parçacıkları
arasında bölüşüm sonucu değiştirmez. Ziyaretçi (visitor) sırası korunmalı: ya çıktılar
toplanıp sırayla verilir (bellek: tüm varyasyonlar) ya da API varyasyon başına bir indirgeme
(`R: Send`) alır ve sonuçları indeks sırasıyla döndürür. İkincisi ajan sorusuna ("hangi
parametre en iyi") uygun.

**H3.4.** wasm32-unknown-unknown'da iş parçacığı yok (SharedArrayBuffer + özel derleme
gerekir); tasarım tek iş parçacığına düşmeli. K17 hedefleri tek çekirdek için; iş parçacığı
hedefe sayılmaz, yalnızca ek pay.

## Deney 3.1 — `par::sweep_reduce_par`

**Yapılan.** `crates/qp-compute/src/par.rs`: parametre kümeleri iş parçacığı sayısı kadar
bitişik bloğa bölünür; her iş parçacığı kendi bloğunda olağan taramayı (`sweep_each`) koşar
ve her varyasyonu hemen `reduce(&SweepColumns) -> R` ile indirger (sütun tutulmaz); sonuçlar
blok sırasıyla birleştirilir → varyasyon sırası korunur. `std::thread::scope`; yeni bağımlılık
yok (rayon gerekmedi: iş bölümü statik ve eşit). wasm32'de iş parçacığı sayısı 1'e düşer
(wasip1 derlemesi ve Node'da koşu doğrulandı). Geçersiz girdi işten önce reddedilir.

**Determinizm kanıtı.** `tests/par.rs`: beş hesap × tek sayıda varyasyon × iş parçacığı
0, 1, 2, 3, 7, 16, 1.000 → her varyasyonun tüm çıktılarının özeti sıralı taramayla aynı, aynı
sırada.

**Ölçüm** (M4 Max, 12P+4E; yük ortalaması ≈ 3,4; profil A; medyan ms, 5 koşu):

İndirgeme = son değer (yazılan sütun okunmaz):

| Hesap | 1 | 2 | 4 | 8 | 12 | 16 |
|-------|--:|--:|--:|--:|---:|---:|
| SMA | 15,1 | 8,1 | 4,3 | 2,5 | 2,3 | 2,2 |
| EMA | 8,7 | 5,4 | 2,9 | 2,1 | 1,8 | 1,6 |
| RSI | 24,2 | 12,8 | 8,3 | 5,1 | 3,8 | 3,5 |
| MACD | 41,7 | 21,5 | 11,3 | 6,1 | 6,0 | 5,6 |
| Bollinger | 40,7 | 21,4 | 14,1 | 8,0 | 11,4 | 7,9 |

İndirgeme = her çıktıyı oku (FNV özeti; ağır okuma):

| Hesap | 1 | 2 | 4 | 8 | 12 | 16 |
|-------|--:|--:|--:|--:|---:|---:|
| SMA | 97,5 | 48,5 | 24,8 | 12,7 | 11,3 | 12,1 |
| RSI | 104,5 | 53,5 | 27,4 | 14,2 | 13,0 | 13,1 |
| MACD | 260,0 | 130,0 | 72,1 | 37,1 | 29,0 | 26,6 |
| Bollinger | 330,9 | 165,0 | 92,2 | 47,2 | 32,1 | 33,9 |

**Değerlendirme.** 4 iş parçacığına kadar neredeyse doğrusal (3,5–3,9×), 8'de 6–7×, 12–16'da
7–8×'de doyuyor (4 verimlilik çekirdeği, paylaşılan önbellek/bellek; ölçüm sırasında makinede
başka iş vardı). Bellek bant genişliği sınırı bu boyutta görünmüyor: çıktı sütunu her
varyasyonda yeniden kullanılan, önbellekte kalan bir tamponda.

**Karar: al (ek API).** K17 hedefleri tek çekirdek için; iş parçacığı onların yerine geçmez.
Masaüstü/sunucuda "binlerce varyasyon" sorusunu 4–8 kat kısaltır. WASM'da tarayıcı iş
parçacığı (SharedArrayBuffer, COOP/COEP başlıkları, `wasm-bindgen-rayon` benzeri) ayrı bir
kabuk kararı — öneri.
