# Kapsülün yüzü (R4, K43)

> Seçim persona ve eleştirmen **ajanlarıyla yapılmış bir ön elemedir**. Persona ajanı insan
> değildir; küçük pikseli ve kısa bakışı insan gibi deneyimlemez. İlk iki tasarım gösterimde
> ayardan seçilebilir ve son söz insan testi v2'dedir (`rounds/R4/human-test/`, `DECISION.md`).

Kanıt: `rounds/R4/experiments/M8/RESULTS.md` (ölçüler, ham kayıtlar `raw/`), şartname
`rounds/R4/experiments/harness/SPEC.md` §2 ve ekleri E1–E7, tasarım notları
`rounds/R4/experiments/M8/<ad>/DESIGN.md`. Kavramlar `docs/CONCEPTS.md` ("Kapsülün yüzü",
"Başlık cümlesi", "Ek bilgi", "Süre ifadesi", "Geçerlilik", "Başlık yapı taşları").

## Seçilen: `vadrel` — "cümle önde"

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

### Kapsül dili ile (R5/`m17-int`, `docs/LANGUAGE.md`)

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

## Ölçüler (SPEC §2; başlık #9 hariç, E7)

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

## İkinci: `fotolay` — "olayın resmi"

Küçük, sade bir fiyat izi (bar başına nokta), olayın anında ince dikey çizgi, olayın kanıt
geometrisi (seviye/bant/dönüş noktaları), işaret anın üstünde; üstte kısa başlık
(`{Olay}, {zaman}: {geçerlilik}. {karne}`). Geçerli değilse resmin tamamı soluk. 228 px.
Persona doğruluğunda `vadrel`'in 0,04 gerisinde (eşik altı), eleştirmen sırasında geride.
Zayıflığı: derleyici her veri paneline eksen/ızgara/değer etiketi ekliyor ("eksensiz resim"
tam kurulamadı; gramere "çıplak panel" bayrağı önerisi); en küçük yazı 11 px (eksen etiketi).
Ek bilgi sayımı bir yoruma dayanır (grafiğin etiketleri ayrı sayılsaydı 4 olur, eşikten elenirdi).

## Elenenler ve gerekçe

- **`tavrik` — "işaret önde"** (76 px işaret, tek satır başlık, 156 px). Eşikleri geçti,
  eleştirmenlerde `vadrel` ile birinciye eşit (1,67), ama persona tam doğrulukta 0,596 ile
  üçüncü (birincil ölçüde `fotolay`'ın 0,13 gerisinde). Okuyucular yön sözcüğü taşımayan
  başlıklarda ("Aşırı alım", "Para girişi") yönü "belirsiz" okudu; "karnesi yok" ifadesi
  "hiçbir bilgi göremedim" diye okundu.
- **`r3` — R3 kartı** (başvuru, sıralanır, seçilmez). Başlık 205 karakter (eşik 80), açık
  temada metin karşıtlığı 2,79 (eşik 4,5), ek bilgi 3 (eşik 2); persona tam doğru 0,141.
  Gösterimde adla seçilebilir kalır (var olan altınlar `r3` ile sabittir).

## Açık sorunlar

1. **Kapanmış işaretin karşıtlığı 2,06 (< 3, WCAG 1.4.11):** alfabe kuralı (gri + tazeliğe göre
   saydam), bütün yüzlerde aynı; yüz tasarımıyla düzelmez. Alfabe değişikliği var olan glif
   altınlarını değiştirir; R4'te yapılmadı, öneri (REPORT).
2. **#1 "bugün" ↔ "4 saat önce":** anahtar 3 saatten sonrasını "bugün" sayıyor, ürün "4 saat
   önce" yazıyor; okuyucuların 23/24'ü "birkaç saat önce" seçti. Tasarımları ayırmıyor; anahtar
   ya da süre ifadesi kuralı (T3-5) insan testinde gözlenmeli.
3. **#9 yetkisiz okuyucu:** düz metin zaman, geçerlilik ve karne vermiyor (0/6 her tasarımda).
   Yetkisiz okuyucu için şablonlu rakamsız yüz önerisi. (R5/`m17-int`) Kapsül dili seçiliyken
   metin, dilin rakamsız yüzüdür: zaman, geçerlilik, nitel karne ve neyin hangi yetkiyle
   açıldığı (`docs/LANGUAGE.md` "Yetkisiz okurun yüzü"); insan testinde yeniden ölçülmeli.
4. **Gri kopya kanıtı (§6) eksik:** bir eleştirmen gri kopyaları açmadı; gri ΔE ölçüsü
   düzenekte yok. Erişilebilirlik insan testinde (renk körü katılımcı varsa) ayrıca bakılmalı.
5. **Olay sözcüğü eşlemesi** ürünün ad öbeği metnine bağlı (kırılgan); `HeadlineParts`'a olay
   türü alanı eklenmesi önerisi.
6. Persona modelinin tavanı: S1b ve S5 0,91–1,00 ve F ile H neredeyse aynı; "beş saniyelik
   bakış" F personasında görünmüyor. Bu yüzden ajan sıralaması kaba bir sıralamadır.
