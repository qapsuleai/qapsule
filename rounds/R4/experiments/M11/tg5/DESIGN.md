# M11 — `tg5`: negatif karo / sönük iz

Kod: `crates/qp-grammar/src/glyph/outcome/tg5.rs` (+ `glyph/outcome.rs`'te `mod tg5;` ve
`STYLES`'a `&Tg5,`). Düzenek değişmedi; varsayılan `r3`, altınlar bayt eşit.

## Fikir

Hüküm küçük bir köşe işaretinde değil, **glifin bütün mürekkep dengesinde** okunur
(baskı/mühür ve skor tabelası esinli: kazanan satır ters basılır, kaybeden sönükleşir).
İki sonuç renk olmadan da her kanalda birbirinin tersidir:

| | gerçekleşti | tutmadı (ve süresi doldu) | açık |
|---|---|---|---|
| zemin | koyu dolu levha (siluet: yuvarlak köşeli kare, kutunun tamamı; rozet: bütün para) | boş | boş |
| glif | levhadan oyulmuş, zemin renginde (negatif baskı) | soluk gri (muted, %55 opaklık) | yön renginde |
| köşe (sağ alt, ≈ kutunun %43'ü) | açık disk içinde koyu ✓, levha renginde halka | zemin halesi üstünde kalın çıplak ✗ | yok |

- Glif bozulmaz: alfabe glifi açıkmış gibi çizer (`open_look`), stil yalnız renkleri
  çevirir ve levha/köşe ekler; tür, yön, güç, güven, teyit, sunucu kanalları durur.
- Açık aşamalar ve geri çekildi: alfabenin kendi çizimi (değişmez).
- Kurallar: yalnız IEEE temel işlemleri (köşe yayları sabit tablo), renkler temadan, bütün
  ilkeller kutuda (düzenek: kutu dışı 0).
- `legend` (TR, 25 sözcük): "Koyu dolu kare içinde açık renkli şekil ve köşede ✓
  gerçekleşti; soluk gri şekil ve köşede kalın ✗ tutmadı; renkli ve köşe işaretsiz şekil
  hâlâ açık." (rozette "kare" → "para", "şekil" → "desen"/"rozet").

## Kendi ölçüm (`m11 measure --style tg5`, `M11/raw/tg5/distinct.{json,md}`)

| alfabe | en kötü koşul ΔE | çift / koşul | başvuru `r3` | oran | gerçekleşti↔açık en kötü | tutmadı↔açık en kötü |
|---|---:|---|---:|---:|---:|---:|
| siluet | **42,11** | `squeeze.up`, 16 / açık / protanopi | 2,98 | ×14,1 | 37,48 | 6,26 |
| rozet | **16,44** | `breakout.down`, 16 / açık / gri | 11,33 | ×1,45 | 15,83 | 13,74 |

İki alfabe üzerinden ana ölçü **16,44** (rozet). Kutu dışı ilkel 0. Seçim kuralı 1'i iki
alfabede de geçer. Sayfalara (`m11 sheets`, tohum 1, siluet ve rozet) baktım: koyu karo /
gri+✗ / renkli üçlüsü 4× büyütmede ilk bakışta ayrılıyor. Okuyucu ajan koşmadım.

## Bilinen zayıflıklar

- **Görsel ağırlık:** gerçekleşen kapsüller akışta en koyu öğe olur; dikkat "tuttu"lara
  kayar (tasarımın dürüstlüğüne karşı bir eğilim). İnsan testinde bakılmalı.
- **Güven levhada zayıf:** güven dolgusu levhada zemin renginde %24 açıklık olarak kalır;
  yüksek güvende oyulmuş glif ile açık bant arasındaki karşıtlık düşer (ör. kırılım ↑,
  güven 0,7).
- **Rozet** levhası halkayı da zemin rengine çevirir: para kenarında açık halka parçaları
  kalır; görünüm siluetteki kadar temiz değil ve kazanç (×1,45) siluetten çok küçük
  (rozetin kendi kalın çerçevesi zaten güçlü bir başvuru).
- **Tutmadı ↔ açık** siluette gri koşulda 6,26: ayrımı çoğunlukla ✗ ve solukluk taşır;
  açık glifin rengi gri kopyada orta tonlu olduğundan levha farkı kadar güçlü değil
  (yine de r3'ün 2,23'ünün üstünde).
- Süresi doldu ile tutmadı aynı (şema gereği, kapsam dışı).
