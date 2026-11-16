# M8 — `tavrik`: işaret önde

Kod: `crates/qp-grammar/src/faces/tavrik.rs` (kayıt satırı `faces.rs`'te). Dal `r4/m8-b`.
Okuyucuya gitmez.

## Amaç

Kapalı yüzün baskın öğesi **büyük işarettir** (76 px; `r3` kartında 40 px). Ne olduğu (tür),
yönü ve geçerliliği önce işaretin şeklinden okunur; akışı kaydıran kişi metni okumadan
işaret sütunundan örüntüyü görür (yukarı/aşağı okları, gri kapanmış olanlar, rozetler).
Yanında tek, kısa bir başlık cümlesi: ne oldu · ne zaman · hâlâ geçerli mi · geçmiş karnesi.

## Ne gösterir / ne göstermez

- Gösterir: enstrüman (sahnenin başlık satırı, yalnız sembol — varsayılan başlıktaki "1d/1h"
  periyot eki iç terim olduğu için çıkarıldı), işaret (alfabe ve sonuç stili bakış ayarından;
  `siluet` ve `rozet` ile denendi), başlık cümlesi.
- Göstermez: fiyat çizgisi, tarih ekseni, karne satırı (`Karne x/n` yalnız Türkçe ve "az
  örnek"i söylemiyor; karne başlığın son öbeğine taşındı). Ek bilgi birimi **0**.
- Tavsiye yok: cümlede eylem sözcüğü yok.

## Yerleşim (390 CSS px genişlik, yükseklik 156 px)

Tek kart paneli (bütün katmanlar panel konumlu: ızgara/eksen yok). Başlık satırında sembol
(16 px). Altında solda işaret (76 × 76 px), sağında 14 px boşlukla başlık (15 px, temanın metin
rengi), panel sağ kenarında satır kırılır; en uzun başlık 4 satır, kesilme yok. Görünür aralık
son bar (ay etiketi çizilmez).

## Başlık şablonu (`HeadlineParts`)

`<Olay> · <son geçişin anı (moved)> · <geçerlilik> · <geçmiş karnesi>`; olay büyük harfle
başlar (Türkçe i/İ). Karne "geçmişte …" / "past: …" ile verilir ki kapsülün kendi sonucu
("gerçekleşti") ile karışmasın. 78 karakteri aşarsa karne "geçmişte"sini, az örnekte
sayılarını bırakır ("az örnek" asla düşmez); sert sınır 80. Rakam eklemez (yetkisizde parça
metinleri rakamsız).

| Durum | TR | EN |
|---|---|---|
| #1 | Yukarı kırılım · 4 saat önce · hâlâ geçerli · geçmişte 74'te 36 tuttu | Upside breakout · 4 hours ago · still holds · past: 36 of 74 played out |
| #3 | Aşırı alım · cuma günü · artık geçerli değil · geçmişte 41'de 16 tuttu | Overbought · on Friday · no longer holds · past: 16 of 41 played out |
| #6 | Aşağı kırılım · 10 Şubat'ta · tutmadı · geçmişte 5'te 4 tuttu (az örnek) | Downside breakout · on 10 February · did not play out · few cases |
| #10 | Olağandışı para akışı · dün · hâlâ geçerli · karnesi yok | Unusual money flow · yesterday · still holds · no record yet |
| #12 | Aşağı dönüş · dün 17:00'de · hâlâ geçerli · geçmişte 17'de 9 tuttu (az örnek) | Downward reversal · yesterday at 17:00 · still holds · few cases |

## Düzeneğin hesabı (`m8 --face tavrik`, 2026-11-16)

| Ölçü | Değer | Eşik |
|---|---|---|
| Başlık en çok / ort. TR (sahneli 10 durum) | 78 / 70,9 | ≤ 80 |
| Başlık en çok / ort. EN (sahneli 10 durum) | 76 / 66,6 | ≤ 80 |
| Başlık (düzeneğin tüm 11 durumu) | TR 104 / 73,9; EN 95 / 69,2 — #9 yüzünden | **geçmez** |
| En düşük metin karşıtlığı açık / koyu | 15,8 / 11,74 | ≥ 4,5 |
| İşaret karşıtlığı (tanıtıcı parça) en düşük açık / koyu | 2,06 / 2,36 (#5–#7 kapanmış) | **geçmez** |
| En küçük yazı | 15 px | ≥ 11 |
| Ek bilgi | 0 | ≤ 2 |
| İşaret alanı | 5.776 px², yüzün %9,5'i (`r3` %1,7) | — |

Gri kopya: yön okun yönüyle, kapanmış/artık geçerli değil köşe rozetinin şekliyle (✓, ✗,
halka) ayrılır.

## Bilinen zayıflıklar

1. **#9 (yetkisiz okuyucu)**: kabuk sahne çizmez, kapsülün uzun cümlesini düz metin
   gösterir (104/95 karakter); şablon orada uygulanmaz. Bütün tasarımlarda aynı (README Açık
   sorular 2); başlık eşiğini bu durum düşürür, yüz dosyasından düzeltilemez.
2. **Kapanmış işaretin karşıtlığı** (2,06–2,94): alfabe kapanmış hikâyeyi gri ve tazelikle
   soluk çizer; glif katmanı renk/opaklık almaz, düzenek de yalnız işaretin kendi ilkellerini
   ölçer. Yüz dosyasından değiştirilemez; `r3` de aynı değerde.
3. Başlık tek metin katmanı: durum ve karne ayrı, küçük "ek bilgi" olarak çizilemedi —
   bulgunun metin alanları `headline`, `ledger` (yalnız TR, "az örnek" yok) ve `claim`;
   yeni alan `layer.rs` ister (kapsam dışı). Bu yüzden karne başlığın son öbeği.
4. Gramerin satır kırma kuralı öbekleri bölebilir ("15 / Mayıs'ta"); bölünmez boşluk da
   kırılır.
5. EN'de az örnekte karne sayıları ve "past:" kısalmayla düşer (uzunluk); TR'de kalır.
6. Başlık uzunluğu `r3`'ten çok kısa ama ortalaması 70 civarı; eşitlik bozucu "kısa başlık"
   bu tasarımın lehine değil.
