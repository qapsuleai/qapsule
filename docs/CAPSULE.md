# Kapsül ve Anlam İşaretleri — Tasarım Notu

Durum: ana akışta kararlaştırılan çerçeve (2026-11-15, K26–K31). R3 bu notu girdi alır;
ayrıntıları R3 tasarlar ve ölçer.

## 1. Üç katman, iki süzgeç

| Katman | Ne yapar | Nerede | Kullanıcıya nasıl ulaşır |
|--------|----------|--------|--------------------------|
| Olgu | Her şeyi tespit eder (formasyon tarayıcı gibi): tetikler, eskitir, listeler | Sunucuda, tüm semboller için, herkes adına bir kez; aynı dedektörler cihazdaki yansıda da çalışır | Liste: isteyen açar, tarar |
| Bulgu | Önem sıralar: nadirlik, güç, bağımsız kanıt ailelerinin teyidi, bu türün karnesi | Sunucu | Sıralı liste |
| Kapsül | Kişiye göre seçer: izleme listesi, pozisyon, niyetler, hassasiyet | Kişiye özel | Önüne gelir |

## 2. Kapsülün zamanı (K26, K27)

Üç tür vardır; dilleri ve ömürleri farklıdır:

| Tür | Örnek | Dil | Ömür |
|-----|-------|-----|------|
| Olay | "RSI 70'i yukarı kesti" | Geçmiş zaman, ne zaman olduğuyla | Olduğu anda biter; tazeliği solar |
| Durum | "RSI 70'in üstünde" | Şimdiki zaman | Koşul sürdükçe; bozulunca biter |
| Beklenti | "148 aşılırsa kırılım teyit olur" | Koşullu | Koşul gerçekleşene ya da geçersizleşene kadar |

Cümle her zaman olgunun zamanını ve bugünkü geçerliliğini yansıtır. Geçmişte olmuş bir
olay şimdiki zamanla sunulmaz.

## 3. Kapsül bir hikâyedir (K28)

Aynı enstrümanda aynı tema için ayrı ayrı kapsüller doğmaz; tek kapsül evrilir:

`aday → doğdu → güçlendi | zayıfladı → teyit edildi → gerçekleşti | geçersiz | süresi doldu | geri çekildi`

- **Aday:** bar kapanmadan görülen, fiyat dönerse kaybolabilecek oluşum. Yalnızca o anda
  o enstrümanı izleyene gösterilir; hiçbir zaman itilmez, karneye yazılmaz.
- **Kesinleşme:** bar kapanınca otorite kimlik verir ve karneye yazar.
- **Eskime:** türün doğal ömrü (bar sayısıyla, saatle değil), açık geçersizlik koşulu ve sonuç.
- **Geri çekildi (K37):** dayandığı veri hatalı çıktı. Piyasanın görüşü çürütmesi değildir;
  karneye yazılmaz.
- **Sonuç:** her kapsül sonunda "gerçekleşti" ya da "gerçekleşmedi" olarak kapanır; karne
  buradan beslenir.

## 4. Anlam işaretleri (K29, K30)

İşaret formasyonun şeklini değil anlamını kodlar. Formasyonun adı ve geometrisi kaybolmaz;
kapsül açıldığında kanıt olarak çizilir.

**Altı anlam türü (ilk faz):**

| Tür | Anlamı | Tipik kaynaklar |
|-----|--------|-----------------|
| Devam | Hareket sürüyor | Trend, bayrak, hareketli ortalama dizilimi |
| Dönüş | Hareket tersine dönüyor olabilir | Omuz-baş-omuz, ikili tepe/dip, uyumsuzluk |
| Kırılım | Bir sınır aşıldı | Bant, kanal, direnç/destek kırılımı |
| Sıkışma | Hareket daralıyor; yön belirsiz | Üçgen, bant daralması, düşük oynaklık |
| Aşırılık | Olağandışı uzama | Aşırı alım/satım, banttan taşma |
| Akış | Fiyat hareketinden bağımsız para girişi/çıkışı | Hacim, takas, kurumsal akış |

**Görsel kanallar:**

| Kanal | Kodladığı |
|-------|-----------|
| Şekil | Tür |
| Yön | Yukarı, aşağı, belirsiz |
| Büyüklük / kalınlık | Güç |
| Doluluk | Güven: bu türün karnesi |
| Çizgi biçimi | Aşama: aday (kesik), kesinleşmiş (dolu), sonuçlanmış |
| Solukluk | Tazelik |
| Teyit işaretleri | Kaç bağımsız kanıt ailesi destekliyor |

