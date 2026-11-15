# R2/T8a — Anlamla başvuru: yapılandırılmış seçiciler

Dal: `r2/t8a-selectors` (taban `r2/int`). Şartname: `docs/PROTOCOL.md` §17 (seçilmiş gibi
yazıldı). Ölçüm (T9) bu notun kapsamı dışında; burada tasarım, gerekçe, yirmi görevin bu
tasarımla yazımı ve bilinen zayıflıklar var.

## 1. Fikir

Kimlik alan her alana kimlik **ya da** seçici nesnesi yazılır. Seçici kapalı, tipli,
küçük bir sözlükle öğeyi nitelikleriyle tarif eder:

```json
{"op":"set_encoding","view":{},"layer":{"shows":{"study":"bollinger"},"output":"upper"},
 "encoding":{"color":"down"}}
```

Otorite seçicileri mesaj tipli biçime ayrıştırılmadan önce çözer, kimliği yerine yazar,
`ack.resolved` / `result.resolved` ile geri verir. Tipli mesaj, günlük ve öneriler yalnız
kimlik görür; motorun geri kalanı (açılım, uygulama, yeniden oynatma) değişmedi.

## 2. Sözlük

| Tür | Anahtarlar | Tür sırası (`first`/`latest` ve aday sırası) |
|-----|------------|------|
| seri `SeriesSel` | `kind` (bars/study/points), `study`, `params`, `on`, `symbol`, `label` | doküman |
| görünüm `ViewSel` | `title`, `shows` | doküman |
| panel `PanelSel` | `name`, `shows`, `mark` | yığın (üstten alta) |
| katman `LayerSel` | `shows`, `output`, `mark`, `panel`, `value`, `fact`, `finding`, `label` | çizim |
| olgu `FactSel` | `kind`, `on`, `bar` | bar, sonra doküman |
| bulgu `FindingSel` | `kind`, `on`, `fact`, `state` | en büyük olgu barı, sonra doküman |
| öneri `ProposalSel` | `state` | doküman |
| ölçek `ScaleSel` | `panel` | — (panel çözülür, `"y<id>"`) |
| öğe `ItemSel` | tek anahtar: `series` / `view` / `fact` / `finding` / `proposal` | alt türün |

Her seçicide `first`, `latest`. 35 anahtar adı, 9 tür; iç içe alanlar (`shows`, `on`,
`panel`, `fact`, `finding`) kimlik ya da seçici alır.

**Neden bu anahtarlar:** her biri `welcome.summary`'de zaten görünen bir niteliğe
karşılık gelir (`kind`, `study`, `params`, `label`, panel `name`, katman `mark`, olgu
`kind`/`bar`, kapsül durumu). Ajan özetin diliyle seçici yazar. `shows` "bunu çizen"
ilişkisini (görünüm/panel/katman → seri) tek adla kurar; `on` "bunun üstünde/hakkında"
ilişkisini (hesap → girdi, olgu → seri). Görev metinlerindeki kalıplar: "RSI(14)'ün paneli"
`{"shows":{"study":"rsi","params":{"period":14}}}`, "fiyat paneli" `{"mark":"candle"}`,
"üst bant" `{"shows":{"study":"bollinger"},"output":"upper"}`, "70 çizgisi"
`{"mark":"rule","value":70}`, "son kırılım" `{"kind":"breakout","latest":true}`.

## 3. Eşleşme ve sıralama

1. Verilen her anahtar tutmalı (VE); `{}` hepsine uyar ("tek görünüm" = `"view":{}`).
2. Metin büyük/küçük harf duyarsız tam eşitlik; sayılar `f64` tam eşitlik. Benzerlik,
   puan, olasılık yok.
