# Qapsule Görsel Grameri — v0

Durum: **v0** (R2). Tiplerin tek kaynağı `crates/qp-schema/src/grammar.rs` (görünüm) ve
`crates/qp-schema/src/scene.rs` (sahne tarifi); JSON Schema oradan üretilir
(`qp schema export`: `View`, `SceneDesc`, protokol şemaları). Bu belge ile şema çelişirse
şema doğrudur ve bu belge düzeltilir. Protokoldeki yeri: `docs/PROTOCOL.md` §5, §17.

Gramer, `qp/0` protokolünün **kelime dağarcığıdır**: ajan ya da arayüz bir görünümü komut
mesajlarıyla kurar (`put_view`, `insert_layer`, `set_encoding`…), otorite görünümü
dokümanda saklar, gramer derleyicisi (`qp-grammar`) görünümü ve veriyi bir **sahne
tarifine** çevirir; çizici (`qp-render`) ve anlamsal okuma (`screen`, `hit`) aynı tarifi
okur. Mum grafiği bu gramerin yalnızca bir cümlesidir (ilke 8).

```
 komutlar ──► otorite ──► doküman (View) ──┐
                 │                          ▼
                 └─ qp-compute ─ değerler ─► qp-grammar ─► sahne tarifi ─┬─► qp-render (PNG, tarayıcı)
                                                                          └─► screen, hit (ekransız)
```

Örneklerdeki kimlikler (`px`, `rsi14`, `three`…) örnek dokümanlarındır; gerçek bir
dokümanın kimlikleri `welcome.summary`'den okunur (§12).

## 1. Görünüm (`View`)

Bir görünüm üstten alta yığılmış **panellerden** oluşur; paneller tek bir **zaman eksenini**
paylaşır; her panelin kendi **değer ölçeği** vardır. **Katmanlar** sırayla çizilir.

```json
{"id":"three","x":{"kind":"time"},
 "panels":[{"id":0,"name":"price","y":{"kind":"linear"}},
           {"id":1,"name":"rsi","y":{"kind":"linear","domain":[0,100]}}],
 "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
           {"id":"rsi","mark":"line","series":"rsi14","panel":1}],
 "emphasis":[]}
```

| Alan | Anlamı |
|------|--------|
| `id` | Doküman genelinde tek. |
| `title` | Başlık; yoksa ilk bar serisinin sembolü ve periyodu (`SAMPLE 1d`). |
| `theme` | Tema adı (§9); yoksa `dark`. |
| `x` | Zaman ölçeği (§3). `x.domain` görünümün **kalıcı aralığıdır**. |
| `panels` | Paneller, üstten alta (§2). |
| `layers` | Katmanlar, çizim sırasıyla (sonraki üstte) (§4). |
| `emphasis` | Kalıcı vurgu (§8). |
| `y` | **Kullanımdan kalktı** (`qp/0`): panellerin ölçekleri konum sırasıyla. Otorite yüklerken `panels`'a çevirir (§11). |

Görünümün **ilk bar serisi**: katman sırasıyla ilk `candle` katmanının serisi; yoksa
verisi bar serisi olan ilk katmanın serisi; yoksa bir hesap serisinin girdisi olan bar
serisi. Bar numaraları (`from_bar`, `to_bar`, `tap`) ona göredir.

## 2. Panel (`Panel`)

```json
{"id":2,"name":"macd","weight":1,"y":{"kind":"linear","id":"y2"}}
```

- `id`: kalıcı panel **numarası**; konum değildir. Panel taşınsa, araya panel girse de
  değişmez; katmanlar ona `panel` alanıyla başvurur. Verilmezse en büyük id + 1.
  `qp/0` dokümanlarında id = eski konum (0, 1, 2…), böylece eski katmanların `panel`
  değerleri anlamını korur.
- `name`: insan ve anlamla başvuru için ad (`price`, `rsi`). İsteğe bağlı.
- `weight`: göreli yükseklik (> 0); varsayılan ilk panel 3, diğerleri 1.
- `y`: panelin değer ölçeği; kimliği `y<id>` (`y0`, `y2`).

Panel işlemleri kimlikle: `insert_panel {view, panel, layers?, after?|before?}`,
`delete_panel {view, panel}` (paneldeki katmanlarla birlikte), `move_panel {view, panel,
after?|before?}`, `set_panel {view, panel, name?, weight?}`.

## 3. Ölçekler (`Scale`)

| `kind` | Eksen | Eşleme |
|--------|-------|--------|
| `time` | yatay | **Bar sırası.** Her bara bir yuva; seans boşlukları (gece, hafta sonu, tatil) yer kaplamaz. `domain` epoch ms `[t0, t1]`. |
| `linear` | dikey | `y = top + (max − v)/(max − min) · h` |
| `log` | dikey | aynısı `log10(v)` üzerinde; `v ≤ 0` çizilmez, sayılır (`skipped`). |

- Ölçek kimlikleri: zaman ekseni `x`, panel ölçeği `y<panel id>`. Değişiklik:
  `set_scale {view, scale, kind?, domain?, auto_domain?}`.
- **Otomatik alan:** sabit `domain` yoksa paneldeki görünen bütün öğelerin değerleri
  (mumda `l`/`h`, çizgide `y`, bant ve alanda `y`/`y2`, çubukta `y` ve taban, kuralların
  sabitleri) üzerinden en küçük/en büyük; iki yana %5 pay (açıklık en az `1e-9`); log
  ölçekte pay log uzayında.
- **Kalıcı aralık ile geçici bakış ayrıdır (G6).** `x.domain` komutla değişir
  (`set_view_range`, `set_scale x`), günlüğe girer, herkes için aynıdır. Bir bağlantının
  kaydırma/yakınlaştırması (`input`) varlık durumudur, günlüğe girmez. Görünen aralık:
  bağlantının bakışı varsa o; yoksa `x.domain`'e düşen barlar; yoksa bütün barlar.
