# Tasarım sözleşmesi (RD2; iskelet)

Bu belge kuralları taşır, sayıyı taşımaz (OV-1). Her değerin sahibi çekirdekteki görünüş
kaydıdır (`crates/qp-grammar/src/look.rs`, `look/*.rs`); kabuk değerleri yalnız `css_vars`
çıktısından okur (`qp look css <ad>`). "Değerler" bölümündeki tablolar kayıttan üretilir
(`scripts/design-contract`); elle yazılmaz, `scripts/design-check` farkı 0 olmadıkça düşer.
Her kural bir `design-check` satırına bağlıdır (P1–P13, H1–H3). Eşikler tek yerdedir:
`crates/qp-grammar/tests/design_check/thresholds.rs` (başlangıç; Arkin'in ara kararından
sonra bir kez ayarlanır ve buraya yazılır).

Yön seçilene kadar değer bloğu R5'in reddedilen görünüşünü (`oltan`) karşılaştırma için
gösterir; seçilen yön gelince blok onun iki profiliyle değişir.

## Renk rolleri

- **Krom nötrdür:** krom yüzey ve metin rollerinin (`chrome.*`, `bg`, `surface`, `text`,
  `muted`, `grid`, `axis`, `line`) kroması eşiğin altındadır (P1).
- **Kroma veride ve anlamdadır:** seri paleti (`s1`–`s8`), `up` / `down` ve anlam rolleri
  dışında renk yoktur; bir grafik sahnesindeki ayrı ton sayısı sınırlıdır (P2).
- **Yukarı / aşağı sakindir:** koyu temada `up` ve `down` orta doygunluk ve ışıklıktadır,
  neon değildir (P3).
- **Anlam ışıklıkla da ayrılır:** `up`, `down`, `valid`, `invalid` beş görme koşulunda
  ayrışır; bilerek paylaşılan roller (`neutral` = `invalid` = `closed`) biçimle ayrılır (H3).
- **Köken kendi rolündedir:** insanın, ajanın ve motorun yazdığı işaret `origin.human`,
  `origin.agent`, `origin.engine` rollerinden boyanır, mum renginden değil; ajan çizgisi ayrıca
  kesiklidir (`--qp-agent-dash`), renk körlüğünde ayrım biçimle korunur (H2, H3).
- **Vurgu yalnız seçim ve odaktır:** `focus` / `chrome.accent` süs için kullanılmaz.
- **Karşıtlık iki profilde de tutar:** metin 4,5:1, metin dışı öğe 3:1 (H1, H2).

## Tip ölçeği

- Altı rol vardır: `headline`, `title`, `body`, `label`, `figure`, `note`; sahnede her metin
  bir roldedir, boyu rolden gelir.
- Rakamlar `figure` rolünün yazı tipinde tabular çizilir (P11).
- Masaüstünde gövde ve grafik yazısı yoğun aralıktadır; telefonda gövde ve en küçük yazı
  dokunma profilindedir (P10).

## Boşluk

- Bütün aralıklar boşluk ölçeğinin bir basamağıdır (`--qp-space-0..5`); başka aralık yoktur.
- Fiyat ekseni kendi oluğundadır (`chart.axis_gutter_px`); eksen yazısı çizim alanına,
  başka bir yazıya ya da seçim çizgisine binmez, çerçeveden taşmaz (P6).

## Yarıçap

- Denetim ve kart köşeleri küçük ve sabittir (`--radius`, `--qp-radius-card`) (P12).

## Kenar

- Tek kenar rolü vardır (`chrome.line` = `line`); arka plana karşı saç çizgisidir, denetim
  sınırı (`chrome.control`) 3:1'dir (P13). Gölge varsa `--qp-shadow` tek değerdir.

## Grafik alfabesi

- Sağ fiyat ekseni, iki düzeyli zaman ekseni, son değer etiketi, değer çizgisi, imleç ve
  eksen hapları, OHLC lejantı, altta hacim: yeniden icat edilmez (`chart.*`; P4, P5, P7).
- Veri üstündeki anlam işareti küçüktür ve veriyi örtmez (`chart.mark_max_px`; P8).
- Masaüstü çalışma alanı birden çok bölge ve grafik bölmesi gösterir (P9, galeri düzen dökümü).

## Yoğunluk (iki profil)

- Bir yön iki görünüş kaydeder: `<ad>` (`dense`, masaüstü) ve `<ad>-tel` (`touch`, telefon;
  `..<AD>` ile yazılır, yalnız tip, krom, boşluk ve işaret değişir). `--qp-profile` hangisi
  olduğunu söyler.
- Karşıtlık ve renk körlüğü eşikleri iki profilde aynıdır; en küçük yazı ve hedef boyu
  profile göredir (P10).

## Hareket

- Süre ve eğri kayıttadır (`--motion`, `--ease`); hareket bilgi taşır, süs değildir; azaltılmış
  hareket tercihinde kabuk geçişleri kapanır.

## Yapılmayacaklar

