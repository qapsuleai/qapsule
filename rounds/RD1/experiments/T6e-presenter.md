# T6e — Deterministik sunucu (presenter) mi, dil modeli grameri mi yazsın?

Deney: RD1 / T6e. Sınanan çalışma kararı: K21-1 (`DECISIONS.md`) — "dil modeli çizim
döngüsünde yer almaz; ajan anlam düzeyinde konuşur, motordaki deterministik sunucu gramere
çevirir. Ajan grameri doğrudan da yazabilir."

Alternatif: her sunumda dil modeli tam `View` JSON'unu (grameri) kendisi yazar.

## 1. Hipotez (ölçümden önce yazıldı)

H1 — Gecikme. `show_finding` uçtan uca (oturum: ayrıştırma, yetki, açılım, uygulama,
günlük) release derlemede medyanı < 100 µs, p99 < 1 ms olur; sunucu fonksiyonunun kendisi
< 10 µs. Dil modeli ile grameri yazmak ise çağrı başına yüzlerce ms – birkaç saniye sürer;
yani telefonda "bir kare / 100 ms" bütçesini model yolu karşılayamaz, sunucu yolu 3–4 kertelik
payla karşılar.

H2 — Belirlenimcilik. Aynı doküman 1 000 çalıştırmada bayt bayt aynı görünümü verir
(tek farklı çıktı). Model yolunda N örnek içinde birden fazla farklı çıktı olur (sıcaklık
> 0 ise neredeyse kesin; yapı bile değişebilir: panel sayısı, renk, alan sırası, `title`).

H3 — Doğruluk. Sunucu her zaman şemaya uyar ve anlamca doğrudur (var olan seriler,
hesap kendi panelinde, olgu seviyesi kural olarak işaretli). Model çıktısı çoğunlukla
şemaya uyar ama anlam hataları (yanlış/uydurma seri kimliği, `domain` için bar indeksi
yerine zaman damgası karışıklığı, `encoding.y`'nin sayı olarak yazılması, olgu barının
işaretlenmemesi) örneklerin anlamlı bir kısmında görülür; küçük model büyükten daha sık
hata yapar.

H4 — Maliyet ve ölçek. 10k kullanıcı × 50 kapsül görüntüleme/gün × birkaç yüzey
≈ 10⁶ sunum/gün. Sunucu yolunda maliyet bir CPU çekirdeğinin saniyelerine denk gelir
(pratikte sıfır). Model yolunda sunum başına ~3–5k girdi + ~200–400 çıktı token'ı ile
günlük maliyet küçük modelde yüzlerce, büyük modelde binlerce dolar mertebesinde olur.

H5 — Denetim. Sunucu yolunda bir düzenleyici, günlükteki `show_finding` girdisini ve
dokümanı yeniden oynatarak görünümü bayt bayt yeniden üretebilir. Model yolunda
yeniden üretim yalnızca modelin çıktısı saklanırsa mümkündür; "neden böyle göründü"
sorusunun cevabı ise modelin içindedir ve tekrar sorulduğunda farklı olabilir.

Beklenen sonuç: K21-1 doğrulanır. Grameri ajanın doğrudan yazması, sunucunun bilmediği
yeni görünümler (ilke 8) için değerlidir; ama bu yazım bir kez yapılır, komut yolundan
geçer, dokümana kaydedilir ve sonraki her sunumda deterministik olarak yeniden kullanılır —
yani model yine çizim döngüsünde değildir.

Çürütme ölçütü: model yolu medyan gecikmede < 100 ms'ye inerse, N örneğin tamamında tek
bir yapısal çıktıya yakınsarsa ve anlam hatası yapmazsa, K21-1'in "model çizim döngüsünde
değil" kısmı gerekçesini kaybeder (yalnızca maliyet gerekçesi kalır).

## 2. Yöntem

Kod: `experiments/t6e-presenter/` (çalışma alanı üyesi değil; `qp-schema` ve `qp-session`'a
yol bağımlılığı). Ham çıktılar: `rounds/RD1/experiments/raw/t6e/`.

