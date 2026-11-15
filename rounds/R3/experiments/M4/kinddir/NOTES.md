# M4 aday `kinddir` — "Tür + yön"

Crate: `rounds/R3/experiments/M4/kinddir/` (`m4_kinddir::theme()`, ad `kinddir`). Düzenek:
`harness-sim` (kayıt satırı eklendi). Ölçümler başvuru sonuç kuralı (`reference.atr_barrier`)
ve başvuru sıralayıcıyla. Ham: `M4/raw/kinddir/`; gerçek veri hamı depoya girmez, yalnız
toplu özet (`summary-real.*`).

## Kural

Öncül: aynı enstrüman (ve seri), aynı anlam türü ve aynı yön → aynı hikâye; farklı tür ayrı
hikâyedir. Tema anahtarı `<tür>.<yön>` (ör. `breakout.up`). Bir olgu için sırayla:

1. **Durum çıkışı (`exit`):** aynı tür ve yönde, aynı dedektörün `enter`'ını taşıyan açık
   hikâyeye katılır (birden çoksa en son etkin olan) → motor `weakened / state_ended` yazar.
   Böyle hikâye yoksa `Ignore`: bitmiş bir durumdan hikâye doğmaz. Çıkışta boşluk sınırı yok;
   çıkış her zaman kendi girişine aittir.
2. **Yönlü akış olgusu (K30 istisnası):** `flow` türünde, yönü `up`/`down` olan olgu, son
   kendi-tür olgusu en çok **1 bar** önce (aynı bar ya da bir önceki bar) olan açık, yönlü bir
   *fiyat anlamlı* hikâyeye (kırılım, dönüş, devam) katılır: aynı yön → `confirmed`
   (yeni aile) / `strengthened`; ters yön → `weakened / opposite_fact`. İkisi birden varsa en
   son etkin olan; eşitlikte teyit kazanır. Yakında fiyat hikâyesi yoksa 3. kurala düşer
   (kendi `flow.<yön>` hikâyesi).
3. **Aynı tür + yön:** hikâyenin son kendi-tür olgusu en çok **10 bar** önceyse katılır (en
   son etkin olan); daha eskiyse **yeni hikâye** (uzun sessizlikten sonra gelen ikinci kırılım
   ikinci bölümdür).
4. **Ters yön, aynı tür:** ayrı hikâye (`breakout.up` ile `breakout.down` iki ayrı iddia).
5. **Nötr yön:** yalnız kendi türünün nötr hikâyesine katılır; hiçbir şeyi teyit etmez, hiçbir
   şeye karşı çıkmaz (nötr akış olgusu fiyat hikâyesine katılmaz).

Durumsuzdur (yalnız açık hikâyelere bakar), deterministiktir; saat, ağ, rastgelelik yok.
Bütün katılımlar motorun kabul ettiği biçimdedir (motorun düşürdüğü olgu: her veride 0).

## Seçimler ve nedenleri

- **Teyit (K30) nasıl ele alınıyor?** Katı bir tür + yön anahtarı hacmin kırılımı teyit
  etmesine hiç izin vermez; üstelik bütün fiyat dedektörleri `price` ailesinde olduğu için
  hiçbir fiyat hikâyesi `confirmed`'a ulaşamaz (katı çeşitte etiketli kümede çok aileli
  hikâye sayısı 0; teyit işareti yalnız akış hikâyelerinde görünür). Bu, öncülün dürüst
  sonucudur ve K30 ile çelişir. Bu yüzden tek bir istisna aldım: **akış türü kanıttır**,
  kendi başına iddia değildir — yakında aynı yönde bir fiyat iddiası varsa ona aile olarak
  eklenir. Kimlik kuralı fiyat anlamlı türler için katı kalır (kırılım asla devam
  hikâyesine, aşırılık asla kırılıma katılmaz). Aşırılık ve sıkışma akışla teyit edilmez:
  aşırı alımda hacim patlaması "teyit" mi "tükenme" mi belirsiz; sıkışma yönsüzdür.
- **Pencere 1 bar:** hacim teyidi kırılım barında ya da ertesi barda anlamlıdır. Etiketli
  kümede 0, 1, 2, 3 bar aynı sonucu verir (hacim kırılım barında); rejimli pazarda pencere
  büyüdükçe akışın tesadüfen bağlandığı olgu artar (s1: 0 bar 1.161, 1 bar 1.370, 2 bar
  1.501, 3 bar 1.606). Ölçülemeyen doğruluk yerine dar pencereyi seçtim. Katılan akış olgusu
  pencereyi uzatmaz (yalnız fiyat türü olgular sayılır).
