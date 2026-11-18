# M16 — tek bakış yüzü ve görünüş: kör ölçüm sonucu

Ölçüm ajanı `r5/m16-measure`, 2026-11-18. Düzenek: `rounds/R5/experiments/harness/glance/`
(SPEC §G, Ek G1–G5; **Ek G5 bu dalda ölçümden önce ayrı commit'te**: `85338e2`). Ham özet (ekran
görüntüsü yok): [`run/`](run/); bütün tablolar [`run/tables.md`](run/tables.md) (`tables.py` ile
üretildi). Örnek görüntüler (sentetik, tr/açık, `live_event`, kırpılmış): [`samples/`](samples/).

> Ajan okuyucu insan değildir; bu ölçüm bir ön elemedir (SPEC §G0). Son söz Arkin'in telefondaki
> karşılaştırması ve insan testindedir.

## Özet

| | `oltan` | `zemre` | `bertan` | `kivra` | `r4` (başvuru) | bozuk |
|---|---|---|---|---|---|---|
| Sert eşikler (§G7 + Ek G5) | geçti | geçti | **kaldı: renk** | geçti | kaldı: 5 eşik | kaldı |
| Okuma aşağı kalmama (1b; tam boy, üst sınır ≥ `r4` 0,866) | 0,865 [0,847, 0,882] | 0,862 [0,844, 0,880] | 0,855 [0,835, 0,874] | **0,775 [0,740, 0,809] → elendi** | 0,866 [0,847, 0,884] | 0,837 |
| **Birincil: kısma kopyası (F2:H1, %95)** | **0,471 [0,445, 0,497]** | 0,279 [0,257, 0,301] | 0,345 [0,319, 0,370] | 0,243 [0,225, 0,260] | 0,318 [0,295, 0,343] | 0,183 [0,171, 0,197] |
| Eleştirmen BT (%95) | **2,15 [1,82, 2,55]** | 1,24 [0,95, 1,54] | 1,30 [0,99, 1,65] | 1,79 [1,47, 2,19] | −1,20 [−1,64, −0,85] | −5,29 [−5,74, −4,94] |
| Gri kopya `dir`+`valid` | 1,000 | 1,000 | 0,997 | **0,873** | 1,000 | 1,000 |
| K52 (JIT açık bağlayıcı + JIT kapalı ≤ %10) | geçti | geçti | geçti | geçti | geçti | — |
| Paket (WASM brotli, `r4`'e göre) | +27,8 KB | +25,4 KB | +29,0 KB | +71,6 KB | 1 038,7 KB | — |

**Seçim (SPEC §G9, adım adım):** adım 1 sert eşikler → `bertan` (Ek G5 renk) ve `r4` elenir, hiçbir
eşik feragat edilmedi (her eşiği en az bir aday geçti) → adım 1b okuma aşağı kalmama → `kivra`
elenir → adım 2 birincil: `oltan` tek başına ilk grupta (aralığı `zemre`'ninkiyle örtüşmüyor) →
**kazanan `oltan`, ayırt edildi.** Bütünleştirmeye girecek varsayılan: **`oltan`**. İlk iki
(gösterimde seçilebilir): **`oltan`, `zemre`**.

Bozuk tasarım birincilde ve BT'de kesin son → koşu geçerli (§G8). Maliyet: **17,02 $** (bütçe 30 $).

**Duyarlılık (karar değil, bilgi):** `bertan` yalnız Ek G5 renk kapısında kalıyor; Ek G5'ten önceki
sahne tablosuyla (`colour_scene`) geçiyordu. O kapı uygulanmasaydı `bertan` 1b'yi geçer (üst sınır
0,874 ≥ 0,866), birincilde yine `oltan` tek başına ilk grupta olurdu → kazanan yine `oltan`, ama ilk
iki **`oltan`, `bertan`** olurdu (birincil nokta tahmini 0,345 > `zemre` 0,279). İlk ikinin ikincisi
Ek G5'e bağlıdır; orkestratörün bilmesi gereken tek çatal bu (aşağıda "Açık sorular 1").

## Düzenek ve protokol (okumalardan önce sabitlendi)

**Adaylar ve derlemeler.** Dallar birleştirilmedi; her aday kendi dalının `git archive` kopyasından
(scratchpad, depo dışı) ayrı derlendi (`scripts/demo.sh` WASM paketi + `qp` release;
`CARGO_TARGET_DIR=target/wt/m16m-<ad>`):

| ad | kaynak | seçim |
|---|---|---|
| `bertan` | `r5/m16-bertan` b658c65 | `?look=bertan&face=bertan` |
| `kivra` | `r5/m16-kivra` 1a774ce | `?look=kivra&face=kivra` |
| `oltan` | `r5/m16-oltan` 0abd4e5 | `?look=oltan&face=oltan` |
| `zemre` | `r5/m16-zemre` f05a98a | `?look=zemre&face=zemre` |
| `r4` (başvuru) | `main` d28cd2c + `broken/look.patch` | `?look=r4&face=vadrel` |
| `kestim` (bozuk, Ek G3; gizli ad) | aynı derleme | `?look=kestim&face=vadrel` |

Dört aday dalı `6808268`'den çıkıyor; `main` (d28cd2c) ondan sonra yalnız M18 belgelerini aldı (kod
farkı yok). Hepsi aynı metinle: `--voice marla` (Ek G4-1; 192 yüz kaydının hepsinde `used: marla`).

