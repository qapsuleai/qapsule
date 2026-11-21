# M26 `olcek26` — periyoda göre ölçeklenen dedektör eşikleri

Savunulan seçenek (GOAL M26, ikinci sütun): gün içi barlar kısaldıkça aynı bar sayısı daha
kısa bir saat süresini ve daha fazla gürültüyü kapsar; ürün sabitleri (20 bar kırılım, 120
bar bant geçmişi, 2,5× hacim) günlük için seçildi. `olcek26` aynı yedi dedektörü (aynı
kimlik, aynı sıra) bar periyoduna göre ölçeklenen sabitlerle kurar. Yalnız toplu sayılar;
yalnız kalibrasyon kopyası (`fixtures/local/rp4/calib/`, `--split calib`); `--split test`
koşulmadı.

## Tanım

- Ölçek `z = ln(8 saatlik seanstaki bar sayısı) / ln 96`: 5 dk 1, 15 dk 0,7593, 1 sa 0,4556,
  günlük 0 (çalışma anında `ln` yok; sınıf başına sabit). Her sabit
  `günlük + z·(5dk − günlük)`; pencerelerde çarpan `1 + z·(M − 1)`, en yakın tamsayı, ürün
  penceresinin altına inmez.
- Periyot dedektörün kendi barlarından: ilk 3 barın ardışık en küçük zaman farkı
  (< 15 dk → 5 dk; < 1 sa → 15 dk; < 8 sa → 1 sa; gerisi günlük). Harness fabrikası
  (`fn() -> Vec<Box<dyn Detector>>`) periyot almadığı için. İlk 3 bar tamponlanır, karar
  verilince iç dedektöre yeniden oynatılır; her iç dedektörün ısınması > 3 bar, çıktı
  doğrudan kurulan iç dedektörle bar bar aynı (test). Ürüne alınırsa periyot açıkça
  verilmeli (seri kimliğinden), çıkarım değil.
- Günlükte ürün kurucuları (`Default`), sürüm 1: günlük davranış bayt eşit (test; harness
  `1d` koşusunda yedi dedektörün bütün ölçüleri başvuruyla fark 0,000 [0,000, 0,000]).
  Gün içinde değişen dedektörün sürümü 2 (karne anahtarı ayrışır; harness kimlikle eşler).

**Seçilen sabitler (5 dk ucu; 3 sabit):**

| Dedektör | Sabit | Ürün | 5 dk | 15 dk | 1 sa | Neden |
|---|---|---|---|---|---|---|
| `breakout.range` | pencere | 20 | 60 | 50 | 38 | Kısa barda 20 bar (100 dk) kırılımı gürültü; denemelerde üç periyotta da isabet + |
| `squeeze.bandwidth` | bant geçmişi | 120 | 360 | 302 | 229 | 120 × 5 dk ≈ 1,25 gün; sıkışma çok günlük dip olmalı (öğle durgunluğu değil) |
| `flow.volume_surge` | çarpan | 2,5 | 4,0 | 3,64 | 3,18 | Kısa barda hacim oranının kuyruğu kalın; sabit 2,5× 5 dk'da sembol·gün başına 4 doğum |
| `continuation.ma_stack`, `reversal.rsi_divergence`, `extreme.rsi`, `flow.institutional` | — | ürün | ürün | ürün | ürün | Denemelerde ölçekleme isabeti tutarlı biçimde artırmadı (aşağıda); akış verisi yok |

## Başvuru (`r3`, ürün sabitleri), kalibrasyon

Gün içi: 100 sembol, 241 gün (2025-11-03 → 2026-10-16), 23.104 sembol·gün (1 sa: 24.086).
Günlük: 98 sembol, 1.751 gün, 146.195 sembol·gün. İsabet = gerçekleşti / kapanan, sembol
kümeli bootstrap %95.

