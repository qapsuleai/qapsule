# M8 alternatifi `vadrel` — cümle önde

Kod: `crates/qp-grammar/src/faces/vadrel.rs` (kayıt: `faces.rs`, tek `FaceDesign` girdisi).
Yükseklik **188 CSS px** (390 genişlikte). Okuyucuya gitmez.

## Amaç

Kapalı yüzün baskın öğesi tek, büyük, gündelik bir cümle. Kişi beş saniyede yalnız cümleyi
okusa da **ne oldu, ne zaman (son hareket) ve hâlâ geçerli mi** sorularının cevabını alır.
İşaret cümlenin yanında küçük ve sakin; tek satır ek bilgi kısaltılmış karne.

## Ne gösterir / ne göstermez

Gösterir:
- **Başlık cümlesi** (20 px, büyük yazı): enstrüman özne, olay gündelik bir fiil öbeğiyle
  ("önemli bir eşiğin üstüne çıktı", "kısa sürede aşırı yükseldi"), zaman son geçişin anı
  (`moved`; düzeneğin S2 anahtarıyla aynı an), geçerlilik ayrı kısa cümle ("Hâlâ geçerli.",
  "Artık geçerli değil.", "Kapandı: 15 Mayıs'ta gerçekleşti.").
- **İşaret** (32 px, sol üst, cümlenin yanında): bakış ayarının alfabesi (`siluet`, `rozet`)
  ve sonuç stili; kabukta kod yok.
- **Ek bilgi (1):** yüzün üst satırı, kısaltılmış karne: "Benzerleri: 74'te 36 tuttu" /
  "Past: 36 of 74 played out"; az örnekte "(az örnek)" / "(few cases)" korunur; karne boşsa
  "Benzerleri: henüz örnek yok" / "Past: no cases yet".
- **Geçerlilik üç kanaldan:** sözcük (cümlenin son kısmı), işaret (gri siluet + köşe rozeti)
  ve cümlenin rengi: geçerliyken tam metin rengi (`text`), artık geçerli değil ya da
  kapandıysa `muted` (karşıtlık yine ≥ 5,2:1). Gri kopyada ayrım sözcük ve işaret şekliyle.

Göstermez: fiyat çizgisi, grafik, rakam etiketi, seviye çizgisi, "aile/teyit/bar" sözcükleri,
tavsiye. Doğuş anı (`when`) yalnız doğuş son hareketse yazılır (aşağıda).

## Yerleşim

Derleyicinin çizim kenarları geçerli (çizim alanı x 8..318, üst 34, alt `188 − 24`).

```
┌──────────────────────────────────────────────┐  0
│ Benzerleri: 74'te 36 tuttu        (16 px)    │  başlık satırı (görünümün title'ı)
│ ┌──┐ SYN03 önemli bir eşiğin                 │  34
│ │◆ │ üstüne çıktı; 4 saat önce               │
│ └──┘ güçlendi. Hâlâ geçerli.     (20 px)     │  en çok 5 satır
│                                              │
└──────────────────────────────────────────────┘  188
```

- Tek panel `face`; içindeki her katman panel konumlu (`glyph`, `headline`), bu yüzden ızgara
  ve eksen yok. x alanı yalnız son bar (tarih etiketi çizilmez).
- Başlık katmanı `DataSource::Finding` + `Mark::Text` + alan `headline` (düzenek buradan ölçer).
- Ek bilgi satırı görünümün `title`'ıdır (derleyicinin başlık satırı; başvuru `r3`'te orada
  "SYN03 1d" durur). Düzeneğin `extra_units` sayımı başlık satırını saymaz (0 yazar); **gerçek
  ek bilgi sayısı 1'dir** (karne).
- Hikâyesi olmayan bulguda başvuru kuralı (`depth::capsule_view`); parça yoksa başlık kapsülün
  cümlesi, üst satır boş.

## Başlık şablonu (`vadrel::headline`)

Biçimler (ilk ≤ 80 karakter olan seçilir):

| Durum | TR | EN |
|---|---|---|
| açık, son hareket doğuş | `{Zaman} {olay}. {Geçerlilik}.` | `{Time}, {event}. {Validity}.` |
| açık, güçlendi/teyit/zayıfladı | `{olay}; {zaman} güçlendi. {Geçerlilik}.` | `{event}; strengthened {time}. {Validity}.` |
| kapandı | `{olay}. Kapandı: {zaman} {sonuç}.` | `{event}. Closed: {ending} {time}.` |

80'i aşarsa sırayla: olay yerine ürünün ad öbeği (`SYN03: yukarı kırılım`), sonra kısa biçim
(açıkta zaman başa, fiil düşer; kapanmışta "Kapandı:" etiketi düşer). Birim testi 16 olay ×
6 en uzun zaman ifadesi × 10 aşama/geçerlilik × 2 dil, 6 karakterlik enstrümanla hepsinin
≤ 80 olduğunu, zamanı, geçerliliği ve enstrümanı içerdiğini, iç terim taşımadığını, yetkisizde
rakam eklemediğini denetler.

S8 örnekleri (düzeneğin hesabından, açık tema):

