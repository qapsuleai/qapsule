# M20: açılmış kapsül ve kanıt ekranı, ölçüm sonucu

Ölçüm ajanı `r5/m20-measure`, 2026-11-18. Düzenek: `rounds/R5/experiments/harness/glance/`. SPEC §G
kuralları geçerli; **Ek G7** ölçümden önce yazıldı ve commit edildi (655e93e). Ek G7'nin
içeriği: (a) iki derinlik, birincil ölçü kapsül derinliğinde; (b) deterministik dokunma sayısı;
(c) K52 aynı ekranda; (d) `e_who` anahtarı; (e) rakam taraması; (f) sınırlar. Bu commit'ten sonra
görüntü çekildi ve okuyucu çağrıldı. Ham özet [`run/`](run/) altında (ekran görüntüsü yok).
Bütün tablolar `run/tables.md` dosyasında; [`tables.py`](tables.py) ile üretildi.

> Ajan okuyucu insan değildir; bu ölçüm bir ön elemedir (SPEC §G0). Son söz Arkin'in
> telefondaki karşılaştırması ve insan testindedir.

## Özet

| | `tesir` | `nobra` | `r4` (başvuru) | bozuk |
|---|---|---|---|---|
| **Birincil**: kapsül derinliğinde `evidence` + `evidence_visitor` (F2:H1, %95) | **0,968 [0,949, 0,987]** | 0,848 [0,812, 0,884] | 0,732 [0,679, 0,784] | 0,726 [0,673, 0,777] |
| yetkili okur / yetkisiz okur | 0,942 / 0,993 | **0,698** / 1,000 | 0,964 / **0,500** | 0,951 / 0,500 |
| İkincil: kanıt derinliği (F2:H1, %95) | 0,952 [0,915, 0,981] | 0,975 [0,954, 0,993] | 0,972 [0,954, 0,991] | 0,972 [0,947, 0,993] |
| Eleştirmen BT (%95) | 1,81 [1,20, 2,46] | **2,41 [1,80, 3,35]** | −0,82 [−1,23, −0,46] | −3,41 [−3,80, −2,80] |
| Dokunma: dayanak / kim doğruladı / neden rakam yok | 1 / 1 / 1 | 1 / 1 / 1 | 1 / 1 / **2** (yetkisizde dayanak da 2) | — |
| K52 kapsül derinliğinde (JIT açık ve kapalı) | geçti | geçti | geçti | — |
| Yetkisizde rakam (sert) | 0 | 0 | 0 | 0 |

**Seçim (önceden yazılı kural, Ek G7d'deki sıkı `e_who` anahtarı):** sert eşikler kimseyi elemiyor.
Birincilde `tesir` tek başına ilk grupta: `nobra`'nın üst sınırı 0,884, `tesir`'in alt sınırı
0,949'un altında kalıyor. **Kazanan `tesir`**, ayırt edildi. İlk iki (gösterimde seçilebilir
kalır): **`tesir`, `nobra`**. Bütünleştirmeye girecek varsayılan: **`tesir`**.

**Kararı tek bir anahtar seçimi belirliyor (aşağıda "Açık soru 1").** `nobra`'nın yetkili okurdaki
düşüklüğünün tamamı `e_who` sorusundan geliyor. Bu soruda 48 cevabın 44'ü "sunucu doğruladı";
anahtar ise "bu cihazda sınandı". `nobra`'nın cümlesi "Sunucu doğruladı ve bu cihazda da
sınandı." ve K48'e göre ikisi de doğru. Bilgi olarak koşulan gevşek anahtarla (sunucu ∪ cihaz)
durum tersine dönüyor: `nobra` 0,998 [0,993, 1,000], `tesir` 0,979 [0,963, 0,992]. Aralıklar
ayrık, kazanan **`nobra`** olurdu, ilk iki yine aynı (`run/sensitivity-loose-who.json`). Ek G7d
sıkı anahtarı ölçümden önce, GOAL'ın "sunucu doğruladı ↔ bu cihazda sınandı ayrımı anlaşılıyor"
ölçütüne dayanarak seçti. Kural bu yüzden `tesir` der. Ama bu anahtarın sonucu belirlediği açıkça
yazılmalı.

