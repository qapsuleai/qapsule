# Qapsule Alan Protokolü — `qp/0`

Durum: **v0** (RD1; R2'de gramer eki — §5.1 gramer işlemleri, §5.4 `hit`, §4.1 `summary`, §13 `no_origin`/`ambiguous`, §17 anlamla başvuru; RP2'de veri düzlemi ve tutarlılık eki — §18 veri bağı, veri olayları, veri yetkisi, dondurulmuş kanıt, artımlı canlı yol; §19 iddia kanıtı, `claims` ve `view` konuları, yansının görüntü kuralı; K25 gereği `present` belirsizliği, gramer katman adları, geleceğe uzanan bakış; hepsi ek, kırıcı değil). Mesaj tiplerinin tek kaynağı `crates/qp-schema/src/protocol.rs`;
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
input      (kaydır, yakınlaştır)  →    result (viewport | at | interaction)
attend     (dikkat yönlendir)     →    ack (delivered); hedefe notify
clock      (yalnızca host)        →    (cevap yok; süre aşımları işlenir)
replay     (R4: oynatma denetimi) →    result (replay)            ya da error   (§21)
visible    (RP3: ekrandaki yüzler) →    result (viewer); viewer konusu notify  (§22)
(barındırıcı girdisi: piyasa durumu, R6)  →  viewer olayında market      (§24)
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
otoritede deterministik olarak hesaplanır (`qp-compute`). **Bağlı seriler**
(`source.kind = "bind"`, RP2) de veri taşımaz: barları veri düzleminden gelir, her kişinin
veri yetkisiyle çözülür (§18; K23). Belge içi bar serileri (`inline`) verilerini
`data.bars` içinde taşır; test verisi ve küçük örnekler için kalır (§14).

**Veri düzlemi ve anlam düzlemi (K23).** Protokol yalnızca anlamı taşır. Bağlı serilerin
barları ve canlı fiyatları protokol mesajı değildir, günlüğe girmez; barındırıcı onları
veri olayları olarak motora doğrudan verir (§18.4). Bu yüzden günlük ve yeniden oynatma
değişmezi bağlı serilerde de aynıdır: doküman veri olaylarıyla değişmez; veri olayının
yol açtığı motor girdileri (kapsül geçersizleşmesi) tam değişimleriyle günlüktedir.

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
  (RP2) **Barındırıcı çerçevesi:** `qp serve --host-frames` ile `{"host":{"data":<DataEvent>}}`
  satırı bir veri olayını barındırıcı yolundan (`Session::handle_host_frame` →
  `Session::host_data_event`, §18.4) motora verir; çıktısı olayın yol açtığı bildirimlerdir,
  reddedilen olay `{"host":{"error":"…"}}` döner. Barındırıcıya ayrıca **`host` satırları**
  yazılır (bağlantı bildirimlerinden önce, her biri bir satır): `seq` boşluğu
  `{"host":{"gap":{"series":"px","feed":{…},"expected":1,"got":7}}}`, bağın başka nedenle
  `stale` olması (büyüme sınırı) `{"host":{"stale":{"series","feed","reason"}}}`. Bu satır
  hata değildir (olay alındı, uygulanacak bir şey yoktu); barındırıcı bağı yeniden çözer
  ya da boşluğu doldurur. Taşıma düzeyidir ve **yalnız
  barındırıcınındır**: protokol mesajı değildir, bağlantı (`conn`) taşımaz, günlüğe girmez.
  Bayrak yalnız akışın sahibi barındırıcıysa verilir; bayraksız böyle bir satır çerçeve
  hatasıdır (`conn` yok). Akışa güvenilmeyen istemcileri çoğullayan bir aracı `host`
  satırlarını geçirmemelidir.

- **JSON-RPC eşlemesi** (MCP, LSP benzeri ortamlar için): istemci mesajı
  `{"jsonrpc":"2.0","id":<id>,"method":"qp/<type>","params":<mesajın geri kalanı>}`;
  `ack`/`result` → `result`, `error` → `error.data`, `notify` → bildirim `qp/notify`.
  MCP üzerinde taşıma için bkz. §16 ve `rounds/RD1/EMBEDDING.md`.

## 4. Mesaj türleri

Sekiz tür: **Komut, Değişim, Sorgu, Abonelik, Girdi, Öneri, Dikkat, Yetenek.**
Aşağıda her biri, alanları ve örneğiyle. Alanların tam listesi şemadadır.
(R4) Barındırıcının yeniden oynatma kaynağına giden **`replay`** denetim mesajı ayrıca
§21'de; dokümana yazmaz, günlüğe girmez.

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
| `intents` | — | `intent_triggered`; (R5B) `intent_matched` (izlemenin kişisine; §25.5) |
| `view` (RP2) | `view` (seçici de olur) | `view_changed` (görünümün okuduğu bir serinin barları değişti; §18.8) |
| `claims` (RP2/T8) | — | `claim` (otoritenin veri kaynaklı iddiası, kanıtıyla: geçici, kesin, geri çekildi, temizlik değişti; §19.3). Veri kaynaklı `condition_met`, `intent_triggered`, `finding_state` ve motor girdisinin `change`'i de isteğe bağlı `evidence` (`ClaimEvidence`) taşır |
| `comments` (R5B) | `capsule?`, `instrument?` | `comment` (okuyucunun alabileceği yeni doğrulanmış yorum; §25.4) |
| `stories` (R5/M18) | `instrument`, `series?` | `story` (hikâye listesinin değişen satırı: doğum, geçiş, kapanış, geçerlilik, akıştaki yer; §23.4); açık görünüm okuyucusuna o enstrümanın `server_comment`'i |

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
| `look` | `view`, `from_bar`, `to_bar` | Yüzey görünen aralığı bildirir. `to_bar` son barın ötesine, görünümün **son gelecek yuvasına** kadar gidebilir (K25; aşağıda). |
| `pan` | `view`, `bars` (+ yeniye) | Aralığı kaydırır; sınırlara (ilk bar, son yuva) kırpılır. |
| `zoom` | `view`, `factor` (>1 yakınlaştırır), `anchor_bar?` | Genişlik = round(genişlik / factor), en az 2 bar; `anchor_bar` (varsayılan: son görünen bar) ekranda aynı göreli konumda kalır. |
| `tap` | `view`, `panel`, `bar` | Odak o bar olur; cevap `at` sonucudur. |

**Geçici mi kalıcı mı?** Kullanıcının kendi ekranındaki hareketi (kaydırma, yakınlaştırma,
"grafiği yakınlaştır") `input`tur ve yalnız o bağlantının bakışını değiştirir. Herkesin
gördüğü kalıcı aralık ("görünümü son 60 bara ayarla, böyle kalsın") `set_view_range`
komutudur ve günlüğe girer.

Her `look`, `pan` ve `zoom`, aralık değişmese bile kişinin `attention` abonelerine
`viewport` olayı üretir. `tap` olay üretmez (cevabı `at`'tir; odağı günceller). `tap` ve
görünümle ilgili bar numaraları görünümün **ilk bar serisine** göredir.

Bağlantının başlangıç aralığı: görünümün `x.domain`'i varsa ona düşen yuvalar; yoksa son
N bar (masaüstü 120, telefon 60, saat 20; ekransız yüzeylerde de aynı kural, N = 120)
**ve ardından görünümün bütün gelecek yuvaları**.

**Gelecek yuvaları (K25).** Görünümün çizdiği nokta serilerinin son bardan sonraki
noktaları (ör. tahmin yelpazesi) son barın ardından yuvalar kaplar (`docs/GRAMMAR.md` §3):
son bardan sonraki k'inci nokta `son bar + k` yuvasıdır. Bakış bu yuvalara uzanabilir:
`look`/`zoom` (`anchor_bar`) son yuvaya kadar kabul edilir, sonrası `invalid`
(`expected: ["0..=<son yuva>"]`); `pan` ve `zoom` son yuvaya kırpılır; `tap` yalnız barlar
içindir (`bar` ≤ son bar). `screen` ve `hit` görünen gelecek yuvalardaki öğeleri gösterir;
`screen.last_close` görünen son **barın** kapanışıdır. Kalıcı aralık (`set_view_range`) yine
barlarla sınırlıdır. Döküm: `29-future-look`.

#### 4.6.1 Piksel hareketleri ve açılma (R4)

Kabuk px'i bara çevirmez, isabet testi yapmaz, açılmaya karar vermez (`docs/ARCHITECTURE.md`
"Kabuk sınırı"). Ham hareketi **piksel** olarak gönderir; çekirdek onu bağlantının **bakış
durumu** (§7) ve hedefin sahne tarifi (`docs/GRAMMAR.md` §14) üzerinden çözer. Bar uzayındaki
dört hareket (`look`, `pan`, `zoom`, `tap`) aynen kalır.

| Hareket | Alanlar (`?` isteğe bağlı) | Etki |
|---------|---------|------|
| `tap_at` | `target?`, `x`, `y`, `width?`, `height?` | Hedefin sahnesinde nokta sorgusu (`hit` ile **aynı kod**: `qp_session::hit_scene`). Sonra açılma kuralı (`docs/GRAMMAR.md` §18.6): kapsül bir derinlik açılır; grafikte kanıt öğesine dokunmak kanıtı açar; düz bir mum odaklanır (`at`, `tap` gibi). Belge görünümünde bir kapsülün işaretine (`glyph`) dokunmak o kapsülü açar. |
| `drag` | `target?`, `dx`, `width?`, `height?` | Yatay sürükleme: `bars = round(−dx / slot)` (parmak sağa → eski barlar görünür). Belge görünümünde `pan` ile aynı yoldan (bağlantı bakışı, `viewport` olayı); açık kapsülün grafiğinde bakış durumunun aralığı aynı kurallarla (`qp_grammar::gesture::pan_range`) kayar. |
| `pinch` | `target?`, `factor` (>1 yakınlaştırır), `x?`, `width?`, `height?` | `x` sütunundaki bar yerinde kalır (yoksa son görünen bar); `zoom` ile aynı kural (`zoom_range`). |
| `open` | `capsule` (kapsül ya da bulgu kimliği; R4/S1: **kapsül seçicisi** de, §17.1 — cevabın `resolved`'ında `/gesture/capsule`), `to?`, `width?`, `height?` | Kapsülü bir derinlik (ya da `to`'ya kadar) açar: `sentence → capsule → chart → evidence`. |
| `close` | `to?`, `width?`, `height?` | Açık kapsülü bir derinlik geri (ya da `to`'ya; `sentence` = akışa dönüş) kapatır. |

- **Hedef** (`target`, `SurfaceTarget`): `{"view": …}` (belgenin bir görünümü) ya da
  `{"capsule": …}` (akıştaki yüzü ya da açık kapsül, o bağlantıdaki derinliğiyle); ikisi
  birden `invalid`. Verilmezse **o anki sahne**: açık kapsül varsa o, yoksa bağlantının o
  anki görünümü.
- **Yüzey boyutu.** Piksel, hedefin **çerçevesine** göredir. Kabuk çerçeveyi harekete
  `width`/`height` ile ekler; çekirdek onu bakış durumunda **çerçeve anahtarı** başına saklar
  (`view:<id>`, `face` — akıştaki kapsül yüzü, `capsule`, `chart`) ve sonraki hareketler
  boyutsuz gelebilir. Hiç bildirilmemişse `screen`'in yüzey çerçevesi (telefon 390×300;
  yüz için aynı genişlik × 140). Sınırlar `200..=16384` × `120..=16384`, dışı `invalid`
  (`/gesture/width`). Gerekçe: `hello.surface.screen` cihazın tamamıdır, oysa telefonda akış
  kartı ile grafik farklı boyuttadır; hedef başına ayrı bir `look` mesajı da her düzen
  değişiminde sıra bağımlılığı doğururdu. Boyut hareketle gelince her mesaj kendi başına
  çözülebilir (ajan durumsuz sürebilir), kabuk düzen değişince bir kez gönderir.
- **Cevap:** `result` + `data.kind = "interaction"` (`Interaction`): `hit` (dokunulan yer),
  `at` (odaklanan bar), `viewport` (yeni aralık), `opening` (derinlik geçişi) ve `state`
  (hareketten sonraki bakış durumu). Bar uzayındaki hareketler eski cevaplarını verir.
- **Açılma cevabı** (`Opening`): `capsule`, `finding`, `from`, `to`; `presentation` —
  sunucunun `to` derinliğindeki çıktısı (§10.2'nin kuralları, bağlantının profili o
  derinlikte: `capsule` = kart `capsule_view`, `chart`/`evidence` = görünüm ve olgular;
  yetkisiz okuyucuya kıvılcım ve görünüm yok, §18.3); `frame` — yeni sahnenin çerçevesi;
  `shared` — **geçişte ortak kalan öğeler**: kimlik + iki sahnedeki kutular
  (`docs/GRAMMAR.md` §18.6; kabuk yalnız iki kutu arasını boyar). Listeyi bağlantının
  geçiş tasarımı (`style.transition`, §4.6.2) kurar. **Tek yanlı kayıtlar** (R4/S1): iki
  kutusu da olan kayıt ortak öğedir; yalnız `before` ya da yalnız `after` kutusu olan kayıt
  ortak öğe **değildir**, tasarımın ara kare kuralına verdiği **kare verisidir** (o sahnede
  bir bölgenin kutusu). Bugün yalnız `zumlak` yazar: `scene:before` (eski sahnenin tamamı),
  `pane:before` (eski sahnenin fiyat paneli), `scene:after` (yeni sahnenin tamamı); ayrıca
  iki kutulu `span` (iki fiyat panelinin ortak bar × fiyat penceresi). Ortak öğe sayan bir
  istemci tek yanlı kayıtları saymaz; anlamını bilmediği kaydı yok sayar. (Bunları
  `Opening.shared` dışına taşımak geçiş arayüzünün kare işlevine sahneleri vermeyi
  gerektirir; `rounds/R4/experiments/S1.md`.) `evidence` — `evidence`
  derinliğinde `capsule` sorgusunun olgu başına kanıt erişimi (yetkiliye değerler ve
  dondurulmuş kanıt, yetkisize `requires` ve `rests_on`; §20.2, K34). Kanıt yeniden
  hesaplanmaz, aynı kod döner.
- **Derinlik sınırı:** `surface.max_depth` (`sentence`/`capsule`) açılmayı sınırlar; ötesi
  `invalid` (`/gesture/to`). Telefonun profil derinliği (`capsule`) yalnız `present`'in
  varsayılanıdır; kullanıcı açtığında grafiğe ve kanıta inilir.
- **Hatalar:** açık kapsül yokken `close`, `open` ile `sentence`, açık olmayan bir kapsülde
  (yüz) `drag`/`pinch`, pozitif olmayan `factor`, sonlu olmayan `x`/`y` → `invalid`;
  bilinmeyen ya da okuyucunun göremediği kapsül → `not_found` (aynı cevap).
- Her piksel hareketi ve açılma, girdi sayacını ve son girdi zamanını günceller; dokümana
  ve günlüğe **hiçbir şey yazmaz** (§7). `drag`/`pinch` `viewport` olayı üretir (açık
  kapsülün grafiğinde görünüm kimliği `present-<bulgu>` ya da kapsülün saklı görünümü).
- Dökümler: `51-input-tap-hit`, `52-input-pan-pinch`, `53-input-open-close`,
  `54-input-evidence`, `55-input-presence`, (R4/S1) `56-input-look` (bakış ayarının M9–M11
  alanları, `zumlak`'ın `Opening.shared`'ı), `58-input-open-selector` (`open` + seçici) (üretici:
  `cargo run -p qp-session --example make_input_transcripts -- --write`).

#### 4.6.2 Bakış ayarı — `style` (R4/T5)

Bağlantının sahnelerinin **nasıl** kurulacağını seçer: kapsül yüzü tasarımı, işaret alfabesi,
tema. Bakış durumunun (§7) parçasıdır: günlüğe girmez, dokümanı değiştirmez; aynı bağlantının
sonraki piksel hareketleri ve açılmaları bu ayarla derlenir (isabet testi dahil), kabuğun yansısı
aynı adlarla aynı sahneyi derler.

```json
{"type":"input","id":"i9","gesture":{"kind":"style","face":"card","alphabet":"rozet","theme":"light"}}
```

| Alan | Değerler | Varsayılan |
|------|----------|------------|
| `face` | `qp_grammar::faces::FACES` (`r3`: §18.4 kapsül kartı, 240 px; `card` eski adı) | `r3` |
| `alphabet` | `qp_grammar::glyph::ALPHABETS` (`siluet`, `rozet`) | `siluet` (ürünün) |
| `theme` | `dark`, `light`; `view`: görünümün kendisi | `view` |
| `feed` (R4/H-M8910) | `qp_grammar::feeds::LAYOUTS` (akış düzeni; `r3`: `capsules` sırası) | `r3` |
| `transition` (R4/H-M8910) | `qp_grammar::transitions::TRANSITIONS` (açılma geçişi; `r3`: T4 ortak öğeleri, doğrusal ara kare) | `r3` |
| `outcome` (R4/H-M8910) | `qp_grammar::glyph::outcome::STYLES` (sonuç işareti, M11) | `r3` |
| `seen` (R4/H-M8910) | son bakış zamanı (ms); `0` siler | yok |
| `voice` (R5/m17-int) | `qp_capsule::voice::VOICES` (kapsül dili: `r4` bugünkü metin, `marla`, `selvi`; `docs/LANGUAGE.md`) | `r4` |
| `open` (R5/M20) | `qp_grammar::open::OPENS` (açılmış kapsül tasarımı: `r4` bugünkü kart, `tesir` tek sütun, `nobra` soru kartları) | `r4` |
| `story` (R5/M18) | `qp_grammar::stories::STORIES` (hikâye çizelgesi, grafik derinliğinde: `r4` yok, `pavka` hikâye omurgası, `korda` ömür şeritleri) | `r4` |
| `hide_closed` (R5/M18, K54) | `true` / `false`: hikâye çizelgesinde kapanmış hikâyeleri gizle (seçili hikâye kalır; R5/S tek ad, `ViewState.hide_closed`) | `false` |
| `clock` (RP4/T7) | `true` / `false`: satırlarda olayın zamanı (`ViewState.clock`) | `false` |

- Verilmeyen alan değişmez; bilinmeyen ad `invalid` (`/gesture/face`, `/gesture/alphabet`,
  `/gesture/theme`, `expected` bilinen adlar). Doğrulama her şeyden önce: hatalı istek hiçbir
  şeyi değiştirmez.
- Cevap `interaction`; `state` (`ViewState`) yeni `face`, `alphabet`, `theme` alanlarını taşır
  (varsayılandaysa alan yoktur: eski cevaplar bayt eşit).
- (R5/M20) `open`: `ViewState.open` (varsayılanda yok). Kapsül derinliğinin görünümünü
  seçilen tasarım kurar (`docs/GRAMMAR.md` §18.4). Kendi yüksekliğini söyleyen bir tasarımda
  otorite kapsül derinliğini o yükseklikte açar (`frames.capsule`'ün genişliği, tasarımın
  yüksekliği; yüze dokunarak açmada da); tasarım değişince hatırlanan `capsule` çerçevesi
  unutulur. Sahnesi olmayan okur için (veri yetkisi yok) metin sütunu yazan bir tasarımda
  kapsül derinliğindeki `presentation.facts` de dolar (grafik derinliğindeki gibi, okura göre
  ayıklanmış).
- (R5/m17-int) `voice`: `ViewState.voice` (varsayılanda yok). Bir dil seçiliyken kapsül
  öğeleri `voice` taşır (§20.2), kapsül sahneleri (yüz, açılmış kart) metnini o dilden alır ve
  sözlük terimlerini işaretler: `SceneDesc.terms` (isteğe bağlı; `TermMark`: `term`, `text`,
  `layer`, `hit` kutusu), nokta sorgusu `HitInfo.term` (isteğe bağlı). `tap_at` bir terimin
  üstündeyse kapsülü açmaz, cevabın `hit.term`'ünü döner. `r4`'te bu alanların hiçbiri yoktur
  (dökümler bayt eşit).
- (RP4/T7) `clock`: `ViewState.clock` (varsayılanda yok). Açıkken satırlar olayın zamanını
  taşır (`qp_capsule::realize::stamp`): bir dil seçiliyse kapsül öğesinin `voice.headline`'ı
  "`<zaman> · `" ile başlar (yüz ve açılmış kart aynı başlığı çizer; `voice.face` yeniden
  kurulur; ses zamanla birlikte başlığı kendi 80 karakter sınırında tutar, gerekirse daha kısa
  başlık biçimini seçer), hikâye çizelgesinin (`korda`) şerit etiketleri de aynı zamanı taşır (olaydan sonra; etiket
  sığmazsa kesilen uç zamandır, şeridin yeri zamanı zaten gösterir; `Session::row_stamp`). Zaman, satırın olayıdır: açık hikâyede doğuş, kapanmışta kapanış
  barı (barın açılış zamanı). Gün içi periyotta ve "şimdi"yle aynı gündeyse `SS:DD`, değilse
  tarih (`12 Kas` / `Nov 12`; başka yılsa yılıyla); günlük, haftalık, aylık periyotta tarih.
  "Şimdi" serinin son barıdır (verinin saati; sistem saati değil). Gösterim saat dilimi
  dokümanın `display_tz`'si, yoksa borsanınki (K14). Yetkisiz okur (K33) zaman almaz. `r4`
  sesinde (ses yok) kapsül satırı değişmez. Kapalıyken her döküm bayt eşittir; sunum kabuğu
  açar (`ShellCore::presented_look`, `?clock=0` kapatır).
- (R5/M20) `open`: `ViewState.open` (varsayılanda yok). Kapsül derinliğini o tasarım kurar
  (otorite `tap_at`/`hit` için, cihaz boyamak için aynı kuralla) ve kendi yüksekliğini
  bildirebilir (`capsule` çerçevesi). Soru kartlı tasarımda (`nobra`) açık soru
  `ViewState.ask`'tadır: kart paneline `tap_at` onu açar/kapatır (açılma yok), kanıt grafiğine
  dokunuş grafiği açar; kapsül kapanınca ya da başkası açılınca `ask` silinir. Sahnesiz okuyucuya
  (A32) kabuk aynı soruları çekirdeğin metniyle listeler. `r4`'te alanların hiçbiri yoktur.
- M8 yüz alternatifleri `qp_grammar::faces` kaydına bir satırdır (ad, **yükseklik**, görünüm
  kuralı); yüz kendi yüksekliğini bildirir, kabuk onu çekirdekten alır. M9 akış düzenleri
  `qp_grammar::feeds`, M10 geçişleri `qp_grammar::transitions` kaydındadır: düzen kabuğun
  dizeceği satırları (kapsül, grup başlığı metni, kısa işaret) verir; geçiş `Opening.shared`'ı
  ve ara kareleri (öğe başına kaynak sahne, kutu, opaklık; yeni sahnenin opaklığı) verir.
  Varsayılanlarda (`r3`) cevaplar ve dökümler bayt eşittir. Kullanım ve ölçüm:
  `rounds/R4/experiments/harness/m8910/README.md`. Alfabe değişimi derleyici seçeneğidir
  (`CompileOptions::alphabet`); ürünün alfabesi (`glyph::alphabet()`) değişmez.
- **Protokol varsayılanı ile gösterimin varsayılanı** (R4/S1). Yukarıdaki tablonun
  varsayılanları protokolündür: bakış ayarı göndermeyen bağlantının (ajan, dökümler, çıplak
  `present`/`open`) cevapları değişmez. Gösterim kabuğunun çekirdeği (`ShellCore`) ise
  `hello`'dan hemen sonra M9–M11 seçimlerini ister:
  `{"kind":"style","feed":"elikov","transition":"zumlak","outcome":"tg5"}`
  (`qp_grammar::feeds::PRESENTED_FEED`, `transitions::PRESENTED_TRANSITION`,
  `glyph::outcome::PRESENTED_OUTCOME`); sayfanın kendi seçimi (URL, Ayarlar) ondan sonra
  gider ve geçerlidir. Yüz varsayılanı (M8) değişmedi. Akış satırları (grup başlıkları,
  kısa işaretler) protokol cevabı değildir: çekirdek `capsules` cevabından ve bu bakış
  durumundan dizer (`qp_grammar::feeds`; yerel gün bilgisini de çekirdek verir:
  `FeedCtx.local`, gösterim saat dilimi `Europe/Istanbul`).
