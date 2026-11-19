# R6 — Canlı Veri

## Neden

Bugüne kadar Qapsule sentetik pazarda ve geçmiş barların yeniden oynatılmasında yaşadı.
Bu round onu gerçek, canlı BIST akışına bağlar: gerçek semboller, gerçek saat, gerçek
seans. Tasarım turu (RD2) ve profesyonel çalışma alanı (R7) bunun üstüne kurulacak.

RI1 keşfi yolu belirledi (`docs/matriks/`), Arkin kararı verdi (K60): **Qapsule kendi
kimliğiyle doğrudan Matriks MQTT'ye bağlanır.** Quantex'in iç NATS yayını, gösterge
servisleri ve anomali motoru bu round'da kullanılmaz.

## Bağlantı (K61, K62)

- Bağlantı bilgileri `.env.local`'dadır ve Arkin girer: `MATRIKS_TOKEN_SERVICE_URL`,
  `MATRIKS_API_USERNAME`, `MATRIKS_API_PASSWORD`, `MATRIKS_HARDWARE_ID`,
  `MATRIKS_SERVICE_TYPE_CODE`, `MATRIKS_MQTT_RT_HOST`, `MATRIKS_MQTT_DL_HOST`,
  `MATRIKS_MQTT_MODE`, `MATRIKS_MQTT_STATS_HOST`, `MATRIKS_MQTT_NEWS_HOST`
  (`docs/DATA_ACCESS.md`). Dosya açılmaz; kod değerleri ortamdan okur.
- Örnek uygulama: `/Users/arkns/LocalGit/quantex/services/quantex-stream/matriks-adapter`
  (salt okunur). Belirteç alma, uç nokta başına bağlantı, konu kuralları ve hangi istek
  için hangi proto orada kurulu. Bağlantı mantığı Qapsule tarafında **Rust'ta yeniden
  yazılır**.
- **Proto dosyaları olduğu gibi kopyalanır (K62).** Matriks protoları sabittir. Örnek
  bağlayıcının proto klasöründeki dosyaların **tamamı**, değiştirilmeden Qapsule deposuna
  alınır (yalnız bugün kullanılanlar değil). Elle düzenlenmez, yeniden yazılmaz, alan
  eklenip çıkarılmaz. Rust tipleri derleme sırasında bu dosyalardan üretilir. Yanında
  köken kaydı durur: kaynak yol, `quantex` commit'i, tarih, dosya başına özet.
- Bu round'da yalnız **piyasa uç noktası** kullanılır (fiyat kartı ve borsa saati /
  açıklık iletisi). İşlem, derinlik, haber, istatistik uç noktaları kapsam dışı.

## Çalışan sistemleri koruma kuralları (kesin)

Aynı kimlikle aynı uç noktaya ikinci bağlantı birincisini düşürür
(`docs/matriks/CONNECTIVITY.md` §2.4). Bu yüzden:

1. **Tek bağlantı.** Uç nokta başına süreç içinde tek bağlantı; ayrıca makinede bir kilit
   dosyası (`fixtures/local/` altında) aynı anda iki Qapsule sürecinin bağlanmasını engeller.
2. **Canlı bağlantıyı yalnız orkestratör açar, sırayla.** Alt ajanlar canlı bağlantı
   açmaz; kayıtlı iletiler üzerinde çalışır. Paralel iki canlı deneme birbirini düşürür.
3. **"Çoklu bağlantı" bildirimi = dur.** Bağlantı kapanır, kendiliğinden yeniden denenmez,
   durum rapora yazılır ve Arkin'e bildirilir. Bu bildirim bizim yanlış bir şey yaptığımızı
   gösterir (Arkin).
4. **Kimlik Qapsule'e özgüdür.** `MATRIKS_HARDWARE_ID` boşsa `qapsule-` önekiyle makine
   adından türetilir; rastgele değer üretilmez (her koşuda yeni kimlik istemiyoruz).
   `quantex` deposundaki hiçbir kimlik, belirteç ya da ortam dosyası kullanılmaz, okunmaz.
5. **Önce gecikmeli kip.** İlk bağlantı ve çözücü denemeleri `MATRIKS_MQTT_MODE=delayed`
   ile; anlık kipe yalnız gecikmeli kipte her şey çalıştıktan sonra geçilir. Kipi kod
   değiştirmez; ajan hangi kipte olduğunu ortamdan okur ve anlık kip gerekiyorsa Arkin'den ister.
6. **Küçük başla.** İlk denemeler 3–5 sembolle; evren adım adım büyütülür (30 → 100 → daha
   fazlası) ve her adımda abonelik sınırı, hata ve yük ölçülür. Bir adım hata verirse geri dönülür.
7. **Yeniden bağlanma ölçülüdür:** üstel geri çekilme, üst sınır, art arda başarısızlıkta dur.
   Belirteç isteği sıklığı sınırlı; belirteç bellekte, diske yazılmaz.
8. `quantex` deposu salt okunur; orada hiçbir şey çalıştırılmaz. NATS'a ve Quantex
   servislerine bağlanılmaz.