Bozuk tasarım birincilde ve BT'de kesin son, yani koşu geçerli (§G8). Ama `r4`'ten ancak 0,006
farkla ayrılıyor; eşleştirilmiş fark ayrık değil (M18'deki gibi, bkz. "Sorunlar 2").

Maliyet: **≈ 1,93 $**. Okuyucu 970 çağrı ≈ 0,90 $ (haiku 954 + deneme 4: 0,71 $; sonnet 16:
0,19 $). Eleştirmen 72 sonnet çağrısı 1,03 $. Bütçe 15 $.

## Düzenek ve protokol (okumalardan önce sabitlendi)

**Adaylar ve derlemeler.** Dallar birleştirilmedi. Her aday kendi dalının `git archive`
kopyasından ayrı derlendi: kopya scratchpad'de, depo dışında; `qp` release + `scripts/demo.sh`
WASM paketi; `CARGO_TARGET_DIR=target/wt/m20m-<ad>`.

| ad | kaynak | seçim |
|---|---|---|
| `tesir` | `r5/m20-tesir` 4f1b091 | `?open=tesir` |
| `nobra` | `r5/m20-nobra` b1ade54 | `?open=nobra` |
| `r4` (başvuru) | `main` d8f83c5 + `broken/look.patch` (yalnız bozuk görünüş kaydı eklenir) | parametresiz (bugünkü kart) |
| `kestim` (bozuk, Ek G3; gizli ad) | `r4` ile aynı derleme | `?look=kestim` (açılmış kapsül `r4`) |

İki aday dalı `d28cd2c`'den çıkıyor. `main` o noktadan sonra yalnız belge aldı (`crates`, `shells`,
`scripts` farkı yok), yani başvurunun kodu adayların tabanıyla aynı. Bütün görüntüler `look=r4`
ve `voice=marla` ile çekildi: kayıtta istenen ses = gönderilen ses = `marla`.

**Durumlar** (§G2, `probe` + `states`, `r4` derlemesi; S* = adım 1):
- `live_event` = `cap.SYN06.breakout.up.118`, yetkili okur `arkin`. Kanıt aileleri price + volume;
  açık kapsülün güveni "Bu cihazda sınandı".
- `visitor` = `cap.SYN28.continuation.down.118`, `principal=visitor`. Veri yetkisi yok; güven
  "Sunucu doğruladı".

Her tasarım için 2 durum × 2 derinlik (kapsül, kanıt) × 2 dil × 2 tema = 16 görüntü; toplam 64
(402×874, DPR 1, `canvas=run`). Hedef kapsül, dokunma (kapsül 1, kanıt 2) ve kanıt satırları
dört derlemede aynı (`run/key.json`, `run/states.json`). Kanıt derinliği tasarımlar arasında aynı
ekrandır: iki aday da onu değiştirmedi, hepsinde 390×300 çerçeve.

**Sorular** (`gl/keys.py`; anahtarlar motor verisinden):

| kod | soru | anahtar | şans |
|---|---|---|---|
| `e_rests` | Bu iddia neye dayanıyor? | kanıt satırlarının aileleri (price, volume) | 0,20 |
| `e_who` | Bunu kim doğruladı? | `open().trust` (Ek G7d): yetkili okurda `device`, yetkisizde `server` | 0,333 |
| `valid` | geçerlilik (5 seçenek) | `valid` | 0,20 |
| `numbers` | Kapsülde rakam görüyor musun? ("neden rakam göremiyorum") | `no_license`: hayır, ekran bunun yetki yüzünden olduğunu söylüyor | 0,333 |

Okuyucu: haiku, personalar F ve H. Kapsül derinliğinde 6 tekrar (birincil, Ek G3-2); kanıt
derinliğinde 3; kısma kopyasında 6. Sonnet alt kümesi: kapsül derinliği, tr/açık, 1 tekrar.
Tek bakış dışı 6 çağrı puanlamadan çıkarıldı. Eleştirmen: sonnet, 6 çift × 2 durum (kapsül
derinliği) × 2 sıra × 3 tekrar, tr/açık. İstemde tasarım adı yok (`check_prompt`).

### Uygulama notları (sonuçtan bağımsız; okuyucu ve eleştirmen sonuçları görülmeden)

