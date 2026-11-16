# M10 alternatifi `zumlak` — açılma bir kamera hareketi

Kod: `crates/qp-grammar/src/transitions/zumlak.rs` (kayıt: `transitions.rs`'te `mod zumlak;` ve
`TRANSITIONS`'ta tek satır). Seçim: `?transition=zumlak` ya da Ayarlar → Geçiş. Süre 480 ms.
Bu not okuyucuya gitmez.

## Amaç

Kapsülün küçük fiyat çizgisi ile grafiğin fiyat paneli aynı düzlemi çizer: bar × fiyat. Açılma
bu düzlemde bir **yakınlaşmadır**: kapsülün içindeki çizgi ve işaretli an büyüyüp grafiğin
kendisi olur; olgu gözün altında kalır, yeni bağlam (mumlar, kanıt seviyeleri, eksenler) onun
etrafında belirir. Hedef SPEC §4 G1/G2: "grafikte işaretli yer, az önce kapsülün söylediği şeyin
neresi?" sorusunun cevabı geçişin kendisi olsun. Okuyucu kapsüldeki noktanın büyüyüp grafikteki
mum olduğunu görür, iki ekranı kendisi eşleştirmek zorunda kalmaz.

## Ne gösterir, ne göstermez

- **Ortak pencere (`span`):** iki sahnenin fiyat panellerinin **ikisinde de görünen** bar × fiyat
  penceresi (sürekli bar koordinatı ve fiyat aralıklarının kesişimi), her sahnede bir kutu. İki
  kutu aynı veriyi çizer; birinden ötekine giden eşleme (eksen başına ölçek + kayma) kameradır.
- **Kamera:** `p = 0`'da eski sahne yerinde; yeni sahne, kendi pencere kopyası eskinin
  penceresinin tam üstüne gelecek biçimde küçültülüp yerleştirilir. `p = 1`'de yeni sahne kendi
  yerinde. Arada ölçek **geometrik** ilerler (`k^e`, `detmath` ile belirleyici; yakınlaşma böyle
  düzgün görünür), odak noktası (kapsülün olgu noktası; yoksa işaretli barın sütunu pencere
  ortası yüksekliğinde; yoksa pencere ortası) eski sayfa konumundan yeni sayfa konumuna doğru
  bir çizgide gider. Yumuşatma `smoothstep`.
- **Yalnız eski fiyat paneli kamerayla gider** (`pane:before`); kapsülün sözleri ve işareti
  yerinde kalıp söner (`scene:before`). Böylece hiçbir yazı büyütülmez ve büyütülmüş kopya
  başlık şeridine taşmaz.
- **İşaretli barın sütunu** (yeni sahnenin `bar:<i>`'si) ilk kareden tam opaklıkla, kamerayla
  çizilir: grafik bu sütundan büyür.
- **İşaret** (`glyph`) kapsüldeki yerinden grafikteki yerine (barın üstü) uçar.
- Yeni sahne `p` 0,05–0,55 arasında belirir; eski sözler 0,15–0,6, eski panel 0,35–0,8 arasında
  söner; gerçek grafik tuvali 0,85–1 arasında açılır (o anda kopya onunla çakışır).
- Kapanış (grafik → kapsül) aynı hareketin tersidir (uzaklaşma); cümle → kapsül de aynı kuralla
  (yüzün çizgisi kapsülün çizgisine). Ortak pencere yoksa (bir yanda fiyat paneli/sahne yok)
  tasarım başvuru `r3` karesine düşer.
- Grafiğin kendisi bütün geçişlerde aynıdır (`depth::chart_view`; README Açık sorular 3):
  yakınlaşma grafiği işaretlemez, yalnız oraya nasıl gelindiğini gösterir.
- Metin şablonu yok: tasarım sözcük üretmez (TR/EN farkı yok; başlık, karne kapsülün kendi
  metinleridir). Renk yok: yalnız sahnelerin kendi pikselleri kopyalanır (`Theme` dışı renk yok).

## Ortak öğeler (ölçülen) ve taşıyıcılar

`Opening.shared`: iki sahnenin ortak çapaları (`glyph`, `bar:<i>`, `fact:<id>`,
`evidence:<fact>`, iki yanda varsa `headline`/`ledger`) **`price` hariç** + `span`. `price`
çıkarıldı: paneller ortak pencerenin dışında farklı veri gösterir; aynı olan kısım `span`'dır
(iki kez saymamak için). Üç tek yanlı kayıt bütün sahneleri kare işlevine taşır
(`scene:before`, `pane:before`, `scene:after`; tek kutulu oldukları için düzenek onları ortak
saymaz — bilerek; alan payını şişirmesinler).

Kare kuralı: öğe listesi (sayı, sıra, kaynak, `src`) `p`'den bağımsız; yalnız `rect` ve
`opacity` değişir (birim testi `camera_ends_and_frame_rule`). Boyama sırası: eski sahne
(yerinde), eski fiyat paneli (kamera), yeni sahne (kamera), işaretli barın sütunu, işaret.

## Düzeneğin hesabı (`m8910 m10 --transition zumlak`, 2026-11-16)

| durum | ortak | öğeler | alan payı önce | alan payı sonra | kayma ort. / en çok (px) | dokunma |
|---|---:|---|---:|---:|---:|---:|
| 6 | 3 | evidence, glyph, span | 0,2813 | 0,2553 | 93,97 / 143,01 | 2 |
| 101 | 4 | bar, evidence, glyph, span | 0,0957 | 0,6816 | 145,71 / 258,01 | 2 |
| 112 | 3 | bar, glyph, span | 0,0946 | 0,6706 | 155,68 / 274,01 | 2 |
| 3 | 4 | bar, evidence, glyph, span | 0,1460 | 0,6707 | 145,18 / 266,01 | 2 |
| **ort.** | **3,5** | | **0,1544** | **0,5696** | **135,14** | **2** |

(Başvuru `r3`: 3,5 öğe, 0,2803 / 0,6690, kayma 124,86, dokunma 2.) Alan payı `r3`'ten düşük:
`price` (bütün panel) yerine yalnız gerçekten aynı veriyi çizen pencere sayılıyor. Kayma
düzeneğin tanımıyla önce/sonra kutularının merkez farkıdır (çoğu işaretin kapsül köşesinden
barın üstüne uçuşu); yakınlaşmanın ara karelerdeki odak sabitliğini ölçmez.

Ekranlara bakıldı (`raw/screens/06489420-*`; commit edilmez): 101, 112, 3'te `p = 1/3` karesinde
küçük grafik kapsülün çizgisinin tam üstünde (mumlar çizginin üstüne oturuyor, işaretli sütun
kapsüldeki noktanın yerinde), `2/3`'te grafik büyüyerek yerine gidiyor, işaret barın üstüne
iniyor. `DRY_RUN=1 run_readers.sh m10 zumlak {F,H} - 3`: istemler yazıldı, yasak sözcük denetimi
geçti, okuyucu koşulmadı.

## Bilinen zayıflıklar

- **Olgu kapsülün penceresinde değilse** (durum 6: kırılım 93 bar önce, kapsül son ~60 barı
  gösteriyor) yakınlaşma tersine döner: grafik daha geniş bir zamanı gösterdiği için `p = 0`'da
  kapsülden büyük başlar ve küçülür (uzaklaşma); odak pencere ortasıdır, işaretli an ilk karede
  ekranda yoktur. Bu durumda G1'e katkısı `r3`'ten fazla olmayabilir.
- **Eksenler bağımsız ölçeklenir** (fiyat ekseni genelde zamandan daha çok büyür): gerçek bir
  kamera değil, iki eksenli bir yakınlaşma; ara karelerde grafik biraz basık/uzun görünür.
- **Odak tam sabit kalmaz:** grafiğin yerleşimi bütün tasarımlarda aynıdır, olgunun son konumu
  kapsüldekinden farklıdır (kapsül çizgisi kart içinde aşağıda, grafik panelinde yukarıda); odak
  bu iki nokta arasında düz gider. "Aynı noktada kalma" ancak grafik kuralı geçiş tasarımına
  açılırsa (README Açık sorular 3) tam sağlanabilir: grafiğin aralığını ve panel yerini olgu
  kapsüldeki yerinde kalacak biçimde seçmek.
- Kopyalar sabit katmanda (`#fx`) olduğu için büyüyen grafik kısa bir an sayfanın başka
  öğelerinin (cümle şeridi) üstüne binebilir; yalnız eski panel büyütüldüğü için az.
- Yeni sahnenin küçük kopyası ilk karelerde kabaca ölçeklenmiş piksellerdir (tarayıcının kopya
  ölçeklemesi); yazılar `p` küçükken okunmaz (bilerek: okunacak yazı eski sözlerdir).
- Taşıyıcı kayıtlar (`scene:*`, `pane:before`) protokoldeki `Opening.shared`'a girer; adları
  çapa adlarıyla çakışmaz, ama `shared`'ı okuyan bir istemci onları tek yanlı öğe olarak görür.
