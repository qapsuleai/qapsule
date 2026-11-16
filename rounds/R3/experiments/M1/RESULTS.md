# M1 — İşaret alfabesi: sonuçlar ve seçim (ön eleme)

Düzenek: `rounds/R3/experiments/harness/` (M1 bölümü; `71fd241`, alternatiflerden önce).
Alternatifler ayrı dallarda, ayrı alt ajanlarca, birbirini görmeden: **M1a `siluet`**
(`r3/m1a-siluet` `d9f70d8`), **M1b `rozet`** (`r3/m1b-rozet` `70caea9`), **M1c `iz`**
(`r3/m1c-iz` `b1a7b2e`; özgün dil, K18). Değerlendirme dalı `r3/eval-m1`. Hesap ölçümü
değerlendirme dalında yeniden koşuldu ve ajanların değerleriyle aynı çıktı. Okuma testleri
orkestratörce, alternatifler bittikten sonra; her alfabe için aynı tohumlar (1–3), koşul
başına 3 taze okuyucu (`claude -p --model sonnet`, yalnız `Read`, her okuyucu tek alfabe
görür). Alternatif ajanlar okuma testi koşmadı.

## (a) Hesapla ayırt edilebilirlik (ΔE76, σ = 0,5 px, ±1 px)

| Ölçü | siluet | rozet | iz |
|---|---:|---:|---:|
| **Ana ölçü:** en kötü koşulda K'nin en zayıf çifti | 4,60 (16/açık/gri) | 2,55 (16/açık/gri) | **7,55** (16/açık/protan) |
| 16 px açık normal görme: K en zayıf / ortalama | 7,16 / 21,10 | 3,74 / 7,88 | **10,70 / 21,85** |
| Kanal en zayıf: yön | 5,07 | 2,57 | **8,50** |
| Kanal en zayıf: aşama | 1,64 | 2,02 | **3,58** |
| Kanal en zayıf: güç | **2,37** | 2,27 | 2,21 |
| Kanal en zayıf: teyit | 1,16 | **2,17** | 1,19 |
| Kanal en zayıf: güven (karne) | **2,41** | 1,49 | 0,64 |
| Kanal en zayıf: sunucu işareti | **2,84** | 1,65 | 1,97 |
| Kutu dışı ilkel | 0 | 0 | 0 |

(ΔE ≈ 2,3 "zar zor fark edilir" eşiği olarak bilinir; bütün alfabelerin bazı ikincil
kanalları bu eşiğin altında.)

## (b) Okuma ve (c) öğrenme (taze görsel okuyucular; 3 okuyucu × 16 / 12 glif)

| Koşul | Alan | siluet | rozet | iz | şans |
|---|---|---:|---:|---:|---:|
| açıklamasız | tür | **0,88** | 0,85 | 0,67 | 0,17 |
| açıklamasız | yön | 1,00 | 1,00 | 1,00 | 0,33 |
| açıklamasız | tam doğru | 0,44 | **0,48** | 0,25 | 0,01 |
| kısa açıklamalı | tür | **1,00** | **1,00** | 0,92 | 0,17 |
| kısa açıklamalı | güç | 0,83 | **0,96** | 0,81 | 0,50 |
| kısa açıklamalı | tam doğru | 0,83 | **0,92** | 0,75 | 0,01 |
| öğrenme (5 örnek → 12 yeni) | tür | 0,75 | **0,94** | 0,75 | 0,17 |
| öğrenme | aşama | 0,94 | 0,86 | **1,00** | 0,20 |
| öğrenme | tam doğru | 0,44 | 0,44 | 0,44 | 0,01 |

Ham: `M1/raw/<alfabe>/{distinct.*,readers/,scores.*}`, günlük `M1/raw/readers-run.log`.

**Düzenek kusuru (dürüstlük notu):** açıklamasız ve açıklamalı sayfalarda bütün glifler
`final` aşamasında ve neredeyse hepsi aynı güçte çıktı (çoğunluk tabanı aşama 1,00, güç 0,98).
Bu iki koşulda aşama ve güç doğruluğu bilgi taşımaz; aşama yalnız öğrenme koşulunda ölçüldü.
Öneri: düzenekte kanal çeşitlemeleri aşamayı da dolaşsın (R3 sonrası; düzenek bu round'da
değiştirilmedi, çünkü alternatifler görüldükten sonra düzeneği değiştirmek kuralı çiğner).

## Seçim (öneri): **M1a `siluet`**; insan testinde `siluet` ile `rozet` karşılaştırılır

Önceden yazılmış bir birleştirme kuralı yoktu (SPEC yalnız ölçüleri tanımlıyor); kural sonuç
görüldükten sonra konduğu için açıkça yazılıyor:

1. Hiçbir ölçüde sonuncu olmayan aday tercih edilir. `iz` hesapta açık ara birinci ama
   okumada açıklamasız türde (0,67) ve tam doğruda (0,25) açık ara sonuncu: yeni bir dil
   öğrenilmeden okunmuyor. `rozet` okumada birinci ama hesapta sonuncu; ana ölçüsü (2,55)
   fark edilme eşiğine çok yakın ve 16 px normal görmede bile 3,74.
2. `siluet` her ölçüde ikinci ya da birinci; okumada `rozet`le arasındaki fark küçük
   örneklemde gürültü düzeyinde (açıklamalı tam doğru 40/48 vs 44/48), yalnız öğrenme-tür
   (27/36 vs 34/36) belirgin.

Bu bir **ön elemedir** (K39): okuyucu bir görsel dil modelidir, 32 px glif 4× büyütülerek
gösterildi, okuyucu sayısı küçük. Son söz insan testinde: `rounds/R3/human-test/` paketi
`siluet` ve `rozet`'i birlikte sınar; `iz` öğrenme süresi uzun bir dil olarak öneriler
bölümünde kalır. İnsan testi `rozet`'i öne çıkarırsa alfabe değişir; gramerdeki `glyph`
işareti alfabeden bağımsızdır (`Alphabet` trait'i), değişim tek satırdır.

## Bilinen zayıflıklar (seçilende)

`siluet`: teyit kanalı zayıf (1,16), aşama aday↔kesin sıkışmada zayıf (1,64), kesikli çizgi
kıvrık silüetleri parçalıyor, akış↔dönüş en yakın tür çifti, ✓/✗ rozeti 16 px'te çok küçük
(`M1/siluet/NOTES.md`). Bağımsızlık notu: `iz` ajanı paylaşılan geçici klasörde başka bir
ajanın betiğinin kendi betiğinin üstüne yazıldığını, onların dosyalarına bakmadığını bildirdi.
