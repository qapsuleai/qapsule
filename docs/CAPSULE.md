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