- **Ters akış zayıflatır:** öncülde aynı tür içinde "ters yön" ayrı hikâye olduğundan §9.3'ün
  `opposite_fact` geçişine başka yoldan ulaşılmaz; yükselen kırılım barında güçlü satış
  akışı tam bu geçişin anlattığı şeydir. Bedeli: motor zayıflatan olgunun ailesini eklemez,
  düzenek bunu "düşen aile" sayar (etiketli kümede tohum başına 12–21). Kapalı çeşit
  aşağıda.
- **Boşluk 10 bar:** K28'e göre tek tema tek hikâyedir; boşluk yalnız uzun sessizlikten sonra
  ikinci hikâyeyi ayırır. 3 / 5 / 7 bar daha fazla bölünme getirdi, arka planla karışmayı
  azalttı (84 → 54), yanlış birleştirmeyi değiştirmedi (zaten 0). Başvuru sonuç kuralında olay
  ufku 10 bar olduğundan boşluk olaylar için pratikte görünmez; daha uzun ufuklu bir sonuç
  kuralıyla anlam kazanır.
- **En son etkin hikâye:** boşluk kuralı aynı temada iki açık hikâyeye izin verdiğinden
  katılım en eskiye değil son kendi-tür olgusu en yeni olana yapılır.

## Ölçümler (son hâl)

Etiketli bölüm kümesi ve rejimli pazar (3 tohum), başvuruyla karşılaştırmalı:

| Veri | Azalma kinddir (başvuru) | Yanlış birleştirme | Karışık hikâye | Bölünme kinddir (başvuru) | Yok sayılan | Motorun düşürdüğü | Tür/yön farklı | Düşen aile |
|---|---|---|---|---|---|---|---|---|
| episodes-s1 | 0,445 (0,410) | 0,000 | 0 | 0,913 (0,941) | 990 | 0 | 243 | 12 |
| episodes-s2 | 0,455 (0,417) | 0,000 | 0 | 0,916 (0,944) | 959 | 0 | 265 | 21 |
| episodes-s3 | 0,459 (0,420) | 0,000 | 0 | 0,904 (0,937) | 972 | 0 | 266 | 12 |
| rejimli s1 | 0,484 (0,378) | — | — | — | 806 | 0 | 1.370 | 176 |
| rejimli s2 | 0,488 (0,379) | — | — | — | 798 | 0 | 1.341 | 199 |
| rejimli s3 | 0,493 (0,386) | — | — | — | 759 | 0 | 1.382 | 187 |
| gerçek 1day | 0,453 (0,425) | — | — | — | 740 | 0 | 161 | 21 |
| gerçek 1hour | 0,471 (0,411) | — | — | — | 1.345 | 0 | 578 | 80 |

"Tür/yön farklı" sütununun tamamı fiyat hikâyesine katılan akış olgularıdır (bilerek: cümlede
tür olarak değil, teyit işareti/aile olarak görünürler). "Düşen aile"nin tamamı ters akışla
zayıflatılan hikâyelerdir.

Şablon başına bölünme (3 tohum toplamı): `breakout_volume_continuation` 0,851 (başvuru
0,909), `squeeze_breakout` 0,903 (0,922), `pair_same` 0,792 (0,878), `pair_opposite` 0,803
(0,886), `extreme_enter_exit` 0,995 (0,995), `divergence_breakdown` 0,974 (0,978).

