# R2/T1 — Altı görünüm (taslak, tiplerden önce)

Bu belge gramer tiplerinden **önce** yazıldı. Her görünüm bir **protokol mesaj dizisidir**
(elle yazılmış görünüm dosyası değil): başlangıç dokümanı + komutlar. Gramer bu dizilerle
sınanır; tipler bu dizileri taşıyabilecek biçimde yazılır. Sonraki sürümleri
`fixtures/views/*.json` dökümleridir (T4).

Ortak başlangıç dokümanı `base`: tek bar serisi `px` (`fixtures/bars/sample.json`,
200 günlük bar, `SAMPLE 1d`), görünüm yok. (d) için `growth`: aynı biçimde, 10'dan 400'e
üstel büyüyen sentetik 200 bar.

Gösterim kuralı: `→` istemciden otoriteye; her dizi `hello` ile başlar (kısaltıldı).

---

## V0 — R0 sahnesi: mum + SMA(20)

Amaç: var olan altın görselin (`fixtures/golden/sample.png`) **yalnızca protokol
mesajlarıyla** kurulmuş bir görünümden aynen çizilmesi (ölçüt 3).

```json
→ {"type":"command","id":"c1","ops":[
    {"op":"add_study","id":"sma20","study":"sma","inputs":["px"],"params":{"period":20}},
    {"op":"put_view","view":{"id":"r0","x":{"kind":"time"},
      "panels":[{"id":0,"y":{"kind":"linear"}}],
      "layers":[{"id":"candles","mark":"candle","series":"px","panel":0},
                {"id":"sma","mark":"line","series":"sma20","panel":0}]}}]}
```

Sınadığı: varsayılan tema (R0 renkleri), başlık satırı (sembol + periyot, son kapanış),
çizgi katmanının açıklaması ("SMA20 <son değer>"), son fiyat etiketi, ay ızgarası,
ısınma barlarında nokta yok. Hesap yok: SMA değerleri otoritenin `qp-compute`'undan.

## V1 (a) — üç panel: fiyat, RSI, MACD histogramı

```json
→ {"type":"command","id":"c1","ops":[
    {"op":"add_study","id":"rsi14","study":"rsi","inputs":["px"]},
    {"op":"add_study","id":"macd","study":"macd","inputs":["px"]},
    {"op":"put_view","view":{"id":"three","x":{"kind":"time"},
      "panels":[{"id":0,"name":"price","y":{"kind":"linear"}},
                {"id":1,"name":"rsi","y":{"kind":"linear","domain":[0,100]}},
                {"id":2,"name":"macd","y":{"kind":"linear"}}],
      "layers":[{"id":"candles","mark":"candle","series":"px","panel":0}]}}]}
→ {"type":"command","id":"c2","ops":[
    {"op":"insert_layer","view":"three","layer":{"id":"rsi","mark":"line","series":"rsi14","panel":1}},
    {"op":"insert_layer","view":"three","layer":{"id":"ob","mark":"rule","panel":1,"encoding":{"y":70}}},
    {"op":"insert_layer","view":"three","layer":{"id":"os","mark":"rule","panel":1,"encoding":{"y":30}}},
    {"op":"insert_layer","view":"three","layer":{"id":"hist","mark":"bar","series":"macd","panel":2,
      "encoding":{"y":"histogram",
        "color":{"when":{"field":"histogram","op":"ge","than":0},"then":"up","else":"down"}}}}]}
```

Sınadığı: kimlikli panel ve katman ekleme; sabit 70/30 kurallarının kökeni (adlandırılmış
eşik `rsi.overbought`/`rsi.oversold` — otorite bağlar, bağlayamazsa reddeder); çok çıktılı
hesabın tek çıktısı (`y: "histogram"`); koşullu renk; çubuk tabanı 0. **Sorun bulundu:**
koşuldaki `0` da bir sabit; kökeni `macd.zero` eşiği olmalı → kural: görünümdeki her
sayısal veri sabiti (konum ve koşul eşikleri) bir kökene çözülür.

## V2 (b) — Bollinger bandı alan olarak

```json
→ {"type":"command","id":"c1","ops":[
    {"op":"add_study","id":"bb","study":"bollinger","inputs":["px"]},
    {"op":"put_view","view":{"id":"boll","x":{"kind":"time"},
      "panels":[{"id":0,"y":{"kind":"linear"}}],
      "layers":[
        {"id":"band","mark":"band","series":"bb","panel":0,
         "encoding":{"y":"upper","y2":"lower","color":"accent","opacity":0.18}},
        {"id":"mid","mark":"line","series":"bb","panel":0,"encoding":{"y":"middle","color":"accent"}},
        {"id":"candles","mark":"candle","series":"px","panel":0}]}}]}
```

Sınadığı: iki çıktı arası bant; katman sırası = çizim sırası (bant mumların altında);
ısınmada bant yok (iki uç da `null` olmayan barlarda çizilir; boşlukta bant kesilir);
tema rengi adı (`accent`).

## V3 (c) — belirsizlik yelpazesi (sentetik üç olasılık bandı)

Altı sentetik nokta serisi (`q05…q95`), son bardan başlayıp 20 bar ileriye uzanır.
Gelecek noktalar bar dizisinin sonuna ardışık seans olarak yerleşir (seans boşlukları
kapalı zaman ölçeği).

