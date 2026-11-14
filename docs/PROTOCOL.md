# Qapsule Alan Protokolü — `qp/0`

Durum: **v0** (RD1). Mesaj tiplerinin tek kaynağı `crates/qp-schema/src/protocol.rs`;
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
dokümanı **bayt düzeyinde** aynen elde edilir. Yeniden oynatma için hesap kodu, sunucu
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
 "seq":4,"epoch":0,
 "profile":{"depth":"sentence","series_data":false,"max_points":0,"max_glyphs":1,"speech":false},
 "limits":{"max_message_bytes":1048576,"max_ops":64,"max_bars":10000,"max_subscriptions":32,"max_string":4096}}
```

- `surface.kind`: `desktop` | `phone` | `watch` | `voice` | `headless`.
- `surface.max_depth`: yüzey türünün izin verdiğinden daha sığ bir üst sınır istemek için.
- `features`: isteğe bağlı özellikler (`"attention"`, `"narrative"`). v0 otoritesi ikisini de
  destekler; tanımadığı özelliği sessizce dışarıda bırakır.
- `hello` olmadan gelen her mesaj, ikinci bir `hello`, bilinmeyen `principal` ya da
  desteklenmeyen `protocol` → `error` `handshake`.

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
etiketli; `result.seq` cevabın yansıttığı günlük konumudur.

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
| `put_view` | `view` | Görünümü ekle/değiştir (Görünüm v0 tipi; R2'de gramer). |
| `add_layer` | `view`, `layer`, `index?` | Katman ekler (yoksa sona). |
| `remove_layer` | `view`, `index` | Katmanı siler. |
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

### 5.2 Örnek: üç işlem tek komutta

```json
{"type":"command","id":"c3","ops":[
  {"op":"add_study","id":"sma20","study":"sma","inputs":["px"],"params":{"period":20},"show":{"view":"main","panel":0}},
  {"op":"add_study","id":"sma50","study":"sma","inputs":["px"],"params":{"period":50},"show":{"view":"main","panel":0}},
  {"op":"add_intent","text":"SMA20 SMA50'yi yukarı keserse haber ver",
   "trigger":{"kind":"compare","left":"sma20","op":"cross_above","right":"sma50"}}]}
```

### 5.3 Değişimler (`ChangeOp`)

`put_series`, `put_view`, `insert_layer`, `delete_layer`, `set_x_domain`, `put_fact`,
`put_finding`, `put_capsule`, `set_capsule_state`, `put_intent`, `push_ledger`,
`pop_ledger`, `put_narrative`, `put_proposal`, `delete`, `append_bars`, `truncate_bars`,
`set_last_bar`. Her biri tam öğe taşır; `put_*` değişimlerinin isteğe bağlı `index` alanı
yeni öğenin koleksiyondaki yeridir (yoksa sona; var olan öğe yerinde değiştirilir) —
silmenin tersi öğeyi eski yerine koyar, böylece sil + geri al dokümanı bayt düzeyinde
aynı bırakır; köken (`provenance`) otorite tarafından damgalanır
(istemci köken yazmaz). İstemcinin `Op`'ları değişimlere açılır; örneğin
`reject_finding` → `set_capsule_state` + `push_ledger`; `add_study` (+`show`) →
`put_series` + `insert_layer`.

### 5.4 Sorgular (`Query`, alan `"kind"`)

| `kind` | Alanlar | Cevap (`data.kind`) |
|--------|---------|---------------------|
| `describe` | — | `description`: işlemler, sorgular, konular, hesaplar ve varsayılan parametreleri, kişiler ve rolleri, sorananın yetkileri. |
| `get` | `id` | `item`: öğe ve koleksiyonu. |
| `list` | `collection`, `state?` | `items`. Koleksiyonlar: `series`, `views`, `facts`, `findings`, `capsules`, `intents`, `narratives`, `proposals`; `state` yalnızca öneriler için. |
| `document` | — | `document`: dokümanın tamamı (anlık görüntü; devir için). |
| `log` | `after_seq?`, `since_t?`, `until_t?`, `author?`, `limit?` | `log`: girdiler, `seq` artan. `limit` en son n girdiyi verir. |
| `study` | `series`, `range?`, `output?`, `reduce?` | `values` (barlarla hizalı, ısınma `null`; K13/K20) ya da `reduced` (`min`, `max`, `mean`, `first`, `last`, `count`; `null`lar atlanır; `min`/`max`/`first`/`last` bar ve zamanı da verir). Bar serisinde çıktılar `o`,`h`,`l`,`c`,`v`. |
| `at` | `series`, `bar?` ya da `t?` | `at`: OHLCV ve o bar serisine dayanan her hesabın o bardaki değerleri. |
| `screen` | `principal?` | `screen`: kişinin ekranlı (`desktop`, `phone`, `watch`) her bağlantısı için, bağlantı açılış sırasıyla: görünüm, görünen aralık, sembol ve aralık (görünümün ilk bar serisinden), paneller (panel numarası artan; `series` = o paneldeki katmanların serileri, katman sırasıyla, tekrarsız; `marks` = bütün katmanların işaretleri, katman sırasıyla), olgu barı görünen aralıkta olan bulguların kapsülleri, görünen son barın kapanışı. |
| `attention` | `principal` | `attention`: kişinin en son etkin ekranlı bağlantısının bakış aralığı (girdi yoksa başlangıç aralığı), son odak (`tap`: görünüm, panel, bar), zamanı, yönergeler. |
| `present` | `finding?`, `surface?` | `presentation` (§10). `finding` yoksa en önemli canlı bulgu (en yüksek güven; eşitlikte kimlik sırası). |

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
`base` verilen bir komutun dokunacağı kimliklerden biri, `seq > base` olan ve geri
alınmamış bir girdide değiştiyse komut uygulanmaz:

```json
{"type":"error","id":"c9","error":{"code":"conflict",
 "message":"`macd` changed after base 4 (seq 6 by editor)","path":"/ops/0",
 "conflicts":[6],"hint":"Read the item again and resend with base=6, or omit base."}}
