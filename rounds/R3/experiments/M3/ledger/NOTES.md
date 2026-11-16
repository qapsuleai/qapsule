# M3 adayı `ledger`: "Karne"

Öncül: bir hikâyenin önemi, onu doğuran **karne anahtarının** (`tür/dedektör@sürüm`)
geçmişte ne kadar **gerçekleştiğidir**. Örneklem küçükken temkinli olunur. NORTH_STAR'ın
"ne kadar emin?" ve "daha önce oldu mu?" sorularına doğrudan bağlanır. Geleceğin yatırımcısı
için "az ama önemli" de şöyle okunur: yalnız geçmişi kanıtlanmış olanı öne çıkar.

Crate: `rounds/R3/experiments/M3/ledger/` (`m3_ledger::ranker()`, ad `m3.ledger`). Düzenekte
`--alt ledger` adıyla kayıtlı.

## Kural

Her şey `RankCtx`'ten okunur: `ledger` (bu anahtar) ve `ledger_all` (bütün anahtarlar).
İkisi de sıralanan bardan **kesin olarak önce** kapanan satırları sayar. Durum, saat ve
rastgelelik yoktur.

1. **Başvuru oranı** `b`: *öteki* anahtarların isabeti, yani (`ledger_all − ledger`) üstüne
   Beta(1,1) düzeltmesi. Hiç kapanış yoksa `0,5`. Anahtarın kendisi dışarıda tutulur;
   böylece büyük bir anahtar kendini kıyas noktasına çekmez.
2. **Sonsal:** anahtarın isabet oranı için önsel Beta(k·b, k·(1−b)), k = `prior_n` sözde
   kapanış. Anahtarın `realized / n`'i ile güncellenir. Yeni anahtar **başvuru oranından**
   başlar, üstünden değil; kademe 1'i kapanışlarla kazanması gerekir.
3. **Güven bandı:** `[lo, hi] = ortalama ∓ z·sd`. Beta sonsalının normal yaklaşımıdır;
   α, β ≥ k olduğundan yaklaşım yeterli.
4. **Kademe:**
   - `lo ≥ b + δ` → **1**: anahtarın ötekilerden daha sık gerçekleştiğinden %99 eminiz;
   - `hi < b` → **3**: anahtarın ötekilerden daha seyrek gerçekleştiğinden %99 eminiz;
   - aksi hâlde **2**: kanıt yok. Soğuk başlangıç da buraya düşer.
5. **Önem** = `lo` ∈ [0, 1], yani temkinli isabet oranı. Kademe içinde sıralama yapar.
   Cümleye "geçmişte en az şu sıklıkta gerçekleşti" diye girebilir (K32: nicel).

## Parametreler ve gerekçe

| Parametre | Değer | Neden |
|---|---|---|
| `prior_n` (k) | 50 | İsabetler 0,5 çevresinde olduğu için tek kapanışın sd'si ≈ 0,5. +0,1'lik bir farkı gürültüden ayırmak ~50 kapanış ister. Önsel ağırlığı bu ölçekte tutulunca erken şans serileri kademe 1'e çıkamaz. |
| `z` | 2,326 | Tek yanlı %99. Birkaç anahtar (sentetikte 7) birlikte sınandığından çoklu karşılaştırmaya karşı sıkı tutuldu. |
| `δ` | 0 | Ayrı bir "pratik fark" eşiği gerekmedi: k = 50 ve z = 2,326 fark küçükken zaten kademe vermiyor. Bir eşik az olunca keyfî sayı da bir az. |

**Hedef dikkat bütçesi** (30 enstrümanlık izleme listesi, `important`): günde ~0,5 kapsül,
yani haftada 2–3. Her biri kanıtlanmış bir geçmişle gelir. Gerekçe: NORTH_STAR geleceğin
yatırımcısı için "az ama önemli" diyor. Günde birkaç tane bildirim itme olur. Haftada 1'in
altı ise ürünü "ölü" gösterir: sıfır gün oranı 0,9'u aşar. `balanced` çok daha geniştir
(günde ~6): yalnız kanıtlanmış zayıf anahtarları (kademe 3) çıkarır.

### Duyarlılık taraması (sentetik, 3 tohum; important günlük ort. · K1 payı · K1 − K3)

Tek bir tohuma uydurmamak için bütün ızgara üç tohumda birlikte okundu. Seçim: tohumlar
arası en kararlı ve bütçeye uyan nokta.

