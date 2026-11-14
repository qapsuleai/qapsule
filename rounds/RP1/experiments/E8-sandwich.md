# E8 — Kanıtlı geç başlangıç ("sandviç"): geçmişi koşmadan bit-eşit pencere

Dal: `rp1/e8-sandwich` (taban `caab6e3`). Yürüten: orkestratör. Fikir E7.5'teki gözlemden
doğdu (geç başlatılan EMA/RSI yeterli ısınmadan sonra tam çalıştırmayla bit bit aynı).

## Hipotez (denemeden önce yazıldı)

**Gözlem.** Büzülen özyinelemeli hesapların kararlı güncellemesi, *yuvarlamalı* kayan noktada
da durumuna göre monoton azalmayandır:

- EMA: `s' = fl(fl(x·a) + fl(s·(1−a)))`, `1−a ≥ 0`. `fl(s·c)` (c ≥ 0) ve `fl(k + ·)` monoton
  azalmayan; bileşimleri de öyle.
- RSI: `ag' = fl(fl(fl(ag·(p−1)) + g) / p)`, aynı şekilde `al'`; ikisi de kendi durumunda
  monoton azalmayan.

**Sandviç.** Bar `s−1`'deki gerçek durum `T` bilinmiyor ama sınırları biliniyor:
EMA için `[min x, max x]` (geçmişin ağırlıklı ortalaması; yuvarlama payıyla genişletilmiş),
RSI için `[0, en büyük kazanç]` ve `[0, en büyük kayıp]`. Alt sınırdan `L`, üst sınırdan `U`
yörüngesi başlatılır. Monotonluk tümevarımla her adımda `L ≤ T ≤ U` verir. `L` ile `U` bir
barda bit bit eşitlenirse `T` de onlara eşittir; o bardan sonra üçü aynı işlemleri yaptığı
için aynı kalır. **Sonuç: o bardan itibaren her çıktı tam çalıştırmayla bit-eşit — bu bir
kanıt, olasılık değil.** Eşitlenme istenen pencereden önce olmazsa başlangıç geriye çekilir
(iki katı); sonunda tam çalıştırmaya düşülür. Yani yöntem her zaman doğru; yalnızca hızı
veriye bağlı.

**Beklenen.** E7.5'e göre EMA(20) ≈ 1.000, RSI(14) ≈ 1.000 bar içinde eşitleniyor; büzülme
oranı `1−a` ve `(p−1)/p` olduğu için gereken bar sayısı periyotla kabaca doğrusal büyür
(≈ 50·p). 10.000 barlık seride son 500 barı isteyen bir sorguda kısa periyotlar için ≈ 5–10 kat,
1.000.000 barlık seride ≈ 100–1000 kat. Bilinen sınır: SMA/Bollinger'in kayan toplamı büzülmez
(E7.5), bu yöntem onlara uygulanamaz. Düz (sabit) seride iki yörünge farklı kayan nokta sabit
noktalarına takılıp hiç eşitlenmeyebilir — o zaman tam çalıştırmaya düşülür.

## Deney 8.1 — `late::ema_tail` / `late::rsi_tail`

**Yapılan.** `crates/qp-compute/src/late.rs`. `Bounds::new(x, from)` geçmişin tamamı için bir
kez hesaplanır (min/maks fiyat, en büyük kazanç/kayıp; yuvarlama için göreli 1e-6
genişletilmiş; sonlu olmayan geçmişte sınır yok → tam çalıştırma) ve bütün periyotlarca
paylaşılır. Geç başlangıç `from − 24·p` (EMA) ya da `from − 48·p` (RSI); sınırlar buluşmazsa
en erken geçerli bardan bir kez daha, sonra tam çalıştırma. Sonuç her durumda tam
çalıştırmayla bit-eşit; `Certificate { start, met, steps }` nasıl hesaplandığını söyler.

