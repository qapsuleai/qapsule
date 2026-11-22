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
| `dark`, `light` | Renk rolleri (`Theme`): §9'daki adlar + `surface` (yüzey), `line`, `neutral` (yönsüz işaret), `valid`, `invalid` (artık geçerli değil: işaret ve cümle), `closed` (kapanmış işaret), (T3b) `closed_alpha` (kapanmış işaretin en düşük saydamlığı, 0–255: alfabe işareti tazeliğe göre soldurur, A31; kapanmış işaret bunun altına inmez, böylece görünüş ≥ 3:1'i tutturabilir; (T3c) bu taban alfabelerde (`siluet`, `rozet`) ve her sonuç biçiminde geçerlidir: `tg5`'in "gerçekleşti" plakası ve köşe işaretleri bu tabandadır, "tutmadı" izi `closed` renginde çizilir ve en güçlü çizgisi tabana çıkacak biçimde soldurulur (soluk güven plakası oranını korur); 0 = yalnız tazelik), (T3c) `fresh_alpha_min` (açık işaretin tazelik solmasının tabanı, 0–255: açık işaret de tazeliğe göre solar ama bunun altına inmez; 0 = yalnız tazelik), (T3c) `plate_alpha_min` (işaretin güven plakasının — `siluet`'in alttan dolan soluk dolgusu, `rozet`'in seviyesi — en düşük saydamlığı, 0–255; plaka tazelikle solar ama bunun altına inmez; 0 = yalnız tazelik), `focus` (odak); `chrome` (kabuğun kendi renkleri: zemin, metin, ikincil metin, çizgi, kart, vurgu, vurgu üstü metin, sayfa, bildirim ve hata şeridi, "yetki gerekir" = kanıt ekranının turuncu yazısı) | §9 değerleri; `neutral`=`accent`, `valid`=`text`, `invalid`=`closed`=`muted`; `closed_alpha`, `fresh_alpha_min` ve `plate_alpha_min` 0; `chrome` = R4 `style.css` renkleri |
| `scheme` | Okur seçmediğinde düzen | `dark` |
| `space` | Boşluk ölçeği (kabuk: `--qp-space-0..5`), çizim alanı kenar boşlukları (sol, sağ, üst = başlık satırı, alt), panel arası, işaret ile cümle arası | `[4,6,8,12,16,24]`, `[8,72,34,24]`, 8, 10 |
| `surface` | Kabuk köşe yarıçapı, telefon çerçevesi ve rozet yarıçapı, çizgi kalınlığı, gölge (yok = düz), vurgu şeridi ve perde saydamlığı | 6, 18, 8 px; 1; düz; 56, 160 |
| `glyph` | İşaretin boyu: yüzde, kartta, grafikte, boyu verilmemiş katmanda; yüzdeki yeri (`leading`: cümlenin solunda, `above`: üstünde) | 32, 40, 32, 24; `leading` |
| `motion` | Geçiş süresi çarpanı, eğri (`linear` ya da `cubic-bezier`, sabit adımlı çözüm), kabuk geçiş süresi | 1, `linear`, 0 ms |
| `chrome` (T3b) | Kabuğun boyları (CSS px): metin boyu ve satır yüksekliği, küçük yazı (bildirim, ipucu, uyarı, kod, bölüm başı), tablo, en küçük yazı (yüz rozeti, açık katmandaki güven işareti, adım sayacı), yüzün güven işareti ("Sunucu doğruladı"); bölüm başının kalınlığı, harf aralığı (em), üst boşluğu; sekme en düşük yüksekliği, iç boşluğu, alt çizgisi; düğme/seçim/alan en düşük yüksekliği ve iç boşluğu; tarih alanı genişliği; alttaki oynatma çubuğunun yüksekliği; tablo hücresi, rozet ve yüz metni iç boşluğu; yüz metninin en düşük yüksekliği | 14 / 1,35; 12; 13; 11; 10; 600, 0,04 em, 10; sekme 0 (= içerik), 6, 2; denetim 0, 4×6; 120; 48; 3×4, 1×6, 14×12; 60 |
| `evidence` (T3b) | Kanıt geometrisi: renk rolü, "… gerekir" notunun renk rolü, seviye/salınım çizgi kalınlığı, bant kenarı ve dolgu saydamlığı, salınım noktası çapı | `accent`, `warn`, 2, 1, 0,18, 9 px |
| `offsets` (T3b) | Sahnenin küçük ofsetleri (px): başlığın soldan içeriği, başlık satırı taban çizgisi, kapanış metninin yeri, satır kenar boşluğu, açıklamalar arası, alt panel açıklamasının içeriği ve taban çizgisi, kart karne satırının alttan boşluğu, kanıt notunun soldan içeriği ve işaret şeridinin altındaki boşluğu, `y`'siz işaret katmanının üstü | 4, 23, 140, 8, 12, 4, 12, 2, 4, 8, 2 |
| `profile`, `origin`, `chart` (RD2/S1) | Yoğunluk profili (`legacy`, `dense` = masaüstü, `touch` = telefon; bir yön `<ad>` ve `<ad>-tel` olarak iki görünüş kaydeder); köken renk rolleri (`human`, `agent`, `engine`, iki düzende) ve ajan çizgisinin kesiği (yoksa düzenden türetilir: `text`, `accent`, `muted`; kesik = hikâye çizelgesininki); grafik alışkanlıkları (`ChartLook`: fiyat ekseni etiket aralığı, iki düzeyli zaman ekseni, son değer etiketi, değer çizgisi, imleç ve eksen hapları, OHLC lejantı, hacim bölmesi payı, veri üstü işaretin en büyük boyu, mum gövde oranı, sağ eksen oluğu; derleyici RD2/C2a'dan sonra eski olmayan görünüşte okur: "Grafik alfabesi"). `surface`'a kart yarıçapı ve CSS gölgesi eklendi. Ölçüt: `scripts/design-check`, değerler `docs/design/CONTRACT.md` | `legacy`; türetilmiş köken; `ChartLook::legacy(72, 32)` (90 px, tek düzey, etiket var, değer çizgisi yok, imleç yok, yalnız `C`, hacim 0, 0,7); kart 6, gölge `none` |

**Okuyanlar.** Derleyici (kenar boşlukları, panel arası, başlık/etiket/rakam boyu,
metin genişliği, satır yüksekliği, şerit ve perde, başlık satırı ofsetleri), yüz tasarımları (`FaceViewFn` görünüşü
alır; yüz yüksekliği `faces::height_in(look, face)`), kart ve grafik kuralları
(`depth::capsule_view_in`, `chart_view_in`), işaret alfabeleri (renk rolleri), geçiş
ilerlemesi (`transitions::progress_in`), kabuk (CSS özel özellikleri, aşağıda).

**Yazı tipi.** Bir varlık (`look::FONTS`) tek aile ve tek ağırlıklı **statik TTF**
dosyasıdır (fontdue değişken yazı tipinin yalnız varsayılan örneğini çizer, woff2
okumaz): kimlik, aile, ağırlık, dosya, lisans dosyası, SPDX, tuvaldeki CSS ailesi,
ortalama ilerleme (em), yükselme (em) ve (T3b) **glif ilerleme tablosu** (`advances`).
Tablo dosyadan fontdue'nün ölçüleriyle **üretilir**, elle yazılmaz
(`crates/qp-grammar/src/advances.rs`, `QP_BLESS=1 cargo test -p qp-render --test fonts`;
aynı sınama tablo dosyayla uyuşmazsa düşer): eşlenen her kod noktasının ilerlemesi (font
birimi; ardışık eşit ilerlemeler tek koşu), em başına birim, eksik glifin ilerlemesi.
Derleyici metin genişliğini (satır kırma, sığdırma, açıklama yeri, metin isabet kutusu)
bununla ölçer: orantılı ailede karakterlerin ilerlemeleri toplanır, `ceil(toplam · boy /
em)`; eşit aralıklı ailede (her boşluklu glif eksik glifle aynı) R3'ün `ceil(karakter ·
boy · advance)` hesabı aynen kalır (`r4` bayt eşit). Aralık (kerning) yok: fontdue
uygulamaz, tuval `fontKerning = "none"`. Sahnedeki metin
varlığını `Prim::Text::font` ile adlandırır (yoksa varsayılan `plex-mono`; `r4`
sahneleri bu alanı hiç yazmaz). Başvuru çizici (`qp_render::fonts`), `canvas` yolu
(`FontFace`, baytlar çekirdekten: `core.fonts()` = `[{id, family, weight}]`, `core.font_bytes(id)`; ağ yok; T3c: 400 dışı ağırlıktaki bir dosya, aynı yayımlanmış ailenin dik 400 dosyasının CSS ailesi altında kendi ağırlığıyla da listelenir — kabuğun gövde ailesindeki `font-weight: 600` sentezlenmez, gerçek dosyayla çizilir; kaydı çekirdek belirler, kabuk aynen uygular) ve `gpu`
yolu aynı dosyayı kullanır. Tuval `fontKerning = "none"` ile çizer: fontdue GPOS aralığı
uygulamaz, iki yol aynı yerleşimi verir. fontdue şekillendirme yapmaz (`tnum`
uygulanamaz): tablo rakamı gereken aile ya varsayılan rakamları tablo olan bir aile
olmalı ya da dosya derleme öncesi cmap'te 0–9 tablo gliflerine yeniden eşlenmeli
(`rounds/R5/RESEARCH.md`).

**Roller sahnede.** `CompileOptions::roles` (ve `qp scene --roles`) her metne rol adını
yazar (`Prim::Text::role`): derleyicinin metinleri kendi rolüyle; görünümün metin
katmanları boylarına göre (`headline`, (T3c) `title`, `body`, `label`, `note` boyları her
görünüşte farklıdır, kayıt sınaması; `title` boyundaki metin — `vadrel`'in dil seçiliyken
çizilen karne bandı — görünüşün `title` yazı tipiyle çizilir, `r4`'te Plex Mono); işaretin metinleri `glyph`. Varsayılan kapalı: ürün
sahneleri ve özetleri değişmez. Ölçüm düzeneği öğe başına rengi, boyu ve rolü sahneden,
rolün yazı tipini ve renk rollerini `qp look show <ad>`'dan okur.

**Grafik alfabesi (RD2/C2a, `compile/alphabet`).** Eski olmayan bir görünüş
(`profile` `dense` ya da `touch`; `Look::familiar`) grafiğin alışılmış öğelerini
`ChartLook`'tan çizer; `legacy` görünüşler R0 yolunu bayt bayt korur (kod `scene.rs`'ten
aynen taşındı). Sağ fiyat ekseni: her `price_label_px`'te bir etiket (adım
1–1,5–2–2,5–3–4–5–7,5 × 10^k'den hedefe en yakını), etiket yalnız kendi bölmesinin
yüksekliğinde ve `axis_gutter_px` oluğunda (sağ kenar payı olur); son değer etiketinin ve
fiyat hapının altında kalan etiket çizilmez (P4, P6). İki düzeyli zaman ekseni
(`two_level_time`): küçük birim (gün, ay ya da saat) `muted`, büyük birim (ay, yıl ya da
gün) `text` rolünde; düzey, ortalama aralığı bir etiket ve 8 px'e yeten en ince düzeydir;
etiketler öncelikle (büyük birim, ilk barın bağlam etiketi, küçük birim) yerleşir, bir
etikete, tarih hapına ya da çerçeve kenarına değen atlanır (P5, P6). Son değer etiketi
(`last_value_tag`) ve ilk bölmede noktalı değer çizgisi (`value_line`). İmleç
(`crosshair_pills`): `CompileOptions::cursor` (`Cursor {bar, price}`; şema ve protokol
değişmez; gösterimde geçici, `qp render --cursor <bar>[,<fiyat>]`) kesikli nötr çizgi,
sağ eksende fiyat hapı ve zaman ekseninde tarih hapı çizer. OHLC lejantı
(`ohlc_legend`): başlığın sağında imleçteki (yoksa son görünen) barın O H L C değerleri,
değişim ve yüzde (yön renginde); `touch` profilinde C ve yüzde; sığmayan parça sabit
sırayla düşer. Harfler iki dilde O H L C: Türkçe A Y D K sözlükte yok, karar verilince
tek satır (`alphabet/legend.rs`). Sayılar okuyucunun dilinde (`reader_lang`; Türkçe
"2.925,95", "+%1,28"; İngilizce "2,925.95", "+1.28%"); 100 000 ve üstü tek birimde kısa
("1,2 Mn" / "1.2M"); ay kısaltmaları çekirdekte. **Hacim** (`volume_share`): fiyat
bölmesinin alt `volume_share` payında, mumların altında, yön renklerinde soluk (alfa
0,35) histogram. Seçim OpenChart ve Vela'nın kalıbı (ADOPTED OV-10): ayrı bölme açmadan
aynı zaman ekseninde bağlam verir, bölme yüksekliği yemez; görünüm hacmi kendisi
çiziyorsa (bar serisinin `v`'si `bar` katmanında) o bölme kalır, çubukları aynı soluk
alfayı alır. Veri üstü anlam işareti `mark_max_px` ile sınırlı (P8, `Look::mark_px`),
mum gövdesi `body_ratio` (eski görünüşlerde 0,7: R0).

**Kabuk.** Kabuğun renkleri, yazı ailesi, yarıçapları, geçiş süresi ve (T3b) bütün
boyları (yazı boyları, iç boşluklar, sekme/düğme yükseklikleri: `chrome`; boşluklar:
`space.scale`; çizgi: `surface.line` → `--qp-rule`) çekirdekten CSS özel özelliği olarak
gelir (`look::css_vars`, `core.css()`; `qp look css <ad>`); `core.css()` ayrıca çerçeveleri
ekler (`--qp-phone-w/h`, `--qp-face-w/h`, `--qp-scene-w/h`). `style.css` yalnız `var(--…)`
okur: renk de uzunluk da taşımaz; mimari sınaması (`arch-check.mjs`, R7) px/rem/em/ms ve
çıplak sayıyı reddeder. İstisnalar gerekçeli: `0`, `100%`, `100vh/vw`, `z-index`, `flex`
ve iki `@media` genişliği (medya sorgusu özel özellik okuyamaz).

**Yeni görünüş ekleme (M16 alt ajanları için tarif):**

1. `crates/qp-grammar/src/look.rs`'te `M16 looks` işaretinin altına bir `const` `Look`
   yaz (`..R4` ile başla, değişeni yaz; ad `[a-z][a-z0-9_-]*`, sıradan bir kelime değil);
   `LOOKS` dizisine bir satır ekle. `headline`, `title`, `body`, `label`, `note` boyları farklı olmalı.
2. Renk: iki temayı (`dark`, `light`) ve her birinin `chrome`'unu ver. Karşıtlık
   (metin ≥ 4,5:1, büyük metin ve işaret ≥ 3:1, `closed` dahil) ölçüm düzeneğinde sınanır.
3. Yeni yazı tipi gerekiyorsa: statik TTF'yi (ağırlık başına bir dosya) ve lisansını
   (`OFL.txt` ya da eşdeğeri; aile başına ayrı adla, ör. `OFL-Inter.txt`)
   `crates/qp-render/assets/fonts/`'a koy; `look::FONTS`'a bir `FontAsset` satırı
   (kimlik, aile, ağırlık, dosya, lisans, SPDX, `css_family` benzersiz, ölçülmüş ortalama
   `advance` ve `ascent`, `advances: &crate::advances::<KİMLİK_BÜYÜK>`),
   `qp_render::fonts::FILES`'a aynı sırayla bir `include_bytes!` satırı ekle; ilerleme
   tablosunu üret: `QP_BLESS=1 cargo test -p qp-render --test fonts` (`advances.rs`'i
   yazar); aynı sınama `QP_BLESS`'siz iki listenin, dosyanın, lisansın, Türkçe harflerin ve
   tablonun dosyayla tuttuğunu sınar. Ayrılmış yazı tipi adı (RFN; Plex, Source Sans 3)
   taşıyan bir dosyanın **alt kümesi alınırsa aile yeniden adlandırılmalıdır**. Paket
   boyutu: her dosya WASM paketine aynen girer (`plex-mono` 135 580 bayt); `verify.sh`
   `wasm` adımının boyut notunda önce/sonra yaz.
4. Sert eşikler kayıttan tutturulur (`ornek` örnek; `look::tests::ornek_meets_the_hard_thresholds`):
   en küçük yazı (`types` rolleri ve `chrome` boyları), dokunma hedefi (`chrome.tab_h`,
   `chrome.control_h` ≥ 44; `bar_h` bir denetimi alacak kadar), kapanmış işaret
   (`closed` + `closed_alpha`, ≥ 3:1), (T3c) açık işaret en düşük tazelikte (0) (yön rengi
   + `fresh_alpha_min`, ≥ 3:1; `glyph::outcome::tg5::tests::ornek_signs_keep_3_to_1_at_freshness_zero`),
   kanıt yazısı (`warn` ya da `evidence.note_color`
   rolü; kabukta `chrome.req`).
5. Gösterimde `?look=<ad>` ya da Ayarlar → Görünüş; `qp look show <ad>` kaydı yazar.
6. `r4` altınları değişmemeli (`scripts/verify.sh`); yeni görünüş için altın eklenmez
   (round'un `v2` kararı ayrı).

**Kayıt dışında kalanlar** (bilinen, T3b sonrası): eksen etiketlerinin iç ofsetleri ve
ay etiketi aralığı (72 px), son değer etiketinin iç boşluğu, (RD2/C2a'dan sonra yalnız
eski görünüşlerde) mum gövdesi oranı,
işaret alfabelerinin iç oranları ve tazelik eğrisi (`siluet` 0,3 + 0,7·tazelik, `rozet`
0,22 + 0,78·tazelik; kapanmışın tabanı `closed_alpha`, açığın `fresh_alpha_min`), `tavrik` ve `fotolay` yüzlerinin
kendi boyları (elenen M8 tasarımları; renkleri görünüşten), geçiş tasarımlarının kendi
süreleri (görünüş yalnız çarpar) ve `tutamir`'in iğne boyu, **grafik derinliğinin
görünümü**: oturum grafik görünümünü `depth::chart_view` ile (yani `r4` kaydıyla) kurar
(`qp-session` `replica/viewer/open.rs`), bu yüzden `glyph.chart_px`, `evidence` ve kanıt notu
ofsetleri ürünün grafik derinliğinde henüz `r4` değerindedir (renk rolleri ve başlık
satırı ofsetleri derlemede bağlantının görünüşünden okunur); kabuğun yapısal değerleri
(`z-index`, `flex`, `@media` genişlikleri) ve telefon çerçevesi boyu (yüzey `390×844`).

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
(R5/M20) En alttaki panel bir kart paneliyse (§18.2; yalnız metin) ay etiketleri değer
ölçeği olan son panelin altına yazılır (küçük bir grafiğin altında metin blokları); bütün
paneller kartsa yine en alttakinin altına. Var olan görünümlerde (en alttaki panel kart
değil ya da görünen aralıkta ay değişimi yok) çıktı aynıdır.
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
  (R5/M20) Panele göre `x2` verilirse satır orada kırılır (panelin sağ kenarını aşıp sağ
  boşluğa uzanabilir: kart panelinde değer ölçeği yoktur); `x2` olmadan bugünkü kural.
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

**(R5/M20) Açılmış kapsül tasarımları** (`qp_grammar::open`, `OPENS`; `style` `open`,
`ViewState.open`): kapsül derinliğinin görünümünü seçilen tasarım kurar; `r4` (varsayılan)
yukarıdaki karttır. Bir tasarım kendi çerçeve yüksekliğini söyleyebilir (`height`; kabuk
çekirdeği kapsül derinliğini o yükseklikte açar, sayfa katmanı kaydırır), kendi metin
alanlarını okur (bulgu satırında `open.*`, `open::text`), bir metin katmanının bütün panelini
bir sözlük girdisine bağlayabilir (`open::layer_term`: panel dokunma hedefi, ilk satır altı
çizili) ve sahnesi olmayan okur için açılmış katmanın metnini yazabilir (`text`). Sağlayıcının
iki yeni okuması: `reader_lang` (okurun dili) ve `device_check` (cihazın kendi verisinde
sınaması, K48; otoritede yok). `tesir`: `rounds/R5/experiments/M20/tesir/DESIGN.md`.

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