1. **Rakam taraması (Ek G7e) dar kurulmuştu.** İlk taramada her tasarımda 24 isabet çıktı.
   Hepsi ya uygulamanın başlıktaki oynatma saati (`#clock`, "2023-06-19 10:00") ya da kanıt
   derinliğindeki veri adıydı (`rests_on` = "bars SYN28 1d (feed default)"; dört tasarımda aynı
   öğe). Bunlar kapsül değeri değil. Saat ve kanıt satırının `requires`/`rests_on` adları dışarıda
   bırakıldı ve her tasarımda sonuç 0 oldu. Lafzi okumayla da sonuç değişmezdi: dört tasarım
   birden kalırdı, eşik uygulanmazdı (§G9.1).
2. **Adım 1b** (yüz okuma kapısı) Ek G7a'ya göre M20'de yok. Düzenek `--reading` kümesi olarak
   kanıt derinliğini aldığı için 1b o kümeyle koşuldu ve kimseyi elemedi; sonuç aynı.
3. K52'nin ilk koşusunda kabukta `--open` parametresi kayboldu (zsh'de tırnaksız değişken) ve
   adaylar `r4` ekranıyla ölçüldü. O koşu atıldı, üç tasarım doğru parametreyle yeniden ölçüldü.

## Deterministik kapı (§G7; açık sahnenin boyama listesi + DOM, iki derinlik)

| tasarım | metin karşıtlığı en kötü / eşik | metin dışı en kötü | en küçük yazı | dokunma < 44 | renk: geçemeyen çift (açık/koyu) | yetkisizde rakam | ekran karakteri ort. |
|---|---|---|---|---:|---|---|---:|
| `tesir` | 1,65 / 4,5 (kanıt derinliği "Bu kanıt şu yetkiyi gerektirir:") | 2,32 (up, çizgi) | 11 | 72 | 3/4 | 0 | 485 |
| `nobra` | 1,65 / 4,5 (aynı öğe) | 2,32 | 11 | 72 | 3/4 | 0 | 618 |
| `r4` | 1,65 / 4,5 (aynı öğe) | 2,32 | 11 | 72 | 5/6 | 0 | 318 |
| bozuk | 1,20 / 4,5 | 1,21 | 11 | 68 | 10/10 | 0 | 312 |

**E6 (§G9.1):** metin karşıtlığı, metin dışı karşıtlık, dokunma ve renk eşiklerini hiçbir aday,
`r4` dahil, geçemedi; bu dört eşik uygulanmadı. Ortak nedenler M18'dekiyle aynı: kabuğun
düğmeleri (Geri, Kanıt, sekmeler < 44 px), kanıt derinliğinin yetki satırı (1,65:1, her tasarımda
aynı DOM öğesi), grafiğin `up` çizgisi. En küçük yazı (11 px) herkeste geçti. Yetkisizde rakam
herkeste 0: `visitor` görüntülerinde DOM metni ve sahne metni tarandı, 8'er görüntü. Kalan sert
eşik K52.

## K52, aynı ekranda: kapsül derinliği (Ek G7c, `k52open.mjs`, 3 tekrarın ortancası)

| tasarım | kip | çerçeve | tik p95 | sahne boyama p95 | dururken uzun kare | tekerlek evresi (kip, kare p99, uzun kare payı) | ilk kapsül | geçti |
|---|---|---|---|---|---|---|---|---|
| `tesir` | JIT açık (`phone-jit-est`) | 390×670 | 1 ms | — (evrede boyama yok) | 0 | scroll, 18 ms, 0 | 308 ms | evet |
| `tesir` | JIT kapalı (`phone`) | 390×670 | 1 ms | — | 0,002 | scroll, 19 ms, 0 | 337 ms | evet |
| `nobra` | JIT açık | 390×560 | 1 ms | 3 ms (37 boyama/10 s) | 0 | zoom, 18 ms, 0 | 321 ms | evet |
| `nobra` | JIT kapalı | 390×560 | 1 ms | 11 ms (sapmalı ≈ 20) | 0,015 | zoom, 19 ms, 0 (0 / 0 / 0,003) | 348 ms | evet |
| `r4` | JIT açık | 390×300 | 1 ms | 2 ms (37/10 s) | 0 | zoom, 18 ms, 0 | 336 ms | evet |
| `r4` | JIT kapalı | 390×300 | 1 ms | 4 ms (sapmalı ≈ 7) | 0,002 | zoom, 19 ms, 0 | 354 ms | evet |