3. İç içe seçici = uyduğu öğeler kümesi (kendi `first`/`latest`'inden sonra). Boş küme →
   iç içe seçicinin yolunda `not_found` (ajan hatanın nerede olduğunu görür).
4. Bir eşleşme: çözülür. Birden çok: `first`/`latest` seçer, yoksa `ambiguous`.
   Hiç: `not_found`.
5. **Sıralama kuralı** (adaylar, `first`, `latest` için tek kural): türün sırası (tablo).
   `ambiguous` adayları bu sırayla (en çok 10) — böylece hatayı okuyan ajan "ilkini
   istiyorsam `first`" diye düzeltebilir; ayrı bir "en iyi" puanı yok, çünkü hepsi aynı
   derecede uyuyor. `not_found` adayları: en çok anahtarı tutanlar önce, eşitlikte türün
   sırası (en çok 5); metin değeri dokümanda hiç yoksa `did_you_mean` dokümandaki
   değerlerden.
6. Komutta işlemler sırayla çözülür: n'inci işlemin seçicileri önceki işlemlerin
   uygulandığı taslağa karşı (aynı komutta eklenen seriye/katmana seçiciyle başvurulur).

Panel/katman, işlemin `view`'ındaki görünümde aranır (hedefte `target.view`, yoksa
`set_emphasis`'in görünümü, o da yoksa bütün görünümler).

## 4. Uygulama

- `crates/qp-session/src/select.rs`: mesajdaki kimlik alanlarının yerleri (`sites`, işlem
  türüne göre tablo), çözücü (`Resolver`), hatalar, özet ipuçları.
- `Session::handle_raw`: ayrıştırmadan önce `resolve_selectors`. Seçici yoksa maliyet bir
  JSON gezintisi. Komutta seçici varsa işlem başına bir taslak kopya ve dry-run açılım
  (yalnız son seçicili işleme kadar).
- `locate.rs`: etiketsiz birleşimde (id | seçici) hata, değerin JSON tipine uyan dalınki
  (nesne verildiyse seçicinin "bilinmeyen anahtar" hatası, `did_you_mean` ile). Var olan
  17 döküm ve testler değişmeden geçiyor.
- Şema: seçici türleri ve `*Ref` (kimlik | seçici) türleri `protocol.rs`'te; işlem alanları
  Rust'ta `String`/`u32` kaldı, `#[schemars(with = …)]` ile şemada "kimlik ya da seçici".
  İç içe yapılar için tel biçimleri (`LayerIn`, `EncodingIn`, `TargetIn`, `ConditionIn`…);
  alan adlarının saklanan tiplerle aynı olduğunu `wire_forms_mirror_stored_types` denetler.
  MCP araçları şemadan üretildiği için seçici kabul ediyor (elle denendi: `op_set_encoding`
  ve `query_study` seçiciyle çalıştı, `resolved` döndü).
- `welcome.summary.select` (`SelectHints`): her öğe için şu an ona çözülen en kısa seçici;
  `screen` panellerinde `select`. İpuçları aynı çözücüyle üretilir, kendi öğesine çözüldüğü
  testte denetleniyor.

## 5. Yirmi görev bu tasarımla

Hepsi `crates/qp-session/tests/selectors.rs::r_tasks`'te, gizli kurulumun eşiyle (SMA50,
bar 120/185 kırılım olguları ve bulguları) **dokümanın hiçbir kimliği yazılmadan**
çalıştırılıyor ve ölçütleri denetleniyor (20/20). R14: `r2/int` (0cfaa26) birleştirmesinden
sonra nokta sorgusu çalışıyor; `hit` görünüm seçicisini çözüyor ve `result.resolved`
döndürüyor (test artık gerçek `hit` cevabı istiyor). Ajanın kendi seçtiği yeni kimlikler
(`rsi7`) serbest.

