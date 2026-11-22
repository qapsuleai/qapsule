# Tasarım sözleşmesi (RD2; iskelet)

Bu belge kuralları taşır, sayıyı taşımaz (OV-1). Her değerin sahibi çekirdekteki görünüş
kaydıdır (`crates/qp-grammar/src/look.rs`, `look/*.rs`); kabuk değerleri yalnız `css_vars`
çıktısından okur (`qp look css <ad>`). "Değerler" bölümündeki tablolar kayıttan üretilir
(`scripts/design-contract`); elle yazılmaz, `scripts/design-check` farkı 0 olmadıkça düşer.
Her kural bir `design-check` satırına bağlıdır (P1–P13, H1–H3). Eşikler tek yerdedir:
`crates/qp-grammar/tests/design_check/thresholds.rs`. Eşikler Arkin'in ara kararından
(2026-11-22) sonra gözden geçirildi ve başlangıç değerleriyle tutuldu (değişiklik istenmedi).

Seçilen yön **`tavek`** (RD2 ara karar, `rounds/RD2/DECISION.md`); değer bloğu onun iki
profilini (`tavek`, `tavek-tel`) gösterir. `oltan` R5'in reddedilen görünüşüdür (K57);
kayıtta yalnız karşılaştırma için durur.

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
  kesiklidir (`--qp-agent-dash`), motor çizgisi noktalı, insanınki düz; renk körlüğünde ayrım
  biçimle korunur (H2, H3).
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
- Saat ve tarih etiketleri (zaman ekseni, tarih hapı) görünümün `display_tz` diliminde yazılır
  (K14); fiyat adımı `chart.price_label_px`'e en yakın adım (1 / 1,5 / 2 / 2,5 / 3 / 4 / 5 /
  7,5 × 10ⁿ); her ölçekli bölme en az bir eksen etiketi taşır (kısa bölmede daha ince adımdan).
  Neden bu merdiven: alışılmış 1 / 2 / 2,5 / 5 merdiveninde komşu adımlar 2× uzak; tasarım
  fikstüründe masaüstü etiket aralığı 35 px'e düşüyor (P4 alt sınırı 40). İnce merdivende
  komşular en çok 1,5× uzak, aralık her bölme boyunda hedefin ±%22'sinde kalır (RD2/C2a-fix).
- **Her seviye eksende biter** (`chart.origin_levels`): yatay seviye kökeninin renginde ve
  çizgi biçimindedir ve sağ eksende o renkte dolu bir etiketle (değeri) biter; altındaki fiyat
  etiketi çizilmez, son değer etiketine değen seviye etiketi düşer (P6). Sol uç çipi (kim +
  durum) ve kapsülün kanıt parantezi bu turda maket ve şartnamedir (`mockups/tavek/c2.html`).

### Sabit alfabe ölçüleri (kayıtta değil)

Görünüş kaydında değildir, yönler değiştiremez (satır bütçesi; RD2/C2a-fix). Değerler
`crates/qp-grammar/src/compile/alphabet.rs` `ALPHABET_FIXED` ile aynıdır;
`design_check` (`contract_lists_the_fixed_measures`) eşitliği sınar.

