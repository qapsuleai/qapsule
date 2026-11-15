# Qapsule Alan Protokolü — `qp/0`

Durum: **v0** (RD1; R2'de gramer eki — §5.1 gramer işlemleri, §5.4 `hit`, §4.1 `summary`, §13 `no_origin`/`ambiguous`, §17 anlamla başvuru; hepsi ek, kırıcı değil). Mesaj tiplerinin tek kaynağı `crates/qp-schema/src/protocol.rs`;
JSON Schema oradan üretilir (`qp schema export`): `ClientMsg`, `ServerMsg`, `Op`,
`ChangeOp`, `Query`, `LogEntry`, `InboundLine`, `OutboundLine`. Bu belge ile şema
çelişirse şema doğrudur ve bu belge düzeltilir.

Bu protokol motor (doküman **otoritesi**), arayüzler (masaüstü, telefon, saat, ses) ve
ajanlar arasındaki konuşmadır. Arayüz ve ajan **aynı** mesajları kullanır; hiçbir istemciye
özel bir kapı yoktur (K21-4).

## 0. Hızlı bakış

```
istemci → otorite                      otorite → istemci
-----------------                      -----------------
hello      (kimim, hangi yüzey)   →    welcome  (rol, yetki, profil, sınırlar, seq)
command    (dokümanı değiştir)    →    ack (applied | proposed)  ya da error
query      (anlamsal okuma)       →    result                    ya da error
subscribe / unsubscribe           →    ack, sonra notify …
input      (kaydır, yakınlaştır)  →    result (viewport | at)
attend     (dikkat yönlendir)     →    ack (delivered); hedefe notify
clock      (yalnızca host)        →    (cevap yok; süre aşımları işlenir)
```

Her mesaj tek bir JSON nesnesidir; türü `"type"` alanıdır. Örnek bir konuşma:

```json
→ {"type":"hello","protocol":"qp/0","principal":"scout","surface":{"kind":"headless"}}
← {"type":"welcome","protocol":"qp/0","doc":"doc-thyao","principal":"scout","role":"proposer", "...": "..."}
→ {"type":"query","id":"q1","query":{"kind":"study","series":"rsi14","range":{"last":50},"reduce":"max"}}
← {"type":"result","id":"q1","seq":4,"data":{"kind":"reduced","series":"rsi14","reduce":"max","bars":50,"values":{"rsi":{"value":71.37,"bar":150,"t":1700000000000}}}}
```

## 1. Kavramlar

| Kavram | Kodda | Anlamı |
|--------|-------|--------|
| Otorite | authority | Bir dokümanın tek yazma noktası. Komutları sıraya koyar, günlüğe yazar, değişimi yayar. Belge başına tek otorite vardır (§8). |
| Bağlantı | connection | Bir istemcinin otoriteyle konuştuğu kanal. Her bağlantı `hello` ile başlar. |
| Kişi/ajan kimliği | principal | Konuşanın kimliği (`arkin`, `scout`). Rolü otoritenin yapılandırmasındadır, istemci kendi rolünü söyleyemez. |
| Yüzey | surface | Bağlantının sunum ortamı: `desktop`, `phone`, `watch`, `voice`, `headless`. |
| Profil | profile | Otoritenin yüzeye göre seçtiği sunum derinliği ve veri sınırları (§10). |
| Günlük | log | Dokümana uygulanmış girdilerin sıralı, boşluksuz listesi. `seq` 1'den başlar. |
| İşlem | `Op` | Komuttaki istek: anlam düzeyinde ("RSI ekle", "bulguyu reddet"). |
| Değişim | `ChangeOp` | İşlemin otoritede açıldığı ilkel, yeniden oynatılabilir hali. |
| Öneri | `Proposal` | Onay bekleyen yazı. Dokümanın içinde durur (§6). |
| Varlık durumu | presence | Kalıcı olmayan oturum durumu: her bağlantının görünür aralığı (viewport), odak, dikkat yönergeleri. Günlüğe girmez (§7). |

Bar numaraları her yerde **0 tabanlı sıra numarasıdır** (`bar: 0` ilk bar). Zamanlar
epoch milisaniye, UTC.

Bu belgedeki örnekler `doc-thyao` örnek dokümanının kimliklerini (`px`, `rsi14`, `main`,
`fnd-overbought`…) kullanır. Gerçek bir dokümanın kimlikleri farklıdır: önce
`{"kind":"list","collection":"series"}` (ve `views`, `findings`) ile öğrenin.

## 2. Durum modeli

Otoritenin durumu üç parçadır:

1. **Doküman** (`Document`, `qp-schema`): seriler, görünümler, olgular, bulgular,
   kapsüller, niyetler, karne, anlatımlar (`narratives`) ve öneriler (`proposals`).
   Kalıcıdır; yalnızca günlükten türetilir.
2. **Günlük** (`LogEntry` listesi): her girdi `seq`, `at` (son `clock`), yazar, varsa
   onaylayan, isteği (`request`), değişimleri (`changes`) ve tersini (`inverse`) taşır.
3. **Varlık durumu** (presence): bağlantıların görünür aralığı, odak noktası ve dikkat
   yönergeleri. Geçicidir; günlüğe girmez, yeniden oynatılmaz, ama sorgulanabilir ve
   abone olunabilir. (Vega'nın sinyalleri ve Yjs'nin "awareness" durumu gibi.)

**Değişmez:** başlangıç dokümanı + günlükteki `changes` sırayla uygulanırsa, otoritenin
dokümanı **bayt düzeyinde** aynen elde edilir. "Başlangıç dokümanı" otoritenin yüklediği
**kanonik** biçimdir (ör. hesap parametreleri varsayılanlarla tamamlanıp sıralanmış);
`seq` 0'daki `{"kind":"document"}` sorgusu bu biçimi verir. Yeniden oynatma için hesap kodu, sunucu
(presenter), saat ya da rastgelelik gerekmez; değişimler tam öğeleri taşır.

Hesap serileri (`source.kind = "study"`) dokümanda veri taşımaz; değerleri girdilerinden
otoritede deterministik olarak hesaplanır (`qp-compute`). Bar serileri verilerini
`data.bars` içinde taşır (v0'da yalnızca `inline` ve `study` kaynakları desteklenir; bkz. §14).

Bütün öğe kimlikleri (seri, görünüm, olgu, bulgu, kapsül, niyet, anlatım, öneri)
doküman genelinde **tektir**. Kimlik biçimi: `^[A-Za-z0-9][A-Za-z0-9_.:-]{0,63}$`.

## 3. Taşıma ve çerçeve

Protokol taşımadan bağımsızdır. Tanımlı iki taşıma:

- **Süreç içi:** kabuk motoru doğrudan çağırır (`qp_session::Session::handle_raw(conn, json)`),
  dönen mesajları alır.
- **Satır tabanlı akış** (`qp serve`): standart girdi/çıktı; her satır bir JSON nesnesi.
  Birden çok mantıksal bağlantı tek akışı paylaşabilir:

```json
→ {"conn":"a","msg":{"type":"hello", "...": "..."}}
← {"conn":"a","msg":{"type":"welcome", "...": "..."}}
```

  Çerçeve okunamazsa (`conn` yok, satır JSON değil) çıktı `conn: ""` ile bir `error`dur.
  Ayrıştırılamayan bir mesajı test etmek için `msg` bir JSON **dizgesi** olabilir; dizge
  ham mesaj metni olarak otoriteye verilir (`{"conn":"a","msg":"{not json"}`).
  Test koşucuları için `qp serve --batch-end`: her giriş satırının çıktıları bittiğinde
  `{"batch_end":true}` satırı yazılır (taşıma düzeyi; protokolün parçası değil).

- **JSON-RPC eşlemesi** (MCP, LSP benzeri ortamlar için): istemci mesajı
  `{"jsonrpc":"2.0","id":<id>,"method":"qp/<type>","params":<mesajın geri kalanı>}`;
  `ack`/`result` → `result`, `error` → `error.data`, `notify` → bildirim `qp/notify`.
  MCP üzerinde taşıma için bkz. §16 ve `rounds/RD1/EMBEDDING.md`.

## 4. Mesaj türleri

Sekiz tür: **Komut, Değişim, Sorgu, Abonelik, Girdi, Öneri, Dikkat, Yetenek.**
Aşağıda her biri, alanları ve örneğiyle. Alanların tam listesi şemadadır.

### 4.1 Yetenek — `hello` / `welcome`

Her bağlantının ilk mesajı `hello`dur. Yüzey kendini tanıtır; otorite profil seçer.

```json
{"type":"hello","protocol":"qp/0","principal":"arkin",
 "surface":{"kind":"watch","screen":[198,242],"input":["touch","crown"]}}
```

```json
{"type":"welcome","protocol":"qp/0","doc":"doc-thyao","principal":"arkin","role":"owner",
 "permissions":{"read":true,"write":true,"propose":true,"decide":true,"attend":true,"feed":false},
 "seq":4,"now":1760003900000,"epoch":0,
 "profile":{"depth":"sentence","series_data":false,"max_points":0,"max_glyphs":1,"speech":false},
 "limits":{"max_message_bytes":1048576,"max_ops":64,"max_bars":10000,"max_subscriptions":32,"max_string":4096}}
```

- `surface.kind`: `desktop` | `phone` | `watch` | `voice` | `headless`.
- `welcome.now`: otoritenin şimdiki zamanı (son `clock`). "Son yarım saat" gibi göreli
  sorular için buradan ya da her `result`'taki `now`'dan başlanır.
- `surface.max_depth`: yüzey türünün izin verdiğinden daha sığ bir üst sınır istemek için.
- `features`: isteğe bağlı özellikler (`"attention"`, `"narrative"`). v0 otoritesi ikisini de
  destekler; tanımadığı özelliği sessizce dışarıda bırakır.
- `hello` olmadan gelen her mesaj, ikinci bir `hello`, bilinmeyen `principal` ya da
  desteklenmeyen `protocol` → `error` `handshake`.
- `welcome.summary` (R2, `DocSummary`): dokümanın kısa özeti — seriler (kimlik, tür,
  hesap/parametreler, alanlar, açıklama metni `RSI14`), görünümler → paneller → katmanlar
  (kimlik, işaret, ne çizdiği), olgular, bulgular (en son bar), bekleyen öneri sayısı. Ajan
  ilk mesajında kimliklere buradan başvurabilir (`docs/GRAMMAR.md` §12).

### 4.2 Komut — `command` → `ack`

Dokümanı değiştirmenin **tek** yolu. İnsan hareketi de ajan çağrısı da aynı komuttur.

```json
{"type":"command","id":"c1",
 "ops":[{"op":"add_study","study":"rsi","inputs":["px"],"params":{"period":7},
         "show":{"view":"main","with":"rsi14"}}]}
```

```json
{"type":"ack","id":"c1","status":"applied","seq":5,"created":["series-5-0"]}
```

- `ops`: sırayla, **ya hep ya hiç** uygulanır. Biri hata verirse hiçbiri uygulanmaz.
  `accept_proposal` ve `undo` komuttaki **tek** işlem olmalıdır (bir girdinin tek yazarı
  ve tek `undoes`'u vardır); değilse `invalid`.
- `mode`: `auto` (varsayılan) — yetki varsa doğrudan yaz, yoksa öneri oluştur (§6);
  `direct` — doğrudan yaz ya da `forbidden`; `propose` — her zaman öneri.
- `base`: iyimser eşzamanlılık. Komutun dokunduğu öğelerden biri `base`'den sonra
  değiştiyse `conflict` döner (§8.2). Verilmezse komut o anki durumun üstüne uygulanır.
- `reason`: neden; günlüğe ve öneriye yazılır.
- `ack.created`: komutun oluşturduğu öğelerin kimlikleri (istemcinin verdiği ya da
  otoritenin atadığı), işlem sırasıyla. `accept_proposal` için: önerinin işlemlerinin
  oluşturduğu kimlikler.
- Bir komut ayrıştırılamasa da (`schema` hatası) mesajda okunabilen bir `id` varsa hata
  o `id` ile döner.
- `ack.status = "proposed"` ise `ack.proposal` öneri kimliğidir ve doküman değişmemiştir.
- Uygulanan bir `undo` için `ack.undoes` geri alınan girdinin konumudur.

İşlemler (`Op`, alan `"op"`) — ayrıntı §5.

### 4.3 Değişim — `notify` + `event.kind = "change"`

Uygulanan her günlük girdisi, `changes` konusuna abone olan herkese değişim olarak gider.
Yalnızca değişen kısım taşınır; sıralı ve yeniden oynatılabilirdir.

```json
{"type":"notify","sub":"s1","event":{"kind":"change","seq":5,"at":1700000600000,
 "author":"arkin","cause":"c1","changes":[
   {"op":"put_series","series":{"id":"series-5-0","source":{"kind":"study","study":"rsi",
     "inputs":["px"],"params":{"period":7.0}}}},
   {"op":"insert_layer","view":"main","index":5,
    "layer":{"mark":"line","series":"series-5-0","panel":1,"encoding":{}}}]}}
```

İstemci bir yansı tutuyorsa: `seq` = kendi son `seq`'i + 1 olmalıdır; değilse boşluk vardır
ve `{"kind":"log","after_seq":<son>}` ile eksikleri ister ya da `document` ile yeniden
yükler. Değişim türleri (`ChangeOp`) §5.3'te.

### 4.4 Sorgu — `query` → `result`

Anlamsal okuma; dokümanı asla değiştirmez. Cevap `result.data` içinde, `kind` ile
etiketli; `result.seq` cevabın yansıttığı günlük konumu, `result.now` otoritenin
şimdiki zamanıdır.

```json
{"type":"query","id":"q2","query":{"kind":"at","series":"px","bar":150}}
```

```json
{"type":"result","id":"q2","seq":4,"data":{"kind":"at","at":{"series":"px","bar":150,
 "ohlcv":{"t":1700000000000,"o":101.2,"h":103.0,"l":100.8,"c":102.6,"v":1200000.0},
 "studies":{"rsi14":{"rsi":71.37},"macd":{"histogram":0.41,"macd":1.92,"signal":1.51},
            "bb20":{"bandwidth":0.08,"lower":96.1,"middle":100.2,"upper":104.3}}}}}
```

Sorgu türleri §5.4'te.

### 4.5 Abonelik — `subscribe` → `ack`, sonra `notify`

```json
{"type":"subscribe","id":"s2","topic":{"kind":"finding","finding":"fnd-overbought"}}
{"type":"ack","id":"s2","status":"subscribed"}
… sonra bulgu geçersizleşince:
{"type":"notify","sub":"s2","event":{"kind":"finding_state","finding":"fnd-overbought",
 "capsule":"cap-overbought","state":"invalidated","seq":9}}
```

Konular (`topic.kind`):

| Konu | Alanlar | Olay(lar) |
|------|---------|-----------|
| `changes` | `ids?` | `change` (yalnızca bu kimliklere dokunan girdiler, `ids` verilirse) |
| `finding` | `finding` | `finding_state` |
| `condition` | `condition` | `condition_met` (koşul **doğru olduğu anda**, kenar tetiklemeli) |
| `proposals` | — | `proposal` (oluştu, kabul, ret, süre aşımı) |
| `attention` | `principal?` (varsayılan: abone) | `attention` (yönerge), `viewport` (bakış değişti) |
| `intents` | — | `intent_triggered` |

Abonelik kimliği `subscribe` mesajının `id`'sidir. `unsubscribe {"id":…, "sub":"s2"}`.
Bir bağlantı en çok `limits.max_subscriptions` abonelik tutar.

### 4.6 Girdi — `input` → `result`

Etkileşim çekirdekte yaşar: kabuk ham hareketi gönderir, motor yorumlar. Girdi varlık
durumunu değiştirir (bakılan aralık, odak), **dokümanı değiştirmez**. Kalıcı bir aralık
için komut `set_view_range` kullanılır.

```json
{"type":"input","id":"i1","gesture":{"kind":"zoom","view":"main","factor":2.0}}
{"type":"result","id":"i1","seq":4,"data":{"kind":"viewport","viewport":{"view":"main","from_bar":170,"to_bar":199}}}
```

| Hareket | Alanlar | Etki |
|---------|---------|------|
| `look` | `view`, `from_bar`, `to_bar` | Yüzey görünen aralığı bildirir. |
| `pan` | `view`, `bars` (+ yeniye) | Aralığı kaydırır; sınırlara kırpılır. |
| `zoom` | `view`, `factor` (>1 yakınlaştırır), `anchor_bar?` | Genişlik = round(genişlik / factor), en az 2 bar; `anchor_bar` (varsayılan: son görünen bar) ekranda aynı göreli konumda kalır. |
| `tap` | `view`, `panel`, `bar` | Odak o bar olur; cevap `at` sonucudur. |

**Geçici mi kalıcı mı?** Kullanıcının kendi ekranındaki hareketi (kaydırma, yakınlaştırma,
"grafiği yakınlaştır") `input`tur ve yalnız o bağlantının bakışını değiştirir. Herkesin
gördüğü kalıcı aralık ("görünümü son 60 bara ayarla, böyle kalsın") `set_view_range`
komutudur ve günlüğe girer.

Her `look`, `pan` ve `zoom`, aralık değişmese bile kişinin `attention` abonelerine
`viewport` olayı üretir. `tap` olay üretmez (cevabı `at`'tir; odağı günceller). `tap` ve
görünümle ilgili bar numaraları görünümün **ilk bar serisine** göredir.

Bağlantının başlangıç aralığı: görünümün `x.domain`'i varsa ona düşen barlar; yoksa son
N bar (masaüstü 120, telefon 60, saat 20; ekransız yüzeylerde de aynı kural, N = 120).

### 4.7 Öneri — `command` (öneri kipi) ve karar işlemleri

Ajanlar varsayılan olarak **öneri** yazar; yetkiyle doğrudan yazar (K21-5). `proposer`
rolündeki bir ajanın `mode: auto` komutu otomatik olarak öneriye dönüşür:

```json
{"type":"command","id":"c7","reason":"RSI 70'i yukarı kesti",
 "ops":[{"op":"add_finding","claim":"Momentum güçlü.","facts":["fact-rsi-cross70"],
         "confidence":0.6,
         "invalidation":{"kind":"threshold","series":"rsi14","op":"cross_below","value":70.0},
         "capsule":"Momentum güçlü"}]}
{"type":"ack","id":"c7","status":"proposed","seq":6,"proposal":"prop-6","created":["fnd-6-0","cap-6-1"]}
```

Kimlikler öneri anında atanır (yukarıda `fnd-6-0`), kabul edilirse aynen kullanılır.
Öneri oluşturulmadan önce işlemler o anki dokümana karşı **denetlenir** (uygulanmadan);
hata varsa öneri oluşmaz ve hata hemen döner. Şu işlemler öneri olamaz; yetkisi olmayan
için her kipte `forbidden` döner: `accept_proposal`, `reject_proposal`, `undo`,
`append_bars`, `update_last_bar`.
Karar, `decide` yetkisi olan birinin komutudur:

```json
{"type":"command","id":"c8","ops":[{"op":"accept_proposal","proposal":"prop-6"}]}
{"type":"ack","id":"c8","status":"applied","seq":7,"created":["fnd-6-0","cap-6-1"]}
```

Kabulde önerinin işlemleri **o anki** doküman üzerinde açılır. Oluşan girdinin yazarı
öneren (`scout`), onaylayanı karar veren (`arkin`) olur; oluşan öğelerin
`provenance.approved_by` alanı karar vereni taşır. Açılım hata verirse (ör. dayandığı olgu
silinmiş) kabul `error` döner ve öneri `pending` kalır. Ret: `reject_proposal` (not ile).
Süre aşımı: otoritenin yapılandırmasında `proposal_ttl_ms` varsa, `clock` o süreyi
geçtiğinde öneri `expired` olur (yazarı `qp`, motor). Bekleyen öneriler:
`{"kind":"list","collection":"proposals","state":"pending"}`.

### 4.8 Dikkat — `attend` ve `attention` sorgusu

Dikkat iki yönlüdür ve **dokümanı değiştirmez**:

- **Kullanıcının baktığı yer:** bağlantıların `input` ile bildirdiği aralık ve odak.
  Okuma: `{"kind":"attention","principal":"arkin"}`. Abonelik: `attention` konusu.
- **Ajanın baktırmak istediği yer ve sırası:** `attend` yönergesi.

```json
{"type":"attend","id":"a1","directive":{"to":"arkin","steps":[
  {"target":{"view":"main","panel":1,"series":"rsi14","from_bar":150,"to_bar":150},
   "say":"RSI burada 70'i kesti."},
  {"target":{"view":"main","panel":0,"series":"px","from_bar":199,"to_bar":199},
   "say":"Son barın hacmi."}]}}
{"type":"ack","id":"a1","status":"delivered","created":["att-1"]}
```

Yönerge, hedef kişinin `attention` aboneliklerine `notify` olarak gider ve
`attention` sorgusunda `directives` altında görünür (en son 16 yönerge). `from` ve `at`
otorite tarafından doldurulur. Gerekli yetki: `attend`.

Kalıcı, adım adım bir **anlatım çizelgesi** dikkat değil, dokümandır: `add_narrative` (§11).

## 5. Katalog

### 5.1 İşlemler (`Op`, komutta)

| `op` | Alanlar (`?` isteğe bağlı) | Etki / açılım |
|------|----------------------------|---------------|
| `put_series` | `series` | Seriyi ekle/değiştir. |
| `add_study` | `id?`, `study`, `inputs`, `params?`, `show?{view, with?, panel?}` | Hesap serisi ekler; `show` varsa görünüme `line` katmanı ekler. `with`: o serinin çizildiği panel; `panel`: panel numarası; ikisi de yoksa yeni panel (en büyük panel + 1). |
| `set_params` | `series`, `params` | Hesap serisinin parametrelerini birleştirerek değiştirir; kimlik ve katmanlar korunur. Değişim: serinin tamamıyla `put_series`. |
| `put_view` | `view` | Görünümü ekle/değiştir (gramer v0: `docs/GRAMMAR.md`). |
| `add_layer` | `view`, `layer`, `index?` | Katman ekler (yoksa sona). **`index` kullanımdan kalktı** (eşzamanlı yazıda kayar); `insert_layer` kullanın. |
| `remove_layer` | `view`, `index` | **Kullanımdan kalktı:** katmanı konumla siler; `delete_layer` kullanın. |
| `insert_layer` | `view`, `layer`, `after?`, `before?` | (R2) Katmanı kimlikle yerleştirir: `after` katmanından hemen sonra, `before`'dan hemen önce, yoksa sona. `layer.panel` var olan bir panel id'si. Sabitlerin kökeni denetlenir (§13 `no_origin`). Değişim: `insert_layer` (konumla, otoritenin hesapladığı). |
| `delete_layer` | `view`, `layer` | (R2) Katmanı kimlikle siler. |
| `split_layer` | `view`, `layer` | (R2/T8a) Çok çıktılı bir `line` katmanını (`y`'siz; Bollinger: `upper`/`middle`/`lower`, MACD: `macd`/`signal`) çıktı başına bir katmana böler: her biri açık `y` ile, aynı panelde, aynı çizim yerinde (çıktı sırasıyla), aynı kodlamayla. Yeni kimlikler en küçük kullanılmayan `l<k>` (bölünen kimlik yeniden kullanılmaz). Değişim: `delete_layer` + `insert_layer`'lar (tersi baytı geri getirir). `ack.created`: yeni katman kimlikleri, çıktı sırasıyla. Tek çıktılı katman → `invalid`; vurgu hedefiyse `in_use`. |
| `move_layer` | `view`, `layer`, `panel?`, `after?`, `before?` | (R2) Katmanı başka panele ve/veya çizim sırasında başka yere taşır. Değişim: `put_layer`. |
| `set_encoding` | `view`, `layer`, `encoding`, `unset?`, `origin?`, `label?` | (R2) Verilen kanalları değiştirir, `unset`'tekileri kaldırır, diğerleri kalır. Değişim: `put_layer`. |
| `insert_panel` | `view`, `panel`, `layers?`, `after?`, `before?` | (R2) Panel ekler (`after` panelinin hemen altına, `before`'un hemen üstüne, yoksa en alta), isteğe bağlı katmanlarıyla. `ack.created`: panel id'si (ondalık), sonra katman kimlikleri. |
| `delete_panel` | `view`, `panel` | (R2) Paneli ve içindeki katmanları siler. |
| `move_panel` | `view`, `panel`, `after?`, `before?` | (R2) Paneli yığında taşır. |
| `set_panel` | `view`, `panel`, `name?`, `weight?` | (R2) Panelin adı/göreli yüksekliği. |
| `set_scale` | `view`, `scale`, `kind?`, `domain?`, `auto_domain?` | (R2) Ölçeği kimlikle değiştirir (`x`, `y<panel>`): tür (`linear`/`log`), sabit alan ya da otomatik. |
| `set_emphasis` | `view`, `emphasis` | (R2) Görünümün kalıcı vurgu listesini değiştirir. |
| `set_view_range` | `view`, `from_bar`, `to_bar` | Görünümün kalıcı yatay aralığı: `x.domain = [t(from_bar), t(to_bar)]` (görünümün ilk bar serisine göre). |
| `add_fact` | `id?`, `kind`, `series`, `bar`, `text`, `evidence?` | Olgu ekler (`t` bar zamanından). |
| `add_finding` | `id?`, `claim`, `facts`, `confidence`, `invalidation`, `capsule?` | Bulgu ekler; `capsule` (başlık) verilirse canlı kapsül de. |
| `update_finding` | `finding`, `claim?`, `confidence?`, `invalidation?` | Verilen alanları değiştirir. |
| `approve_finding` | `finding`, `note?` | Kapsül `approved`; karneye kayıt (`state: approved`, `outcome: pending`, not). |
| `reject_finding` | `finding`, `note?` | Kapsül `rejected`; karneye kayıt (`state: rejected`, `outcome: pending`, `recorded_at` = saat, not). |
| `add_intent` | `id?`, `text`, `trigger?` | Etkin niyet ekler. |
| `set_intent_active` | `intent`, `active` | Niyeti açar/kapatır. |
| `add_narrative` | `id?`, `about`, `steps` | Anlatım çizelgesi ekler (§11). |
| `show_finding` | `finding`, `view?` | **Sunucu (presenter)** bulgu için deterministik bir görünüm kurar ve kapsüle bağlar (§10.3). |
| `remove` | `id` | Öğeyi siler. Seri silinirse onu çizen katmanlar da silinir (değişimler: önce `delete_layer`'lar en büyük sıradan küçüğe, sonra `delete`); başka öğe ona dayanıyorsa `in_use` (seri: hesap girdisi, olgu, bulgu/niyet koşulu; olgu: bulgu; bulgu: kapsül, anlatım; görünüm: kapsül; öneriler silinemez) ve `expected` = dayanan öğelerin kimlikleri, alfabe sırasıyla. |
| `append_bars` | `series`, `bars` | Bar ekler (zamanlar son bardan sonra, artan). Yetki: `feed` ya da `write`. |
| `update_last_bar` | `series`, `bar` | Son barı değiştirir (aynı `t`). |
| `undo` | `seq?` | Bir girdiyi tersini yeni girdi olarak yazarak geri alır (§8.3). |
| `accept_proposal` | `proposal`, `note?` | Öneriyi kabul eder (§4.7). Yetki: `decide`. |
| `reject_proposal` | `proposal`, `note?` | Öneriyi reddeder. Yetki: `decide`. |

Atanan kimlikler: `<tür>-<seq>-<n>`; `tür` ∈ `series`, `view`, `fact`, `fnd`, `cap`,
`intent`, `narr`; `seq` girdinin konumu, `n` o girdide oluşturulan öğenin sırası (0'dan).
Öneri kimliği `prop-<seq>`.

**Hesaplar ve parametreleri:** `sma`, `ema`, `rsi` → `period` (varsayılan 20, 20, 14);
`macd` → `fast` 12, `slow` 26, `signal` 9; `bollinger` → `period` 20, `k` 2. Çıktılar:
`sma`, `ema`, `rsi` → aynı adlı tek çıktı; `macd` → `macd`, `signal`, `histogram`;
`bollinger` → `upper`, `middle`, `lower`, `bandwidth`. Bir koşulda hesap serisinin
**ana çıktısı** kullanılır: hesabın adıyla aynı olan çıktı (`bollinger` için `middle`),
ya da serinin `source.output`'u. Bar serisinde koşul değeri kapanıştır (`c`).

**Koşullar** (`Condition`, `kind`): `threshold {series, op, value}`,
`compare {left, op, right}`, `text {text}`. `op` ∈ `lt`, `le`, `gt`, `ge`, `cross_above`,
`cross_below`. `lt…ge` son barda değerlendirilir; `cross_above`: önceki bar `< ` sağ taraf
ve son bar `>=` sağ taraf (önceki değer boşsa yanlış); `cross_below` simetriği.

**Durum mu, kesişim mi?** `lt`/`le`/`gt`/`ge` bir **durumdur** (son barda doğru mu);
`cross_*` bir **olaydır** (yalnızca kesişimin olduğu barda doğru). Geçersizlik koşulu ve
niyet tetikleyicisi her girdiden sonra yeniden değerlendirilir; bu yüzden:

| Doğal dil | Koşul |
|-----------|-------|
| "fiyat 100'ün altına inerse geçersiz say" / "100'ün altındaysa" | `{"kind":"threshold","series":"px","op":"lt","value":100}` |
| "RSI 70'i aşağı keserse" | `{"kind":"threshold","series":"rsi14","op":"cross_below","value":70}` |
| "SMA20, SMA50'yi yukarı keserse haber ver" | `{"kind":"compare","left":"sma20","op":"cross_above","right":"sma50"}` |

Bir durum koşulu, olay kaçırılsa bile (ör. fiyat boşlukla 100'ün altında açılırsa)
yakalanır; bir kesişim koşulu yalnızca kesişim barında tetiklenir.

### 5.2 Örnek: üç işlem tek komutta

```json
{"type":"command","id":"c3","ops":[
  {"op":"add_study","id":"sma20","study":"sma","inputs":["px"],"params":{"period":20},"show":{"view":"main","panel":0}},
  {"op":"add_study","id":"sma50","study":"sma","inputs":["px"],"params":{"period":50},"show":{"view":"main","panel":0}},
  {"op":"add_intent","text":"SMA20 SMA50'yi yukarı keserse haber ver",
   "trigger":{"kind":"compare","left":"sma20","op":"cross_above","right":"sma50"}}]}
```

### 5.3 Değişimler (`ChangeOp`)

`put_series`, `put_view`, `insert_layer`, `delete_layer`, `put_layer`, `put_panel`,
`delete_panel`, `set_emphasis`, `set_x_domain`, `put_fact`,
`put_finding`, `put_capsule`, `set_capsule_state`, `put_intent`, `push_ledger`,
`pop_ledger`, `put_narrative`, `put_proposal`, `delete`, `append_bars`, `truncate_bars`,
`set_last_bar`. Her biri tam öğe taşır; `put_*` değişimlerinin isteğe bağlı `index` alanı
yeni öğenin koleksiyondaki yeridir (yoksa sona; var olan öğe yerinde değiştirilir) —
silmenin tersi öğeyi eski yerine koyar, böylece sil + geri al dokümanı bayt düzeyinde
aynı bırakır; köken (`provenance`) otorite tarafından damgalanır
(istemci köken yazmaz). İstemcinin `Op`'ları değişimlere açılır; örneğin
`reject_finding` → `set_capsule_state` + `push_ledger`; `add_study` (+`show`) →
`put_series` + `insert_layer`.

Gramer değişimleri (R2): değişimler otoritenin sırasıyla uygulandığından konum taşıyabilir
(eşzamanlılık sorunu isteklerde, değişimlerde değil). `put_layer {view, index, layer}`
aynı kimlikli katmanı kaldırıp `index`'e koyar (değiştirme + taşıma; tersi eski katman ve
eski konum); `delete_layer` artık isteğe bağlı `layer` kimliğini de taşır (uygulamada
denetlenir); `put_panel {view, index, panel}`, `delete_panel {view, panel}`,
`set_emphasis {view, emphasis}`. Çakışma denetimi (`base`) katman düzeyindedir: dokunulan
anahtarlar `görünüm/katman` (`main/l3`) ve yapısal değişimler için `görünüm/panels`;
böylece aynı görünümün farklı katmanlarına eşzamanlı yazılar çakışmaz.

### 5.4 Sorgular (`Query`, alan `"kind"`)

| `kind` | Alanlar | Cevap (`data.kind`) |
|--------|---------|---------------------|
| `describe` | — | `description`: işlemler, sorgular, konular, hesaplar ve varsayılan parametreleri, kişiler ve rolleri, sorananın yetkileri. |
| `get` | `id` | `item`: öğe ve koleksiyonu. |
| `list` | `collection`, `state?`, `full?` | `items`. Koleksiyonlar: `series`, `views`, `facts`, `findings`, `capsules`, `intents`, `narratives`, `proposals`; `state` yalnızca öneriler için. Seriler varsayılan olarak **verisiz** listelenir (barlar büyüktür); `full: true` ya da `get` veriyi de verir; değerler için `study`/`at`. |
| `document` | — | `document`: dokümanın tamamı (anlık görüntü; devir için). |
| `log` | `after_seq?`, `since_t?`, `until_t?`, `within_ms?`, `author?`, `limit?` | `log`: girdiler, `seq` artan. `within_ms`: `now`'dan geriye bu kadar milisaniye ("son 30 dakika" = `1800000`). `limit` en son n girdiyi verir. |
| `study` | `series`, `range?`, `output?`, `reduce?` | `values` (barlarla hizalı, ısınma `null`; K13/K20) ya da `reduced` (`min`, `max`, `mean`, `first`, `last`, `count`; `null`lar atlanır; `min`/`max`/`first`/`last` bar ve zamanı da verir). Bar serisinde çıktılar `o`,`h`,`l`,`c`,`v`. |
| `at` | `series`, `bar?` ya da `t?` | `at`: OHLCV ve o bar serisine dayanan her hesabın o bardaki değerleri. |
| `screen` | `principal?` | `screen`: kişinin ekranlı (`desktop`, `phone`, `watch`) her bağlantısı için, bağlantı açılış sırasıyla: görünüm, görünen aralık, sembol ve aralık (görünümün ilk bar serisinden), paneller (yığın sırasıyla; `panel` = panel id'si; R2'de ayrıca `name`, `scale` {id, tür, görünen alan} ve `layers` — kimlik, işaret, veri, alanlar, açıklama, sabitler ve kökenleri, görünen öğe sayısı, son değerler, sahne tarifinden; `series` = o paneldeki katmanların serileri, katman sırasıyla, tekrarsız; `marks` = bütün katmanların işaretleri, katman sırasıyla), olgu barı görünen aralıkta olan bulguların kapsülleri, görünen son barın kapanışı. |
| `attention` | `principal` | `attention`: kişinin en son etkin ekranlı bağlantısının bakış aralığı (girdi yoksa başlangıç aralığı), son odak (`tap`: görünüm, panel, bar), zamanı, yönergeler. |
| `hit` | `view?`, `x`, `y`, `width?`, `height?` | (R2) `hit`: görünüm `width`×`height` (varsayılan 1200×600) çizildiğinde (`x`,`y`) pikselinde ne olduğu — panel, bar, zaman, ölçekteki değer ve en üstteki öğe (katman, işaret, veri, bar, zaman, değerler, kökenler); boş alanda `item` yok. Görünen aralık: soranın o görünümdeki bakışı, yoksa kalıcı aralık, yoksa bütün barlar. Sahne tarifinden cevaplanır, çizim yapılmaz (`docs/GRAMMAR.md` §16). |
| `present` | `finding?`, `surface?` | `presentation` (§10). `finding` yoksa en önemli canlı bulgu (en yüksek güven; eşitlikte kimlik sırası). |

**Hedef** (`Target`; dikkat, anlatım adımları ve R2'de görünüm vurgusu): `view`, `panel` (panel id'si), `layer` (R2), `series`, `output`
(serinin çıktısı ya da bar alanı: hacim için `"v"`), `item` (bulgu, olgu…), `from_bar`,
`to_bar`. Hepsi isteğe bağlıdır; olabildiğince belirli yazılır.

**Aralık** (`range`): tam olarak bir biçim — `{"last": n}` ya da
`{"from_bar": a, "to_bar": b}` (kapsayıcı) ya da `{"from_t": …, "to_t": …}`. Barlarla
kırpılır; boş aralık `invalid`.

## 6. Öneri yaşam döngüsü

`pending → accepted | rejected | expired`. Her geçiş bir günlük girdisidir
(`put_proposal`), `proposals` abonelerine `proposal` olayı gider. Öneri dokümanın
içindedir; reddedilen ve süresi dolan öneriler de iz olarak kalır.

## 7. Varlık durumu (presence)

Bağlantı başına: yüzey, profil, görünür aralık (viewport), odak (son `tap`), son girdi
zamanı. Kişi başına: dikkat yönergeleri (en son 16). Varlık durumu günlüğe girmez ve
yeniden oynatılmaz; otorite değişince (devir) sıfırlanır. Ajanlar onu `screen` ve
`attention` sorgularıyla okur; böylece insanın baktığı yer ajanın okuduğu aynı sistemdedir
(AI-native testi 4), ama saniyede onlarca kaydırma kalıcı günlüğü şişirmez.

## 8. Otorite, sıralama, eşzamanlılık, geri alma, devir

### 8.1 Tek otorite ve sıralı günlük

Belge başına tek otorite; komutlar geliş sırasıyla tek tek uygulanır; her biri bir günlük
girdisi olur. Değişimler `seq` sırasıyla yayılır. Çakışan yazılar sunucu sırasıyla, öğe
düzeyinde "son yazan kazanır"; ama `base` ile istemci bunu bilerek reddettirebilir.

Motorun kendisi de yazar olabilir (yazar kimliği `qp`, köken yazarı
`{"kind":"engine","name":"qp","version":…}`). Bir seri verisini, bir hesap serisinin
parametrelerini ya da bir bulgunun geçersizlik koşulunu değiştiren her girdiden sonra
otorite canlı (`live`) kapsüllü bulguların geçersizlik koşullarını değerlendirir; koşul
doğruysa yazarı `qp` olan ayrı bir girdi kapsülü `invalidated` yapar. Etkin niyetlerin
tetikleyicileri ve `condition` abonelikleri de her girdiden sonra değerlendirilir; kenar
tetiklemelidir (önce yanlış, şimdi doğru); bir abonelik ya da niyet oluştuğu anda koşul
zaten doğruysa, o an başlangıç kabul edilir ve olay üretilmez. `condition_met` ve `intent_triggered`
olaylarındaki `seq`, koşulu doğru yapan girdinin konumudur; `bar` son bardır.

### 8.2 İyimser eşzamanlılık (`base`)

Her girdi dokunduğu kimlikleri kaydeder (seri, görünüm, bulgu…; karne için `ledger`).
`base` mevcut günlük konumundan büyükse komut `invalid` döner (`path: /base`); böyle bir
taban denetlenemez. `base` verilen bir komutun dokunacağı kimliklerden biri, `seq > base` olan ve geri
alınmamış bir girdide değiştiyse komut uygulanmaz:

```json
{"type":"error","id":"c9","error":{"code":"conflict",
 "message":"`macd` changed after base 4 (seq 6 by editor)","path":"/ops/0",
 "conflicts":[6],"hint":"Read the item again and resend with base=6, or omit base."}}
```

Çevrimdışı kalan bir istemci döndüğünde komutlarını eski `base` ile gönderir; çakışmayanlar
uygulanır, çakışanlar bu hatayla döner ve istemci okuyup yeniden gönderir (ya da öneriye
çevirir). Yakınsama otoritenin sırasıyla garanti edilir.

Bilinen sınır (T6d): `base` komut başına tektir. Çevrimdışı biriken komutlar aynı öğeye
art arda dokunursa, istemcinin **kendi** önceki yazısı sonrakini `conflict` yapar (20
komutta 1 uygulandı). Öğe başına `base` (ya da "kendi girdilerimi sayma" kuralı) ile 19'u
uygulandı; v1 için önerilir.

### 8.3 Geri alma

`undo` hedef girdinin `inverse` değişimlerini yeni bir girdi olarak yazar (günlük yalnızca
büyür; köken korunur). Yeni girdinin `undoes` alanı hedefi gösterir. Hedef verilmezse:
konuşanın **yazarı olduğu** en son, geri alınmamış ve kendisi geri alma olmayan girdisi.
Geri alma `write` yetkisi ister (öneri olamaz). Başkasının girdisini (onayladığı girdiler
dahil) geri almak `owner` rolü ister. Karar içeren bir girdiyi (kapsül onay/ret, karne
kaydı, öneri durumu) geri almak ayrıca `decide` yetkisi ister. Hedeften sonra
gelen, geri alınmamış bir girdi aynı kimliklerden birine dokunduysa `undo_conflict`
(`conflicts` ile) döner; böylece başka bir yazarın sonraki işi sessizce ezilmez.
Bir geri almayı geri almak yeniden yapmaktır (redo).

### 8.4 Devir (hand-over)

Doküman sunucudan cihaza (ya da geri) devredilebilir: eski otorite `document` anlık
görüntüsünü ve `seq`'i verir, yazmayı durdurur; yeni otorite aynı dokümanla ve
`epoch + 1` ile başlar. `welcome.epoch` istemcilerin hangi otorite soyunda olduğunu
bildirir. Devirde eski otoritenin bütün bağlantıları kapanır; istemciler yeni otoriteye
`hello` ile yeniden bağlanır ve `seq`'i `welcome`'dan alır. Böylece bir `base` değeri her
zaman bağlantının kendi epoch'una aittir; komutta ayrıca epoch taşınmaz.

**v0 durumu ve ölçüm (T6d, `rounds/RD1/experiments/T6d-handover.md`).** Motor iki uçta aynı
baytları ve bitleri üretiyor (yerel = WASM); 10.000 barlı bir dokümanın anlık görüntüsü 750 KB
(gzip 211 KB), cihazda hazır olma 2,6 ms (yerel) / 3,2 ms (WASM). Önerilen varsayılan:
**otorite sunucuda kalır, cihaz hesapları kendisi yapan bir yansıdır** (yalnızca `change`
olayları akar). Tam devir yalnızca başka yazar ve canlı ajan yokken (açık "çevrimdışı al" ya
da yalnız yerel doküman), süreli bir kira (lease) ile. Bu epoch kuralı bölünmeyi **tespit
eder, birleştirmez**; devir sırasında gelen besleme/ajan yazıları, kira ve geri alma
(reclaim), eski otoritenin durduğunun kanıtı (fencing), `(epoch, seq)` süreklilik ve büyük
dokümanlar için parçalı anlık görüntü v0'da tanımlı değildir (açık konu, `REPORT.md`). Ölçüm ve
alternatifler: `rounds/RD1/experiments/` (K21-3).

## 9. Kimlik, yetki, köken ve denetim izi

Kişiler ve rolleri otoritenin yapılandırmasındadır (`Principal {id, author, role}`);
istemci yalnızca kimliğini söyler (kimlik doğrulama taşımanın işidir; v0 kapsamı dışı).

| Rol | read | write | propose | decide | attend | feed |
|-----|:----:|:-----:|:-------:|:------:|:------:|:----:|
| `owner` | ✓ | ✓ | ✓ | ✓ | ✓ | |
| `editor` | ✓ | ✓ | ✓ | | ✓ | |
| `proposer` | ✓ | | ✓ | | ✓ | |
| `viewer` | ✓ | | | | | |
| `feed` | | | | | | ✓ |
| `host` | ✓ | ✓ | | | | ✓ |

- `append_bars`/`update_last_bar`: `feed` ya da `write`. `clock`: yalnızca `host`.
- `approve_finding`/`reject_finding`: `decide` (insan kararıdır; karneye işlenir).
- `accept_proposal`/`reject_proposal`: `decide`.
- Köken: değişimlerdeki her öğenin `provenance.author`'u otoritenin bildiği yazardır;
  `at` son `clock`'tur; kabul edilen önerilerde `approved_by` karar verendir. İstemcinin
  gönderdiği öğelerde köken alanı yoktur (işlemler köken taşımaz).
- Denetim izi: günlüğün kendisi. Her girdide yazar, onaylayan, neden (`reason`), istek,
  değişimler, ters değişimler, zaman; `log` sorgusuyla okunur.

## 10. Yetenek uzlaşması ve sunum derinliği

### 10.1 Profiller

| Yüzey | `depth` | `series_data` | `max_points` | `max_glyphs` | `speech` |
|-------|---------|:-------------:|-------------:|-------------:|:--------:|
| `desktop` | `chart` | ✓ | 500 | 6 | |
| `phone` | `capsule` | ✓ | 60 | 3 | |
| `watch` | `sentence` | | 0 | 1 | |
| `voice` | `sentence` | | 0 | 0 | ✓ |
| `headless` | `chart` | ✓ | 10000 | 6 | |

`surface.max_depth` profili daha sığa çekebilir (derinlik sırası:
`sentence < capsule < chart < workspace`). v0'da `workspace` derinliği sunulmaz.

### 10.2 `present` cevabı

- **`sentence`**: `text` (kapsül başlığı, yoksa bulgunun iddiası) ve `state`. Görsel yok.
- **`capsule`**: + `glyphs` (en çok `max_glyphs`), `confidence`, `spark` (bulgunun ilk
  olgusunun serisinden, son `max_points` bar).
- **`chart`**: + `view` (kapsülün görünümü; yoksa sunucunun kurduğu görünüm — dokümana
  yazılmaz), `facts` (kanıt olgular).

### 10.3 Sunucu (presenter)

`show_finding` ajanın anlam düzeyinde konuşmasıdır: "bu bulguyu göster". Motordaki
deterministik sunucu görünümü kurar: panel 0'da olgunun bar serisinin mumları; olgu bir
hesap serisindeyse panel 1'de o serinin çizgisi ve olgunun `evidence.level` değerinde bir
kural (`encoding.y` = değerin en kısa yazımı, tamsayıysa ondalıksız: `"70"`); yatay aralık
`x.domain = [t(max(0, olgu barı − 30)), t(son bar)]`; `y` her panel için `linear`.
Görünüm kimliği verilmezse `view-<seq>-0`. Aynı doküman her zaman aynı
görünümü üretir. Ajan grameri doğrudan da yazabilir (`put_view`, `add_layer`); ikisi aynı
komut yolundadır (K21-1, ölçüm `rounds/RD1/experiments/`).

## 11. Zaman: saat, canlı güncelleme, anlatım çizelgesi

- **Saat bir girdidir.** Otoritenin saati yoktur; `host` rolündeki bağlantı `clock`
  gönderir. Günlük zamanları, öneri süre aşımları bu saate göre işler. Aynı girdiler aynı
  sonucu verir.
- **Canlı güncelleme:** veri `append_bars`/`update_last_bar` ile gelir; değişim olarak
  yayılır; ardından bulgular, niyetler ve koşul abonelikleri değerlendirilir.
- **Anlatım çizelgesi** (`Narrative`): bir öğe (genellikle bulgu) hakkında sıralı adımlar;
  her adımda `at_ms` (başlangıçtan milisaniye), `target` (görünüm, panel, seri, öğe, bar
  aralığı) ve `say`. Dokümanda yaşar, kökenlidir, kabukta oynatılır; ekransız yüzeyde
  `say` metinleri sırayla okunur.

```json
{"op":"add_narrative","about":"fnd-overbought","steps":[
  {"at_ms":0,"target":{"view":"main","panel":0,"series":"px","from_bar":130,"to_bar":150},"say":"Fiyat yükseliyor."},
  {"at_ms":4000,"target":{"view":"main","panel":1,"series":"rsi14","from_bar":150,"to_bar":150},"say":"RSI 70'i kesti."},
  {"at_ms":8000,"target":{"view":"main","panel":0,"series":"bb20","from_bar":150,"to_bar":150},"say":"Fiyat üst banda yakın."}]}
```

Adımlar `at_ms` artan sırada olmalıdır (değilse `invalid`).

## 12. Sürümleme

- Sürüm dizgesi `qp/<büyük>`; v0 = `qp/0`. `hello.protocol` tam eşleşmeli; değilse
  `handshake` hatası `expected: ["qp/0"]` ile.
- Aynı büyük sürüm içinde değişiklikler yalnızca **ek**tir: yeni isteğe bağlı alan, yeni
  işlem, yeni sorgu, yeni konu, yeni olay. Var olan alanın anlamı değişmez.
- **Otorite katıdır:** tanımadığı alanı reddeder (`schema`, `did_you_mean` ile). Bu, ajan
  yazımında yazım hatalarını sessizce yutmamak içindir. **İstemciler hoşgörülüdür:**
  sunucu mesajlarındaki tanımadıkları alanları ve olay türlerini yok sayar.
- İsteğe bağlı yetenekler `features` ile pazarlık edilir (ör. ileride ikili kodlama).
- Yeni bir işlem eklemek otorite için ek, eski otoriteye gönderen istemci için
  `schema` hatası demektir; istemci `describe` ile önceden bakabilir.

## 13. Hata modeli

Her hata, ajanın yardım almadan düzeltebileceği kadar bilgi taşır:

```json
{"type":"error","id":"c4","error":{"code":"not_found",
 "message":"series `rsi` does not exist","path":"/ops/0/show/with",
 "did_you_mean":["rsi14"],"hint":"Use an existing series id; list them with {\"kind\":\"list\",\"collection\":\"series\"}."}}
```

| Kod | Anlamı | Tipik ek bilgi |
|-----|--------|----------------|
| `parse` | JSON değil | — |
| `schema` | Şemaya uymuyor (bilinmeyen alan, eksik alan, yanlış tip, bilinmeyen `op`/`kind`) | `path`, `expected` (geçerli alan/değer adları), `did_you_mean` |
| `handshake` | `hello` eksik/tekrar, bilinmeyen kişi, sürüm | `expected` |
| `forbidden` | Yetki yok | `hint` (ör. "use mode auto to propose") |
| `not_found` | Kimlik yok | `path`, `did_you_mean` |
| `duplicate` | Oluşturulacak kimlik var | `path` |
| `invalid` | Değer aralık dışı/tutarsız | `path`, `expected` |
| `in_use` | Silinecek öğeye başka öğe dayanıyor | `expected` (dayananlar) |
| `conflict` | `base`'den sonra değişti | `conflicts` |
| `undo_conflict` | Geri alınamaz | `conflicts` |
| `not_pending` | Öneri beklemede değil | — |
| `limit` | Boyut sınırı | `expected` (sınır) |
| `unsupported` | Bu otoritede desteklenmiyor | `hint` |
| `no_origin` | (R2) Katmandaki sayısal sabitin kökeni yok (olgu, adlandırılmış eşik, seri değeri) | `path` (sabitin yeri), `candidates`, `hint` |
| `ambiguous` | (R2) Anlamla başvuru birden çok öğeye uyuyor; otorite rastgele seçmez | `path`, `candidates` (türün sırasıyla, §17) |

`candidates` (R2): `{id, about}` listesi — kullanılacak kimlik (ya da köken için JSON
metni) ve ne olduğu. `ack.resolved` (R2): komuttaki anlamla başvuruların çözüldüğü
kimlikler (`{"/ops/0/layer": "l3"}`); günlükte istek çözülmüş kimliklerle saklanır, böylece
yeniden oynatma değişmez.

`path`, gelen mesajın içine JSON göstericisidir (`/ops/1/params/period`). `did_you_mean`
en çok 5 öneri verir: adaylar, düzenleme uzaklığı ≤ 3 olanlar ve verilen adı içeren ya da
onun içinde geçenler; sıralama düzenleme uzaklığı (alt dizge adayları için de gerçek
uzaklık), sonra alfabe sırası. Adaylar ilgili ad uzayıdır: seri kimlikleri, görünüm
kimlikleri, alan adları… Bir komutta hata varsa **hiçbir** işlem uygulanmaz.

## 14. Güvenlik ve sınırlar

- Dışarıdan gelen her mesaj güvenilmezdir. Sınırlar `welcome.limits`'te bildirilir ve
  ayrıştırmadan **önce** (bayt sayısı) ve sonra (işlem, bar, dizge uzunluğu) denetlenir.
- JSON iç içe derinliği serde_json'ın sınırıyla (128) sınırlıdır.
- (R2) Satır içi **nokta serileri** de kabul edilir (`data.kind = "points"`): boş olmayan,
  `t` kesin artan, `v` sonlu ya da `null`; nokta sayısı `limits.max_bars` ile sınırlı
  (ör. tahmin yelpazesi bantları, `docs/GRAMMAR.md` §3).
- v0 otoritesi seri kaynağı olarak yalnızca `inline` ve `study` kabul eder; `file`,
  `matriks`, `nats` → `unsupported`. Dosya bağları otoritenin yapılandırmasıyla, taban
  klasöre hapsedilerek açılır (R1 bulgusu; `qp_source::open_binding`), protokolden değil.
- Köken istemciden alınmaz; otorite damgalar. Rol istemciden alınmaz; yapılandırmadan gelir.
- Yetki her işlem için ayrı denetlenir; bir komuttaki tek yetkisiz işlem tüm komutu
  `forbidden` (ya da `auto` kipinde öneri) yapar.

## 15. Uygunluk dökümleri

`fixtures/protocol/conformance/*.json`: dilden ve taşımadan bağımsız senaryolar. Biçim:

```json
{
  "name": "add-study-same-panel",
  "covers": ["command", "change"],
  "init": {"document": "fixtures/protocol/docs/thyao.json", "principals": [ … ],
           "clock": 1700000000000, "epoch": 0, "proposal_ttl_ms": null},
  "steps": [
    {"send": {"conn": "a", "msg": { … }}, "expect": [ {"conn": "a", "msg": { … }} ]},
    {"send_raw": {"conn": "a", "line": "{not json"}, "expect": [ … ]}
  ],
  "final": {"document": { … }}
}
```

Kurallar: her adımın çıktıları **tam olarak** `expect` uzunluğunda ve sırasında olmalıdır;
her beklenen mesaj gerçek mesajın **alt kümesi** olarak eşleşir (nesnelerde beklenen her
anahtar eşleşmeli, fazla anahtar serbest; diziler aynı uzunlukta ve öğe öğe; sayılar tam
eşit; `"$any"` her değerle eşleşir; bir nesnede değeri `"$absent"` olan anahtar gerçek mesajda **bulunmamalıdır**). `final.document` da alt küme eşleşmesidir. Her döküm
sonunda koşucu günlüğü baştan oynatır ve dokümanın baytlarının aynı olduğunu denetler.

**Çıktı sırası** (deterministik): önce gönderene cevap; sonra oluşan her girdi için sırayla
`change`, `finding_state`, `proposal` olayları; sonra `condition_met` ve
`intent_triggered`; her olay, abonelerine bağlantı açılış sırası ve abonelik sırasıyla.

## 16. Gömme

- **MCP:** komutlar ve sorgular şemadan üretilen MCP araçlarıdır (her `Op` ve her `Query`
  türü için bir araç, girdi şeması o türün JSON Schema'sı; ayrıca genel `command` ve
  `query` araçları). Araç sonucu `structuredContent` olarak protokolün `ack`/`result`/
  `error` mesajıdır; hata `isError: true`. Olaylar v0 uyarlayıcısında
  `notifications/message` ile gider (MCP 2026-07-28'de kullanımdan kalkmış bir yöntem; doğal
  yol `subscriptions/listen` + `log` sorgusu). Ayrıntı: `rounds/RD1/EMBEDDING.md`.
- **A2UI ve MCP Apps:** analiz `rounds/RD1/EMBEDDING.md`.

## 17. Anlamla başvuru (R2) — yapılandırılmış seçiciler

> Tasarım T8a (`rounds/R2/experiments/T8a.md`). Bu bölüm, tasarım seçilmiş gibi tam
> şartnamedir; seçim R2/T9 ölçümüyle yapılır (`rounds/R2/REPORT.md`).

Bir mesajda **kimlik alan her yere** (seri, görünüm, panel, katman, olgu, bulgu, öneri,
ölçek, herhangi bir öğe) kimlik yerine bir **seçici** (selector) yazılabilir: öğeyi
nitelikleriyle tarif eden küçük bir nesne. Otorite seçiciyi o anki dokümana karşı
deterministik olarak çözer, yerine kimliği yazar, çözümü cevapta geri verir ve günlüğe
yalnızca kimlikli isteği yazar.

```json
{"op":"set_encoding","view":{},"layer":{"shows":{"study":"rsi"}},"encoding":{"width":2}}
```

"Tek görünümdeki, RSI'ı çizen katmanın kalınlığını 2 yap."
Kimlik bir dizge (panelde sayı), seçici bir **nesnedir**; ikisi aynı alanda yazılabilir.

### 17.1 Sözlük

Kapalı ve tiplidir (`qp-schema`: `SeriesSel`, `ViewSel`, `PanelSel`, `LayerSel`,
`FactSel`, `FindingSel`, `ProposalSel`, `ScaleSel`, `ItemSel`); bilinmeyen anahtar
`schema` hatasıdır (`did_you_mean` ile). Her seçicide ayrıca `first` ve `latest`
(§17.3) vardır.

| Tür | Anahtarlar | Örnek ("…") |
|-----|------------|-------------|
| seri | `kind` (`bars`/`study`/`points`), `study`, `params` {ad: sayı}, `on` (seri), `symbol`, `label` | `{"study":"rsi","params":{"period":14}}` "RSI(14)"; `{"kind":"bars"}` "fiyat serisi"; `{"study":"sma","params":{"period":50}}` |
| görünüm | `title`, `shows` (seri) | `{}` "tek görünüm"; `{"shows":{"study":"macd"}}` |
| panel | `name`, `shows` (seri), `mark` | `{"mark":"candle"}` "fiyat paneli"; `{"shows":{"study":"rsi"}}` "RSI'ın paneli"; `{"name":"macd"}` |
| katman | `shows` (seri), `output`, `mark`, `panel` (panel), `value`, `fact` (olgu), `finding` (bulgu), `label` | `{"output":"upper"}` "üst bant" (tek başına çizen katman, §17.1); `{"mark":"rule","value":70}` "70 çizgisi" |
| olgu | `kind`, `on` (seri), `bar` | `{"kind":"breakout","first":true}` "eski kırılımın olgusu" |
| bulgu | `kind` (olgularından birinin türü), `on` (seri), `fact` (olgu), `state` (kapsül durumu) | `{"kind":"breakout","latest":true}` "son kırılım" |
| öneri | `state` | `{"state":"pending","latest":true}` |
| ölçek | `panel` (panel; zorunlu) | `{"panel":{"mark":"candle"}}` → `"y0"` |
| öğe | tek anahtar, türün adı: `series`, `view`, `fact`, `finding`, `proposal` | `{"finding":{"kind":"breakout","latest":true}}` |

Anahtarların anlamı:

- `study`, `kind`, `name`, `title`, `symbol`, `label`, `output`: metin; **büyük/küçük
  harf duyarsız** tam eşitlik (benzerlik yok). `label`, `welcome.summary`'deki açıklama
  metnidir (`RSI14`, `MACD(12,26,9)`; katmanda verilmişse katmanın `label`'ı).
- `params`: verilen her parametre eşit olmalı (`f64` tam eşitlik); verilmeyenler serbest.
  Dokümandaki parametreler varsayılanlarla tamdır: `{"period":14}` varsayılan RSI'a uyar.
- `on` (seri): hesabın girdilerinden biri o seri. `on` (olgu/bulgu): olgu o seri hakkında.
- `shows`: görünümde/panelde bir katman ya da katmanın kendisi o seriyi çizer
  (`layer.series`).
- `mark`: panelde o işaretli bir katman var; katmanda işaret odur.
- `output`: katman o alanı **tek başına** çizer — `y` onu adlandırır; ya da `y` yoktur ve
  katman varsayılan olarak tek bir çıktı çizer, o da budur (tek çıktılı hesap, bar
  serisinde `c`). Başka çıktılarla birlikte çizen katman uymaz: tek Bollinger çizgisi
  (`upper`, `middle`, `lower`'ı birlikte çizer) `{"output":"upper"}`'a uymaz, çünkü onu
  boyamak üç bandı birden boyar. Hiçbir katman uymuyor ama böyle bir katman çıktıyı
  başkalarıyla birlikte çiziyorsa hata `invalid`'dir: aday o katman, `hint` hazır diziyi
  verir — aynı komutta önce `split_layer`, sonra stil:

  ```json
  [{"op":"split_layer","view":{},"layer":{"shows":{"study":"bollinger"}}},
   {"op":"set_encoding","view":{},"layer":{"output":"upper"},"encoding":{"color":"down"}}]
  ```

  Seçiciler işlem işlem çözüldüğü için ikinci işlemin `output`'u, ilkinin oluşturduğu
  `upper` katmanına çözülür (`ack.created`: `["l5","l6","l7"]`, `resolved`
  `{"/ops/1/layer":"l5"}`).
- `value`: katmanın `y` ya da `x` sabiti bu sayıya tam eşit (kuralın seviyesi).
- `bar`: olgunun bar numarası. `state`: kapsül/öneri durumu (şema adları).

### 17.2 Nerede yazılır

| Mesaj | Alanlar |
|-------|---------|
| `add_study` | `inputs[]`, `show.view`, `show.with`, `show.panel` |
| `set_params`, `add_fact`, `append_bars`, `update_last_bar` | `series` |
| `add_layer`, `insert_layer` | `view`, `layer` (aşağıda), `after`, `before` |
| `delete_layer`, `split_layer`, `move_layer`, `set_encoding` | `view`, `layer`, `panel`, `after`, `before`, `encoding` (aşağıda) |
| `insert_panel` | `view`, `layers[]`, `after`, `before` |
| `delete_panel`, `move_panel`, `set_panel` | `view`, `panel`, `after`, `before` |
| `set_scale` | `view`, `scale` |
| `set_emphasis` | `view`, `emphasis[].target` |
| `remove_layer`, `set_view_range` | `view` |
| `add_finding`, `update_finding` | `facts[]`, `finding`, `invalidation` (koşul) |
| `approve_finding`, `reject_finding`, `show_finding` | `finding` |
| `add_intent` | `trigger` (koşul) |
| `add_narrative` | `about` (öğe), `steps[].target` |
| `remove` | `id` (öğe) |
| `accept_proposal`, `reject_proposal` | `proposal` |
| sorgu `get` / `study`, `at` / `hit` / `present` | `id` (öğe) / `series` / `view` / `finding` |
| `subscribe` | `changes.ids[]` (öğe), `finding.finding`, `condition.condition` |
| `attend` | `directive.steps[].target` |

- **Katman nesnesi** (`insert_layer.layer`, `add_layer.layer`, `insert_panel.layers[]`):
  `series`, `fact`, `finding`, `panel` ve kodlamadaki seri başvuruları
  (`encoding.{x,y,y2}.series`, `encoding.color.when.than.series`).
- **Hedef** (`Target`): `view`, `panel`, `layer`, `series`, `item` (öğe seçicisi).
- **Koşul** (`Condition`): `threshold.series`, `compare.left`, `compare.right`.
- `put_view`, `put_series`, `input` hareketleri ve kökenler (`origin`) seçici almaz
  (bütün öğe yazılır ya da arayüz kimlikle konuşur).

**Panel ve katman hangi görünümde aranır?** İşlemin `view` alanındaki görünümde
(`show.panel` için `show.view`; hedefte `target.view`, yoksa `set_emphasis`'in görünümü).
Görünüm adı verilmemişse (dikkat ve anlatım hedefleri) bütün görünümlerde, görünüm sırası
ve görünüm içi sırayla aranır. Görünüm alanı bir seçiciyse önce o çözülür. Var olmayan
bir görünüm `not_found` (görünüm alanının yolunda).

### 17.3 Eşleşme ve sıralama kuralları

1. Verilen **her** anahtar tutmalıdır (VE). `{}` o türün bütün öğelerine uyar.
2. İç içe seçici (`shows`, `on`, `panel`, `fact`, `finding`) uyduğu öğelerin **kümesidir**
   (kendi `first`/`latest`'i uygulandıktan sonra); tek olması gerekmez. Yerine kimlik
   de yazılabilir (`{"shows":"rsi14"}`). İç içe seçici hiçbir öğeye uymuyorsa hata onun
   yolunda `not_found` olur.
3. **Türün sırası** (adaylar hep bu sırada tutulur):
   seri, görünüm, öneri → doküman sırası (eklenme); panel → yığın sırası (üstten alta);
   katman → çizim sırası; olgu → bar, eşitlikte doküman sırası; bulgu → olgularının en
   büyük barı, eşitlikte doküman sırası. Görünüm verilmemişse önce görünüm sırası.
4. **Tek eşleşme** → çözülür. **Birden çok** → `first: true` ilkini, `latest: true`
   sonuncusunu seçer (bulguda "en son", panelde "en alttaki", katmanda "en üstteki");
   ikisi yoksa `ambiguous`. İkisi birden → `invalid`. **Hiç** → `not_found`.
5. Bulanık eşleşme, ağırlık, olasılık yoktur. Aynı doküman + aynı mesaj = aynı çözüm.

### 17.4 Çözüm, cevap ve günlük

- Seçiciler mesaj tipli biçime ayrıştırılmadan **önce** çözülür (`Session::handle_raw`):
  her seçici yerinde kimliğe (panelde sayıya, ölçekte `"y<id>"`'e) çevrilir, sonra mesaj
  her zamanki gibi denetlenir ve uygulanır. Böylece tipli mesaj, günlük (`request`) ve
  öneriler (`proposal.ops`) **yalnız kimlik** taşır; yeniden oynatma ve bayt eşitliği
  değişmez.
- Komutta işlemler **sırayla** çözülür: n'inci işlemin seçicileri, önceki işlemlerin
  uygulanmış olduğu taslak dokümana karşı çözülür (aynı komutta eklenen bir katmana
  seçiciyle başvurulabilir). Önceki işlem hata veriyorsa taslak ilerlemez; hatayı komutun
  kendisi bildirir (ya hep ya hiç değişmez).
- `ack.resolved` ve `result.resolved`: `{mesajdaki JSON göstericisi: kimlik}` — yalnız
  en dıştaki seçiciler (iç içe olanlar değil); panel kimliği ondalık dizge (`"0"`).
  `subscribe` ve `attend` ack'leri de taşır. Sorgu cevabının kendisi de kimliği verir
  (ör. `reduced.series`, `presentation.finding`, `item.id`).
- Öneri: seçiciler öneri anında çözülür; öneri çözülmüş kimlikleri saklar; kabulde
  yeniden çözülmez.
- Seçici kullanmak `read` yetkisi ister (çözüm dokümanı okur); `feed` rolü kimlik
  kullanır (`forbidden`, seçicinin yolunda).
- Hata sırası: önce seçicilerin şeması ve çözümü, sonra mesajın geri kalanının şeması,
  sonra yetki ve uygulama.

### 17.5 Hatalar

| Durum | Kod | Ek bilgi |
|-------|-----|----------|
| Bilinmeyen seçici anahtarı / yanlış tip | `schema` | `path` anahtarın yolu, `expected`, `did_you_mean` |
| Birden çok eşleşme | `ambiguous` | `path`, `candidates` (eşleşenler, türün sırasıyla, en çok 10), `hint` |
| Eşleşme yok | `not_found` | `path`, `candidates` (en yakınlar, en çok 5), `did_you_mean` (metin değerinde yazım hatası), `hint` |
| `first` ve `latest` birlikte | `invalid` | `path` |
| `output` yalnız çok çıktılı bir katmanda (birlikte çiziliyor) | `invalid` | `path`, `candidates` (o katman), `hint` (`split_layer` + stil dizisi) |

`ambiguous` adaylarının sırası **türün sırasıdır**; yani `first: true` adayların ilkini,
`latest: true` sonuncusunu seçer — ajan ya bir anahtar ekler, ya `first`/`latest` ekler,
ya da adayın kimliğini kullanır. `not_found` adayları: en çok anahtarı tutan öğeler önce,
eşitlikte türün sırası; hiçbir anahtar tutmuyorsa türün ilk beş öğesi. Bir metin
anahtarının değeri dokümanda hiç yoksa ileti bunu söyler ve `did_you_mean` dokümandaki
değerlerden önerir (`bolinger` → `bollinger`).

### 17.6 İlk mesajda neye başvurulabilir

- `welcome.summary.select` (`SelectHints`): özetteki her öğe için **şu an ona çözülen**
  bir seçici; anahtar öğe kimliği (panel `"<görünüm>/<panel>"`, katman
  `"<görünüm>/<katman>"`, o görünüm içinde çözülür). Sıra: en kısa anlamlı seçici
  (`{"study":"rsi"}`), gerekiyorsa parametreler, sonra `first`/`latest`, sonra bar.
  Hiçbir seçici tek değilse öğe listede yoktur (kimliği kullanılır).
- `screen` cevabında her panelin `select`'i (o görünüm içinde).

### 17.7 Örnekler (`thyao` örnek dokümanı)

```json
→ {"type":"query","id":"q1","query":{"kind":"study",
   "series":{"study":"rsi","params":{"period":14}},"range":{"last":50},"reduce":"max"}}
← {"type":"result","id":"q1","seq":0,"now":1760000000000,
   "data":{"kind":"reduced","series":"rsi14","reduce":"max","bars":50,"values":{"rsi":{"value":78.115,"bar":154,"t":1754438400000}}},
   "resolved":{"/query/series":"rsi14"}}

→ {"type":"command","id":"c1","ops":[{"op":"set_scale","view":{},
   "scale":{"panel":{"mark":"candle"}},"kind":"log"}]}
← {"type":"ack","id":"c1","status":"applied","seq":1,
   "resolved":{"/ops/0/scale":"y0","/ops/0/view":"main"}}
   (günlükte: {"op":"set_scale","view":"main","scale":"y0","kind":"log"})

→ {"type":"command","id":"c2","ops":[{"op":"set_encoding","view":{},
   "layer":{"mark":"line"},"encoding":{"width":2}}]}
← {"type":"error","id":"c2","error":{"code":"ambiguous",
   "message":"3 layers match {\"mark\":\"line\"}; the authority does not pick one",
   "path":"/ops/0/layer",
   "hint":"Add a key that tells them apart, add \"first\":true or \"latest\":true (candidates are in paint order), or use one of the candidate ids.",
   "candidates":[{"id":"l1","about":"bb20.upper/middle/lower line (panel 0 of view main)"},
                 {"id":"l2","about":"rsi14.rsi line (panel 1 of view main)"},
                 {"id":"l4","about":"macd.macd/signal line (panel 2 of view main)"}]}}

→ {"type":"command","id":"c3","ops":[
   {"op":"add_study","id":"rsi7","study":"rsi","inputs":[{"kind":"bars"}],"params":{"period":7}},
   {"op":"insert_panel","view":{},"panel":{"name":"rsi7","y":{"kind":"linear"}},
    "after":{"mark":"candle"},"layers":[{"mark":"line","series":"rsi7"}]}]}
← {"type":"ack","id":"c3","status":"applied","seq":2,"created":["rsi7","3","l5"],
   "resolved":{"/ops/0/inputs/0":"px","/ops/1/after":"0","/ops/1/view":"main"}}

→ {"type":"command","id":"c4","ops":[{"op":"set_emphasis","view":{},"emphasis":[
   {"target":{"item":{"finding":{"kind":"cross_above","latest":true}}},"kind":"highlight"}]}]}
← {"type":"ack","id":"c4","status":"applied","seq":3,
   "resolved":{"/ops/0/emphasis/0/target/item":"fnd-overbought","/ops/0/view":"main"}}

→ {"type":"command","id":"c5","ops":[{"op":"delete_panel","view":{},
   "panel":{"shows":{"study":"bolinger"}}}]}
← {"type":"error","id":"c5","error":{"code":"not_found",
   "message":"no series matches {\"study\":\"bolinger\"}; no series has study `bolinger`",
   "path":"/ops/0/panel/shows","did_you_mean":["bollinger"],
   "candidates":[{"id":"px","about":"THYAO 1d: bars"}, "…"]}}
```

Uygunluk dökümü: `crates/qp-session/tests/transcripts/t-select.json` (başarı, belirsizlik,
`first`/`latest`, bulunamama, şema, öneri). Yirmi R2 görevinin kimliksiz yazımı:
`crates/qp-session/tests/selectors.rs` (`r_tasks`).

### 17.8 Şema

Seçici türleri ve kimlik-ya-da-seçici türleri (`SeriesRef`, `ViewRef`, `PanelRef`,
`LayerRef`, `FactRef`, `FindingRef`, `ProposalRef`, `ScaleRef`, `ItemRef`) `qp-schema`'dadır.
İşlem/sorgu alanları Rust'ta kimlik tipinde (`String`, `u32`) kalır; JSON Schema'ları
`#[schemars(with = …)]` ile "kimlik ya da seçici"dir. İç içe yapılar için tel biçimleri
(`LayerIn`, `EncodingIn`, `TargetIn`, `ConditionIn`, `EmphasisIn`, `NarrativeStepIn`,
`AttentionDirectiveIn` …) yalnız şemayı tarif eder; alan adlarının saklanan tiplerle aynı
kaldığını bir test denetler. Saklanan görünüm (`View`) şeması seçici içermez. Bilinen
uzlaşma: `Op` tipi hem istek hem saklanan istek (`LogEntry.request`, `Proposal.ops`)
olduğundan saklanan isteğin şeması da seçiciye izin verir görünür; otorite oraya yalnız
kimlik yazar. MCP araçları bu şemadan üretildiği için her `op_*`/`query_*` aracı seçici
kabul eder.

## Ek A — Eski `Command` tipi

`qp-schema` v0'daki `Command {provenance, op}` ve `Op` (doküman şeması) bu protokolden
önceki taslaktır; köken istemciden geldiği için protokolde kullanılmaz. Şema dışa
aktarımında `Op` adı artık protokol işlemini gösterir.