```

Çevrimdışı kalan bir istemci döndüğünde komutlarını eski `base` ile gönderir; çakışmayanlar
uygulanır, çakışanlar bu hatayla döner ve istemci okuyup yeniden gönderir (ya da öneriye
çevirir). Yakınsama otoritenin sırasıyla garanti edilir.

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
zaman bağlantının kendi epoch'una aittir; komutta ayrıca epoch taşınmaz. Ölçüm ve
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

`path`, gelen mesajın içine JSON göstericisidir (`/ops/1/params/period`). `did_you_mean`
en çok 5 öneri verir: adaylar, düzenleme uzaklığı ≤ 3 olanlar ve verilen adı içeren ya da
onun içinde geçenler; sıralama düzenleme uzaklığı (alt dizge adayları için de gerçek
uzaklık), sonra alfabe sırası. Adaylar ilgili ad uzayıdır: seri kimlikleri, görünüm
kimlikleri, alan adları… Bir komutta hata varsa **hiçbir** işlem uygulanmaz.

## 14. Güvenlik ve sınırlar

- Dışarıdan gelen her mesaj güvenilmezdir. Sınırlar `welcome.limits`'te bildirilir ve
  ayrıştırmadan **önce** (bayt sayısı) ve sonra (işlem, bar, dizge uzunluğu) denetlenir.
- JSON iç içe derinliği serde_json'ın sınırıyla (128) sınırlıdır.
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
eşit; `"$any"` her değerle eşleşir). `final.document` da alt küme eşleşmesidir. Her döküm
sonunda koşucu günlüğü baştan oynatır ve dokümanın baytlarının aynı olduğunu denetler.

**Çıktı sırası** (deterministik): önce gönderene cevap; sonra oluşan her girdi için sırayla
`change`, `finding_state`, `proposal` olayları; sonra `condition_met` ve
`intent_triggered`; her olay, abonelerine bağlantı açılış sırası ve abonelik sırasıyla.

## 16. Gömme

- **MCP:** komutlar ve sorgular şemadan üretilen MCP araçlarıdır (her `Op` ve her `Query`
  türü için bir araç, girdi şeması o türün JSON Schema'sı; ayrıca genel `command` ve
  `query` araçları). Araç sonucu `structuredContent` olarak protokolün `ack`/`result`/
  `error` mesajıdır; hata `isError: true`. Abonelikler ve dikkat için bkz.
  `rounds/RD1/EMBEDDING.md`.
- **A2UI ve MCP Apps:** analiz `rounds/RD1/EMBEDDING.md`.

## Ek A — Eski `Command` tipi

`qp-schema` v0'daki `Command {provenance, op}` ve `Op` (doküman şeması) bu protokolden
önceki taslaktır; köken istemciden geldiği için protokolde kullanılmaz. Şema dışa
aktarımında `Op` adı artık protokol işlemini gösterir.