**Doğruluk kanıtı (test).** `tests/late.rs`: 5 seri türü (rastgele yürüyüş, sabit seri,
uzun düz bölgeler, sıfır blokları, küçük çarpanlı) × n = 0, 1, 30, 3.000 × periyot 1, 2, 3,
9, 14, 20, 50, 199 × `from` = 0, n/3, n−500, n−1, n × EMA/RSI = 1.600 durum; kuyruktaki her
bar tam çalıştırmayla bit bit aynı. Rastgele yürüyüşte n=3.000, son 500 bar, periyot ≤ 50 için
kanıt **zorunlu** (testte `assert`) ve hepsi kanıtlanıyor. Sabit seride sınırlar farklı sabit
noktalara takılıp buluşmayabiliyor; o zaman tam çalıştırmaya düşülüyor (test bunu da kapsıyor).

**İlk sürüm yavaştı (kayıt).** İlk ölçümde geç başlangıç tam taramadan 2–5 kat yavaştı:
(a) sınırlar her denemede, periyot başına geçmişin tamamı taranarak hesaplanıyordu (1M barda
periyot başına 1M eleman); (b) ilk geri bakış 8·p, gereken mesafenin (EMA ≈ 18·p, RSI ≈ 37·p)
altındaydı; (c) kanıtlanamayan uzun periyotlar şeritsiz skaler tam çalıştırmaya düşüyordu.
Düzeltme: sınırlar bir kez; geri bakış ölçülen katsayılardan; kanıt mesafesi pencereye
sığmayan periyotlar doğrudan şeritli tam taramaya ("hibrit").

**Ölçüm** (aarch64, `cargo bench -p qp-compute --bench profiles -- --only late`, 5 koşu
medyanı; son 500 bar istenir):

| Hesap | Bar | Periyot | Tam tarama (ms) | Hibrit (ms) | Hız | Geç başlangıç / kanıtlanan | Tam taramada kalan |
|-------|----:|--------:|----------------:|------------:|----:|---------------------------:|-------------------:|
| EMA | 10.000 | 1.000 | 8,54 | 8,12 | 1,1× | 378 / 378 | 622 |
| RSI | 10.000 | 1.000 | 22,86 | 22,44 | 1,0× | 192 / 192 | 808 |
| EMA | 1.000.000 | 100 | 118,2 | **2,66** | **44,5×** | 100 / 100 | 0 |
| RSI | 1.000.000 | 100 | 233,5 | **10,83** | **21,6×** | 100 / 100 | 0 |

Sınırların buluşması için gereken bar (10k seri): EMA p2=37, p5=92, p14=255, p20=378, p50=955,
p100=1.807, p200=3.937; RSI p2=58, p5=175, p14=522, p20=747, p50=1.935, p100=4.100, p200=7.721.
Kabaca EMA ≈ 19·p, RSI ≈ 38·p (büzülme oranlarıyla uyumlu: `1−a ≈ 1−2/p`, `1−1/p`).

**Bit-eşitlik neden kanıt.** Monotonluk + tümevarım: `L ≤ T ≤ U` her barda; `L = U` bit
eşitliği `T`'yi belirler; sonrasında aynı işlemler. Sınırların gerçek durumu kapsaması:
EMA durumu girdilerin ağırlıklı ortalaması, yuvarlama sapması göreli `≈ 2u/a ≤ 2u·p`
(p ≤ 1e9 için < 2,3e-7) — 1e-6 genişletme bunu kapsar; RSI ortalamaları negatif olmayan
değerlerden oluştuğu için alt sınır tam 0.

**Karar: al (deneysel API olarak), genişletme sonra.**
Uzun geçmişte (dakikalık barlarla yıllar ≈ 1M bar) "görünür pencere"yi isteyen sorgular ve
canlı grafiğe hesap ekleme (attach) için 20–45 kat. Bugün çekirdek tam uzunlukta çıktı
üretiyor (K13); pencere API'si bir ekleme. Sonraki adımlar:
- MACD (üç EMA; sinyal EMA'sının girdisi `fL−sU … fU−sL` aralığıyla sınırlanır; çıkarma her
  iki argümanda monoton) ve canlı `Streaming` durumunun geç başlangıçla kurulması (D profili
  attach).
- Sandviç yörüngelerinin şeritli (SoA) koşturulması: geç başlangıç kısmı da vektörleşir.
- SMA/Bollinger'in kayan toplamı büzülmez; bu yöntem onlara uygulanamaz (E7.5: fark kalıcı
  ~1e-15). Onlar için tanım değişikliği (tamsayı kuruş toplamı, E7.6) gerekir — ana akış kararı.
