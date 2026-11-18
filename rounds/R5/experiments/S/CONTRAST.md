# R5/fix47 — karşıtlık, bütün öğeler (kabul ölçütü 4)

Yazan: R5 `fix47` alt ajanı, 2026-11-18. Bağlam: `AUDIT.md` koşul 2 (B1), `DECISIONS.md` A46.

## Yöntem

- **Düzenek:** `contrast/probe.mjs` (denetçinin `rounds/R5/audit/probe.mjs`'inden türetildi: aynı
  kanca, aynı tutamaklar) ve `contrast/analyze.py` (renk hesabı denetçinin `audit/colour.py`'si:
  WCAG 2.2 göreli parlaklık, sRGB'de düz alfa bindirme). Gösterim varsayılanlarıyla
  (`oltan` + `marla` + `pavka` + `tesir` + `surek`, `siluet` alfabesi, `tg5`), sentetik pazar,
  iki tema × iki dil, duyarlılık `all`:
  - akış yüzleri, adım 1 / 40 / 120 / 200 (48 yüz sahnesi);
  - adım 200'deki akışın canlı ve kapanmış/geçersiz kapsülleri (7 kapsül: 1 canlı, 6 kapanmış
    ya da artık geçerli değil) üç derinlikte: açılmış kapsül, kanıt, grafik + hikâye çizelgesi
    (84 sahne);
  - kabuğun DOM metni (akış, açılmış kapsül, kanıt, grafik, Ayarlar sekmesi; 44 blok) ve
    denetimleri (`input`, `select`; yerel onay kutuları ekran görüntüsünün piksellerinden, 66).
- **Kural** (düzeneğinkinden sıkı, A46): metin 4,5:1, **600 kalın sayılmaz** (3:1 yalnız ≥ 24 px);
  metin dışı 3:1 bitişik renge karşı (WCAG 1.4.11). Bitişik renk: ilkelden önce boyanan
  dikdörtgen/dolgu/dairelerin sahne zeminine bindirilmesi. Bir dikdörtgen, çizgi ya da şekil en
  ayırt edici kenarıyla geçer (kenarın 1 px dışı örneklenir); halka, deliğine karşı ölçülür;
  ≤ 2 px ince dikdörtgen zemin sayılmaz (kesişen bir kural). **Süs (eşiksiz, listelenir):**
  `grid`, `line` rolleri (ızgara, ayraç), saydam plaka ve bantlar (alfa < 255: güven plakası,
  kanıt bölgesi, seçili sütun/satır bandı), yüzey rengindeki ilkel (zeminin işaretten görünmesi).
  Düğmeler metinleriyle işaretli (metin olarak ölçüldü).
- Yeniden üretmek (depo kökünden; boş bir port seç):

      qp look show oltan > look.json
      node rounds/R5/experiments/S/contrast/probe.mjs <qp> <port> probe.json --shots <dir> --limit-s 900
      python3 rounds/R5/experiments/S/contrast/analyze.py probe.json look.json

## Sonuç (düzeltmelerden sonra)

| yüzey | tema | metin öğesi | en kötü metin (oran / eşik, boy, rol, örnek) | metin dışı öğe (eşikli) | en kötü metin dışı (oran, tür, rol) | eşik altı |
|---|---|---|---|---|---|---|
| yüz (akış) | koyu | 190 | **9,77** / 4,5, 16 px, `muted`, karne | 150 | **3,09**, dolgu, gri (`invalid`) plakasında | 0 |
| yüz (akış) | açık | 190 | **14,35** / 4,5, 16 px, `muted`, karne | 150 | **3,08**, dolgu, gri plakasında | 0 |
| açılmış kapsül (`tesir`) | koyu | 311 | **8,20** / 4,5, 13 px, `accent`, son değer etiketi “242.89” | 212 | **3,09**, dolgu, gri | 0 |
| açılmış kapsül (`tesir`) | açık | 311 | **6,42** / 4,5, 13 px, `accent`, son değer etiketi | 212 | **3,08**, dolgu, gri | 0 |
| kanıt derinliği | koyu | 194 | **5,67** / 4,5, 13 px, son değer etiketi | 2 174 | **3,09**, dolgu, gri | 0 |
| kanıt derinliği | açık | 194 | **5,69** / 4,5, 13 px, son değer etiketi | 2 174 | **3,07**, dikdörtgen, kapanmış şerit seçili satır bandında | 0 |
| hikâye çizelgesi (`pavka`) | koyu | 194 | **5,67** / 4,5, 13 px | 2 174 | **3,09**, dolgu, gri | 0 |
| hikâye çizelgesi (`pavka`) | açık | 194 | **5,69** / 4,5, 13 px | 2 174 | **3,07**, kapanmış şerit seçili satır bandında | 0 |
| kabuk (DOM) | koyu | 618 | **8,26** / 4,5, 13 px, `trust` “Sunucu doğruladı” | 169 | **4,09**, `select` sınırı | 0 |
| kabuk (DOM) | açık | 618 | **6,33** / 4,5, 16 px, saat | 169 | **3,40**, `select` sınırı | 0 |

