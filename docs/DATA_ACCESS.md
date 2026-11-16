# Gerçek Veriye Erişim (geliştirme ve ölçüm için)

Ölçümlerin bir kısmı (karne, önem sıralaması, sonuç tanımı) sentetik veride anlamlı
sonuç vermez. Bu dosya, gerçek geçmiş verinin Matriks servisinden nasıl çekileceğini ve
nasıl saklanacağını kaydeder.

## Kurallar (K23)

- Çekilen veri yalnızca `fixtures/local/` altına yazılır. Bu klasör `.gitignore` ile
  dışlanmıştır; içeriği, türevleri ve ondan üretilen görseller commit edilmez.
- Raporlara yalnızca toplu istatistik girer; bar değerleri girmez.
- Erişim belirteci depo kökündeki `.env.local` dosyasında `MATRIKS_TOKEN` olarak durur.
  Belirteci Arkin girer. Ajan dosyayı açmaz, belirteci yazdırmaz, komut satırına koymaz;
  betik onu ortamdan okur.
- İstekler yalnızca `https://api.matriksdata.com` adresine yapılır (test ortamı
  `apitest.matriksdata.com` kullanılmaz).
- Servisi yormamak için istekler sıralı ve aralıklı yapılır; aynı veri ikinci kez
  çekilmez (yerelde varsa atlanır).

## Bar uç noktaları

Kaynak: https://analistdocs.matriksdata.com/tarihsel-tick-veriler/bar ve
`docs/MATRIKS_PROTOCOL.md`.

| Uç | Periyotlar |
|----|------------|
| `GET /dumrul/v1/tick/bar.gz` | `1min`, `5min`, `1hour`, `1day` |
| `GET /dumrul/v1/tick/derived-bar` | `10min`, `15min`, `30min`, `120min`, `240min`, `weekly`, `monthly`, `yearly` |

- Başlık: `Authorization: jwt <belirteç>` (küçük harf `jwt`; eski prototipten, belgede yazmıyor).
- Parametreler: `symbol`, `start` ve `end` (`YYYY-AA-GG`), `period`. Alternatif: `end` ya da
  `timestamp` ile `count` (1 dakikalıkta en çok 1440, diğerlerinde 1000); ya da
  `startTimestamp` + `endTimestamp`.
- `divide=false`: düzeltilmemiş fiyat ve hacim döndürür. Varsayılan düzeltilmiş veridir.
  Hangisinin çekildiği dosya adında ve üst bilgide belirtilir (açık karar A8 ile ilgili).