## Gizlilik ve veri kuralları

- Belirteç, parola, yetki başlığı ve uç nokta adresleri günlüğe, hata iletisine, teste,
  belgeye ve depoya yazılmaz (günlükte maskelenir; bunun testi vardır).
- Gerçek piyasa verisi ve ham ileti kayıtları yalnız `fixtures/local/` altındadır; depoya
  girmez; ondan üretilen görsel ve türev de girmez. Rapora yalnız toplu istatistik.
- Depodaki testler ve CI ağa çıkmaz: elde kurulmuş (gerçek veri içermeyen) proto
  iletileriyle çalışır.
- Canlı gösterim yalnız bu makinede ya da Tailscale ağında açılır; `--lan` ile gerçek veri
  yerel ağa açılmaz (betik bunu reddeder ya da açıkça uyarır).

## Kapsam

1. **Matriks çözücü** (`qp-source`): proto → alan düzeyinde veri olayı (K23). **Alan
   varlığı korunur:** gelmeyen alan "değişmedi" demektir, 0 değildir (RI1'in en önemli
   uyarısı; `docs/matriks/MESSAGES.md` §3.2). İlk durum ve güncelleme ayrımı.
2. **Borsa saati ve açıklık:** saat ve açıklık iletisi oturumun saat girdisi olur (K44:
   saat barındırıcıdan); seans evresi (`stockStatus`) sözlüğe "seans evresi" olarak
   eklenir ve kapsül diline girer ("seans kapalı", "kapanış seansında").
3. **Canlı bar üretici:** fiyat kartından borsa takvimine hizalı bar (K14), bütün
   periyotlarda; kapanış seansı; canlı barın geçici, kesinleşen barın kesin olması (RP2).
4. **Geçmişle ek yeri:** geçmiş REST'ten (`docs/DATA_ACCESS.md`), canlı MQTT'den; aynı
   barın iki kaynaktan eşitliği ölçülür; hacim birimi (adet / TL) ve düzeltilmiş /
   düzeltilmemiş fiyat seçimi açıkça yapılır.
5. **Bağlantı yaşam döngüsü:** belirteç alma ve yenileme; kopma → veri "bayat" (`stale`)
   işaretlenir → yeniden bağlanınca REST'ten eksik aralık yeniden çözülür (PROTOCOL §18.4);
   yerel sıra numarası (Matriks iletisinde sıra yok).
6. **Ham ileti kaydı ve oynatma:** bağlayıcı aldığı ham iletileri (bayt + alış zamanı)
   isteğe bağlı olarak `fixtures/local/` altına yazar; `qp replay` aynı çözücüyle bu kaydı
   oynatır; aynı kayıt iki kez → bayt eşit çıktı. Alt ajanlar ve testler bununla çalışır.
7. **Canlı gösterim:** `scripts/demo.sh --live`: sunucu (otorite) evrendeki semboller
   için canlı veriyi alır, dedektörleri koşar, kapsülleri üretir; cihaz görüntüleyicidir
   (K48) ve yalnız açık enstrümanın verisini ister. Gerçek sembol adlarıyla.
8. **Ölçek ölçümü:** 30 / 100 / (mümkünse) bütün Pay Piyasası sembolüyle sunucu yükü,
   bellek, ileti hızı, tikten ekrana gecikme; abonelik sınırı gözlemi.
9. **RI1'in açık sorularının canlıda cevaplanması:** zaman alanlarının birimi ve kökeni,
   hacim alanları, ilk durumun yeniden abonelikte gelip gelmediği, belirteç süresi dolunca
   açık bağlantının davranışı, konu adı farkları. Bulgular `docs/matriks/` belgelerine işlenir.
10. **İç içe gösterge adresi (K63):** `SEMBOL:PERİYOT:GÖSTERGE[İÇ:param]:param` biçimi
    okunur ve Qapsule hesap zincirine çevrilir (ör. `ISCTR:5m:EMA[RSI:18]:20`); değer
    Qapsule çekirdeğinde hesaplanır.
11. **Küçük işler:** gösterim varsayılanları `korda` ve `live_new` (K57); RI1'in sözlük
    adayları ("seans evresi", "Matriks kimliği") `docs/CONCEPTS.md`'ye.

## Kapsam dışı

- Quantex NATS yayını, gösterge servisleri, anomali motoru (bağlam olarak bile).
- İşlem tiki, derinlik, kurumsal akış (AKDE), takas, haber: lisans cevabı ve kanıt ailesi
  kararından sonra. Akış ailesi dedektörü canlıda veri bulamaz; bunu dürüstçe söyler.
- Kullanıcı başına Matriks yetkisi (kişinin kendi belirteci). Bu round tek sistem kimliğiyle,
  yalnız geliştirme ve gösterim içindir; son kullanıcıya dağıtım değildir.
- Tasarım değişikliği (RD2). Ajan yüzeyi (R5B). Sunucuya dağıtım.

## Ölçümle karar verilecek konular (K39)

