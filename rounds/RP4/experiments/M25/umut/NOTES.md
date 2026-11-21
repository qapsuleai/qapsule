# M25 alternatifi `umut` — "umut kesildi" durdurması (zamanla daralan, asimetrik bariyer)

Dal `rp4/m25e-savunu`. Düzenek `rounds/RP4/experiments/harness` (SPEC.md değiştirilmedi).
Yalnız kalibrasyon kopyası (`fixtures/local/rp4/calib/`); `--split test` çalıştırılmadı.
Bu dosyada yalnız toplu sayılar var. Tam tablolar: `calib/M25.md`, `calib/M25.csv`
(`harness report` çıktısı).

## 1. Fikir

`r3` durdurmayı hedefin aynası olarak sabit `−k·A`'ya koyar (`k = 0,3·√h`, `A` = doğumdaki
ATR14). 5 dk'da bu birkaç fiyat adımıdır: doğumdan hemen sonraki tek ters bar hikâyeyi
"tutmadı" diye kapatır, hikâyenin önünde ise daha `h − 1` bar vardır. Erken kapanma (ECR)
tam olarak bu: hikâyenin önünde zaman varken verilen hüküm.

`umut` durdurmayı şu soruya bağlar: **iddia kalan ömründe hâlâ gerçekleşebilir mi?**
Doğumdan `t` bar sonra, kalan `τ = h − t ≥ 1` bar ve iddia yönündeki hareket
`d = ±(kapanış − P0)` iken:

- `d ≥ T` (`T = M·k·A`) → **gerçekleşti** (`umut.target`);
- `d < 0` ve `T − d ≥ Z·A·√τ` → **tutmadı** (`umut.implausible`): fiyat iddianın tersinde ve
  hedefe kalan mesafe kalan ömrün `Z` oynaklığından büyük (difüzyon ölçeği, `r3`'ün de
  kullandığı `√h`);
- ufuk barında (`τ = 0`) durdurma kapalı; karar yoksa motor `expired` der (`r3` gibi).

Durdurma düzeyi `T − Z·A·√τ` başta çok uzaktadır ve zamanla `P0`'a doğru **daralır**. Yönsüz
hikâyeler `r3` ile birebir aynı yargılanır.

Neden bu (diğer savunucuların fikirlerinden farkı): adım tabanı, iki kapanış teyidi,
yüksek/düşük ve periyot ölçeği eşiğin **büyüklüğünü ya da tetikleyicisini** değiştirir; `umut`
ise hükmün **zamana bağlı anlamını** değiştirir. ECR'nin tanımı "tutmadı denen, sonra
ömrü içinde hedefe ulaşan"dır; `umut`'un durdurması doğrudan "ömrü içinde hedefe ulaşması
olasılık dışı" demektir, yani ölçünün tümleyenini kural yapar. Sentetik rastgele yürüyüşte
`r3`'ün ECR'si 0,16–0,20 (aşağıda): gerçek 5 dk'daki 0,148 veri tuhaflığı değil, sabit
simetrik bariyerin yapısal sonucudur; eşik büyütmek bunu yalnız ölçekler.

Diğer adaylar (seçilmedi): gün içi mevsimselliğe göre düzeltilmiş ya da Parkinson oynaklığı
— ölçüt `A`'yı iyileştirir ama ECR'nin yapısal kısmına (sabit simetrik durdurma) dokunmaz;
"ilk `f·h` barda durdurma yok" — tek sabitli basamak, `umut`'un sürekli sürümüdür ve basamağın
hemen arkasında aynı tek-bar duyarlılığını yeniden yaratır.

## 2. Sabitler (2 yeni; `r3`'ün 7'si aynen: `C_DIR`, `C_NEUTRAL`, ATR14/100, ufuklar)

| Sabit | Değer | Nasıl |
|---|---|---|
| `Z` | 1,5 (ATR14 / √bar) | **Önsel**, hiçbir piyasa koşusundan önce, hiç değişmedi. Sentetik yürüyüşte ATR14 ≈ 1,35 σ (kapanıştan kapanışa), yani `1,5·A ≈ 2σ`: sürüklenmesiz yürüyüşün durdurmadan hedefe dönme olasılığı ≲ %5 (sürekli ilk geçiş; ayrık kapanışlarda daha az). |
| `M` | 1,45 (hedef = `M·0,3·√h·A` = `0,435·√h·A`) | **Yalnız sentetik yürüyüşte** (`null_walk.py`, tohum sabit): kuralın sürüklenmesiz tabanı karar verilenlerin %50'si olsun diye (`r3` gibi). Piyasa verisinde ayarlanmadı. |

