# M21 — hareket: yeniden ölçüm sonucu (v2)

Ölçüm ajanı `r5/m21-remeasure`, 2026-11-18. Düzenek: `rounds/R5/experiments/harness/glance/`, SPEC §G +
Ek G6 + **Ek G8** (okumalardan önce commit `96885a4`) + **G8-9** (mekanik denemeden sonra, okumalardan önce,
`ec0967a`). Sürücü `m21v2.py`. Ham özet (ekran görüntüsü yok): [`run-v2/`](run-v2/); bütün tablolar
`run-v2/tables.md` (`tables_v2.py` ile üretildi). İlk (geçersiz) koşu: [`RESULTS.md`](RESULTS.md).

> Ajan okuyucu insan değildir; bu ölçüm bir ön elemedir (SPEC §G0). Son söz Arkin'in telefondaki
> karşılaştırması ve insan testindedir.

## Özet

| | `surek` | `nabiz` | `r4` | `still` (hareketsiz) | bozuk hareket |
|---|---|---|---|---|---|
| Geçerlilik: `m_from` son kareyle (şans 0,25, tek yönlü) | 0,000 [0, 0,010] geçerli | 0,000 [0, 0,010] geçerli | 0,000 [0, 0,010] geçerli | 0,003 [0, 0,014] geçerli (soru herkes için geçerli) | 0,000 geçerli |
| **Süreklilik `m_from` (birincil, F2:H1)** | 0,573 [0,524, 0,625] | **0,698 [0,651, 0,744]** | 0,373 [0,324, 0,424] | yok (ara karesi yok) | 0,020 [0,007, 0,038] |
| Fark edilme: isabet − yanlış alarm | −0,019 [−0,073, 0,031] | **0,113 [0,047, 0,181]** | −0,024 [−0,080, 0,031] | −0,002 [−0,057, 0,054] | −0,071 [−0,120, −0,021] |
| Eleştirmen BT | −0,03 [−0,27, 0,21] | **1,45 [1,08, 1,92]** | −0,36 [−0,67, −0,06] | 0,70 [0,41, 1,04] | −1,76 [−2,31, −1,38] |
| K52 standart (JIT açık) | geçti | geçti | geçti | geçti | — |
| K52 geçiş, JIT açık (≤ %1) | %1,19 ✗ | %1,30 ✗ | %1,30 ✗ | %1,19 ✗ | — (eşik hiçbir tasarımda tutmadı → uygulanmadı, §G9.1) |
| K52 geçiş, JIT kapalı (≤ %10) | %1,8 geçti | %1,8 geçti | %1,9 geçti | %1,6 geçti | — |
| Hareketi azalt bilgi kaybı (G6-5 lafzı, metin) | **0** | **4** ((a) durağan metin) → elenir | 0 | 0 (tanım gereği) | — |

**Düzenek sınamaları geçti:** soru bütün tasarımlarda son kareyle cevaplanamıyor (okuyucular son karede
%87–89 "bilmiyorum", %11–13 "olduğu yerde beliriyor" diyor); bozuk hareket birincilde, BT'de ve fark
edilmede **kesin son**. Koşu geçerli.

**Önceden yazılı kuralın sonucu: kazanan `surek`** (varsayılan `surek`, ilk iki `surek`, `r4`). Ama karar
tek bir kapıya asılı: `nabiz` üç ölçünün üçünde de birinci ve aralıkları ayrık; yalnız bilgi kaybı
kapısının (a) maddesinde, lafzî dizgi karşılaştırmasıyla elendi ve o 4 "kayıp" büyük olasılıkla bir
karşılaştırma yapaydır (aşağıda "Bilgi kaybı"). Bu yorum sonuçtan sonra geldiği için seçimde kullanılmadı;
duyarlılık olarak yazılıdır: kapıyı geçseydi kazanan `nabiz` olurdu.