- **Barların ötesi:** nokta serilerinin son bardan sonraki noktaları (ör. tahmin yelpazesi)
  ardışık seanslar sayılır: son bardan sonraki k'inci nokta `son bar + k` yuvasına yerleşir.
  Görünen aralık bu yuvaları kapsayacak kadar sağa uzar (yalnız aralık belirtilmemişse).
  **Bakış gelecek yuvalara uzanır (K25):** görünümün son yuvası = son bar + gelecek yuva
  sayısı. Bir bağlantının bakışı bu son yuvaya kadar gidebilir: `look` (ve `zoom`'un
  `anchor_bar`'ı) son yuvaya kadar kabul edilir, sonrası `invalid`; `pan`/`zoom` son yuvaya
  kırpılır. Ekranlı bağlantının **varsayılan bakışı** son N barı ve ardından bütün gelecek
  yuvaları kapsar (masaüstünde 200 bar + 20 tahmin noktası → yuva 80..219); `x.domain`
  verilmişse zamanı alana düşen yuvalar (gelecek yuvalar dahil, derleyiciyle aynı kural).
  Böylece `screen` ve `hit` ekranlı bağlantılarda da gelecek yuvalardaki öğeleri (yelpaze
  bantları) gösterir; ekransız bağlantılar, `qp render` ve `qp explain` zaten gösteriyordu.
  Veri kısalırsa (geri alınan `append_bars`) kayıtlı bakış güncel son yuvaya kırpılır.
  `tap` ve kalıcı aralık (`set_view_range`) barlarla sınırlı kalır. Döküm: `29-future-look`.

## 4. Katman (`Layer`)

```json
{"id":"hist","mark":"bar","series":"macd","panel":2,
 "encoding":{"y":"histogram",
   "color":{"when":{"field":"histogram","op":"ge","than":0},"then":"up","else":"down"}},
 "origin":{"/encoding/color/when/than":{"kind":"threshold","name":"macd.zero"}}}
```

| Alan | Anlamı |
|------|--------|
| `id` | Görünüm içinde tek, kalıcı. Verilmezse en küçük kullanılmayan `l<k>`. |
| `mark` | İşaret (§5). |
| `series` / `fact` / `finding` | Veri (§6); en çok biri. Hiçbiri yoksa katman yalnız kökenli sabitler çizer. |
| `panel` | Panel **id**'si. |
| `encoding` | Kodlama kanalları (§7). |
| `origin` | Sabitlerin kökeni (§10), katman içindeki JSON göstericisiyle. |
| `label` | Açıklama metni; yoksa veriden (`SMA20`, `RSI14`, `MACD(12,26,9)`). |

Katman işlemleri kimlikle: `insert_layer {view, layer, after?|before?}`,
`delete_layer {view, layer}`, `split_layer {view, layer}` (çok çıktılı `line`'ı çıktı başına
katmana böler; tek bandı ayrı boyamak için), `move_layer {view, layer, panel?, after?|before?}`,
`set_encoding {view, layer, encoding, unset?, origin?, label?}` (verilen kanallar
değişir, ötekiler kalır). `add_layer.index` ve `remove_layer` kullanımdan kalktı.

## 5. İşaretler (`Mark`)

| İşaret | Gereken | Varsayılan | Örnek |
|--------|---------|------------|-------|
| `candle` | bar serisi | `o h l c`; renk `up` (c ≥ o) / `down` | `{"mark":"candle","series":"px"}` |
| `bar` | `y` | taban `y2` ya da 0 (panele kırpılır); genişlik mum gövdesi | MACD histogramı, hacim `{"y":"v"}` |
| `line` | `y` | çok çıktılı hesapta `y` yoksa her **çizim çıktısı** için bir çizgi; kalınlık 1,5 | `{"mark":"line","series":"sma20"}` |
| `area` | `y` | alt kenar `y2`, yoksa panelin altı; opaklık 0,25 | kapanış alanı |
| `band` | `y`, `y2` | opaklık 0,2; kenar çizgisi yalnız `width` verilirse | Bollinger, yelpaze |
| `rule` | `y` sabit (yatay) ya da `x` sabit (dikey) | renk `muted`, kalınlık 1 | 70 seviyesi |
| `point` | `y` | `x` = satırın barı; çap 6 | olgu barı |
| `text` | `text`, `y` | `x` = satırın barı; boyut 11 | olgu metni |

- **Çizim çıktıları:** `bollinger` → `upper`, `middle`, `lower`; `macd` → `macd`, `signal`;
  tek çıktılı hesap → o çıktı. (`bandwidth`, `histogram` yalnız açıkça `y` ile.)
- **Boşluk (`null`):** ısınma barlarında değer yoktur; çizgi ve bant orada kesilir
  (parçalara bölünür), çubuk ve nokta çizilmez. Kırpma yapılmaz (K13).
- **Belirsizlik yelpazesi** ayrı bir işaret değildir: üst üste, artan opaklıklı `band`
  katmanlarıyla kurulur (bkz. `rounds/R2/tasks/VIEWS.md` V3).

## 6. Veri başvurusu

Gramer veriyi **taşımaz, başvurur** (K23): katman bir seriye, olguya ya da bulguya
kimliğiyle bağlanır; değerleri otorite `qp-compute` ile hesaplar ve derleyiciye verir.

| Veri | Satırlar | Alanlar | Ana alan |
|------|----------|---------|----------|
| bar serisi | her bar | `o h l c v` | `c` |
| hesap serisi | her bar | çıktılar (`rsi`; `upper middle lower bandwidth`; `macd signal histogram`) | hesabın adıyla aynı çıktı (`bollinger`: `middle`) ya da `source.output` |
| nokta serisi | her nokta | `v` | `v` |
| olgu (`fact`) | tek satır | `bar t value text` + kanıt anahtarları (`level`) | `level` varsa o, yoksa `value` |
| bulgu (`finding`) | olgusu başına | olgu alanları + `claim confidence` | olgununki |

Olgunun `value` alanı, olgunun serisinin ana çıktısının olgu barındaki değeridir.

**Başka bir serinin alanı:** kanal değeri `{"series":"q95","field":"v"}` olabilir; satırlar
bar sırasıyla eşlenir (aynı bar yuvası). İki seri arası bant böyle kurulur:

```json
{"mark":"band","series":"q05","encoding":{"y":"v","y2":{"series":"q95","field":"v"}}}
```

## 7. Kodlama kanalları (`Encoding`)

Kapalı ve tipli küme; bilinmeyen kanal `schema` hatasıyla reddedilir (`did_you_mean`).
RD1/T6e'de serbest kanal adları modelin tanımsız kanallar yazmasına yol açmıştı.

| Kanal | Değer | Not |
|-------|-------|-----|
| `x` | alan adı · sayı · `{"series","field"}` | Varsayılan: satırın barı. Sabit → dikey kural. |
| `y` | alan adı · sayı · `{"series","field"}` | Varsayılan: ana alan. `"70"` gibi sayı yazılı dizge de sabittir (`qp/0` biçimi). |
| `y2` | aynı | Bant/alan alt kenarı, çubuk tabanı. |
| `color` | renk · koşullu · basamaklı | Aşağıda. |
| `opacity` | `0..1` | Rengin alfasıyla çarpılır. |
| `width` | piksel | Çizgi, kural, bant kenarı. |
| `size` | piksel · `{"field","range":[a,b]}` | Nokta çapı, metin boyu. |
| `text` | alan adı | `text` işareti (olgunun `text`'i, bulgunun `claim`'i). |

**Renk** üç biçimde:

```json
"color": "#ef5350"                                  // sabit (ya da tema adı: "down")
"color": {"when":{"field":"c","op":"ge","than":{"series":"sma50","field":"sma"}},
          "then":"up","else":"down"}               // koşullu, bar başına
"color": {"field":"rsi","thresholds":[30,70],"colors":["up","muted","down"]}   // değere bağlı
```

- Koşul: `field op than`; `op` ∈ `lt le gt ge`; `than` sayı (köken gerekir) ya da alan.
  Bir taraf `null` ise `else` rengi.
- Basamaklı: `colors[i]`, `thresholds[i−1] ≤ değer < thresholds[i]`; `colors` uzunluğu
  `thresholds` + 1; eşikler artan; her eşiğin kökeni gerekir.
- **Renk değişen çizgi ve alan** parçalara bölünür; iki komşu bar arasında renk değişiyorsa
  bölünme noktası kesişimin doğrusal ara değeridir (koşul alan–alan ya da alan–sabitse;
  temel işlemlerle), değilse iki barın ortası.

## 8. Vurgu (`Emphasis`) ve hedef

```json
{"target":{"item":"fnd-9e1"},"kind":"highlight"}
{"target":{"layer":"rsi"},"kind":"focus"}
{"target":{"panel":0,"from_bar":180,"to_bar":199},"kind":"dim"}
```

Hedef (`Target`) dikkat yönergesi ve anlatım adımıyla aynıdır: `view`, `panel`, `layer`,
`series`, `output`, `item`, `from_bar`, `to_bar`. `item` bir olgu ya da bulguysa barları
olgu barlarıdır (bulguda en küçük–en büyük). Çizim:

- `highlight`: hedef barların arkasında `accent` renkli yarı saydam şerit (hedef panellerde;
  panel verilmemişse bütün panellerde); hedef bir katmansa çizgisi 1 px kalın.
- `focus`: hedef dışındaki her şeyin üstüne arka plan renginde yarı saydam örtü.
- `dim`: hedefin üstüne aynı örtü.

Kalıcı vurgu `set_emphasis {view, emphasis}` ile. Dikkat yönergesi (`attend`) ve anlatım
adımı aynı hedefle **geçici** vurgu olarak çizilir (derleyiciye ayrıca verilir; dokümana
yazılmaz).

## 9. Tema

Tema görünümden ayrıdır: görünüm yalnız tema adını ve **tema renk adlarını** taşır;
değerler motordadır. Aynı görünüm başka temayla çizilebilir.

| Ad | `dark` (varsayılan; R0 renkleri) |
|----|------|
| `bg` | `#101218` |
| `grid` | `#222630` |
| `axis` | `#545a68` |
| `text` | `#c8cdd7` |
| `muted` | `#828896` |
| `up` | `#26a69a` |
| `down` | `#ef5350` |
| `accent` | `#ffb74de6` (R0'daki SMA çizgisi) |
| `warn` | `#ffca28` |
| `s1`…`s8` | `accent`, `#42a5f5`, `#ab47bc`, `#66bb6a`, `#ec407a`, `#26c6da`, `#8d6e63`, `#bdbdbd` |

`light` teması aynı adlarla açık zeminli değerler verir. Renk değeri `#rrggbb`,
`#rrggbbaa` ya da yukarıdaki adlardan biri olmalı; değilse `invalid` (yakın adlar
`did_you_mean`'de). Rengi verilmemiş çizgi/alan/bant katmanları panel içindeki sırayla
`s1`, `s2`… alır (mumlar sayılmaz).

### 9a. Görünüş (`Look`, R5/T3; Ö44)

Görünüş, bir görsel yönün **bütün tasarım değerlerini** taşıyan çekirdek kaydıdır
(`crates/qp-grammar/src/look.rs`, `look::LOOKS`). Tema (§9) görünüşün bir renk düzenidir:
her görünüşün bir `dark` ve bir `light` teması vardır. Bağlantı görünüşü `style`
hareketinin `look` alanıyla seçer (`ViewState.look`; yoksa `r4`); derleme seçeneği
`CompileOptions::look`. Görünüm (§1) görünüş adı taşımaz: aynı görünüm her görünüşle
çizilebilir. Kayıt saf veridir; aynı görünüm, veri ve görünüş aynı sahneyi verir.

| Alan | İçerik | `r4` |
|------|--------|------|
| `types` | Tip ölçeği, roller: `headline` (başlık cümlesi), `title` (sahnenin üst satırı: grafik başlığı ve kapanış; yüzün ek bilgisi), `body` (kart cümlesi, grafik notu), `label` (tarih etiketi, açıklama, karne satırı), `figure` (eksen değeri, son değer etiketi), `note` (kanıt etiketi). Her rol: boy (px), satır yüksekliği (boyun katı), yazı tipi varlığı | 20 / 16 / 12 / 11 / 11 / 9 px; satır 1,3; hepsi `plex-mono` |
| `dark`, `light` | Renk rolleri (`Theme`): §9'daki adlar + `surface` (yüzey), `line`, `neutral` (yönsüz işaret), `valid`, `invalid` (artık geçerli değil: işaret ve cümle), `closed` (kapanmış işaret), `focus` (odak); `chrome` (kabuğun kendi renkleri: zemin, metin, ikincil metin, çizgi, kart, vurgu, vurgu üstü metin, sayfa, bildirim ve hata şeridi, "yetki gerekir") | §9 değerleri; `neutral`=`accent`, `valid`=`text`, `invalid`=`closed`=`muted`; `chrome` = R4 `style.css` renkleri |
| `scheme` | Okur seçmediğinde düzen | `dark` |
| `space` | Boşluk ölçeği (kabuk: `--qp-space-0..5`), çizim alanı kenar boşlukları (sol, sağ, üst = başlık satırı, alt), panel arası, işaret ile cümle arası | `[4,6,8,12,16,24]`, `[8,72,34,24]`, 8, 10 |
| `surface` | Kabuk köşe yarıçapı, telefon çerçevesi ve rozet yarıçapı, çizgi kalınlığı, gölge (yok = düz), vurgu şeridi ve perde saydamlığı | 6, 18, 8 px; 1; düz; 56, 160 |
| `glyph` | İşaretin boyu: yüzde, kartta, grafikte, boyu verilmemiş katmanda; yüzdeki yeri (`leading`: cümlenin solunda, `above`: üstünde) | 32, 40, 32, 24; `leading` |
| `motion` | Geçiş süresi çarpanı, eğri (`linear` ya da `cubic-bezier`, sabit adımlı çözüm), kabuk geçiş süresi | 1, `linear`, 0 ms |

**Okuyanlar.** Derleyici (kenar boşlukları, panel arası, başlık/etiket/rakam boyu,
genişlik tahmini, satır yüksekliği, şerit ve perde), yüz tasarımları (`FaceViewFn` görünüşü
alır; yüz yüksekliği `faces::height_in(look, face)`), kart ve grafik kuralları
(`depth::capsule_view_in`, `chart_view_in`), işaret alfabeleri (renk rolleri), geçiş
ilerlemesi (`transitions::progress_in`), kabuk (CSS özel özellikleri, aşağıda).

**Yazı tipi.** Bir varlık (`look::FONTS`) tek aile ve tek ağırlıklı **statik TTF**
dosyasıdır (fontdue değişken yazı tipinin yalnız varsayılan örneğini çizer, woff2
okumaz): kimlik, aile, ağırlık, dosya, lisans dosyası, SPDX, tuvaldeki CSS ailesi,
ortalama ilerleme (em; derleyicinin genişlik tahmini) ve yükselme (em). Sahnedeki metin
varlığını `Prim::Text::font` ile adlandırır (yoksa varsayılan `plex-mono`; `r4`
sahneleri bu alanı hiç yazmaz). Başvuru çizici (`qp_render::fonts`), `canvas` yolu
(`FontFace`, baytlar çekirdekten: `core.fonts()`, `core.font_bytes(id)`; ağ yok) ve `gpu`
yolu aynı dosyayı kullanır. Tuval `fontKerning = "none"` ile çizer: fontdue GPOS aralığı
uygulamaz, iki yol aynı yerleşimi verir. fontdue şekillendirme yapmaz (`tnum`
uygulanamaz): tablo rakamı gereken aile ya varsayılan rakamları tablo olan bir aile
olmalı ya da dosya derleme öncesi cmap'te 0–9 tablo gliflerine yeniden eşlenmeli
(`rounds/R5/RESEARCH.md`).

**Roller sahnede.** `CompileOptions::roles` (ve `qp scene --roles`) her metne rol adını
yazar (`Prim::Text::role`): derleyicinin metinleri kendi rolüyle; görünümün metin
katmanları boylarına göre (`headline`, `body`, `label`, `note` boyları her görünüşte
farklıdır, kayıt sınaması); işaretin metinleri `glyph`. Varsayılan kapalı: ürün
sahneleri ve özetleri değişmez. Ölçüm düzeneği öğe başına rengi, boyu ve rolü sahneden,
rolün yazı tipini ve renk rollerini `qp look show <ad>`'dan okur.

**Kabuk.** Kabuğun renkleri, yazı ailesi, yarıçapları ve geçiş süresi çekirdekten CSS
özel özelliği olarak gelir (`look::css_vars`, `core.css()`; `qp look css <ad>`);
`style.css` yalnız `var(--…)` okur, renk değeri taşımaz.

**Yeni görünüş ekleme (M16 alt ajanları için tarif):**

1. `crates/qp-grammar/src/look.rs`'te `M16 looks` işaretinin altına bir `const` `Look`
   yaz (`..R4` ile başla, değişeni yaz; ad `[a-z][a-z0-9_-]*`, sıradan bir kelime değil);
   `LOOKS` dizisine bir satır ekle. `headline`, `body`, `label`, `note` boyları farklı olmalı.
2. Renk: iki temayı (`dark`, `light`) ve her birinin `chrome`'unu ver. Karşıtlık
   (metin ≥ 4,5:1, büyük metin ve işaret ≥ 3:1, `closed` dahil) ölçüm düzeneğinde sınanır.
3. Yeni yazı tipi gerekiyorsa: statik TTF'yi (ağırlık başına bir dosya) ve lisansını
   (`OFL.txt` ya da eşdeğeri; aile başına ayrı adla, ör. `OFL-Inter.txt`)
   `crates/qp-render/assets/fonts/`'a koy; `look::FONTS`'a bir `FontAsset` satırı
   (kimlik, aile, ağırlık, dosya, lisans, SPDX, `css_family` benzersiz, ölçülmüş ortalama
   `advance` ve `ascent`), `qp_render::fonts::FILES`'a aynı sırayla bir `include_bytes!`
   satırı ekle; `cargo test -p qp-render --test fonts` iki listenin, dosyanın, lisansın ve
   Türkçe harflerin tuttuğunu sınar. Ayrılmış yazı tipi adı (RFN; Plex, Source Sans 3)
   taşıyan bir dosyanın **alt kümesi alınırsa aile yeniden adlandırılmalıdır**. Paket
   boyutu: her dosya WASM paketine aynen girer (`plex-mono` 135 580 bayt); `verify.sh`
   `wasm` adımının boyut notunda önce/sonra yaz.
4. Gösterimde `?look=<ad>` ya da Ayarlar → Görünüş; `qp look show <ad>` kaydı yazar.
5. `r4` altınları değişmemeli (`scripts/verify.sh`); yeni görünüş için altın eklenmez
   (round'un `v2` kararı ayrı).

**Kayıt dışında kalanlar** (bilinen): derleyicinin küçük yerleşim ofsetleri (başlık
taban çizgisi 23, kapanış metni x+140, etiket ofsetleri, ay etiketi aralığı 72 px), mum
gövdesi oranı, kanıt noktası çapı (9 px), işaret alfabelerinin iç oranları, `tavrik` ve
`fotolay` yüzlerinin kendi boyları (elenen M8 tasarımları; renkleri görünüşten), geçiş
tasarımlarının kendi süreleri (görünüş yalnız çarpar) ve `tutamir`'in iğne boyu, grafik
derinliğindeki işaret boyu (sunum görünümü protokolde taşındığı için `r4`), kabuğun
yazı boyları ve iç boşlukları.

## 10. Köken (`Origin`)

İlke 6: ekrandaki her işaret deterministik kaynağına kadar izlenebilir. Seriye ya da olguya
bağlı kanallar kökenini zaten taşır. **Sayısal veri sabitleri** — konum kanallarındaki
sayılar (`y: 70`), koşul eşikleri, basamak eşikleri — bir kökene bağlanmak zorundadır:

| Köken | Örnek | Değer |
|-------|-------|-------|
| olgu alanı | `{"kind":"fact","fact":"fact-rsi-cross70","field":"level"}` | olgunun kanıt anahtarı ya da `bar`/`t`/`value` |
| adlandırılmış eşik | `{"kind":"threshold","name":"rsi.overbought"}` | motorun listesi (`describe` → `thresholds`) |
| seri değeri | `{"kind":"series","series":"px","field":"c","at":"last"}` | canlı: son bardaki değer; sabit **yazılmaz** |

Adlandırılmış eşikler (v0): `rsi.overbought` 70, `rsi.oversold` 30, `rsi.mid` 50,
`macd.zero` 0.

**Kural:**

1. Köken `layer.origin` içinde, sabitin katman içindeki JSON göstericisiyle durur:
   `/encoding/y`, `/encoding/y2`, `/encoding/x`, `/encoding/color/when/than`,
   `/encoding/color/thresholds/<i>`.
2. Kökeni verilmemiş bir sabit için otorite **adlandırılmış eşiği** arar: sabitin
   panelinde çizilen hesap serilerinin eşikleri arasında değeri **tam olarak** eşit tek bir
   ad varsa kökeni yazar (RSI panelinde `30` → `rsi.oversold`). Olgulara otomatik bağlamaz.
3. Bulamazsa komut `no_origin` hatasıyla reddedilir; ileti katmanı, alanı ve nedeni söyler,
   `candidates` panelin eşiklerini ve paneldeki serilere ait sayısal kanıtlı olguları
   (yakın değerli önce) verir:

```json
{"type":"error","id":"c4","error":{"code":"no_origin",
 "message":"layer `l5` of view `main`: constant 123.45 at /encoding/y has no origin",
 "path":"/ops/0/layer/encoding/y",
 "candidates":[{"id":"{\"kind\":\"fact\",\"fact\":\"f-brk-2\",\"field\":\"level\"}","about":"breakout level 121.30 (bar 185)"}],
 "hint":"Bind it to a fact (add one with add_fact, evidence {\"level\":123.45}) or a named threshold, via the layer's `origin`."}}
```

4. Köken verildiyse çözülmelidir (olgu var, alan sayısal; eşik adı biliniyor; seri var).
   Sabit de yazılmışsa kökenin değerine **bit düzeyinde** eşit olmalı; `series` kökeninde
   sabit yazılmaz. Sabit yazılmayıp köken verildiyse otorite değeri sabit olarak yazar.
5. Stil sabitleri (renk, opaklık, kalınlık, boyut) veri iddiası değildir, köken istemez.
6. Kural kullanımdan kalkan `add_layer` için de geçerlidir; yüklenen dokümanlara da
   uygulanır (yüklemede kökeni bulunamayan sabit dokümanı geçersiz kılar).

## 11. Kanonik biçim ve `qp/0` uyumu

Otorite her görünümü yüklerken ve her yazıda kanonik biçime getirir:

1. `y` var, `panels` yoksa: `panels[i] = {id: i, y: y[i]}`; `y` boşaltılır. İkisi birden
   → `invalid`. İkisi de yoksa: katmanların `panel` değerlerinden, artan, `linear`.
2. Ölçek kimlikleri: `x`, `y<panel id>`.
3. Katman kimliği yoksa en küçük kullanılmayan `l<k>` (katman sırasıyla).
   **Tek ad kuralı (K25):** kimlikli katman kimliğini korur; otorite hiçbir katmanı yeniden
   adlandırmaz. Sunucunun (presenter) kurduğu görünümler gramerin derinlik kurallarının
   verdiği adları taşır (§13: grafik görünümü `candles`, `line`, `level`; kapsül görünümü
   `line`, `level`, `fact`) — `show_finding` ile dokümana yazılan görünümde, `present`
   cevabında, `screen`/`hit`/özet/seçici çözümünde ve dökümlerde aynı ad. `l<k>` yalnız
   **kimliksiz eklenen** katmanlar içindir (yükleme, `add_layer`, `insert_layer`/
   `insert_panel` kimliksiz katman, `split_layer`'ın yeni katmanları: en küçük kullanılmayan
   `l<k>`, bölünen kimlik yeniden kullanılmaz). R2'deki `l0…l2` yeniden adlandırması
   kaldırıldı.
4. Sabitlerin kökeni (§10, kural 2).
5. `layer.panel` var olan bir panel olmalı (`not_found`, `did_you_mean`: panel id'leri ve
   adları).

Böylece `qp/0` dokümanları ve mesajları anlamını korur: eski `{"mark":"rule","panel":1,
"encoding":{"y":"70"}}` hâlâ geçerlidir ve kökeni `rsi.overbought` olarak yazılır; RD1'in
uygunluk dökümleri değişmeden geçer.

## 12. Doküman özeti ve anlamla başvuru

`welcome.summary` (`DocSummary`), ajanın ilk mesajında neye başvurabileceğini görmesi
içindir: seriler (kimlik, tür, hesap ve parametreler, alanlar, açıklama metni), görünümler →
paneller → katmanlar (kimlik, işaret, ne çizdiği: `rsi14.rsi line`, `rule 70
(rsi.overbought)`), olgular, bulgular (en son bar). Anlamla başvurunun seçilen tasarımı:
`docs/PROTOCOL.md` §17.

## 13. Derinlik kuralları ve yüzey indirgemesi (G4)

Sunucu (presenter) aynı bulgu için üç derinlikte deterministik çıktı üretir; kurallar
`qp-grammar`'dadır, otorite onları uygular:

| Derinlik | Çıktı |
|----------|-------|
| `sentence` | Metin: kapsül başlığı, yoksa bulgunun iddiası. Görsel yok. |
| `capsule` | Tek panelli görünüm: olgunun serisinin ana çıktısı (bar serisiyse `c`) **son `max_points` bar** çizgi; olgu seviyesi varsa kural (köken: olgu); olgu barı görünürse nokta (veri: olgu). Başlık metni ve güven. |
| `chart` | Sunucunun görünümü: panel 0 olgunun bar serisinin mumları (`candles`); olgu bir hesap serisindeyse panel 1 onun çizgisi (`line`) ve olgu seviyesinde kural (`level`, köken: olgu); `x.domain = [t(max(0, olgu barı − 30)), t(son bar)]`; olgu barına `highlight` vurgusu. |

**Yüzey indirgemesi:** profil (`docs/PROTOCOL.md` §10.1) derinliği seçer; `max_points`
kapsülün bar sayısını, `max_glyphs` işaret sayısını sınırlar; `series_data: false` ise
kapsül görünümü verilmez, yalnız metin. Grafikte görünen bar sayısı `max_points`'i aşarsa
ayrıntı düzeyi (§15) devreye girer.

## 14. Sahne tarifi (`SceneDesc`)

Derleyicinin çıktısı; çizimden bağımsız, **tek doğruluk kaynağı**.

```
SceneDesc { view, width, height, background, x: XMap, bars, lod?,
            panels: [ScenePanel { id, name?, rect, y: YMap,
                                  layers: [SceneLayer { id, mark, data, fields, constants,
                                                        label?, items: [Item], prims }] }],
            emphasis, under: [Prim], over: [Prim] }
Item { bar?, t?, values: {alan: değer}, hit: PixelRect, bars? }
Prim = rect | polyline | fill | circle | text
```

- Boyama sırası: `under` (vurgu şeritleri, ızgara, eksen çizgileri, eksen etiketleri) →
  panellerin katmanları (sırayla) → `over` (son değer etiketleri, başlık, açıklamalar,
  karartma örtüleri).
- Çizici yalnız `Prim`'leri boyar; anlamsal okuma yalnız `panels`/`layers`/`items`'ı okur.
  İkisi de aynı tarif üzerinden çalıştığı için "ekranda ne var" ile "çizilen" ayrışamaz.

**Yerleşim** (R0 ile uyumlu): çizim alanı sol 8, sağ `W − 72`, üst 34, alt `H − 24`;
paneller ağırlıklarına göre bölünür, aralarında 8 px; tek panelde alan R0'ınkiyle aynı.
Bar yuvası `slot = (sağ − sol) / yuva sayısı`, yuva merkezi
`floor(sol + (i − from + 0.5) · slot)`. Fiyat etiketleri sağda (`{:.2}`), ay etiketleri
yalnız en alttaki panelin altında, ay değişiminde dikey ızgara (etiketler arası ≥ 72 px).
Başlık satırı: sol `başlık`, `C <son kapanış>` (yön rengi); sağda panel 0'ın çizgi/alan/bant
açıklamaları sağa yaslı (`SMA20 <son değer>`), alt panellerde açıklamalar panelin sol
üstünde. Her panelde ilk veri katmanının son değeri sağ eksende etiketlenir.

**Belirleyicilik:** yerleşim `f64`'te yalnız temel işlemlerle (`+ − × ÷`, `floor`,
`round`, `min`, `max`) yapılır; `ln`/`log10`/`powf` hedefler arasında farklı sonuç
verebildiği için log ölçek, temel işlemlerle yazılmış belirleyici bir `log10` kullanır
(üs ayrıştırma + sabit adımlı seri). Sayı biçimlendirme Rust'ın `core::fmt`'si. Aynı görünüm
+ veri her hedefte aynı tarifi ve aynı pikselleri verir.

## 15. Görüş alanı ve ayrıntı düzeyi (LOD)

Görünen yuva sayısı çizim alanının piksel genişliğini aşarsa (`slot < 1`) barlar
`k = ceil(yuva / genişlik)` büyüklüğünde kovalarda birleştirilir: mumda ilk `o`, en büyük
`h`, en küçük `l`, son `c`; çizgide kova başına ilk/en küçük/en büyük/son (dikey parça);
çubukta mutlak değeri en büyük. Sahne öğesi `bars: k` taşır; `SceneDesc.lod` kova
büyüklüğünü verir. Ölçüm: `rounds/R2/REPORT.md` (ölçüt 10).

## 16. Anlamsal okuma

- **`screen`:** her ekranlı bağlantı için görünüm, görünen aralık (gelecek yuvalar dahil, §3) ve panel başına katmanlar
  (kimlik, işaret, veri, alanlar, açıklama, sabitler ve kökenleri, görünen öğe sayısı, son
  değerler) — sahne tarifinden.
- **Nokta sorgusu (`hit`):** pikselin paneli, bar yuvası, zamanı, ölçekteki değeri ve
  oradaki en üstteki öğe (katman kimliği, işaret, veri, bar, zaman, değerler, kökenler).
  Boş alanda `item` yoktur ("öğe yok"). Öğe kutuları: mum gövde+fitil (en az 3 px geniş),
  çizgi noktası yuva genişliğinde ±4 px (çok çıktılı çizgide her çıktı için ayrı öğe), kural tam genişlik ±3 px, bant/alan yuva sütunu,
  nokta çapı + 2 px. Birden çok öğe varsa boyama sırasında en sonuncusu.
- **Gidiş-dönüş:** her öğe için veri → piksel (sahne) → nokta sorgusu → aynı bar ve aynı
  değerler (özellik testi).

## 17. Elenen tasarım seçenekleri

| Seçenek | Neden elendi |
|---------|--------------|
| Vega-Lite'ı olduğu gibi almak | Zaman ekseni sürekli zaman; seans boşluğu, canlı son bar, çok panelli ortak eksen, `null` ısınma ve köken birinci sınıf değil. Kanal sözlüğü ve `mark`/`encoding` ayrımı alındı. |
| Katmanları panellerin içine yerleştirmek (`panels[i].layers`) | Daha düzenli ama `qp/0`'da katman düz listede ve `panel` alanı numara; RD1 dökümleri değişmeden geçmeliydi. Ayrıca çizim sırası paneller arası tek liste olarak kalıyor. |
| Panel kimliği dizge (`"rsi"`) | Okunaklı, ama eski `panel: 1` numarasıyla iki ayrı tip olurdu. Numara kimlik + isteğe bağlı `name` seçildi. |
| Serbest kanal adları (`BTreeMap<String,String>`, v0) | T6e: modeller tanımsız kanallar yazdı (9/10). Kapalı, tipli `Encoding` + `deny_unknown_fields`. |
| Sabiti kanal içinde nesneyle yazmak (`"y":{"value":70,"origin":…}`) | Eski `"y":"70"` biçimiyle bozulurdu (RD1 dökümü 16 tam bu biçimi bekliyor). Köken katman düzeyinde, göstericiyle; sabit yerinde kalıyor. |
| Kökensiz sabite izin verip uyarı vermek | İlke 6 ile çelişir; ajan uyarıları okumaz. Red + seçenek listesi. |
| Sabitleri olgulara otomatik bağlamak | Aynı değerli olgu tesadüf olabilir; bağ iddia içerir. Yalnız adlandırılmış eşik (geleneksel seviye) otomatik. |
| Ayrı `fan`/`cone` işareti | Üst üste `band` ile kurulabiliyor; yeni işaret = yeni enum değeri (G10), gerekirse sonra. |
| Sürekli renk geçişi (gradyan) | Basamaklı renk v0 için yeterli; gradyan ara değer ve tema kararı istiyor. |
| Gelecek barları takvimle yerleştirmek | Takvim verisi `qp-calendar`'da var, ama v0'da "ardışık seans" kuralı belirleyici ve takvimsiz yeterli. |
| Sahne tarifini `qp-grammar`'ın iç tipi yapmak | Çizici, anlamsal okuma ve ileride GPU kabuğu aynı biçimi okuyacak; tek şema kaynağı `qp-schema` (`scene.rs`). |
| Çizimde `ln`/`log10` | Hedefler arası belirleyicilik R0 kuralıyla yasak; belirleyici yordam yazıldı. |
| Tema renklerini görünümde saklamak | Görünüm ile sunum karışır; tema motorda, görünümde yalnız ad. |

## 18. Anlam işareti ve kapsül görünümü (R3)

R3/T7 eki. Kavramlar `docs/CONCEPTS.md` "Kapsül (R3)", çerçeve `docs/CAPSULE.md` §4, §9.6.
Eklerin hepsi isteğe bağlı alan ya da yeni enum değeridir: R2 görünümleri, dökümleri,
altın görselleri ve `explain` dosyaları değişmez (doğrulama: `scripts/verify.sh` `views`
adımı ve `crates/qp-cli/tests/views.rs`).

### 18.1 `glyph` işareti

```json
{"id":"glyph","mark":"glyph","finding":"fnd.SYN03.breakout-up.144","panel":0,
 "encoding":{"size":32}}
```

- **Veri:** katmanın `finding`'i; işaret o bulguyu sunan kapsülün işaretidir. Parametreler
  (`GlyphSpec`: tür, yön, güç, güven, aşama, tazelik, teyit, sunucu işareti) derleyiciye
  `DataProvider::glyph(kapsül_ya_da_bulgu)` ile gelir (varsayılan: yok; kapsülün kendisi
  `DataProvider::capsule`). Otoritenin sağlayıcısı R3/T7'de kuralı
  `qp_grammar::depth::story_glyph` ile uygular: motorun `qp_capsule::glyph_spec(story,
  son_bar, GLYPH_MIN_LEDGER_N = 20)` kuralının aynısı (testle karşılaştırılır). Motoru
  çalıştıran oturum kendi değerini verebilir.
- **Satır:** bulgunun ilk olgusu (bar, zaman, olgu alanları). Bir katman bir işaret çizer.
- **Kodlama:** `x` yoksa olgu barı, `{"panel":…}` ise panele göre; `y` yoksa panelin
  üstü (bar üstünde işaret şeridi), alan ya da kökenli sabit ise o değer, `{"panel":…}`
  ise panele göre; `size` sabit piksel (varsayılan 24). `color`, `opacity`, `width`,
  `y2`, `text` ve alana bağlı `size` `invalid`: renkler yalnız temadan gelir.
- **Çizim:** ürün alfabesi (`qp_grammar::glyph::alphabet()`, R3/M1 seçimi `siluet`)
  kutuya `Prim`'ler üretir; sahne tarifi ilkelleri taşır, çizici hesap yapmaz. Kutu panelin
  içinde tutulur. Alfabe değişimi `alphabet()` içindeki tek satırdır (insan testi
  `rozet`'i öne çıkarırsa).
- **Geçerlilik kanalı (R4/T3, K42/A28):** `GlyphSpec.validity` (`valid` / `no_longer` /
  `closed`; isteğe bağlı, yoksa bilinmiyor). Alfabe yalnız `no_longer`'da farklı çizer:
  açık hikâyenin durum koşulu artık tutmuyorsa siluet gri olur ve sağ alt köşeye "artık
  geçerli değil" rozeti (içi boş halka + yatay çizgi) gelir; ✓ (dolu daire) ve ✗ (çıplak
  çarpı) sonuç rozetlerinden ayrı, renksiz de okunur. `valid`, `closed` ve alan yokken
  çizim R3'teki gibidir (g1 altınları değişmedi; geçerlilik tablosu yeni altın
  `g2-glyphs-validity-{light,dark}.png`). Motoru ve barları bilen sağlayıcı
  (`qp-session` `DocProvider::glyph`) değeri cümlenin kullandığı kuralla hesaplar
  (`qp_capsule::glyph_spec_at` + `sentence::validity_now`); belgeden yalnız hikâyeyi okuyan
  `story_glyph` alanı boş bırakır. Böylece cümle "artık geçerli değil" diyorsa işaret de
  der (K42; test `crates/qp-session/tests/capsule_k42.rs`, iki dilde).
- **Sonuç işareti stili (R4/M11):** "gerçekleşti" ile "tutmadı"yı ayıran sonuç işareti
  (`docs/CONCEPTS.md` "Sonuç işareti") adla seçilen bir `OutcomeStyle` ile çizilir
  (`qp_grammar::glyph::outcome`; stil alfabeyi sarar, her alfabe için ayrı çizebilir).
  Seçim derleme seçeneğidir: `CompileOptions.outcome` (tema geçersiz kılması gibi bir bakış
  ayarı; görünüme yazılmaz). Yoksa `r3`: alfabenin R3 çizimi aynen (altınlar bayt eşit).
  Bilinmeyen ad → `invalid` (`(options)/outcome`). M11 alternatifleri yalnız
  `glyph/outcome.rs`'e bir tip ve `STYLES`'a bir satır ekler.
- **Okuma:** öğe değerleri `strength`, `freshness`, `confirmations`, varsa `confidence`;
  `hit` öğenin verisini `{"kind":"finding","finding":…}` olarak döndürür (kapsül, bulgusu
  üzerinden: kimlikler `cap.…` ↔ `fnd.…`).
- **Hatalar:** `finding` yok → `invalid` (`/finding`; `fact`/`series` ile → `/mark`);
  işareti olmayan bulgu → `not_found` (`/finding`).

### 18.2 Panele göre konum (`{"panel": f, "px": d}`)

Konum kanallarının yeni biçimi: panelin genişliğinin (`x`) ya da yüksekliğinin (`y`) `f`
kesri artı `d` piksel. **Düzen sabitidir**, veri iddiası değildir; köken istemez (§10 kural
5). Yalnız `text` ve `glyph` alır; başka işarette `invalid`.

- `text`: panele göre `x` → metin o noktaya sola yaslanır, panelin sağ kenarında satır
  kırılır, panelin altını aşan satırlar düşer (son kalan satır "…" ile biter); bir bara
  bağlı olmadığı için katman ilk satırını çizer. Panele göre `y` → ilk satırın üstü.
- **Kart paneli:** bütün katmanları panele göre `y` kullanan bir panelin değer ölçeği
  gösterilmez (ızgara, değer etiketi, eksen çizgisi yok).
- Bulgu satırlarının yeni metin alanları: `headline` (kapsül cümlesi), `ledger` (karne
  satırı); olgu satırının `requires` alanı (§18.4).

### 18.3 `x2`: parça ve bar aralığı

`Encoding.x2` (bar aralığının son barı):

- `rule` + `y` + `x` + `x2`: `x` barından `x2` barına yatay parça; `y2` de verilirse
  `(x, y)`'den `(x2, y2)`'ye eğik parça. Değerler kökenli sabittir.
- `band`/`area` + `x` + `x2`: yalnız `x..=x2` barları.
- Veri bağlamayan `point` + `x`: noktanın barı.
- Bar sabitleri: kökeni barsa (olgunun `bar`'ı ya da geometri barı) bar sırası, değilse
  zaman (dikey kuralla aynı kural).

**Geometri kökenleri:** `Origin::Fact` alanı olgunun kanıt geometrisinin barlarını da
adresler: `geometry.<i>.from_bar`, `geometry.<i>.to_bar`, `geometry.<i>.points.<j>.bar`
(`qp_grammar::depth::geometry_field`; otoritenin `canon`'u aynı işlevi kullanır).

### 18.4 Derinlik kuralları (R3)

R2 bulguları (olgusunda `meaning`, kapsülünde `story` yok) §13'teki kurallarla aynen
sunulur. R3:

| Derinlik | Çıktı |
|----------|-------|
| `sentence` | Kapsülün cümlesi (`headline`). |
| `capsule` | Kapsül kartı (kapsülün `story`'si varsa): panel 0 `card` (ağırlık 2) — `glyph` (40 px, sol üst), `sentence` (cümle, 13 px, kırılır), `ledger` (karne satırı: `Karne <gerçekleşen>/<kapanan>` ya da `henüz karne yok`, sol alt); panel 1 `price` (ağırlık 3) — `line` (bar serisinin kapanışı, son `max_points` bar, açıklama `C`), olgunun `level`'i varsa `level` kuralı (köken: olgu), görünürse `fact` noktası. Başlık türetilir (sembol, periyot). |
| `chart` | (ilk olguda `meaning` varsa) R2 grafik kuralı + mum panelinde bulgunun her olgusunun kanıt geometrisi + kapsülün işareti olgu barının üstünde (`glyph`, 32 px). Görünen aralık ilk olgunun 30 bar öncesinden ya da ilk geometri barının 5 bar öncesinden (hangisi önceyse) son bara. Olguya `highlight`. |

**Kanıt geometrisi** (`depth::evidence_layers`): `Level` → `level` parçası (`rule`,
`x..x2`, `accent`, 2 px); `Band` → `band` (mumların altında, `accent`, opaklık 0,18, kenar
1 px); `Points` → ardışık noktalar arasında `swing` parçaları ve her nokta için
`swing-point`. Bütün konumlar (değerler ve barlar) **yalnız kökenle** bağlanır: katman sayı
taşımaz, derleyici değeri okuyucunun sağlayıcısındaki olgudan çözer (K33). Kimlikler
tekrar ederse `-2`, `-3`… eki (K25).

**Kanıt yetkisi:** geometrinin bir değeri olgunun kanıtında yoksa (yetkisiz okuyucu için
otorite ayıklamış) o geometri çizilmez; olgunun satırına `requires` metni eklenir ("Kanıt
değerleri veri yetkisi gerektirir: `<entitlement>`") ve grafik onu panelin sol üstünde
`evidence` metin katmanında (`warn`) gösterir. Yetkili okuyucu için kurulmuş bir görünüm
yetkisiz sağlayıcıyla derlenirse sıfır çizilmez, `not_found` döner. Not: `show_finding`
görünümü dokümana yazarken otorite kökenli değeri sabit olarak yazar (§10 kural 4); lisanslı
olgunun saklanan görünümünü okuyucuya göre ayıklamak oturumun işidir (R3 bütünleştirme).

### 18.5 Elenen seçenekler

| Seçenek | Neden elendi |
|---------|--------------|
| Katmana ayrı `capsule` veri alanı | `finding` bağı yeterli (kapsül bulgusuyla bire bir); yeni `DataSource` türü `hit`/`screen`/`explain`'i ve oturumu genişletirdi. |
| Kanıt değerlerini görünüme sabit olarak yazmak | Lisanslı olgunun rakamı görünümle yetkisiz okuyucuya gidebilirdi (güvenlik incelemesi); değer yalnız kökenden çözülür. |
| Başlığı birden çok satıra kırmak | Var olan altın görseller değişirdi (`d1-capsule` başlığı zaten kesiliyor); cümle kartta ayrı metin katmanı. |
| Kart için gramer dışı özel çizim | Kart da gramer cümlesi olmalı (ilke 8): üç katman, panele göre konum. |

### 18.6 Açılma ve ortak öğeler (R4)

R4/T4 eki (`docs/PROTOCOL.md` §4.6.1). Kurallar `qp_grammar::depth` (`deeper`,
`shallower`, `presenter_depth`, `tap_opens`, `transition_anchors`, `shared_elements`) ve
piksel → bar aritmetiği `qp_grammar::gesture` (`drag_bars`, `pinch_anchor`, `pan_range`,
`zoom_range`; otoritenin `pan`/`zoom`'u ile aynı kural, testle karşılaştırılır). Hiçbiri
dokümana yazmaz.

**Açılma derinlikleri:** `sentence` (açık kapsül yok: akış) → `capsule` (açılmış kapsül;
sahnesi §18.4 kartı) → `chart` (işaretli grafik; §18.4 grafik kuralı ya da kapsülün saklı
görünümü) → `evidence` (aynı grafik + olgu başına kanıt erişimi). `evidence` sunucuda
`chart` derinliğiyle çizilir. Akıştaki kapalı kapsülün **yüzü** bugün kartla aynı sahnedir
(M8 yüzü ayrı bir derinlik kuralına taşıyabilir; açılma kuralı değişmez).

**Dokunmanın açtığı (`tap_opens`):**

| Derinlik | Dokunulan | Sonuç |
|----------|-----------|-------|
| `sentence` (yüz) | her yer | `capsule` |
| `capsule` | her yer | `chart` |
| `chart` | kanıt öğesi: olgu ya da bulgu katmanı (olgu noktası, işaret) ya da sabiti bir olgudan gelen katman (seviye, bant, salınım) | `evidence` |
| `chart` | başka yer | açılma yok; bar odaklanır |
| `evidence` | her yer | açılma yok |

**Geçiş öğeleri (`transition_anchors`):** bir kapsül sahnesinde kimlikli öğeler ve
kutuları (öğenin isabet kutularının birleşimi): `glyph` (işaret katmanı), `headline` ve
`ledger` (bulgunun metin katmanları), `fact:<olgu>` (olgu çizen katman), `evidence:<olgu>`
(sabiti olgudan gelen katmanlar), `price` (bar serisini çizen ilk katmanın paneli),
`bar:<i>` (bulgunun ilk olgusunun barı görünüyorsa o panelde bar sütunu). **Ortak öğeler
(`shared_elements`):** iki sahnede de olan kimlikler, kimlik sırasıyla, iki kutuyla. Bir
tarafta sahne yoksa (`sentence`, yüz çizmeyen yüzey) cümle yine görünür: `headline` o
tarafta kutusuz döner. Kabuk geçiş animasyonunu yalnız bu iki kutu arasında boyar; hangi
öğenin süreceğine çekirdek karar verir (M10 iki geçiş tasarımını bu yapı üzerine kurar;
yeni bir öğe yeni bir kimlikle eklenir, yapı değişmez).

Örnek (`53-input-open-close`): kapsül → grafik geçişinde `bar:124`, `evidence:fx…124`,
`glyph`, `price` ortaktır; grafikte başlık metni olmadığı için `headline` ortak değildir.