Okuma:
- Üç tasarım da K52'yi geçiyor.
- `tesir`'in uzun sütunu tekerlekle gerçekten kayıyor (`scroll` kipi, uzun kare 0). `r4` ve
  `nobra`'da tekerlek kapsül derinliğinde hiçbir şeyi kaydırmıyor: kabuk onu `zoom` kipi sayıyor,
  ölçülen şey hareketsiz bir ekranın kareleri.
- `tesir` 10 saniyelik oynatmada sahneyi hiç yeniden boyamadı: küçük resim kanıtın kendi
  penceresini gösteriyor ve tasarım notuna göre yalnız yeni barda boyanıyor. Bu yüzden sahne
  boyama maliyeti bu koşuda örneklenmedi. Tasarımın kendi ölçümü 2 ms (JIT açık) ve 10 ms (JIT
  kapalı); bar başına bir uzun kare riskini de not ediyor.
- `nobra` JIT kapalıyken her tikte büyük tuvali boyuyor: sahne p95 11 ms, sapmayla ≈ 20 ms.
  Dururken uzun kare %1,5. K52'nin JIT kapalı maddesi ("kullanılabilir", ≤ %10) geçiyor.
- Makine başka ajanlarla paylaşılıyordu (yük 3,5–7,9).

## Görev doğruluğu

### Birincil: kapsül derinliği (F2:H1; birim = görüntü × persona × tekrar × model)

| tasarım | hepsi | yetkili | yetkisiz | F | H | birim F/H |
|---|---|---|---|---|---|---|
| `tesir` | **0,968 [0,949, 0,987]** | 0,942 | 0,993 | 0,980 | 0,943 | 50/50 |
| `nobra` | 0,848 [0,812, 0,884] | 0,698 | 1,000 | 0,853 | 0,837 | 50/49 |
| `r4` | 0,732 [0,679, 0,784] | 0,964 | 0,500 | 0,750 | 0,697 | 50/50 |
| bozuk | 0,726 [0,673, 0,777] | 0,951 | 0,500 | 0,750 | 0,677 | 50/50 |

### Soru başına (haiku, F+H; Wilson %95; n ≈ 48)

| tasarım | derinlik | `e_rests` | `e_who` yetkili (anahtar cihaz) | `e_who` yetkisiz (anahtar sunucu) | `valid` | `numbers` | `e_who` gevşek (bilgi) |
|---|---|---|---|---|---|---|---|
| `tesir` | kapsül | 1,00 | 0,94 [0,83, 0,98] | 1,00 | 0,85 | 0,98 | 1,00 |
| `nobra` | kapsül | 1,00 | **0,08 [0,03, 0,20]** | 1,00 | 0,98 | 1,00 | 1,00 |
| `r4` | kapsül | 1,00 | 1,00 | 1,00 | 0,83 | **0,00** | 1,00 |
| bozuk | kapsül | 1,00 | 1,00 | 1,00 | 0,77 | 0,00 | 1,00 |
| `tesir` | kanıt | 1,00 | 1,00 | 1,00 | 0,70 | 0,96 | 1,00 |
| `nobra` | kanıt | 1,00 | 1,00 | 1,00 | 0,83 | 0,96 | 1,00 |
| `r4` | kanıt | 1,00 | 1,00 | 1,00 | 0,75 | 1,00 | 1,00 |

Cevap dağılımları (kapsül derinliği):
- **`e_who`, yetkili okur.** `tesir`: cihaz 45, sunucu 3. `nobra`: sunucu 44, cihaz 4. `r4`:
  cihaz 48; `r4` bunu üst çubuktaki küçük "Bu cihazda sınandı" yazısından okuyor. Okur, `nobra`'nın
  "Sunucu doğruladı ve bu cihazda da sınandı" cümlesinin ilk yarısını alıyor. `tesir` cümleyi
  ters sırada kuruyor: büyük "Bu cihazda sınandı", altında "Sunucu da doğruladı".
- **`numbers`, yetkisiz okur.** `r4` ve bozuk: 48'in 48'i "hayır; nedenini söylemiyor". Bugünkü
  kart "neden rakam göremiyorum" sorusunu kapsül derinliğinde cevaplamıyor; ancak kanıt
  derinliğinde cevaplıyor (1,00). İki aday bunu kapsül derinliğinde cevaplıyor: `tesir` 0,98,
  `nobra` 1,00.
