# Kuzey Yıldızı

> **Grafik bir resim değil; insanın ve yapay zekânın aynı anda okuyup yazdığı canlı bir dokümandır.**
>
> **İnsan grafiğe bakmaz; grafik insana haber verir.**

Bu dosya projenin değişmeyen çekirdeğidir. Her tasarım kararı, her round hedefi ve her
alt görev buraya göre sınanır. Değişiklik yalnızca Arkin'in onayıyla yapılır ve
`DECISIONS.md` içine kaydedilir.

## Ne yapıyoruz

AI-ready bir grafik aracı değil, **AI-native bir görsel anlam sistemi**.

Bugünün grafik araçları insanın eli ve gözü için tasarlandı; AI onlara sonradan eklendi
("ben söyleyeyim, AI çizsin"). Biz yönü ters çeviriyoruz: sistemin varsayılan yazarı ajan,
insan ise okuyan, onaylayan ve düzelten taraf. Grafik ölmüyor; çalışma tezgâhı olmaktan
çıkıp **kanıt** oluyor.

## Kimin için

Tek motor, tek doküman, iki uç:

- **Alışkın kullanıcı** (bugünün profesyoneli, eli klasik grafiğe alışmış): bildiği grafiği
  görür, alışkanlığı bozulmaz. AI katmanları üstünde sessizce belirir.
- **Geleceğin yatırımcısı** (bugün 20 yaşında): liste, tablo, ham veri görmek istemez.
  Bilgi önüne kapsül olarak gelir. Mobil cihazı bir veri ekranı değil, ajanlarını
  izlediği ve yönettiği **kontrol kulesidir**.

İkisi ayrı ürün değildir. Profesyonel grafik sonuna kadar açılmış kapsüldür; kapsül
sonuna kadar katlanmış grafiktir.

## İlkeler

1. **Grafik = doküman.** Her şey (seri, görünüm, bulgu, çizim, niyet) tek bir şemada
   yaşar; kaydedilir, karşılaştırılır, taşınır.
2. **Tek komut yolu.** İnsan hareketi ile ajan çağrısı aynı komuttur.
3. **Sayıyı motor söyler, yorumu model yapar.** Tüm hesaplar LLM'siz ve deterministiktir.
   Model hiçbir zaman sayı uydurmaz; hesaplatır.
4. **Hesap, ajanın düşünme aracıdır.** Çekirdek, istek anında binlerce parametre
   varyasyonunu deneyebilecek kadar hızlıdır. Sunucu yaygın olanı yayınlar, çekirdek
   gerisini o an üretir.
5. **Kapsül en küçük birimdir.** Tek iddia, tek bakışlık görsel, canlı, açılabilir,
   taşınabilir, karneli.
6. **Her görsel öğe bir olguya bağlıdır.** Ekrandaki her işaret deterministik
   kaynağına kadar izlenebilir.
7. **Anlam zoom'u.** Cümle → kapsül → işaretli grafik → tam çalışma alanı arasında
   kesintisiz geçiş.
8. **Görsel gramer, sabit grafik tipleri değil.** Motor yapı taşı sunar; ajan bugün
   aklımıza gelmeyen görünümler kurar. Mum grafiği bu gramerin yalnızca bir cümlesidir.
9. **Ekransız çalışır.** Hiçbir insan bakmasa da sistem değer üretir.
10. **Kalıcı çekirdek, değiştirilebilir kenarlar.** Model, veri kaynağı, ekran, gömme
    protokolü ve hesap donanımı değişebilir; şema, gramer, hesap çekirdeği ve komut
    yolu kalır.

## AI-native testi

Her tasarım kararı bu beş soruya vurulur. Biri "hayır" ise AI-ready araca kayıyoruz.

| # | Soru |
|---|------|
| 1 | Ekransız çalışır mı? |
| 2 | Varsayılan yazar ajan mı? |
| 3 | Her görsel öğe bir olguya bağlı mı? |
| 4 | İnsan girdisi ajanın da okuduğu aynı dokümana mı yazılıyor? |
| 5 | Ürün API mi (arayüz yalnızca bir istemci mi)? |

## İnsanın görmek isteyeceği beş soru

Görsel dilimiz bu sorulara cevap verecek şekilde tasarlanır:

1. **Ne değişti?**
2. **Neden?**
3. **Ne kadar emin?**
4. **Daha önce oldu mu?**
5. **Beni ilgilendiriyor mu?**

## Ne değiliz

- TradingView kopyası değiliz. Manuel çizim aracı sayısında yarışmıyoruz.
- Sohbet penceresi eklenmiş grafik değiliz.
- Fiyat tahmin ürünü değiliz. Tahmin, takılabilir/eklenebilir bir sağlayıcıdır; merkez değildir.
- Yatırım tavsiyesi motoru değiliz. Her kapsül sayıya dayanır, kanıtına açılır ve karnesini taşır.

## Vazgeçilmezler

- Kapsül her zaman kanıtına açılır. Kullanıcının veri yetkisi yetmiyorsa kanıtın var
  olduğu, neye dayandığı ve hangi yetkiyi gerektirdiği söylenir.
- Karne her zaman görünürdür.
- Hesap her zaman deterministik ve yeniden üretilebilirdir.
