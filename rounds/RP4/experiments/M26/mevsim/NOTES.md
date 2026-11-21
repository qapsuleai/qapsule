# M26 alternatifi `mevsim` — gün içi mevsimselliğe göre düzeltilmiş eşikler

Dal `rp4/m26c-mevsim`. Ölçüm düzeneği `../../harness` (SPEC §5, değiştirilmedi). Veri yalnız
kalibrasyon kopyası `fixtures/local/rp4/calib/` (100 sembol; 5 dk / 15 dk / 1 sa
2025-11-03 → 2026-10-16, 241 gün; 1 gün 2018-01-02 → 2025-01-01). `--split test`
çalıştırılmadı. Aşağıda yalnız toplu sayılar var.

## Fikir ve tasarım

Gün içinde hacim ve oynaklık U biçimlidir. "Son N barın ortalamasının k katı" türü eşikler bu
yüzden açılışta ve kapanışta (kapanış seansı son bara birleşik) yapısal olarak tetiklenir.

- `season::Season`: bir niceliğin **aynı saat dilimindeki** son `DAYS` = 20 geçmiş işlem
  günündeki ortalaması. Dilim = XIST seans açılışından dakika; günün son barı (açıklığı seans
  kapanışına ulaşan; yarım günler dahil) ayrı `CLOSE` dilimi. Çarpan `f = ortalama(dilim) /
  genel`, `genel` = sıcak dilim ortalamalarının ortalaması, her günün ilk barında dondurulur
  (yalnız geçmiş günler). Dilim en az `MIN_DAYS` = 5 günlük değil → `f = 1`.
- Bar uzunluğu serinin kendisinden çıkarılır (aynı günün iki ardışık barı arasındaki en küçük
  fark). Hiç aynı günde iki bar görmeyen seri (günlük) gün içi değildir: `f = 1` tam.
- **Periyot tabanı** `MIN_LEN_MS` = 1 saat: bar uzunluğu bunun altındaysa `f = 1` tam
  (deneme 3; aşağıda).
- `flow.volume_surge` v2: `u = hacim / f`; `u > 2,5 · sma20(önceki u)`. Eşdeğeri: eşik dilimin
  olağan hacmiyle ölçeklenir. Pencere ve kat ürünün değerleri.
- `breakout.range` v2: gerçek aralığın (TR) mevsim çarpanı `g`; pay `m = max(0, g − 1) ·
  ATR14`; `kapanış > hh + m` (`< ll − m`). `g ≤ 1` dilimlerde v1.
- Diğer beş dedektör ürünün kendisi. Kimlikler ve sıra ürün setiyle aynı; değişen iki dedektörün
  sürümü 2. Durum her dedektörün kendi akışkan durumunda; saat, ağ, rastgelelik, dosya yok.
- Ayarlanan sabitler: `DAYS`, `MIN_DAYS`, `MIN_LEN_MS` (3). `DAYS` ve `MIN_DAYS` ilk denemede
  konuldu, değiştirilmedi.

## Başvuru (`r3`, kalibrasyon) — açılış ve kapanış doğumları

| Periyot | Dedektör | doğum | açılış payı | açılış − kalan | kapanış payı | kapanış − kalan | ft |
|---|---|---|---|---|---|---|---|
| 5m | `flow.volume_surge` | 93788 | 0.070 | +0.073 [0.061, 0.086] | 0.116 | +0.103 [0.087, 0.120] | −0.092 [−0.105, −0.079] |
| 5m | `breakout.range` | 134031 | 0.092 | +0.055 [0.046, 0.064] | 0.031 | −0.049 [−0.065, −0.033] | −0.028 [−0.037, −0.020] |
| 5m | (hepsi) | 419310 | 0.074 | +0.056 [0.049, 0.064] | 0.044 | +0.038 [0.026, 0.050] | −0.050 [−0.057, −0.042] |
| 15m | `flow.volume_surge` | 30726 | 0.216 | −0.018 [−0.032, −0.005] | 0.290 | +0.036 [0.023, 0.050] | −0.013 [−0.024, −0.002] |
| 15m | `breakout.range` | 49721 | 0.207 | +0.012 [0.000, 0.023] | 0.066 | −0.059 [−0.079, −0.040] | +0.005 [−0.006, 0.017] |
| 15m | (hepsi) | 148179 | 0.174 | −0.007 [−0.015, 0.002] | 0.102 | −0.002 [−0.012, 0.009] | +0.005 [−0.002, 0.012] |
| 1h | `flow.volume_surge` | 4578 | 0.473 | −0.005 [−0.040, 0.030] | 0.269 | −0.022 [−0.059, 0.012] | +0.011 [−0.021, 0.044] |
| 1h | `breakout.range` | 13988 | 0.422 | −0.049 [−0.067, −0.030] | 0.150 | −0.066 [−0.094, −0.039] | +0.053 [0.036, 0.071] |
| 1h | (hepsi) | 36710 | 0.373 | −0.028 [−0.040, −0.016] | 0.153 | −0.043 [−0.057, −0.028] | +0.032 [0.022, 0.042] |