- **`e_rests`** her tasarımda ve iki derinlikte ≈ 1. Okuyucu "fiyat" cevabını başlık cümlesinden
  ("20 günün zirvesini aştı") veriyor. Bu soru tasarımları ayırmıyor.

**Kısma kopyası** (bilgi; σ 4,5). Doğruluk `tesir` 0,196, `nobra` 0,104, `r4` 0,120, bozuk
0,107. Okuyucular bulanık açılmış kapsülde yön ve geçerlilik için çoğunlukla "bilmiyorum"
diyor. `t_valid` her tasarımda ≈ 0. İlk göze çarpan: `nobra`'da grafik (43/48), `tesir` ve
`r4`'te cümle. Açılmış kapsül tek bakış ekranı değil; bu küme bu konuda bilgi vermiyor.

**İkinci boy (sonnet, tr/açık, 1 tekrar).** `tesir` 1,000, `nobra` 0,889, `r4` 0,750, bozuk
0,750. Haiku ile uyum 0,925 (n = 40), sıralama aynı yönde.

## Dokunma sayısı (Ek G7b, `taps.mjs`; tr/açık)

| tasarım | neye dayanıyor (yetkili) | kim doğruladı (yetkili) | neye dayanıyor (yetkisiz) | kim doğruladı (yetkisiz) | neden rakam göremiyorum (yetkisiz) |
|---|---|---|---|---|---|
| `tesir` | 1 | 1 | 1 | 1 | 1 |
| `nobra` | 1 | 1 | 1 | 1 | 1 |
| `r4` | 1 | 1 | 2 (kanıt derinliği) | 1 | **2** (kanıt derinliği) |

Hiçbir cevap kaydırma gerektirmedi. İki adayda bütün cevaplar kapsülü açan tek dokunuşta
görünüyor, ızgara araması gerekmedi. `nobra`'nın kısa cevapları kart kapalıyken görünür; ayrıntı
için bir dokunuş daha gerekir. Ayrıntılar yüklemlere girmedi.

`r4`'te yetkisiz okur için 362 aday dokunuş denendi (ızgara + DOM, 3 ekran). En kısa yol: kapsül
→ "Kanıt" düğmesi → kanıt derinliğinin yetki satırı, toplam 2 dokunuş.

Sınır: `r4`'te yetkili okurun "dayanak" yüklemi (aile adı) ayrıntı cümlesindeki "hacim" sözcüğüyle
1'de sağlandı. Yüklem kaba bir ölçü: görüntüde aile adı geçiyor mu, ona bakar; o adın gerekçe
olarak sunulup sunulmadığına bakmaz.

## Eleştirmen (ikili kör, sonnet; kapsül derinliği)

| tasarım | BT [%95] | kazandı / kaybetti |
|---|---|---|
| `nobra` | 2,41 [1,80, 3,35] | 32 / 4 |
| `tesir` | 1,81 [1,20, 2,46] | 28 / 8 |
| `r4` | −0,82 [−1,23, −0,46] | 12 / 24 |
| bozuk | −3,41 [−3,80, −2,80] | 0 / 36 |

**Çelişki:** `nobra | tesir` 2/6 = 0,333 > %30, bu çift **ayırt edilemedi**. Öteki çiftlerde 0.

**Uzunluk yanlılığı:** BT ile ekran karakteri arasında Spearman 1,0. Kesin sonuçlarda daha çok
metinli olanın kazanma payı 0,971 (n = 34) > 0,7, eleştirmen sonucu **"uzunluk yanlılığı
şüpheli"**. Eleştirmen sırası (`nobra` 618 > `tesir` 485 > `r4` 318 > bozuk 312 karakter) ekran
metni sırasının tam aynısı. BT seçimde kullanılmadı: birincil tek aday bıraktı.

## Yetkisiz okur

- **Veri yetkisi olmayan okur** (`visitor`; sentetik dünyadaki tek yetkisiz okur):
  - `tesir` yetkisiz okurda sahneyi metin sütunuyla değiştiriyor; `nobra` aynı soruları DOM
    kartları olarak listeliyor ve "Neden rakam göremiyorum? Kanıtın değerleri veri yetkisi
    gerektiriyor." diyor. İkisi de üç soruyu tek dokunuşta cevaplıyor (doğruluk 0,993 / 1,000).
  - `r4` bugün "kim doğruladı" sorusunu cevaplıyor, "neden rakam yok" sorusunu kapsül
    derinliğinde hiç cevaplamıyor (0,00); kanıt derinliğinde 2 dokunuşla cevaplıyor.
  - Rakam taraması: üç tasarımda 0.
