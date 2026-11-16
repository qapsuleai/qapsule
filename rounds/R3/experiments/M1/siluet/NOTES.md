# M1 aday "siluet" — tasarım notları

Öncül: her anlam türü kendi tanıdık siluetidir; yön siluetin baktığı yerdir; öteki kanallar
sade ve alışıldık kalır. Tür bilgisini yalnız siluet taşır; renk, dolgu, işaretler ona
binmez, yanına eklenir.

Crate: `m1_siluet` (`pub struct Alphabet`, `pub const LEGEND`, 109 sözcük). Kendi görmem
için kontak sayfası: `cargo run --example contact -- <çıktı.png> [büyütme]` (satırlar `K`,
sütunlar 16/24/32 px açık, 16/32 px koyu, sonra 32 px'te kanal çeşitlemeleri).

## Yerleşim

Kutu kenarı `s`. Siluet `(0,5 s; 0,43 s)` merkezli, kenarı en çok `0,75 s` olan karede;
alt şerit (`y ≥ 0,81 s`) teyit blokları, sağ alt köşe kapanış rozeti, sağ üst köşe sunucu
işareti. Her ilkel kutunun içinde (testte 16–64 px, iki tema, kesirli kutu konumları).

## Kanallar

| Kanal | Kodlama | Neden |
|---|---|---|
| **Tür** | devam = düz çapraz ok; dönüş = U dönüşlü ok (sağ bacaktan iner, dipten döner, sol bacaktan çıkar); kırılım = yatay çubuğu delip geçen dikey ok (çubuk yazı renginde); sıkışma = birbirine yaklaşan iki çizgi (üçgen formasyonu); aşırılık = iki omuzlu sivri tepe ve ucunda nokta; akış = iki dalgalı akıntı ve üstünde tek geniş ok başı | Her biri piyasa ya da trafik işaretlerinden tanıdık bir biçim. Ok başları farklı yerde: devam sağ üst, dönüş sol üst, kırılım ve akış orta (kırılımda çubuk, akışta dalga ile ayrılır), sıkışma ve aşırılıkta ok başı yok. |
| **Yön** | Yönelim: aşağı = yukarının dikey aynası; yönsüz = çeyrek tur sağa (devam: yatay ok, kırılım: dikey çubuk) ya da simetrik biçim (sıkışma `>`, akış `≈` ok başsız, aşırılık iki yönlü sivri). Renk yönü **yineler**: `up`, `down`, yönsüz `accent`. | Renk körlüğünde ve gride yön yine biçimden okunur (yön çiftinin en zayıfı gri koşulda 5,07). |
| **Güç** | Siluet boyu `0,75 s·min(1; 0,72 + 0,35·güç)` ve çizgi kalınlığı `s·(0,05 + 0,1·güç)` (en az 1,1 px). | "Büyük ve kalın = güçlü", alışıldık. 0,8 üstünde boy doyar, kalınlık sürer. |
| **Güven** | Kutunun alt kenarından güven düzeyine kadar dolan soluk zemin (silüet rengi, %24 saydamlık); güven yoksa zemin yok. | Doluluk = güven, sözlükteki adıyla. "Karne yok" ile "karne kötü" farklı görünür. |
| **Aşama** | aday = kesik çizgi + içi boş ok başı / içi boş nokta; kesinleşmiş = dolu, renkli; kapanmış = gri (`muted`) siluet ve rozet: gerçekleşti = dolu yuvarlak içinde ✓, gerçekleşmedi = yalın ✗, geri çekildi = yatay üstü çizili. | Kesik = taslak, gri = geçmiş, ✓/✗ = sonuç, üstü çizili = silinmiş; hepsi alışıldık. Gerçekleşti/gerçekleşmedi arasında kütle farkı (dolu yuvarlak) ölçüyü 1,2'den yukarı taşıdı. |
| **Tazelik** | Bütün ilkellerin saydamlığı `0,3 + 0,7·tazelik`. | Solan = eski. |
| **Teyit** | Alt şeritte soldan sağa 1–5 küçük blok (yazı renginde). | Sayılabilir işaret; 16 px'te bitişik bloklar bir ölçek çubuğu gibi okunur. |
| **Sunucu** | Sağ üstte dolu, yazı renginde kıvrık köşe üçgeni. | Damga/onay köşesi; silueti örtmez. |

Renkler yalnız temadan: `up`, `down`, `accent`, `muted`, `text`, `bg`. Kapanış rozetinin
arkası `bg` ile oyulur.

## Belirlenimcilik

Yalnız IEEE temel işlemleri ve `sqrt` (IEEE 754'te doğru yuvarlanır, her hedefte aynı bit);
yarım çember ve sinüs örnekleri sabit tablolar (`HALF_CIRCLE`, `SINE16`). `sin`/`cos`/`powf`
yok. Test: aynı girdi → aynı ilkel listesi.

## Ayırt edilebilirlik (son koşu, `harness m1 measure --alt siluet`)

- **Ana ölçü (en kötü koşulda `K`'nin en zayıf çifti): 4,60** — `reversal.down` ↔
  `flow.up`, 16 px / açık / gri. (Kontrol: 0,00.)
- `K` ortalamasının en düşüğü: 6,72 (16/açık/gri); 16/açık/normal: en zayıf 7,16, ortalama 21,10.
- Kutu dışı ilkel: 0.

| Kanal | En zayıf çift (en kötü koşul) | Koşul |
|---|---:|---|
| yön | 5,07 | 16/açık/gri (dönüş ↑/↓) |
| tazelik | 4,97 | 16/açık/protan |
| sunucu | 2,84 | 16/açık/protan |
| güven | 2,41 | 16/açık/protan (aşırılık: yok ↔ 0,3) |
| güç | 2,37 | 16/açık/protan (0,2 ↔ 0,9) |
| aşama | 1,64 | 24/açık/gri (sıkışma: aday ↔ kesinleşmiş) |
| teyit | 1,16 | 32/açık/protan (1 ↔ 2) |

Ayrıntı: `../raw/siluet/distinct.md`.

### Denenip bırakılanlar (ölçüyle)

- Akışta iki ayrı ok başı: 3,40 (dönüşün U bacaklarına benzedi). Yatay dalga + üstte ok
  başı: 3,14 (aşırılığa benzedi). Ters fazlı dalgalar: 3,31. Seçilen: aynı fazlı, genliği
  0,13 iki dikey dalga + geniş ok başı.
- Dönüş için çapraz "swoosh": 3,83 (devamın çapraz okuna benzedi). U'nun ok başını sol
  bacağa almak devam ↔ dönüş çiftini açtı.
- Daha kalın çizgi (0,06 + 0,1·güç): 4,07 — küçük boyda biçimler kütleleşip birbirine
  yaklaşıyor.

## Bilinen zayıflıklar

- **Aday aşaması eğrileri parçalıyor.** Kesik çizgi dalgalı akışı, U dönüşünü ve aşırılığın
  omuzlarını 32 px'te bile kırık parçalara böler; aday glifte tür okunurluğu düşer. Aday ↔
  kesinleşmiş ayrımı da zayıf (1,64; düz çizgili sıkışmada en düşük).
- **Teyit kanalı en zayıf kanal (1,16).** Bloklar alt şeride sığmak zorunda; 16 px'te
  aralıklar 1 px'in altında, sayı değil uzunluk okunur.
- **Güven zemini gürültülü.** Büyük soluk alan küçük boyda siluetin önüne geçiyor; üst
  kenarı yatay bir çizgi gibi görünüp kırılımın çubuğuyla karışabilir. Koyu temada bulanık
  bir kahverengi/bordo verir.
- **Güç mutlak okunmaz.** Tek başına duran bir glifte "küçük mü büyük mü" karşılaştırmasız
  belli değil; okuyucu testinde güç sınıfı tahmin düzeyinde kalabilir.
- **Akış ↔ dönüş** en yakın tür çifti (iki bacak + üstte kütle); gri 16 px'te 4,60.
- **Aşırılık `λ`/`Y` harfine benziyor**; "uçta nokta" anlamı açıklamasız okunmayabilir.
- **Kapanış rozeti küçük** (16 px'te ~5 px): ✓ ile ✗ ayrımı 16 px'te zor; gri siluet
  "kapanmış"ı taşır, sonucun kendisi büyütülmeden zor okunur.
- Yönsüz biçimler (sıkışma `>`, akış `≈`) dışındaki yönsüz tür çeşitlemeleri `K`'de yok;
  ölçülmediler (devam →, kırılım dikey çubuk, aşırılık iki yönlü, dönüş ⊂).
- Ölçü ΔE ortalamasıdır; ok başı gibi küçük ama anlamlı farkları seyreltir. Okuma testi bu
  notlar yazılırken koşulmadı (düzenleyici koşacak).