Gözlem: hacim yoğunlaşması gerçek (5 dk'da günün 1/96'sı olan son bar hacim doğumlarının
%11,6'sı; 1 sa'te açılış payı %47). Ama **5 dk'da açılış ve kapanış doğumları geri kalandan
daha isabetli** (`ft` < 0): SPEC'in "yanlış tetikleme" tanımına (isabeti geri kalandan düşük)
uymuyorlar. Olası neden (sınanmadı): sonuç kuralının bariyeri 5 dk ATR'si ile küçük; gün
sonundaki ve açılıştaki doğumlar gece boşluğunu ya da açılış oynaklığını görür, süresi dolma
(ıska sayılır) azalır. Yanlış tetikleme yalnız 1 sa'te (ve 15 dk'da kapanışta, kırılımda)
görülüyor.

## Denemeler (hepsi kalibrasyon; düzenekte kayıtlı, yeniden üretilebilir)

Fark = alternatif − başvuru, sembol kümeli bootstrap %95, eşli.

**1a `mevsim_t1v`** — yalnız hacim mevsimsel, bütün gün içi periyotlar.

| Periyot | hacim isabet farkı | hacim yük oranı | havuz `ft` farkı | §5 |
|---|---|---|---|---|
| 5m | −0.036 [−0.040, −0.033] | 0.94 | +0.008 [0.004, 0.010] | (1) kaldı, (3) kaldı |
| 15m | −0.039 [−0.045, −0.033] | 0.72 | −0.002 [−0.006, 0.002] | (1) kaldı, (3) kaldı |
| 1h | +0.007 [−0.008, 0.023] | 0.83 | −0.002 [−0.006, 0.001] | (1) geçti, (3) kaldı |

Açılış/kapanış hacim doğumları 5 dk'da 6609/10835 → 940/671 (−%91), ama kalan doğumlar ve
öğle saatinde eklenenler daha az isabetli: hacim isabeti düşüyor.

**1b `mevsim_t1vb`** — hacim + kırılım mevsimsel, bütün gün içi periyotlar.

| Periyot | kırılım isabet farkı | hacim isabet farkı | havuz isabet farkı | havuz yük oranı | havuz `ft` farkı | §5 |
|---|---|---|---|---|---|---|
| 5m | −0.005 [−0.006, −0.004] | −0.035 [−0.039, −0.032] | −0.009 | 0.95 | +0.015 [0.010, 0.020] | (1)(3) kaldı |
| 15m | −0.002 [−0.004, −0.000] | −0.038 [−0.045, −0.031] | −0.007 | 0.89 | +0.005 [0.000, 0.010] | (1)(3) kaldı |
| 1h | +0.006 [0.002, 0.009] | +0.006 [−0.008, 0.023] | +0.003 [0.001, 0.005] | 0.93 | −0.005 [−0.011, −0.000] | **aday** |

**2 `mevsim_t2g`** — hacim "kapı" kipinde (ürün koşulu da gerekir; mevsim yalnız eler) +
kırılım, bütün gün içi periyotlar. 5 dk hacim isabet farkı −0.026 [−0.030, −0.021], havuz
`ft` +0.019 [0.014, 0.024]; 15 dk −0.016 / +0.011; 1 sa havuz isabet +0.004 [0.002, 0.006],
`ft` −0.004 [−0.008, +0.001]. Hiçbir periyotta aday değil. Sonuç: 5 dk ve 15 dk'da açılış /
kapanış doğumlarını elemek isabeti düşürüyor; bu periyotlarda fikrin önermesi tutmuyor.

**3 `mevsim` (son)** — 1b, periyot tabanı 1 saat (`MIN_LEN_MS`). Taban, başvurunun kendi
tablosundan okunan işaretle seçildi (açılış/kapanış doğumları yalnız 1 sa'te geri kalandan
anlamlı düşük); 5 dk ve 15 dk'da set ürünle bit eşit.