- **Değerleri gizli ama barları açık okur** (lisanslı sunucu olgusu) sentetik dünyada **yok**. Bu
  durum ölçülemedi; bu bir sınır. `nobra` bu okur için sahnede ayrı bir "Neden rakam
  göremiyorum?" kartı kuruyor; `tesir` dayanak satırını "Kilitli: flow/SYN03 yetkisi" şablonuna
  çeviriyor. İkisi de yalnız birim sınamasıyla gösterildi (tasarım notları); bu ölçümde yoktur.

## Bozuk tasarımın yeri (§G8, Ek G3)

`broken-check` sonucu:
- birincilde **kesin son**: 0,726, `r4` 0,732;
- BT'de **kesin son**: −3,41;
- birincil aralığı en iyinin aralığının altında.

Koşu **geçerli**. Eşleştirilmiş birincil fark adaylara karşı ayrık: `tesir` +0,24 [0,18, 0,30],
`nobra` +0,12 [0,04, 0,20]. **`r4`'e karşı ayrık değil:** +0,007 [−0,006, +0,020]. Bozuk kayıt
yine `r4` üstüne kuruldu (Ek G7f) ve `r4` ile aynı soruları kaybediyor. Bozuk kayıt yalnız
okunurluğu düşürüyor; M20 sorularında içerik belirleyici, okunurluk değil.

## Seçim kuralının adım adım uygulanışı (§G9 + Ek G7)

1. **Sert eşikler.** Metin karşıtlığı, metin dışı karşıtlık, dokunma ve renk eşiklerini kimse
   geçemedi; uygulanmadı (E6). En küçük yazı herkeste geçti. Yetkisizde rakam herkeste 0. K52
   herkeste geçti. Kalan: `tesir`, `nobra`, `r4`. Bozuk seçime girmez.
   1b. Ek G7a'ya göre yok. Kanıt derinliği kümesiyle koşulduğunda da kimseyi elemiyor.
2. **Birincil** (kapsül derinliği, F2:H1, %95): `tesir` 0,968 [0,949, 0,987] > `nobra` 0,848
   [0,812, 0,884] > `r4` 0,732. En iyinin aralığıyla örtüşen yalnız `tesir` var; ilk grup
   {`tesir`}.
3. BT ve 4. kanıt derinliği: ilk grupta tek aday olduğu için gerekmedi.
5. **Kazanan `tesir`**, ayırt edildi. Bağ çözme sırası nokta tahminleriyle işletildi (birincil →
   BT → kanıt derinliği → daha az metin → ad): `tesir`, `nobra`, `r4`. **İlk iki `tesir`,
   `nobra`**; **varsayılan `tesir`**.

**Duyarlılık (bilgi; gevşek `e_who`, sunucu ∪ cihaz).** 1 aynı. 2'de `nobra` 0,998 [0,993,
1,000], `tesir` 0,979 [0,963, 0,992], ilk grup {`nobra`}. Kazanan **`nobra`**, ilk iki `nobra`,
`tesir`. İki okumada da ilk ikinin üyeleri aynı; yalnız varsayılan değişiyor.

## Açık sorular

1. **`e_who` anahtarı ve varsayılan.** Ek G7d sıkı anahtarı ölçümden önce yazdı. Gerekçe GOAL
   8'deki "sunucu doğruladı ile bu cihazda sınandı ayrımı anlaşılıyor" ölçütüydü; buna göre
   açılmış açık-veri kapsülünün en belirgin cevabı "cihazda sınandı".
   - `nobra` iki olguyu bir cümlede ve sunucu önde veriyor. Okuyucu bunu "sunucu" diye okuyor;
     bu yanlış okuma değil, ama ayrım öne çıkmıyor.
   - Kural `tesir` der. Gevşek anahtarla sonuç `nobra` olur.
   - Orkestratör ya kuralı uygular (varsayılan `tesir`) ya da GOAL ölçütünü gevşek okur. Ayrıca
     `nobra`'nın cümlesinin sırasını değiştirmek ("Bu cihazda sınandı; sunucu da doğruladı")
     tasarım düzeyinde ucuz bir düzeltme olur.
   - İlk ikinin ikisi de gösterimde kalıyor.