- Yanıt: JSON dizi. Alanlar: `date`, `time` (epoch milisaniye; R3'te doğrulandı),
  `open`, `high`, `low`, `close`, `volume` (işlem tutarı, TL), `totalQuantity` (adet),
  `weightedAverage`.
- Belgede yazmayanlar (hız sınırı, en geniş tarih aralığı) R3'te ölçüldü: aşağıda
  "R3 doğrulamaları".

## Çekim

`scripts/fetch-bars.sh` (R3). Depo kökünden çalıştırılır:

```
scripts/fetch-bars.sh --symbols fixtures/local/symbols.txt --period 1day --start 2018-01-01 \
    [--end YYYY-AA-GG] [--unadjusted] [--out DİZİN] [--keep-raw] [--pause SN]
```

- `fixtures/local/symbols.txt`: satır başına bir sembol (`#` ile başlayan satır yorum).
  Arkin belirler.
- Belirteç: ortamdaki `MATRIKS_TOKEN`; yoksa ve `./.env.local` varsa betik onu kendisi
  yükler. Yine yoksa hangi değişkenin gerektiğini yazar ve 3 ile çıkar. Belirteç komut
  satırına girmez: `Authorization` başlığı curl'e standart girdiden yapılandırma olarak
  verilir; betik `set -x` kullanmaz.
- Çıktı: `fixtures/local/bars/<sembol>.<periyot>.<adj|unadj>.json`, şemadaki `Series`
  biçiminde (satır içi barlar, `instrument` = `{symbol, exchange: BIST, calendar: XIST}`);
  yanında `<…>.meta.json`: çekim tarihi, istek parametreleri, istek başına satır sayısı,
  birleştirilen kapanış seansı barı sayısı. Belirteç yazılmaz.
- Aralık parçalanır (günlük 3 yıl, saatlik 90 gün, 1 dakikalık 1 gün; yanıtlar küçük
  kalsın diye; sunucu `start`/`end` ile sınır koymuyor, aşağıya bkz.). İstekler sıralı,
  arada 1 sn.
- Yeniden deneme: HTTP 429/5xx ve **eksik gelen 200 yanıtları** (beklenen satırın %85'inden
  az; beklenen = bugünden önceki hafta içi gün × gün başına bar) 5, 10, 15 sn arayla en
  çok 3 kez yeniden istenir. Hâlâ eksikse veri tutulur ve üst bilgide `short: true` olur
  (başlangıçtan sonra listelenen sembol meşru olarak eksiktir). Hata veren sembol atlanır,
  diğerleriyle devam edilir, sonda özet yazılır.
- Dosya zaten varsa sembol atlanır (yeniden çekmek için dosya silinir).
- `--keep-raw`: ham yanıtlar ve yanıt başlıkları `<out>/../raw/` altında tutulur
  (doğrulama için).

### Okuma

`qp_source::load_local_bars(dizin, periyot)` düzeltilmiş dosyaları, `load_local_bars_with(…,
false)` düzeltilmemişleri okur: `(sembol, barlar)` listesi, sembole göre sıralı. Dizin ya da
dosya yoksa boş liste döner; okunamayan ya da `validate_bars`'tan geçmeyen dosya atlanır;
hiçbir durumda panik yok. `qp_source::local_bars_dir()` varsayılan dizindir
(`fixtures/local/bars`); git worktree içinde veri ana çalışma ağacındadır, o zaman
`QAPSULE_LOCAL_BARS` ortam değişkeni ile gösterilir.

## R3 doğrulamaları

İlk çekim 2026-11-15'te küçük isteklerle (tek sembol, birkaç gün) yapıldı. Değer değil,
yalnız biçim ve toplu istatistik.

**Zaman.**

- `time` epoch **milisaniye**dir (UTC).
- Günlük barda `time` işlem gününün **İstanbul yerel gece yarısıdır** (ör. 3 Mart günü
  için 2 Mart 21:00 UTC). `date` aynı günün yerel tarihidir; ikisi tutarlı.
- Saatlik barda `time` barın **başlangıcıdır**: 10:00 … 18:00 yerel (07:00 … 15:00 UTC),
  günde 9 bar. Kanıt: 1 dakikalık barlar saat başlangıcına göre toplanınca 9 saatlik barın
  9'unun hacmi tutuyor, sona göre toplanınca hiçbiri tutmuyor.
- Açılış seansı ilk bara (10:00) dahil; ilk 1 dakikalık bar 10:00. 18:00 saatlik barı ve
  18:04 dakikalık barı **kapanış seansıdır** (tek fiyat: açılış = yüksek = düşük =
  kapanış). Günlük kapanış bu barın kapanışıdır; günlük açılış ilk saatlik barın açılışı,
  günlük yüksek/düşük saatlik barların uç değerleri, günlük hacim saatlik (ve dakikalık)
  hacimlerin toplamı (oran 1,0000).
- Denenen günde 1 dakikalık barlar 10:00–17:59 arası 480 bar + 18:04 kapanış seansı barı.

**Proje kuralına eşleme (K14; çekim betiği yazarken yapar, okuyucu dönüştürmez).**

- Günlük: `t` = yerel gün + 10:00 İstanbul = 07:00 UTC (XIST seans açılışı;
  `qp-calendar` `day_span(gün).open` ile aynı). İstanbul 2016'dan beri yıl boyu UTC+3.
- Gün içi: `t` = `time` (bar başlangıcı). 18:00 ve sonrasında başlayan bar (kapanış
  seansı) aynı günün bir önceki barına birleştirilir (yüksek/düşük genişler, kapanış
  kapanış seansınınki, hacim toplanır). Böylece her bar XIST seansının [10:00, 18:00)
  içinde kalır ve `bar_at(1h, t).open == t`. Günlük kapanış, son saatlik barın
  kapanışına eşit kalır. Yarım günlerde kapanış seansı zaten 12:00 saat barının içinde.
- Doğrulama: çekilen tüm günlük ve saatlik barlar için `bar_at(aralık, t).open == t`
  (0 hizasız bar) ve tüm dosyalar `validate_bars`'tan geçiyor.
- `v` = `volume` (TL işlem tutarı). Adet (`totalQuantity`) yazılmaz.

**Aralık ve sınır.**

- `start` + `end` ile istekte sunucu satır sınırı uygulamıyor: tek istekte 2002 günlük
  satır (8 yıl) ve 4485 saatlik satır (2 yıl) geldi. Belgedeki 1000/1440 sınırı `count`
  kipi içindir. Betik yine de parçalar.
- `bar.gz` adına rağmen gövde gzip dosyası değil; yanıt `Content-Encoding: gzip` ile
  sıkıştırılmış düz JSON (curl `--compressed` açar). Betik gövde gzip olarak gelirse de
  açar.

**Hız sınırı.**

- Yanıt başlıklarında hız sınırı bilgisi yok (`X-RateLimit-*`, `Retry-After` yok).
- 7 isteğin arasız art arda gönderimi ve 1 sn arayla 130'dan fazla istek: hiç 429 yok.
- **Eksik 200 yanıtı:** sunucu ara sıra 200 ile eksik ya da boş dizi döndürüyor (ilk
  tam çekimde 90 isteğin 5'i: 0, 33, 120, 299 satır; yeniden çekimde 3 tane daha). Hepsi
  ilk yeniden denemede tamam geldi. Betik bu yüzden satır sayısını denetler.
- Geçersiz sembolde sunucu 404 değil **503** döndürüyor
  (`{"error":{"code":"EAGAIN",…}}`); yeniden deneme düzeltmez.

**Düzeltilmiş ve düzeltilmemiş (`divide=false`).**

- Endekste (XU100) fark yok.
- Beş hissenin hepsinde fark var: 2219 günün 1916–2190'ında düzeltilmiş kapanış farklı;
  sembol başına 2–12 düzeltme adımı (oran sıçraması > %0,2); en eski günde birikimli
  düzeltme çarpanı sembole göre 0,07 ile 0,98 arası (bedelsiz bölünmeler küçük, yalnız
  temettü olanlar 1'e yakın).
- Düzeltme **geriye doğrudur**: son kurumsal işlemden sonra iki seri aynı, öncesi
  çarpanla ölçeklenir. Yani düzeltilmiş seri her yeni kurumsal işlemde geçmişiyle birlikte
  değişir; yerel dosya çekim tarihindeki anlık görüntüdür (üst bilgide `fetched`).
  Karne ve ileriye bakma testleri bu yüzden tek bir çekimle çalışmalıdır.
- `volume` (TL tutar) iki seride aynı; düzeltme yalnız fiyatlara uygulanıyor.

## Sonrası için (aynı belgelerin diğer bölümleri)

Sol menüde başka veri aileleri de var; R6 ve sonrasında değerlendirilecek:

| Bölüm | Qapsule'daki karşılığı |
|-------|------------------------|
| Tarihsel Tick: İndikatörler | Kendi hesap çekirdeğimizin Matriks değerleriyle karşılaştırılması (bağımsız kâhin) |
| Tarihsel Tick: Trade, En İyi Alış Satış, Mini Bar | Gün içi dedektörler, hacim ailesi |
| Takas Analizleri, Kurum Hisse Analizleri | Akış kanıt ailesi; sunucu olgusu (K33) |
| Türev Enstrüman Verileri | Türev kanıt ailesi |
| Haberler ve Ekonomik Takvim, Temel Analiz | Haber/temel kanıt ailesi; "neden" sorusu |
| MQTT, Discovery, Topach | Canlı veri bağlayıcısı (R6) |
| Meta Veriler | Enstrüman, fiyat adımı, takvim |
| Alarm Yapısı | Niyetler (R9) |