| # | Mesaj (işlemler / sorgu) |
|---|------|
| R01 | `add_study {id:"rsi7", study:"rsi", inputs:[{"kind":"bars"}], params:{period:7}}` + `insert_panel {view:{}, panel:{name:"rsi7", y:{kind:"linear"}}, after:{"mark":"candle"}, layers:[{mark:"line", series:"rsi7"}]}` |
| R02 | `split_layer {view:{}, layer:{"shows":{"study":"bollinger"}}}` + `set_encoding {view:{}, layer:{"output":"upper"}, encoding:{color:"down"}}` (T9 sonrası; eski yol — `y:"middle"` + iki `insert_layer` — da çalışır) |
| R03 | `set_emphasis {view:{}, emphasis:[{target:{item:{"finding":{"kind":"breakout","latest":true}}}, kind:"highlight"}]}` |
| R04 | `{"kind":"present","finding":{"kind":"breakout","latest":true},"surface":"watch"}` |
| R05 | `set_scale {view:{}, scale:{"panel":{"mark":"candle"}}, kind:"log"}` |
| R06 | `insert_layer {view:{}, layer:{mark:"rule", panel:{"shows":{"study":"rsi"}}, encoding:{y:30}}}` (köken `rsi.oversold` otomatik) |
| R07 | `delete_panel {view:{}, panel:{"shows":{"study":"macd"}}}` |
| R08 | `insert_panel {view:{}, panel:{name:"volume", y:{kind:"linear"}}, after:{"mark":"candle"}, layers:[{mark:"bar", series:{"kind":"bars"}, encoding:{y:"v"}}]}` |
| R09 | `insert_layer {view:{}, layer:{mark:"band", series:{"study":"bollinger"}, panel:{"mark":"candle"}, encoding:{y:"upper", y2:"lower", opacity:0.2}}}` |
| R10 | `insert_layer {view:{}, layer:{mark:"bar", series:{"study":"macd"}, panel:{"shows":{"study":"macd"}}, encoding:{y:"histogram", color:{when:{field:"histogram", op:"ge", than:0}, then:"up", else:"down"}}}}` |
| R11 | `insert_layer {view:{}, layer:{mark:"line", series:{"kind":"bars"}, panel:{"mark":"candle"}, encoding:{y:"c", color:{when:{field:"c", op:"ge", than:{series:{"study":"sma","params":{"period":50}}, field:"sma"}}, then:"up", else:"down"}}}}` |
| R12 | `set_view_range {view:{}, from_bar:140, to_bar:199}` (bar sayısı özetteki `len`'den; seçiciyle ilgisiz) |
| R13 | `{"kind":"screen"}` (paneller `select` taşır) |
| R14 | `{"kind":"hit","view":{},"x":900,"y":200,"width":1200,"height":600}` |
| R15 | `set_encoding {view:{}, layer:{"shows":{"study":"rsi"}}, encoding:{width:2}}` |
| R16 | `insert_layer {view:{}, layer:{mark:"point", fact:{"kind":"breakout","first":true}, panel:{"mark":"candle"}}}` |
| R17 | `add_fact {id:"lvl-12345", kind:"level", series:{"kind":"bars"}, bar:199, text:"…", evidence:{level:123.45}}` + `insert_layer {view:{}, layer:{mark:"rule", panel:{"mark":"candle"}, encoding:{y:123.45}, origin:{"/encoding/y":{kind:"fact", fact:"lvl-12345", field:"level"}}}}` |
| R18 | `move_panel {view:{}, panel:{"shows":{"study":"macd"}}, before:{"shows":{"study":"rsi"}}}` |
| R19 | `{"kind":"present","finding":{"kind":"breakout","latest":true},"surface":"phone"}` |
| R20 | `insert_panel {view:{}, panel:{y:{kind:"linear"}}, layers:[{mark:"line", series:{"study":"bollinger"}, encoding:{y:"bandwidth"}}]}` |

Gözlem: yirmi görevde kullanılan sözlük çok dar — `view:{}`, panelde `mark`/`shows`,
katmanda `shows`(+`output`), seride `kind`/`study`/`params`, bulgu/olguda
`kind`+`first`/`latest`. `on`, `symbol`, `label`, `title`, `name`, `value`, `state`
görevlerde gerekmedi (ama özet ipuçlarında `value` "70 kuralı" için çıkıyor).

## 6. Elenen seçenekler

| Seçenek | Neden elendi |
|---------|--------------|
| Doğal dil ya da bulanık eşleşme ("rsi paneli" metni, düzenleme uzaklığı ile en yakın) | Determinizm ve "rastgele seçme" yasağıyla çelişir; yanlış ama emin bir eşleşme sessiz hata üretir. Yazım hatası `did_you_mean` ile önerilir, seçilmez. |
| Belirsizlikte puanla "en iyi"yi seçmek | Alt sınır: belirsizse seçme. Puan aday sırasında bile kullanılmadı (hepsi aynı derecede uyuyor); aday sırası = `first`/`latest`'in sırası, tek kural. |
| Seçicileri Rust tipine taşımak (`Op` alanları `SeriesRef`) | Tip olarak en dürüstü, ama açılım/uygulama kodunun (≈3 000 satır) her kimlik okumasını değiştirir; günlük tipinde `Ref::Id` taşınırdı. Çözümü ayrıştırmadan önce yapmak motorun geri kalanını hiç değiştirmedi; şemada tarif `schemars(with)` ile. |
| Her yerde türü saran seçici (`"series":{"series":{…}}`) | Tek biçim ama her alanda kekeme; alanın türü zaten belli. Tür yalnız çok türlü alanlarda (`target.item`, `about`, `remove.id`, `get.id`) yazılıyor. |
| Komutu başlangıç dokümanına karşı tek seferde çözmek | Aynı komutta eklenen katmana/seriye seçiciyle başvurulamazdı. Sıralı çözüm ek maliyetli (işlem başına taslak), yalnız seçici varsa. |
| Varlık durumuna bakan seçiciler ("ekranımdaki görünüm") | Çözüm dokümana karşı olmalı; bağlantı durumu günlüğe girmez, yeniden oynatılamaz. |
| İpuçlarını özet öğelerinin içine koymak (`series[i].select`) | Var olan özet testi öğeleri tam karşılaştırıyor; ayrı `summary.select` haritası ek ve tipli. |
| Kimliği seçicinin içinde de kabul etmek (`{"id":"rsi14"}`) | İki yazım, aynı anlam; kimlik zaten alanın kendisine yazılabiliyor. |
| `nth` / sıra numarası | Görevlerde gerekmedi; `first`/`latest` + bir anahtar yetti. Gerekirse eklenir (ek). |

## 7. Bilinen zayıflıklar

1. **`view` her işlemde zorunlu.** Tek görünümde `"view":{}` yazmak gerekir; bilmeyen ajan
   `view` alanını atlar → `schema` (eksik alan). Şartname örnekleri bunu göstermeli.
2. **Doküman büyüdükçe seçici belirsizleşir.** `{"study":"rsi"}` ikinci bir RSI eklenince
   `ambiguous` olur; hata adayları ve düzeltme yolunu verir ama bir tur harcar. Kalıcı bir
   kayıtta (anlatım, niyet) zaten kimlik saklanıyor, sorun yalnız yazım anında.
3. **R02** (T9'da düzeltildi, §9): `output` artık yalnız o çıktıyı tek başına çizen katmana
   uyar; bölünmemiş katmanda `invalid` + hazır `split_layer` dizisi döner.
4. **Sözlük bilgisi gerekiyor**: `mark:"candle"` = "fiyat paneli" çıkarımını ajan yapmalı;
   panel adı yoksa "price" adıyla başvurulamaz (adlar dokümanda varsa `name` çalışır).
5. **Şema uzlaşması**: `Op` hem istek hem saklanan istek (günlük, öneri) tipi; saklanan
   isteğin şeması da seçiciye izin verir görünür (otorite yalnız kimlik yazar). Tel
   biçimleri (`LayerIn`…) saklanan tiplerin aynası; ad uyumu testle korunuyor, belge
   metinleri elle.
6. **Hata sırası değişti** (yalnız seçicili mesajlarda): seçici hatası, mesajın başka bir
   yerindeki şema hatasından önce gelir.
7. **Kimlik alanı olmayan yerler**: `put_view`, `origin` (köken olgusu), `input` hareketleri,
   niyet/kapsül/anlatım kimlikleri seçici almaz.
8. **`resolved`'da panel kimliği dizge** (`"0"`), mesajda sayı.
9. `feed` rolü seçici kullanamaz (okuma yetkisi yok; çözüm dokümanı sızdırırdı).
10. Seçicili komutta işlem başına doküman kopyası: büyük dokümanda (10 000 bar) maliyet
    ölçülmedi.

## 9. T9 sonrası düzeltmeler

T9 ölçümü bu tasarımı seçti: ilk denemede 19/20 (kimliksiz; B 10/20, taban 2/20), sonunda
19/20. Tek başarısızlık R02: `{output:"upper"}` üç çıktıyı birlikte çizen tek Bollinger
katmanına uydu, `set_encoding` üç bandı birden boyadı ve `applied` döndü — sessiz yanlış
sonuç. Düzeltmeler:

1. **`output` kuralı daraltıldı:** katman o çıktıyı **tek başına** çizmeli (`y` onu
   adlandırır ya da `y` yok ve tek varsayılan çıktısı odur). Birlikte çizen katman uymaz.
   Hiçbir katman uymuyor ama çıktıyı başkalarıyla çizen bir katman varsa `invalid`:
   aday o katman, `hint` hazır dizi
   `[{"op":"split_layer","view":"main","layer":"l1"},{"op":"set_encoding","view":"main","layer":{"output":"upper"},"encoding":{…}}]`.
   Böylece aynı hata artık sessiz değil, düzeltme yolu da tek adım.
2. **Yeni işlem `split_layer {view, layer}`:** çok çıktılı `line`'ı çıktı başına katmana
   böler (açık `y`, aynı panel, aynı yer, aynı kodlama; kimlikler en küçük kullanılmayan
   `l<k>`, bölünenin kimliği yeniden kullanılmaz). Değişim `delete_layer` + `insert_layer`'lar,
   geri alma baytı geri getirir; `ack.created` çıktı sırasıyla. Seçiciler işlem işlem
   çözüldüğünden `[split_layer {layer:{shows:{study:"bollinger"}}}, set_encoding
   {layer:{output:"upper"}, …}]` tek komutta çalışır (test:
   `split_then_style_the_upper_band_in_one_command`, `t-select.json`).
3. **Otorite hatası (T9 buldu):** `screen` panelleri panel kimliği sırasıyla veriyordu,
   §5.4 yığın sırası diyor. `move_panel` sonrası doküman [0,2,1], `screen` [0,1,2]
   gösteriyordu. Artık yığın sırası (test: `screen_lists_panels_in_stack_order`); RD1'in 17
   dökümü değişmeden geçiyor (orada kimlik sırası = yığın sırası).

## 8. Doğrulama

- `cargo test --workspace` yeşil; RD1'in 17 dökümü `inproc`, `serve`, `mcp` taşımalarında
  geçiyor; `t-select.json` (yeni döküm: başarı, belirsizlik, `first`/`latest`, bulunamama,
  şema, öneri, günlükte kimlik) üç taşımada geçiyor.
- `selectors.rs`: 20 görev (kimliksiz), sıralı adaylar ve `first`/`latest`, hata
  iletileri, günlük/öneri kimlikleri ve yeniden oynatma, aynı komutta oluşturulana
  başvuru, `subscribe`/`attend`, `feed` yasağı, ipuçlarının kendi öğesine çözülmesi.