| # | Konu | Alternatif | Ölçüm |
|---|------|-----------|-------|
| M22 | Canlı barın kurulması | (a) fiyat kartının son fiyat ve birikimli hacim alanlarından; (b) varsa hazır bar alanlarından; (c) karma | Aynı gün için REST barıyla fark (açılış, yüksek, düşük, kapanış, hacim): eşit bar oranı, en büyük sapma; kapanış seansı |
| M23 | Kopma sonrası onarım | (a) kopan aralığı REST'ten yeniden çöz; (b) yeniden abonelikte gelen ilk durumla yetin; (c) ikisi | Bilerek kesilen bağlantıda (yalnız kendi bağlantımız) onarım sonrası barların REST'le eşitliği; onarım süresi; yanlış ya da erken kapsül (0 olmalı) |
| M24 | Evren aboneliği | sembol başına konu; toplu ya da joker abonelik (destekleniyorsa) | Abonelik süresi, hata, ileti hızı, sunucu yükü; adım adım 30 / 100 / tümü |

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | Koruma kuralları uygulanmış ve test edilmiş: süreç içi ve makine kilidi; "çoklu bağlantı" bildiriminde dur ve yeniden deneme (benzetilmiş bildirimle test); kimlik türetme `qapsule-` önekli ve belirleyici; günlük maskeleme testi. Round boyunca "çoklu bağlantı" bildirimi alındıysa her biri raporda (ne zaman, neden). |
| 2 | Proto dosyalarının tamamı değiştirilmeden kopyalandı; köken kaydı var; bir test kopyanın özetlerinin kayıtla aynı olduğunu denetliyor; Rust tipleri bu dosyalardan üretiliyor (elle yazılmış mesaj tipi yok). Çözücü: kullanılan her mesaj için alan varlığı korunuyor (gelmeyen alan değeri değiştirmiyor; test); ilk durum ve güncelleme doğru birleşiyor; elde kurulmuş iletilerle birim testleri CI'da ağsız geçiyor. |
| 3 | Borsa saati ve açıklık oturumun saat girdisi; seans evresi kapsül dilinde; borsa kapalıyken gösterim bunu söylüyor ve bayat veriyi canlı gibi göstermiyor. |
| 4 | Canlı bar K14'e uygun; M22 ölçülmüş: en az bir tam seansta, en az 5 sembolde, günlük ve gün içi periyotlarda REST barlarıyla karşılaştırma tablosu; farklar açıklanmış. |
| 5 | Ek yeri: geçmiş + canlı birleşik seride sıçrama yok; hacim birimi ve düzeltme seçimi belgeli; göstergeler ek yerinde sürekli (test). |
| 6 | Kopma: M23 ölçülmüş; kopmada veri bayat işaretleniyor, kapsüller buna göre davranıyor (yanlış ya da erken gösterim 0); yeniden bağlanma ölçülü; belirteç yenileme çalışıyor ve tek bağlantı korunuyor. |
| 7 | Ham ileti kaydı ve oynatma: aynı kayıt iki kez bayt eşit çıktı; canlı yol ile oynatma aynı çözücüyü kullanıyor; kayıtlar yalnız `fixtures/local/` altında. |
| 8 | `scripts/demo.sh --live`: borsa açıkken gerçek sembollerle kapsül akışı; en az bir kapsülün canlı doğduğu ya da durum değiştirdiği gözlenmiş ve kaydı (ileti dökümü, veri içermeyen özet) raporda; cihaz görüntüleyici (akışta bar aboneliği yok; açıkken yalnız o enstrüman). |
| 9 | Ölçek: M24 ve yük ölçümü tablosu (sembol sayısı × sunucu işlemci, bellek, ileti/sn, tikten ekrana gecikme p50/p95); gözlenen abonelik sınırı ya da "sınıra ulaşılmadı". |
| 10 | RI1 açık soruları: canlıda cevaplananlar `docs/matriks/` belgelerine işlendi (soru → cevap → nasıl gözlendi); cevaplanamayanlar listeli. |
| 11 | İç içe gösterge adresi okunuyor; en az üç örnek (`RSI:14`, `EMA[RSI:18]:20`, iki düzey iç içe) Qapsule zincirine çevriliyor ve sonuç doğrudan kurulan zincirle bit eşit. |
| 12 | Gösterim varsayılanları `korda` ve `live_new`; sözlük güncel. |
| 13 | Var olan altınlar, dökümler ve hesap özeti değişmedi; `scripts/verify.sh` yeşil ve ağsız; CI yeşil. Gizli bilgi taraması temiz; depoda gerçek veri, uç nokta adresi, belirteç yok. |
| 14 | `REPORT.md`, `AUDIT.md`; `docs/matriks/LIVE.md` (bağlayıcının nasıl çalıştığı, nasıl başlatılır, sorun giderme); `STATUS.md`, `ROADMAP.md`; önerilen kararlar ayrı bölümde; disk hijyeni (K56). Raporun sonunda Arkin için: canlı gösterimi nasıl açarım. |