| # | TR | EN |
|---|---|---|
| 1 | SYN03 önemli bir eşiğin üstüne çıktı; 4 saat önce güçlendi. Hâlâ geçerli. | SYN03 broke above a key level; strengthened 4 hours ago. Still holds. |
| 2 | 3 işlem günü önce SYN11 önemli bir eşiğin altına indi. Hâlâ geçerli. | 3 trading days ago, SYN11 broke below a key level. Still holds. |
| 3 | SYN06 kısa sürede aşırı yükseldi; cuma günü zayıfladı. Artık geçerli değil. | SYN06 rose too far too fast; weakened on Friday. No longer holds. |
| 4 | Dün SYN03 dar bir aralığa sıkıştı. Hâlâ geçerli. | Yesterday, SYN03 got squeezed into a narrow range. Still holds. |
| 5 | SYN25 önemli bir eşiğin altına indi. Kapandı: 15 Mayıs'ta gerçekleşti. | SYN25 broke below a key level. Closed: played out on 15 May. |
| 6 | SYN25 önemli bir eşiğin altına indi. Kapandı: 10 Şubat'ta tutmadı. | SYN25 broke below a key level. Closed: did not play out on 10 February. |
| 7 | SYN13 için büyük alım geldi. Kapandı: 10 Nisan'da süresi doldu. | SYN13 saw unusually heavy buying. Closed: ran out of time on 10 April. |
| 10 | Dün SYN09 için olağandışı para hareketi oldu. Hâlâ geçerli. | Yesterday, SYN09 saw unusual money flow. Still holds. |
| 11 | SYN12 önemli bir eşiğin altına indi; 2 saat önce güçlendi. Hâlâ geçerli. | SYN12 broke below a key level; strengthened 2 hours ago. Still holds. |
| 12 | SYN05 yükselişten aşağı döndü; dün 17:00'de güçlendi. Hâlâ geçerli. | SYN05 turned down after rising; strengthened yesterday at 17:00. Still holds. |

## Hesap (düzenek `m8 --face vadrel`, sert eşikler SPEC §2(b))

| Ölçü | Değer | Eşik |
|---|---|---|
| başlık en çok / ort. (sahneli 10 durum) | TR 75 / 66,1 · EN 77 / 66,0 | ≤ 80 ✓ |
| başlık en çok (düzeneğin hesabı, #9 dahil) | TR 104 · EN 95 | ≤ 80 ✗ (#9) |
| en düşük metin karşıtlığı | açık 5,78 · koyu 5,27 | ≥ 4,5 ✓ |
| işaret karşıtlığı, açık durumlar | açık 5,16 · koyu 5,29 | ≥ 3 ✓ |
| işaret karşıtlığı, kapanmış (#5–7) | açık 2,06 · koyu 2,36 | ≥ 3 ✗ |
| en küçük yazı | 16 px (karne satırı; başlık 20 px) | ≥ 11 ✓ |
| ek bilgi | düzenek 0; gerçek 1 (karne satırı) | ≤ 2 ✓ |
| yüz yüksekliği | 188 px | (E4: kayıt) |

## Bilinen zayıflıklar

1. **#9 (yetkisiz okuyucu):** yüz sahnesi yok, kabuk kapsülün uzun cümlesini düz metin gösterir
   (ürün kuralı, bütün tasarımlarda aynı; README Açık soru 2). Düzeneğin başlık eşiği bu metni de
   sayar (TR 104), bu yüzden "başlık ≤ 80" düzenek hesabında geçmez; şablon orada çizilmez.
2. **Kapanmış işaretin karşıtlığı:** `siluet` kapanmış ve eski hikâyeyi gri ve tazelikle soluk
   çizer (opaklık 0,3 + 0,7·tazelik); #5–7'de tanıtıcı parça 2,06–2,94. Başvuru `r3` ile aynı
   değerler; yüz tasarımı alfabenin rengini değiştiremez (`glyph/`, bu görevin dışında).
   Geçerlilik yine sözcük ve soluk cümle rengiyle görünür.
3. **Olay sözcükleri eşlemesi** ürünün ad öbeği metnine (`realize::tr::noun_state`,
   `en::noun_bare`) bağlı: o metinler değişirse şablon ürünün ad öbeğine düşer (iç terim
   "kırılım" görünür, yanlış olmaz). `HeadlineParts`'ta tür alanı olsaydı eşleme gerekmezdi.
4. **Son hareket ≠ doğuş:** zaman son geçişin anıdır; doğuş gün içi geçişte yazılmaz (#1:
   doğuş 5 saat önce, cümle "4 saat önce güçlendi"). "Güçlendi" teyidi de kapsar (iç terim yok).
5. **Ek bilgi satırı üstte:** derleyicinin başlık satırı sabit konumda ve metin renginde (16 px);
   karne satırı cümleden önce okunabilir. Boy farkı (20 / 16) ve işaretin cümleyle hizası
   hiyerarşiyi cümleye verir, ama satır `muted` yapılamıyor.
6. Sağdaki 72 px çizim kenarı (derleyicinin değer etiketi payı) boş kalır; cümle 268 px'te
   kırılır, bu yüzden 188 px gerekir.
7. İngilizce kalıplar ("rose too far too fast", "kept sliding") gündelik ama kısa tutmak için
   virgülsüz; EN sayfa okuyucuya gitmez, yalnız hesapta.
