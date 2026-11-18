# Kapsülün yüzü (R5: `oltan`; R4: `vadrel`; K43, K7)

> Seçimler okuyucu ve eleştirmen **ajanlarıyla yapılmış bir ön elemedir**. Ajan insan değildir;
> küçük pikseli ve kısa bakışı insan gibi deneyimlemez. İlk iki tasarım gösterimde ayardan,
> adresle ve galeride seçilebilir; son söz Arkin'in telefondaki karşılaştırması ve insan testi
> v3'tedir (`rounds/R5/human-test/`, `DECISION.md`).

Kanıt (R5): `rounds/R5/experiments/M16/RESULTS.md` (kör ölçüm, seçim), `run/tables.md`; düzenek
`rounds/R5/experiments/harness/glance/SPEC.md` §G ve Ek G1–G7; tasarım notları
`rounds/R5/experiments/M16/<ad>/DESIGN.md` (`r5/m16-<ad>` dallarında); görsel dilin tamamı
`docs/DESIGN.md`; kapsül dili `docs/LANGUAGE.md`. R4 kanıtı aşağıda "Tarihçe". Kavramlar
`docs/CONCEPTS.md` ("Kapsülün yüzü", "Başlık cümlesi", "Ek bilgi", "Süre ifadesi", "Geçerlilik",
"Başlık yapı taşları", "Görünüş").

## Seçilen (R5): `oltan` — işaret önde / saat yüzeyi

Gösterimin varsayılanı (R5/S): görünüş **`oltan`** + yüz **`oltan`**, kapsül dili **`marla`**.
`?look=oltan` eşlik eden yüzünü de getirir (`look::COMPANION_FACES`; `r4` → `vadrel`, `zemre` →
`zemre`); bir yüz adlandırılırsa o kalır. Protokolün varsayılanları değişmedi (yüz `r3`, dil `r4`):
çıplak bağlantılar, dökümler ve v1 altınlar bayt eşit. Kod: yüz `crates/qp-grammar/src/faces/oltan.rs`,
görünüş kaydı `crates/qp-grammar/src/look.rs` (`OLTAN`).

```
┌──────────────────────────────────────────────┐   yüz 390 × 226 CSS px (r4/vadrel/marla: 259)
│ ┌────────────┐ SYN20 son 20 günün en          │
│ │            │ yüksek fiyatını (227,08)       │  başlık: 19 px yarı kalın, ≤ 5 satır,
│ │   işaret   │ aştı · bugün, hâlâ geçerli     │  176 px sütunda
│ │  120 px    │                                │
│ └────────────┘                                │
│ Benzer 20 durumun 11'inde fiyat 10 işlem      │  karne: 16 px normal, muted, ≤ 2 satır
│ günü içinde yükseldi.                         │
└──────────────────────────────────────────────┘
```

- **İlke:** ekranın kahramanı anlam işaretidir (K7). Yüzün en büyük ve en yüksek karşıtlıklı öğesi
  o; göz önce işarete düşer, yönü, türü ve geçerliliği oradan okur. Cümle ikinci sıradadır ama rahat
  okunur. Bulanık ve yarım boy kopyada yüz bir renkli işarete ve gri bir metin bloğuna iner.
- **Tek bakışın içeriği (K43):** bir cümle (`marla` başlığı, ≤ 80 karakter), bir işaret, bir ek
  bilgi (karne cümlesi). Başlık satırı boş; enstrüman başlığın ilk sözcüğü. Güven işareti
  ("Sunucu doğruladı") sağ altta, rozet sağ üstte. `marla`'nın 1 062 başlığının en uzunu 176 px
  sütunda 5 satır, 328 karnenin hepsi ≤ 2 satır (ölçüldü). İşaret 128 px olunca başlık 6 satıra
  çıkıyordu; bu yüzden 120 px.
- **Yazı:** Atkinson Hyperlegible Next (Braille Institute; SIL OFL 1.1), başlık 600, geri kalan 400;
  iki statik örnek, toplam 73 572 B. Tablo rakamları cmap'te `tnum` gliflerine eşlendi. Türkçe 18
  harfin hepsi var; `₺` yok (`marla` kullanmıyor). En küçük yazı her yerde 13 px.