Maliyet: **≈ 12,5 $** (okuyucu 7 313 çağrı 6,35 $, eleştirmen 360 çağrı 6,11 $); bütçe 15 $. Kullanım
sınırına takılan çağrı: 0.

## Düzenek ve protokol (Ek G8, G8-9; okumalardan önce sabitlendi)

**Tek derleme.** Bütün tasarımlar `main` 4fd0840'ın tek derlemesinden (`qp` release + `scripts/demo.sh`
WASM paketi; dalda yalnız SPEC/düzenek ayrı, ürün kodu aynı), `look=oltan`, `voice=marla`, öteki seçimler
gösterim varsayılanı, 402×874 @1×, `canvas=run`, duyarlılık `all`, 2 dil × 2 tema:

| ad | adres |
|---|---|
| `surek` | `motion=surek&reduced_motion=0` |
| `nabiz` | `motion=nabiz&reduced_motion=0` |
| `r4` | `motion=r4&reduced_motion=0` (açılış `zumlak`, yüz değişimi hareketsiz) |
| `still` | `motion=r4&reduced_motion=1` |
| bozuk hareket (gizli ad `tirvan`) | `motion=surek&reduced_motion=0` + `shoot.mjs` `brokenMotionInit` |
| `surek-az`, `nabiz-az` (yalnız bilgi kaybı) | aday + `reduced_motion=1` |

**Süreklilik (`m_from`).** İki hedef (canlı `cap.SYN06.breakout.up.118`, tutmadı
`cap.SYN26.breakout.down.51`; adım 1), her biri akış alanının (üst çubuğun altı, 4 eşit şerit) 1–4. şeridinin
ortasına kaydırıldı → 8 açılış durumu; gerçekleşen yüz üstleri 76/272/468/663 px, orta yarıları tek şeritte.
Okuyucuya yalnız iki ara kare (I = p 0,25, II = p 0,5) şeritli gösterildi; **önce karesi yok**. Soru:
"Açılan kapsül ekranın hangi numaralı şeridinden geliyor?" (1–4 / olduğu yerde beliriyor / bilmiyorum).
Anahtar: geçişten önceki DOM kutusunun dikey ortasının şeridi (tasarımdan bağımsız). Denetim
(`motion2_final`): aynı soru, iki panel de son kare.

**Bozuk hareket.** Her hareket parçası (`#fx canvas`) yarım ekran sıçratılır (merkezi üst yarıdaysa +437,
altta −437 px), tohumlu titretilir (±24/±16 px, ±6°), yüz tuvali yerinde solmaz. Ürüne girmez (Playwright
başlatma betiği). Mekanik denemede bütün açılış ve değişim parçaları yanlış şeritteydi.

**Fark edilme.** 4 değişim (`c_born` 216, `c_strengthened` 225, `c_failed` 333, `c_realized` 207) + 4 denetim
(`n_live1` 201, `n_live2` 218: ekranda canlı kapsül var; `n_still1` 211, `n_still2` 300: yok). `mid` (oltan:
`surek`/bozuk/`r4`/`still` 225 ms, `nabiz` 562 ms) ve `after` (1000 ms) tek kareler, tam ekran şeritli,
`n_where` sorusu. Puan: isabet − yanlış alarm (durum başına ortalama; F2:H1; persona × durum katmanlı
önyükleme).

**Okuyucu** haiku, F ve H, 6 tekrar (7 296 çağrı); boylar arası uyum sonnet 20 çağrı. **Eleştirmen** sonnet,
"0" | A | B, `CRITIC_M21`, tr/açık, 2 açılış (`o_live_2`, `o_fail_3`) + 4 değişim × 10 çift × 2 sıra × 3 tekrar
= 360. Körlük ilk koşudaki gibi (rastgele adlı tek PNG, yasak sözcük denetimi tasarım adlarıyla, kayıtlar
`run-v2/*.jsonl.gz`). Tek bakış dışı çağrı: okuyucu 3 birim (çıkarıldı), eleştirmen 0. Örnek istemler
`run-v2/prompts.md`.