Çok aileli (teyitli) hikâye, s1: etiketli küme 231 (başvuru 0), rejimli 944 (başvuru 361;
başvurununkilerin hepsi akış hikâyesi, kinddir'de 652 kırılım, 165 devam, 14 dönüş, 113 akış).
Rejimli s1'de 3.115 akış olgusunun 1.370'i fiyat hikâyesine bağlandı (1.115 aynı yön, 255 ters).

Çeşitler (s1; deneme, seçim değil):

| Çeşit | Etiketli azalma | Rejimli azalma | Bölünme | Tür/yön farklı (rejimli) | Düşen aile (rejimli) |
|---|---|---|---|---|---|
| katı (akış istisnası yok) | 0,409 | 0,378 | 0,942 | 0 | 0 |
| pencere 0 | 0,444 | 0,468 | 0,915 | 1.161 | 89 |
| **pencere 1 (seçilen)** | **0,445** | **0,484** | **0,913** | **1.370** | **176** |
| pencere 2 | 0,445 | 0,496 | 0,913 | 1.501 | 214 |
| pencere 2, ters akış zayıflatmaz | 0,444 | 0,469 | 0,915 | 1.181 | 0 |
| pencere 1, boşluk 5 | 0,439 | 0,473 | 0,915 | 1.371 | 181 |

Katı çeşit başvuru kuralıyla neredeyse aynıdır (fark: en eskiye değil en son etkine katılma).

## Zayıflıklar (dürüstçe)

- **Bölünme yüksek kalıyor (~0,91).** Öncülün özü: türler arası geçen bölümler (sıkışma →
  kırılım, uyumsuzluk → kırılış, aşırılık + kırılım) her zaman ayrı hikâyelere düşer
  (`extreme_enter_exit` 0,995, `divergence_breakdown` 0,974). K28'in "bir tema bir hikâye"si
  burada "bir tür bir hikâye" olarak okunuyor; kullanıcı bir bölüm için birden çok kapsül
  görür. Bölünmenin bir kısmı da başvuru sonuç kuralının hikâyeleri erken kapatmasından gelir
  (kapanan hikâyeye olgu katılamaz); tema kuralı bunu değiştiremez.
- **Çelişen iki açık hikâye:** aynı türde ters yön ayrı hikâye olduğu için `breakout.up` ve
  `breakout.down` aynı anda açık kalabilir; fiyat türleri arasında `opposite_fact` hiç
  oluşmaz (yalnız akış zayıflatır).
- **Akış istisnası öncülü esnetir** ve ölçülemiyor: etiketli kümede kurumsal akış yok; rejimli
  pazarda akışın doğru mu tesadüfen mi bağlandığını gösteren etiket yok. Rejimli s1'de akış
  olgularının %44'ü bağlanıyor; sentetik akış fiyat getirisiyle kısmen ilişkili olduğundan
  bu "teyit"in bir kısmı aynı barın getirisinin yansımasıdır (aile bağımsızlığı K30'un
  varsaydığı kadar güçlü değil).
- **Aynı bar sırası:** motor olguları `(dedektör, kod)` sırasıyla işler; `flow.*` olguları
  `reversal.*` ve `squeeze.*`'tan önce gelir. Aynı barda uyumsuzluk + hacim olursa hacim
  dönüş hikâyesini teyit edemez (henüz doğmamıştır). Kırılım (`breakout.*`) önce geldiği için
  asıl durum çalışır.
- **Yok sayılan çıkışlar** başvuruyla aynı (tohum başına ~960–990): hikâyesi sonuç kuralıyla
  kapanmış durumların çıkışı. "Durum bitti" bilgisi bu durumda kaybolur.
- **Boşluk kuralı etiketle sınanmadı:** başvuru olay ufku (10 bar) boşluğa eşit olduğundan
  etkisi yalnız durum türlerinde ve birim testlerde görünür. Boşluktan sonra aynı temada iki
  açık hikâye olabilir (K28 ile gerilim; kimlikleri doğum barıyla ayrılır).
- **Nötr akış** (açılış = kapanış barında hacim) hiçbir şeyi teyit etmez; nadir ama kayıp.
- Parametreler (pencere 1, boşluk 10) ölçüye göre değil gerekçeye göre seçildi; etiketli küme
  ikisine de duyarsız, rejimli pazarda doğruluk ölçülemiyor.

## Testler

`cargo test` (17 test): kuralı gerçek motorda (scaffold sonuç kuralı, uzun ufuk) koşar —
aynı tür+yön katılımı ve tema anahtarı, boşluk sınırı (tam 10 katılır, 11 yeni), en son etkine
katılma, ters yön ayrı, kırılım barında hacim → `confirmed` (fiyat+hacim), ertesi bar
kurumsal akış → üçüncü aile, katılan akışın pencereyi uzatmaması, ters akış → `opposite_fact`
(ve kapalı çeşitte ayrı hikâye), katı çeşit, aşırılık/sıkışmaya akış katılmaz, nötr akış,
boşluktan sonra bile çıkış kendi durumunu bitirir, hikâyesiz çıkış yok sayılır, yeniden giriş
(boşluk içi katılır, dışı yeni), hiçbir katılımın motorca düşürülmemesi, belirlenimcilik.