```json
→ {"type":"command","id":"c1","ops":[
    {"op":"put_series","series":{"id":"q05","source":{"kind":"inline"},
      "data":{"kind":"points","points":[{"t":1759881600000,"v":124.69}, "… 20 nokta"]}}},
    "… q20, q35, q65, q80, q95 aynı biçimde",
    {"op":"put_view","view":{"id":"fan","x":{"kind":"time"},
      "panels":[{"id":0,"y":{"kind":"linear"}}],
      "layers":[
        {"id":"candles","mark":"candle","series":"px","panel":0},
        {"id":"b90","mark":"band","series":"q05","panel":0,
         "encoding":{"y":"v","y2":{"series":"q95","field":"v"},"color":"accent","opacity":0.15}},
        {"id":"b60","mark":"band","series":"q20","panel":0,
         "encoding":{"y":"v","y2":{"series":"q80","field":"v"},"color":"accent","opacity":0.22}},
        {"id":"b30","mark":"band","series":"q35","panel":0,
         "encoding":{"y":"v","y2":{"series":"q65","field":"v"},"color":"accent","opacity":0.30}}]}}]}
```

Sınadığı: iki **farklı seri** arası bant (kanal başka bir seriye başvurur); nokta serisi
(`v` alanı); bar dizisinin ötesine uzanan veri (x alanı sağa genişler); opaklık katmanı.
**Sorun bulundu:** ilk taslakta `y2` yalnız alan adıydı; iki seri arası bant için kanal
değeri `{"series","field"}` nesnesi olabilmeli.

## V4 (d) — logaritmik ölçek

```json
→ {"type":"command","id":"c1","ops":[
    {"op":"put_view","view":{"id":"logv","x":{"kind":"time"},
      "panels":[{"id":0,"y":{"kind":"log"}}],
      "layers":[{"id":"candles","mark":"candle","series":"px","panel":0}]}}]}
→ {"type":"command","id":"c2","ops":[{"op":"set_scale","view":"logv","scale":"y0","kind":"log"}]}
```

(`growth` dokümanı üzerinde.) Sınadığı: log ölçek eşleme ve ızgara adımları (1-2-5 ×10^k
ondalık); **belirleyicilik:** `ln`/`log10` hedefler arasında farklı olabilir (R0 kuralı:
yalnız temel işlemler) → log, temel işlemlerle yazılmış belirleyici bir yordamla; ölçeğin
kimlikle değiştirilmesi (`set_scale`). Değeri ≤ 0 olan veri log ölçekte `invalid` değil,
çizilmez ve `screen`'de sayılır.

## V5 (e) — koşullu renkli çizgi/alan

```json
→ {"type":"command","id":"c1","ops":[
    {"op":"add_study","id":"sma50","study":"sma","inputs":["px"],"params":{"period":50}},
    {"op":"add_study","id":"rsi14","study":"rsi","inputs":["px"]},
    {"op":"put_view","view":{"id":"cond","x":{"kind":"time"},
      "panels":[{"id":0,"y":{"kind":"linear"}},{"id":1,"y":{"kind":"linear","domain":[0,100]}}],
      "layers":[
        {"id":"fill","mark":"area","series":"px","panel":0,
         "encoding":{"y":"c","y2":{"series":"sma50","field":"sma"},"opacity":0.3,
           "color":{"when":{"field":"c","op":"ge","than":{"series":"sma50","field":"sma"}},
                    "then":"up","else":"down"}}},
        {"id":"close","mark":"line","series":"px","panel":0,"encoding":{"y":"c","color":"text"}},
        {"id":"rsi","mark":"line","series":"rsi14","panel":1,
         "encoding":{"color":{"field":"rsi","thresholds":[30,70],"colors":["up","muted","down"]}}}]}}]}
```

Sınadığı: koşullu renk (alan, iki seri arası; kesişimde renk değişir); değere bağlı
basamaklı renk; eşiklerin kökeni (30/70 → `rsi.*`). **Sorun bulundu:** koşullu renkli
çizgide renk değişimi bar ortasında mı, kesişim noktasında mı? → kural: parçalar bar
arasında bölünür, kesişim noktası doğrusal ara değerle (temel işlemler) bulunur.

---

## Derinlik görünümleri (D1, D2)

Doküman `thyao` (bulgu `fnd-overbought`, olgu `fact-rsi-cross70`, `evidence.level = 70`).

```json
→ {"type":"query","id":"q1","query":{"kind":"present","finding":"fnd-overbought","surface":"watch"}}
→ {"type":"query","id":"q2","query":{"kind":"present","finding":"fnd-overbought","surface":"phone"}}
→ {"type":"query","id":"q3","query":{"kind":"present","finding":"fnd-overbought","surface":"desktop"}}
```

- `sentence` (saat): yalnız metin.
- `capsule` (telefon): tek panel; olgunun serisinin son `max_points` (60) barı çizgi,
  olgu seviyesinde kural (köken: olgu), olgu barında nokta (veri: olgu); başlık metni.
- `chart` (masaüstü): sunucunun görünümü (mum, hesap paneli, seviye kuralı) + olgu barına
  vurgu (`emphasis`).

Sınadığı: derinlik kuralı gramerde; yüzey profilinin indirgemesi (`max_points`); vurgu.