- **Belge:** `fixtures/protocol/docs/thyao.json` (200 günlük bar; `rsi14`, `macd`, `bb20`;
  bulgu `fnd-overbought`, olgu `fact-rsi-cross70` bar 150, `evidence.level = 70`).
- **Sunucu yolu** (`t6e-presenter bench`, release, `lto = "thin"`, Apple M4 Max, rustc 1.92.0):
  - `presenter_fn`: `qp_session::present::presenter_view` `pub(crate)` olduğu için birebir
    kopyası; kopyanın çıktısının oturumun ürettiği görünümle bayt bayt aynı olduğu her
    koşuda doğrulanır (`copy_equals_session_view: true`). 100 000 ölçüm, 1 000 ısınma.
  - `presenter_fn+serialize`: aynı, JSON baytlarına yazım dahil.
  - `show_finding` uçtan uca: `Session::handle_raw` ile ham komut metni → ayrıştırma, şema,
    yetki, açılım, uygulama, günlük, çıkış satırlarının JSON'a yazımı. Her ölçüm taze bir
    oturumda (oturum kurulumu zamanlayıcının dışında), 3 000 ölçüm.
  - `present` sorgusu (okuma yolu, dokümana yazmaz): masaüstü/`chart` (sunucu her seferinde
    çalışır, çünkü kapsülün görünümü yok) ve telefon/`capsule` (görünüm yok, 60 noktalı spark),
    20 000'er ölçüm.
  - Belirlenimcilik: 1 000 taze oturumda `show_finding`; çıkış satırlarının, son dokümanın ve
    görünümün baytlarının özeti (sabit anahtarlı SipHash) sayılır. Ayrıca 3 ayrı süreçte
    görünüm özeti karşılaştırılır. Tüm ölçüm 3 kez koşuldu (`bench-run{1,2,3}.json`).
- **Model yolu:** `raw/t6e/llm-prompt.md` — ajanın sahip olacağı bilgiyi içeren tek, kendi
  başına yeterli bir istem: bulgu, olgusu, seri kimlikleri ve çıktı adları, bar sayısı ve
  zaman damgaları, kısa gramer notları, `qp-schema`'dan üretilmiş `View` JSON Schema'sı,
  "yalnızca `View` JSON'unu yaz" talimatı (7 143 bayt). Sunucunun kuralı isteme **yazılmadı**
  (model görünümü kendi seçer). Örnekleri orkestratör topladı: `haiku` (Claude Haiku 5.5) ve
  `sonnet` (Claude Sonnet 5.5), varsayılan sıcaklık, 5'er örnek (`raw/t6e/llm-samples/`,
  yöntem ve sınırlar `README.md`'de).
  - Değerlendirme (`t6e-presenter eval`): biçim (yalnız JSON mu), `qp_schema::validate("View")`,
    tipli ayrıştırma, motorun kabulü (`put_view` komutu oturumdan geçer mi), anlam denetimleri
    (seriler var mı, `px` mumları panel 0'da mı, `rsi14` kendi panelinde mi, 70 seviyesi
    `rsi14` panelinde kural olarak var mı, olgu barı işaretli mi, `x.domain` ms cinsinden ve
    olgu zamanını içeriyor mu), kanonik (anahtarları sıralı) JSON ve yapısal imza üzerinden
    farklı çıktı sayısı. Sonuç: `raw/t6e/llm-samples/eval.json`.
  - Gecikme: örneklerin süreleri alt ajan duvar saatidir (ajan başlangıcı + bir `Read` turu
    dahil) → **üst sınır**. Token: alt ajan toplamı (~49–51 k) ajan sistem istemiyle şiştiği
    için kullanılmadı; girdi = 7 143 B / 3,5 ≈ 2 040 token, çıktı = bayt / 3,5 alındı.
  - Fiyat: <https://platform.claude.com/docs/en/about-claude/pricing>, 2026-11-14'te okundu:
    Haiku 5.5 $0,10 / $0,50 (MTok girdi/çıktı, ≤100 k istem), Sonnet 5.5 $2 / $10;
    Batch API %50 indirim (gerçek zamanlı sunumda kullanılamaz).