## Geçerlilik sınaması (G8-1)

| tasarım | uyaran | n | son kareyle doğruluk [Wilson %95] | tek yönlü (alt sınır ≤ 0,25) | iki yönlü lafız (0,25 aralıkta) |
|---|---|---|---|---|---|
| `surek` | final \| final | 384 | 0,000 [0,000, 0,010] | geçerli | hayır (şansın altında) |
| `nabiz` | final \| final | 383 | 0,000 [0,000, 0,010] | geçerli | hayır |
| `r4` | final \| final | 384 | 0,000 [0,000, 0,010] | geçerli | hayır |
| bozuk | final \| final | 384 | 0,000 [0,000, 0,010] | geçerli | hayır |
| `still` (G8-1 ii) | kendi `motion2` (= son kare) | 388 | 0,003 [0,000, 0,014] | **geçerli → soru herkes için geçerli** | hayır |

Okuyucular son karede şerit uydurmuyor: cevapların %87–89'u "bilmiyorum", %11–13'ü "olduğu yerde
beliriyor". Soru yalnız ara karelerle cevaplanabiliyor; ilk koşunun (a) sorunu giderildi. Doğruluk şansın
*altında* olduğu için iki yönlü lafız tutmuyor; önceden yazılı kural tek yönlüdür.

## Süreklilik doğruluğu (birincil)

| tasarım | birincil (F2:H1, %95) | şerit 1 / 2 / 3 / 4 (ham pay) | "olduğu yerde beliriyor" |
|---|---|---|---|
| `nabiz` | **0,698 [0,651, 0,744]** | 0,73 / 0,57 / 0,51 / 0,94 | 0,09 |
| `surek` | 0,573 [0,524, 0,625] | 0,92 / 0,43 / 0,32 / 0,61 | 0,01 |
| `r4` | 0,373 [0,324, 0,424] | 0,75 / 0,18 / 0,16 / 0,36 | 0,03 |
| bozuk | 0,020 [0,007, 0,038] | 0,05 / 0 / 0 / 0,02 | 0 |
| `still` | birincili yok (0,002) | | 0,10 |

Aralıkların hiçbiri örtüşmüyor. Okuma (bilgi): `nabiz`'in çapraz geçişinde yüzün kendisi p 0,25'te
yerinde (saydamlık 0,71) duruyor; `surek`'in uçan parçası p 0,25'te bile hedefe doğru yola çıkmış (orta
şeritlerde bir şerit yukarıda); `r4`'ün `zumlak`'ı p 0,25'te parçaları neredeyse son yerinde gösteriyor
(1. şerit dışında düşük). Ajan okuyucular üçünde de 1. şeride yanlı (bozukta 230/384 "1").

Boylar arası uyum (haiku–sonnet, aynı uyaranda aynı kod, n = 20): **0,25** (düşük). Sonnet'in doğruluğu aynı
sırayı veriyor (`nabiz` 3/4, `surek` 2/4, `r4` 1/4, bozuk 0/4, `still` 0/4), ama alt küme küçük.

## Fark edilme (G8-3)

| tasarım | isabet − YA | yalnız mid | yalnız after | isabet: hedef canlı / kapanmış | YA: canlılı / canlısız denetim |
|---|---|---|---|---|---|
| `nabiz` | **0,113 [0,047, 0,181]** | 0,076 | 0,149 | 0,64 / 0,25 | **0,55** / 0,13 |
| `still` | −0,002 [−0,057, 0,054] | −0,028 | 0,024 | 0,10 / 0,17 | 0,15 / 0,14 |
| `surek` | −0,019 [−0,073, 0,031] | −0,049 | 0,010 | 0,10 / 0,15 | 0,18 / 0,10 |
| `r4` | −0,024 [−0,080, 0,031] | −0,024 | −0,024 | 0,11 / 0,16 | 0,17 / 0,13 |
| bozuk | −0,071 [−0,120, −0,021] | −0,122 | −0,021 | 0,06 / 0,09 | 0,15 / 0,13 |

