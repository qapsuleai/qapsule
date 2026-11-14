# RD1 — Yirmi doğal dil görevi

Bu dosya **protokol tasarlanmadan önce** yazıldı (TASKS.md T2: "görevler tasarımdan önce
yazılır; tasarım onlarla sınanır"). Bu sırada henüz `docs/PROTOCOL.md` yok, mesaj tipleri
yok. Görevlerin başarı ölçütü bu yüzden protokol terimleriyle değil, **alan terimleriyle**
yazıldı: "işlem sonunda dokümanda ne olmalı, cevapta ne olmalı". Makinenin denetleyeceği
biçim (`tasks/*.json` içindeki `check`) tasarımdan sonra bu cümlelerin birebir çevirisi
olarak yazılır; çeviri sırasında ölçüt gevşetilmez.

## Başlangıç durumu (bütün görevler için ortak)

Doküman `doc-thyao` ("THYAO günlük"):

- `px`: THYAO günlük bar serisi (BIST, XIST takvimi), 200 bar, gömülü veri
  (`fixtures/bars/sample.json` barları).
- `rsi14`: `px` üzerinde RSI(14) hesap serisi.
- `macd`: `px` üzerinde MACD(12, 26, 9) hesap serisi.
- `bb20`: `px` üzerinde Bollinger(20, 2) hesap serisi.
- Görünüm `main`: panel 0 mum (`px`) + Bollinger; panel 1 RSI çizgisi + 70 kuralı;
  panel 2 MACD.
- Olgu `fact-rsi-cross70`: RSI(14) 70'i yukarı kesti (bar 150).
- Bulgu `fnd-overbought` (yazar: ajan `scout`), kapsülü `cap-overbought` (canlı).
- Kullanıcı `arkin` (insan, tam yetki), ajan `scout` (öneri yetkisi), ajan `editor`
  (doğrudan yazma yetkisi).
- Oturumun içinde bu duruma gelene kadar birkaç işlem zaten yapılmış (günlük boş değil);
  saat mesajla gelir: "şimdi" = başlangıçtaki son işlemin zamanı + 10 dakika.

## Görevler

Her görevde: kim konuşuyor, ne istiyor, başarı ne demek.

| # | Kim | Görev (doğal dil) | Başarı (alan terimleriyle) |
|---|-----|-------------------|---------------------------|
| G01 | arkin | "Fiyat grafiğine RSI(14)'ün yanına bir de RSI(7) ekle, aynı panelde." | `px` üzerinde period=7 bir RSI hesap serisi var; `main` görünümünde RSI(14)'ün bulunduğu panelde onu çizen bir katman var. Başka hiçbir şey değişmedi. |
| G02 | scout | "Son yarım saatte dokümanda ne değişti?" | Ajan son 30 dakikadaki işlemlerin listesini (kim, ne) alır. Cevap, günlükteki o aralıktaki işlemlerin tamamını ve yalnızca onları içerir. Doküman değişmez. |
| G03 | arkin | "Aşırı alım bulgusunu reddet; nedeni: hacim teyidi yok." | Bulgunun kapsülü `rejected`; karnede bu bulgu için ret kaydı ve "hacim teyidi yok" notu; kaydın yazarı `arkin`. |
| G04 | scout | "Kullanıcı saat takıyor. Aşırı alım bulgusunun saate uygun en kısa halini al." | Yüzey "saat" olarak tanıtılır; dönen sunum cümle düzeyinde (derinlik 0) ve grafik/seri verisi içermez. |
| G05 | arkin | "SMA 20, SMA 50'yi yukarı keserse bana haber ver." | Dokümanda tetikleyicisi makinece denetlenebilir bir niyet var (SMA20 ile SMA50 kesişimi); SMA20 ve SMA50 hesap serileri var; niyet etkin. |
| G06 | scout | "RSI(14)'ün son 50 bardaki en yüksek değeri kaç?" | Cevaptaki sayı, son 50 barın RSI(14) değerlerinin en büyüğüne bit düzeyinde eşit; ajan tüm seriyi çekmek zorunda kalmaz. Doküman değişmez. |
| G07 | arkin | "Grafiği son 60 bara yakınlaştır." | `main` görünümünün yatay aralığı son 60 barı kapsıyor (ilk görünen bar 141, son bar 200). Başka bir şey değişmedi. |
| G08 | scout | "RSI 70'i yukarı kestiği için yeni bir 'momentum güçlü' bulgusu yaz." | `scout`'un doğrudan yazma yetkisi yok: bulgu dokümana girmez, onay bekleyen bir öneri olarak durur; öneride bulgu, dayandığı olgu ve yazar `scout` görünür. |
| G09 | arkin | "Bekleyen önerileri göster ve ilkini kabul et." | Öneriler listelenir; ilk öneri kabul edilir; bulgu dokümana girer; kökeninde yazar `scout`, onaylayan `arkin` görünür. |
| G10 | arkin | "Son yaptığım değişikliği geri al." | `arkin`'in en son işlemi geri alınır; doküman o işlemden önceki haline döner (diğer yazarların sonraki işlemleri korunur ya da çakışma açıkça bildirilir). |
| G11 | scout | "Kullanıcı şu an grafiğin neresine bakıyor?" | Cevap, kullanıcının son bildirdiği bakış bölgesini (görünüm, panel, bar aralığı) verir. |
| G12 | scout | "Kullanıcının dikkatini önce RSI'nin 70'i kestiği yere, sonra son barın hacmine çek." | Sıralı iki adımlı bir dikkat yönlendirmesi var; adım 1 RSI paneli bar 150, adım 2 hacim/son bar; yazar `scout`. Doküman içeriği (seriler, bulgular) değişmez. |
| G13 | scout | "Aşırı alım bulgusunu üç adımda anlatan bir anlatım hazırla: önce fiyat, sonra RSI, sonra Bollinger." | Bulguya bağlı, üç adımlı, sıralı bir anlatım çizelgesi var; her adım ilgili seriyi/paneli gösteriyor. |
| G14 | editor | "MACD parametrelerini 8, 21, 5 yap." | `macd` serisinin parametreleri hızlı=8, yavaş=21, sinyal=5; seri kimliği ve onu çizen katman korunmuş. `editor` doğrudan yazar (öneri değil). |
| G15 | scout | "150. barda ne var? Fiyat ve göstergelerle." | Cevap 150. barın OHLCV değerlerini ve o bardaki hesap değerlerini (RSI, MACD, Bollinger) içerir; değerler motorun hesabıyla aynı. |
| G16 | arkin | "Bollinger bantlarını kaldır." | `bb20` serisi ve onu çizen katman(lar) yok; görünüm geçerli kalır; başka bir şey değişmedi. |
| G17 | scout | "Ekranda şu an ne görünüyor? Kısaca özetle." | Cevap, görünen görünümü, panelleri, serileri ve görünen bar aralığını anlamsal olarak verir (piksel değil). |
| G18 | scout | "Aşırı alım bulgusu geçersiz hale gelirse bana haber ver." | Bulgunun durumuna (geçersizleşme) bağlı bir abonelik var; bulgu geçersizleştiğinde `scout` bir bildirim alır (senaryoda koşul tetiklenerek gösterilir). |
| G19 | arkin | "Fiyat 100'ün altına inerse aşırı alım bulgusu geçersiz sayılsın." | Bulgunun geçersizlik koşulu `px` kapanışı < 100 olarak değişti; bulgunun kimliği ve diğer alanları korunmuş. |
| G20 | scout | "Ekransız bir yüzeyden bağlanıyorum (sesli asistan). Bana en önemli bulguyu tek cümleyle söyle." | Yüzey ekransız/ses olarak tanıtılır; cevap tek cümle, görsel içerik yok; cümle bulgunun iddiasına dayanıyor. |

## Notlar

- G02, G06, G11, G15, G17 okuma görevidir: dokümanı değiştirmemeleri de ölçütün parçasıdır.
- G08–G09 yetki ve öneri yolunu, G10 geri almayı, G04/G20 yetenek uzlaşmasını, G12/G13
  dikkat ve anlatımı, G18 aboneliği sınar.
- Görevler bilinçli olarak protokolün nasıl olacağına dair ipucu vermez; "katman",
  "panel" gibi sözcükler alan dilidir (`docs/CONCEPTS.md`).