| Dedektör | 5 dk isabet | 15 dk isabet | 1 sa isabet | 1 gün isabet |
|---|---|---|---|---|
| `breakout.range` | 0,415 [0,409, 0,421] | 0,435 [0,429, 0,440] | 0,466 [0,455, 0,475] | 0,515 [0,505, 0,524] |
| `continuation.ma_stack` | 0,418 [0,407, 0,427] | 0,439 [0,430, 0,447] | 0,484 [0,474, 0,495] | 0,523 [0,510, 0,535] |
| `extreme.rsi` | 0,420 [0,412, 0,426] | 0,430 [0,422, 0,437] | 0,473 [0,459, 0,487] | 0,526 [0,510, 0,540] |
| `flow.volume_surge` | 0,408 [0,389, 0,423] | 0,437 [0,426, 0,447] | 0,452 [0,439, 0,467] | 0,468 [0,450, 0,485] |
| `reversal.rsi_divergence` | 0,419 [0,413, 0,425] | 0,414 [0,407, 0,422] | 0,458 [0,440, 0,476] | 0,434 [0,414, 0,455] |
| `squeeze.bandwidth` | 0,367 [0,348, 0,383] | 0,521 [0,501, 0,542] | 0,504 [0,475, 0,533] | 0,579 [0,547, 0,609] |
| havuz | 0,413 [0,403, 0,421] | 0,437 [0,430, 0,442] | 0,470 [0,463, 0,477] | 0,509 [0,502, 0,516] |
| doğum / sembol·gün | 18,15 | 6,41 | 1,52 | 0,200 |