| Ad | Değer | Neden |
|---|---|---|
| `VOLUME_ALPHA` | `0.35` | hacim fiyattan sonra okunur; mumların altında soluk kalır |
| `DASH` | `4, 3` | imleç çizgisi kesik/boşluk (px): veri çizgisinden ayrılır |
| `DOT` | `2, 2` | değer çizgisi nokta/boşluk (px): imleçten ayrı, daha hafif |
| `GAPS` | `3, 8` | lejantta harf–değer ve parça–parça aralığı (px) |
| `PILL_PAD_X` | `6` | etiket, hap ve fiyat etiketi metninin iç boşluğu (px; R0 etiketi) |
| `PILL_PAD_Y` | `6` | hap boyuna figür boyutunun üstüne eklenen (px) |
| `MID` | `0.36` | figürü piksel satırına ortalayan taban çizgisi kayması (em; R0: 11 px'te 4) |
| `BASELINE_PAD` | `5` | zaman etiketi taban çizgisinin etiket boyu ötesindeki payı (px; R0: 11 px'te 16) |
| `LABEL_GAP` | `8` | zaman düzeyi seçilirken iki etiket arası en az ortalama boşluk (px) |
| `PLACE_GAP` | `6` | yerleşen iki zaman etiketi arası en az boşluk (px) |

## Ölçüm

`scripts/design-check` (`crates/qp-grammar/tests/design_check`) her ölçütü ya kayıttan ya
derlenmiş sahneden okur; sahne ölçütleri `fixtures/design/desk.json` görünümünü, imleç
120. bardayken derler (RD2/C2a).

- **P2 (sahne):** çizilen her ilkelin rengi; tam saydam ilkel ve arka plan rengi sayılmaz,
  alfa yok sayılır (ton bir renk rolüdür). CIELAB kroması 6'nın altındaki bütün renkler
  **tek nötr ton** sayılır; kalanlar ΔE00 > 5 ile kümelenir.
- **P4 (sahne):** fiyat etiketlerinin ortanca aralığı (masaüstü) ve 300 px çizim alanındaki
  etiket sayısı.
- **P5 (sahne):** 390 px'te zaman ekseni etiket sayısı; iki düzey = kayıtta
  `two_level_time` **ve** sahnede en az iki renk rolündeki zaman etiketi (küçük / büyük birim).
- **P6 (sahne):** eksen etiketi kutularının çizim alanıyla, başka metinle (son değer
  etiketi, haplar, lejant), ince dikey çizgiyle ve çerçeve kenarıyla kesişimi.
- **P7 (sahne):** son değer etiketi, ilk bölmede noktalı değer çizgisi, imleç (aynı sütunda
  en az üç kesik), oluktaki fiyat hapı ve zaman eksenindeki tarih hapı, başlık satırında
  O H L C (telefon: C).
- **P8 (kayıt + sahne):** `chart.mark_max_px` ≤ 28 ve sahnede çizilmiş en yüksek anlam
  işareti (derleyici işaret boyunu `mark_max_px` ile sınırlar).
- **Seviyeler (sahne, RD2/C2b):** `desk-levels` görünümü (aynı fikstürde: insanın izleme
  seviyesi ve ajanın önerdiği destek, her biri yazarının olgusundan; RSI 70/30 eşikleri,
  motorun) iki temada derlenir: her seviyenin çizgisi köken renginde ve biçiminde (insan tek
  düz çizgi, ajan `agent_dash`, motor `DOT`), eksen etiketi köken renginde dolu; P6 bu
  sahneleri de sayar (`design_check/levels.rs`).
- **H3 (kayıt):** anlam ve köken rollerinin her çifti; yalnız adı konmuş, bilerek paylaşılan
  çiftler (`valid`=`text`, `invalid`=`closed`=`neutral`; `docs/DESIGN.md` §1.6) atlanır.

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

`tavek` (masaüstü, yoğun):

<!-- generated:look=tavek -->
Görünüş `tavek`: profil `dense`, varsayılan düzen `dark`.

#### Renk rolleri
| rol | grup | koyu | açık |
|---|---|---|---|
| `chrome.bg` | krom nötr | #101114ff | #f6f6f7ff |
| `chrome.card` | krom nötr | #18191dff | #ffffffff |
| `chrome.fg` | krom nötr | #e6e7eaff | #18191cff |
| `chrome.muted` | krom nötr | #9699a0ff | #5c5f66ff |
| `chrome.line` | krom nötr | #2c2e33ff | #d6d7daff |
| `chrome.control` | krom nötr | #686b72ff | #80838aff |
| `chrome.page` | krom nötr | #0a0a0cff | #e8e9ebff |
| `s1` | veri | #689ee8ff | #2860c8ff |
| `s2` | veri | #dea048ff | #b86c00ff |
| `s3` | veri | #4ebac8ff | #007c8aff |
| `s4` | veri | #d678aaff | #b03278ff |
| `s5` | veri | #b4ba68ff | #6e7414ff |
| `s6` | veri | #c49678ff | #82563cff |
| `s7` | veri | #8caa8cff | #466e46ff |
| `s8` | veri | #b4b4b8ff | #646468ff |
| `grid` | veri | #1f2125ff | #ecedefff |
| `axis` | veri | #6c6f76ff | #80838aff |
| `up` | anlam | #68c496ff | #148054ff |
| `down` | anlam | #d45c5fff | #961a26ff |
| `warn` | anlam | #e2b254ff | #965c00ff |
| `valid` | anlam | #e6e7eaff | #18191cff |
| `invalid` | anlam | #616369ff | #8c8e94ff |
| `neutral` | anlam | #616369ff | #8c8e94ff |
| `closed` | anlam | #616369ff | #8c8e94ff |
| `focus` | anlam | #689ee8ff | #2860c8ff |
| `origin.human` | köken | #e6e7eaff | #18191cff |
| `origin.agent` | köken | #a896f0ff | #6040c4ff |
| `origin.engine` | köken | #7c7f86ff | #73767dff |

Ajan çizgisinin kesiği: `4.0 3.0`.

#### Tip ölçeği
| rol | px | satır | yazı tipi |
|---|---|---|---|
| `headline` | 16.0 | 1.3 | `atkinson-next-semibold` |
| `title` | 14.0 | 1.3 | `atkinson-next-semibold` |
| `body` | 13.0 | 1.4 | `atkinson-next` |
| `label` | 11.0 | 1.3 | `plex-mono` |
| `figure` | 11.0 | 1.3 | `plex-mono` |
| `note` | 11.5 | 1.3 | `atkinson-next` |

#### Boşluk, yarıçap, kenar
| değer | kayıt |
|---|---|
| boşluk ölçeği | 4, 8, 12, 16, 24, 32 |
| çizim alanı kenarı (sol, sağ, üst, alt) | 8, 64, 28, 24 |
| panel arası | 8 |
| denetim yarıçapı | 6.0 |
| kart yarıçapı | 8.0 |
| rozet yarıçapı | 4.0 |
| çizgi kalınlığı | 1.0 |
| gölge | `0 1px 2px rgba(0, 0, 0, 0.18), 0 4px 12px rgba(0, 0, 0, 0.12)` |

#### Yoğunluk
| değer | kayıt |
|---|---|
| gövde (`--qp-fs`) | 13.0 |
| küçük / tablo / en küçük | 12.0, 12.0, 11.0 |
| denetim / sekme yüksekliği (0 = içerik) | 28.0, 32.0 |

#### Hareket
| değer | kayıt |
|---|---|
| süre çarpanı | 0.9 |
| eğri | `cubic-bezier(0.2, 0, 0, 1)` |
| kabuk geçişi (ms) | 120.0 |

#### Grafik
| değer | kayıt |
|---|---|
| `axis_gutter_px` | 64.0 |
| `body_ratio` | 0.72 |
| `crosshair_pills` | true |
| `last_value_tag` | true |
| `mark_max_px` | 20.0 |
| `ohlc_legend` | true |
| `origin_levels` | true |
| `price_label_px` | 48.0 |
| `two_level_time` | true |
| `value_line` | dotted |
| `volume_share` | 0.18 |
<!-- /generated -->

`tavek-tel` (telefon, dokunma):

<!-- generated:look=tavek-tel -->
Görünüş `tavek-tel`: profil `touch`, varsayılan düzen `dark`.

#### Renk rolleri
| rol | grup | koyu | açık |
|---|---|---|---|
| `chrome.bg` | krom nötr | #101114ff | #f6f6f7ff |
| `chrome.card` | krom nötr | #18191dff | #ffffffff |
| `chrome.fg` | krom nötr | #e6e7eaff | #18191cff |
| `chrome.muted` | krom nötr | #9699a0ff | #5c5f66ff |
| `chrome.line` | krom nötr | #2c2e33ff | #d6d7daff |
| `chrome.control` | krom nötr | #686b72ff | #80838aff |
| `chrome.page` | krom nötr | #0a0a0cff | #e8e9ebff |
| `s1` | veri | #689ee8ff | #2860c8ff |
| `s2` | veri | #dea048ff | #b86c00ff |
| `s3` | veri | #4ebac8ff | #007c8aff |
| `s4` | veri | #d678aaff | #b03278ff |
| `s5` | veri | #b4ba68ff | #6e7414ff |
| `s6` | veri | #c49678ff | #82563cff |
| `s7` | veri | #8caa8cff | #466e46ff |
| `s8` | veri | #b4b4b8ff | #646468ff |
| `grid` | veri | #1f2125ff | #ecedefff |
| `axis` | veri | #6c6f76ff | #80838aff |
| `up` | anlam | #68c496ff | #148054ff |
| `down` | anlam | #d45c5fff | #961a26ff |
| `warn` | anlam | #e2b254ff | #965c00ff |
| `valid` | anlam | #e6e7eaff | #18191cff |
| `invalid` | anlam | #616369ff | #8c8e94ff |
| `neutral` | anlam | #616369ff | #8c8e94ff |
| `closed` | anlam | #616369ff | #8c8e94ff |
| `focus` | anlam | #689ee8ff | #2860c8ff |
| `origin.human` | köken | #e6e7eaff | #18191cff |
| `origin.agent` | köken | #a896f0ff | #6040c4ff |
| `origin.engine` | köken | #7c7f86ff | #73767dff |

Ajan çizgisinin kesiği: `4.0 3.0`.

#### Tip ölçeği
| rol | px | satır | yazı tipi |
|---|---|---|---|
| `headline` | 19.0 | 1.3 | `atkinson-next-semibold` |
| `title` | 17.0 | 1.3 | `atkinson-next-semibold` |
| `body` | 16.0 | 1.4 | `atkinson-next` |
| `label` | 13.0 | 1.3 | `plex-mono` |
| `figure` | 13.0 | 1.3 | `plex-mono` |
| `note` | 13.5 | 1.3 | `atkinson-next` |

#### Boşluk, yarıçap, kenar
| değer | kayıt |
|---|---|
| boşluk ölçeği | 4, 8, 12, 16, 24, 32 |
| çizim alanı kenarı (sol, sağ, üst, alt) | 8, 56, 28, 24 |
| panel arası | 12 |
| denetim yarıçapı | 6.0 |
| kart yarıçapı | 8.0 |
| rozet yarıçapı | 4.0 |
| çizgi kalınlığı | 1.0 |
| gölge | `0 1px 2px rgba(0, 0, 0, 0.18), 0 4px 12px rgba(0, 0, 0, 0.12)` |

#### Yoğunluk
| değer | kayıt |
|---|---|
| gövde (`--qp-fs`) | 16.0 |
| küçük / tablo / en küçük | 14.0, 14.0, 13.0 |
| denetim / sekme yüksekliği (0 = içerik) | 44.0, 48.0 |

#### Hareket
| değer | kayıt |
|---|---|
| süre çarpanı | 0.9 |
| eğri | `cubic-bezier(0.2, 0, 0, 1)` |
| kabuk geçişi (ms) | 120.0 |

#### Grafik
| değer | kayıt |
|---|---|
| `axis_gutter_px` | 56.0 |
| `body_ratio` | 0.72 |
| `crosshair_pills` | true |
| `last_value_tag` | true |
| `mark_max_px` | 24.0 |
| `ohlc_legend` | true |
| `origin_levels` | true |
| `price_label_px` | 48.0 |
| `two_level_time` | true |
| `value_line` | dotted |
| `volume_share` | 0.18 |
<!-- /generated -->