**Aynı içerik denetlendi:** 216 görüntüde (6 tasarım × 4 dil/tema × 9 ekran) her durum × dil × tema
için hedef kapsül ve oynatma adımı altı derlemede aynı. Durum kümesi (`r4` derlemesiyle `probe` +
`states`; kuru koşuyla aynı, S* = adım 1): `live_event` SYN06 kırılım yukarı; `no_longer` SYN28
(adım 40); `closed_realized` SYN21; `closed_failed` SYN26; `few_cases` SYN04 (adım 80, `watch`);
`no_record` SYN09 (adım 40, `watch`); `state_live` ve `visitor` SYN28 devam aşağı; `feed` adım 1.

**Çekim:** 402×874, DPR 1, `canvas=run`, `anim=0`, duyarlılık `all`, TR+EN × açık+koyu; `--kinds
face,feed` (kanıt ve hareket M20/M21'in konusu, M16 seçimine girmez; çekilmedi). 792 okuyucu uyaranı,
240 eleştirmen çifti (C(6,2) = 15 × 8 durum × 2 sıra; tr/açık).

**Okuyucu:** haiku, F ve H, seçenek sırası tohumla karışık, tek bakış. Kısma (birincil) 6 tekrar
(2 302 çağrı; 2 kayıt deneme çağrısıydı, aynı kod adıyla tekrar kullanıldı), tam boy + yetkisiz + gri
+ akış 3 tekrar (2 448), yarım boy 3 tekrar (1 152). Sonnet alt kümesi tr/açık yüz kümeleri 1 tekrar
(96). Eleştirmen sonnet, 240 çift × 3 tekrar (720). İstemlerde tasarım adı yok: `check_prompt` her
istemde (`key.json`'daki adlar + sabit liste), `run/prompts.md`'de aday adı 0 kez. Tek bakış dışı
(birden çok `Read`) 18 kayıt puanlamadan çıkarıldı.

### SPEC'ten sapmalar

1. **Kanıt ve hareket ekranı çekilmedi** (README maliyet tablosu onları sayıyordu). M16 yüzü ölçer;
   seçim kuralı onları kullanmaz.
2. **Adaylar ayrı derlemelerle çekildi** (§G8 "bütün adaylar aynı derlemeyle"). Dallar
   birleştirilemediği için tarif böyle istedi; `r4` ve bozuk aynı derlemede. İçerik eşitliği
   yukarıda denetlendi.
3. **K52 JIT kapalı "kullanılabilir" maddesi kapı olarak** (Ek G5-4; M18'in kararıyla aynı). Hiçbir
   adayı etkilemedi.

## Deterministik kapı (§G7 + Ek G5; ajan ölçümünden önce)

| tasarım | metin karşıtlığı en kötü / eşik | işaret karşıtlığı (≥ 3) | en küçük yazı (≥ 11) | dokunma < 44 | renk (Ek G5) en kötü çift | yüz karakter ort. | ekran karakter ort. | geçti |
|---|---|---|---|---:|---|---:|---:|---|
| `bertan` | 6,39 / 4,5 | 3,17 | 13 | 0 | **açık `up ↔ invalid` 7,28 / 10 (gri)** | 131 | 652 | hayır |
| `kivra` | 6,17 / 4,5 | 3,17 | 12 | 0 | koyu `down ↔ invalid` 11,45 / 10 | 133 | 900 | evet |
| `oltan` | 6,33 / 4,5 | 3,46 | 13 | 0 | koyu `up ↔ valid` 10,53 / 10 | 130 | 666 | evet |
| `zemre` | 6,61 / 4,5 | 3,11 | 12 | 0 | açık `down ↔ neutral` 10,43 / 10 | 170 | 601 | evet |
| `r4` | 2,75 / 4,5 (`karnesi yazı tura`, 11 px) | 2,06 | 10 | 108 | koyu `down ↔ invalid` 0,50 / 10 | 131 | 687 | hayır |
| bozuk | 1,62 / 4,5 | 1,26 | 10 | 108 | açık `down ↔ invalid` 0,31 / 15 | 127 | 678 | hayır |

**Renk kapısı ayrıntısı (Ek G5; tema başına, beş koşul, eşik 10: en küçük yüz işareti `kivra` 40,
`zemre` 56, `bertan` 64, `oltan` 120 px):**

- `bertan` açık: `up ↔ invalid` ve `up ↔ closed` 7,28 (gri).
- `bertan` koyu: `up ↔ invalid`/`closed` 9,35 (deutan); `down ↔ invalid`/`closed` 7,51 (gri).
- Öteki üç aday iki temada da bütün çiftleri geçiyor.
- Ek G5'ten önceki sahne tablosuna göre (bilgi): `bertan`, `kivra`, `oltan` geçerdi, `zemre` kalırdı.
  Yani Ek G5 iki adayın kapı sonucunu çevirdi.

**Bilerek paylaşılan roller** (normal ΔE00 < 5; sayılmadı; gri soruyla sınandı):

- Hepsinde `valid = text`, `invalid = closed`.
- `bertan`, `kivra`, `zemre`: `neutral = valid = text`.
- `oltan`: `neutral = invalid = closed` (tasarım notundaki "gri = yön yok" kararı).

Bunların biçimle okunması: gri kopyada `dir`+`valid` `no_longer` ve `closed_failed` durumlarında
her adayda 0,98–1,00 (aşağıda).

Kabuk yazısı ve dokunma hedefleri dört adayda düzeltilmiş (en küçük 12–13 px, < 44 px hedef yok);
`r4` kabuğu 33 px yüksek sekmeler ve 10 px güven yazısıyla kalıyor.

## K52 (yüz akışı; 3 tekrarın ortancası; ms)

| tasarım | JIT açık: tik / yüz / sahne p95 | ilk kapsül | kaydırma kare p99 | JIT kapalı: tik / yüz / sahne p95 | kaydırma p99 | kaydırmada uzun kare payı | geçti |
|---|---|---:|---:|---|---:|---:|---|
| `r4` | 1 / 1 / 2 | 200 | 19 | 1,6 / 2 / 5,6 | 19 | 0 | evet |
| `bertan` | 1 / 1 / 2 | 215 | 19 | 1,6 / 1 / 5,6 | 19 | 0 | evet |
| `kivra` | 1 / 1 / 1 | 203 | 19 | 1,6 / 1 / 7,4 | 19 | 0 | evet |
| `oltan` | 1 / 1 / 2 | 200 | 19 | 1,6 / 2 / 7,4 | 19 | 0 | evet |
| `zemre` | 1 / 1 / 2 | 204 | 19 | 1,6 / 2 / 5,6 | 28 | 0,021 | evet |

Hepsi bütçenin içinde. `zemre` JIT kapalı kaydırmada %2,1 uzun kare: eşiğin (%10) altında ama tek
sıfır olmayan değer bu.

## Paket boyu (`pkg/sizes.json`; yazı tipleri WASM'a gömülü)

| tasarım | wasm ham | wasm gzip | wasm brotli | `r4`'e göre brotli | yazı tipi dosyaları |
|---|---:|---:|---:|---:|---:|
| `r4` | 5 342 433 | 1 566 385 | 1 038 710 | — | 135 580 |
| `bertan` | 5 455 990 | 1 627 496 | 1 067 733 | +29 023 | 186 524 |
| `kivra` | 5 447 281 | 1 617 052 | 1 110 308 | +71 598 | 359 748 |
| `oltan` | 5 499 388 | 1 639 746 | 1 066 543 | +27 833 | 209 152 |
| `zemre` | 5 453 499 | 1 620 421 | 1 064 080 | +25 370 | 183 672 |


## Birincil: kısma kopyası (σ 4,5, 0,5×; F2:H1, %95 önyükleme)

| tasarım | toplam | tr/açık | tr/koyu | en/açık | en/koyu |
|---|---|---|---|---|---|
| `oltan` | **0,471 [0,445, 0,497]** | 0,411 | 0,422 | 0,542 | 0,512 |
| `bertan` | 0,345 [0,319, 0,370] | 0,242 | 0,221 | 0,477 | 0,440 |
| `r4` | 0,318 [0,295, 0,343] | 0,243 | 0,223 | 0,397 | 0,411 |
| `zemre` | 0,279 [0,257, 0,301] | 0,214 | 0,190 | 0,401 | 0,312 |
| `kivra` | 0,243 [0,225, 0,260] | 0,236 | 0,211 | 0,314 | 0,210 |
| bozuk | 0,183 [0,171, 0,197] | 0,190 | 0,167 | 0,223 | 0,153 |

Birim şansı ≈ 0,36 (`t_first` 0,25, `t_dir` 0,333, `t_valid` 0,5 dengeli). Soru başına (Wilson %95,
n ≈ 383):

| soru | `oltan` | `bertan` | `r4` | `zemre` | `kivra` | bozuk |
|---|---|---|---|---|---|---|
| `t_first` (işaret önce) | **0,83** | 0,54 | 0,61 | 0,52 | 0,58 | 0,45 |
| `t_dir` | **0,41** | 0,29 | 0,21 | 0,20 | 0,12 | 0,09 |
| `t_valid` (dengeli öncesi ham) | 0,16 | 0,25 | 0,15 | 0,13 | 0,03 | 0,01 |

`oltan`'ın üstünlüğü işaretin ilk göze çarpan öğe olmasından (K7: 120 px işaret) ve bulanıkta yönün
seçilmesinden geliyor. Her adayda TR, EN'den belirgin düşük: kısma kopyasında okuyucu çoğu zaman
"bilmiyorum" diyor. Bunu dil farkı mı, model davranışı mı belirliyor, ayrılamadı (aşağıda "Sorunlar 2").

## Tam boy okuma (`face` + `face_visitor`, F2:H1; kapı ve 4. sıra)

| tasarım | tam boy | yarım boy (bilgi) |
|---|---|---|
| `r4` | 0,866 [0,847, 0,884] | 0,849 |
| `oltan` | 0,865 [0,847, 0,882] | 0,810 |
| `zemre` | 0,862 [0,844, 0,880] | 0,806 |
| `bertan` | 0,855 [0,835, 0,874] | 0,842 |
| bozuk | 0,837 [0,817, 0,855] | 0,754 |
| `kivra` | **0,775 [0,740, 0,809]** | 0,768 |

`kivra`'nın düşüşü soru başına:

- `dir` 0,86, `valid` 0,82, `kind` 0,67, `when` 0,53; öteki adaylar 0,98–0,99 / 0,69–0,70.
- Gri kopyada `few_cases` durumunda 0,00 (48/48 yanlış).
- Yoğun satırda az örnekli kapsülün yönü ve geçerliliği okunmuyor.

`advice` (hiçbir şey söylemiyor) her tasarımda 1,00. `numbers` (yetkisiz) her tasarımda düşük
(0,29–0,50): okuyucu çoğu zaman yetkisiz yüzde de rakam gördüğünü söylüyor. Bu tasarımdan bağımsız
(bozukta da 0,50).

## Gri kopya — "biçimle de okunur" (Ek G5-3; birincilden ayrı)

| tasarım | renkli tam boy `dir`+`valid` | gri `dir`+`valid` |
|---|---|---|
| `oltan` | 0,99 | 1,00 |
| `zemre` | 0,99 | 1,00 |
| `bertan` | 0,98 | 1,00 |
| `r4` | 0,98 | 1,00 |
| `kivra` | 0,86 | **0,87** (`few_cases` 0,00) |
| bozuk | 0,99 | 1,00 |

Gri kopyada aralık dışı düşüş yok: gri, renkliden düşük değil. Paylaşılan renk çiftlerinin durumları
(`no_longer`, `closed_failed`, `closed_realized`) her adayda 0,98–1,00. Ajan okuyucu tam boyda metni
okuduğu için bu ölçü üst sınıra dayanıyor; anlamın yalnız biçimle taşındığını tek başına kanıtlamaz
(SPEC §G0).

## Eleştirmen (sonnet, tr/açık; BT, %95)

| tasarım | BT |
|---|---|
| `oltan` | **2,15 [1,82, 2,55]** |
| `kivra` | 1,79 [1,47, 2,19] |
| `bertan` | 1,30 [0,99, 1,65] |
| `zemre` | 1,24 [0,95, 1,54] |
| `r4` | −1,20 [−1,64, −0,85] |
| bozuk | −5,29 [−5,74, −4,94] |

Çelişki oranı > %30 (ayırt edilemedi):

- `bertan|kivra` 0,42;
- `kivra|zemre` 0,38;
- `oltan|zemre` 0,33.

Öteki 12 çift ≤ 0,25. Uzunluk yanlılığı yok:

- BT ile ekran karakteri Spearman −0,03;
- kesin sonuçlarda "daha çok metinli kazandı" payı 0,445 (n 308; eşik 0,7).

Seçimde BT'ye gerek kalmadı (adım 2'de tek aday). Tercih sırası yine birincille uyumlu: en üstte
`oltan`.

Okuyucu modelleri arası uyum (haiku–sonnet, tr/açık yüz kümeleri, ilk tekrar): 0,861 (n 576).

## Bozuk tasarım (§G8)

`run/broken-check.json`: **geçerli**.

- Birincilde kesin son: 0,183; en yakın `kivra` 0,243.
- Aralığı en iyinin aralığının altında.
- BT'de kesin son.
- Eşleştirilmiş fark her tasarımdan ayrık:
  - `oltan` +0,286 [0,258, 0,313]
  - `bertan` +0,159
  - `r4` +0,135
  - `zemre` +0,097
  - `kivra` +0,060 [0,040, 0,080]
- Tam boy okumada son değil (bilgi; `kivra` daha düşük).

## Seçim kuralının uygulanışı (`run/scores-result.json`)

1. Sert eşikler: geçen `kivra`, `oltan`, `zemre`; `bertan` (renk), `r4` (metin karşıtlığı, işaret
   karşıtlığı, en küçük yazı, dokunma, renk) elendi; feragat yok.
2. 1b, okuma aşağı kalmama (başvuru `r4` 0,866): `kivra` üst sınır 0,809 < 0,866 → elendi.
3. Birincil: `oltan` 0,471 [0,445, 0,497], `zemre` 0,279 [0,257, 0,301]; örtüşme yok → ilk grup `oltan`.
4. Adım 3–4 gerekmedi. **Kazanan `oltan` (ayırt edildi).** Varsayılan `oltan`; ilk iki `oltan`, `zemre`.

## Maliyet

| kalem | çağrı | $ |
|---|---:|---:|
| okuyucu haiku | 5 904 | 4,84 |
| okuyucu sonnet | 96 | 1,60 |
| eleştirmen sonnet | 720 | 10,58 |
| **toplam** | 6 720 | **17,02** |

## Sorunlar ve açık sorular

1. **İlk ikinin ikincisi Ek G5'e bağlı.** `bertan`'ı eleyen tek eşik Ek G5 renk kapısı: açık temada
   `up` ile `invalid`/`closed` gri tonlamada 7,28. Ek G5 ölçümden önce yazıldı; kural uygulandı. Kapı
   uygulanmasaydı ilk iki `oltan`, `bertan` olurdu. Ek G5 iki noktada `valid`'i ve işaretin kullandığı
   rolleri de çiftlere katıyor. Bunun anlam rengi olarak doğru kapsam olup olmadığı orkestratörün
   kararı. Kazanan iki okumada da `oltan`.
2. **Kısma kopyasında TR < EN** (her tasarımda, bozuk dahil). Mutlak doğruluklar düşük (0,18–0,47);
   sıralama dört birleşimin hepsinde `oltan` önde, bozuk sonda.
3. `numbers` (yetkisiz yüzde rakam var mı) bütün tasarımlarda şansa yakın. Okuyucu sorusu ya da kabuk
   metni sorunu olabilir (tasarımdan bağımsız; M16 kapsamı dışı).
4. `kivra` gri `few_cases` 0/48: az örnek durumunda yön ve geçerlilik yoğun satırda okunmuyor.
   Tasarımcıya bilgi.

## Öneriler (kapsam dışı)

- Ek G5 rol kümesi için kısa bir karar satırı: `valid` ve işaretin rolleri (`text`) çiftlere girsin mi?
- `face_visitor/numbers` sorusunun metni, ya da yetkisiz yüzün "rakam gösterilmiyor" ibaresi gözden
  geçirilsin.