**Kanıt aileleri:** fiyat, hacim, akış (takas, kurumsal), türev, haber/temel. Bir bulgunun
gücü, kaç bağımsız ailenin onu teyit ettiğiyle artar. Aynı aileden beş gösterge tek teyit
sayılır.

**Okunabilirlik:** işaret başlangıçta her zaman cümlesiyle birlikte gelir. Ayırt
edilebilirlik küçük boyutta hesapla ölçülür; yapay zekâ ile ilk eleme yapılır; son söz
gerçek kullanıcı testindedir.

## 5. Dil: gözlem, tavsiye değil

Cümle beklenti değil gözlem ve geçmiş sıklık bildirir: "Bu durum geçmişte 100 seferin
62'sinde on bar içinde yukarı sonuçlandı." Karne dilin parçasıdır. Hukuki sınır açık
karar A4'tür; son sözü hukuk ekibi söyler.

## 6. Hassasiyet (K31)

Kullanıcı kendi hassasiyet seviyesini seçer; seçim arkadaki dikkat bütçesini belirler.

- Az sayıda seviye (ör. yalnız önemliler / dengeli / hepsi), kullanıcı grubuna göre
  farklı varsayılan.
- Alışkın kullanıcı için liste her şeyi gösterir; itme düşük tutulur.
- Geleceğin yatırımcısı için varsayılan dar: az ama önemli.
- Kullanıcının pozisyonu ya da bıraktığı niyetle ilgili kapsüller (özellikle
  geçersizleşme) seviyeden bağımsız önceliklidir.
- Sistem davranıştan öğrenip seviyeyi kendiliğinden değiştirmez; öneride bulunabilir,
  kararı kullanıcı verir.

## 7. Veri lisansı ve kanıtın açılması (K33)

Borsa veri lisansları iki kullanım ayırır: verinin kendisini **göstermek** ve veriyi
gösterimsiz kullanıp ondan **hesaplanmış yorum** üretmek. Örnek: kurumsal akış lisansı
olmayan kullanıcıya akış rakamları gösterilemez, ama rakam içermeyen bir yorum
gösterilebilir ("bu hissede X kurumu son bir haftanın alış ortalamasının üzerine çıktı").

Tasarıma etkisi:

- Her olgu, dayandığı verinin **lisans sınıfını** taşır.
- Kanıt, kullanıcının yetkisinin izin verdiği derinliğe kadar açılır. Yetki yoksa kapsül
  yorumu ve karneyi gösterir; ham kanıt için "bu kanıt şu veri lisansını gerektirir" der.
  Kanıtın var olduğu ve neye dayandığı her zaman söylenir.
- Kısıtlama arayüzde değil **otoritede** uygulanır: yetkisiz kullanıcının cihazına ham
  veri hiç gönderilmez. Bu yüzden bu tür olgular cihazdaki yansıda yeniden hesaplanamaz;
  yalnızca sunucuda hesaplanır ve yorum olarak gelir (**sunucu olgusu**).
- Yorum cümlesi ham değeri geri çıkarmaya izin vermeyecek biçimde kurulur: rakam yok,
  yalnızca nitel ilişki (üstünde, altında, arttı, azaldı).
- Karne lisanstan bağımsızdır: "bu tür yorum geçmişte ne kadar tuttu" herkese gösterilir.

## 8. Dilin sınırı (K32)

Nicel olarak ifade edilebilen her şey gösterilebilir: istatistik, geçmiş sıklık,
göstergelerin ürettiği sinyaller. Kapsülün dili sayıya dayanır; sayıya dayanmayan öznel
öneri kapsül olmaz.

---

## 9. Uygulama ayrıntısı (R3)

Bu bölüm yukarıdaki çerçeveyi değiştirmez; uygulanabilir ayrıntıya indirir. Kavramlar
`docs/CONCEPTS.md` "Kapsül (R3)"; şema `qp-schema::capsule`; sözleşme
`crates/qp-facts/src/lib.rs`, `crates/qp-capsule/src/lib.rs`, `crates/qp-grammar/src/glyph.rs`.
Ölçümle seçilen noktalar (M1–M7) "seçim" diye işaretlidir; seçimler `rounds/R3/experiments/`.

### 9.1 Katmanlar ve crate'ler

| Katman | Crate | Girdi → çıktı |
|--------|-------|---------------|
| Olgu | `qp-facts` | barlar (+ lisanslı akış) → `Fact` (`meaning` dolu). Toplu = akışkan, bit düzeyinde. |
| Bulgu + kapsül + karne | `qp-capsule` | kesin olgular + barlar → hikâye geçişleri (`Story`), karne, önem, cümle, hassasiyet süzgeci, sunucu olgusu yorumu |
| Otorite | `qp-session` | `qp-capsule`'ü bar kapanışında koşturur, sonucu komut yolundan dokümana ve günlüğe yazar; sorgular, abonelik, itiraz, geri çekme |
| İşaret | `qp-grammar` (`glyph`), `qp-render` | `GlyphSpec` → sahne ilkelleri → piksel |