## 3. Sonuçlar

### 3.1 Sunucu: gecikme (ns; 3 koşu, aralık koşular arası)

| Ölçüm | n | medyan | p99 |
|-------|--:|-------:|----:|
| `presenter_fn` (yalnız fonksiyon) | 100 000 | 125 | 167–209 |
| `presenter_fn` + JSON yazımı | 100 000 | 459–500 | 542–584 |
| `show_finding` uçtan uca (oturum) | 3 000 | 6 583–6 917 | 11 833–13 250 |
| `present` sorgusu, masaüstü `chart` | 20 000 | 5 958–6 083 | 6 250–7 084 |
| `present` sorgusu, telefon `capsule` | 20 000 | 2 750–2 875 | 2 958–3 542 |

Telefondaki 100 ms bütçesinin %0,01'inden azı. (Ölçüm masaüstü sınıfı bir CPU'da; telefon
CPU'su 3–5 kat yavaş varsayılsa bile ~30 µs.)

### 3.2 Sunucu: belirlenimcilik ve boyut

| Ölçüt | Sonuç |
|-------|-------|
| 1 000 koşuda farklı çıkış satırı (ack + notify) | **1** |
| 1 000 koşuda farklı son doküman | **1** |
| 1 000 koşuda farklı görünüm (oturum / fonksiyon) | **1 / 1** |
| 3 ayrı süreçte görünüm özeti | aynı (`b6566f6d79eeff63`) |
| Görünüm boyutu | 294 B (sıkı), 523 B (girintili) |
| `show_finding` çıkışı (`ack` + değişim bildirimi) | 94 B |
| `present` cevabı: masaüstü / telefon | 7 318 B / 2 352 B (seri verisi dahil) |

Sunucunun görünümü (`raw/t6e/presenter-view.json`): panel 0 `px` mumları; panel 1 `rsi14`
çizgisi + `y = "70"` kuralı; `x.domain = [t(120), t(199)]`; iki `linear` y ölçeği.

### 3.3 Model yolu: doğruluk (5 + 5 örnek)

| Denetim | Haiku 5.5 | Sonnet 5.5 |
|---------|:---------:|:----------:|
| Yalnız JSON (çit/yazı yok) | 5/5 | 5/5 |
| JSON Schema geçerli (`qp-schema`) | 5/5 | 5/5 |
| Tipli ayrıştırma + `put_view` motorca kabul | 5/5 | 5/5 |
| Yalnız var olan seriler | 5/5 | 5/5 |
| `px` mumları panel 0'da | 5/5 | 5/5 |
| `rsi14` kendi panelinde | 5/5 | 5/5 |
| 70 seviyesi `rsi14` panelinde kural | 5/5 | 5/5 |
| Olgu barı (t = 1753920000000) işaretli | 3/5 | 5/5 |
| `x.domain` ms cinsinden, olguyu içeriyor (ya da otomatik) | 5/5 | 5/5 |

Hipotez H3'ün "anlam hataları" kısmı **çürüdü**: iki model de katı denetimlerin hepsini geçti.
Sert hatalar (uydurma seri, sayı tipli `encoding`, bar indeksi/zaman karışıklığı) hiç görülmedi.
Modeller sunucudan *daha zengin* görünüm kurdu: 8/10 olgu barına dikey kural koydu (sunucu
koymuyor), Sonnet nokta ve metin etiketi ekledi.

Ama katı denetimin yakalamadığı, ilkelerle çelişen öğeler var:

