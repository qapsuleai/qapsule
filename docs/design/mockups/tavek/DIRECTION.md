# Yön B — `tavek` ("grafik anlamı taşır")

## İddia

Qapsule'ün kendi katmanı yan panelde değil fiyat grafiğinin üstünde, grafiğin kendi
dilbilgisiyle okunur: her seviye fiyat ekseninde bir etiketle biter, sol ucunda kimin yazdığını
ve hangi durumda olduğunu söyler, kapsülün kanıt penceresi soluk bir bant ve zaman ekseninde
bir parantezdir. Köken renk artı çizgi biçimidir (insan düz mürekkep, ajan kesik mor, motor
noktalı grafit); grafiğe yeni ton ekleyen tek köken ajandır, bu yüzden sahne sakin kalır ve
ayrım renk körlüğünde de biçimle korunur. Yan paneller destekler ve "Grafikte" ile grafiğe
döner; izleme durumu, öneri ve kapsül önce grafikte görülür.

## Ortak alfabeden alınanlar (değişmeden)

- Grafik dili OV-5…OV-10: sağ fiyat ekseni kendi oluğunda (64 px; telefonda 56), etiket 48 px'te
  bir, iki düzeyli zaman, OHLC + değişim lejantı (telefonda C), son değer etiketi + noktalı değer
  çizgisi, kesikli imleç + eksen hapları, altta hacim (%18). Çakışma önleme OV-12 (P6 = 0).
- Renk disiplini OV-2, AT-G1, VV-D5: krom kromasız (P1 ≤ 0,012); kroma yalnız veri, anlam ve
  kökende; durum hiçbir zaman yalnız renkle söylenmez (şekil + sözcük + renk).
- Yoğunluk OV-4, AT-G4: masaüstü gövde 13, grafik rakamı 11 (Plex Mono, tablo rakamı), denetim
  28 px; telefon gövde 16, en küçük 13, 44/48 px hedef.
- Biçim dili OV-4: denetim köşesi 6, kart 8, eksen etiketi 4; tek saç çizgisi kenar; yalnız
  yüzen kartta hafif gölge. Yerleşim OV-13 (üç sütun), ajan paneli OV-17/OV-18/AT-C3, izleme
  listesi OV-22, sağlık OV-21/VV-B1 `_base` iskeletinden olduğu gibi.
- Alarm çizgisinin eksen etiketi (OV-12, AT-C8 "eksende etiketle biten çizgi"): Qapsule'ün
  bütün seviyeleri bu tanıdık biçimi kullanır; yeni simge icat edilmedi.

## Qapsule'e özgü katmanda ne yapıyor

| Konu | Grafikte | Grafik dışında |
|---|---|---|
| **Köken** | İnsan: düz mürekkep çizgi, dolu eksen etiketi. Ajan: kesik mor çizgi (`--qp-agent-dash` 4 3), kesik çerçeveli etiket. Motor: noktalı grafit çizgi, noktalı çerçeveli etiket. | Satırın sol kenarı aynı çizgi biçimini tekrarlar (izleme görevleri, kapsül akışı); araç çubuğunda ve durum şeridinde çizgi anahtarı ("İnsan, düz / Ajan, kesik / Motor, noktalı"). |
| **İzleme durumu** (Y31, Y32) | Seviyenin sol ucunda çip: "İzleme ◐ Kırıldı ■■□ 2/3 bar"; kırılım barında küçük dikey çentik, çentikten bugüne seviye tam güçte, öncesi soluk. Tutundu ● / geri döndü ◇ / kuruldu ○. | Görev satırında aynı sözcük ve "Grafikte" bağı; telefonda eksen etiketi biçiminde "112,40 │ ◐ Kırıldı 2/3". |
| **Çizimden niyete** | Trend çizgisinin başında tek satırlık niyet hapı ("Altında kapanış → uyar · 3 bar · günlük"), destek çizgisinin boş tarafında; tam kart yan panelde. | — |
| **Ajan önerisi** | Önizleme kesik mor seviye + eksende kesik etiket; seçiliyken sol uç çipinde adı konmuş iki eylem ("Reddet", "Ekle"). Onaysız uygulanmaz. | Onay kartı "Grafikte" bağıyla; telefon kulesinde öneri kendi küçük grafiğinde. |
| **Kapsül** | Kanıt penceresi: `evidence` bandı (grafit, alfa 0,08) bütün bölmeleri boydan geçer; zaman ekseninde "Kanıt" parantezi (tarih hapı biçiminde); pencerenin tepesinde ≤ 12 px motor elması ve "Kapsül …" yazısı (P8: işaret ≤ 20 px masaüstü, 24 px telefon). | Açılmış kapsülde "Kanıt penceresi grafikte işaretli · 14–22 Ağu"; sicil ("tutma %64, n = 42, taban %51", "Yeniden oynatma", "Sinyal, tavsiye değil") alt sayfada. |
| **Kur ve göster, sicil** | Bu turda grafikte ayrı işaret yok: kuran ajansa seviye kesik başlar, onayla düz mürekkebe döner (köken değişimi görünür kalır). | Alt sayfa ve akış satırı. |