- `rozet` R3/M1 deneyinden aynen taşındı; `validity` kanalı yoktur (K42: "artık geçerli
  değil" yalnız `siluet`'te işarette görünür; insan testi `rozet`'i seçerse eklenmeli).

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
| `set_audience` | `principal?`, `audience` | (R3, §20.3) Kişinin kapsül tercihleri (grup, hassasiyet, izleme listesi, pozisyonlar). Kendi: `read`; başkası: `owner`. Öneri olamaz. |
| `set_sensitivity` | `principal?`, `sensitivity` | (R3, §20.3) Yalnız hassasiyet seviyesi (`null`: grubun varsayılanı). |
| `add_comment` | `id?`, `about {capsule \| instrument}`, `lang`, `text`, `cites`, `external?` | (R5B, §25.4) Doğrulanmış yorum; yetki `propose` ya da `write`, doğrudan (öneri olmaz). Ret: `comment_rejected` + `reasons`. Değişim: `put_comment`. |
| `add_watch` | `id?`, `for?`, `text`, `match`, `expires_t?`, `max_matches?` | (R5B, §25.5) İzleme niyeti; yetki `read`, öneri olmaz; kişi başına 32. Değişim: `put_intent` (`watch`). |
| `remove_watch` | `intent` | (R5B, §25.5) İzlemeyi kaldırır (`state: removed`). |

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
`set_last_bar`, (R3) `put_audience` (`principal`, `audience` ya da `null`), (R5B) `put_comment`. Her biri tam öğe taşır; `put_*` değişimlerinin isteğe bağlı `index` alanı
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
| `screen` | `principal?` | `screen`: kişinin ekranlı (`desktop`, `phone`, `watch`) her bağlantısı için, bağlantı açılış sırasıyla: görünüm, görünen aralık, sembol ve aralık (görünümün ilk bar serisinden), paneller (yığın sırasıyla; `panel` = panel id'si; R2'de ayrıca `name`, `scale` {id, tür, görünen alan} ve `layers` — kimlik, işaret, veri, alanlar, açıklama, sabitler ve kökenleri, görünen öğe sayısı, son değerler, sahne tarifinden; `series` = o paneldeki katmanların serileri, katman sırasıyla, tekrarsız; `marks` = bütün katmanların işaretleri, katman sırasıyla), olgu barı görünen aralıkta olan bulguların kapsülleri, görünen son barın kapanışı; (R4) bağlantının bakış durumu `view_state` (§7). |
| `attention` | `principal` | `attention`: kişinin en son etkin ekranlı bağlantısının bakış aralığı (girdi yoksa başlangıç aralığı), son odak (`tap`: görünüm, panel, bar), zamanı, yönergeler. |
| `hit` | `view?`, `x`, `y`, `width?`, `height?` | (R2) `hit`: görünüm `width`×`height` (varsayılan 1200×600) çizildiğinde (`x`,`y`) pikselinde ne olduğu — panel, bar, zaman, ölçekteki değer ve en üstteki öğe (katman, işaret, veri, bar, zaman, değerler, kökenler); boş alanda `item` yok. Görünen aralık: soranın o görünümdeki bakışı (ekranlı bağlantıda varsayılan bakış; gelecek yuvalar dahil, §4.6), yoksa kalıcı aralık, yoksa bütün yuvalar. Sahne tarifinden cevaplanır, çizim yapılmaz (`docs/GRAMMAR.md` §16). |
| `present` | `finding?`, `surface?` | `presentation` (§10). `finding` yoksa (K25): tek canlı bulgu varsa o; **birden çok canlı bulgu varsa otorite seçmez**, `ambiguous` döner — `candidates` bütün canlı bulgular, seçicinin bulgu sırasıyla (§17.3: olgularının en büyük barı, eşitlikte doküman sırası) — **artan** sıra: son aday en yeni olandır, `{"latest":true}` onu seçer, `hint` `finding` ya da bir seçici (`{"latest":true}`) yazmayı söyler; hiç canlı bulgu yoksa `not_found`. |
| `binding` | `series?` | (RP2) `bindings`: bağlı serilerin soran için durumu (`ready`, `pending`, `no_entitlement`, `no_data`, `stale`), nedeni, çapa (`anchor_t`, bar 0'ın zamanı) ve otoritenin veri konumu (§18.3). |
| `verify_evidence` | `item` (olgu ya da bulgu) | (RP2) `evidence_check`: olgunun (bulguda her olgunun) dondurulmuş kanıtının bugünkü veriye karşı sınaması — veri özeti ve kanıt değerleri; fark varsa bildirir (§18.5). |
| `capsules` | `instrument?`, `stages?`, `open?`, `meaning?`, `since_bar?`, `since_t?`, `audience?`, `limit?` | (R3, §20) `capsules`: soranın görebildiği kapsüller, önem sırasıyla; yetkisi olmayan lisanslı akış için `comments` (sunucu yorumları). |
| `capsule` | `capsule` (kapsül ya da bulgu kimliği) | (R3, §20) `capsule_detail`: hikâye (geçişler, nedenler), olgular, cümle, karne, işaret tarifi, olgu başına kanıt erişimi (K34). |
| `ledger` | `key?`, `meaning?`, `detector?`, `by_kind?` | (R3, §20) `ledger`: anahtar (ya da tür) başına kapanan ve gerçekleşen sayısı; lisanstan bağımsız. |
| `stories` | `instrument`, `series?`, `range?`, `limit?`, `after?` | (R5/M18, §23) `stories`: enstrümanın aralıktaki bütün hikâyeleri (kapsülleri), okuyucunun gördüğü biçimde, doğum sırasıyla, sayfalı; bar yok. |
| `preview` | `proposal`, `width?`, `height?` | (R5B, §25.6) `preview`: bekleyen önerinin değişimleri ve eklediği/değiştirdiği görünümlerin sahne özeti; doküman değişmez. |

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
zamanı ve (R4) **bakış durumu** (`ViewState`): açık kapsül ve bulgusu, derinliği
(`sentence`/`capsule`/`chart`/`evidence`), o derinlikte gösterilen görünümün kimliği,
grafik derinliğinde görünen aralık ve kabuğun bildirdiği çerçeve boyutları (§4.6.1). Akış
listesindeki kaydırma konumu kabuğun düzen öğesidir, çekirdeğe gönderilmez. Ajan bakış
durumunu `screen` cevabındaki `view_state` alanından okur (yalnız bir piksel hareketi ya da
açılma onu ayarladıysa). Test: 1000 girdiden sonra günlük uzunluğu ve doküman özeti aynı
(`crates/qp-session/tests/interaction.rs`). Kişi başına: dikkat yönergeleri (en son 16). Varlık durumu günlüğe girmez ve
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
doğruysa yazarı `qp` olan ayrı bir girdi kapsülü `invalidated` yapar. Bar yazısında ve
veri olayında değerlendirilenler, değişen bar serisine dayanan koşullardır (§18.8:
dayanmayanın değeri değişmemiştir). Bir kapsülü yeniden `live` yapan girdiden sonra da
(ör. motorun geçersizleştirme girdisinin geri alınması, silinen kapsülün geri gelmesi) o
kapsülün koşulu **hemen** değerlendirilir; koşul hâlâ doğruysa motor kapsülü yeniden
`invalidated` yapar. Böylece hiçbir kapsül, koşulu doğruyken dayandığı seri tikleyene
kadar `live` kalmaz ve artımlı yol baştan hesapla bayt düzeyinde aynı kalır (RP2 T5
denetimi O1). Geçersizleştirme girdisini geri almak kapsülü kalıcı olarak canlandırmaz;
bunun yolu bulgunun geçersizliğini değiştirmektir (`update_finding`). Etkin niyetlerin
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
- (R5B, §25) Bu dört karar ayrıca **insan kökenli** olmalıdır (yazarı `human`, bağlantı
  ajan kökenli işaretli değil); aksi `forbidden`, `reason: "human_only"`, rol ne olursa
  olsun. Ajan kökenli komut, işaretli bağlantıda ya da `init.agent_policy = "r5b"`
  oturumunda izin listesine tabidir (`agent_cannot_write`, `proposal_only`).
  `Principal.on_behalf`: ajanın okuyucu yetkisi o kişininkidir (kesişim, §25.7).
- `add_comment`: `propose` ya da `write` (doğrudan); `add_watch`/`remove_watch`: `read`.
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

**Hangi bulgu (K25).** `finding` verilmişse o (kimlik ya da seçici, §17). Verilmemişse:
tek canlı bulgu (kapsülü `live`) → o; birden çok → `ambiguous` (§5.4; "otorite rastgele
seçmez", güven ya da kimlik sırasıyla seçim yapılmaz); hiç → `not_found`. Döküm:
`28-present-ambiguous`.

```json
{"type":"query","id":"q2","query":{"kind":"present"}}
{"type":"error","id":"q2","error":{"code":"ambiguous","message":"2 live findings and no `finding`; the authority does not pick one","path":"/query",
 "hint":"Pass `finding` with one of the candidate ids, or a selector such as {\"latest\":true} …",
 "candidates":[{"id":"fnd-overbought","about":"… (cross_above at bar 150, live)"},{"id":"fnd-momentum","about":"… (cross_above at bar 180, live)"}]}}
```

- **`sentence`**: `text` (kapsül başlığı, yoksa bulgunun iddiası) ve `state`. Görsel yok.
- **`capsule`**: + `glyphs` (en çok `max_glyphs`), `confidence`, `spark` (bulgunun ilk
  olgusunun serisinden, son `max_points` bar).
- **`chart`**: + `view` (kapsülün görünümü; yoksa sunucunun kurduğu görünüm — dokümana
  yazılmaz), `facts` (kanıt olgular).

(R4) Aynı kurallar `input` ile açılmada (`open`, `close`, açan `tap_at`) `opening.presentation`
olarak döner (§4.6.1); orada derinliği profil değil bakış durumu seçer.

### 10.3 Sunucu (presenter)

`show_finding` ajanın anlam düzeyinde konuşmasıdır: "bu bulguyu göster". Motordaki
deterministik sunucu görünümü kurar: panel 0'da olgunun bar serisinin mumları; olgu bir
hesap serisindeyse panel 1'de o serinin çizgisi ve olgunun `evidence.level` değerinde bir
kural (`encoding.y` = değerin en kısa yazımı, tamsayıysa ondalıksız: `"70"`); yatay aralık
`x.domain = [t(max(0, olgu barı − 30)), t(son bar)]`; `y` her panel için `linear`.
Katman kimlikleri gramerin kendi adlarıdır (K25; `docs/GRAMMAR.md` §11): `candles`, `line`,
`level` (kapsül görünümünde `line`, `level`, `fact`). Görünüm kimliği verilmezse `view-<seq>-0`. Aynı doküman her zaman aynı
görünümü üretir. Bağlı seride (§18) barlar komutun başındaki okuma dokümanından okunur
(`add_fact`, `set_view_range` gibi); görünüm bar zamanlarını okuduğu için konuşanın bulgunun
olgularının serilerine veri yetkisi olmalıdır, yoksa `no_entitlement` (`/ops/i/finding`).
Ajan grameri doğrudan da yazabilir (`put_view`, `add_layer`); ikisi aynı
komut yolundadır (K21-1, ölçüm `rounds/RD1/experiments/`).

## 11. Zaman: saat, canlı güncelleme, anlatım çizelgesi

- **Saat bir girdidir.** Otoritenin saati yoktur; `host` rolündeki bağlantı `clock`
  gönderir. Günlük zamanları, öneri süre aşımları bu saate göre işler. Aynı girdiler aynı
  sonucu verir.
- **Canlı güncelleme:** veri `append_bars`/`update_last_bar` ile (belge içi seri) ya da
  barındırıcının veri olayıyla (bağlı seri, §18.4) gelir; bar yazısı değişim olarak
  yayılır; ardından **yalnız değişen seriye dayanan** bulgular, niyetler ve koşul
  abonelikleri değerlendirilir ve o seriyi okuyan görünümlerin `view` abonelerine
  `view_changed` gider (artımlı canlı yol, §18.8).
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
| `no_entitlement` | (RP2) Soranın bağlı serinin beslemesi için veri yetkisi yok (borsa veri lisansı; rol yetkisi değil) | `path` (seri başvurusu), `hint` |
| `no_data` | (RP2) Bağlı serinin verisi yok: kaynak bağı tanımıyor, pencere boş ya da çözüm bekliyor | `path`, `hint` |
| `comment_rejected` | (R5B, §25.4) Yorum doğrulayıcıdan geçmedi | `path`, `reasons` (`check`, `code`, `token?`, `sentence?`, `hint`) |

(R5B) `reason`: kodun makinece okunur alt nedeni — `forbidden` için `human_only`,
`agent_cannot_write`, `proposal_only` (§25.2–§25.3). R5B öncesi hatalarda yazılmaz.

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
- Otorite seri kaynağı olarak `inline`, `study` ve (RP2) `bind` kabul eder; `file`,
  `matriks`, `nats` → `unsupported` (eski yer tutucular; yerlerini `bind` aldı). Dosya
  beslemeleri barındırıcının yapılandırmasıyla, taban klasöre hapsedilerek açılır (R1
  bulgusu; `qp_source::open_binding`), protokolden değil. Veri bağının sınırları ve yetki
  denetimi §18.7'de.
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

Bir adım tam olarak birini taşır: `send`, `send_raw` ya da (RP2) `data`. `data` bir veri
olayıdır (§18.4): koşucu onu bağlantıdan değil barındırıcı yolundan motora verir; `expect`
olayın yol açtığı bildirimlerdir (çoğu zaman boş). Veri düzlemi `init.data` ile kurulur
(§18.6). Üç taşımada barındırıcı yolu: süreç içi `Session::data_event`; `qp serve`
`--host-frames` ile barındırıcı çerçevesi (§3); MCP'de koşucunun `Hub::data_event` yolu —
bir MCP aracı **değildir**, ajan veri olayı enjekte edemez. Motorun reddettiği olay adımı
başarısız sayar. Barındırıcı satırları (`{"host":{"gap":…}}`) bağlantı çıktısı
olmadığından `expect`'e girmez; koşucu `qp serve` çıktısında onları atlar.

Koşucu `send.msg`'i **yazıldığı haliyle, ham JSON olarak** iletir (RP2; R2 denetimi K2):
tipli bir `ClientMsg`'e çevirmez. Bu yüzden **anlamla başvuru (§17) seçici nesneleri** içeren
mesajlar da `send` ile yazılır; üç taşıma (süreç içi, `qp serve`, MCP) aynı JSON'u otoriteye
ulaştırır (MCP'de araç argümanı olarak). `send_raw` yalnız JSON olmayan ya da biçimi bozuk
satırlar içindir (`parse`, `schema` hataları).

Kurallar: her adımın çıktıları **tam olarak** `expect` uzunluğunda ve sırasında olmalıdır;
her beklenen mesaj gerçek mesajın **alt kümesi** olarak eşleşir (nesnelerde beklenen her
anahtar eşleşmeli, fazla anahtar serbest; diziler aynı uzunlukta ve öğe öğe; sayılar tam
eşit; `"$any"` her değerle eşleşir; bir nesnede değeri `"$absent"` olan anahtar gerçek mesajda **bulunmamalıdır**). `final.document` da alt küme eşleşmesidir. Her döküm
sonunda koşucu günlüğü baştan oynatır ve dokümanın baytlarının aynı olduğunu denetler.

**Çıktı sırası** (deterministik): önce gönderene cevap; sonra oluşan her girdi için sırayla
`change`, `finding_state`, `proposal` olayları; sonra (RP2) `view_changed` (her bar
değişimi için sırayla); sonra `condition_met` ve `intent_triggered`; sonra (RP2/T8) `claim`
olayları (iddiaların oluşma sırasıyla: barın kapanması ve düzeltmeler, sonra koşullar,
niyetler, geçersizleşmeler; §19.3); her olay, abonelerine bağlantı açılış sırası ve
abonelik sırasıyla.

**Dökümler (RP2'ye kadar 36):** 01–17 RD1, 18–27 R2, RP2'de: 28 belirsiz sunum (K25),
29 geleceğe bakış (K25), 30 veri bağı, 31 yetki yok / veri yok, 32 kanıt doğrulama, 33 tikte
yalnız etkilenenlerin uyanması, 34 koşul yazma yetkisi + çapa geri alma + boşluk, 35
tutarlılık (iddia kanıtı, geçici → kesin → geri çekme, temizlik), 36 sorumlu kişiler ve
yeniden canlanan kapsül. RP2'de değişen var olan fikstürler: döküm 11'in bir `note` metni ve
`fixtures/views/d2-chart.explain.txt` (katman adları; ikisi de K25 gereği,
`rounds/RP2/REPORT.md`). **R3:** 37 itiraz ve geri çekme (`dispute`, kesin bar kaynağı,
geri çekilen kapsül; §19.9); §20 kapsül eki: 38 kapsül yaşam döngüsü (`capsules` konusu,
doğdu → teyit edildi → güçlendi → gerçekleşti, geçersiz; `capsules`, `capsule`, `ledger`),
39 yetki (yetkili / yalnız bar yetkili / yetkisiz okuyucu: lisanslı hikâye, sunucu yorumu,
"gerektirir"), 40 K26 cümlesi (geçmiş + "artık … değil" / "no longer", TR ve EN; süresi
doldu), 41 hassasiyet ve pozisyon geçişi (`set_audience`, `set_sensitivity`), 42 adayın
iz bırakmaması, 43 geri çekme (kapsül itirazı haklı → `retracted`, karnede yok), 44 bağlı
seride `verify_evidence.item` seçicisi (çözüm, `ambiguous`, `schema`) ve `show_finding`
(R3/E düzeltmeleri). 38–44
`cargo run -p qp-session --example make_capsule_transcripts -- --write` ile üretilir. **R4 (girdi):**
51–55: piksel dokunma ve isabet, sürükleme ve
kıstırma, açılma/kapama ve ortak öğeler, kanıta inme (yetkili/yetkisiz), bakış durumunun
günlüğe girmemesi (§4.6.1).
**R4/T2 (§21):** 49 yeniden oynatma denetimi (`init.replay`: durum, adım, hız, duraklat,
sürdür, tarihe ve zamana ileri git, izleyicinin `forbidden`'ı, `invalid`/`schema` hataları,
geri gitmede otoritenin yeniden kurulması, `epoch` 1, bağlantının yeniden `hello`'su), 50
yeniden oynatma kaynağı olmayan otoritede `unsupported`. `cargo run -p qp-session --example
make_replay_transcripts -- --write` ile üretilir. Koşucu `init.replay` taşıyan dökümü bir
yeniden oynatma barındırıcısıyla (`qp_session::ReplayHost`) koşar: süreç içi
`conform::InProcReplay`, `qp serve` (`init.replay` görünce barındırıcıyı kurar;
`--no-local-data`), MCP'de `Hub::shared_replay` (`replay_<kind>` araçları). Dökümlerde
yerel veri hiç okunmaz.
**R5/M18 (§23):** 60–63 enstrümanın hikâye listesi (yetkili okuyucu, yetkisiz okuyucular,
~200 hikâyelik aralık, `stories` konusu); `cargo run -p qp-session --example
make_story_transcripts -- --write` ile üretilir. Var olan dökümler değişmedi.
**R5/M19 (§23.2):** 64 barındırıcının yazı tura kuralı açık (`init.feed_rule`): akış ve liste
`balanced` ve `all` seviyesinde, gizlenen sayısı, pozisyonla K42 muafiyeti, `story` olayları,
karne; `cargo run -p qp-session --example make_feed_rule_transcript -- --write` ile üretilir.
Var olan dökümler değişmedi (kural yokken yeni alanlar hiç yazılmaz).
**R5B (§25):** 65–70 ajan yüzeyi (insan kararı, ajan yazamaz, yorum kabul/ret, yetkisiz kişi
adına yorum, niyet yaşam döngüsü ve eşleşme, önizleme); `cargo run -p qp-session --example
make_agent_transcripts -- --write` ile üretilir. Var olan dökümler değişmedi.

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
`FactSel`, `FindingSel`, `ProposalSel`, `ScaleSel`, `ItemSel`; R4: `CapsuleSel`); bilinmeyen anahtar
`schema` hatasıdır (`did_you_mean` ile). Her seçicide ayrıca `first` ve `latest`
(§17.3) vardır.

| Tür | Anahtarlar | Örnek ("…") |
|-----|------------|-------------|
| seri | `kind` (`bars`/`study`/`points`), `study`, `params` {ad: sayı}, `on` (seri), `symbol`, `label` | `{"study":"rsi","params":{"period":14}}` "RSI(14)"; `{"kind":"bars"}` "fiyat serisi"; `{"study":"sma","params":{"period":50}}` |
| görünüm | `title`, `shows` (seri) | `{}` "tek görünüm"; `{"shows":{"study":"macd"}}` |
| panel | `name`, `shows` (seri), `mark` | `{"mark":"candle"}` "fiyat paneli"; `{"shows":{"study":"rsi"}}` "RSI'ın paneli"; `{"name":"macd"}` |
| katman | `shows` (seri), `output`, `mark`, `panel` (panel), `value`, `fact` (olgu), `finding` (bulgu), `label` | `{"output":"upper"}` "üst bant" (tek başına çizen katman, §17.1); `{"mark":"rule","value":70}` "70 çizgisi" |
| olgu | `kind` (olgunun tam türü), `on` (seri), `bar` | `{"kind":"breakout.range.up","first":true}` "eski yukarı kırılımın olgusu" |
| bulgu | `kind` (olgularından birinin tam türü), `on` (seri), `fact` (olgu), `state` (kapsül durumu); R4: `instrument`, `meaning` (anlam türü), `stage` (hikâye aşaması), `open` (hikâyesi açık mı) | `{"kind":"breakout.range.down","latest":true}` "son aşağı kırılım"; `{"meaning":"breakout","open":true,"latest":true}` "son açık kırılım hikâyesinin bulgusu" |
| kapsül (R4) | `instrument`, `meaning` (anlam türü), `direction`, `stage`, `open`, `born_bar`, `series` | `{"instrument":"THYAO","meaning":"breakout","open":true}` "THYAO'nun açık kırılım kapsülü" |
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
- **Olgu türü tamdır** (R4 açıklaması): motorun olguları `<dedektör>.<kod>` türündedir
  (`breakout.range.up`, `breakout.range.down`, `continuation.ma_stack.bull`,
  `extreme.rsi.overbought`, `flow.institutional.net_buy` …); `{"kind":"breakout"}` bunlara
  **uymaz** (tam eşitlik). Anlam türüne göre seçmek için bulguda ya da kapsülde `meaning`
  (`breakout`, `continuation`, `reversal`, `squeeze`, `extreme`, `flow`) kullanılır.
- (R4) Bulgu ve kapsül seçicisinin hikâye anahtarları (`instrument`, `meaning`,
  `direction`, `stage`, `open`, `born_bar`, `series`) bulguyu sunan kapsülün `story`'sine
  bakar; hikâyesiz bulgu/kapsül bu anahtarlarla eşleşmez. `open: true` aşaması kapanmamış
  (`realized`, `invalidated`, `expired`, `retracted` dışında) demektir.
- **Kapsülün sırası** (R4): son geçişin barı, eşitlikte doğum barı, sonra doküman sırası;
  seçicide `latest: true` **en son hareket edeni** (son geçiş barı en büyük olanı) seçer.
  Bu, `capsules` sorgusunun `latest` süzgecinden farklıdır: o **en son doğanı** verir
  (§20.2; kod: `qp_session::select` kapsül sırası ve `capsules_query`). İkisi de kodun
  davranışıdır; ad aynı, sıra farklı (R4/S1 notu). Kapsül seçicisi yalnız hikâyeli
  kapsüllere çözülür ve kapsül kimliğine (`cap.…`) çevrilir.

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
| sorgu `verify_evidence` / `binding` | `item` (öğe: `{"fact":…}` ya da `{"finding":…}`) / `series` |
| sorgu `capsule` (R4) | `capsule` (kapsül seçicisi; kimlik ya da bulgu kimliği de olur) |
| `input` `open` (R4/S1) | `gesture.capsule` (kapsül seçicisi; kimlik ya da bulgu kimliği de olur) |
| `subscribe` | `changes.ids[]` (öğe), `finding.finding`, `condition.condition` |
| `attend` | `directive.steps[].target` |

- **Katman nesnesi** (`insert_layer.layer`, `add_layer.layer`, `insert_panel.layers[]`):
  `series`, `fact`, `finding`, `panel` ve kodlamadaki seri başvuruları
  (`encoding.{x,y,y2}.series`, `encoding.color.when.than.series`).
- **Hedef** (`Target`): `view`, `panel`, `layer`, `series`, `item` (öğe seçicisi).
- **Koşul** (`Condition`): `threshold.series`, `compare.left`, `compare.right`.
- `put_view`, `put_series`, `input` hareketleri (R4/S1: `open.capsule` dışında) ve kökenler (`origin`) seçici almaz
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
   büyük barı, eşitlikte doküman sırası; kapsül (R4) → son geçiş barı, doğum barı,
   doküman sırası. Görünüm verilmemişse önce görünüm sırası.
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

## 18. Veri düzlemi (RP2) — veri bağı, veri olayları, dondurulmuş kanıt

> K23: piyasa verisi protokolün içinden akmaz; kaynağından kendi biçiminde gelir ve motora
> veri sağlayıcı üzerinden girer. Doküman bar taşımaz, **veri bağı** taşır. Kavramlar
> `docs/CONCEPTS.md` "Veri düzlemi". Tipler `qp-schema::data` ve `qp-schema::protocol`.

### 18.1 İki düzlem

| | Veri düzlemi | Anlam düzlemi (protokol) |
|---|---|---|
| Ne taşır | barlar, canlı fiyat (alan düzeyinde) | komut, değişim, olgu, bulgu, kapsül durumu, koşul olayları, sorgu cevapları |
| Biçim | kaynağın kendi biçimi (Matriks protobuf, NATS, dosya); motora `DataEvent` olarak girer | JSON mesajları (§3) |
| Günlük | girmez | her yazı bir girdi |
| Kim çözer | her taraf kendi veri yetkisiyle (otorite, her yansı) | tek otorite |

Protokolde bar yalnızca küçük sorgu sonuçlarında (`study`, `at`, sınırlı aralıklarla), test
verisinde (`inline`) ve ajanın açıkça istediği kısa aralıklarda bulunur.

### 18.2 Veri bağı

Bir serinin kaynağı `bind` olabilir:

```json
{"op":"put_series","series":{"id":"px","interval":"1d",
  "instrument":{"symbol":"THYAO","exchange":"BIST"},
  "source":{"kind":"bind","symbol":"THYAO","interval":"1d","window":{"last":10000}}}}
```

| Alan | Kural |
|------|-------|
| `symbol` | `^[A-Za-z0-9][A-Za-z0-9._-]{0,31}$` |
| `interval` | kanonik yazım: `Nm` (1 ≤ N ≤ 1440, N 60'ın katı değilse), `Nh` (1 ≤ N ≤ 24), `1d`, `1w`, `1M`. `60m` gibi kanonik olmayan yazım `invalid` (`did_you_mean: ["1h"]`); böylece aynı besleme tek anahtardır. `1d` seans barıdır, `24h` değildir. |
| `window` | tam olarak biri: `{"last": n}` (1 ≤ n ≤ `limits.max_bind_bars`), `{"from_t": t}`, `{"from_t": a, "to_t": b}` (a ≤ b; kapalı pencere canlı bar almaz) |
| `feed` | isteğe bağlı besleme adı (`matriks`, `delayed`…), `symbol` ile aynı desen; yoksa barındırıcının varsayılanı |
| `anchor_t` | **otorite yazar**, istemci yazmaz (`invalid`): bar 0'ın zamanı (§18.3) |

Sembol ve besleme adı büyük/küçük harf **duyarlıdır** (`THYAO` ≠ `thyao`); BIST sembolleri
büyük harftir.

- Bağlı seri `data` taşımaz (`invalid`, `/…/data`). Serinin `instrument.symbol` ve
  `interval` alanları verilmişse bağınkiyle aynı olmalıdır (`invalid`).
- Bağlı seriye `append_bars` / `update_last_bar` → `invalid` ("bağlı serinin verisi veri
  düzleminden gelir"). Bu işlemler belge içi seriler içindir.
- Hesap serileri bağlı serilere bağlanabilir (`inputs`); hesap ve görünüm kodu bağlı ile
  belge içi seriyi ayırt etmez.
- Aynı `symbol` + `interval` + `feed` (besleme, `FeedKey`) bağlı seriler veriyi paylaşır;
  pencereleri farklı olabilir.
- `welcome.summary.series[].binding` soranın bağ durumunu, `len` otoritenin tuttuğu bar
  sayısını verir.

### 18.3 Çözüm, bağ durumu ve çapa

Barındırıcı otoriteye bir **veri sağlayıcı** (`SourceProvider`) verir. Sağlayıcı bir bağı
bir kişi adına çözer: yetki denetimi, ilk barlar, sonra veri olayları. Otorite:

1. **Kendi motor işi için** (bulgu geçersizleşmesi, koşullar, niyetler, sunucu) bağı
   rolü `host` olan ilk kişinin veri yetkisiyle çözer. Çözüm komut yolunda değil, bağ
   dokümana girdiğinde (yüklemede ya da `put_series` uygulandıktan sonra) yapılır; doküman
   değişmez.
2. **Her okuma için** soranın yetkisini ayrıca denetler: soranın beslemeye yetkisi yoksa
   o serinin verisini (ve ona dayanan hesapların değerlerini) okuyan sorgu
   `no_entitlement` döner. Rol yetkisi (`read`) ile veri yetkisi ayrıdır.

**Bağ durumu** (`BindingState`): `ready`, `pending` (sağlayıcı henüz cevap vermedi),
`no_entitlement`, `no_data`, `stale` (veri tutuluyordu ama güvenilmez: `seq` boşluğu ya da
boyut sınırı; barındırıcı yeniden çözene kadar değerler verilmez). `binding` sorgusu her bağlı seri için durumu, nedeni,
**çapayı** (`anchor_t`, bar 0'ın zamanı) ve otoritenin veri konumunu (`DataPosition`:
bar sayısı, son bar zamanı, son uygulanan kaynak sıra numarası, bütün barların özeti)
verir.

**İki çözüm modeli birlikte geçerlidir.** Otorite tek çözüm tutar (yukarıda 1) ve okumaları
soranın yetkisine göre süzer (2). Her **yansı** (cihaz) bağı otoriteden değil kendi veri
sağlayıcısından, kendi kullanıcısının yetkisiyle ve çapadan çözer; otorite yansıya veri
göndermez (K23).

**Veri yetkisi** (`entitlement`) besleme ve sembol düzeyindedir: `SYMBOL` (her beslemede o
sembol), `feed/SYMBOL` (yalnız o beslemede), `feed/*`, `*`. Gerçek zamanlı ve gecikmeli
lisans farklı beslemelerdir (`feed`).

**Yetkisiz kişiye ne gider?** Kural: **veri düzleminin değerleri gitmez, anlam düzlemi
gider.**

| Çıktı | Yetkisiz soran |
|-------|----------------|
| `study`, `at`, `hit` değerleri, `screen` katman `last` ve `last_close`, `present` kıvılcımı ve görünümü, `list full` | verilmez (`no_entitlement` ya da alan boş; `screen`'de katman `binding: no_entitlement`) |
| `summary.series[].len`, bağın kendisi, `binding` sorgusunda durum ve çapa | verilir (veri değil) |
| olgular, bulgular, kapsül durumları ve onların metinleri, kanıt değerleri (`evidence`), `finding_state` | verilir (anlam düzlemi; dokümanın parçası). **Açık soru** (Arkin): olgunun kanıt değerleri (ör. "RSI 71,4") veri lisansı kapsamında mı? Kapsamdaysa yetkisiz kişiye `evidence` değerleri gizlenir; v0 göstermektedir. |
| iddia kanıtının değerleri (`ClaimEvidence.values[].value`/`prev`; §19.7) | `change`, `finding_state`, `claim` (kapsül) ve `log` sorgusunda kanıt **değerleri çıkarılır**; `t`, seri kimlikleri, olgunluk, temizlik, çapa ve beslemeler kalır. Süzgeç kanıtın oluşturulurken kaydettiği beslemelere (`feeds`) göre yapılır, bugünkü dokümana göre değil |
| `condition_met`, `intent_triggered` | **verilmez**: koşulun okuduğu (hesap girdileri üzerinden) her bağlı beslemeye yetkili olmayan aboneye gitmez; o abone için koşul değerlendirilmez, kenar durumu korunur (RP2 T4 denetimi Y1) |
| `verify_evidence` | `no_entitlement` |

**Koşul yazmak veri okumaktır (Y1).** Bir koşulun sonucu, kenar kenar, okuduğu verinin
değerini söyler: eşiği ikiye bölerek yazan biri kapsül durumundan kapanışı 20 komutta
0,001 duyarlıkla öğrenebilir (`rounds/RP2/audits/T4.md`). Bu yüzden bağlı veride
değerlendirilen bir şeyi **yazmak ya da ona abone olmak**, yazanın / abonenin koşulun
okuduğu (hesap girdileri üzerinden, özyinelemeli) **her bağlı beslemeye veri yetkisi**
ister; yoksa `no_entitlement`, koşulun serisinin yolunda:

| Yazı / abonelik | Yol |
|-----------------|-----|
| `add_finding` / `update_finding` geçersizliği | `/ops/i/invalidation/series` (`left`, `right`) |
| `add_intent` tetikleyicisi | `/ops/i/trigger/series` (`left`, `right`) |
| `subscribe` `condition` konusu | `/topic/condition/series` (`left`, `right`) |
| `add_fact` (olgunun bar zamanı) | `/ops/i/series` (önceden de böyleydi) |
| katman sabitinin `fact` kökeni, `field: value` (bağlı olgu) | `/…/origin/<k>/fact` |

Yetki, bağın çözümüne değil dokümandaki bağa bakar (aynı komutta konan bağ da sayılır).
Öneri kabulünde önerenin yetkisi denetlenir (öneri anında da: kuru koşu). Sağlayıcı yoksa
kimse yetkili değildir.

**Koşulu yeniden yönlendirmek de veri okumaktır (RP2 T5 düzeltmeleri).** Bir bulgunun
geçersizliğinin ya da bir niyetin tetikleyicisinin (hesap girdileri üzerinden,
özyinelemeli) dayandığı bir seriyi `put_series` ile yeniden yazan kişi, serinin **yeni**
halinin dayandığı her bağlı beslemeye (kendi bağı ya da hesap girdileri) veri yetkisi
ister; yoksa `no_entitlement`, yol `/ops/i/series/source` (hesap girdisinde
`/ops/i/series/source/inputs/k`). Hiçbir koşulun okumadığı seriyi rolü izin veren herkes
bağlayabilir (aşağıdaki değerlendirme kuralı onu sonradan korur).

**Değerlendirme tarafı.** Bu oturumun kişilerinden, bir bulgunun geçersizliğinden
**sorumlu** olanların hepsi koşulun okuduğu her bağlı beslemeye yetkili değilse
geçersizlik **değerlendirilmez** (yanlış sayılır; kapsül `live` kalır). Sorumlular:

1. bulgunun yazarı (köken yazarı);
2. geçersizliği en son yazan (geçersizliği değiştiren son `add_finding` /
   `update_finding` / geri alma girdisinin yazarı; geçersizliğe dokunmayan düzenleme
   sayılmaz);
3. koşulun dayandığı her serinin (hesap girdileri üzerinden) bağını ya da tanımını en son
   yazan (son `put_series` / geri alma girdisinin yazarı; motorun çapa damgası sayılmaz,
   başlangıç dokümanındaki seriler için kimse).

Sorumlulardan oturumun kişisi olmayanlar (başka yerden gelen doküman, motor) sayılmaz:
yalnız onlar varsa koşul otoritenin görüşüyle değerlendirilir. Kural iki yolu da kapatır:
yetkisiz bir `editor`'ün yetkili birinin bulgusunun geçersizliğini belge içi bir seriye
çevirip seriyi sonra (yetkili birine) bağlatması (2. madde), ve yetkisiz birinin bağladığı
ya da hesap girdisini değiştirdiği seriye yetkili birinin koşul yazması (3. madde); bir
geri alma bağı geri getirirse geri alan sorumlu olur. Kural iki yolda (artımlı ve baştan)
aynıdır. `condition_met` ve `intent_triggered` aboneye kendi yetkisiyle süzülür
(yukarıdaki tablo).

**Açık soru (Arkin) — kalan bir bitlik sızıntı.** Kapsül durumu (`live` → `invalidated`)
anlam düzlemindedir ve belgeyle herkese gider. Koşulu **yetkili** kişiler kurduysa (bütün
sorumlular yetkili), bu durumu okuyan yetkisiz kişi koşulun sonucunu (bir bit: eşik aşıldı
mı, ne zaman) öğrenir. Yetkisiz kişi koşulu kendisi seçemediği ya da yönlendiremediği için
ikili arama yapamaz; ama yetkili yazar isteyerek çok sayıda eşik yazarsa değer sızar. v0
bunu kabul eder; seçenekler: (a) kapsül durumunu da yetkisize gizlemek (`finding_state`
dahil), (c) gevşek bırakıp lisans metnine yazmak. (Önceki (b) seçeneği — koşulun okuduğu
serinin bağını yazanın yetkisini de istemek — yukarıdaki değerlendirme kuralıyla
uygulandı.)

**Uygulama notları (T4, referans otorite).**

- Sağlayıcı yoksa bağ `pending` (`reason`: sağlayıcı yok); rolü `host` olan kişi yoksa
  `no_entitlement` (`reason`: host yok). Sağlayıcı geç cevap verdiğinde ya da boşluk / sınır
  sonrası barındırıcı `Session::refresh_bindings` ile hazır olmayan bağları yeniden çözer.
- Çapa damgası, bağı dokümana sokan girdinin hemen arkasından gelen motor girdisidir
  (yüklemede girdi 1); geçersizleşme girdilerinden öncedir. Yalnız çapası değişen bağ
  yeniden çözülmez.
- Okuma süzgeci bir serinin **bütün** veri bağımlılıklarına bakar: hesap serilerinde bütün
  girdiler (özyinelemeli); katmanlarda kendi serisi, olgusunun / bulgusunun bütün
  olgularının serileri, `x`/`y`/`y2` başvuruları, renk koşulunun sağ yanı ve sabitlerin
  `series`/`fact` kökenleri. Biri soran için hazır değilse katman çizilmez (`screen`'de
  `binding`, `items: 0`, `last` yok); görünümün bar serisi hazır değilse `hit` serinin
  hatasını `/query/view`'da döner. `present` bulgunun herhangi bir olgusu ya da kapsülün
  kayıtlı görünümü okunamıyorsa kıvılcım ve görünüm vermez.
- `welcome.summary.series[].binding` yalnız bağlı serilerde bulunur (hazır olanlarda
  `ready`).
- `verify_evidence` hazır olmayan ya da yetkisiz bağda hata değil, olgu başına durum döner
  (`no_entitlement`, `unavailable`); aşağıdaki tablodaki "hata" bu sorguda olgu durumudur.
- Hazır olmayan bağa dayanan koşul aboneliği ve niyet değerlendirilmez; önceki kenar
  durumu (`last`) korunur.

**Çözülemeyen bağ belgeyi durdurmaz.** Durumu `ready` olmayan bir bağlı seri ve ona dayanan
hesaplar için:

| Yer | Davranış |
|-----|----------|
| `study`, `at`, `hit` öğesi, `verify_evidence` | `no_entitlement` / `no_data` hatası, serinin yolunda |
| görünüm, `screen`, `hit` | o serinin katmanları çizilmez; `screen` katmanında `binding` durumu; diğer katmanlar ve paneller çalışır |
| geçersizlik koşulu, niyet, `condition` aboneliği | değerlendirilmez (yanlış sayılır); kenar durumu korunur |
| `present` | kıvılcım (`spark`) ve görünüm verilmez; metin ve durum verilir |
| komutlar | dokümanı değiştiren komutlar çalışır (veri okumaz); `add_fact` olgunun bar zamanını okuduğu için, koşul yazan komutlar sonuçları veriyi söylediği için yazarın veri yetkisi ister (yukarıda) |

**Bar numaraları ve çapa.** Bağlı serinin bar numarası **çapadan** (bar 0'ın zamanı,
`anchor_t`) sayılır; sonradan gelen barlar sona eklenir, pencere kaymaz. Çapa dokümanda
kalıcıdır: otorite bir bağı ilk kez `ready` olarak çözdüğünde çözümün ilk barının
zamanını, yazarı `qp` olan bir motor girdisiyle (`put_series`, bağın `anchor_t` alanı)
dokümana yazar. O andan sonra her çözüm (otoritenin yeniden başlatmadan sonraki çözümü,
her yansınınki) `from_t = anchor_t`'den başlar; `last` yalnız ilk çözümün büyüklüğüdür.
Böylece `Fact.bar`, `set_view_range`, hedeflerin `from_bar`/`to_bar`'ı ve `condition_met.bar`
yeniden başlatmadan sonra da aynı barı gösterir. Kaynakta tam `anchor_t` zamanlı bar
yoksa (sağlayıcının geçmişi kısaldı ya da değişti) bar numaraları tutmaz: bağ `stale` olur
(`reason`: çapa barı yok), değer verilmez. Çapa ancak bağı yeniden yazan bir komutla
(`put_series`; pencere ya da sembol değişir, `anchor_t` yazılmaz) sıfırlanır; ilk yeni
çözümde yeniden damgalanır.

**Çapa damgası geri alınmaz (Y2, O2).** Çapa damgası girdisinin (yazarı `qp`, her değişimi
`anchor_t`'li bir bağlı seri, tersi aynı seri `anchor_t`'siz) geri alınması — owner için de
— `undo_conflict`'tir ("çapa ancak bağı yeniden yazan bir komutla sıfırlanır"); aksi halde
bağ yeniden çözülüp başka bir çapayla damgalanır ve `Fact.bar` artık olgunun `t`'sini
göstermez. Buna karşılık bağı **koyan ya da yeniden yazan** girdinin geri alınması onun
çapa damgası tarafından engellenmez (o damga, geri alınan girdinin koyduğu bağın aynısını
damgalıyorsa çatışma sayılmaz): geri alma seriyi kaldırır (yeni seri) ya da önceki bağı
**önceki çapasıyla** geri koyar (yeniden yazma); damga onunla gider, bar numaraları
yeniden yazmadan önceki gibidir. Geri almanın geri alınması seriyi damgalı haliyle geri
getirir. Bunun dışında hiçbir yol aynı bağ için farklı bir çapa damgalamaz. Düzlemler arası kimlik yine de **zamandır** (`t`): olgular
`t` taşır, kanıt başvurusu zaman aralığıdır.

**Hesap başlangıcı çapadır.** Hesaplar bar 0'dan başlar; üstel ve Wilder ortalamaları
bütün geçmişi hatırladığından (RSI(14)'te eski bir barın etkisi ~500 bar sonra `f64`
duyarlılığının altına iner; `rounds/RP2/RESEARCH.md` §1.2) farklı bardan başlayan iki taraf
aynı bar için farklı bitler üretir ve eşik yakınında kesişim konusunda bile ayrışabilir. Bu
yüzden her taraf bağı çapadan çözer (dokümandaki `anchor_t`); tutarlılık kuralları (§19)
buna dayanır.

### 18.4 Veri olayları

Veri olayı (`DataEvent`) alan düzeyindedir; yalnızca değişen alanlar gelir (Matriks
`SymbolMessage` `UPDATE` gibi):

```json
{"feed":{"symbol":"THYAO","interval":"1d"},"seq":41,"change":{"kind":"bar","bar":{"t":1760054400000,"o":301.5,"h":301.5,"l":301.5,"c":301.5,"v":0}}}
{"feed":{"symbol":"THYAO","interval":"1d"},"seq":42,"change":{"kind":"patch","t":1760054400000,"fields":{"c":302.25,"h":302.25,"v":1200}}}
{"feed":{"symbol":"THYAO","interval":"1d"},"seq":43,"change":{"kind":"patch","t":1759968000000,"fields":{"c":299.0}}}
```

- `bar`: son bardan sonra yeni bar (`t` kesin büyük).
- `patch`: var olan bir barın verilen alanları (en az biri). Son barda **canlı
  güncelleme**, daha eski barda **düzeltme** (veri sağlayıcının geçmiş düzeltmesi,
  bölünme ayarı).
- `seq`: beslemenin sıra numarası, 1'den boşluksuz. Numarayı **barındırıcının veri
  katmanı** verir ve bir besleme için hiç sıfırlanmaz: kaynak kendi numarasını taşımıyorsa
  (Matriks, QoS 0) ya da yeniden başlarsa barındırıcı yerel, kesintisiz bir numara üretir.
  **İlk barlar ile `seq` hizalıdır:** sağlayıcının `ready` cevabı ilk barlarla birlikte
  `seq`'i verir — ilk barlar o numaraya kadarki (o dahil) bütün olayları içerir; sonra
  gelen `seq` ≤ o numara olan olaylar yinelemedir. Yeniden çözüm aynı kuralla devam eder. Alıcı (otorite ya da yansı):
  `seq` ≤ son → yineleme, yok sayılır (eşgüçlü); `seq` > son + 1 → **boşluk**: besleme
  `stale` olur, olay uygulanmaz, barındırıcıya boşluk bildirilir (yeniden çözme ya da
  boşluk doldurma barındırıcının işi; piyasa verisi geleneği). Bu hata değildir:
  referans otoritede `Session::host_data_event` `Ok(DataOutcome { lines, notices })` döner,
  `notices` içinde `HostNotice::Gap { series, feed, expected, got }` (büyüme sınırında
  `HostNotice::Stale { series, feed, reason }`); bağ başına yalnız `stale`'e geçişte bir
  kez. `qp serve --host-frames` bunu `{"host":{"gap":{…}}}` yazar (§3). `Session::data_event`
  yalnız bildirimleri (`lines`) döner; barındırıcı `host_data_event`'i kullanmalıdır.
- Kurallara uymayan olay (geride kalan `bar`, olmayan `t`'ye `patch`, boş `patch`, sonlu
  olmayan değer, sonrası `validate_bars`'a uymayan bar — ör. `h < l`) uygulanmaz ve
  barındırıcıya hata olarak döner; doküman ve bağlantılar etkilenmez. **Reddedilen olay
  `seq`'i ilerletmez**; sonraki olay boşluk olarak görülür ve besleme `stale` olur (yeniden
  çözüm gerekir). Boşluktan sonra besleme yeniden çözülene kadar `stale` kalır; eksik
  numara sonradan gelse de uygulanmaz.
- **Pencere dışı olay** (kapalı pencerede `to_t`'den sonraki `bar`; çapadan önceki bir bara
  `patch`) yok sayılır ve `seq`'i **ilerletir** (pencereyle ilgisi olmayan veri sahte boşluk
  doğurmasın).
- Kontrol sırası: besleme, yineleme (`seq` ≤ son: içeriğe bakılmadan yok sayılır), boşluk /
  `stale`, olayın biçim kuralları (boş `patch`, sonlu olmayan değer), sonra pencere (pencere
  dışı → yok say, `seq` ilerler), sonra bar kuralları. Yani pencere dışı ama biçimi bozuk olay
  reddedilir (`seq` ilerlemez).
- (T4) Olay beslemesine bağlı her seriye uygulanır. Beslemesi bağlı olup henüz çözülmemiş
  (`pending`, `no_entitlement`, `no_data`) bir seriye gelen olay kabul edilir, bir şey
  değiştirmez (yeniden çözüm onu içerir). Ne bağlı bir serinin ne de otoritenin kendi test
  sağlayıcısının (`init.data`) bildiği besleme hatadır (`unknown feed`).

**Olay protokol mesajı değildir.** Barındırıcı onu motora doğrudan verir (referans otoritede
`Session::data_event`); bağlantılardan gelmez, günlüğe girmez, `change` olayı üretmez.
Yol açtığı şeyler ise anlam düzlemindedir ve bugünkü kurallarla yayılır (§8.1): canlı
kapsüllerin geçersizlik koşulu doğru olursa motor girdisi (`qp`) ve `finding_state`;
`condition_met`; `intent_triggered`. Bir veri olayı yalnızca o beslemeye bağlı serilere
dayanan hesapları, koşulları ve görünümleri etkiler (§18.8).

**Belirleyicilik.** Otoritenin çıktısı (mesajlar + günlük) aynı başlangıç dokümanı, aynı
mesajlar ve aynı veri olayları aynı sırayla verildiğinde aynıdır. Yeniden oynatma
(başlangıç + değişimler) için veri olayları gerekmez.

### 18.5 Dondurulmuş kanıt

**Sayıyı motor söyler (O3).** Olgu bir hesap serisindeyse ve istemci `evidence.value` ya
da `evidence.prev` verdiyse, bunlar hesabın ana çıktısının olgunun barındaki ve bir önceki
bardaki değerine **bit düzeyinde** eşit olmalıdır; değilse `invalid`
(`/ops/i/evidence/value`, `expected`: motorun değeri), motorun değeri yoksa (ısınma, bar 0'da
`prev`) `invalid`. Diğer anahtarlar (`level` …) ve bar serilerindeki olgular denetlenmez.
Böylece `verify_evidence`'in `changed` dediği değer farkı yalnız verinin değişmesinden
gelir. (Bağlı seride okuma dokümanı komutun başındaki haliyle kullanılır; aynı komutta
konan bir hesap serisi denetlenmez.)

Otorite bir olguyu eklerken (`add_fact`; R3'te dedektörler) olguya **kanıt başvurusu**
(`evidence_ref`, `EvidenceRef`) damgalar; istemci göndermez:

```json
"evidence_ref":{"series":"px","binding":{"symbol":"THYAO","interval":"1d","window":{"last":10000}},
  "from_t":1514851200000,"to_t":1754438400000,"bars":1843,"alg":"fnv1a64","digest":"9c1f0e5a7b2d4c11"}
```

- `series`: olgunun serisinin bar serisi (hesap serisinde girdisinin bar serisi, §5.1).
- Aralık: bar 0'dan olgunun barına kadar (kapsayıcı). Gerekçe: göstergelerin değeri bütün
  geçmişe dayanır (üstel ve Wilder ortalamaları); geçmişin herhangi bir yerindeki düzeltme
  iddiayı etkileyebilir.
- `digest`: veri özeti — FNV-1a 64 (standart başlangıç), her bar için sırayla `t`'nin
  (`i64`) ve `o`, `h`, `l`, `c`, `v`'nin IEEE-754 bitlerinin (`-0.0` → `0.0`) 8 bayt
  küçük-endian hali;
  16 küçük onaltılık hane (`qp_schema::data::data_digest`). Her hedefte aynı. Kriptografik
  değildir: düzeltmeyi yakalar, kasıtlı sahteciliği değil (paylaşılan, imzalı kanıt R6'da
  gömülü ikili ek kararıyla birlikte).
- Belge içi serilerde de damgalanır (`binding` yok).

**`verify_evidence`** (`item`: olgu ya da bulgu; kimlik ya da öğe seçicisi, §17.2 —
`{"finding":{"kind":"breakout.range.down","latest":true}}`) her olgu için:

1. Bar serisinin bugünkü barlarında `from_t`…`to_t` aralığını **zamana göre** bulur, bar
   sayısını ve özeti hesaplar (`current`).
2. Olgu bir hesap serisindeyse kanıt değerlerinden `value` ve `prev`'i olgunun barında
   (zamanla bulunur) yeniden hesaplar (`values`: `then` / `now`).
3. Durum: `verified` (özet ve değerler bit düzeyinde aynı), `changed` (ileti ilk farkı söyler:
   bar sayısı, özet, değer), `no_ref`, `unavailable` (bağ hazır değil ya da aralık artık
   yok), `no_entitlement`.

Bulgu `verified` ise bütün olguları `verified`'dır. Sorgu dokümanı değiştirmez.
İleti biçimi (T4): `bar count in t=a…b: n → m`, `data digest x → y: a bar in t=a…b
changed; `value` v → w` (değer farkı varsa eklenir) ya da yalnız değer farkı. Değerler bit
düzeyinde karşılaştırılır; `prev` olgunun barından bir önceki barın ana çıktısıdır.

### 18.6 Test veri sağlayıcısı (dökümler)

`init.data` (`InitData`) otoritenin barındırıcısını taklit eder: beslemeler (`feeds`; her
biri `bars`, `path` ya da `synthetic {n, seed}` ile ilk barlarıyla), yetkiler
(`entitlements`: kişi → `SYMBOL`, `feed/SYMBOL`, `feed/*` ya da `*`; listede olmayan kişinin
yetkisi yok; otoritenin kendi işi rolü `host` olan ilk kişinin yetkisiyle; `synthetic`
beslemesi `interval`'den bağımsız olarak günlük hafta içi barlar üretir) ve hiç cevap vermeyen beslemeler
(`pending`). Adımdaki `data` (§15) veri olayını barındırıcı yolundan verir. Pencere
`{"last": n}` sağlayıcının son `n` barıdır.

### 18.7 Güvenlik ve sınırlar

- Veri bağı dış girdidir: `symbol`, `feed` deseni, `interval` kapalı kümesi, pencere
  kuralları ve `window.last ≤ limits.max_bind_bars` (varsayılan 100.000) şemadan sonra,
  uygulamadan önce denetlenir. Sağlayıcı `from_t` penceresi için daha çok bar verirse
  otorite en son `max_bind_bars` barı tutar ve `binding` sorgusunun `reason`'ında söyler
  (çapa bu kırpılmış ilk bardır). Açık pencere canlı barlarla büyür; tutulan bar sayısı
  `2 × max_bind_bars`'a ulaşırsa bağ `stale` olur (`reason`: sınır) ve yeni bar
  uygulanmaz. Çapası olan bağın çözümü de aynı `2 × max_bind_bars` sınırını kullanır (aksi
  halde yeniden başlatma `ready` bağı `stale` yapardı); çapa kırpılmaz; yeniden çapalamak bağı yeniden yazan bir komut ister. Bir dokümandaki bağlı
  seri sayısı `max_ops × 4`'ü (256) aşamaz (`limit`).
- Patch sonrası bar `qp_schema::validate_bars` kurallarına uymalıdır (sonlu değerler);
  uymayan olay uygulanmaz.
- Kapalı pencerede `to_t`'den sonraki `bar` olayı pencere dışıdır: yok sayılır, `seq`'i
  ilerletir (§18.4); `to_t` içindeki bir bara `patch` (düzeltme) uygulanır.
- Dosya beslemeleri barındırıcı yapılandırmasıyla, taban klasöre hapsedilerek açılır
  (`qp_source::resolve_binding_path`); protokolden yol gelmez. `init.data.feeds[].path`
  depo köküne hapsedilir.
- Veri yetkisi her okumada soranın kimliğiyle denetlenir; bir kişinin yetkisi olmayan veri
  ona hiçbir sorgu cevabında (değer, kıvılcım, nokta sorgusu) ve hiçbir koşul bildiriminde
  (`condition_met`, `intent_triggered`) gitmez; veride değerlendirilen koşulu ancak yetkili
  kişi yazar (§18.3, "Koşul yazmak veri okumaktır"; kalan bir bitlik sızıntı açık soru).
- Bağlı seride `window.last` sınırı oturumun `limits.max_bind_bars`'ıdır (varsayılanı değil).
- Anlık görüntü (`document`) bağlı serilerin barlarını içermez (doküman taşımaz); paylaşılan
  doküman veri lisansını taşımaz (K23).

### 18.8 Artımlı canlı yol ve etkilenenler

Bir veri olayı ya da belge içi bar yazısı (`append_bars`, `update_last_bar`) dokümanı
kopyalamadan uygulanır ve yalnızca etkilediklerini günceller: o beslemeye (seriye) dayanan
hesapların akışlı durumu (`push` / son barı düzelt; geçmiş düzeltmede o hesaplar baştan),
o hesaplara dayanan koşullar ve canlı kapsüller, o serileri çizen görünümler. Etkilenmeyen
görünüme ve aboneliğe hiçbir mesaj gitmez. Sonuç baştan hesapla bayt düzeyinde aynıdır.

**Etkilenen kümesi.** Değişen bar serisi `X` olsun (veri olayında beslemesine bağlı her
seri; bar yazısında yazılan seri).

- Hesap serileri: girdisi `X` olan her hesap serisi. `X` bir kez tikledikten sonra bu
  hesapların hepsi her tikte güncel tutulur (görünmese de); başka bar serisindeki hesaplar
  hiç dokunulmaz.
- Koşullar (`condition` abonelikleri, etkin niyetlerin tetikleyicileri, canlı kapsüllerin
  geçersizlik koşulları): okuduğu serilerden biri `X`'e (hesap girdileri üzerinden,
  geçişli) dayanıyorsa değerlendirilir; dayanmıyorsa değeri değişmemiştir, değerlendirilmez
  ve kenar durumu korunur. Geçersizleşme motor girdisi ve bildirimleri §8.1 ve §15'teki gibi.
  Bu eşitlik, canlı bir kapsülün koşulunun en son değerlendirildiği andan beri yalnız
  dayandığı serilerin değişmesiyle değişebilmesine dayanır: kapsülü `live` yapan, bulgusunu
  ya da dayandığı bir seriyi yazan her girdiden sonra o kapsül değerlendirilir (§8.1).
- Görünümler: katmanlarının okuduğu seriler (`series`, olgu ve bulgu olgularının serileri,
  `x`/`y`/`y2` başvuruları, renk koşulunun sağ yanı, sabitlerin `series`/`fact` kökenleri)
  ve onların hesap girdileri (geçişli) `X`'i içeriyorsa `view` abonelerine `view_changed`.

**`view` konusu ve `view_changed`.**

```json
{"type":"subscribe","id":"va","topic":{"kind":"view","view":"va"}}
… feed 1'de son barın canlı düzeltmesi:
{"type":"notify","sub":"va","event":{"kind":"view_changed","view":"va","series":"px",
 "from_bar":99,"to_bar":99,"seq":1}}
```

- `series`: barları değişen bar serisi (görünüm onu ya da ona dayanan bir hesabı okur).
  Bir değişim, bir görünüme seri başına bir olay.
- `from_bar`: değerleri değişmiş olabilecek ilk bar (`series`'in bar dizini): yeni bar,
  düzeltilen son bar, geçmiş düzeltmede düzeltilen bar (sonrası da değişebilir); bağ
  durumunun değişmesinde (ör. `stale`) 0. `to_bar`: `series`'in şimdiki son barı; bar yoksa
  yok. Kabuk `from_bar`'dan sonrasını yeniden çizer.
- `seq`: değişimin görüldüğü günlük konumu (bar yazısında girdinin `seq`'i; veri olayı
  girdi yazmaz: o anki konum).
- Yalnız veri içindir: görünümün tanımındaki değişim (`put_view`, katman işlemleri,
  `set_params`) günlük girdisidir ve `changes {ids}` ile gelir.
- Bağlı seride olay yalnız o beslemeye veri yetkisi olan kişilere gider (zamanlama da veri
  düzlemindendir; §18.3). Konu kimlik ya da seçici alır; görünüm yoksa `not_found`.

**Uygulama (referans otorite).** Belge içi bar komutu dokümana yerinde uygulanır (başarısız
işlem uygulananları tersleriyle geri alır); günlük girdisi kopyalayan yolunkiyle bayt bayt
aynıdır. Motor girdileri de yerinde yazılır. Veri olayında okuma dokümanına yeni bar eklenir,
düzeltilen bar yerine yazılır (bar dizisinin kopyası yok). Hesap değerleri seri başına akışlı
durumla tutulur. Koşulların yapıdaki yeri (dayandığı seriler, aralarındaki bağlı seriler ve
beslemeleri, kapsülün sorumluları; §18.3) bir dizinde tutulur ve günlüğe yalnız bar yazısı
olmayan bir girdi geldiğinde yeniden kurulur; her koşul, niyet ve kapsül değerini önbellek
yuvalarından okur; veri yetkisi bir değişimde kişi ve besleme başına bir kez sorulur.
Kaynak: `crates/qp-session/src/live.rs`, `values.rs`; ölçüm ve tasarım
`rounds/RP2/experiments/T5.md`, `T5-fixes.md`.

## 19. Tutarlılık: otorite ile yansının aynı veriye baktığı (RP2)

> Seçilen tasarım (RP2/T7, `rounds/RP2/experiments/T7-COMPARE.md`): **B + §18.5**. Otorite
> veri kaynaklı her iddiasına kanıt değerlerini ekler; yansı bir iddiayı yalnız **kendi
> verisinde** aynı yüklem tuttuğunda gösterir. Dondurulmuş kanıt (`EvidenceRef`,
> `verify_evidence`, §18.5) olduğu gibi kalır. Kavramlar `docs/CONCEPTS.md` "Tutarlılık".
> Referans: `crates/qp-session/src/claims.rs` (otorite), `replica.rs` (yansı).

### 19.1 Sorun ve kural

Veri protokolün dışından akar (K23): otorite ile yansı aynı beslemeyi farklı anlarda,
farklı kayıplarla, bazen farklı kaynaklardan görür. Otorite "koşul t'de doğru oldu" dediğinde
yansının verisi henüz o barı görmemiş, aynı barı başka değerle görmüş ya da hiç
görmeyecek olabilir. **Kural:**

1. Otoritenin veri kaynaklı her anlam mesajı **iddianın kanıtını** taşır (§19.2).
2. Yansı bir iddiayı yalnız şu üçü birlikte doğruyken **iddia olarak** gösterir: otorite onu
   ileri sürüyor (`asserted`); yansının kendi verisi `t`'yi **aynı çapadan** çözülmüş
   olarak kapsıyor; iddianın yüklemi (koşulun kendisi) **yansının kendi verisinde** `t`'de
   tutuyor. Veri her değiştiğinde aynı adımda yeniden sınanır; tutmadığı anda gizlenir.
3. Gösterilen değerler **yansının kendi değerleridir** (ekrandaki kanıt iddiayla çelişemez).
4. **Kesin** (`final`) etiketi: yansıda bar kapalı **ve** otorite kesin dedi **ve** iki
   taraftan en az biri temiz (§19.4). Hiçbir şey erken gösterilmez.

Bu, yanlış (M2/M3) ve erken (M1) gösterimi yapı gereği sıfırlar (RP2/T7 ölçümü; T8'de
gerçek yansıyla yeniden ölçüldü, `rounds/RP2/experiments/T8.md`).

### 19.2 Mesaj alanları: `ClaimEvidence`

Şu olaylar isteğe bağlı `evidence` alanı taşır (eklemelidir; eski istemci yok sayar):
`condition_met`, `intent_triggered`, veri kaynaklı `finding_state` ve onu yazan motor
girdisinin `change`'i (günlük girdisinde `LogEntry.evidence`; `log` sorgusuyla yetişen
yansı da alır). Ayrıca yeni `claim` olayı (§19.3) her zaman taşır.

```json
"evidence":{"series":"px","t":1760054400000,
  "values":[{"series":"px","t":1760054400000,"value":106.0,"prev":103.0}],
  "maturity":"provisional","clean":true,"anchor_t":1759708800000,
  "feeds":[{"symbol":"THYAO","interval":"1d"}]}
```

| Alan | Anlam |
|------|-------|
| `series` | İddianın bar serisi (koşulun ilk işleneninin bar serisi; bağlı ya da belge içi) |
| `t` | İddianın bar zamanı: düzlemler arası kimlik (§18.3). İddia kimliği = neyin iddiası + `t` |
| `values` | Koşulun her işleneni için (koşul sırasıyla) ana çıktının `t`'deki (`value`) ve bir önceki bardaki (`prev`) değeri; ısınmada yok. İşlenen başka bir bar serisindeyse kendi `t`'si |
| `maturity` | `final`: bar otoritenin verisinde kapalı (sonrasında bar var ya da pencere kapalı, `to_t`); `provisional`: canlı bar. Koşullar son barda değerlendirildiğinden kenar anında hep `provisional`'dır; kesinlik `claim` ile gelir |
| `clean` | Otoritenin verisi `t`'ye kadar onarılmamış sıra boşluğu içermiyor. Boşluk (§18.4) beslemeyi `stale` yapar; o anki son bar ve sonrası "kirli"dir; yeniden çözüm kayıp olayı kapsarsa (çözümün `seq`'i ≥ beklenen) ya da kayıp olay sonradan sırayla gelirse temizlenir |
| `anchor_t` | Hesap başlangıcı: bar serisinin çapası (§18.3). Belge içi seride yok |
| `feeds` | Kanıtın dayandığı bağlı beslemeler (bar serisi ve işlenenler, hesap girdileri üzerinden), iddia **oluşturulurken** kaydedilir; veri yetkisi süzgeci bunlara bakar (§19.7) |

Değerler **hesap önbelleğinin sütunlarındandır**: kenarı bulan değerlendirme onları zaten
okumuştur; ek hesap yoktur. Kanıt bar taşımaz (K23): birkaç sayıdır.

### 19.3 `claims` konusu ve `claim` olayı

```json
{"type":"subscribe","id":"k","topic":{"kind":"claims"}}
{"type":"notify","sub":"k","event":{"kind":"claim",
  "claim":{"kind":"condition","sub":"c"},"state":"asserted",
  "evidence":{"series":"px","t":1760054400000,"values":[…],"maturity":"final","clean":true,…},
  "seq":2}}
```

- `claim` (`ClaimRef`): `{"kind":"condition","sub":…}` (alıcı bağlantının kendi koşul
  aboneliği), `{"kind":"intent","intent":…}`, `{"kind":"capsule","capsule":…,"finding":…}`.
  Koşul iddiaları yalnız aboneliğin bağlantısına, diğerleri bütün `claims` abonelerine gider.
- `state`: `asserted` ya da `withdrawn`. Olgunluk ve temizlik `evidence`'tadır.
- Yaşam döngüsü (otorite, `Claims::after_bars`):
  1. **Kenar** (koşul doğru oldu, niyet tetiklendi, kapsül geçersizleşti): `asserted`,
     `provisional`.
  2. **Bar kapandı** (o seriye yeni bar geldi): iddia kapanan barda yeniden sınanır (önbellek
     değerleriyle). Tutuyor → `asserted`, `final` (kapanış değerleriyle); tutmuyor →
     `withdrawn`.
  3. **Düzeltme** (kapalı bir bar düzeltildi): o bardan sonraki kayıtlı iddialar yeniden
     sınanır; değerleri değiştiyse yeniden `asserted`, tutmuyorsa `withdrawn`.
  4. **Boşluk** (besleme `stale`): kirlenen iddialar son değerleriyle `clean:false` olarak
     yeniden `asserted`. **Yeniden çözüm**: yeniden sınanır (temizlik yeniden hesaplanır).
  5. Canlı barın kendi tikleri iddiayı yeniden göndermez (gürültü); bar kapanınca 2. adım.
- Doküman `withdrawn` ile değişmez: geçersizleşen kapsül geçersiz kalır (motor girdisi
  günlükte); yansının görüntüsü iddiayı düşürür. (Öneri: kapsül geçersizleşmesinin
  kesinleşmeyi beklemesi, `T8.md` §8.)
- Otorite en çok 4096 iddiayı izler (en eskisi düşer).

Döküm: `fixtures/protocol/conformance/35-consistency.json` (canlı barda geçici → bar
kapanınca kesin → kapalı barın düzeltmesiyle geri çekildi → yeni geçici → sıra boşluğundan
sonra `clean:false`; `condition_met`, `finding_state` ve motor girdisinin `change`'i kanıtlı).

### 19.4 Yansının algoritması ve görüntü kuralı

Yansı (`qp_session::Replica`) otoriteyle **aynı motor kodunu** çalıştırır: dokümanı aynı
uygulama yoluyla izler, veri düzlemi aynı `FeedStore` kurallarıyla (§18.4), hesaplar aynı
önbellek ve canlı yolla (§18.8), yüklem aynı koşul anlamıyla, görünüm aynı gramer
derleyicisiyle. Kendi motor girdisi yazmaz, çapa damgalamaz, izleyici değerlendirmez.

1. `welcome` → `subscribe changes`, `subscribe claims`, `query document`. Doküman `seq`
   S'de yüklenir; `change` olayları `seq` sırasıyla uygulanır (S'den küçükler yok sayılır,
   delik varsa `{"kind":"log","after_seq":…}` ile yetişilir).
2. Bağlı seriler **kendi** sağlayıcısından, kendi kişisinin yetkisiyle, dokümandaki
   `anchor_t`'den çözülür (§18.3). Barındırıcı kendi beslemesinin olaylarını verir.
3. Her `claim` (ve yedek olarak kanıtlı `change`) iddia başına son sözü günceller.
4. Her girdiden sonra (mesaj, veri olayı, barındırıcı işareti) her `asserted` iddia için:

| Durum (kendi verisinde) | Görüntü |
|---|---|
| koşul bilinmiyor; bağ hazır değil (`no_entitlement`, `stale`, `no_data`); çapa farklı ya da yok; `t` kendi verisinde yok | `pending` (nedeniyle) |
| yüklem tutuyor, otorite `final`, kendi barı kapalı, (otorite temiz ∨ kendisi temiz) | `shown`, `final` |
| yüklem tutuyor, otorite `final`, barındırıcı barı kesinleşmiş değerle verdi (`set_settled`) ama grafik kapatmadı | `shown`, `provisional` |
| yüklem tutuyor, otorite `provisional`, kendi barı canlı, **canlı gösterim açık** | `shown`, `provisional` |
| yüklem tutuyor, başka her durum (otoritenin kesinini ya da kendi kapanışını bekliyor; iki taraf da kirli) | `pending` |
| yüklem tutmuyor, otorite `final`, kendi barı kapalı ve temiz | barındırıcıdan kesin bar çekme ister (`take_refetch`); çekildikten (`refetched`) sonra hâlâ tutmuyorsa `disputed` ("kaynaklar farklı"; iddia olarak gösterilmez) |
| yüklem tutmuyor, otorite `final`, kendi barı kapalı değil ama barındırıcı kesinleşmiş değerle verdi (`set_settled`), temiz | (R3/M7) bir kez kesin bar çekme ister (`take_refetch`, `[çapa, t]`: geride kalan beslemenin kaçırdığı düzeltme ve bölünme önceki barlarda olabilir); `pending` (`not_held`) — bar kapanmadan `disputed` olmaz |
| yüklem tutmuyor, başka durum | `pending` |

`withdrawn` iddia görüntüden çıkar. Görüntü durumu belirleyicidir (iddia ve `t` sırasıyla).

**Canlı bar gösterimi (ürün kararı).** Varsayılan **kapalı** (`ReplicaOptions.live`):
yansının verisi gerçeğin en az bir ağ gecikmesi gerisindedir; canlı barda geçici iddia sıfır
hatayla gösterilemez (`T7-COMPARE.md` ortak bulgu 3). Açıldığında yalnız otoritenin geçici
iddiası ve kendi canlı barında yüklem tutarken `provisional` gösterilir.

### 19.5 Barındırıcının görevleri (yansı tarafı)

Veri düzlemi barındırıcınındır (§18.4); yansı şunları ister ya da kabul eder:

- **Kendi beslemesi:** olayları sırayla, boşluksuz `seq` ile verir (kaynak boşluğunu kendi
  veri katmanında onarabilir ya da boşlukta `Replica::resync` ile yeniden çözer).
- **Kesin bar çekme** (settled fetch, Matriks REST karşılığı): `take_refetch()` istekleri
  için `[çapa, t]` aralığını kesinleşmiş kaynaktan çeker, verisine uygular (olay ya da
  `resync`), `refetched(series, t)` der.
- **Kirli işareti:** kendi beslemesinde bir barın kapanış olayları kaybolmuş olabilirse
  `set_unclean(series, from_t)`; onarınca `None`.
- **Kesinleşmiş değer işareti:** beslemesi geride kaldığı için bir iddianın barını kesin
  kaynaktan doldurduysa `set_settled(series, t)`.
- **Çapa:** sağlayıcısının geçmişi çapaya kadar uzanmıyorsa eksik aralığı kesin kaynaktan
  tamamlar (yoksa bağ `stale`, iddialar `pending`).

### 19.6 Doküman ve görünüm özetleri; yerel ↔ WASM

- `document_digest`: dokümanın JSON baytlarının FNV-1a 64'ü (16 onaltılık).
- `view_digest`: görünümün sahne tarifinin (`SceneDesc`, verilen boyutta, tarafın kendi
  verisiyle derlenmiş) JSON baytlarının FNV-1a 64'ü.
- Aynı tik akışında otorite ile yansının özetleri her adımda aynıdır; yansı ister yerel
  ister WASM olsun (`qp replica-tape`, `crates/qp-wasm/tests/replica_tape.mjs`,
  `scripts/verify.sh` adım `replica`). Yansının verisi geride kaldığında görünüm özeti
  farklıdır (doğru: farklı veri), doküman özeti aynıdır; veri yetişince ikisi de aynıdır.

### 19.7 Veri yetkisi

Kanıt değerleri veri düzlemi değeridir (§18.3). `feeds`'teki **her** beslemeye yetkisi
olmayan alıcıya `values[].value` ve `prev` gönderilmez (`change`, `finding_state`, kapsül
`claim`'i, `log` sorgusu); anlam (`t`, seri kimlikleri, olgunluk, temizlik) gider. Koşul ve
niyet iddiaları, `condition_met` / `intent_triggered` gibi, yetkisiz aboneye hiç gitmez
(§18.3, T4 Y1). Süzgeç kanıtın oluşturulurken kaydettiği beslemelere bakar (sonradan bağ
değişse ya da silinse de; bilinmeyen → değer yok). Saklanan günlük girdisi değerlerle
kalır (yeniden oynatma ve yetkili okurlar). **Yetkisiz yansı** kendi verisini çözemez
(`no_entitlement`): iddiaları doğrulayamaz, hepsini `pending` (`reason: no_entitlement`)
gösterir; değer göstermez.

### 19.8 Dondurulmuş kanıtla ilişki ve sınırlar

- `ClaimEvidence` canlı tutarlılık içindir (bu an, bu bar). Sonradan sınanabilirlik
  `EvidenceRef` ve `verify_evidence`'tadır (§18.5): bit eşit değerler aynı veriyi kanıtlamaz
  (iki farklı geçmiş aynı RSI'yı verebilir); veri özeti bunu yakalar.
- Koşullar son barda değerlendirilir: geçmişte bir düzeltmenin **yeni** doğurduğu iddia
  (eski bir barda kesişim) iddia edilmez; yalnız kayıtlı iddialar yeniden sınanır.
- Yansıdan otoriteye "itiraz" (`dispute`) R3'te eklendi (§19.9); "sen söylemiyorsun"
  (`check`) mesajı hâlâ yok (öneri). `disputed` yansıda görünür.
- Kanal sırası: `qp/0` bağlantısı sıralı bir akıştır (§3). Sırasız taşımada çerçeve numarası
  taşıma düzeyinde sırayı kurar (düzenekte `bs` böyle yapar).
- İki uç aynı kapanış tikini kaybetmiş ve ikisi de temiz sanıyorsa aynı yanlışta
  anlaşabilirler; buna karşı kesinlik sağlayıcının kesin barına dayanır (barındırıcının
  kirli işareti ve kesin bar çekmesi).

### 19.9 İtiraz ve geri çekme (R3)

> K36 (A20 cevabı): "ikisi birden" — otorite kesin barla doğrular ve yansıdan itiraz yolu
> açılır. Bu alt bölüm §19.8'deki "`dispute` v0'da yok" maddesinin yerini alır (`check`
> yine yok). Referans: `crates/qp-session/src/dispute.rs`; ölçüm
> `rounds/R3/experiments/M7/RESULTS.md`. Kavramlar: CAPSULE.md §9.3 (`retracted`, K37).

**Sorun.** §19'un kuralı yansıyı yanlış göstermekten korur; ama otorite **kendi**
beslemesinden (bozuk tik, sıra dışı olay, bölünme) yanlış bir **kesin** iddia çıkarırsa
yansı onu `disputed` gösterir ve iddia orada kalır: otorite geri çekmez, doğru iddia hiç
gelmez (RP2 son denetimi O1). İki mekanizma, ikisi de barındırıcı üzerinden ve belirleyici:

**1. Otoritenin kesin bar hijyeni.** Barındırıcı açar (`Session::set_settle_check(true)`;
dökümde `init.data.settled.verify_closed`). Bağlı serinin bir barı kapandığında ya da
besleme kapalı bir barı düzelttiğinde (o bardan son kapalı bara kadar) otorite
barındırıcıdan **kesin bar** ister (`take_settle_requests()` → `SettleRequest {series, feed,
from_t, to_t}`; Matriks REST karşılığı, §19.5'in otorite tarafı). Barındırıcı kesin
kaynaktan çeker (bar henüz kesinleşmediyse sonra yeniden dener) ve cevabı verir:
`settled_bars(series, bars)`. Otorite:

- Kendi barlarından farklı olan **kapalı** barları kesin değerlerle düzeltir (canlı bar
  hiç düzeltilmez; tutmadığı bar yok sayılır).
- O barlara dayanan kayıtlı iddiaları yeniden sınar (§19.3 adım 3): tutmuyorsa
  `withdrawn`, değerleri değiştiyse yeniden `asserted` — ikisi de `reason: "data_corrected"`.
- Cevabın kapsadığı ilk bardan son kapalı bara kadar, koşul aboneliklerinin **kesin
  veride** tuttuğu ama kayıtlı olmayan iddialarını `asserted` (`final`, `data_corrected`)
  yapar. Hijyen açıkken beslemenin kendi düzeltmesi de (kapalı bir bara `patch`) aynı
  taramayı hemen yapar (sağlayıcının düzeltmesi; kesin cevap sonra doğrular). Bu, §19.8'deki
  "düzeltmenin yeni doğurduğu iddia iddia edilmez" sınırını hijyen açıkken kaldırır (ör.
  beslemede kaybolan kapanış tiki, iddia doğuran düzeltme). Niyet ve kapsül
  geçersizleşmeleri taranmaz.
- Hijyen açıkken **yeniden çözüm** (`Session::resync_bindings`; ör. bölünmenin anlık
  görüntüsü, onunla birlikte gelen düzeltme) kapalı barları değiştirdiyse bu da
  sağlayıcının düzeltmesidir: değişen ilk kapalı bardan son kapalı bara kadar kesin istek
  (`SettleRequest`) ve aynı tarama hemen yapılır (R3/M7 AUD2 düzeltmesi). Hijyen kapalıyken
  yeniden çözüm eskisi gibi yalnız kayıtlı iddiaları yeniden sınar.

Kesinlik bu doğrulamayı **beklemez** (gecikme maliyeti yok); yanlış kesin iddia en geç
"kesinleşme gecikmesi + kesin kaynak cevabı" kadar sonra düşer.

**2. Yansının itirazı (`dispute`).** Yansı bir iddiayı kendi temiz, yeniden çekilmiş
verisinde `disputed` yaptığında (§19.4) otoriteye itiraz gönderir — (iddia, `t`,
otoritenin kanıtı) başına bir kez: yalnız kendi verisinin özeti değişti diye yeniden itiraz
etmez (haksız itirazın cevabı aynı kanıtı yeniden ileri sürer: döngü yok)
(`ReplicaOptions.dispute`, varsayılan kapalı; barındırıcı açar):

```json
{"type":"dispute","id":"d1","claim":{"kind":"capsule","capsule":"cap-floor","finding":"fnd-floor"},
 "t":1760054400000,"values":[{"series":"px","t":1760054400000,"value":104.0,"prev":103.0}],
 "anchor_t":1759708800000,"digest":"<yansının çapadan t'ye kadar barlarının data_digest'i>"}
```

- Yalnız otoritenin **kayıtlı** iddiası itiraz edilebilir (koşul iddiası: gönderenin kendi
  aboneliği; kapsül ve niyet iddiası: dokümanın). İddianın dayandığı her beslemeye veri
  yetkisi olmayan kişinin itirazı, ileri sürülmeyen iddianınkiyle **aynı** cevabı alır
  (`unknown`; karar veri hakkında bilgi verir, §19.7; iddianın varlığı da sızmaz). Okuma
  izni yeter (izleyici itiraz edebilir). `digest` verilirse tam 16 küçük onaltılık hanedir
  (`data_digest`), değilse `invalid` (`/digest`).
- Otorite **yansının değerleriyle karar vermez**: `values`, `anchor_t`, `digest` kayıt ve
  eşgüçlülük içindir. Karar kesin veridedir: otorite barındırıcıdan `[çapa, t]` aralığının
  kesin barlarını ister (`SettleRequest`) ve cevapta (yukarıdaki 1. maddenin adımları)
  iddiayı yeniden sınar.
- **Haklı** (kesin veride tutmuyor): iddia `withdrawn`, `reason: "dispute_upheld"` (bütün
  alıcılarına). **Haksız** (tutuyor): iddia kalır; otoritenin kanıtı yalnız itiraz edene
  `asserted`, `reason: "dispute_rejected"` olarak gider; yansı `disputed` göstermeye devam
  eder (kaynaklar gerçekten farklı).
- Cevap bir `result`'tır: `{"kind":"dispute","verdict":…,"evidence"?}`. `verdict`:
  `pending` (barındırıcının kesin cevabı bekleniyor; karar sonra `claim` olayıyla gelir),
  `upheld`, `rejected` (+ otoritenin kanıtı), `duplicate` (aynı bağlantıdan aynı iddia,
  `t` ve `digest` daha önce itiraz edildi), `unknown` (otorite bu iddiayı ileri sürmüyor:
  hiç ya da artık, ya da kişinin yetkisi yok), `limited` (bağlantı başına en çok 16 açık
  itiraz, ömrü boyunca en çok 256 itiraz; aynı iddia ve `t` için en çok 4 farklı `digest`).
- Eşzamanlı kesin kaynağı olan barındırıcıda (test sağlayıcısı) karar aynı adımda verilir.
  Kesin kaynağı olmayan barındırıcıda itirazlar `pending` kalır.
- Taşıma: `qp serve` ve süreç içi yolda sıradan bir mesajdır; MCP'de araç değildir (yansı
  barındırıcısının mesajıdır, ajanın değil; `input` gibi doğrudan iletilir).

**Kapsülün geri çekilmesi (K37).** Bir kapsül iddiası (geçersizleşmesi) kesin veri ya da
haklı itirazla `withdrawn` olursa otorite motor girdisi (`qp`) yazar: `put_capsule`, hikâyenin
aşaması `retracted`, geçmişine `{stage: "retracted", bar, t, reason: "data_corrected" |
"dispute_upheld"}`. Karneye satır yazılmaz. Hikâyesi olmayan (R3 öncesi) kapsülde girdi
yazılmaz, yalnız iddia düşer. `state` alanı (R3 öncesi) değiştirilmez: o tipte `retracted`
yok (şema önerisi: `rounds/R3/experiments/M7/RESULTS.md`). Sıradan besleme düzeltmesiyle
(sebepsiz) düşen kapsül iddiası bugünkü gibi dokümanı değiştirmez (§19.3).

**`claim` olayına ek.** İsteğe bağlı `reason`: `data_corrected`, `dispute_upheld`,
`dispute_rejected`. Sıradan yaşam döngüsünde (§19.3) yoktur; eski istemci yok sayar.

**Barındırıcı ek API'si (referans otorite).** `set_settle_check`, `take_settle_requests`,
`settled_bars`, ve `resync_bindings` (bütün bağlı serileri sağlayıcıdan yeniden çöz: ör.
bölünme anlık görüntüsünü sağlayıcısına koyan barındırıcı; kayıtlı iddialar yeniden
sınanır). Test sağlayıcısının kesin kaynağı: `init.data.settled = {feeds: [{feed, bars}],
verify_closed}` — sağlayıcının kapalı barları, verilen barlar onlarınkinin yerine.

Döküm: `fixtures/protocol/conformance/37-dispute-retract.json` (otoritenin bozuk tikinden
yanlış kesin kapsül iddiası → yansının itirazı haklı: `withdrawn` + `dispute_upheld`, kapsül
`retracted`, karne boş; kesin verinin desteklemediği itiraz: `rejected`, kanıt yalnız
itiraz edene; aynı itiraz `duplicate`; ileri sürülmeyen iddia `unknown`).

## 20. Kapsül (R3)

Çerçeve `docs/CAPSULE.md` (§9 uygulama), kavramlar `docs/CONCEPTS.md` "Kapsül (R3)".
Şema `qp-schema::protocol` (ek alanların hepsi isteğe bağlı; R3 öncesi dökümler bayt bayt
aynı), referans otorite `crates/qp-session/src/capsule.rs`. MCP araçları şemadan üretilir
(`op_set_audience`, `op_set_sensitivity`, `query_capsules`, `query_capsule`,
`query_ledger`); R4 alanları (`capsule` seçicisi, `latest`, `born_since_*`, `direction`,
`validity`) araçlarda da görünür.

### 20.1 Otoritede kapsül motoru

Barındırıcı yapılandırması (`SessionInit.capsules`, protokol mesajı değil):

```json
"capsules": {"series": ["px"], "flow": [{"series": "px", "seed": 1, "feed": "flow"}]}
```

- `series`: motorun koştuğu bar serileri (bağlı ya da belge içi; sembolü olan). Yoksa motor
  yok: R3 öncesi davranış.
- `engine` (RP4, isteğe bağlı): motorun **kapsül sabit kaydı** (`docs/CAPSULE.md` §9.9):
  `"r3"` ya da `"rp4"`. Kayıt sonuç kuralını ve dedektör setini birlikte seçer
  (`qp_capsule::EngineConfig::named`, `qp_facts::detectors_named`). **Yoksa `r3`**
  (protokol varsayılanı; alan yazılmaz, var olan dökümler bayt eşit). Bilinmeyen ad
  oturum başlarken reddedilir (`init.capsules.engine: unknown record …`). `rp4`'te
  hikâyenin `resolution.rule`'u `rp4`, bulgunun `invalidation` metni `rp4` kuralını anar;
  `breakout.range` ve `flow.volume_surge` olgularının `detector_version`'ı 2'dir (karne
  anahtarı ayrı). Herkese açık motor (K1) aynı kayıtla koşar. Yansı ve görüntüleyici,
  kendi verisiyle koştuğu dedektörleri hikâyelerin `resolution.rule`'undan seçer.
  `qp sim --rule rp4`, `qp demo --rule rp4` bu alanı yazar (`r3` için yazmaz).
- Bir bar **kesinleşince** (sonraki bar geldi ya da pencere kapalı; RP2 olgunluğu)
  dedektörler (`qp_facts::default_detectors`, akışkan) ve motor (`EngineConfig::r3`; ya da
  `engine` kaydınınkiler) o barı
  işler. Bakış başına (bir yazı, bir veri olayı, yükleme) ilerleyen her seri için **tek motor
  girdisi** (`qp`) yazılır, değişim olmasa da: günlüğün ritmi bar kapanışıdır, lisanslı bir
  hikâyenin varlığı değil. Özet `capsules <seri> bar <i>` ya da `… bars a..=b`.
- Girdinin değişimleri sırayla: geçişe yol açan olgular (`put_fact`; `evidence_ref`
  `add_fact`'teki gibi damgalı, §18.5), hikâye başına bulgu (`put_finding`: iddia = TR cümle,
  `invalidation` metin koşulu — sonuç kuralı karar verir, belge koşulu değil) ve kapsül
  (`put_capsule`: `story`, durum), kapananların karne satırları (`push_ledger`: `capsule`,
  `key`, `stage`, `bar`).
- Durum eşlemesi: `born … confirmed` → `live`; `realized` → `live` (kapanış karnede);
  `invalidated` → `invalidated`; `expired` → `expired`; `retracted` → `invalidated`, karne
  satırı yok. Kişinin kararı (`approve_finding` / `reject_finding`; kapsül onay/ret için tek
  yol bunlardır) korunur: motor `approved`/`rejected`'ı ezmez.
- Doküman ve günlük bütün kapsül geçmişini taşır: başlangıç + değişimler aynı dokümanı
  (aynı hikâyeleri) bayt bayt verir; aynı girdiler aynı günlüğü verir (artımlı = baştan).
- **Lisanslı akış** (`flow`): test barındırıcısında `qp_source::synthetic_flow(barlar, seed)`.
  Akış değerleri yalnız otoritede okunur (sunucu olgusu, K33) ve seçilen güven yolu
  **mühür**'ü (`qp_capsule::trust::Muhur`, M6) besler. Akış verisinin yetkisi
  `feed/SYMBOL` (varsayılan `flow/THYAO`) biçiminde, §18.3 kuralıyla denetlenir.
- Kapsül cümlesi **okuyucu için** yazılır: gerçekleştirici (M2, `Template`) dizinin son
  kesin barı itibarıyla (K26), bağlantının dili (`hello.surface.lang`: `tr` varsayılan,
  `en`) ve okuyucunun yetkisiyle (aşağıda). Saklanan `headline` yetkili TR görünümüdür;
  `present`, kapsül kartı (`capsule_view`) ve `capsules`/`capsule` cevapları okuyucunun
  cümlesini kullanır (motor yokken ve dil TR iken saklanan başlık; R3 c1–c5 fikstürleri
  değişmedi). İşaret tarifi motorun `glyph_spec`'idir.

### 20.2 Sorgular

| `kind` | Cevap |
|--------|-------|
| `capsules` | `capsules`: `CapsuleItem` listesi — kimlik, bulgu, enstrüman, seri, tür, yön, zaman türü, aşama, durum, kademe, önem, teyit, güven, doğum/son bar, son geçiş zamanı, `ledger_at_birth`, `sentence {text, tr, en}`, (R4) `validity` (`valid` / `no_longer` / `closed`, cümleyle aynı bar itibarıyla; K42/A28), `relevant` (pozisyon/niyet; anlamı §20.4), `redacted`. Sıra: önem azalan, sonra son geçiş zamanı azalan, sonra kimlik. Süzgeçler VE'lenir (aşağıdaki tablo); `audience: true` soranın kitlesini uygular (§20.4, hikâyenin şimdiki aşamasıyla). Cevabın ayrıca `comments` alanı vardır: soranın akış yetkisi olmayan enstrümanlarda sunucu yorumları. |
| `capsule` | `capsule_detail`: `item`, `story` (bütün geçişler: bar, zaman, aşama, neden, olgular; R4: `condition` = durum koşulu, `resolution` = beklenen çözüm `{rule, condition?}`), `facts` (okuyucuya göre), `ledger` (anahtarın şimdiki karnesi: son kesin bardan önce kapananlar), `glyph` (R4: `validity` dahil, öğeninkiyle aynı), `evidence` — olgu başına `{fact, family, license, open, values?, evidence_ref?, requires?, rests_on}`: yetkiliye değerler ve dondurulmuş kanıt; yetkisize "şu yetkiyi gerektirir" (`requires`, ör. `bist/THYAO`) ve neye dayandığı (`rests_on`, her zaman) — K34. (R4) `capsule` alanı kapsül kimliği, bulgu kimliği ya da **kapsül seçicisi** alır (§17.1; `result.resolved`'da `/query/capsule`). Görünmeyen ya da olmayan kapsül: `not_found` (ikisi aynı). |
| `ledger` | `ledger`: `rows` (anahtar ya da `by_kind` ile tür başına `n`, `realized`; R4: satırda `direction`), `total`. Anahtar `<tür>/<yön>/<dedektör>@<sürüm>` (K42/A23). Süzgeçler: `key`, `meaning`, `detector`, (R4) `direction`; `by_kind` yönleri toplar (`direction` verilmişse yalnız o yön, satır `direction`'ı taşır). Geri çekilenler sayılmaz. Lisanstan bağımsız (K33): her lisanslı akışa yetkili okuyucuya motorun karnesi, açık görünüm okuyucusuna (§20.5) açık-yalnız motorun karnesi — lisanslı veriden hiç etkilenmez. |

(R5/m17-int) Bağlantı bir kapsül dili seçtiyse (`style` `voice`, §4.6.2) `capsules`,
`capsule` ve `capsules` konusu her `CapsuleItem`'a isteğe bağlı `voice` ekler: `CapsuleVoice`
`{voice, headline, detail, ledger, glossary?, face}` — `glossary` `GlossEntry {term, text}`
listesi, `face` sahnesi olmayan okurun metin yüzü (başlık, karne, ayrıntı; satır sonlarıyla).
Okurun dilinde ve aynı izleyici kuralıyla (yetkisize rakamsız, K33). Varsayılan `r4`'te alan
yoktur.

`capsules` süzgeçleri (hepsi isteğe bağlı, VE'lenir):

| Alan | Anlamı |
|------|--------|
| `instrument` | Yalnız bu enstrüman. |
| `stages` | Yalnız bu aşamalar (`born`, `strengthened`, `weakened`, `confirmed`, `realized`, `invalidated`, `expired`, `retracted`). |
| `open` | `true` yalnız açık, `false` yalnız kapanmış hikâyeler. |
| `meaning` | Yalnız bu anlam türü (`continuation`, `reversal`, `breakout`, `squeeze`, `extreme`, `flow`). |
| `since_bar` / `since_t` | Son geçişi bu bardan / zamandan (ms) itibaren olanlar (geçişi süzer: eski doğumlu ama yeni hareket eden hikâye de gelir). |
| `born_since_bar` / `born_since_t` (R4) | Bu bardan / zamandan itibaren **doğanlar** (doğum barı / doğum geçişinin zamanı). |
| `latest` (R4) | Yalnız **en son doğan** eşleşen kapsül: en büyük doğum zamanı, sonra doğum barı, sonra kimlik. Öteki süzgeçlerden sonra, `limit`'ten önce uygulanır. Kapsül **seçicisinin** `latest`'i başkadır: en son **hareket edeni** seçer (§17.1). |
| `audience` | Soranın kitlesi (§20.4). |
| `limit` | En çok bu kadar. (R4, K42-FEED) `audience: true` iken sınır, soranın pozisyon/niyet kapsülünün **şimdiki** olumsuz geçişini (`relevant`, aşama `weakened`/`invalidated`/`retracted`, son geçişi dizinin son kesin barında; günlük dizide bu seans) kesmez: bunlar önce alınır, kalan yer sıraya göre dolar; cevap yine sıralı ve en çok `limit`. |

```json
→ {"type":"query","id":"q","query":{"kind":"capsule","capsule":{"instrument":"THYAO","meaning":"breakout","direction":"up","latest":true}}}
← {"type":"result","id":"q","data":{"kind":"capsule_detail","detail":{"item":{"id":"cap.THYAO.breakout.up.147","validity":"closed",…},…}},
   "resolved":{"/query/capsule":"cap.THYAO.breakout.up.147"}}
→ {"type":"query","id":"r","query":{"kind":"capsules","latest":true}}
→ {"type":"query","id":"s","query":{"kind":"capsules","born_since_bar":145,"meaning":"breakout"}}
→ {"type":"query","id":"l","query":{"kind":"ledger","meaning":"breakout","direction":"down"}}
```

- **Seçicide gizli kalan** (R4/S1, `capsule_detail.withheld`): `capsule` bir seçiciyse ve
  okuyucunun enstrümanın lisanslı akışına yetkisi yoksa, seçici okuyucunun görebildikleri
  arasında çözülür (§20.5); okuyucuya `capsules` sorgusunda zaten verilen sunucu yorumlarından
  çözülen kapsülün son geçişinden **daha yeni** olanlar varsa cevap bunları ve gerektirdikleri
  yetkiyi taşır: `"withheld":{"requires":["flow/*"],"comments":[…]}`. Seçicinin `meaning`'i
  `flow` dışındaysa, kimlikle sorulduysa ya da okuyucu yetkiliyse alan yoktur. Yeni bir varlık
  kâhini değildir: söylenen yalnız yorumların okuyucuya zaten söylediğidir.

Dökümler: 45 (karne yönü), 46 (olumsuz geçiş), 47 (geçerlilik), 48 (kapsül seçicisi ve
`capsules` süzgeçleri), (R4/S1) 57 (seçicide `withheld`).

### 20.3 İşlemler: kitle

| `op` | Alanlar | Değişim |
|------|---------|---------|
| `set_audience` | `principal?`, `audience {group, sensitivity?, watchlist?, positions?}` | `put_audience` |
| `set_sensitivity` | `principal?`, `sensitivity` (`important`/`balanced`/`all`; `null` = grubun varsayılanı) | `put_audience` |

**`Audience` biçimi** (`qp-schema::capsule::Audience`; bilinmeyen alan `schema`):

| Alan | Tip | Anlamı |
|------|-----|--------|
| `group` | `"habituated"` \| `"future"` (zorunlu) | Kullanıcı grubu. Varsayılan hassasiyeti: `habituated` (alışkın kullanıcı) → `all`; `future` (geleceğin yatırımcısı) → `important`. |
| `sensitivity` | `"important"` \| `"balanced"` \| `"all"` (isteğe bağlı) | Seçilen seviye; yoksa grubun varsayılanı. Sistem hiç değiştirmez (K31). |
| `watchlist` | sembol dizisi (isteğe bağlı) | İzleme listesi; boşsa her enstrüman. |
| `positions` | `{sembol: miktar}` (isteğe bağlı) | Pozisyonlar; işaret uzun/kısa, `0` pozisyon sayılmaz. |

```json
{"type":"command","id":"c1","ops":[{"op":"set_audience","audience":
  {"group":"future","sensitivity":"balanced","watchlist":["THYAO","ASELS"],"positions":{"THYAO":100,"ASELS":-50}}}]}
{"type":"command","id":"c2","ops":[{"op":"set_sensitivity","sensitivity":null}]}
```

- `set_audience` kişinin tercihini **tamamen değiştirir** (verilmeyen alan boşalır;
  izleme listesine ekleme/çıkarma için bütün listeyi yeniden yazın). `set_sensitivity` yalnız
  seviyeyi değiştirir (`null`: grubun varsayılanı); kişinin henüz tercihi yoksa grup
  `habituated` olur.
- `principal` yoksa soranın kendisi. Kişi kendi tercihini `document.audiences[<kendisi>]`'nden
  okur (yalnız kendi girdisi gelir; aşağıdaki gizlilik kuralı).

Kişi kendi tercihini yazar (`read` yetkisi yeter); başkasınınkini yalnız `owner`
(`forbidden`). **Öneri olamazlar** (kişisel veri öneriyle herkese gitmesin). Sistem bunları
hiç yazmaz (K31; test). **Gizlilik:** bir kişinin tercihleri (pozisyonlar, izleme listesi,
seviye) başka hiçbir okuyucuya gitmez: `document`, `change`, `log` (değişim, ters değişim,
istek ve özet), `welcome` özeti dahil; yalnız kendisi ve barındırıcı (`host`, depoyu zaten
tutar) görür. `owner` başkasınınkini yazabilir ama okuyamaz.

### 20.4 `capsules` konusu

`{"kind":"capsules","instrument?":…,"all?":false}`. Her motor girdisinden sonra abonenin
görebildiği hikâye geçişleri: `{"kind":"capsule","capsule":CapsuleItem,"transition":…,"seq"}`.
`all` yoksa kitle süzgeci: izleme listesi (boşsa her enstrüman; pozisyon ve etkin niyetin
enstrümanı her zaman), sonra hassasiyet kademesi (`important` 1, `balanced` 1–2, `all`
hepsi). (R4, K42/A22) Pozisyon ya da etkin niyetle ilgili kapsülün (`relevant: true`)
yalnız **olumsuz geçişleri** — `weakened`, `invalidated`, `retracted` — kademeden bağımsız
geçer; doğumu ve öteki geçişleri herkes gibi kademeye tabidir. Konu her geçişi **kendi
aşamasıyla** değerlendirir (bildirilen `transition.stage`); `capsules` sorgusunun
`audience`'ı hikâyenin şimdiki aşamasıyla (CAPSULE §9.8). `relevant` alanı yalnız
"pozisyon ya da etkin niyetle ilgili" demektir; tek başına seviyeyi aşmaz. Kişi tercih
yazmadıysa grup `habituated` (`all`). Döküm 46: `important` seviyesinde pozisyonu olan
okuyucuya kademe 3 doğum gitmez, aynı hikâyenin zayıflaması gider. Akış yetkisi olmayan aboneye lisanslı
hikâyeler yerine mühürün iletileri: `{"kind":"server_comment","comment":{instrument,
series, bar, t, codes, tr, en, trust:"server", attachments}}` (rakam yok; ekler: gereken
yetki, kanıtın dayandığı, kural, makbuz).

### 20.5 Okuyucuya göre izdüşüm (K33, K34, K36, M6 bütünleştirme kuralı)

Otorite tek doküman ve tek günlük tutar (yeniden oynatma bunlardan); **gönderdiğini**
okuyucuya göre kurar (yalnız R3 öğeleri: `meaning`'li olgular, `story`'li kapsüller; R3
öncesi öğeler değişmez):

1. **Açık görünüm (R3/K1, girişimsizlik yapı gereği).** Motor lisanslı akış okuyorsa
   otorite ikinci bir motor, **açık-yalnız motor** koşar: aynı `EngineConfig::r3()`, aynı
   kesin barlar, yalnız açık lisans sınıfındaki olgular; ve ikinci bir doküman ile
   günlük, **açık dünya**, tutar. Açık dünyanın **kendi sıra numaraları** vardır (R3/K1b):
   - her motor bakışı (bar kapanışında, seri başına) ve barları düzelten her kesin veri
     cevabı (özet `settled <seri>` ya da `retract …`) → açık-yalnız motorun kendi
     değişimleri; çapa damgası ve bar yazıları (seri verisi) → aynen;
   - **her kişinin komutu** (doğrudan, öneri, kabul, geri alma) açık dünyada **kendi
     başına** genişletilir (açık `seq` ve kimliklerle): orada uyuyorsa bir girdi, uymuyorsa
     hiçbir şey — otoritenin dünyasında ne olduğundan bağımsız. Yetkili kişinin
     `undo { seq }`'su, `accept_proposal`'ı ve `reject_proposal`'ı otoritenin girdilerini
     ya da önerilerini adlandırır: açık dünyada bu komutun kendi girdilerinin yaptığı açık
     girdiye/öneriye çevrilir (yoksa hiçbir şey); `seq`'siz `undo` açık dünyada, kişinin
     açık girdileri arasında çözülür. Kişi, okuduğu dünyanın
     cevabını alır (hata, `base` çakışması, `seq`, kimlikler);
   - açık dünyanın kendi kapsüllerinin (kişilerin, hikâyesiz) §8.1 geçersizleşmesi orada
     değerlendirilir ve orada yazılır.
   Otoritenin açık dünyada karşılığı olmayan girdisi (ör. yetkili kişinin yalnız lisanslı
   bir bulguyu onaylaması, yalnız otoritede olan bir kapsülün geçersizleşmesi) açık
   dünyada **hiç iz bırakmaz**: yer tutucu yok, sıra numarası kayması yok. **Açık görünüm
   okuyucusunun** komutu otoritenin dünyasına yalnız açık değişimlerin geri çevirisiyle,
   **hep ya da hiç**, ulaşır: her değişimin adlandırdığı kimlik ya iki dünyada da yok
   (yeni) ya da iki dünyada **bayt eşit** bir öğe (kanonik JSON) — hiçbir motor öğesi, hiçbir
   öneri —, değişimler birlikte uygulanmalı; yoksa otorite hiçbir şey almaz (okuyucu bunu
   göremez). Açık görünümlü kişinin önerileri ve öneri kararları yalnız açık dünyadadır. **Lisanslı akışların hepsine yetkili olmayan** okuyucu (kısmi yetki dahil:
   sıralayıcının bağlamı bütün enstrümanları kapsadığından, yetkisi olmayan bir akıştan
   arınmış hikâye ancak açık dünyada vardır) **açık görünüm okuyucusudur**: bütün okuma
   yolları (sorgular — `capsules`, `capsule`, `ledger`, `log`, `document`, `list`,
   `describe`, `present`, `verify_evidence` —, abonelik, `hello` özeti, girdi, dikkat,
   seçici çözümü, hata önerileri), `change`, `finding_state` ve `proposal` olayları ve
   `capsules` konusunun geçişleri açık dünyanın dokümanı, günlüğü, okuma dokümanı ve
   karnesiyle çalışır; aldığı her `seq` (`welcome`, `result`, `ack`, olaylar) açık dünyanın
   numarasıdır; itirazı itirazcının dünyasında yönlendirilir ve açık-yalnız motorda karara
   bağlanır; `claims` konusunda yalnız kendi koşul iddialarını alır (belge öğesi iddialarına
   itirazı `unknown`); `intents` konusunda açık dünyanın niyetleri açık dünyada
   değerlendirilir (kendi etkinliği ve kenar durumuyla). Böylece bulgunun `confidence`'ı,
   kapsülün cümlesi, hikâyesi, `importance`/`tier`'ı ve sırası, `ledger_at_birth`, karne
   sayıları, girdilerin ve olayların varlığı ve numaraları **yalnız açık veriye ve
   kişilerin komutlarına** bağlıdır — yetkili kişinin aynı komutları dahil. Bu okuyucuya
   lisanslı veriden etkilenen tek kanal mühür iletileridir (`trust: server`; `capsules`
   sorgusunun `comments`'i, `server_comment` olayı). Çalışan motorun kapsülü (izlenen
   serideki hikâye) §8.1 veri koşuluyla geçersizleşmez (yalnız sonuç kuralıyla kapanır).
   Test: aynı açık barlar, farklı lisanslı akış tohumları (ve akışsız koşu) → mühür
   iletileri dışında bayt eşit (`crates/qp-session/tests/non_interference.rs`; denetçinin
   `rounds/R3/audit/src/bin/k1_probe.rs`).
2. Açık görünüm okuyucusu olmayan (bütün lisanslı akışlara yetkili) okuyucu otoritenin
   dünyasını alır; lisanslı olgu yalnız akışın yetkisi olmayana gizlenir (aşağıdaki kurallar
   artık yalnız bu dünyada, örn. akışa yetkili ama bar beslemesine yetkisiz okuyucu için
   geçerlidir): doğuran olgusu lisanslı hikâye hiç gitmez; lisanslı olgu açık hikâyeye
   katıldıysa o olgu ve geçişi gizlenir.
3. Hikâyenin bar beslemesine yetkisi olmayana olgular kanıt değeri ve `evidence_ref`
   (veri özeti) olmadan, cümleler rakamsız (`redacted`) gider (her iki dünyada).

Uygulama: sorgu, abonelik, girdi, dikkat ve seçici çözümü okuyucunun dokümanında çalışır
(hatalar ve öneriler de); komut önce okuyucunun dokümanında kuru koşulur (görmediği öğeye
dokunan komut, olmayan öğedeki hatayı alır). `show_finding` lisanslı olgunun kanıt değerini
görünüme sabit olarak yazmaz (yalnız köken). Yetkisiz okuyucuya giden her mesajda lisanslı
değer, kimlik ve özet olmadığı testle taranır (`crates/qp-session/tests/capsules.rs`).

**Sınırlar.** Otoritenin günlüğü ve `seq`'i değişmedi (yeniden oynatma onlardan); açık
görünüm okuyucusu yalnız açık dünyanınkini görür. Kişilerin komutları açık dünyaya da
geçer (orada kendi başına uygulanarak): girişimsizlik lisanslı **veri** içindir, kişinin
seçimi için değil; aynı komutlar her yapılandırmada aynı açık sonucu verir. Yetkili kişinin
yarattığı öğelerin kimlikleri (`fnd-<seq>-<n>`) iki dünyada farklı olabilir (her dünya
kendi `seq`'iyle). Açık dünyada bir
kapsül, otoritenin dünyasındaki lisanslı doğumlu bir hikâyenin kimliğini taşıyabilir
(kimlik = enstrüman, tür, yön, doğum barı): ikisi ayrı öğedir.
Yansı aynalanan dokümanı izdüşümle alır (akış yetkisi olmayan yansının dokümanı otoritenin
dokümanından farklıdır; RP2 yansı özetleri bu durumda karşılaştırılamaz).

### 20.6 Aday

Aday sunucudan gelmez (K28, A16): yansı açık barı **kendi verisinden** hesaplar
(`Replica::candidates`, `qp_session::capsule::candidates`: açık dedektörler +
`Engine::candidates`). Otorite açık barda hiçbir şey söylemez; bar kapanınca aday ya
kesinleşir (olgu yeniden çıkar) ya da kaybolur — günlükte ve karnede izi kalmaz (döküm 42).

### 20.7 İtiraz ve geri çekme (§19.9 ile)

`dispute` `claim: {"kind":"capsule", capsule, finding}` motorun bir kapsülünü gösterirse
iddia "hikâyenin olguları kesin barlarda da çıkar"dır. Otorite çapadan `t`'ye kesin bar ister;
cevap barı düzeltirse dedektörler düzeltilmiş barlarda baştan koşulur: olgusu artık çıkmayan
(ya da değerleri değişen) her hikâye **geri çekilir** (`retracted`, neden `dispute_upheld` —
itiraz edilmişse — ya da `data_corrected`; karneye yazılmaz, kapanmış hikâyenin satırı
sayılmaz). Dedektörler düzeltilmiş veriden devam eder; düzeltilmiş verinin yeni doğurduğu
olgu ileri sürülmez (geçmiş yeniden yazılmaz). Karar: geri çekildiyse `upheld`, değilse
`rejected` (kanıt: seri, `t`, kesin, çapa, beslemeler). Aynı itiraz `duplicate`; görünmeyen
kapsül ya da bar yetkisi yok: `unknown`.


> **Mühür anahtarları (R3, güvenlik incelemesi):** makbuz ve denetim günlüğü anahtarları
> barındırıcının gizli anahtarlarıdır; barındırıcı oturumu kurarken `Session::new_with_trust_keys` / `Session::with_provider_and_trust_keys`
> ile verir (ilk sunucu yorumundan önce; sonradan `set_trust_keys` reddedilir) (belgeye ve günlüğe girmez). Verilmezse otorite herkese açık **demo** anahtarlarını
> kullanır; bu yalnız sentetik test beslemesi (`init.capsules.flow`) ve dökümler içindir.

## 21. Yeniden oynatma (R4)

Kavramlar `docs/CONCEPTS.md` "Gösterim (R4)" (yeniden oynatma, oynatma saati, oynatma
konumu, oynatma denetimi); karar K44. Kaynak `crates/qp-source/src/replay.rs`
(`Replay`), barındırıcı `crates/qp-session/src/replayhost.rs` (`ReplayHost`), şema
`qp-schema::protocol` (`ClientMsg::Replay`, `ReplayControl`, `ReplayState`,
`QueryResult::Replay`, `SessionInit.replay` / `ReplayInit`). Tasarım ve ölçüm
`rounds/R4/experiments/T2.md`.

### 21.1 Kaynak

Geçmiş barlar (yerel gerçek veri `fixtures/local/bars`, K23; yoksa sentetik pazar) canlı
akış gibi verilir. Her beslemenin kaba barları (ör. `1d`) ve isteğe bağlı **ince** barları
(ör. `1h`) vardır. Başlangıçtan önceki barlar ilk çözümdür (barındırıcının sağlayıcısı
tutar, `seq` 0); sonraki her bar §18.4 veri olaylarıyla gelir:

- **ince** (`fine`): barın açılışında `bar` olayı `{o, h: o, l: o, c: o, v: 0}`; her ince
  barın sonunda birikimli durumun değişen alanlarıyla `patch` (son barın aralığına
  kıstırılmış, `h` hiç düşmez, `l` hiç yükselmez, `v` hiç azalmaz); son ince adım barı
  **tam** son haline getirir;
- **ohlc** (ince veri yok; belirleyici ve **etiketli** bar içi yol): açılış → ilk uç
  (yükselen barda dip, düşende tepe) → ikinci uç → kapanış, XIST seansına yayılmış;
  son adım bar;
- **bar** (`path: false`): tek `bar` olayıyla bütün bar.

Hangi günün hangi yolla kurulduğu durumda sayılır (`days {fine, ohlc, bar}`); sayfa ince
veri olmayan günü etiketleyebilir.

**Adım:** tek bir sanal zaman. Bir adım o zamandaki bütün beslemelerin olaylarını taşır;
eşitlikte sıra besleme anahtarıdır (sembol, periyot, besleme). Adım sınırları hızdan
bağımsızdır. **Oynatma konumu** (`ReplayCursor`): sanal zaman, atılan adım sayısı,
besleme başına verilen olay sayısı. Olaylar konumun fonksiyonudur.

Sentetik pazar: `ReplayInit.fine = "1h"` ile `qp_source::synthetic_market_hourly` (günlük
barlar saatlik barlardan toplanır; saatlikler ince barlardır), yoksa `synthetic_market`;
semboller `SYN01…`. Yerel veri: `<bars>/../symbols.txt` sırasıyla (yoksa alfabetik) ilk
`instruments` sembol; ince periyot dosyası (`1hour`, `5min`) yoksa o günler `ohlc`.
Yerel veri istenip bulunamazsa sentetik pazar kullanılır ve durum bunu söyler
(`synthetic: true`, `note`). Gerçek veri makineden çıkmaz; dökümler yalnız sentetiktir.

### 21.2 Barındırıcı

`SessionInit.replay` (`ReplayInit`, barındırıcı yapılandırması; protokol mesajı değil):
`source` (`synthetic` | `local`), `instruments` (30), `days` (260), `seed` (1), `interval`
(`1d`), `fine`, `history` (120 bar) ya da `from_t`, `to_t`, `path` (true), `feed`, `speed`
(4), `running` (false: duraklatılmış başlar), `entitlements` (boşsa her kişiye `*`),
`bind_all` (her beslemeye bağlı seri, kimlik sembolün küçük harfi, pencere ilk çözüm),
`capsules_all` (kapsül motoru her birinde, §20.1). `init.data` ile birlikte verilemez:
barındırıcı `init.data`'yı yeniden oynatmanın ilk barlarından kurar. Otoritenin ilk saati
ilk çözümün son barının zamanıdır (`init.clock` yerine).

Her adımda barındırıcı otoriteye önce **saati** (`Session::host_clock`, bir `clock`
mesajının aynısı; otoritenin saati yoktur, §11), sonra adımın veri olaylarını sırayla
(`Session::host_data_event`) verir; ortaya çıkan bildirimler bugünkü kurallarla yayılır
(§15 çıktı sırası). Duvar saati yalnız **ne zaman** adım atılacağını belirler
(`ReplayHost::pump`): çalışırken hız × geçen süre kadar adım; içerik sanal zamanın
fonksiyonudur. Aynı veri + aynı denetimler aynı konumlarda (`ReplayHost::controls`:
`(adım, denetim)` kaydı) → bayt düzeyinde aynı mesaj akışı.

### 21.3 `replay` mesajı

```json
→ {"type":"replay","id":"r1","control":{"kind":"step","n":12}}
← {"type":"result","id":"r1","seq":42,"now":1678802400000,"data":{"kind":"replay","replay":{
    "t":1678802400000,"step":13,"steps":180,"events":26,"from_t":1678690800000,"to_t":1680879600000,
    "paused":true,"done":false,"speed":8.0,"synthetic":true,"source":"synthetic",
    "instruments":2,"interval":"1d","fine":"1h","days":{"fine":40,"ohlc":0,"bar":0},"epoch":0}}}
← … adımların yol açtığı bildirimler (abonelere)
```

| `control.kind` | Alanlar | Ne yapar |
|----------------|---------|----------|
| `status` | — | Durum (değiştirmez). |
| `pause` | — | Duvar saatiyle ilerlemeyi durdurur (`step`, `seek` çalışır). |
| `resume` | — | Duvar saatiyle, şimdiki hızla ilerler. |
| `speed` | `steps_per_s` (`0 < x ≤ 1000`) | Hız: duvar saniyesi başına adım. |
| `step` | `n` (1; en çok 100000) | Şimdi `n` adım. |
| `seek` | tam olarak biri: `t` (o ana kadarki her adım), `date` (`YYYY-AA-GG`: borsa saat diliminde o günün başı; o günden hiçbir şey alınmamış), `step` (o kadar adım) | İleri: aradaki adımlar atılır, bildirimleri `result`'tan sonra gelir. **Geri:** otorite başlangıcından yeniden kurulur ve konuma sessizce ilerletilir — yeni otorite (`epoch + 1`, §8.4): bütün bağlantılar kapanır, istemciler yeniden `hello` der; durumda `reset: true`. Kullanıcı yazıları (ör. `set_audience`) yeni otoritede yoktur. |

Cevap her zaman `result` (`kind: "replay"`, `ReplayState`): `t` (sanal zaman = otoritenin
saati), `step`/`steps`, `events`, `from_t`/`to_t`, `paused`, `done`, `speed`,
`synthetic`, `source`, `note?`, `instruments`, `interval`, `fine?`, `days`, `epoch`,
`reset?`. Mesaj dokümana yazmaz, günlüğe girmez; adımların yol açtığı motor girdileri
(kapsül motoru, geçersizleşmeler) bugünkü gibi günlüktedir.

**Yetki.** `status`: `read` izni olan her rol. Zaman çizelgesini değiştiren denetimler
(`pause`, `resume`, `speed`, `step`, `seek`): yalnız `host` ve `owner` rolleri — zaman
çizelgesi bütün okuyucuların ortak dünyasıdır; izleyici (`viewer`), öneren (`proposer`,
ajan), `editor`, `feed` için `forbidden`. Gösterimde tek kullanıcı `owner`dır; aynı oturumu
süren ajan aynı kimlikle bağlanır.

**Hatalar.** Kaynağı olmayan otorite: `unsupported`. Geçersiz alan: `invalid` (`path`:
`/control/steps_per_s`, `/control/n`, `/control/date`, `/control`). Bilinmeyen denetim ya
da tür hatası: otoritenin `schema` hatası. `hello`'dan önce: `handshake`.

**Taşımalar.** Süreç içi `ReplayHost::handle_raw`; `qp serve --init` (`init.replay` varsa
barındırıcı; çalışırken adımlar duvar saatiyle atılır ve bildirimleri girdi satırından
bağımsız yazılır); MCP: `replay` ve şemadan üretilen `replay_status`, `replay_pause`,
`replay_resume`, `replay_speed`, `replay_step`, `replay_seek` araçları. CLI:
`qp replay` (yerel ya da sentetik, periyot, ince periyot, başlangıç/bitiş tarihi, adım
sayısı, kayıtlı denetimler; satır akışı; iki koşu bayt eşit).

### 21.4 Gösterim barındırıcısı `qp demo` (R4/T5)

Yerel gösterimin taşıması (`crates/qp-cli/src/demo.rs`; kullanım `shells/web/README.md`).
Sayfa, protokol ve veri düzlemi **aynı kökendedir**; sayfa başka hiçbir yere istek yapmaz
(CSP `default-src 'self'`; test `shells/web/tests/parity.mjs`).

- `GET /events?principal=…&lang=…` — sunucu olay akışı (SSE). Bir akış bir protokol
  bağlantısıdır. Çerçeveler (her `data:` satırı bir JSON nesnesi):
  - `{"start":{conn, token, principal, lang, data, events, info}}` — ilk çerçeve: bağlantı
    kimliği, bağlantının gizli değeri, yansının verisi (`data`: barındırıcının ilk barları ve
    okuyucunun yetkisi, `InitData`; `events`: o ana kadar uygulanan veri olayları) ve sayfanın
    notları (`info`: adresler, `synthetic`, `note`, `lan`). Geri `seek`'ten sonra (yeni
    otorite, `epoch + 1`) her bağlantıya yeniden gelir; sayfa yansısını baştan kurar ve
    yeniden `hello` der.
  - `{"msg":ServerMsg}` — bu bağlantının protokol mesajları.
  - `{"data":[DataEvent…],"replay":ReplayState}` — **veri düzlemi** (§18.1): adımların veri
    olayları, yansının kendi beslemesi. Protokol mesajı değildir, günlüğe girmez; ayrı
    çerçevedir. Oynatma durumu barındırıcının bilgisidir (sayfanın saati).
- `POST /send?conn=…` — bağlantının protokol mesajları (JSON dizi, sırayla). Zorunlu:
  `X-Qp-Token` (yalnız o bağlantının `start` çerçevesinde verilen 128 bitlik gizli değer;
  işletim sistemi rastgeleliği — barındırıcı kodudur, otoriteye, dokümana ya da özetlere hiç
  girmez, belirleyiciliği etkilemez), `Content-Type: application/json`, aynı kökenden
  `Origin`. Eksik ya da yanlışsa `403`/`415`, mesaj otoriteye gitmez. Cevaplar akıştan gelir.
- Her istekte `Host` bu barındırıcı olmalıdır (`localhost`, `127.0.0.1`, `[::1]`, dinlenen
  adres ya da basılan yerel ağ adresi, portuyla); değilse `403` (DNS yeniden bağlama).
  Varsayılan dinleme `127.0.0.1`; `--lan` `0.0.0.0`'ı dinler (aynı ağdaki telefon; yerel
  ağdaki cihazlar gösterimi görebilir). Test `crates/qp-cli/tests/demo_security.rs`. (RP3) `--https-host AD` ile, TLS'i sonlandırıp `127.0.0.1:PORT`'a `Host`'u koruyarak ileten bir HTTPS vekilinin adı (`AD` ya da `AD:443`) `Host` olarak, `https://AD` de `/send`'in `Origin`'i olarak kabul edilir; başka hiçbir ad ya da köken eklenmez. Varsayılan kapalı. Gerekçe: iOS 27'de JIT ve WebGPU güvenli bağlam (`https://`) ister (`rounds/RP3/CALIBRATION.md`); vekile erişebilen herkes gösterimi görebilir.
- Gösterim oturumu: `qp_wasm::shell::demo_session_init` (`demo_init` + veri yetkisi olmayan
  `visitor` sahibi — ayarlardaki "veri yetkisi yok" kipi; yalnız gösterim için).
- Kabuksuz sürüş: `qp drive --script <adımlar>` aynı oturumu yalnız protokol mesajlarıyla
  sürer ve kabuğun bildirdiği özetleri (doküman, her yüzün ve açık sahnenin `view_digest`'i,
  bakış durumu) basar; tarayıcıdaki kabuğun özetleriyle eşitliği `scripts/verify.sh` `demo`
  adımında denetlenir.


## 22. Görüntüleyici (RP3/M15, K48)

> Karar K48: **cihaz görüntüleyicidir, hesap yeri değildir.** Tespit, bulgu, sıralama ve
> karne otoritededir; akıştaki kapsüller cihaza hazır gelir. Cihaz yalnız kullanıcının o an
> baktığı şey için veri alır, hesaplar ve çizer: açık kapsülün enstrümanı (grafik,
> göstergeler, aday, işaretin canlı hareketi). Cihazın tik başına işi ve belleği evrendeki
> sembol sayısından bağımsızdır. Referans: `crates/qp-session/src/viewer.rs` (otorite ve
> cihaz tarafı), `crates/qp-wasm/src/viewer.rs` (kabuk çekirdeği), `crates/qp-cli/src/demo.rs`
> (barındırıcı). Ölçüm `rounds/RP3/experiments/M15.md`. Kavramlar `docs/CONCEPTS.md`
> "Görüntüleyici (RP3)". Bu bölüm §19'u daraltır: görüntü kuralı (§19.4) yalnız açık
> enstrüman için geçerlidir; akıştaki kapsül otoritenin kesinleşmiş iddiasıdır.

### 22.1 Görünürlük aboneliği — `viewer` konusu ve `visible` mesajı

```json
→ {"type":"subscribe","id":"viewer","topic":{"kind":"viewer","limit":40,"width":390}}
← {"type":"ack","id":"viewer","status":"subscribed"}
← {"type":"notify","sub":"viewer","event":{"kind":"viewer","viewer":{
     "feed":[{"capsule":CapsuleItem,"known_at":1688713200000,"face":true}, …],
     "data":[]},"seq":4501}}
→ {"type":"visible","id":"v1","sub":"viewer","visible":["cap-a","cap-b","cap-c"]}
← {"type":"result","id":"v1","seq":4501,"data":{"kind":"viewer","viewer":{
     "faces":[{"capsule":"cap-a","digest":"…","scene":SceneDesc,"trust":"server"}, …],
     "data":[]}}}
```

- `viewer` konusu (bağlantı başına bir tane; ikincisi `duplicate`): `limit` akış uzunluğu
  (varsayılan 40, 1–200), `width` yüz çerçevesinin CSS genişliği (varsayılan 390, 64–4096;
  yükseklik yüz tasarımınındır, `qp_grammar::faces::height`). `ack`'ten hemen sonra ilk
  `viewer` olayı gelir (akışın tamamı).
- `visible`: aboneliğin ekrandaki kapsülleri (en çok `limit`; öncekinin yerine geçer). Cevap
  `result` (`kind: "viewer"`), değişeni taşır. Varlık durumudur: dokümana ve günlüğe girmez.
  Okuma izni yeter. Bilinmeyen `sub`: `not_found` (`/sub`).
- **Açık kapsül** bağlantının bakış durumudur (`input` `open`/`close`, §4.6.1): ayrı mesaj
  yoktur. Derinlik `sentence` değilse `look.capsule` açıktır.
- MCP'de araç değildir (cihazın mesajıdır, `input` gibi doğrudan iletilir); `subscribe`
  aracı `viewer` konusunu şemadan alır.

### 22.2 `viewer` olayı (`ViewerUpdate`)

Otorite her viewer bağlantısı için cihaza ne gönderdiğini tutar ve her girdiden sonra
(mesaj; veri olayı — yeniden oynatma barındırıcısında **adım başına bir kez**,
`Session::set_viewer_batch`) yalnız **değişeni** iter:

| Alan | Ne zaman | İçerik |
|------|----------|--------|
| `feed` | akış değiştiğinde (ve ilk olayda) | `ViewerItem` dizisi: `capsule` (`capsules` sorgusunun `audience: true, limit` cevabındaki öğe, okuyucunun dünyasında §20.5), `known_at` (geçişin bilinebildiği an: geçiş barından sonraki barın açılışı; yoksa yok = şimdi), `face` (okuyucu serinin her beslemesine yetkili ve bar var: yüz sahnesi gelir; yoksa cihaz cümleyi metin olarak gösterir) |
| `faces` | görünür bir kapsülün yüzü yeni ya da özeti değiştiğinde | `ViewerFace`: `capsule`, `digest` (`view_digest`, §19.6), `scene` (`SceneDesc`: okuyucunun yüz tasarımı, bakış ayarı, dili, cümlesi ve başlık parçalarıyla, otoritenin verisi üzerinde derlenmiş — yansının derleyeceği sahnenin aynısı, `qp_session::viewer::face_scene`), `trust: "server"` |
| `open` | kapsül açıldığında ya da diliminin özeti değiştiğinde | `ViewerOpen` (§22.3) |
| `closed` | açık kapsül kapandığında | `true` |
| `audience` | okuyucunun kitle tercihi değiştiğinde | `Audience` |
| `data` | her olayda | cihazın **veri abonelikleri**: barındırıcının bu bağlantıya sunduğu beslemeler. Akış ekranında boş; kapsül açıkken yalnız açık enstrümanın beslemeleri (okuyucunun yetkili olduğu) |

Hiçbir şey değişmediyse olay yoktur. Hepsi isteğe bağlı alanlardır; `viewer` aboneliği
olmayan bağlantının akışı değişmez (var olan dökümler bayt bayt aynı).

### 22.3 Açık kapsülün dilimi (`ViewerOpen`)

`{capsule, finding, instrument, series, document, digest, feeds}`: cihazın açık kapsülü
**kendi verisinde** çizmesi ve sınaması için gereken doküman parçası
(`qp_session::viewer::slice`): dokümanın kendi alanları; enstrümanın serileri, üzerlerindeki
hesap serileri ve bunların girdileri; o serilerin olguları; bu olgulara dayanan bulgular ve
kapsülleri (enstrümanın hikâyeleri); yalnız bu serilere dayanan görünümler ve niyetler; bu
kapsül ve bulguların karne satırları; okuyucunun kendi kitle tercihi. Doküman sırası korunur;
bu yüzden dilim üzerinde derlenen sahne bütün doküman üzerinde derlenenle aynıdır (test:
`crates/qp-wasm/tests/viewer_m15.rs`, `shells/web/tests/parity.mjs`). `digest` dilimin
`document_digest`'i, `feeds` dilimin dayandığı bağlı beslemeler (yetkili olunanlar).
Dilim yalnız o enstrümanın kalemleri değiştiğinde yeniden gelir.

### 22.4 Cihaz tarafı: açılışta veri bağı ve doğrulama

Cihaz açık kapsül için bir yansı kurar (`qp_session::viewer::OpenSide`): dilimden başlar
(`Replica::start`), bağlı serilerini **kendi** verisinden, dokümandaki çapadan çözer (§18.3);
yalnız o enstrümanın veri olaylarını alır. Otorite açık enstrümanın beslemelerine dayanan
iddiaları (kapsül, niyet, bağlantının kendi koşulları; §19.3) `viewer` aboneliğine de
gönderir; yansı bunlara §19.4 görüntü kuralını uygular. Aday (§20.6) yalnız açık enstrümanda,
yerel veriden hesaplanır.

**Hikâyenin sınanması** (`check_story`): açık kapsülün hikâyesindeki her olgu (sunucu
olguları hariç, K33) cihazın kendi **kapalı** barlarında açık dedektörlerle aynı kimlik ve
aynı kanıt değerleriyle çıkmalıdır. Sonuç ve güven işareti:

| Durum | `TrustState` | İşaret metni (TR / EN) |
|-------|--------------|------------------------|
| Akıştaki kapsül; ya da açık kapsül cihazda sınanamıyor (yalnız sunucu olguları, veri yetkisi yok) | `server` | Sunucu doğruladı / Verified by the server |
| Kendi verisi olgunun barını henüz kapalı olarak kapsamıyor | `checking` | Bu cihazda sınanıyor / Checking on this device |
| Her olgu kendi verisinde çıktı | `device` | Bu cihazda sınandı / Checked on this device |
| Bir olgu çıkmadı ya da bu kapsül üzerindeki bir iddia `disputed` | `differs` | Kaynaklar farklı / Sources differ |

Metin çekirdekten gelir (`qp_session::viewer::trust_text`); kabuk olduğu gibi gösterir.
Protokoldeki karşılığı `TrustMark` (`server` | `device`).

### 22.5 Barındırıcının görevleri (veri düzlemi)

Veri protokolün dışındadır (K23, §18.1). Barındırıcı viewer bağlantısının veri düzlemini
`Session::viewer_data(conn)`'dan kurar:

- Bağlantının `data` listesinde olmayan beslemenin hiçbir olayı gönderilmez.
- Listeye yeni giren besleme için önce o beslemenin ilk barları ve o ana kadarki olayları
  verilir (`ReplayHost::feed_data`), sonra protokol mesajları (dilim veriden sonra gelir).
- Listeden çıkan beslemenin olayları kesilir (cihaz açık yansısını bırakır).

`qp demo` (§21.4): `GET /events?…&mode=viewer|replica`; **varsayılan `viewer`**.
`mode=replica` R4'ün tam yansısıdır (ölçüm başvurusu, sayfada `?mode=replica`). Viewer
bağlantısının çerçeveleri:

- `{"start":{conn, token, principal, lang, mode:"viewer", data:{feeds:[], entitlements},
  events:[], instruments:[sembol…], info}}` — bar yok; `instruments` ayar sayfasının listesi.
- `{"data":[DataEvent…],"replay":ReplayState}` — yalnız abone olunan beslemelerin olayları
  (akış ekranında boş dizi + oynatma durumu).
- `{"bind":{"feeds":[InitFeed…],"events":[DataEvent…]}}` — yeni abone olunan besleme.
- `{"msg":ServerMsg}` — değişmedi.

Kabuksuz sürüş: `qp drive --mode viewer` (her özetten önce akıştaki bütün kapsülleri
görünür yapar; özetler: dilimin özeti `document`, yüzler, açık sahne, bakış durumu, `data`).
`scripts/verify.sh` `demo` adımı tarayıcıdaki iki kipi de sürer: `?mode=replica` = `qp drive`,
varsayılan = `qp drive --mode viewer`, ve viewer sürücüsünün yüzleri ve açık sahnesi = tam
yansınınki.

### 22.6 Sınırlar

- Yüz sahnesi okuyucunun dünyasında derlenir (`as_reader`, §20.5); akış lisansı olmayan
  okuyucunun yüzü de kendi dünyasındandır. Barın veri yetkisi olmayana yüz sahnesi gitmez
  (sahne veriden türer, K33).
- Dilim, enstrümanın bütün olgu ve hikâye geçmişini taşır (gösterimde 10–100 KB); dilimin
  değişen kalemlerini artımlı göndermek öneridir (`M15.md`).
- Akış sorgusu ve görünür yüzlerin derlenmesi otoritenin bağlantı başına işidir; evrenle
  büyür (izin verilen), cihazın işi büyümez.

Döküm: `fixtures/protocol/conformance/59-viewer.json` (abonelik ve ilk akış; görünür
yüzler, özetleriyle; kapsül açılınca dilim ve tek beslemelik veri aboneliği; adım sonrası
değişen yüzler; kapatınca `closed` ve boş abonelik; ikinci abonelik `duplicate`, bilinmeyen
`sub` `not_found`).

## 23. Enstrümanın hikâye listesi (R5/M18)

> Kararlar K50 (grafik derinliği enstrümanın hikâye çizelgesidir), K54 (kapanmışlar
> varsayılan görünür), K53 (yazı tura kapsüller akışta gizli, çizelgede işaretli), K48
> (cihaz listeyi grafik açılınca ister), K33/K34 (yetkisize rakam gitmez), K23 (bar
> taşınmaz). Şema `qp-schema::protocol` (`protocol/story.rs`: `StoryRow`, `StoryFace`,
> `StoryOutcome`, `FeedHidden`), otorite `crates/qp-session/src/story.rs`, cihaz
> `crates/qp-wasm/src/viewer.rs` (`Stories`). Bu bölüm çizimi tanımlamaz: liste, hikâye
> çizelgesinin tasarım alternatiflerinin (M18a–c) ortak girdisidir.

Grafik derinliğinde bir enstrümanın, görünen aralıktaki **bütün** kapsülleri (hikâyeleri)
çizilir: her biri doğum barında işaretiyle, ömrü (doğumdan kapanışa, açıksa son kesin bara
kadar) ve sonucuyla. Liste kapsülün kimliğini, türünü, yönünü, doğum ve kapanış barını,
sonucunu, geçerliliğini, akıştaki yerini ve yüz özetini taşır; **bar taşımaz** (K23): bar
indeksleri ve zamanlar, cihazın kendi veri aboneliğindeki barlara göre çizilir (§22.4).

### 23.1 Sorgu — `stories`

```json
→ {"type":"query","id":"r5","query":{"kind":"stories","instrument":"THYAO","range":{"last":25}}}
← {"type":"result","id":"r5","seq":…,"data":{"kind":"stories","instrument":"THYAO","series":"px",
   "total":5,"stories":[…,
   {"id":"cap.THYAO.reversal.up.383","finding":"fnd.THYAO.reversal.up.383","series":"px",
    "kind":"reversal","direction":"up","tier":3,
    "born_bar":383,"born_t":1782172800000,"last_bar":398,"last_t":1783987200000,
    "closed_bar":398,"closed_t":1783987200000,"stage":"expired","outcome":"expired",
    "validity":"closed","in_feed":false,"hidden":"sensitivity",
    "face":{"event":"yukarı dönüş","direction_word":"yukarı","when":"15 işlem günü önce",
            "moved":"dün","validity_phrase":"süresi doldu",
            "ledger_short":"3'te 3 tuttu (az örnek)","few_cases":true,"ledger_n":3}}]}}
```

| Alan | Anlamı |
|------|--------|
| `instrument` | Enstrüman sembolü (zorunlu). Okuyucunun dokümanında bu sembolün bar serisi yoksa `not_found` (`/query/instrument`, öneriyle). |
| `series` | Enstrümanın bar serisi; yalnız birden çok bar serisi varsa gerekir (yoksa `ambiguous`, `did_you_mean` seriler). Yanlışsa `not_found` (`/query/series`). |
| `range` | §5.4 **aralık** biçimi (`{"last":n}`, `{"from_bar","to_bar"}`, `{"from_t","to_t"}`; tam olarak biri), o serinin barlarına göre çözülür. Yoksa enstrümanın bütün hikâyeleri. Boş aralık ya da iki biçim birden `invalid` (`/query/range`). |
| `limit` | Sayfa boyu: varsayılan **200**, en çok **500** (`STORIES_DEFAULT_LIMIT`, `STORIES_MAX_LIMIT`); `0` ya da 500'den büyük `invalid` (`/query/limit`, `expected`). |
| `after` | Sonraki sayfa: önceki cevabın `next`'i (bir kapsül kimliği). Bu listede olmayan kimlik `not_found` (`/query/after`) — okuyucunun göremediği kapsülün kimliği de aynı cevabı alır (varlık kâhini yok). |

**Aralığa giriş:** hikâyenin ömrü `[born_bar, son]` aralıkla kesişirse listededir; `son`
kapanmış hikâyede kapanış barı, açık hikâyede serinin son kesin barıdır (ya da daha büyükse
son geçiş barı). Böylece aralığın öncesinde doğup içinde süren ya da kapanan hikâye de
gelir (çizelge onun ömrünü çizer).

**Sıra ve kararlılık (deterministik):** doğum zamanı (doğum geçişinin `t`'si), sonra doğum
barı, sonra kapsül kimliği — artan. Yeni doğumlar her zaman sona eklendiğinden `after`
imleci canlı güncellemede kaymaz; geri çekilen hikâye listeden düşmez (`outcome:
"retracted"`). Aynı doküman ve aynı istek aynı baytları verir.

**Cevap** (`data.kind = "stories"`): `instrument`, `series` (bar indekslerinin serisi),
`stories` (bu sayfanın satırları), `total` (aralıktaki satır sayısı, bütün sayfalar), `next`
(sonraki sayfa varsa imleç), `comments` (aşağıda, yetki).

**Satır** (`StoryRow`):

| Alan | Anlamı |
|------|--------|
| `id`, `finding`, `series` | Kapsül, bulgu, bar serisi |
| `kind`, `direction` | Anlam türü, yön |
| `tier` | Önem kademesi (1 en önemli), okuyucunun dünyasında (§20.5) |
| `born_bar`, `born_t` | Doğum barı ve zamanı |
| `last_bar`, `last_t` | Son geçişin barı ve zamanı |
| `closed_bar`, `closed_t` | Kapanış (yoksa hikâye açık) |
| `stage` | Şimdiki aşama |
| `outcome` | Kapanmışsa sonuç: `realized` (gerçekleşti), `invalidated` (tutmadı), `expired` (süresi doldu), `retracted` (geri çekildi; karnede yok) |
| `validity` | `valid` / `no_longer` / `closed`, serinin son kesin barı itibarıyla (K42/A28; kapsül öğesi ve işaretle aynı değer) |
| `in_feed`, `hidden` | Hikâye okuyucunun **akış süzgecinden** geçer mi (akışın uzunluğundan bağımsız); geçmiyorsa neden (aşağıda) |
| `coin_flip` | (K53) Karnesi yazı turaya yakın işareti |
| `face` | Yüz özeti (`StoryFace`): okuyucunun dilinde başlık yapı taşları — `event`, `direction_word`, `when`, `moved`, `validity_phrase`, `ledger_short`, `few_cases`, `ledger_n` (`docs/CONCEPTS.md` "Başlık yapı taşları"; yüz tasarımı başlığı bunlardan kurar). Metinler satırın yazıldığı an itibarıyla; tam cümle ve kanıt `capsule` sorgusuyla (§20.2) alınır |
| `redacted` | Okuyucuya rakam gitmez (K33): yüz metinleri rakamsız, `ledger_n` yok |

### 23.2 Akıştaki yer ve gizlilik (K53, hassasiyet)

Çizelge her hikâyeyi gösterir; akış ise okuyucunun kitlesine göre süzer (§20.4). Satır,
hikâyenin **akışta neden olmadığını** söyler (sıra; ilk tutan neden yazılır):

1. `watchlist` — enstrüman izleme listesinde değil (pozisyon ve etkin niyetin enstrümanı
   her zaman geçer).
2. (K42/A22) Pozisyon ya da etkin niyetle ilgili hikâyenin **olumsuz** aşaması (`weakened`,
   `invalidated`, `retracted`) her durumda akıştadır.
3. `coin_flip` — (K53) karnesi yazı turaya yakın ve okuyucunun hassasiyeti `all` değil.
4. `sensitivity` — kademesi seviyenin altında (`important` yalnız 1, `balanced` 1–2).

**Hassasiyetle ilişki (K53 kural 4):** yazı tura işaretli hikâye `important` ve `balanced`
seviyelerinde akışa girmez (`hidden: "coin_flip"`), **en açık seviyede (`all`) akışta
işaretli görünür** (`in_feed: true, coin_flip: true`); çizelgede her seviyede işaretiyle
bulunur. "Az örnekli" (`face.few_cases`) yazı tura değildir: gizlenmez, söylenir (K53 kural
3). Gizlenen hikâye karneye işlemeye devam eder (K53 kural 2; karne bu süzgeci görmez).
Akış (`capsules` sorgusunun `audience`'ı, `capsules` konusu, `viewer` akışı) ve satırın
`in_feed`'i **aynı kuralı** kullanır (`Session::feed_place`).

**Eşik (R5/M19):** kural `docs/CAPSULE.md` §9.10'dadır (`rate55`: doğum karnesinde `n ≥ 20` ve
tutma oranı < 0,55; geleceğe bakmaz). Barındırıcının yapılandırmasıdır, protokol mesajı değil:
`SessionInit.feed_rule` (`{"rule":"rate55","exempt_negative":true}`; bilinmeyen ad başlangıçta
reddedilir) ya da `Session::set_coin_flip_rule`; gösterim barındırıcısı bir okuyucu için
`set_principal_coin_flip_rule` ile başka kural kurabilir. Verilmezse hiçbir hikâye işaretlenmez
ve akış R4'teki gibidir (var olan dökümler bayt bayt aynı). (2)'deki muafiyet kuralın
`exempt_negative` parametresidir (varsayılan açık, M19 önerisi); kapalıysa yazı tura işaretli
hikâye pozisyonda olumsuz aşamada da `hidden: coin_flip` olur.

Kural açıkken (yalnız o zaman yazılan alanlar): `capsules` öğesinde `coin_flip: true` (akışta
yalnız `all` seviyesinde bulunur; gösterim çekirdeği öğeye "karnesi yazı tura" işaretini
ekler); `audience: true` ile `capsules` cevabında `coin_flip_hidden` (öteki süzgeçlerden geçip
yalnız bu kuralla akış dışında kalan hikâye sayısı); `viewer` olayında akışla birlikte aynı
sayı (`coin_flip_hidden`). Akış boşsa gösterim çekirdeği "Şu an anlatacak bir şey yok." ve
nedenini (bu sayı, ya da izlenenlerde kapsül olmadığı) başlık satırları olarak yazar.

### 23.3 Yetki (K33, K34, Ö37)

Sorgu ve konu okuyucunun dünyasında çalışır (§20.5, `as_reader`):

- **Bütün lisanslı akışlara yetkili** okuyucu otoritenin dünyasını alır: lisanslı akış
  hikâyeleri de listededir, yüzleri rakamlıdır.
- **Lisanslı akış yetkisi olmayan** (açık görünüm) okuyucu açık dünyanın listesini alır:
  lisanslı hikâyeler bu okuyucu için **yoktur** (kimliği `after` olarak verilse bile
  `not_found`); `tier`, karne (`face.ledger_short`, `ledger_n`, `few_cases`), `validity`,
  `in_feed`/`hidden`/`coin_flip` ve sıra yalnız açık veriden türer. Lisanslı verinin bu
  okuyucuya tek kanalı, aralıktaki **sunucu yorumlarıdır** (`comments`: mühür iletileri,
  `trust: "server"`, rakamsız; §20.4 ile aynı öğeler). "Lisanslı kapsülün rakamsız biçimi"
  budur: varlığı ve zamanı yalnız bu iletilerin söylediği kadar.
- **Bar beslemesine yetkisi olmayan** okuyucu (`redacted`): yüz metinleri rakamsız, `ledger_n`
  yok. Satırda kanıt değeri, `confidence`, `importance` (ham önem), karne sayısı ya da veri
  özeti (`evidence_ref`) **hiç yoktur** (kimse için: çizelge bunlara ihtiyaç duymaz; tam
  kanıt `capsule` sorgusundadır, §20.2 kuralıyla).

Girişimsizlik testleri: `crates/qp-session/tests/stories.rs` (aynı açık barlar, farklı
lisanslı akış tohumları → yetkisiz okuyucuya giden `stories` cevapları ve `story` olayları
sunucu yorumları dışında bayt eşit; lisanslı değer ve kimlik taraması; yazı tura kuralı) ve
`crates/qp-session/tests/non_interference.rs`'in sorgu ve konu listesi (`stories`).

### 23.4 Canlı güncelleme — `stories` konusu ve `story` olayı

```json
→ {"type":"subscribe","id":"st","topic":{"kind":"stories","instrument":"THYAO"}}
← {"type":"ack","id":"st","status":"subscribed"}
… bir bar kesinleşince:
← {"type":"notify","sub":"st","event":{"kind":"story","story":StoryRow,"seq":8}}
← {"type":"notify","sub":"st","event":{"kind":"server_comment","comment":{…},"seq":8}}   (yalnız açık görünüm okuyucusu)
```

- Konu `{instrument, series?}` (sorgunun `instrument`/`series` kuralı; yanlışsa aynı
  hatalar, `/topic/...`). Aboneliğin başlangıç durumu `stories` cevabıdır: **abonelik anında
  olay gelmez**; cihaz sorguyu ve aboneliği aynı turda gönderir (otorite mesajları sırayla
  işlediğinden aralarında kaçan değişiklik olmaz).
- Otorite her girdiden sonra (mesaj; veri olayı — yeniden oynatma barındırıcısında adım
  başına, `viewer` olayıyla aynı ritim, §22.2) her `stories` aboneliği için, **okuyucunun
  dünyasındaki günlük konumu** ilerlediyse, enstrümanın her satırının **anahtarını** (aşama,
  son geçiş barı, geçerlilik, kademe, `in_feed`/`hidden`, `coin_flip`, `redacted`) son
  gönderdiğiyle karşılaştırır ve anahtarı değişen ya da yeni olan her satırı bir `story`
  olayıyla gönderir (doğum sırasıyla). Böylece doğum, geçiş, kapanış, bar başına değişen
  geçerlilik ve kitle tercihi değişince akıştaki yer bildirilir. Yüz metinlerinin göreli
  zamanı ("dün") tek başına olay doğurmaz.
- Açık görünüm okuyucusuna konunun enstrümanındaki sunucu yorumları `server_comment`
  olayıyla gelir (§20.4'teki ileti, motor girdisinin olaylarıyla birlikte).
- Satır listeden hiç çıkmaz; bu yüzden "silindi" olayı yoktur.
- **Çıktı sırası** (§15'e ek): `story` olayları, girdinin öteki çıktılarından ve `viewer`
  olayından sonra, bağlantı açılış ve abonelik sırasıyla gelir.
- Dokümana ve günlüğe girmez; okuma izni yeter. MCP'de `subscribe` aracı konuyu, `query`
  araçları `query_stories`'i şemadan alır.

### 23.5 Cihaz tarafı (K48)

Görüntüleyici (§22) listeyi **yalnız grafik derinliğinde** ister: bağlantının bakış durumu
`chart` ya da `evidence` olduğunda ve açık kapsülün dilimi geldiğinde, açık enstrüman için
`stories` sorgusunu (aralıksız, `limit` 500, `next` varsa sonraki sayfalar) ve `stories`
aboneliğini gönderir; satırları tutar, `story` olaylarıyla günceller. Derinlik `capsule`'e ya
da `sentence`'e döndüğünde ya da açık kapsül başka enstrümana geçtiğinde aboneliği bırakır ve
listeyi siler. Akış ekranında ve kapsül derinliğinde hiçbir şey istenmez. Çizim yoktur:
satırlar `ShellCore::stories_json` ile çizelge alternatiflerine verilir.

### 23.6 Sınırlar ve ölçüm

- Sayfa en çok 500 satır; `comments` aralıkla sınırlıdır (sayfalanmaz).
- Konu bütün enstrümanı izler (aralıksız); kitle değişince enstrümanın bütün satırlarının
  yeri değişebilir ve hepsi gelir (döküm 63).
- Ölçüm (`cargo run --release -p qp-session --example make_story_transcripts -- --stats`,
  Apple silicon, release, 1.200 barlık THYAO, son 950 bar): yetkili okuyucuya 202 hikâye →
  ilk sayfa 200 satır, JSON ~108 KB (gzip -9 ~10 KB), otoritede cevap ortanca ~2,9 ms;
  akış yetkisiz okuyucuya 124 satır + 177 sunucu yorumu ~192 KB, ~1,9 ms. Konunun bedeli:
  kesin bar başına otorite işi abonesiz ortanca 0,92 ms, üç aboneyle 2,69 ms (40 barda 76
  `story` olayı).

Dökümler: `fixtures/protocol/conformance/60-stories-entitled.json` (yetkili okuyucu; ~5 ve
~50 hikâyelik aralıklar, sayfalama, İngilizce yüz, kapanmış + açık + akışta gizli karışımı,
hatalar), `61-stories-unentitled.json` (aynı aralık üç okuyucuya: yetkili, akış yetkisiz —
açık dünya ve sunucu yorumları —, veri yetkisiz — rakamsız yüz; lisanslı kimlik imleç
olarak `not_found`), `62-stories-200.json` (~200 hikâyelik aralık, iki sayfa),
`63-stories-live.json` (`stories` konusu: doğum, geçiş, kapanış, geçerlilik, kitle
değişince yer; açık görünüm okuyucusuna sunucu yorumu; abonelikten çıkış).
`cargo run -p qp-session --example make_story_transcripts -- --write` ile üretilir.

## 24. Piyasa durumu (R6)

Kavramlar `docs/CONCEPTS.md` "Canlı veri (R6)" (borsa saati, seans evresi, bayat); dil
`docs/LANGUAGE.md` "Piyasa durumu". Şema `qp_schema::market` (`MarketStatus`, `PhaseCode`,
`PhaseClass`, `MarketSource`), metin `qp_capsule::market`, otorite
`Session::set_market_status` (`crates/qp-session/src/session.rs`), kabuk çekirdeği
`ShellCore::market_json` (`crates/qp-wasm/src/shell.rs`). Testler
`crates/qp-session/tests/market.rs`, `crates/qp-wasm/tests/market_r6.rs`.

### 24.1 Barındırıcı girdisi

Piyasa durumu bir **barındırıcı girdisidir**, protokol mesajı değildir (saat gibi; §11):
`Session::set_market_status(MarketStatus) -> Vec<OutboundLine>` (yeniden oynatmada
`ReplayHost::set_market_status`, geri `seek`'te yeni otoriteye yeniden verilir). Saat yine
`host_clock`'tur; piyasa durumu onun yerine geçmez. Dokümanı değiştirmez, günlüğe girmez.
Aynı durum ikinci kez verilirse hiçbir şey olmaz.

```json
{"exchange_time":1795101000000,"open":false,"holiday":false,"source":"calendar",
 "delayed":true,"stale":false,"phases":{"THYAO":{"code":1,"key":"closed"}}}
```

| Alan | Anlam |
|------|-------|
| `exchange_time` | Borsa zamanı (UTC ms): `mx/timestamp` geldiyse onun; gelmiyorsa (gecikmeli kipte `SUBACK 0x80`) en yeni fiyat kartının `updateDate`'i. Bayatken son verinin zamanı. |
| `open`, `holiday` | Pay piyasası açık mı, bugün tatil mi (`null`: bilinmiyor). |
| `source` | `clock` (MX kaynaklı borsa saati iletisi) ya da `calendar` (XIST takvimi, `qp-calendar`). |
| `delayed` | Gecikmeli besleme (15 dk). |
| `stale` | Bağlantı yok ya da durdu: veri bayat (§18.3 `stale`). |
| `phases` | Sembol → seans evresi (`stockStatus`): `code` beslemenin numarası, `key` makine adı (`qp_matriks::status::SessionPhase::key`). |

**Evre sınıfı** (`PhaseClass`: `closed`, `pre_open`, `continuous`, `closing_auction`,
`closing_price_trades`, `halted`, `other`) evrenin **anahtarından** türetilir
(`PhaseClass::of_key`), numarasından değil: şema çözücünün kod tablosunu kopyalamaz ve
çözücüye bağlanmaz; anahtarlar proto'nun sabit adlarının yılan yazımıdır (K62), bilinmeyen
anahtar `other`'dır.

### 24.2 Okuyucuya izdüşüm (§20.5)

Otorite durumu her okuyucuya **kendi dünyasında** verir (`Session::market_for`):
`open`, `holiday`, `source`, `delayed`, `stale`, `exchange_time` herkese; `phases` yalnız
okuyucunun barlarına veri yetkisi olduğu enstrümanlar için (o enstrümanın bir bağlı serisinin
bütün beslemelerine yetkili). Beslemenin numarası (`code`) okuyucuya gitmez (0 yazılır,
alan gönderilmez). Kapsül cümlesi de aynı kuralla: yetkisiz okuyucunun kapsülü o enstrümanın
evresini söylemez, yalnız piyasanın kapalılığını ve bayatlığı.

### 24.3 Bildirimler

- `viewer` olayı (§22.2) isteğe bağlı `market` alanını taşır: ilk olayda (durum verildiyse)
  ve okuyucunun izdüşülmüş durumu değiştiğinde. Özet bağlantı başına, izdüşülmüş değerden.
- Kapsül öğeleri (`capsules` sorgusu, `capsule` sorgusu, `capsules` konusu, `viewer`
  akışı ve yüzleri) piyasa notunu taşır (`docs/LANGUAGE.md` "Piyasa durumu"): enstrümanın
  evresi sürekli işlem değilse ya da veri bayatsa R4 cümlesinin sonunda bir cümle, sesin
  başlığının sonunda " · <söz>", ayrıntısının sonunda bir cümle, metin yüzü yeniden kurulur.
  Durum değişince `viewer` aboneleri değişen akışı ve yüzleri alır; `capsules` konusu yalnız
  hikâye geçişlerinde yayar (durum değişimi geçiş değildir).
- Durum hiç verilmemişse hiçbir alan yoktur: var olan dökümler bayt bayt aynı.

### 24.4 Gösterim (`qp demo`) çerçevesi

Barındırıcı viewer bağlantısına `viewer` olayıyla zaten gönderir. Ayrıca (tam yansı kipi ya
da `viewer` aboneliğinden önce) ayrı bir SSE çerçevesi yazabilir:

```
{"market": MarketStatus}
```

Okuyucuya izdüşülmüş değer (`Session::market_for(principal)`) yazılmalıdır, ham durum değil.
Kabuk çerçeveyi olduğu gibi `core.on_frame`'e verir; çekirdek durumu tutar ve
`core.status()` cevabına `market: {text, state}` ekler (`state`: `live`, `delayed`,
`closed`, `stale`); kabuk metni `#market` satırında olduğu gibi gösterir. Metin
`qp_capsule::market::status_line`: "Canlı · gecikmeli 15 dk" / "Live · delayed 15 min",
"Seans kapalı" (tatilse "· tatil") / "Session closed", kapanış emir toplamada (evrelerin
çoğu kapanış seansındayken, takvimin `open`'ı ne derse desin; A48, RP4/T7) "Kapanış
seansında" / "Closing auction" (durum `closed`), "Bağlantı yok — veri bayat · son veri
18:10" / "No connection — data stale · last data 18:10". Enstrümanı sürekli işlemde olmayan
ya da veri bayatken hiçbir kapsül canlı işaretini (nabız) göstermez
(`ShellCore::market_live`).

## 25. Ajan yüzeyi (R5B)

Çerçeve `rounds/R5B/GOAL.md` (ilkeler: motor söyler, ajan anlatır; her cümle kanıtına
bağlanır; ajan da bir okuyucudur), sözleşme `rounds/R5B/PLAN.md` §1–§4. Şema `qp-schema`
(`Principal.on_behalf`, `ProtocolError.reason`/`reasons`, `Op::AddComment`/`AddWatch`/
`RemoveWatch`, `Comment`, `Watch`, `Query::Preview`, `Topic::Comments`,
`Event::IntentMatched`/`Comment`, `SessionInit.agent_policy`/`comment_strictness`);
referans otorite `crates/qp-session/src/agent.rs`, `comments.rs`, `watch.rs`,
`preview.rs`. Bütün yeni alanlar isteğe bağlıdır; R5B öncesi dökümler bayt bayt aynı.

### 25.1 Kimlik ve köken

- `Principal.author` zaten `human | agent | engine`'dir. Yeni: `Principal.on_behalf`
  (yapılandırma, istemciden gelmez) — ajanın **adına çalıştığı kişi**. Hedef bilinen,
  yazarı `human` olan ve kendisi kimsenin adına çalışmayan bir kişi olmalıdır; değilse
  oturum başlamaz (zincir ve döngü yok).
- **Ajan bağlantısı:** barındırıcı bir bağlantıyı ajan kökenli işaretler
  (`Session::mark_agent_connection(conn)`, `hello`'dan önce ya da sonra;
  `Session::is_agent_connection`). `qp mcp serve` her MCP bağlantısını işaretler; istemci
  işareti kaldıramaz.
- **Ajan kökeni** = yazarı `agent` olan kimlik **ya da** işaretli bağlantı. **İnsan kökeni**
  = yazarı `human` olan kimlik **ve** işaretsiz bağlantı.
- Ajan kökenli her girdinin `author`'u ajan, ürettiği öğelerin `provenance.author`'u
  `{"kind":"agent",…}`'tır; yorumlar ayrıca `on_behalf`'ı taşır. Kabul edilen önerinin
  girdisi önerenin (ajan), `approved_by` karar verenin (insan) adını taşır (§4.7).

### 25.2 İnsan kararı (koşulsuz)

`accept_proposal`, `reject_proposal`, `approve_finding`, `reject_finding` yalnız **insan
kökenli** komutla olur. Aksi — rol `owner` olsa da, kip ne olursa olsun (`auto`, `direct`,
`propose`), öneri kendi önerisi de başkasınınki de olsa — `forbidden`,
`reason: "human_only"`, `path` karar işleminin yeri. Komuttaki tek bir karar işlemi bütün
komutu reddeder; ajan kararı öneri olarak da gönderemez. Denetim rol ve kip
denetimlerinden **önce** yapılır.

```json
{"type":"command","id":"d","ops":[{"op":"accept_proposal","proposal":"prop-6"}]}
{"type":"error","id":"d","error":{"code":"forbidden","reason":"human_only","path":"/ops/0",
 "message":"`accept_proposal` is a human decision; this command is not of human origin","hint":"…"}}
```

Var olan dökümlerin hiçbiri ajanla karar vermez (üç taşımada doğrulandı). MCP yüzeyi karar
araçlarını listelemez; genel `command` aracıyla gönderilse de otorite reddeder.

### 25.3 Ajanın yazabildikleri (izin listesi)

Kural yalnız **işaretli bağlantıda** ya da `init.agent_policy = "r5b"` olan oturumda
ajan kökenli komutlara uygulanır (RD1 dökümü 05'te `scout` bulgu öneriyor; o döküm
değişmez). Bilinmeyen politika adı oturumu başlatmaz. Kural **izin listesidir**
(`qp_session::agent::agent_op`); listede olmayan her işlem — sonradan eklenen işlemler
dahil — reddedilir:

| Sınıf | İşlemler | Nasıl |
|-------|----------|-------|
| Doğrudan | `add_comment` (§25.4), `add_watch`, `remove_watch` (§25.5) | Uygulanır. |
| Yalnız öneri | `put_view`, `insert_layer`, `delete_layer`, `move_layer`, `split_layer`, `set_encoding`, `insert_panel`, `delete_panel`, `move_panel`, `set_panel`, `set_scale`, `set_emphasis`, `set_view_range`, `add_narrative`, `show_finding`, `add_study` (yalnız yeni kimlikle) | `mode: auto` ve `propose` öneri olur (rol `editor`/`owner` olsa da); `mode: direct` → `forbidden`, `reason: "proposal_only"`. Öneri yetkisi (`propose`) yine gerekir. |
| Asla | Diğer her şey: `add_fact`, `add_finding`, `update_finding`, `put_series` (her kaynakla), `set_params`, `append_bars`, `update_last_bar`, `undo`, `remove`, `add_intent`, `set_intent_active`, `set_audience`, `set_sensitivity`, eski `add_layer`/`remove_layer`, var olan kimlikle `add_study` | `forbidden`, `reason: "agent_cannot_write"`, her kipte (öneri olarak da kabul edilmez). |

Doğrudan ve yalnız-öneri işlemleri aynı komutta karışırsa `invalid`. Olgu, bulgu, kapsül,
karne ve hikâye durumu yalnız motordan (ya da insandan) gelir.

### 25.4 Yorum ve doğrulayıcı

```json
{"op":"add_comment","id?":"…","about":{"capsule":"cap.THYAO.continuation.down.84"},
 "lang":"tr","text":"THYAO kısa ortalamanın (5,23) altında kalıyor [1].",
 "cites":["cap.THYAO.continuation.down.84"],
 "external?":[{"source":"…","quote":"…","values":[…]}]}
```

- `about`: tam olarak biri — `capsule` (kapsül ya da bulgu kimliği; kapsül kimliğiyle
  saklanır) ya da `instrument` (belgedeki bir serinin sembolü). Yetki: `propose` ya da
  `write`; **öneri olmaz**, doğrudan uygulanır. Kimlik `cmt-<seq>-<n>`.
- Sıra: rol ve kip denetimi → (kapsül motoru olan oturumda) yazarın dokümanında kuru koşu
  (görünmeyen kapsül olmayanla aynı `not_found`, öneriler yalnız görünenlerden) →
  **doğrulama** → açılım.
- **Kanıt** (`qp_capsule::comment::Evidence`) yazarın **okuyucu yetkisiyle** toplanır
  (ajan: `on_behalf` kişininki, §25.7), yazarın dünyasında: atıf yapılan her kapsül (ya
  da bulgu) için okuyucunun `capsule` cevabı — kanıt değerleri, olguların değerleri,
  okuyucunun cümlesindeki sayılar (başlık yapı taşları okuyucuya izdüşülmüş halleriyle),
  karne `n` ve `realized`, yön —; atıf yapılan olgu için okuyucunun olgu değerleri.
  Sembol listesi belgedeki serilerin sembolleri. Okuyucu her şeyi göremiyorsa (açık görünüm
  okuyucusu ya da izdüşümü olan okuyucu) otoritenin atıf yapılan öğeler ve enstrüman
  hakkındaki kendi sayıları (olgu değerleri, saklı iddialar) `withheld`'dir.
- **Doğrulama** `qp_capsule::comment::validate` (dört denetim + cümle başına atıf; PLAN
  §2.1). Sıkılık oturum yapılandırması `init.comment_strictness` (`cites`, `cites_numbers`,
  `per_sentence`; yoksa `cites_numbers`); saklanan yorum sıkılığı taşır.
- **Ret:** `error` `code: "comment_rejected"`, `reasons: [Reason…]` (makinece okunur:
  `check`, `code`, `token?`, `sentence?`, `hint`). Doküman değişmez.
- **Kehanet yok:** okuyucu yetkisi kısıtlı yazara lisans nedeni (`licensed_value`,
  `licensed_external`) `unsupported_number` olarak, sayı nedenleri `token`'sız gider;
  görünmeyen kimlik olmayan kimlikle aynı `unknown_id`'dir. Lisanslı değerle ve uydurma
  değerle yazılmış aynı taslak aynı hata baytlarını alır; ret yine olur. Gerçek nedenler
  yalnız barındırıcıda: `Session::comment_audit()` (protokol dışı; M30 ölçümü lisans
  yakalamayı buradan sayar). Not: `cites` sıkılığında sayı denetimi kapalıdır; lisans
  denetimi açık olduğundan orada kabul/ret farkı bir kehanet olarak kalır (açık nokta).
- **Kabul:** `put_comment` değişimi; dokümanda `comments` (boşsa yazılmaz),
  `Comment {id, about, lang, text, cites, external?, strictness, provenance, on_behalf?}`.
- **İki dünya** (§20.5): yetkili yazarın yorumu açık dünyaya yalnız bir açık görünüm
  okuyucusu onu yazabilecek olsaydı geçer (lisanslı sayı yok, her atıf açık); yoksa yalnız
  otoritenin dünyasındadır.
- **Okuma:** `capsule` cevabında `comments` (okuyucunun alabileceği: her atıfı okuyucuya
  görünür; izdüşümü olan okuyucu için metindeki her sayı okuyucunun kendi kanıt değerleri
  arasında — dış alıntılar orada sayılmaz). `comments` konusu
  `{"kind":"comments","capsule?":…,"instrument?":…}` → `{"kind":"comment","comment":…,"seq"}`
  aynı kuralla. İzdüşümü olan okuyucuya `document`, `change`, `log` yalnız gizli atıfı
  olmayan ve sayı içermeyen yorumları taşır; sayı içeren `add_comment` isteği günlükten
  çıkarılır. `list` koleksiyonu `comments`.

### 25.5 Niyet (izleme)

`Intent` korunur; yeni alanlar `watch`, `state`, `matches`:

```json
{"op":"add_watch","id?":"…","for?":"arkin","text":"THYAO'da olumsuz bir şey olursa",
 "match":{"instrument":"THYAO","holdings":false,"meaning":["breakout"],"direction":"up",
          "stages":["born","weakened"],"negative":false},
 "expires_t?":1760100000000,"max_matches?":1}
{"op":"remove_watch","intent":"intent-7-0"}
```

- `add_watch` → `put_intent` (`watch` dolu, `trigger` yok, `state: "active"`). `for`
  yoksa konuşan (ajan: `on_behalf` kişisi); başkası için yalnız `owner` (`forbidden`).
  Yetki `read` (kişisel tercih gibi); öneri olmaz. Kişi başına en çok 32 etkin izleme
  (`limit`, `expected: ["32"]`); `welcome.limits.max_watches` yalnız ajan politikası açık
  oturumda yazılır (eski `welcome`'lar değişmez). `max_matches: 0` → `invalid`.
- `remove_watch` → `put_intent` (`active: false`, `state: "removed"`). Başkasının
  izlemesi olmayanla aynı `not_found` (öneriler yalnız kendi izlemelerinden); etkin değilse
  `invalid`.
- **Yaşam döngüsü:** `active → matched` (`max_matches` dolunca) `| expired` (saat ≥
  `expires_t`) `| removed`. `matches` eşleşme sayısıdır.
- **Eşleşme motorda, deterministik:** kapsül motorunun her bakışında (seri başına tek motor
  girdisi, §20.1) bakışın hikâye geçişleri motorun sırasıyla, etkin izlemeler doküman
  sırasıyla sınanır: önce süresi dolanlar `expired` olur; sonra her geçiş için
  `instrument`, `holdings` (kişinin kitlesindeki sıfırdan farklı pozisyon), `meaning`,
  `direction`, `stages` (geçişin aşaması), `negative` (`weakened`, `invalidated`,
  `retracted`) — verilen her koşul VE'lenir. İzlemelerin yeni sayısı ve durumu **aynı
  motor girdisinde** `put_intent` olarak yazılır (yeniden oynatma aynı dokümanı verir).
  Açık dünya kendi izlemelerini kendi geçişleriyle sınar.
- **Olay:** `intents` konusunda `{"kind":"intent_matched","intent","capsule","stage","bar","t","seq"}`;
  yalnız izlemenin kişisine ve onun adına çalışan ajanlara; açık görünüm okuyucusuna açık
  dünyanın eşleşmeleri (lisanslı hikâye hiç eşleşmez), izdüşümü olan okuyucuya göremediği
  kapsül hakkında olay gitmez. Aynı girdiler → aynı olaylar, bayt eşit (test
  `crates/qp-session/tests/agent_watch.rs`).
- **Gizlilik:** izleme kişisel veridir. Başka okuyucuya (`owner` ve barındırıcı hariç;
  ajan için okuyucu = `on_behalf` kişisi) `document`, `change`, `log`, `list intents`
  başkasının izlemelerini taşımaz. İzleme yalnız kendi kişisinin ilgisini (`relevant`,
  K42) belirler; başka kişinin izlemesi sayılmaz.

### 25.6 Öneri önizlemesi

`{"kind":"preview","proposal":"prop-6","width?":1200,"height?":600}` → `{"kind":"preview",
"proposal","changes":[ChangeOp…],"views":[{"view","new","explain"}]}`: bekleyen önerinin
işlemleri, kabulün o an açacağı gibi, okuyucunun dokümanının bir kopyasında açılır;
`changes` yazılacak değişimler, `views` önerinin eklediği ya da değiştirdiği (ya da yeni
serisini okuyan) her görünümün sahne özeti (`explain`, `docs/GRAMMAR.md` §16). Doküman ve
günlük değişmez. Bekleyen değilse `not_pending`; yoksa `not_found`. MCP `render` aracı
önizlemeyi PNG olarak çizer (`proposal` argümanı).

### 25.7 Okuyucu yetkisi

Ajanın okuduğu her şey — sorgular, abonelikler, `hello` özeti, girdi, izdüşüm (§20.5),
açık dünya, yorum kanıtı, önizleme — `on_behalf` kişinin veri yetkileriyledir: ajan bir
akışa yalnız kişi yetkiliyse **ve** (sağlayıcı ajanın kendi satırı olmadığını kesin
söylemiyorsa) ajanın kendisi de yetkiliyse yetkilidir (kesişim; sağlayıcı bilmiyorsa
kesişim — `SourceProvider::has_row`, kuşkuda ret). Kitle: ajanın kendi kitlesi yoksa
kişininki. Test (`crates/qp-session/tests/agent_reader.rs`): lisanslı akışlı oturumda
(döküm 39) yetkisiz kişi adına çalışan ajan, işaretli ya da işaretsiz bağlantıda, kendi
geniş satırı olsa da her sorgu ve abonelik cevabında kişinin aldığı baytları alır (kimlik
alanları hariç).

### 25.8 Dökümler

65–70 `cargo run -p qp-session --example make_agent_transcripts -- --write` ile üretilir:
65 insan kararı (ajanın kendi önerisini kabul/ret reddi, insan kabulü ve köken), 66 ajan
yazamaz (`agent_policy`, `agent_cannot_write`, `proposal_only`), 67 yorum kabul ve ret
(`comment_rejected`, `comments` konusu), 68 yetkisiz kişi adına yorum (kehanetsiz ret), 69
niyet yaşam döngüsü ve eşleşme (`intent_matched`, `limit`), 70 önizleme. Var olan dökümler
değişmedi.

## Ek A — Eski `Command` tipi

`qp-schema` v0'daki `Command {provenance, op}` ve `Op` (doküman şeması) bu protokolden
önceki taslaktır; köken istemciden geldiği için protokolde kullanılmaz. Şema dışa
aktarımında `Op` adı artık protokol işlemini gösterir.