2. Değerleri gizli ama barları açık okur sentetik dünyada yok. A32'nin "şablonlu rakamsız yüz"
   maddesinin bu yarısı insan testinde ya da lisanslı bir sentetik olguyla ölçülmeli.
3. `tesir`'in sahne boyaması bu K52 penceresinde örneklenmedi (yeni bar boyaması yok).
   Bütünleştirmede yeni bar anındaki uzun kare yeniden ölçülmeli.

## Sorunlar ve sınırlar

1. **Tavan etkisi.** Ajan okuyucu küçük metni okuyor: `e_rests` her yerde ≈ 1, kanıt derinliği
   her tasarımda ≈ 0,95–0,98. Ayrımı yalnız iki soru yapıyor: `e_who` (cümle sırası) ve `numbers`
   (bilgi var mı yok mu). Durum kümesi iki kapsül (§G2), tek sentetik pazar, `server` güvenli
   yetkili kapsül yok (§G11.7).
2. **Bozuk tasarım `r4`'ten ayrılmıyor** (M18'deki gibi). Bu koşunun düzeneği, adaylar
   arasındaki duyarlılığı bozuk kayıtla sınamıyor. Sonraki ölçümde bozuk kayıt bir adayın
   üstüne kurulmalı.
3. **Eleştirmen uzunluk yanlılığı şüpheli** (0,971). BT kararı belirlemedi.
4. **Görünüş ve dil seçilmedi:** bütün ölçüm `look=r4`, `voice=marla` ile yapıldı. Seçilen
   görünüşle yeniden bakılmalı.
5. **Kısma kopyası bu konuda bilgi vermiyor:** okuyucular çoğunlukla "bilmiyorum" dedi.
6. **K52 makinesi paylaşımlıydı.** JIT kapalı değerler WebKit benzetimi (RP3 kalibrasyonu). K52
   açılmış kapsülün yalnız kapsül derinliğinde ölçüldü; akış ve grafik evreleri adayların kendi
   standart K52 ölçümünde geçmişti (tasarım notları), burada yeniden koşulmadı.
7. **Kesilen cevaplar** (ilk cümle / 200 karakter): `nobra` 12, `tesir` 5, `r4` 4, bozuk 4. Puan
   harften alındığı için etkisi yok.

## Düzenek ekleri (bu dalda)

- `SPEC.md` Ek G7 (ölçümden önce, 655e93e).
- `shoot.mjs`:
  - `?open=` parametresi;
  - `capsule` türü (hedef yüze tek dokunuş);
  - çekimden sonra kanıt derinliğinden kanıt satırlarını okuyup kayda `evidence_key` olarak
    yazma (yalnız anahtar için).
- `glance.py`:
  - tasarım başına derleme (`designs.json`'da `qp`, `web`, `open`);
  - `--kinds capsule`;
  - kapsül derinliği kümeleri (`cap_evidence`, `cap_evidence_visitor`; 6 tekrar);
  - kapsül derinliğinin kısma kopyası ve eleştirmen çiftleri;
  - `who_key` (Ek G7d);
  - `compute`: açık sahne ölçüleri, `numbers_visitor`, `k52open` biçimi.
- `gl/keys.py`: kümeler. `gl/compute.py`: `digit_words`, `visitor_digits` (Ek G7e).
  `gl/select.py`: `numbers_visitor` sert eşik.
- `taps.mjs` (Ek G7b), `k52open.mjs` (Ek G7c); `tests/test_m20.py`. Birim testleri: 52 test geçti.
- [`tables.py`](tables.py), [`sensitivity.py`](sensitivity.py).

Yeniden üretim: `probe` → `states` (`r4` derlemesi) → `shoot --kinds capsule,evidence --voice
marla` (designs.json, tasarım başına derleme) → `derive` → `compute` → `taps.mjs` (tasarım
başına) → `k52open.mjs` (tasarım başına, `--open`) → `read` (haiku; sonnet tr/açık 1 tekrar
`--sets cap_evidence,cap_evidence_visitor`) → `critic` → `score --broken <gizli> --primary
cap_evidence,cap_evidence_visitor --reading evidence,evidence_visitor` → `broken-check` →
`tables.py`, `sensitivity.py`.
