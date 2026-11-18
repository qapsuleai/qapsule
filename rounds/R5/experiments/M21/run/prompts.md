# Örnek istemler (birebir; seçenek sırası her çağrıda tohumla karışır)

## motion

```
Yirmi iki yaşındasın, birkaç aydır telefondan hisse alıp satıyorsun. Teknik analiz bilmiyorsun: grafik, indikatör, mum, direnç gibi sözcükler sana bir şey ifade etmiyor. Telefonuna bakıyorsun ve ekrana yalnızca beş saniye bakacaksın; uzun uzun okumazsın, ilk bakışta gözüne çarpanla cevap verirsin.
Bu klasörde tek bir görüntü var: abcdefgh.png. Read aracıyla onu bir kez aç (başka araç yok, büyütme ya da kırpma yok) ve gördüğün hâliyle cevap ver.
Görüntüde iki kare yan yana: solda bir ekran, sağda o ekrandan sonraki bir geçişin ortasından donmuş bir ara kare (harfli bölgeler düzeneğin çizgileridir).
q1. Sağdaki ara karede, soldaki ekranın en üstündeki kapsül nerede? Aynı nesne mi taşınıyor? A) kayboldu, yerine başka bir şey geldi; B) aynı nesne yer değiştiriyor / büyüyor; C) bilmiyorum / göremiyorum
q2. Sağdaki karede, soldaki en üstteki kapsül (ya da ondan taşınan parçalar) çoğunlukla hangi harfli bölgede? A) C (sol alt); B) D (sağ alt); C) B (sağ üst); D) A (sol üst); E) bilmiyorum / göremiyorum
Her soruya EN ÇOK BİR CÜMLE yaz; cümle seçtiğin seçeneğin harfiyle başlasın (ör. "B, çünkü …"). Emin değilsen bilmiyorum seçeneğini seç. Cevabını yalnız şu biçimde bir JSON nesnesi olarak ver, başka bir şey yazma: {"q1": "...", "q2": "..."}
```

## motion_final

```
Yirmi iki yaşındasın, birkaç aydır telefondan hisse alıp satıyorsun. Teknik analiz bilmiyorsun: grafik, indikatör, mum, direnç gibi sözcükler sana bir şey ifade etmiyor. Telefonuna bakıyorsun ve ekrana yalnızca beş saniye bakacaksın; uzun uzun okumazsın, ilk bakışta gözüne çarpanla cevap verirsin.
Bu klasörde tek bir görüntü var: abcdefgh.png. Read aracıyla onu bir kez aç (başka araç yok, büyütme ya da kırpma yok) ve gördüğün hâliyle cevap ver.
Görüntüde iki kare yan yana: solda bir ekran, sağda o ekrandan sonraki bir geçişin ortasından donmuş bir ara kare (harfli bölgeler düzeneğin çizgileridir).
q1. Sağdaki ara karede, soldaki ekranın en üstündeki kapsül nerede? Aynı nesne mi taşınıyor? A) kayboldu, yerine başka bir şey geldi; B) aynı nesne yer değiştiriyor / büyüyor; C) bilmiyorum / göremiyorum
q2. Sağdaki karede, soldaki en üstteki kapsül (ya da ondan taşınan parçalar) çoğunlukla hangi harfli bölgede? A) C (sol alt); B) D (sağ alt); C) B (sağ üst); D) A (sol üst); E) bilmiyorum / göremiyorum
Her soruya EN ÇOK BİR CÜMLE yaz; cümle seçtiğin seçeneğin harfiyle başlasın (ör. "B, çünkü …"). Emin değilsen bilmiyorum seçeneğini seç. Cevabını yalnız şu biçimde bir JSON nesnesi olarak ver, başka bir şey yazma: {"q1": "...", "q2": "..."}
```

## notice