- Yanlış alarm ilk koşudaki %77–79'dan %10–20'ye indi (okuyucular bu istemde çoğunlukla "bilmiyorum"
  diyor; bilmiyorum yanlış alarm sayılmaz). Ölçünün çözünürlüğü yine düşük: `surek`, `r4`, `still` sıfır
  civarında ve ayırt edilemiyor.
- Canlılık ile değişim ayrıldı: `nabiz`'in nabzı canlılı denetimlerde **%55 yanlış alarm** üretiyor
  (canlısızda %13); isabeti de büyük ölçüde hedefin canlı olduğu durumlarda (0,64; kapanmışta 0,25). Net puan
  yine pozitif ve tek ayrık olan.
- `surek` doğumu işaretlemiyor (`changed()` boş, ilk koşudaki gibi) ve durum değişiminde taşınan parça
  etiket boyunda (151 × 22 px); okuyucu onu hareketsizden ayırmıyor.

## Hareketi azalt bilgi kaybı (G8-6; sert kapı, G6-5 lafzı)

| tasarım → azaltılmış | (a) son ekran durağan içerik kaybı | (b) metin öğesi kuralı | (b) herhangi bir ilke (duyarlılık) | hareketli yüz |
|---|---|---|---|---|
| `surek` → `surek-az` | 0 | 0 | 0 | 12 |
| `nabiz` → `nabiz-az` | **4** | 0 | 0 | 24 |
| `r4` → `still` | 0 | 0 | 0 | 0 |

`nabiz`'in (a) kaybı: `c_strengthened`'in 4 dil/tema çekiminde hareket kipinin son ekranında yüzde ayrı bir
metin parçası olarak "güçlendi" / "strengthened" var; hareketi azalt kipinde aynı yüzde bu sözcük
**"canlı · güçlendi" / "live · strengthened"** adlı tek bir metin parçasının içinde duruyor. (a)'nın uygulaması
(Ek G6-5, değiştirilmedi) metin parçalarını tam dizgi olarak karşılaştırdığı için bu bir kayıp sayılıyor.
Durum 5 000 yerine 7 000 ms'de çekilen son karelerle de aynı (`run-v2/loss-end7000.json`; iz süresi
değil, kalıcı etiket). **Önceden yazılı kurala göre `nabiz` elenir.** Sözcük azaltılmış ekranda metin olarak
var; bu yüzden kaybın büyük olasılıkla bir karşılaştırma yapayı olduğunu düşünüyorum. Bu yorum kayıp
sonucunu gördükten sonra geldi (okuyucu/eleştirmen sonuçlarından önce), seçimde kullanılmadı.

## K52 (G8-5)

Kalibre benzetim, 3 koşu ortancası; koşular okuyucu ve eleştirmen çağrılarıyla eşzamanlıydı (ilk koşudaki
gibi tutucu koşul).

| tasarım | `k52.mjs` JIT açık: tik / yüz / sahne p95, ilk kapsül, kaydırma p99 | geçiş uzun kare payı JIT açık / kapalı (p99, en çok) | oynatma 8 s, JIT açık / kapalı | kapı |
|---|---|---|---|---|
| `surek` | 1 / 1 / 3 ms, 241 ms, 19 ms | %1,19 / %1,81 (59, 77 ms) | 0 / 0 | geçti (JIT açık geçiş eşiği uygulanmadı) |
| `nabiz` | 1 / 1 / 2 ms, 194 ms, 18 ms | %1,30 / %1,81 (57, 76 ms) | 0 / 0 | geçti (aynı) |
| `r4` | 1 / 1 / 2 ms, 209 ms, 19 ms | %1,30 / %1,94 (65, 79 ms) | 0 / 0 | geçti (aynı) |
| `still` | 1 / 1 / 2 ms, 194 ms, 18 ms | %1,19 / %1,61 (54, 72 ms) | 0 / 0 | geçti (aynı) |

