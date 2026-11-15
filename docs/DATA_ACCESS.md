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
- Yanıt: JSON dizi. Alanlar: `date`, `time` (örneklerde 13 haneli; milisaniye olduğu
  varsayılır, ilk çekimde doğrulanır), `open`, `high`, `low`, `close`, `volume` (işlem
  tutarı), `totalQuantity` (adet), `weightedAverage`.
- Belgede yazmayanlar: hız sınırı, en geniş tarih aralığı. İlk çekimde küçük isteklerle
  ölçülür ve buraya yazılır.

## Çekim

`scripts/fetch-bars.sh` (R3'te yazılır):

```
scripts/fetch-bars.sh --symbols fixtures/local/symbols.txt --period 1day --start 2018-01-01
```

- `fixtures/local/symbols.txt`: satır başına bir sembol. Arkin belirler.
- Çıktı: `fixtures/local/bars/<sembol>.<periyot>.<düzeltme>.json`, şemadaki `Series`
  biçiminde; yanında çekim tarihi ve istek parametrelerini taşıyan küçük bir üst bilgi.
- Büyük aralıklar `count` sınırına göre parçalanır.
- Betik belirteç yoksa ne yapılacağını söyleyip durur; belirteci istemez, okumaz.

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
