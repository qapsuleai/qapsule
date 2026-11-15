# M6 — Sunucu olgusu güven yolu: sonuçlar ve seçim

Düzenek `harness-sim` (`176f77e`, alternatiflerden önce). Alternatifler ayrı dallarda, ayrı
alt ajanlarca, birbirini görmeden: **M6a `coarse`** (`r3/m6a-coarse` `1258edc`, `8586bfd`),
**M6b `muhur`** (özgün tasarım, K18; `r3/m6b-novel` `4892cf6`, `cdf0916`). Değerlendirme dalı
`r3/eval-sim`; özetler ham kayıttan yeniden puanlandı (bayt eşit). Tam koşum 20–52 dk
sürdüğü için yeniden koşumu denetçiye bırakıldı. Veri yalnız sentetik lisanslı akış (yerelde
gerçek kurumsal akış yok). 8.406 hedef bar (3 tohum).

## Ham sonuçlar (havuz; denetim `echo` = her olguya bir ileti)

| Ölçü | coarse | muhur | echo |
|---|---:|---:|---:|
| Kâhin saldırısı: genişlik/yayılım medyan · p10 | 1,69 · 0,40 | 1,56 · 0,43 | 1,17 · 0,20 |
| Kâhin: bit p90 (düzenek sütunu) | 1,33 | 1,22 | 2,34 |
| Kâhin: bit p99 · en çok (tüm ızgara tutulan hedefler hariç) | **2,20 · 3,98** | 2,73 · 5,42 | 4,42 · 8,04 |
| Çok parçalı aralık | 0 | 790 | 56 |
| Gerçekçi saldırgan: sonsal genişlik medyan (önsel 0,549) · kapsama | 0,586 · 0,835 | **0,851** · 0,853 | 1,083 · 0,889 |
| İleti sayısı | 21.901 | **9.503** | 8.136 |
| Gösterim gecikmesi | 1 bar | **0** | 0 |
| trust=server; rakam (metin/kod/ek); TR/EN ifade | 1; 0/0/0; 1/1 | 1; 0/0/0; 1/1 | 1; 0/0/0; 1/1 |
| Fayda (en az bir ileti alan lisanslı olgu) | **0,843** | 0,826 | 1,000 |
| Yapı gereği sınır | yok (deneysel) | **≤ log2 7 ≈ 2,81 bit/hedef bar, her önsel için, anahtarı bilen saldırgana karşı da** | yok |
| "Sunucu doğruladı" işaretinin sınanabilirliği | metin | **HMAC makbuzu + yetkili denetçinin `replay`'i** | metin |

(Düzenekteki "en çok 10,41" iki alternatifte de aynı: bütün ızgarası tutulan, iletilerin
hiçbir şey söylemediği küçük |net| hedefi; ızgaranın |net|'e göre ölçeklenmesinin yapay
sonucu.)

## Seçim: **M6b — mühür**

- K38 "nitel yorumlardan ham değerin geri çıkarılamadığı **kanıtlanır**" diyor: mühürde sınır
  yapı gereği (ham değer gelir gelmez işaret + ikili büyüklük kovasına indirgenir; konuşan
  fonksiyon ham değeri görmez; tür imzası bunu zorlar) ve testle sınanıyor (kova içi her değer
  bayt eşit döküm; 1e-14…1e25 taramasında en çok 7 farklı döküm). `coarse`'un sınırı deneysel.
- İşaret sınanabilir: makbuz (sunucu anahtarıyla HMAC; iletinin zaten söylediğine dayanır,
  sızdırmaz) ve yetkili denetçi için ham değerlere bağlı denetim günlüğü.
- Gecikme 0 ve yarı yarıya az ileti (dikkat bütçesi).
- `coarse` deneysel kuyrukta daha iyi (en çok 3,98 vs 5,42 bit; çok parçalı aralık yok) ve
  faydası biraz yüksek; bu fark raporda açık.

## Sınırlar

- Yalnız sentetik akış; gerçek kurumsal akışta ölçülmedi.
- Sınır hedef bar başınadır; çok barı birleştiren saldırgan bar başına 2,81 bite kadar
  kazanabilir. Genişlik/yayılım ölçüsü dar bir kovada 5,42 bite çıkabiliyor.
- Lisanslı olguların ~%17'si ileti almıyor; iletilerin ~%29'u dedektör olgusu olmayan barda:
  ileti mühürün kendi kuralını söylüyor, dedektörün olgusunu değil (ürün kararı: aynı kapsül mü?
  S'de: ileti kapsülün yorumu olarak yalnız olgulu barda; öbürleri liste düzeyinde).
- Makbuz simetrik anahtarlı; kullanıcı tek başına doğrulayamaz (öneri: belirleyici Ed25519).
- SHA-256/HMAC crate içinde yazıldı (FIPS 180-4, RFC 4231 test vektörleri geçiyor);
  kriptografi denetimi yapılmadı.