| k | z | δ | s1 | s2 | s3 |
|---|---|---|---|---|---|
| 10 | 1,000 | 0,00 | 4,63 · 0,507 · +0,05 | 2,90 · 0,516 · +0,07 | 2,37 · 0,523 · +0,08 |
| 20 | 1,645 | 0,00 | 1,95 · 0,516 · +0,06 | 1,70 · 0,513 · +0,06 | 1,74 · 0,547 · +0,11 |
| 20 | 1,645 | 0,02 | 0,48 · 0,634 · +0,18 | 0,58 · 0,584 · +0,13 | 0,92 · 0,568 · +0,13 |
| 20 | 1,645 | 0,05 | 0,13 · 0,983 · +0,53 | 0,14 · 0,772 · +0,32 | 0,22 · 0,645 · +0,21 |
| 20 | 2,326 | 0,00 | 0,64 · 0,570 · +0,12 | 0,49 · 0,597 · +0,15 | 0,78 · 0,572 · +0,14 |
| **50** | **2,326** | **0,00** | **0,44 · 0,630 · +0,18** | **0,42 · 0,609 · +0,16** | **0,53 · 0,589 · +0,16** |
| 50 | 2,326 | 0,05 | 0,09 · 1,000 · +0,55 | 0,10 · 0,953 · +0,50 | 0,10 · 0,955 · +0,52 |

İki rejim var:

- Gevşek ayarlarda büyük yönlü anahtarlar (`breakout`, `continuation`) bütünüyle kademe 1'e
  girip çıkar. Bütçe günde 2–5'e çıkar ve tohuma göre değişir; K1'in üstünlüğü küçük kalır.
- Sıkı ayarlarda kademe 1 neredeyse yalnız `squeeze` olur: günde ~0,1 kapsül, K1 payı
  ≈ 1,0. Bu, aşağıdaki zayıflık 2'ye dayanır.

Seçilen nokta ikisinin arasındadır ve en dar tohum yayılımını verir.

## Son ölçüm tablosu (`harness-sim m3 run --alt ledger`)

Ham ve özet: `rounds/R3/experiments/M3/raw/ledger/summary-{synthetic,real}.md`. Gerçek veride
yalnız toplu istatistik var.

**Sentetik (30 × 500 × 3 tohum, havuz)**

| Seviye | Ort. | Medyan | p90 | En çok | Sıfır gün | DK | Rejim oranı | 5 günde tekrar | En uzun dizi |
|---|---|---|---|---|---|---|---|---|---|
| important | 0,46 | 0 | 2 | 8 | 0,726 | 2,08 | **1,37** | 0,130 | 3 |
| balanced | 6,28 | 6 | 10 | 20 | 0,002 | 0,46 | 1,98 | 0,677 | 5 |
| all | 10,69 | 10 | 16 | 28 | 0,001 | 0,36 | 1,73 | 0,834 | 6 |

important günlük ortalaması tohumlara göre 0,44 / 0,42 / 0,53; göreli fark 0,24.

| Veri | K1 kapanan · pay | K2 pay | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|
| havuz | 625 · 0,608 | 0,480 | 0,446 | 0,471 | **+0,162** |
| s1 / s2 / s3 | 192 · 0,630 / 197 · 0,609 / 236 · 0,589 | 0,48 | 0,45 / 0,45 / 0,43 | 0,47 | +0,18 / +0,16 / +0,16 |

İleriye bakma: **geçti** (9 kesim, kademe farkı 0).

Karşılaştırma için (README, aynı düzenek): başvuru `families` important 0,64/gün, rejim oranı
6,05, K1 − K3 +0,011. Control `round_robin` K1 − K3 −0,012.

**Gerçek veri (6 sembol)**

| Veri | important ort. · sıfır gün | balanced ort. | K1 kapanan · pay | K3 pay | Taban | K1 − K3 |
|---|---|---|---|---|---|---|
| real-1day | 0,04 · 0,968 | 1,21 | 77 · 1,000 | 0,503 | 0,546 | +0,497 |
| real-1hour | 0,32 · 0,766 | 7,89 | 156 · 0,987 | 0,445 | 0,493 | +0,542 |

İleriye bakma: **geçti**.

**Kademe 1'in bileşimi** (doğum sayısı; tanı için ayrı bir koşu, depoda değil)