Yerleşim kuralı: **seviyenin sağ ucu değeridir (eksen etiketi), sol ucu kim ve ne durumda
(çip)**. Böylece son barların üstü boş kalır; R5'in "işaret son mumların üstüne biniyor"
kusuru (VISUAL §3.2/4) yapıdan engellenir.

## Kayıt değerleri (temiz oda, `crates/qp-grammar/src/look/tavek.rs`)

- Koyu: serin nötr krom; merdiven gri `neutral` L\*≈42 (3:1 zemine) < aşağı < yukarı < mürekkep. Açık: ters.
- `s1` = vurgu = odak (mavi); ajan moru seri paletine konmadı. Kanıt rengi `muted`.
- Masaüstü tarih etiketleri de Plex Mono 11 (telefonda 13): telefonda Atkinson 13 ile zaman
  ekseni ay düzeyine düşüyor ve 390 px'te 3 etiket kalıyordu (P5); rakam ve tarih tek yazı tipinde.

## Sınama

`scripts/design-check --look tavek` ve `--look tavek-tel`: P1–P8, P10–P13, H1–H3 iki temada
geçer; P9 yalnız galeri düzen dökümüyle ölçülür. Maket renk taraması 0.

## Bilinen zayıflıklar

- **RD2/C2b sonrası:** seviye çizgisinin köken rengi ve biçimi (insan düz, ajan kesik, motor
  noktalı) ve eksendeki köken renkli etiket artık çekirdekte (`chart.origin_levels`,
  `compile/alphabet/levels.rs`; galeride Ç2 `desk-levels` görünümünü çizer). Sol uç çipi,
  kırılım çentiği, kanıt bandı/parantezi ve niyet hapı hâlâ maket (satır bütçesi). Ajan ve
  motor etiketi de dolu çizilir (kesik/noktalı çerçeveli etiket maket kalır). Motor grafiti
  etiket yazısı 4,5:1 tutsun diye açıldı (değerler kayıtta: `look/tavek.rs`, rol `--qp-origin-engine`). Aşağıdaki madde
  ara karardan önceki durumdur.
- Grafik üstü katman bu turda **maket**: çekirdek bugün seviye etiketi, sol uç çipi, kanıt
  parantezi çizmiyor. Yön seçilirse gramer işi gerekir (eksen etiketinin altındaki fiyat
  etiketini düşürmek, parantezin altındaki tarih etiketini düşürmek, çip yerleşimi P6 ile);
  bu turda derleyiciye yeni mantık yazılmadı. Maketteki konumlar hücrenin kesirleri, veriye
  bağlı değil; bir iki yerde parantezin kenarı bir tarih etiketini yarım örtüyor.
- Grafiğe çok şey binebilir: dört seviye + bant + niyet hapı Ç2'de sınırda. Öneri: varsayılan
  olarak yalnız seçili ve durumu değişmiş seviyenin çipi görünür, ötekiler yalnız eksen etiketi.
- İnsan rengi mürekkep olduğu için insan çizgisi metinle aynı tondadır; ayrım kalınlık ve
  eksendeki dolu etiketle. Motor grafiti `invalid` grisine yakın (ayrı H3 kümelerinde; çizgi
  biçimi ayırır).
- Telefonda küçük öneri grafiğinde eksen etiketi son değer etiketiyle çakıştığı için çipe
  taşındı; telefonda eksen etiketi kuralı yer darlığında gevşiyor.
- Çekirdekten gelen iki kusur bu yöne özgü değil: telefon grafiğinde "RSI14 55,10" lejantı
  çizginin üstüne biniyor; Ç4'te `120,00` fiyat etiketi son değer etiketine değiyor.
- Ajan moru birincil düğme mavisinden farklı bir ton; Ç1'de toplam ton sayısı grafikte P2 ile
  ölçülen 5'e ajan moru eklenince 6 olur (sınırda).
