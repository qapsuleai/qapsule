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
  **v0 sınırı (R2 denetimi K1):** ekranlı bir bağlantının (masaüstü, telefon, saat) her zaman
  bir bakışı vardır (varsayılan: son N bar) ve `look` son barın ötesine izin vermez; bu yüzden
  bu bağlantılar için `screen` ve `hit` gelecek yuvaları **göstermez** (yelpaze bantları yalnız
  son bar yuvasıyla görünür). Ekransız bağlantılar, `qp render` ve `qp explain` gelecek
  yuvaları gösterir. Bakışın gelecek yuvalara uzaması ana akış kararıdır (`rounds/R2/REPORT.md`
  açık soru 6).

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
| `chart` | Sunucunun görünümü: panel 0 olgunun bar serisinin mumları; olgu bir hesap serisindeyse panel 1 onun çizgisi ve olgu seviyesinde kural (köken: olgu); `x.domain = [t(max(0, olgu barı − 30)), t(son bar)]`; olgu barına `highlight` vurgusu. |

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

- **`screen`:** her ekranlı bağlantı için görünüm, görünen aralık ve panel başına katmanlar
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