## Son set `mevsim` — kalibrasyon

| Periyot | Dedektör | isabet (alt / başvuru) | isabet farkı | yük oranı | `ft` farkı |
|---|---|---|---|---|---|
| 5m | hepsi | 0.413 / 0.413 | 0 (ürünle aynı) | 1.00 | 0 |
| 15m | hepsi | 0.437 / 0.437 | 0 (ürünle aynı) | 1.00 | 0 |
| 1h | `breakout.range` | 0.472 / 0.466 | +0.006 [0.002, 0.009] | 0.86 | −0.003 [−0.011, 0.004] |
| 1h | `flow.volume_surge` | 0.458 / 0.452 | +0.006 [−0.008, 0.023] | 0.86 | +0.002 [−0.036, 0.038] |
| 1h | diğer dört | aynı | 0 | 1.00 | 0 |
| 1h | (hepsi) | 0.473 / 0.470 | +0.003 [0.001, 0.005] | 0.93 | −0.005 [−0.011, −0.0005] |
| 1d | hepsi | ürünle aynı (bütün satırlar fark 0) | 0 | 1.00 | — |

**SPEC §5 durumu:** 1 sa **aday** ((1) her dedektörde isabet farkının üst ucu ≥ 0, (2) havuz
yük oranı 0,93, (3) havuz `ft` farkının üst ucu −0,0005 < 0 — sınırda). 5 dk ve 15 dk aday
değil ((3) fark tam 0; ürünle aynı). Bütün dedektörler bütün gün içi periyotlarda K53 eşiğinin
altında (`below`) ve "gün içinde anlamsız"; mevsim düzeltmesi bunu değiştirmiyor.

**Açılış / kapanış yanlış tetikleme sayıları** (sayılan doğum; ıska ≈ doğum × (1 − isabet)):

| Periyot | Dedektör | açılış doğum (ıska) alt | başvuru | kapanış doğum (ıska) alt | başvuru |
|---|---|---|---|---|---|
| 1h | `breakout.range` | 3863 (2120) | 5905 (3260) | 1973 (1111) | 2094 (1192) |
| 1h | `flow.volume_surge` | 877 (476) | 2165 (1179) | 296 (170) | 1233 (692) |
| 1h | (hepsi) | 10371 (5571) | 13701 (7413) | 4557 (2519) | 5615 (3122) |
| 5m | `flow.volume_surge` (deneme 1a) | 940 (507) | 6609 (3542) | 671 (356) | 10835 (5488) |
| 15m | `flow.volume_surge` (deneme 1a) | 804 (442) | 6643 (3903) | 527 (294) | 8919 (4759) |

1 sa'te açılış ıskası −%25, kapanış ıskası −%19 (havuz); doğum başına oran ise yalnız biraz
iyileşiyor (havuz `d_open` farkı +0,006 [0,000, 0,011]).

## Süre

`m26 --period 5m` (`--release`, 16 çekirdek): `r3` ~12 s, `mevsim` ~16 s (mevsim durumu 5 dk'da
da tutulur, kullanılmaz), denemeler 14–21 s. 1 sa ~1,3 s.

## Yeniden üretim

```sh
cd rounds/RP4/experiments/harness
export CARGO_TARGET_DIR=<ana ağaç>/target/wt/m26c
cargo build --release; H=$CARGO_TARGET_DIR/release/harness
# depo kökünden, her set (r3 mevsim mevsim_t1v mevsim_t1vb mevsim_t2g) ve periyot (5m 15m 1h 1d):
$H m26 --data fixtures/local/rp4/calib --detectors mevsim --period 1h --split calib
$H report --out <dir>
```

## Açık sorular / öneriler

- 5 dk ve 15 dk'da açılış/kapanış doğumlarının daha isabetli olması sonuç kuralının (ATR
  ölçekli bariyer, gece boşluğu) bir etkisi olabilir; M25 seçimiyle birlikte yeniden bakılmalı.
- 1 sa adaylığı sınırda (üst uç −0,0005); sınama aralığında tutmazsa bugünkü sabitler kalır.
- `squeeze.bandwidth` öğle durgunluğunda (bant daralır) mevsimsel düzeltmeye aday olabilir;
  açılış/kapanış payı küçük olduğu için bu round'da denenmedi.