- Kabukta renk kodu, piksel ya da metin kurulmaz; değer yalnız `css_vars`'tan okunur.
- Krom rollerine renk (dolu not, bordo uyarı şeridi, renkli vurgu yüzeyi) verilmez.
- Neon `up` / `down`, degrade, süs gölgesi, büyük köşe yarıçapı kullanılmaz.
- Veri üstüne büyük simge, yüz ya da plaka konmaz; alışılmış grafik öğesi yerine yeni işaret
  icat edilmez.
- Ajan işareti kullanıcı stiliyle maskelenmez; köken renksiz bırakılmaz.
- Bu belgeye değer elle yazılmaz.

## Değerler

`oltan` (R5, reddedildi — K57; yalnız karşılaştırma için):

<!-- generated:look=oltan -->
Görünüş `oltan`: profil `legacy`, varsayılan düzen `dark`, köken rolleri türetilmiş.

#### Renk rolleri
| rol | grup | koyu | açık |
|---|---|---|---|
| `chrome.bg` | krom nötr | #060608ff | #eeeeeaff |
| `chrome.card` | krom nötr | #121317ff | #ffffffff |
| `chrome.fg` | krom nötr | #f6f6f2ff | #111216ff |
| `chrome.muted` | krom nötr | #aaadb4ff | #52565eff |
| `chrome.line` | krom nötr | #282a30ff | #d6d6d0ff |
| `chrome.control` | krom nötr | #6e7076ff | #7e8084ff |
| `chrome.page` | krom nötr | #000000ff | #c8c8c2ff |
| `s1` | veri | #8aaaffff | #2850dcff |
| `s2` | veri | #ffb040ff | #c46800ff |
| `s3` | veri | #c48cffff | #7e34c4ff |
| `s4` | veri | #5cd6e6ff | #008096ff |
| `s5` | veri | #ff80c4ff | #c42878ff |
| `s6` | veri | #d6d678ff | #787814ff |
| `s7` | veri | #b09682ff | #6e5240ff |
| `s8` | veri | #c8c8c8ff | #606060ff |
| `grid` | veri | #24262cff | #e8e8e4ff |
| `axis` | veri | #626670ff | #8e9096ff |
| `up` | anlam | #4ee094ff | #107642ff |
| `down` | anlam | #f6584aff | #840e16ff |
| `warn` | anlam | #ffc850ff | #965a00ff |
| `valid` | anlam | #f6f6f2ff | #111216ff |
| `invalid` | anlam | #6e7076ff | #888a8eff |
| `neutral` | anlam | #6e7076ff | #888a8eff |
| `closed` | anlam | #6e7076ff | #888a8eff |
| `focus` | anlam | #8aaaffff | #2850dcff |
| `origin.human` | köken | #f6f6f2ff | #111216ff |
| `origin.agent` | köken | #8aaaffff | #2850dcff |
| `origin.engine` | köken | #b9bcc4ff | #272a32ff |

Ajan çizgisinin kesiği: `5.0 3.0`.

#### Tip ölçeği
| rol | px | satır | yazı tipi |
|---|---|---|---|
| `headline` | 19.0 | 1.27 | `atkinson-next-semibold` |
| `title` | 15.0 | 1.3 | `atkinson-next` |
| `body` | 16.0 | 1.375 | `atkinson-next` |
| `label` | 14.0 | 1.3 | `atkinson-next` |
| `figure` | 13.0 | 1.3 | `atkinson-next` |
| `note` | 13.0 | 1.3 | `atkinson-next` |

#### Boşluk, yarıçap, kenar
| değer | kayıt |
|---|---|
| boşluk ölçeği | 4, 8, 12, 16, 20, 28 |
| çizim alanı kenarı (sol, sağ, üst, alt) | 8, 72, 28, 24 |
| panel arası | 10 |
| denetim yarıçapı | 14.0 |
| kart yarıçapı | 14.0 |
| rozet yarıçapı | 10.0 |
| çizgi kalınlığı | 1.0 |
| gölge | `none` |

#### Yoğunluk
| değer | kayıt |
|---|---|
| gövde (`--qp-fs`) | 16.0 |
| küçük / tablo / en küçük | 14.0, 14.0, 13.0 |
| denetim / sekme yüksekliği (0 = içerik) | 44.0, 48.0 |

#### Hareket
| değer | kayıt |
|---|---|
| süre çarpanı | 0.85 |
| eğri | `cubic-bezier(0.2, 0, 0, 1)` |
| kabuk geçişi (ms) | 140.0 |

#### Grafik
| değer | kayıt |
|---|---|
| `axis_gutter_px` | 72.0 |
| `body_ratio` | 0.7 |
| `crosshair_pills` | false |
| `last_value_tag` | true |
| `mark_max_px` | 32.0 |
| `ohlc_legend` | false |
| `price_label_px` | 90.0 |
| `two_level_time` | false |
| `value_line` | none |
| `volume_share` | 0.0 |
<!-- /generated -->