| Bulgu | Haiku | Sonnet | İlgili ilke |
|-------|:-----:|:------:|-------------|
| Hiçbir olguya bağlı olmayan `y = 30` kuralı (aşırı satım) | 2/5 | 5/5 | 6 (her öğe bir olguya bağlı) |
| Kanıt olmayan ek seri (`bb20` bantları) | 0/5 | 4/5 | 6, 5 (tek iddia) |
| Modelin yazdığı sayı/metin (nokta `y = "72.84"`, etiket "RSI 72.8 > 70", konum `"80"`) | 0/5 | 5/5 | 3 (sayıyı motor söyler) |
| Spesifikasyonda tanımı olmayan kanal (`x` sabiti, `text`, `open/high/low/close`) | 4/5 | 5/5 | — (gramer v0'da kanal sözlüğü yok) |
| `title` (model yazımı, örnekler arası farklı) | 5/5 | 5/5 | 3 |

Not: `qp-render` henüz `View`'i yorumlamıyor (R0: yalnız bar dosyası çiziyor) ve `encoding`
şemada serbest bir `string → string` eşlemi; bu yüzden "motor kabul etti" zayıf bir ölçüttür.
Modellerin icat ettiği kanalların (dikey kural için `x`, `text`) ne çizeceği bugün tanımsız.

### 3.4 Model yolu: belirlenimcilik

| | Haiku 5.5 | Sonnet 5.5 | Sunucu |
|-|:---------:|:----------:|:------:|
| Farklı kanonik çıktı | **5 / 5** | **5 / 5** | 1 / 1 000 |
| Farklı yapı (işaret, seri, panel, kanal kümesi, ölçek) | 4 / 5 | 5 / 5 | 1 |

Örnekler arası farklar: katman sayısı (Haiku 4–5, Sonnet 8–10), `bb20` var/yok, katman sırası
(bantlar mumların altında/üstünde), nokta katmanının seriye bağlı olup olmaması, `x.domain`
(yok / 1749000000000 / 1747872000000 / 1748736000000 / 1750291200000…1759968000000 —
sonuncusu son bardan bir gün sonra), renkler (`#e53935`, `#ef5350`, `#c62828`, `#d32f2f`…),
başlık metni, sayıların yazımı (`72.84` / `72.83959265032422`). Aynı bulgu her açılışta
başka bir resim olurdu.

### 3.5 Model yolu: gecikme ve maliyet

| | Haiku 5.5 | Sonnet 5.5 | Sunucu |
|-|----------:|-----------:|-------:|
| Süre medyan (min–maks), ms — üst sınır | 9 159 (6 979–11 576) | 9 551 (8 515–10 313) | 0,0066 |
| Çıktı (bayt medyan / ≈token) | 485 B / ≈139 | 1 006 B / ≈287 | 294 B |
| Sunum başına maliyet, düşünmesiz | ≈ $0,00027 | ≈ $0,0070 | ≈ 0 |
| Sunum başına, +1 000 düşünme token'ı | ≈ $0,00077 | ≈ $0,017 | ≈ 0 |

Maliyet varsayımları: girdi ≈ 2 040 token (yalnız bu istem; gerçek ajan bağlamı dokümanı ve
sistem istemini de taşır → alt sınır). Haiku/Sonnet 5.5'te düşünme varsayılan açık ve çıktı
fiyatından faturalanır; miktarı ölçülemedi, +1 000 token bir duyarlılık satırıdır. İstemin
sabit kısmı (şema, gramer notları) önbelleğe alınabilir; bu girdi maliyetini düşürür, çıktıyı
ve gecikmeyi değiştirmez.

Gecikme notu: süreler alt ajan başlangıcı ve bir araç turu içerir. Doğrudan API çağrısında
~140–290 çıktı token'ı + düşünme için gerçekçi beklenti yine **~1–3 s** (ilk token gecikmesi
+ üretim); bu, 100 ms bütçesinin 10–30 katıdır. 100 ms'nin altına inen bir model yolu için
ağ gidiş-dönüşü tek başına bütçenin önemli kısmını yer.

## 4. Ölçek tahmini

Varsayım: 10 000 kullanıcı × 50 kapsül görüntüleme/gün × 3 yüzey = **1,5 M sunum/gün**
(ortalama ≈ 17/s; tepe 10× ≈ 175/s).