(Kanıt derinliği grafik sahnesini kanıtla birlikte gösterir; sahne ilkelleri grafikteki ile aynı,
kanıt satırları DOM'da ve "kabuk" satırında.)

**En kötü eşikli öğe:** metin dışı **3,07** (açık tema, hikâye çizelgesinde seçili hikâyenin satır
bandındaki kapanmış şerit). En kötü metin **5,67** (koyu, grafiğin 13 px son değer etiketi).
**Eşik altı: 0** (metin ilkeli 1 778, DOM metni 1 236, metin dışı ilkel 10 312 — 892'si süs —,
DOM denetimi 338; sayfa hatası 0).

Rol başına (eşikli metin dışı, iki temada en düşük): `axis` 3,23 / 3,19; gri
(`neutral` = `invalid` = `closed`) 3,09 / 3,07; `down` 4,05 / 6,35; `up` 7,85 / 4,25; `accent`
7,28 / 5,69; mürekkep ≥ 9,99; denetim sınırı (`chrome.control`) 4,09 / 3,40; onay kutuları
(yerel, piksel) ≥ 3.

**Süs sayılanlar** (eşiksiz): ızgara 1,23 / 1,23; ayraç (`line`) 1,04 / 1,06; güven plakaları
(alfa 61; gri 49 / 29) 1,12–2,15; seçili sütun ve satır bantları (`accent`, alfa 20–110)
1,27–1,53; yüzey rengi delikler 1,12–1,28. Tam liste `analyze.py` çıktısında.

## Bulunan ve düzeltilen eşik altı öğeler

| öğe | önce (koyu / açık) | düzeltme | sonra |
|---|---|---|---|
| Açılmış kapsülün geçersiz başlığı (19 px, 600) | 3,75 / 3,46 (`invalid`) | `open::tesir::NO_LONGER_HEADLINE` = `muted` (yüz `oltan` ile aynı kural) | 9,77 / 14,35 |
| Grafik eksen çizgisi (`axis`) | 2,95 / 2,88 | `oltan` kaydı: `#5C606A` → `#626670`, `#96989E` → `#8E9096` | 3,23 / 3,19 |
| Gri işaretin çizgileri, kendi güven plakasının üstünde (yüz, kapsül, grafik; `siluet` ve `tg5` "tutmadı" izi) | 2,93 / 2,70 | yeni tema alanı `grey_plate_alpha_max` (koyu 49, açık 29): gri plakanın tavanı; renkli plakalar değişmedi | 3,09 / 3,08 |
| Hikâye çizelgesi: seçili hikâyenin satır bandındaki başka kapanmış şeritler | 2,85 / 2,73 | `oltan` `story.focus_row_alpha` 40 → 20 | 3,33 / 3,07 |
| Kabuk denetimlerinin sınırı (`select`, `input[type=date]`, metin alanı) | 1,41 / 1,25 (`line`) | yeni kabuk rolü `chrome.control` (`--control`; `oltan` koyu `#6E7076`, açık `#7E8084`; `r4` ve öteki görünüşlerde `line` ile aynı) | 4,09 / 3,40 |

Her düzeltme `look::tests::oltan_meets_the_hard_thresholds`'ta sınanıyor (başlık rolü ≥ 4,5,
eksen ≥ 3, gri plakada ≥ 3, satır bandında bütün şerit renkleri ≥ 3, denetim sınırı ≥ 3);
anlam renkleri değişmedi (ΔE00 ladder testi aynı).

## Sınırlar

- Yalnız gösterim alfabesi `siluet` ve sonuç işareti `tg5` ölçüldü; `rozet` (seçilebilir) ve
  `zemre`/`ornek` görünüşleri ölçülmedi (`grey_plate_alpha_max` onlarda 0).
- Açılmış kapsüller adım 200'deki akıştan (1 canlı, 6 kapanmış/geçersiz); yetkisiz okur ölçülmedi
  (yüz sahnesi çizilmez, metin DOM'da; denetimin 224 DOM düğümü ölçümü önceki koşuda).
- Bitişik renk nokta örneklemesiyle; "en ayırt edici kenar" kuralı bir yorumdur (WCAG 1.4.11
  bitişik renklerden en az birine karşı ayırt edilebilirlik). Saydam bantları süs saymak da bir
  yorumdur: bantların üstündeki işaretler banda karşı ölçüldü.