`Z` ve `M`'nin ikisi de kalibrasyon verisinde taranmadı; kalibrasyonda yalnız iki koşu yapıldı
(§4).

## 3. Simetri ve karnenin tabanı

Hedef ve durdurma artık ayna değil. Yalnız durdurma değişseydi (`M = 1`), sürüklenmesiz
piyasada karar verilenlerin ≈ %61'i gerçekleşirdi (durdurma başta zor): karne 0,5'e göre
değil 0,61'e göre okunmalı, K53 yazı tura eşiği (0,55) anlamsızlaşırdı. Bu yüzden hedef `M`
ile genişletildi; taban yeniden ≈ 0,50:

Sentetik sürüklenmesiz yürüyüş (bar başına 20 alt adım, Wilder ATR14, 15.000 hikâye / ufuk;
`python3 -I null_walk.py 1.5 1.45 15000`):

| ufuk `h` | kural | gerçekleşti | tutmadı | süresi doldu | gerç./karar | ECR |
|---|---|---|---|---|---|---|
| 10 | `r3` | 0,488 | 0,479 | 0,033 | 0,505 | 0,158 |
| 10 | `umut` | 0,440 | 0,456 | 0,103 | 0,491 | 0,011 |
| 15 | `r3` | 0,487 | 0,489 | 0,024 | 0,499 | 0,200 |
| 15 | `umut` | 0,460 | 0,461 | 0,079 | 0,500 | 0,014 |
| 20 | `r3` | 0,492 | 0,488 | 0,020 | 0,502 | 0,202 |
| 20 | `umut` | 0,462 | 0,478 | 0,060 | 0,491 | 0,018 |
| 10 / 15 / 20 | `umut`, `M = 1` | 0,558 / 0,580 / 0,584 | | | 0,612 / 0,615 / 0,607 | |

Yani karnenin tabanı: **karar verilenler içinde ≈ 0,50** (`r3` ile aynı); **bütün hikâyeler
içinde** (süresi dolan ıska sayılır) 0,44–0,46 (`r3`: 0,49), çünkü süresi dolan payı daha
yüksek. Karnede "isabet" süresi dolanı ıska saydığı için, `umut` ile karne sayısı aynı
dedektörde `r3`'ten 0,03–0,05 düşük görünür; bu bilgi kaybı değil tanım farkıdır.

## 4. Denemeler (kalibrasyon, hepsi)