| | Sunucu | Model: Haiku 5.5 | Model: Sonnet 5.5 |
|-|--------|------------------|-------------------|
| Gecikme / 100 ms bütçesi | ~7 µs (bütçenin %0,007'si) | ~1–9 s (10–90×) | ~1–10 s (10–100×) |
| Hesap yükü | 1,5 M × 6,6 µs ≈ **10 CPU-saniye/gün**; tepe ≈ bir çekirdeğin %0,2'si | — | — |
| Maliyet/gün | ≈ $0 | ≈ $410 – $1 160 | ≈ $10 400 – $25 400 |
| Maliyet/yıl | ≈ $0 | ≈ $0,15 M – $0,42 M | ≈ $3,8 M – $9,3 M |
| Çevrimdışı / cihazda (K21-3) | çalışır (wasm, ağ yok) | çalışmaz | çalışmaz |
| Belirlenimcilik | aynı doküman → aynı bayt | 5 örnekte 5 farklı resim | 5 örnekte 5 farklı resim |
| Denetim: "bu resim neden böyleydi?" | günlükteki `show_finding` + doküman yeniden oynatılır, görünüm bayt bayt yeniden üretilir; kural kodda ve `PROTOCOL.md` §10.3'te yazılı | yalnız model çıktısı saklanırsa resim geri getirilir; *neden* sorusu modelin içinde kalır, tekrar sorulunca başka cevap çıkar; model sürümü emekliye ayrılınca yeniden üretim imkânsız | aynı |

Ara yol (ajan görünümü bulgu başına **bir kez** yazar, `put_view` ile dokümana kaydeder,
sunumlar onu kullanır): maliyet sunum sayısıyla değil bulgu sayısıyla ölçeklenir ve gecikme
çizim yolundan çıkar. Bu tam olarak K21-1'in "ajan grameri doğrudan da yazabilir" kısmıdır;
model yine çizim döngüsünde değildir.

## 5. Sonuç

- **H1 (gecikme) doğrulandı.** Sunucu `show_finding` uçtan uca medyan ~6,6 µs, p99 ~12 µs;
  fonksiyon 125 ns. Model yolu (üst sınır) ~9 s; doğrudan API'de bile ~1–3 s. Telefonda
  100 ms bütçesini yalnız sunucu yolu karşılar.
- **H2 (belirlenimcilik) doğrulandı.** Sunucu 1 000/1 000 ve süreçler arası aynı bayt;
  modeller 10/10 farklı çıktı; yapıca da Haiku 4/5, Sonnet 5/5 farklı.
- **H3 (doğruluk) kısmen çürüdü.** Modeller şemayı ve sert anlam denetimlerini tamamen geçti
  ve olgu barını sunucudan daha iyi işaretledi. Sorun hata değil, **keyfîlik**: olguya
  bağlı olmayan öğeler (`y = 30`, `bb20`), modelin yazdığı sayılar ve metinler, tanımsız
  kanallar. Bunlar ilke 3 ve 6 ile çelişir ve her açılışta değişir.
- **H4 (maliyet) doğrulandı, mertebe beklenenden düşük** (Haiku 5.5 ucuz): 1,5 M sunum/gün
  için Haiku ≈ $0,4–1,2 k/gün, Sonnet ≈ $10–25 k/gün; sunucu ≈ $0.
- **H5 (denetim) doğrulandı.** Yalnız sunucu yolunda bir düzenleyici resmi günlükten yeniden
  üretip kuralını okuyabilir.

## 6. K21-1 üzerine öneri

**K21-1 korunmalı** ("dil modeli çizim döngüsünde değil; ajan anlam düzeyinde konuşur,
deterministik sunucu gramere çevirir; ajan grameri doğrudan da yazabilir"). Kanıt: gecikmede
5–6 kerte (µs’ye karşı s), maliyette pratikte sonsuz oran, belirlenimcilik ve denetimde nitel fark. Çürütme
ölçütlerinin hiçbiri karşılanmadı (medyan < 100 ms değil, tek çıktıya yakınsama yok).

Ajanın grameri doğrudan yazması şu yerlerde değerli ve gerekli (ilke 8):

1. **Sunucunun bilmediği görünümler:** çok enstrümanlı karşılaştırma, korelasyon matrisi,
   parametre taraması yüzeyi, olay hizalı üst üste bindirme, bugün aklımıza gelmeyenler.
   Deney gösterdi ki model şemaya uygun, motorca kabul edilen gramer yazabiliyor.
2. **Bir kez yazılıp kaydedilen görünümler:** ajan bulgu oluştururken `put_view` ile görünümü
   yazar ve kapsüle bağlar; sonraki her sunum onu deterministik olarak kullanır (§10.2 zaten
   "kapsülün görünümü; yoksa sunucunun kurduğu" diyor). Maliyet bulgu başına, gecikme
   çizim yolunun dışında, denetim günlükte (`put_view` girdisi = resmin tam tarifi).
3. **Sunucuyu geliştirmenin kaynağı olarak:** modellerin tekrar tekrar yaptığı iyi seçimler
   (olgu barına dikey kural) sunucu kuralına alınabilir; böylece iyileştirme deterministik
   kalır.

Grameri doğrudan yazmanın güvenli olması için koşul: model yazımı yine komut yolundan
geçer, öneri kipine düşebilir (K21-5) ve her görsel öğenin bir olguya bağlı olması (ilke 6)
motorca denetlenebilir hâle getirilmelidir (aşağıda Öneriler).

## 7. Sınırlar

- Model örnekleri az (5 + 5) ve tek bir bulgu/istem için; daha karmaşık bulgularda (çok olgulu,
  karşılaştırmalı) hata oranı değişebilir.
- Model süreleri alt ajan duvar saati (üst sınır); token sayıları bayt/3,5 tahmini; düşünme
  token'ları ölçülmedi.
- İstem, ajanın gerçek bağlamından daha temiz ve küçük (yalnız ilgili seriler); gerçek ajan
  bağlamında maliyet daha yüksek, doğruluk daha düşük olabilir.
- Sunucu gecikmesi masaüstü CPU'da; telefon ölçümü yok (wasm/telefon için büyüklük farkı
  sonucu değiştirmez).
- `qp-render` `View`'i henüz çizmediği için resimler piksel olarak karşılaştırılamadı;
  karşılaştırma yapısaldır.

## 8. Öneriler (kapsam dışı, yapılmadı)

- **Sunucuya olgu barı işareti:** `show_finding` görünümüne olgunun `t` değerinde dikey bir
  kural eklemek (modellerin 8/10'u bunu yaptı; sunucu yapmıyor). `PROTOCOL.md` §10.3 ve
  altın görsel/döküm güncellemesi gerektirir → round tasarımına.
- **Kapalı kanal sözlüğü:** `Layer.encoding` serbest `string → string`; modeller `x` sabiti,
  `text`, `open/high/low/close` gibi tanımsız kanallar yazdı ve motor kabul etti. Gramer
  R2'de kanal adlarını ve değer biçimlerini (alan adı mı, sabit mi) tipe bağlamalı; `put_view`
  bilinmeyen kanalı `schema` hatasıyla reddetmeli.
- **Olguya bağlılık denetimi (ilke 6):** ajanın yazdığı sabit konumlu öğeler (kural, nokta,
  metin) için hangi olguya dayandığını belirten bir alan (ör. `fact`) ve değeri olgunun
  kanıtından türetme; modelin elle yazdığı `72.84` gibi sayılar yerine `evidence.value`'ya
  başvuru.
- **Model yazımı için öneri kipi:** ajanın `put_view`'i varsayılan olarak öneri olsun
  (K21-5 ile uyumlu); yeni görünüm türleri insan onayından geçsin.
- `presenter_view`'i `qp-session` dışına açmak (ör. `pub fn`) deneylerin kopya tutmasını
  gereksiz kılar.