0,55'e göre: hepsi `below`, yalnız günlük `squeeze.bandwidth` `contains`. Açılış/kapanış
(havuz): 5 dk açılış payı 0,074 isabet 0,464, kapanış 0,044 / 0,446, geri kalan 0,407,
`ft` −0,050 [−0,057, −0,042] (açılış/kapanış doğumları geri kalandan **iyi**); 15 dk
0,174 / 0,432, 0,102 / 0,436, kalan 0,438, `ft` +0,005 [−0,002, +0,012]; 1 sa 0,373 / 0,459,
0,153 / 0,444, kalan 0,487, `ft` +0,032 [+0,022, +0,042] (gerçek yanlış tetikleme yalnız
1 sa'te).

## Denemeler (yalnız kalibrasyon; 3 deneme)

İsabet farkı = `olcek26 − r3`, eşli bootstrap aralığı. Diğer dedektörler ürün olduğunda
farkları 0.

**D1** — tek pencere çarpanı `W = 3` (kırılım, hareketli ortalama, uyumsuzluk boşlukları,
sıkışma geçmişi, hacim penceresi), RSI bandı 5 dk'da 80/20, hacim çarpanı 4,0.

| | 5 dk | 15 dk | 1 sa |
|---|---|---|---|
| `breakout.range` | +0,014 [+0,011, +0,016] | +0,006 [+0,003, +0,010] | +0,008 [+0,002, +0,013] |
| `continuation.ma_stack` | −0,004 [−0,009, 0,000] | +0,003 [−0,005, +0,011] | −0,007 [−0,021, +0,008] |
| `extreme.rsi` (bant) | +0,024 [+0,016, +0,033] | +0,002 [−0,011, +0,014] | **−0,019 [−0,037, −0,002]** |
| `flow.volume_surge` (pencere + çarpan) | +0,021 [+0,016, +0,025] | **−0,019 [−0,027, −0,010]** | −0,009 [−0,025, +0,006] |
| `reversal.rsi_divergence` | −0,008 [−0,018, +0,001] | −0,008 [−0,022, +0,005] | +0,016 [−0,008, +0,043] |
| `squeeze.bandwidth` | −0,008 [−0,018, +0,001] | +0,002 [−0,016, +0,020] | +0,026 [−0,005, +0,056] |
| havuz isabet / yük oranı / `ft` farkı | +0,010 / 0,50 / +0,009 [+0,005, +0,013] | 0,000 / 0,50 / +0,005 [−0,001, +0,011] | 0,000 / 0,63 / +0,015 [+0,003, +0,027] |

§5: (1) 5 dk geçti, 15 dk ve 1 sa kaldı (kalın satırlar); (2) geçti; (3) üçünde kaldı.
Sonuç: hareketli ortalama ve uyumsuzlukta ölçekleme işe yaramadı → ürün; hacim penceresi
15 dk'da isabeti düşürdü → pencere ürün, yalnız çarpan; RSI bandı 1 sa'te düşürdü.

**D2** — kırılım ×3, sıkışma ×3, hacim yalnız çarpan 4,0, RSI bandı yerine RSI uzunluğu ×3
(70/30 sabit), hareketli ortalama ve uyumsuzluk ürün.

`extreme.rsi` isabet farkı +0,012 [0,000, +0,024] / +0,003 [−0,016, +0,021] / −0,018
[−0,045, +0,007]; yükü 5 dk'da onda bire indirdi (0,22 vs 2,22) ve `ft` farkını büyüttü
(5 dk +0,041 [+0,006, +0,077], 1 sa +0,076 [+0,024, +0,127]). Hacim yalnız çarpanla: isabet
farkı −0,000 / −0,001 / −0,002 (aralıklar 0'ı kapsar). §5: (1) üçünde geçti, (2) geçti,
(3) üçünde kaldı. Sonuç: RSI ölçeklemesi kazanç getirmiyor, yanlış tetiklemeyi artırıyor →
ürün.

**D3 (seçilen)** — D2 eksi RSI ölçeklemesi: kırılım ×3, sıkışma ×3, hacim çarpanı 4,0.

## Seçilen `olcek26` (= D3), kalibrasyon

| Periyot | Dedektör | isabet | fark (aralık) | yük oranı | `ft` farkı |
|---|---|---|---|---|---|
| 5 dk | `breakout.range` | 0,429 [0,422, 0,435] | +0,014 [+0,011, +0,016] | 0,56 | +0,013 [+0,009, +0,019] |
| 5 dk | `flow.volume_surge` | 0,407 [0,386, 0,424] | −0,000 [−0,005, +0,004] | 0,55 | −0,002 [−0,012, +0,007] |
| 5 dk | `squeeze.bandwidth` | 0,359 [0,339, 0,377] | −0,008 [−0,018, +0,001] | 0,41 | +0,008 [−0,189, +0,179] |
| 5 dk | havuz | 0,418 [0,408, 0,426] | +0,005 [+0,004, +0,006] | 0,74 | +0,008 [+0,005, +0,010] |
| 15 dk | `breakout.range` | 0,441 [0,434, 0,448] | +0,006 [+0,003, +0,010] | 0,61 | +0,006 [−0,001, +0,014] |
| 15 dk | `flow.volume_surge` | 0,436 [0,423, 0,448] | −0,001 [−0,007, +0,005] | 0,62 | 0,000 [−0,012, +0,014] |
| 15 dk | `squeeze.bandwidth` | 0,523 [0,497, 0,549] | +0,002 [−0,016, +0,020] | 0,44 | −0,046 [−0,125, +0,036] |
| 15 dk | havuz | 0,437 [0,430, 0,443] | −0,000 [−0,002, +0,001] | 0,77 | +0,002 [−0,002, +0,005] |
| 1 sa | `breakout.range` | 0,474 [0,463, 0,484] | +0,008 [+0,002, +0,013] | 0,72 | +0,009 [−0,003, +0,020] |
| 1 sa | `flow.volume_surge` | 0,451 [0,433, 0,470] | −0,002 [−0,017, +0,013] | 0,61 | −0,008 [−0,038, +0,024] |
| 1 sa | `squeeze.bandwidth` | 0,530 [0,489, 0,571] | +0,026 [−0,005, +0,056] | 0,52 | −0,027 [−0,116, +0,062] |
| 1 sa | havuz | 0,474 [0,467, 0,481] | +0,004 [+0,001, +0,006] | 0,83 | −0,001 [−0,005, +0,005] |

Değişmeyen dört dedektörün farkı tam 0. Günlük: bütün satırlar başvuruyla aynı.

**SPEC §5 puanlaması** (her periyotta): (1) isabet düşmedi **geçti**; (2) yük ≤ 1,5×
**geçti** (0,74 / 0,77 / 0,83); (3) yanlış tetikleme azaldı **kaldı** (havuz `ft` farkının
üst ucu +0,010 / +0,005 / +0,005, < 0 değil) → **aday değil**. Şartnameye göre bugünkü
sabitler kalır.

Neden (3) kalıyor: periyot ölçeklemesi açılış/kapanış barlarını ayırt etmez; daha uzun
pencere en çok gün ortasındaki zayıf doğumları eler ve geri kalanın isabetini yükseltir,
bu da `ft`'yi (geri kalan − açılış∪kapanış) **artırır**. 5 dk'da başvuruda açılış/kapanış
doğumları zaten geri kalandan iyi (`ft` −0,050); "azaltmak" orada onları daha da öne
çıkarmak demek. Açılış/kapanışı hedefleyen ölçü mevsimsellik seçeneğinin işidir.

## Gün içinde anlamsız dedektörler (SPEC §5 tanımı)

Başvuruda ve `olcek26`'da **altı fiyat/hacim dedektörünün hepsi üç gün içi periyotta da
"anlamsız"** (üst uç < 0,5), tek istisnalar: `squeeze.bandwidth` 15 dk (başvuru 0,521
[0,501, 0,542]: anlamsız değil; `olcek26` 0,523 [0,497, 0,549]: 0,55 altı ve 0,5'i
kapsıyor → anlamsız) ve 1 sa (0,504 / 0,530, 0,5'i kapsıyor → anlamsız). Hiçbiri 0,55'in
üstünde değil. İsabet periyotla düzenli artıyor (havuz 0,413 → 0,437 → 0,470 → 0,509);
ölçekleme bunu 5 dk'da +0,005, 1 sa'te +0,004 kıpırdatıyor, eşik tarafını değiştirmiyor.

**Periyot sınırı önerisi** (kapatma değil; kalibrasyon kanıtı, sınamada doğrulanmalı):

| Dedektör | En kısa periyot | Gerekçe |
|---|---|---|
| `breakout.range`, `continuation.ma_stack`, `extreme.rsi` | 1 gün | 5 dk / 15 dk / 1 sa'in hepsinde üst uç < 0,5; günlükte 0,5'in üstünde (0,515–0,526) |
| `squeeze.bandwidth` | 15 dk | 5 dk'da açıkça kötü (0,367 [0,348, 0,383]); 15 dk ve 1 sa'te 0,5 civarı, günlükte 0,579 |
| `flow.volume_surge`, `reversal.rsi_divergence` | (gün içinde yok) | Gün içinde üst uç < 0,5; günlükte de anlamsız (0,468, 0,434): gün içi sınırı dışında ayrı konu |

Dürüstlük notu: isabet süresi dolanı ıska sayar ve r3 bariyeri simetriktir; yön bilgisi
olmayan bir dedektör 0,5'in altında kalır. "Anlamsız" bu tanıma göredir.

## Yeniden üretim

```sh
cd rounds/RP4/experiments/harness && export CARGO_TARGET_DIR=<ana ağaç>/target/wt/m26b
cargo build --release; H=$CARGO_TARGET_DIR/release/harness; cd <depo kökü>
for p in 5m 15m 1h 1d; do $H m26 --data fixtures/local/rp4/calib --detectors olcek26 --period $p --split calib --out <dir>; \
  $H m26 --data fixtures/local/rp4/calib --detectors r3 --period $p --split calib --out <dir>; done
$H report --results <dir> --out <dir>
```

Süre: 5 dk 4 s, 15 dk 0,7 s, 1 sa 0,4 s, 1 gün 0,1 s (Apple silicon, `--release`). İki koşu
bayt eşit (rapor ve CSV).