- Sentetik:
  - `squeeze` 52–57 (~%75'i);
  - `extreme.rsi` 30–137;
  - `breakout` 11–59;
  - `continuation` yalnız s2'de 102.
  - `flow.institutional` hemen hemen hep kademe 3'te: 1.903–2.211 doğum, isabet ~0,43.
- Gerçek: kademe 1 **yalnız** `squeeze`.
  - Kademe 3: günlükte `breakout` (189); saatlikte `flow.volume_surge` (638), `breakout`
    (426), `reversal` (163).

## Zayıflıklar

1. **Soğuk başlangıç.** Yeni bir anahtar, yeni bir dedektör sürümü (`@2`) ya da yeni bir
   kullanıcı piyasası, ~50+ kapanış birikene kadar kademe 1 vermez.
   - Gerçek günlük veride ilk kademe 1 **bar 526'da** geldi, yani ~2 yıl sonra.
   - Saatlikte bar 523'te geldi. Sentetikte (30 enstrüman) bar 35–265 arasında.
   - O süre boyunca `important` boştur. Kural bunu bilerek seçer ("emin değilsem
     söylemem"), ama ürün için ciddi bir sorundur.
   - Çare kural dışındadır: dedektör sürümleri arasında karne aktarımı, ya da geçmiş
     veride önceden koşulmuş bir karne (geriye dönük ısınma). İleriye bakma kuralını
     bozmadan yapılabilir, çünkü geçmiş barlar zaten geçmiştir.
2. **Anahtarlar arası kıyas elma-armut olabilir.** `squeeze` nötrdür. Başvuru sonuç kuralı
   nötr hikâyeyi "\|hareket\| ≥ 1 ATR" ile gerçekleşmiş sayar; bu, yönlü "önce +1 ATR"
   koşulundan çok daha kolaydır.
   - `squeeze`in ~%95 isabeti kısmen bu tanımdan gelir. Gerçek verideki K1 ≈ 1,0 bu yüzden
     abartılı bir "bilgi" göstergesidir.
   - Sentetikteki +0,16, `extreme` ve `breakout` katkısıyla daha dürüst bir okumadır.
   - Kural yalnız `ledger_all` ile kıyaslayabiliyor; sonuç sınıfına göre ayrı bir taban
     (ör. nötr ↔ yönlü) yok.
   - M5'in seçeceği sonuç kuralı bu tabloyu değiştirir; M3 seçimi onunla yeniden koşulmalı.
3. **Taneciklik anahtar düzeyindedir.**
   - Aynı anahtarın bütün hikâyeleri aynı barda aynı kademeyi alır. Hikâyenin kendi
     teyidi, gücü ya da enstrümanı kademeyi değiştirmez.
   - Bütçe anahtar sıklıklarıyla kuantalanır ve "ince ayar" yapılamaz. Büyük bir anahtar
     (`breakout` ≈ 3/gün) eşiği geçtiği anda `important` sıçrar. Sentetikteki "en çok 8"
     ve DK 2,08 buradan gelir.
   - Durumsuz kural histerezis de kuramaz; eşik çevresindeki anahtar zamanla girip
     çıkabilir.
4. **Yön karneye girmiyor.** Anahtar `tür/dedektör@sürüm`; yön yok.
   - Gerçek günlükte `breakout` yukarı ~0,60, aşağı ~0,40 gerçekleşiyor (tanı koşusu).
     Karne bunları ortalayıp kanıtlanmış zayıf (kademe 3) sayıyor ve yukarı kırılımların
     değerini kaybediyor.
   - Bu, sözleşmenin (`RankCtx`) sınırıdır, kuralın değil.
5. **Tekrar.** important'ta 5 günde tekrar 0,130. Başvurudan (0,080) yüksek, çünkü aynı
   anahtar aynı enstrümanda art arda doğabilir; kural enstrümana bakmaz.
6. **Taban çok anahtarlıdır.** Hangi anahtarın "iyi" sayılacağı ötekilere bağlıdır. Çok
   sık ve zayıf bir anahtar (sentetikte `flow.institutional`, doğumların %42'si) tabanı
   aşağı çeker ve ötekileri kolaylaştırır. Sunucu olgusu olan bu anahtar lisanssız gerçek
   veride yoktur; iki veri kümesinde taban farklı kurulur.

## Öneriler (kapsam dışı)

- Karne anahtarına yön ekle (`tür/dedektör@sürüm/yön`), ya da `RankCtx`'e yöne göre karne
  koy. Gerçek veride en büyük kazanç burada görünüyor.
- `RankCtx`'e sonuç sınıfına göre (nötr / yönlü) ayrı bir `ledger_all` ekle; zayıflık 2'yi
  kapatır.
- Soğuk başlangıç için geriye dönük ısınmış karne. Yerleşik yatırımcıya sıfırdan değil,
  geçmiş barlarla doldurulmuş karneyle başlanır.
- Karne öncülü başka bir öncülle (ör. hikâyenin kendi teyidi) çarpılarak birleştirilebilir:
  karne "hangi tür güvenilir", öteki "bu örnek ne kadar güçlü" sorusunu cevaplar. Bu
  adayın işi değil, M3 bütünleştirmesinin işi.