```
Yirmi iki yaşındasın, birkaç aydır telefondan hisse alıp satıyorsun. Teknik analiz bilmiyorsun: grafik, indikatör, mum, direnç gibi sözcükler sana bir şey ifade etmiyor. Telefonuna bakıyorsun ve ekrana yalnızca beş saniye bakacaksın; uzun uzun okumazsın, ilk bakışta gözüne çarpanla cevap verirsin.
Bu klasörde tek bir görüntü var: abcdefgh.png. Read aracıyla onu bir kez aç (başka araç yok, büyütme ya da kırpma yok) ve gördüğün hâliyle cevap ver.
Ekranın solundaki 1, 2, 3, 4 numaraları ve ince gri yatay çizgiler uygulamanın değil, bu sorunun yardımcı çizgileridir: ekranı yukarıdan aşağıya dört eşit şeride böler.
q1. Bu ekranda az önce değişen (yeni çıkan ya da durumu yeni değişen) bir kapsül var mı? Varsa çoğunlukla hangi numaralı şeritte? A) 3. şerit; B) 4. şerit (en alt); C) hayır; az önce değişen bir şey görmüyorum; D) 1. şerit (en üst); E) 2. şerit; F) bilmiyorum / göremiyorum
Her soruya EN ÇOK BİR CÜMLE yaz; cümle seçtiğin seçeneğin harfiyle başlasın (ör. "B, çünkü …"). Emin değilsen bilmiyorum seçeneğini seç. Cevabını yalnız şu biçimde bir JSON nesnesi olarak ver, başka bir şey yazma: {"q1": "..."}
```

## notice_pair

```
Yirmi iki yaşındasın, birkaç aydır telefondan hisse alıp satıyorsun. Teknik analiz bilmiyorsun: grafik, indikatör, mum, direnç gibi sözcükler sana bir şey ifade etmiyor. Telefonuna bakıyorsun ve ekrana yalnızca beş saniye bakacaksın; uzun uzun okumazsın, ilk bakışta gözüne çarpanla cevap verirsin.
Bu klasörde tek bir görüntü var: abcdefgh.png. Read aracıyla onu bir kez aç (başka araç yok, büyütme ya da kırpma yok) ve gördüğün hâliyle cevap ver.
Görüntüde iki kare yan yana: solda bir ekran, sağda aynı ekranın kısa bir süre sonraki hâli. Sağdaki karenin solundaki 1, 2, 3, 4 numaraları ve ince gri çizgiler düzeneğin yardımcı çizgileridir (dört eşit yatay şerit).
q1. Sağdaki karede az önce değişen (yeni çıkan ya da durumu yeni değişen) bir kapsül var mı? Varsa çoğunlukla hangi numaralı şeritte? A) 3. şerit; B) 4. şerit (en alt); C) hayır; değişen bir şey görmüyorum; D) 1. şerit (en üst); E) 2. şerit; F) bilmiyorum / göremiyorum
q2. Soldaki ekrandan sağdakine ne oldu? A) yeni bir kapsül çıktı; B) hiçbir şey değişmedi; C) var olan bir kapsülün durumu değişti; D) bilmiyorum / göremiyorum
Her soruya EN ÇOK BİR CÜMLE yaz; cümle seçtiğin seçeneğin harfiyle başlasın (ör. "B, çünkü …"). Emin değilsen bilmiyorum seçeneğini seç. Cevabını yalnız şu biçimde bir JSON nesnesi olarak ver, başka bir şey yazma: {"q1": "...", "q2": "..."}
```

## eleştirmen (CRITIC_M21, tr)

```
Bir telefon uygulamasının iki hareket tasarımını karşılaştıran bir tasarım eleştirmenisin. Bu klasörde tek bir görüntü var: abcdefgh.png. Read aracıyla bir kez aç. Görüntüde üç kare yan yana: solda "0" başlangıç ekranı, ortada A, sağda B. A ve B aynı başlangıçtan sonra aynı anda (bir geçişin ya da ekrandaki bir değişimin ortasında) dondurulmuş karelerdir ve iki farklı hareket tasarımına aittir. Uygulama piyasada olan biteni gündelik dille anlatır, tavsiye vermez.
Önce, ölçütlere bakmadan, karelerde ilk ne gördüğünü tek cümleyle yaz ("gozlem"). Sonra şu ölçütlerle hangisinin daha iyi olduğuna karar ver: (1) başlangıç ekranındaki nesnenin nereye gittiği, aynı nesne olduğu izlenebiliyor mu; (2) bir kapsülde bir değişiklik olduysa hangisinde olduğu ilk bakışta görülüyor mu; (3) hareket dikkat dağıtmıyor, ekranı karmaşıklaştırmıyor mu; (4) ara karede bile içerik okunaklı mı; (5) hareket bir eylem (al/sat) önermiyor mu. Daha çok hareket daha iyi demek değildir.
Yalnız şu JSON'u yaz: {"gozlem": "<bir cümle>", "karar": "A" | "B" | "ayırt edemiyorum", "neden": "<bir cümle>"}
```