Dördünün de geçiş penceresinde her açılışta ve her geri dönüşte tek bir 54–98 ms'lik kare var (açılış başına 6
tekrarın 6'sında). Hareketsiz `still`'de de aynı oranda, `look=r4` ile yapılan ilk koşuda yoktu. Bu yüzden
kaynağın hareket tasarımı değil, `oltan`/gösterim açılmış kapsül sahnesinin ilk boyaması olduğu
düşünülüyor (ölçülmedi). §G9.1 / G6-4: hiçbir tasarım (başvurular dahil) "JIT açık geçiş ≤ %1" eşiğini
geçemediği için bu eşik uygulanmadı.

## Eleştirmen BT

| tasarım | BT [%95] |
|---|---|
| `nabiz` | 1,45 [1,08, 1,92] |
| `still` | 0,70 [0,41, 1,04] |
| `surek` | −0,03 [−0,27, 0,21] |
| `r4` | −0,36 [−0,67, −0,06] |
| bozuk | −1,76 [−2,31, −1,38] |

Çelişki oranları ≤ %28 (`nabiz`–`still` 5/18); hiçbir çift "ayırt edilemedi" değil. Eleştirmen hareketsiz
kareyi ara karelerin çoğunun önüne koyuyor (okunaklılık).

## Bozuk hareketin yeri (G8-2)

- Birincil: 0,020, **kesin son** (aralığı herkesinkinin altında).
- BT: −1,76, **kesin son**.
- Fark edilme (bilgi): −0,071, **kesin son** (tek negatif ayrık aralık; vuruş yanlış şeritte okunuyor).

**Kabul kuralı geçti; koşu geçerli.**

## Seçim kuralı adım adım (G8-7)

1. **Sert eşikler.** K52 standart: dördü geçer. K52 geçiş, JIT açık ≤ %1: hiçbiri geçemiyor → uygulanmadı
   (§G9.1). JIT kapalı ≤ %10: dördü geçer. Bilgi kaybı (metin, G6-5 lafzı): `surek` 0, `r4` 0, `still` 0,
   **`nabiz` 4 → elenir**. Kalanlar: `surek`, `r4`, `still`.
2. **Birincil (`m_from`).** Geçerli sorusu olanlar: `surek` 0,573 [0,524, 0,625], `r4` 0,373 [0,324, 0,424];
   `still` yapısal olarak birincilsiz, arkada. En iyinin (`surek`) aralığıyla örtüşen yok → ilk grup {`surek`}.
3. (2b, 3: tek aday kaldığı için koşmadı.)
4. **Kazanan: `surek`.** Varsayılan `surek`; **ilk iki `surek`, `r4`** (bağ çözme: birincili olan önce →
   birincil; `still` üçüncü).

**Duyarlılık (sonradan, seçimde kullanılmadı):** `nabiz`'in (a) kaybı dizgi içerme ile karşılaştırılsaydı
(sözcük azaltılmış etikette var) `nabiz` kapıyı geçerdi. Adım 2'de `nabiz` 0,698 [0,651, 0,744] tek başına
ilk grupta olurdu (`surek` 0,573 [0,524, 0,625] ile örtüşmüyor) → kazanan `nabiz`. Fark edilme ve BT de
`nabiz`'i ayrık biçimde birinci koyuyor. Yani M21 kararı ölçülerden değil, `nabiz`'in "güçlendi" etiketinin
hareket ve hareketi azalt kiplerinde farklı metin parçalarıyla çizilmesinden belirleniyor.

## Varsayılan