Bulgu (`Finding`) ile hikâye bire birdir: her hikâye bir bulgu ve bir kapsül olarak
dokümana girer; bulgunun `facts` listesi hikâyeye katılan olgulardır.

### 9.2 Olgu: zaman, kenar, aday

- Dedektör her barda "bar şimdi kapansaydı" sonucunu verir (`qp-compute`'un bekleyen bar
  modeli). Açık barda verilen sonuç **adaydır**; bar kesinleşince (sonraki bar geldi ya da
  pencere kapandı: RP2 `Maturity::final`) olgu olur.
- Dedektör seviye değil **kenar** bildirir: anlık olay (`point`), durumun başlaması
  (`enter`) ya da bitmesi (`exit`). "Hâlâ doğru" her barda tekrar edilmez.
- Olgu kimliği belirleyicidir: `fx.<enstrüman>.<dedektör>.<kod>.<bar>`.
- Lisanslı veriden türeyen olgu `meaning.server = true`, `license.class = licensed` taşır;
  yalnız otoritede hesaplanır.

### 9.3 Yaşam döngüsü kuralları (sabit; deney değil)

Her kesin olgu, tema kuralının (M4 seçimi: tür + yön, akış teyit olarak — `kinddir`) kararıyla bir hikâyeye katılır, yeni hikâye
başlatır ya da yok sayılır. Bir barda olaylar şu sırayla işlenir: (1) açık hikâyelerin
sonuç kuralı (M5 seçimi: oynaklık bariyeri — `barrier`) bu barla; (2) ufuk dolanlar `expired`; (3) bu barın olguları
katılır/doğar; (4) önem (M3 seçimi: teyit merkezli birleşim — `confirm`) yeniden hesaplanır. Aynı barda birden çok olgu
`(dedektör, kod)` sırasıyla işlenir.

| Olay | Geçiş | Neden kodu |
|------|-------|------------|
| Yeni hikâye | `born` | `new_story` |
| Aynı yönde, hikâyede zaten olan aileden olgu | `strengthened` | `same_family` |
| Aynı yönde, yeni aileden olgu | `confirmed` (aile sayısı +1) | `new_family` |
| Ters yönde olgu | `weakened` | `opposite_fact` |
| Hikâyenin kendi durumunun bittiği (`exit`) olgu | `weakened` | `state_ended` |
| Sonuç kuralı: gerçekleşti | `realized` (kapanır, karne: isabet) | kuralın kodu |
| Sonuç kuralı: geçersiz | `invalidated` (kapanır, karne: ıska) | kuralın kodu |
| Ufuk (bar sayısı) doldu | `expired` (kapanır, karne: ıska) | `horizon` |
| Dayandığı veri düzeltildi / itiraz haklı çıktı | `retracted` (kapanır, karne **yok**) | `data_corrected`, `dispute_upheld` |

Kapandıktan sonra geri çekilen hikâyenin karne satırı silinmez, `retracted_at` ile işaretlenir: geri çekilmeden önce doğanların `ledger_at_birth`'i değişmez, sonra doğanlar o satırı saymaz (aynı `t`'de geri çekmeden sonra işlenen doğum da saymaz).

Kapanmış hikâyeye olgu katılmaz; aynı temada yeni olgu yeni hikâye başlatır.

### 9.4 Karne

- Anahtar: `<tür>/<dedektör>@<sürüm>` (hikâyeyi doğuran olgunun dedektörü).
- Her kapanış (geri çekilme hariç) karneye bir satır yazar: kapsül, anahtar, aşama, bar.
- **İleriye bakma yok:** bir hikâyenin `ledger_at_birth`'i yalnız doğum barının zamanından
  **kesin olarak önce** kapanmış hikâyeleri sayar (çok enstrümanda zaman sırası `t`).
  Sonuç kuralı yalnız `bars[..=now]` görür. Test: doğum anından sonraki veri karıştırılınca
  o ana kadarki karne ve hikâye geçmişi değişmez.
- Karne lisanstan bağımsızdır; herkese gider.

### 9.5 Dil (zaman türleri)

