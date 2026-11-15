# M1 aday alfabe — İz (`iz`)

Öncül (K18, bilinen yöntemlerle sınırlı değil): her işaret bir **anlamın hareket izidir**,
verinin değil. İz veri çizmez; anlamın "nasıl hareket ettiğini" tek bir çizginin
topolojisiyle söyler. Diğer kanallar aynı fikirden türer: kanallar izi değiştirir — kalınlığını,
altındaki zemini, sonunu, çerçevesini, solukluğunu.

Kod: `src/lib.rs` (`m1_iz::Alphabet`, `m1_iz::LEGEND`). Kontakt sayfası:
`cargo run --example contact -- <dizin>` (12 `K` satırı × 14 kanal sütunu, 16/24/32 px,
açık/koyu, 4× büyütme).

## Kanal kanal tasarım

| Kanal | Kodlama | Gerekçe |
|---|---|---|
| **Tür** (şekil) | devam: tek düz çizgi; dönüş: inip U dönüşü yapan eğri; kırılım: gri eşik çizgisini dik basamakla aşıp öbür tarafta yatay kalan iz; sıkışma: tek noktada birleşen iki çizgi; aşırılık: kenara yakın gri sınırı hızlanarak aşan ve tepede taşıp kıvrılan eğri; akış: yatay gri fiyat çizgisinden dikine ayrılan dalgalı akıntı (yön yoksa fiyat çizgisine paralel akar) | Tür = hareketin topolojisi: süren, geri dönen, eşik aşan, daralan, taşan, fiyattan ayrı akan. Yardımcı çizgiler (eşik, sınır, fiyat yolu) ince ve gri (`axis`): izin neye göre hareket ettiğini gösterir, izin kendisiyle karışmaz. |
| **Yön** | Yukarı: iz yükselir; aşağı: aynı izin dikey aynası; belirsiz: yatay (sıkışmada tepe noktası ortada, akışta paralel akıntı). Renk pekiştirir: `up` / `down` / `s2` (mavi) | Yön geometride kodlu, renk ikincil; renk körlüğü ve gri benzetimde de ayna farkı kalır (yön kanalının en kötü değeri 8,50, gri). |
| **Güç** (büyüklük/kalınlık) | Çizgi kalınlığı `S·(0,03 + 0,13·güç)`, ok ucu `S·(0,13 + 0,14·güç)` | İzin "ağırlığı". 16 px'te 0,2 → ~0,9 px, 0,9 → ~2,4 px. |
| **Güven** (doluluk, karne) | İzin **zemini**: türe bağlı bir alan, karne güveniyle orantılı saydamlıkla (`0,16 + 0,56·güven`) boyanır — eğimin altı (devam), U'nun altı (dönüş), basamağın yanındaki duvar (kırılım), kama içi (sıkışma), normal aralığın üstündeki uzama (aşırılık), akıntının yelpazesi (akış). Karne yoksa (`None`) zemin hiç yok | "Doluluk" kelimenin tam anlamıyla izin doldurduğu yer. Zemin biçimleri türler arasında ayrıştırıcı olacak şekilde seçildi (ölçümle; aşağıda). |
| **Aşama** (çizgi biçimi) | aday: kesik çizgi + içi boş ok ucu + yalnız çerçeve köşeleri (geçici); kesinleşmiş: dolu çizgi + dolu ok ucu, çerçeve yok (hâlâ hareket ediyor); gerçekleşti: kapalı çerçeve (iz rengi) + izin sonunda dolu yuvarlak (vardı); gerçekleşmedi: koyu (`text`) kapalı çerçeve + sonda dik çubuk (durdu); geri çekildi: her şey gri, çerçeveli ve büyük çarpıyla üstü çizili | "Sonuçlanmış = kapanmış" fikri doğrudan çerçeve olur; sonucun ne olduğunu izin **sonu** söyler (ok = gidiyor, yuvarlak = vardı, çubuk = engellendi). Kesik çizgi tek başına 16 px'te bulanıklaşıyordu; köşe işaretleri adayı belirgin yaptı (aşama en kötü 1,47 → 3,58). |
| **Tazelik** (solukluk) | Bütün işaretin opaklığı `0,3 + 0,7·tazelik` | İz zamanla solar. |
| **Teyit** (1–5) | İzin altındaki taban satırında aile başına bir nokta (`text` rengi) | Sayılabilir, konumu sabit; izin şeklini bozmaz. |
| **Sunucu işareti** | Taban satırı turuncu (`accent`) bir çizgiye dönüşür ve sağ ucunda turuncu bir elmasla mühürlenir | Kanıt satırı "doğrulanmış zemin" üstünde durur. Yalnız elmas 0,87'de kalıyordu; taban çizgisi 1,97'ye çıkardı. |