1. **Deneme 1** — `Z = 1,5`, `M = 1` (hedef `r3`'le aynı), 5 dk: ECR_own 0,025, SENS farkı
   +0,003 [−0,002, +0,007], p_exp 0,139, p_real 0,474 (`r3` 0,415), **Brier farkı +0,0067
   [+0,0059, +0,0076] → §4 (3) elendi.** Neden: ECR'den kurtulan hikâyeler gerçekleşti
   oldu, gerçekleşme oranı 0,5'e yaklaştı; karne bilgisiz olduğundan Brier ≈ `q(1−q)` ve
   `q` 0,415 → 0,474 olunca mekanik olarak büyüdü (Brier − kendi tabanı: +0,0002, `r3`
   +0,00004; beceri farkı yok). Aynı koşuda §3'teki taban sorunu görüldü.
2. **Deneme 2 (seçilen)** — `M = 1,45` sentetik yürüyüşte tabanı %50'ye getirmek için
   seçildi (yukarıdaki tablo; piyasa verisinde tarama yok), `Z` değişmedi. Dört periyot.
   Yeniden koşu bayt eşit (sabit sayısı alanı hariç: 8 → 9 düzeltmesi).

**Dürüstlük notu:** `M`'nin gerekçesi (taban %50, K53) bağımsızdır, ama `M`'yi ekleme kararı
deneme 1'in Brier sonucunu gördükten sonra verildi. Brier'deki iyileşme (aşağıda) da aynı
mekanizmanın ters yönüdür: gerçekleşme oranı 0,5'ten uzaklaştı. Bkz. §6 (1).

## 5. Sonuçlar (kalibrasyon, yönlü satır; fark = `umut` − `r3`, eşli bootstrap %95)

| Ölçü | 5 dk | 15 dk | 1 sa | 1 gün |
|---|---|---|---|---|
| n (umut / r3) | 339.791 / 383.867 | 122.876 | 30.916 | 24.540 |
| p_real (r3) | 0,367 (0,415) | 0,378 (0,432) | 0,406 (0,468) | 0,462 (0,507) |
| p_inv (r3) | 0,482 (0,467) | 0,468 (0,450) | 0,458 (0,441) | 0,423 (0,420) |
| p_exp (r3) | 0,151 (0,118) | 0,154 (0,118) | 0,136 (0,091) | 0,115 (0,073) |
| **ECR_own** (r3) | **0,026** (0,148) | 0,022 (0,136) | 0,021 (0,128) | 0,018 (0,159) |
| fark | −0,122 [−0,125, −0,119] | −0,113 [−0,117, −0,110] | −0,107 [−0,112, −0,102] | −0,141 [−0,149, −0,133] |
| ECR_ref | 0,036; fark −0,112 [−0,114, −0,109] | 0,032; −0,104 | 0,032; −0,096 [−0,101, −0,090] | 0,030; −0,129 |
| REV (r3) | 0,186 (0,250) | 0,163 (0,235) | 0,151 (0,216) | 0,168 (0,244) |
| Brier (r3) | 0,2325 (0,2428) | 0,2354 (0,2454) | 0,2434 (0,2509) | 0,2415 (0,2462) |
| Brier farkı | −0,0103 [−0,0113, −0,0094] | −0,0100 [−0,0110, −0,0090] | −0,0075 [−0,0087, −0,0062] | −0,0047 [−0,0058, −0,0037] |
| Brier − kendi tabanı (r3) | +0,0002 (+0,00004) | +0,0004 (+0,0001) | +0,0023 (+0,0019) | −0,0071 (−0,0038) |
| kal. farkı farkı | +0,004 [+0,003, +0,006] | +0,005 [+0,001, +0,008] | +0,003 [−0,004, +0,010] | +0,023 [+0,017, +0,028] |
| **SENS** (r3) | **0,199** (0,212) | 0,135 (0,156) | 0,077 (0,091) | 0,078 (0,091) |
| SENS farkı | −0,012 [−0,016, −0,009] | −0,021 [−0,025, −0,017] | −0,014 [−0,019, −0,010] | −0,013 [−0,019, −0,008] |
| gerçek − yalancı (r3) | −0,025 (−0,052) | −0,002 (−0,018) | +0,029 (+0,028) | +0,093 (+0,087) |
| ömür ortanca/p90 (r3) | 8/16 (4/11) | 8/15 | 8/15 | 7/14 |

Yönsüz satır `r3` ile özdeş (kural aynı; bütün farklar 0).

**SPEC §4 (5 dk, kalibrasyon) — `report` mekanik puanlaması: aday.**

1. ECR_own 0,026 ≤ 0,074 (= ½ × 0,148), farkın üst ucu −0,119 < 0 — geçti.
2. ECR_ref farkı [−0,114, −0,109], 0'ın üstünde değil — geçti.
3. Brier farkı alt uç 5 dk −0,0113, 1 sa −0,0087 (≤ 0); kalibrasyon farkı farkı +0,004 ≤ +0,03 — geçti.
4. p_exp 0,151 ≤ 0,40 — geçti.
5. SENS farkı alt uç −0,016 ≤ 0 — geçti.

15 dk ve 1 sa'te aynı ölçüler aynı yönde. 1 gün: §6 sınama aralığında denetlenir (yapılmadı);
kalibrasyonda ECR, SENS, Brier daha iyi, gerçek − yalancı farkı +0,007 [−0,001, +0,014]
(kötüleşme yok), kalibrasyon farkı +0,023 kötü, p_exp +0,042.

## 6. Zayıf yanlar (dürüstçe)

1. **Brier iyileşmesi beceri değil.** "Brier − kendi tabanı" iki kuralda da ≈ 0 (gün içinde
   karne bilgisiz). Farkın kaynağı gerçekleşme oranının 0,5'ten uzaklaşması
   (5 dk 0,415 → 0,367). SPEC §4 (3) bu yüzden taban oranı kaymasına duyarlı: deneme 1 aynı
   mekanizmayla elendi, deneme 2 aynı mekanizmayla geçti. Öneri: §4 (3) "Brier − taban"
   farkıyla okunmalı; o ölçüyle `umut` 5 dk'da +0,0002'ye karşı +0,00004 (ayırt edilemez
   küçüklükte, kötüleşme yok denebilir ama aralığı düzenek vermiyor).
2. **Hedef büyüdü** (`0,435·√h` ATR, `r3` 0,3·√h). "Gerçekleşti" daha büyük hareket ister;
   gün içi p_real 0,05–0,06 düşer, süresi dolan 3–4,5 puan artar (5 dk 0,118 → 0,151).
   ECR_own'un bir kısmı hedefin büyümesinden gelir; ECR_ref (`r3` hedefi) 0,036 ile yine de
   başvurunun dörtte biri, yani asıl etki durdurmadan.
3. **Geç durdurma `P0`'a çok yakın.** `τ = 1`'de durdurma düzeyi `T − 1,5·A`: `h = 10`'da
   `P0`'ın yalnız ≈ 0,12·A altı. Sondan bir önceki barda küçük bir ters kapanış "tutmadı"
   der; anlamı "kalan bir barda hedefe ulaşamaz"dır. Cümle şablonu bunu "süresi içinde
   gerçekleşmesi olası değil" diye söylemeli; kullanıcıya "fiyat iddianın tersine döndü"
   gibi okunmamalı. ECR_own'un küçüklüğünün bir kısmı da kararların geç verilmesinden
   (pencerede az bar kalıyor): ömür ortancası 4 → 8 bar.
4. **Hikâyeler daha uzun açık kalır** (5 dk ömür p90 11 → 16 bar): akışta aynı anda daha çok
   açık hikâye; tema kuralı nedeniyle sayılan hikâye 383.867 → 339.791 (açık hikâye yeni
   olguyu emer).
5. **`levels` durdurması yaklaşık.** Arayüz doğumda sabit durdurma ister; verilen değer en
   geniş (`t = 1`) düzeydir. Düzenek süresi dolan hikâyelerin SENS "en yakın barını" bununla
   seçer; daralan gerçek durdurmaya yakın barlar gözden kaçabilir → süresi dolanlarda SENS
   olduğundan düşük ölçülmüş olabilir.
6. **Model varsayımı.** Durdurma difüzyon (`√τ`) ve tek bir doğum oynaklığına dayanır;
   oynaklık kümelenmesi, gün içi mevsimsellik (açılış barlarının şişirdiği ATR14), gece
   boşluğu ve kalın kuyruklar hesaba katılmadı. `Z` önseldir; gerçek ECR'nin (0,026) sentetik
   ECR'den (0,011–0,018) yüksek olması bunun izidir.
7. **Kalibrasyon farkı biraz kötü** (5 dk +0,004, 15 dk +0,005, 1 gün +0,023; sınır +0,03).
8. Gün içinde karne zaten yazı turadan kötü (gerçek − yalancı 5 dk < 0): bu kuraldan değil,
   dedektörlerden (M26). `umut` farkı küçültüyor (+0,027) ama işaretini değiştirmiyor.

## 7. R6 durumu

Arkin'in gördüğü durum (5 dk, düşüş iddiası, ertesi barda "tutmadı", sonra fiyat iddia
yönünde eşiği aştı) birim testinde: `tests::one_adverse_bar_does_not_stop_early` — `r3` ilk
barda `−0,95·A`'da durur; `umut` ilk barda ancak ≈ `−3,1·A` ters harekette durur, iki bar
sonra hedefe ulaşan hikâye gerçekleşti olur. Gerçek akıştaki yeniden üretim orkestratörün
işidir (ölçüt 5).

## 8. Yeniden üretim

```sh
cd rounds/RP4/experiments/harness
export CARGO_TARGET_DIR=<ana ağaç>/target/wt/m25e
cargo build --release
# depo kökünden:
for p in 5m 15m 1h 1d; do $CARGO_TARGET_DIR/release/harness m25 --data fixtures/local/rp4/calib \
  --rule umut --period $p --out fixtures/local/rp4/results-umut; done
$CARGO_TARGET_DIR/release/harness report --results fixtures/local/rp4/results-umut \
  --out rounds/RP4/experiments/M25/umut/calib
python3 -I rounds/RP4/experiments/M25/umut/null_walk.py 1.5 1.45 15000   # §3 tablosu
```

Süre (16 çekirdek, `--release`): 5 dk 43–100 s (iki kural), 15 dk 5–10 s, 1 sa ve 1 gün < 2 s.