| Zaman türü | Cümle | Örnek (TR) |
|------------|-------|------------|
| Olay | Geçmiş zaman + ne zaman ("bugün", "3 bar önce", tarih) | "THYAO 3 bar önce son 20 barın en yükseğini yukarı kırdı." |
| Durum | Durum sürüyorsa şimdiki zaman; bittiyse geçmiş zaman + "artık geçerli değil" | "RSI 3 bar önce 70'in üstüne çıktı; şu an 55, artık 70'in üstünde değil." |
| Beklenti | Koşullu | "148 üzerinde iki kapanış olursa kırılım teyit olur." |

Karne cümlenin parçası olabilir ("bu durum geçmişte 100 seferin 62'sinde …"); lisanslı
olgunun cümlesi **hiç rakam içermez** (karne ayrı alanda gider). Üretim yöntemi M2 seçimi: belirleyici şablon dilbilgisi (`qp_capsule::realize::Template`), dil modeli yok.

### 9.6 İşaret

`GlyphSpec` kanalları §4'teki tablodur; alfabe (çizim kuralları) M1 ile seçilir ve gramere
`glyph` işareti olarak girer (T7). Kapsül görünümü: işaret + cümle + küçük grafik + karne.

### 9.7 Sunucu olgusu ve güven yolu

Yetkisiz kullanıcıya lisanslı olgudan yalnız güven yolunun (M6 seçimi: mühür — `qp_capsule::trust::Muhur`) ürettiği nitel
iletiler (`QualMessage`) gider: rakam yok, `trust = server` işareti, gösterim barı. Saldırı
ölçüsü: art arda gelen iletilerden ham değer aralığının daraltılması. Yetkisiz okuyucunun
bütün kapsül görünümü (bulgu güveni, önem, kademe, sıra, karne, olayların varlığı) yalnız
açık olgularla beslenen ikinci bir motorun sonucundan kurulur; mühür iletileri dışında hiçbir
kanalı lisanslı veriye bağlı değildir (R3/K1, `docs/PROTOCOL.md` §20.5).

### 9.8 Hassasiyet

`Sensitivity` → kademe süzgeci: `important` kademe 1, `balanced` 1–2, `all` hepsi.
Kişinin pozisyonu ya da etkin niyetiyle ilgili enstrümanın kapsülü (özellikle `weakened`,
`invalidated`) kademeden bağımsız geçer. Varsayılan gruptan gelir; sistem değiştirmez.

### 9.9 Seçimler ve bağlantılar (R3)

| Konu | Seçim (öneri, `DECISIONS.md` Ö29–Ö41) | Uygulama | Kanıt |
|------|------|----------|-------|
| İşaret alfabesi (M1) | `siluet` (insan testi `siluet` ↔ `rozet`) | `qp_grammar::glyph`, gramer `glyph` işareti (GRAMMAR §18) | `experiments/M1/RESULTS.md` |
| Cümle (M2) | şablon dilbilgisi | `qp_capsule::realize`, `sentence::sentences_at` | `M2/RESULTS.md` |
| Önem (M3) | `confirm` | `qp_capsule::rank` | `M3/RESULTS.md` |
| Tema (M4) | `kinddir` | `qp_capsule::theme` | `M4/RESULTS.md` |
| Sonuç (M5) | `barrier`; aşırılıkta "uzama sürdü" gerçekleşti sayılır | `qp_capsule::outcome` | `M5/RESULTS.md` |
| Güven yolu (M6) | mühür; yetkisiz okuyucu lisanslı hikâyeyi yalnız mühür iletileriyle görür (olgu ve kapsülün varlığı ayrıca gitmez) | `qp_capsule::trust::Muhur`, PROTOCOL §20 | `M6/RESULTS.md` |
| İtiraz ve geri çekme (M7) | kesin bar hijyeni + `dispute` | `qp-session` `dispute.rs`, PROTOCOL §19.9 | `M7/RESULTS.md` |
| Otoritede motor | bar kesinleşince `EngineConfig::r3()`; sonuç motor yazarlı değişim olarak günlükte; günlükten yeniden oynatma aynı hikâyeyi verir | `qp-session` `capsule.rs`, PROTOCOL §20 | `S/S-C.md` |
| Hassasiyet | §9.8; kitle kişiye özeldir, başkasına gitmez | `qp_capsule::audience`, `set_audience` | `S/sensitivity.md` |

Kanıtın açılması (vazgeçilmez, K34): yetkili okuyucu kanıt değerlerini görür; yetkisiz okuyucu
yorumu, karneyi ve "bu kanıt şu veri yetkisini gerektirir" (`requires`) ile neye dayandığını
(`rests_on`) alır. Lisanslı akışta bu bilgi mühür iletisinin ekleriyle verilir.