Belirleyicilik: yalnız IEEE temel işlemleri ve `sqrt` (IEEE 754'te doğru yuvarlanır, her
hedefte aynı bit); eğriler polinom Bézier, trigonometri yok; dalga kübik yaylarla kurulur.
Renkler yalnız `Theme`'den (`up`, `down`, `series[1]`, `axis`, `muted`, `text`, `accent`).
Son bir kıskaç her ilkeli kutunun içine alır (yarı kalınlık dahil). Bozuk değerler
(`NaN` güç, sonsuz tazelik, eksi güven, 0 ya da 200 teyit) sonlu ve kutu içinde çizilir.

Testler (`cargo test`): belirleyicilik (6 tür × 3 yön × 5 aşama × 5 kanal birleşimi, 6 boy,
iki tema), kutu içi + boş olmama + sonlu koordinat, renklerin temadan geldiği, bozuk
değerler, açıklamanın ≤ 120 kelime olduğu.

## Ayırt edilebilirlik (son koşu, `harness m1 measure --alt iz`)

Ham kayıt: `rounds/R3/experiments/M1/raw/iz/distinct.{json,md}`.

- **Ana ölçü (en kötü koşulda `K`'nin en zayıf çifti): 7,55** — `breakout.up` / `flow.up`,
  16 px / açık / protanopi. (Kontrol: 0,00.)
- En düşük `K` ortalaması: 10,56 (16 px, açık, gri). Kutu dışı ilkel: 0.
- 16 px açık normal: `K` en zayıf 10,70, ortalama 21,85. Koyu temada `K` en zayıf ≥ 9,54.

| kanal | en zayıf çift ΔE | koşul |
|---|---:|---|
| yön | 8,50 | 16/açık/gri (`flow.up` / `flow.down`) |
| aşama | 3,58 | 24/açık/protan (`flow.up` aday / kesinleşmiş) |
| tazelik | 3,32 | 24/açık/protan |
| güç | 2,21 | 32/açık/protan (`flow.up`) |
| sunucu | 1,97 | 24/açık/gri |
| teyit | 1,19 | 16/açık/protan (1 ↔ 2) |
| güven | 0,64 | 24/açık/protan (`reversal.up` 0,3 ↔ 0,8) |

Yineleme özeti (ana ölçü): ilk taslak 4,09 (devam ↔ kırılım, ikisi de çapraz yükselen ve
zeminleri örtüşüyordu) → kırılımın ve aşırılığın zemini değişti, akış dikine akıntıya
çevrildi, dönüşün zemini U'nun altına alındı → 7,55. Aşama kanalı aday köşeleri ve
"sonuçlanmış = çerçeve" ile 0,90 → 3,58.

## Bilinen zayıflıklar (dürüstçe)

- **16 px'te akış (yukarı/aşağı) bir lekeye dönüşüyor.** Dikey dalga + yelpaze 16 px'te
  "dalgalı" okunmuyor; tür daha çok fiyat çizgisinin konumu ve yelpazeyle ayrılıyor. Ok
  ucu dalganın içinde kayboluyor. 24 px ve üstünde S biçimli akıntı seçiliyor.
- **Güven kanalı zayıf** (0,64): zemin alanı türden türe değişiyor; dönüşte U'nun altı dar
  olduğu için 0,3 ile 0,8 farkı küçük. Karne yok (`None`) ile 0,3 arası da yalnız soluk bir
  zeminin varlığı.
- **Teyit sayısı** (1,19) küçük noktalarla; 16 px'te 4 ile 5'i saymak zor. Okuma testinde
  sorulmuyor ama kullanıcı için zayıf.
- **Zemin seçimleri kısmen ölçüme göre yapıldı**: kırılımın zemini "basamağın yanındaki
  duvar" (izin üstü), devamın ve dönüşün zemini izin altı. Anlamsal olarak tek bir kural
  değil ("izin taradığı alan" türe göre yorumlanıyor); açıklamasız okuyucu zemini anlamayabilir.
- **Aşırılık eğrisinin tepe kıvrımı** sağa-aşağı bakan bir ok ucuyla bitiyor; "yukarı
  aşırılık"ta ok aşağıyı gösterebilir gibi okunabilir (yön asıl olarak eğrinin yükselişinde
  ve renkte).
- **Gerçekleşmedi çubuğu** izin teğetine dik; akışta ve aşırılıkta eğik duruyor, 16 px'te
  küçük koyu bir leke gibi.
- **Kesinleşmiş ile aday** farkı 16 px'te büyük ölçüde köşe işaretlerine dayanıyor; kesik
  çizgi tek başına bulanıklaşıyor.
- **Geri çekildi** çarpısı izin kendisini neredeyse siliyor: türü okumak zorlaşıyor
  (tasarım gereği "üstü çizili", ama tür bilgisi kaybı da var).
- Renk ikincil olsa da yön okuması açıklamasız okuyucuda yeşil/kırmızı alışkanlığına
  dayanıyor; mavi = belirsiz bir gelenek değil, açıklama gerektirir.
- Okuma testleri bu aday tarafından çalıştırılmadı (görev kuralı); okunurluk yalnız
  hesapla ve tasarımcının kendi gözüyle (kontakt sayfası) değerlendirildi.