- **Renk:** anlam renkleri ayrı ışıklık basamaklarında (koyu: gri 47 < aşağı 59 < yukarı 80 <
  mürekkep 97; açık: gri 57 > yukarı 43 > aşağı 28 > mürekkep 6). **Gri = yön yok**: `neutral` =
  `invalid` = `closed`; yönsüz, artık geçerli olmayan ve tutmamış işaret aynı gri, aralarındaki farkı
  biçim (✗, çubuklu halka, rozet) ve sözcük taşıyor. Geçerli olmayan başlık `muted` renginde.
- **İşaret tabanları** (T3c, GRAMMAR §9a): `closed_alpha` 255, `fresh_alpha_min` 255,
  `plate_alpha_min` 61. İşaret tazelikle solmuyor; yaş ve geçerlilik cümlede ("bugün, hâlâ
  geçerli"). Sahnenin öğe değerleri gerçek tazeliği taşımayı sürdürüyor; yalnız çizim solmuyor.
  (R5/fix47) Gri işaretin plakası en çok `grey_plate_alpha_max` (koyu 49, açık 29): gri çizgiler
  plakaya karşı da ≥ 3:1. Açılmış kapsülün (`tesir`) geçerli olmayan başlığı da `muted`.
- **Akış sırası** (R5/fix47, kabul 7): gösterim `live_new` ile açılır — hâlâ geçerli olanlar önce,
  sonra artık geçerli olmayanlar, sonra kapanmışlar; her grupta en yeni önce. `grouped` ölçümde
  öndeydi ama enstrümanları geçerlilikten bağımsız en yeni geçişle sıralıyor (yalnız kapanmış
  hikâyesi olan bir enstrüman canlı iddialı birinin üstüne çıkabiliyor). `?feed_order=grouped`
  ve `?feed_order=layout` (bağlantının düzeni, R5 öncesi) seçilebilir; protokol varsayılanı
  değişmedi.
- **Yüzey:** kadran karoları (köşe 14, gölge yok); dokunma hedefi ≥ 44 CSS px (sekme 48).
- **Yetkisiz okuyucu:** yüz sahnesi çizilmez; kabuk dilin şablonlu rakamsız yüzünü metin olarak
  gösterir (zaman, geçerlilik, nitel karne ve neyin hangi yetkiyle açıldığı; A32,
  `docs/LANGUAGE.md` "Yetkisiz okurun yüzü").

## Ölçüler (M16; SPEC §G7 + Ek G5, 2 dil × 2 tema)

| | **`oltan`** | `zemre` | `bertan` | `kivra` | `r4` (`vadrel`, başvuru) | bozuk |
|---|---:|---:|---:|---:|---:|---:|
| sert eşikler | **geçti** | geçti | kaldı: renk (Ek G5) | geçti | kaldı: 5 eşik | kaldı |
| metin karşıtlığı en kötü (≥ 4,5) | **6,33** | 6,61 | 6,39 | 6,17 | 2,75 | 1,62 |
| işaret karşıtlığı, kapanmış dahil (≥ 3) | **3,46** | 3,11 | 3,17 | 3,17 | 2,06 | 1,26 |
| en küçük yazı (px) | **13** | 12 | 13 | 12 | 10 | 10 |
| 44 px'ten küçük dokunma hedefi | **0** | 0 | 0 | 0 | 108 | 108 |
| anlam renkleri en kötü çift ΔE00 (beş koşul, ≥ 10) | **10,53** | 10,43 | 7,28 | 11,45 | 0,50 | 0,31 |
| yüzdeki en küçük işaret (px) | **120** | 56 | 64 | 40 | 32 | 12 |
| yüz karakter ortalaması | 130 | 170 | 131 | 133 | 131 | 127 |
| tam boy okuma (kapı; üst sınır ≥ 0,866) | 0,865 | 0,862 | 0,855 | **0,775** (elendi) | 0,866 | 0,837 |
| **birincil: kısma kopyası** (F2:H1, %95) | **0,471 [0,445; 0,497]** | 0,279 | 0,345 | 0,243 | 0,318 | 0,183 |
| `t_first` (işaret önce) | **0,83** | 0,52 | 0,54 | 0,58 | 0,61 | 0,45 |
| eleştirmen BT | **2,15** | 1,24 | 1,30 | 1,79 | −1,20 | −5,29 |
| gri kopya `dir` + `valid` | 1,00 | 1,00 | 1,00 | 0,87 | 1,00 | 1,00 |
| paket (WASM brotli, `r4`'e göre) | +27,8 KB | +25,4 KB | +29,0 KB | +71,6 KB | 1 038,7 KB | — |

K52 (yüz akışı) dört adayda da geçti (`oltan` JIT açık tik / yüz / sahne p95 1 / 1 / 2 ms; JIT
kapalı kaydırmada uzun kare 0). Bozuk tasarım birincilde ve BT'de kesin son → koşu geçerli.

**Seçim kuralının uygulanışı** (önceden yazılı, SPEC §G9): (1) sert eşiklerde `bertan` (Ek G5 renk)
ve `r4` elendi, feragat yok; (1b) aşağı kalmama kapısında `kivra` elendi; (2) birincilde `oltan`
tek başına ilk grupta (aralığı `zemre`'ninkiyle örtüşmüyor) → **kazanan `oltan`, ayırt edildi**; ilk
iki `oltan`, `zemre`. Ek G5 uygulanmasaydı kazanan yine `oltan`, ikinci `bertan` olurdu.

## İkinci (R5): `zemre` — zaman şeridi

Alışılmışın dışı yön (K18; `r4`'ü görmeden kuruldu). `?look=zemre` (yüz `zemre` ile). Yüz iki
soruyu ayrı cevaplıyor: büyük işaret (56 px), kalın başlık ve soluk karne "ne oldu, hangi yöne, hâlâ
geçerli mi"yi; alttaki **ömür şeridi** "zaman neresinde"yi (pencere iddianın kendi ömrü; sağ kenar
*şimdi*, dikey çizgi doğum anı). Bütün yüzlerin sağ kenarı aynı x'te olduğu için akışta bir "şimdi
sütunu" oluşuyor. Kâğıt ve mürekkep (varsayılan açık tema), anlam renkleri mavi / zinober / mürekkep
/ gri (yeşil–kırmızı değil), Atkinson Hyperlegible Next 400/700 + fiyat için IBM Plex Mono.
Birincil puanı (0,279) başvurunun (0,318) altında: şerit ve kalın başlık işaretle yarışıyor
(`t_first` 0,52). Ömür şeridi "ne zaman" sorusunda üstünlük getirmedi (0,69 ↔ `oltan` 0,70); asıl
sorusu ("hangisi daha yeni") ajanla sorulmadı.

## Elenenler (R5)

- **`bertan` — sakin editoryal** (Source Sans 3 alt kümesi): yalnız Ek G5 renk kapısında kaldı
  (açık temada `up ↔ invalid` gri tonlamada 7,28 < 10). Birincilde ikinci nokta tahmini (0,345).
- **`kivra` — yoğun ve keskin** (IBM Plex Sans Condensed): sert eşikleri geçti, eleştirmende ikinci
  (1,79), ama tam boy okumada alt sınırın altında (0,775); gri kopyada az örnekli kapsülün yönü ve
  geçerliliği okunmadı (0/48).
- **`r4` görünüşü** başvurudur: v1 altınlarını bayt eşit üretmeye devam eder (`?look=r4`).

Ayrıntı ve gerekçeler: `docs/DESIGN.md` §2–3.

## Açık sorunlar (güncel)

1. **A31 — kapanmış işaretin karşıtlığı: kapandı.** İşaret tabanları (T3c: `closed_alpha` `tg5`'te
   de taban, `fresh_alpha_min`, `plate_alpha_min`; GRAMMAR §9a) görünüş kaydında. `oltan`'da
   kapanmış dahil en kötü işaret 3,46:1 (tutmadı izi, açık tema); sınama
   `glyph::outcome::tg5::tests::oltan_signs_keep_3_to_1_at_freshness_zero` (tazelik 0'da iki alfabe,
   iki tema, 6 tür × 3 yön × 4 evre ≥ 3:1). Bedeli: `oltan`'da tazelik solması yok, yaş cümlede.
   `r4` görünüşü (v1 altınlar) 2,06 ile kalır.
2. **A32 — yetkisiz okuyucunun yüzü: dil bütünleştirmesiyle uygulandı** (R5/`m17-int`). Kapsül dili
   seçiliyken yetkisiz okur dilin şablonlu rakamsız yüzünü görür. M20'de `tesir` yetkisiz okur
   sorularında 0,993 (`r4` 0,500); yetkisizde rakam 0. Değerleri gizli ama barları açık okur sentetik
   dünyada yok, ölçülmedi. Kapanışı insan testi v3'te (`DECISION.md` §3).
3. **A33 — ayrıntı cümlesi yüzle çelişiyordu: dil bütünleştirmesiyle uygulandı.** `marla` ve
   `selvi`'de ayrıntı yüzün yapı taşlarından kuruluyor; çelişki taraması 531 durum × 2 dilde 0.
   Protokol varsayılanı `r4` dilinde R3 cümlesi kalır (dökümler değişmesin diye). Kapanışı insan
   testi v3'te (`DECISION.md` §4).
4. **"Bugün" ↔ "4 saat önce"** (R4 #1): değişmedi; insan testinde gözlenecek.
5. **Gri kopya ve renk körlüğü:** artık ölçülüyor (Ek G5: anlam rolleri beş koşulda ΔE00; gri kopya
   okuyucu sorusu `oltan`'da 1,00). Ajan tam boyda metni okuduğu için gri kopya tek başına "biçimle
   okunur" kanıtı değil; renk körü katılımcı varsa insan testinde ayrıca bakılmalı.
6. **A44 — `oltan`'da akışta canlıyı bulma** 0,65 [0,45; 0,81] (n 23), bozuk dışındaki en düşük
   değer (`r4` 0,92). Seçime girmedi; insan testinde bakılacak.
7. **Yazı tipleri:** `canvas` yolu açılışta gömülü bütün yazı tiplerini yüklüyor; her yeni görünüşün
   yazı tipi herkesin açılışına ekleniyor. Öneri: yalnız seçili görünüşün dosyaları.
8. **Yüz–görünüş bileşimleri:** `?look=oltan&face=vadrel` mümkün ve orada geçersiz başlık
   `invalid` renginde (3,75:1, metin eşiğinin altında); görünüş eşlik eden yüzüyle tasarlandı.
   `fotolay` ve `tavrik` dil seçiliyken karne bandı taşımıyor (`docs/LANGUAGE.md`).
9. **Olay sözcüğü eşlemesi** (R4 #5) yalnız `r4` dilinde (yüz tasarımının kendi başlık şablonu)
   geçerli; dil seçiliyken başlığı dil kurar.
10. **Ajan tavanı:** tam boy okuma her tasarımda 0,78–0,87, adaylar ancak kısma kopyasında ayrılıyor;
    "beş saniyelik bakış" personası ajanda görünmüyor. Ajan sıralaması kaba bir sıralamadır.

---

## Tarihçe (R4): `vadrel` ve M8

R4'te M8 ile seçilen yüz. Bugün `?look=r4` (ya da `?face=vadrel`) ile seçilebilir ve v1 demo
altınlarını üretir. Kanıt: `rounds/R4/experiments/M8/RESULTS.md` (ölçüler, ham kayıtlar `raw/`),
şartname `rounds/R4/experiments/harness/SPEC.md` §2 ve ekleri E1–E7, tasarım notları
`rounds/R4/experiments/M8/<ad>/DESIGN.md`. Aşağıdaki metin R4 sonundaki hâlidir (R5/`m17-int`
notlarıyla); "Açık sorunlar"ın güncel hâli yukarıda.

### Seçilen (R4): `vadrel` — "cümle önde"

Kod `crates/qp-grammar/src/faces/vadrel.rs`; kayıt `faces.rs` (`FACES`). Gösterimin varsayılan yüzü (`PRESENTED_FACE`); protokolün varsayılanı `r3` kalır (çıplak bağlantılar ve dökümler değişmez). Altınlar: `fixtures/golden/demo/feed-*-r4.*` (`vadrel`), `feed-tr-dark-fotolay.*`. Yükseklik 188 CSS px
(telefon genişliği 390'da; kabuk sayıyı çekirdekten alır).

```
┌──────────────────────────────────────────────┐
│ Benzerleri: 74'te 36 tuttu        (16 px)    │  ek bilgi: kısaltılmış karne (tek)
│ ┌──┐ SYN03 önemli bir eşiğin                 │
│ │◆ │ üstüne çıktı; 4 saat önce               │  başlık cümlesi (20 px, en çok 5 satır)
│ └──┘ güçlendi. Hâlâ geçerli.                 │  işaret 32 px, cümlenin solunda
└──────────────────────────────────────────────┘
```

- **Başlık cümlesi** şablon dilbilgisinden (`vadrel::headline`, `HeadlineParts` yapı taşları;
  belirleyici; TR ve EN; ≤ 80 karakter, birim testi bütün olay × aşama × en uzun zaman ifadesi
  bileşimlerinde). Enstrüman özne, olay iç terimsiz bir fiil öbeği ("önemli bir eşiğin üstüne
  çıktı"), zaman son geçişin anı (süre ifadesi, takvime göre: "dün", "cuma günü", "3 işlem günü
  önce", "2 saat önce"), geçerlilik ayrı kısa cümle ("Hâlâ geçerli." / "Artık geçerli değil." /
  "Kapandı: 15 Mayıs'ta gerçekleşti.").
- **İşaret:** bakış ayarının alfabesi (`siluet` varsayılan, `rozet`) ve sonuç işareti (`tg5`).
- **Geçerlilik üç yoldan görünür:** sözcük, işaret ve cümlenin rengi (geçerli değilse `muted`,
  karşıtlık yine ≥ 5,2:1). Gri kopyada sözcük ve işaret şekliyle.
- **Dürüstlük:** karne her zaman görünür; az örnekte "(az örnek)" düşmez; karne yoksa
  "henüz örnek yok". Tavsiye yok.
- **Yetkisiz okuyucu:** yüz sahnesi çizilmez; kabuk çekirdeğin rakamsız cümlesini metin
  olarak gösterir (bütün tasarımlarda aynı ürün kuralı). (R5) Bir kapsül dili seçiliyken bu
  metin dilin şablonlu rakamsız yüzüdür (başlık, karne, ayrıntı; aşağıda).

#### Kapsül dili ile (R5/`m17-int`, `docs/LANGUAGE.md`)

Bağlantı bir kapsül dili seçtiyse (`style` `voice`: `marla` gösterimin varsayılanı, `selvi`;
`r4` protokolün varsayılanı ve bugünkü metin) yüz başlığı ve karnesi o dilden gelir; yüz
tasarımının kendi başlık şablonu (`vadrel::headline` vb.) yalnız `r4`'te çalışır.

- **Kaynak:** sahne sağlayıcısının `DataProvider::voice` yöntemi (otoritede
  `Session::reader_voice`; kopyada kapsül öğesinin `voice`'u — otoritenin aynı işlevle yazdığı).
  `headline` metin alanı sesin başlığıdır (her yüz tasarımında), `ledger` metin alanı karne
  cümlesi, `detail` ayrıntı (açılmış kapsülün kartı).
- **`vadrel`:** başlık aynı yerde (en çok 80 karakter, beş satır); karne tam bir cümle (en çok
  70 karakter) olduğu için başlık satırından çıkar, başlığın altında `title` rolünde (16 px)
  üç satırlık bir banda geçer, `muted` renkte. Başlık satırı boş kalır. Yükseklik
  `vadrel::voiced_height` (`r4` görünüşte **259 px**; kabuk sayıyı `faces::height_for`'dan
  alır).

```
┌──────────────────────────────────────────────┐
│ ┌──┐ SYN20 son 20 günün en yüksek            │
│ │◆ │ fiyatını (227,08) aştı · bugün,         │  başlık cümlesi (sesin başlığı)
│ └──┘ hâlâ geçerli                            │
│                                              │
│ Benzer 20 durumun 11'inde fiyat 10 işlem     │  karne cümlesi (16 px, en çok 3 satır)
│ günü içinde yükseldi.                        │
└──────────────────────────────────────────────┘
```

- **Sözlük:** karne bandındaki terimler altı çizili (`SceneDesc.terms`; başlıkta değil, başlığa
  dokunmak kapsülü açar); dokununca otorite kapsülü
  açmaz, terimi bildirir (`hit.term`), kabuk açıklamayı gösterir (K49).
- **`fotolay`, `tavrik`:** başlığı sesten alır; karne bandı yok (açık sorun, `docs/LANGUAGE.md`).
- **`r3` kartı:** sesin başlığı yerine ayrıntısı ve iki satırlık karnesi (açılmış kapsülün
  kartıyla aynı kural).

Örnekler (S8 durumları, sentetik veri):

| Durum | TR | EN |
|---|---|---|
| olay, geçerli | 3 işlem günü önce SYN11 önemli bir eşiğin altına indi. Hâlâ geçerli. | 3 trading days ago, SYN11 broke below a key level. Still holds. |
| artık geçerli değil | SYN06 kısa sürede aşırı yükseldi; cuma günü zayıfladı. Artık geçerli değil. | SYN06 rose too far too fast; weakened on Friday. No longer holds. |
| kapandı: gerçekleşti | SYN25 önemli bir eşiğin altına indi. Kapandı: 15 Mayıs'ta gerçekleşti. | SYN25 broke below a key level. Closed: played out on 15 May. |
| kapandı: tutmadı | SYN25 önemli bir eşiğin altına indi. Kapandı: 10 Şubat'ta tutmadı. | SYN25 broke below a key level. Closed: did not play out on 10 February. |

### Ölçüler (R4 M8; SPEC §2; başlık #9 hariç, E7)

| | `r3` (R3 kartı, başvuru) | **`vadrel`** | `fotolay` | `tavrik` |
|---|---:|---:|---:|---:|
| yüz yüksekliği (px) | 240 | **188** | 228 | 156 |
| başlık TR en çok / ort. | 205 / 181 | **75 / 66** | 78 / 72 | 78 / 71 |
| başlık EN en çok / ort. | 208 / 186 | **77 / 66** | 80 / 76 | 76 / 67 |
| metin karşıtlığı açık / koyu | 2,79 / 5,27 | 5,78 / 5,27 | 5,78 / 5,27 | 15,8 / 11,7 |
| işaret karşıtlığı (kapanmış) açık / koyu | 2,06 / 2,36 | aynı | aynı | aynı |
| en küçük yazı (px) | 11 | 16 | 11 | 15 |
| ek bilgi | 3 | 1 | 2 | 1 |
| persona tam doğru F / H | 0,06 / 0,30 | **0,76 / 0,79** | 0,70 / 0,79 | 0,61 / 0,58 |
| persona ağırlıklı (F 2, H 1) | 0,141 | **0,768** | 0,727 | 0,596 |
| eleştirmen ortalama sırası | 4,00 | 1,67 | 2,67 | 1,67 |

Şans düzeyi (tam doğru) ≈ 0,00004. #1 ve #9 her tasarımda 0/6 olduğu için ulaşılabilir tavan
0,818 (aşağıda "Açık sorunlar").

**Seçim kuralının uygulanışı** (önceden yazılı; RESULTS'ta adım adım):
1. Sert eşikler: kapanmış işaret karşıtlığını (alfabenin soluk gri çizimi, `r3` dahil 2,06)
   hiçbir tasarım geçemedi → E6 ile bu eşik uygulanmadı. Öteki eşikleri üç alternatif de geçti.
2. Birincil (persona tam doğru, ağırlıklı): `vadrel` 0,768 − `fotolay` 0,727 = 0,040 < 0,05.
3. Eleştirmen ortalama sırası: `vadrel` 1,67 < `fotolay` 2,67 → **`vadrel`**.
4. İlk iki: **`vadrel`, `fotolay`** — gösterimde `?face=` ya da Ayarlar → Yüz tasarımı ile
   seçilebilir; insan testi v2 bu ikisini karşılaştırır.

### İkinci (R4): `fotolay` — "olayın resmi"

Küçük, sade bir fiyat izi (bar başına nokta), olayın anında ince dikey çizgi, olayın kanıt
geometrisi (seviye/bant/dönüş noktaları), işaret anın üstünde; üstte kısa başlık
(`{Olay}, {zaman}: {geçerlilik}. {karne}`). Geçerli değilse resmin tamamı soluk. 228 px.
Persona doğruluğunda `vadrel`'in 0,04 gerisinde (eşik altı), eleştirmen sırasında geride.
Zayıflığı: derleyici her veri paneline eksen/ızgara/değer etiketi ekliyor ("eksensiz resim"
tam kurulamadı; gramere "çıplak panel" bayrağı önerisi); en küçük yazı 11 px (eksen etiketi).
Ek bilgi sayımı bir yoruma dayanır (grafiğin etiketleri ayrı sayılsaydı 4 olur, eşikten elenirdi).

### Elenenler (R4) ve gerekçe

- **`tavrik` — "işaret önde"** (76 px işaret, tek satır başlık, 156 px). Eşikleri geçti,
  eleştirmenlerde `vadrel` ile birinciye eşit (1,67), ama persona tam doğrulukta 0,596 ile
  üçüncü (birincil ölçüde `fotolay`'ın 0,13 gerisinde). Okuyucular yön sözcüğü taşımayan
  başlıklarda ("Aşırı alım", "Para girişi") yönü "belirsiz" okudu; "karnesi yok" ifadesi
  "hiçbir bilgi göremedim" diye okundu.
- **`r3` — R3 kartı** (başvuru, sıralanır, seçilmez). Başlık 205 karakter (eşik 80), açık
  temada metin karşıtlığı 2,79 (eşik 4,5), ek bilgi 3 (eşik 2); persona tam doğru 0,141.
  Gösterimde adla seçilebilir kalır (var olan altınlar `r3` ile sabittir).