Önceden yazılı kurala göre **`surek`** (bugünkü geçici gösterim varsayılanıyla aynı; ilk koşudaki geçici karar
ölçümle desteklendi). İkinci `r4`. `nabiz` kural gereği ilk ikide değil. Ölçülerde birinci olduğu ve elenmesi
bir karşılaştırma ayrıntısına bağlı olduğu için orkestratörün iki seçeneği var: (i) kararı olduğu gibi almak;
(ii) `nabiz`'in azaltılmış etiketini hareket kipindeki parçayla aynı metin parçası yapmak ya da (a)'yı
"sözcük metin olarak duruyor" diye yeniden tanımlamak (ek yazarak) ve yalnız bu hesaplanan kapıyı yeniden
koşmak. Okuyucu ve eleştirmen ölçüleri yeniden koşulmadan kullanılabilir.

## Maliyet

| kalem | çağrı | $ |
|---|---|---|
| okuyucu haiku (`motion2`, `motion2_final`, `notice`) | 7 296 | 6,03 |
| okuyucu sonnet (uyum alt kümesi) | 20 | 0,31 |
| eleştirmen sonnet | 360 | 6,11 |
| **toplam** | | **≈ 12,46** |

Kullanım sınırıyla cevapsız dönen çağrı: 0 (`m21v2.py unlimit` denetledi).

## Sapmalar ve sorunlar

1. **G8-9** (okumalardan önce, mekanik denemeden sonra): şeritler akış alanında (üst çubuk ≈ 91 px; yüz 226 px
   olduğu için tam ekranın 1. şeridine yerleştirilemiyordu); `fail` hedefi `…down.51` (4. şerit için üstünde 3 yüz
   gerekiyordu); bozuk hareket aynalama yerine yarım ekran sıçrama (ortaya binen yüz yerinde kalıyordu). Ayrıca:
   düzeneğin eski `scrollTo` düzeltmesi `window.scrollBy` kullanıyordu, akış ise `.page` içinde kayıyor (o satır
   etkisizdi; ilk koşunun sonuçlarını değiştirmez, `scrollIntoView` çalışıyordu).
2. Bilgi kaybı (a) yapayı: yukarıda. 7 000 ms son kare denemesi kayıp sonucundan sonra yapıldı (yalnız bilgi).
3. K52 JIT açık geçiş eşiği bütün tasarımlarda (hareketsiz dahil) aşıldı; eşik uygulanmadı. Kaynağı
   ölçülmedi (Öneriler 2).
4. Boylar arası uyum düşük (0,25, n = 20); sıralama aynı ama ajan okuyucunun şerit cevapları gürültülü ve
   1. şeride yanlı.
5. Fark edilme ölçüsü ayırıcı değil (yalnız `nabiz` ve bozuk sıfırdan ayrık).

## Öneriler (kapsam dışı)

1. `nabiz`'in hareketi azalt etiketini hareket kipindeki metin parçasıyla aynı biçimde çiz ("güçlendi" ayrı
   parça) ya da düzenekteki (a) karşılaştırmasını sözcük içerme ile yap (ek yazarak); sonra yalnız kapıyı yeniden
   hesapla.
2. `oltan` + `tesir` ile açılış ve geri dönüşte her seferinde tek bir 55–98 ms'lik kare var (hareketsizde de).
   Açılmış kapsül sahnesinin ilk boyaması ayrıca ölçülmeli (K52 JIT açık ≤ %1 eşiği bugün tutmuyor).
3. `surek`: doğumu işaretlemiyor; değişim parçası etiket boyunda ve fark edilmiyor (isabet − YA ≈ 0).
4. `nabiz`: sürekli nabız canlılı ekranda %55 yanlış alarm üretiyor; değişim vuruşu ile nabız daha belirgin
   ayrılmalı.
5. Düzeneğin `scrollTo`'su `.page`'i kaydırmalı (bugün `window.scrollBy` etkisiz).
