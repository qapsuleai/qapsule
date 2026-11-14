# RD1 — Gömme: MCP, MCP Apps, A2UI

`qp/0` çekirdek protokoldür; MCP, MCP Apps ve A2UI **taşıma ya da teslim kanalıdır**
(ilke 10: gömme protokolü değiştirilebilir kenardır). Bu belge, protokolün bunların
üzerinde nasıl taşındığını anlatır. Kaynaklar: `research/agent-protocols.md` (MCP
2026-07-28, MCP Apps 2026-01-26, A2UI v1.0; bu oturumda güncel belgelerden okundu).

## 1. MCP üzerinde (uygulandı: `crates/qp-mcp`, `qp mcp`)

### 1.1 Eşleme

| `qp/0` | MCP karşılığı | Not |
|--------|---------------|-----|
| `hello` | Uyarlayıcının başlatılması (`qp mcp --principal scout --surface headless`) ya da `initialize`/`server/discover` | MCP 2026-07-28 durumsuz; kimlik bağlantıda değil, uyarlayıcının yapılandırmasında. Yüzey bilgisi `clientCapabilities.extensions["dev.qapsule/surface"]` ile de verilebilir. |
| `command` (tek işlem) | Araç `op_<işlem>` (ör. `op_add_study`); girdi şeması o `Op` türünün JSON Schema'sı (`op` alanı hariç), artı isteğe bağlı `mode`, `base`, `reason` | Araçlar `qp-schema`'nın ürettiği şemadan **makinece** üretilir; elle tanım yok. |
| `command` (çok işlem, atomik) | Araç `command`, girdi = `ClientMsg::Command` gövdesi | Birden çok işlemin hep-ya-hiç uygulanması için. |
| `query` | Araç `query_<tür>` (ör. `query_study`) ve genel `query` | Sonuç `structuredContent` = protokolün `result`'u; `outputSchema` = `ServerMsg`. |
| `ack` / `result` | Araç sonucu: `structuredContent` = mesaj, `content[0].text` = aynı JSON | Metin kopyası, yapılandırılmış içeriği okumayan istemciler için. |
| `error` | Araç sonucu `isError: true`, `structuredContent` = protokolün `error` mesajı (`code`, `path`, `expected`, `did_you_mean`, `hint`) | MCP'nin "modelin kendini düzeltmesi için araç hatası" ayrımıyla birebir. JSON-RPC protokol hatası yalnızca çerçeve bozuksa. |
| `subscribe` / `notify` | Araç `subscribe`; olaylar MCP bildirimi `notifications/message` (veri = `notify` mesajı) olarak; ayrıca kaynak `qapsule://doc/<id>/log?after=<seq>` | MCP 2026-07-28 `subscriptions/listen` yalnızca "kaynak değişti" sinyali taşır ve SSE yeniden oynatmayı kaldırdı. Doğru eşleme: dinleme = "seq N'e ulaşıldı", ayrıntı ve yeniden oynatma bizim `log` sorgumuzla. |
| Öneri | `ack.status = "proposed"` araç sonucu olarak; karar için `op_accept_proposal` | İnsan onayı MCP'de elicitation/MRTR (`InputRequiredResult`) ile istenebilir; ama bizde öneri dokümanın içindedir ve kararı **başka bir kişi** verir, aynı araç çağrısının içinde değil. Bu yüzden elicitation yalnızca kabuk tarafında bir kolaylık olabilir. |
| `attend` | Araç `attend` | Ajan → kullanıcı yönü. Kullanıcıya gösterim kabuğun işidir (MCP Apps aşağıda). |
| `input` | **Taşınmaz** | Ham etkileşim çekirdekte kalır; MCP üzerinden saniyede onlarca girdi taşımak yanlış. Gömülü kabuk (WASM) girdiyi yerel çekirdeğe verir. |
| Sürüm | `qp/0` araç adlarında değil, `describe` ve sunucu bilgisinde | Ek araç = ek işlem; kırıcı değişiklik = yeni büyük sürüm. |

### 1.2 Uyarlayıcı kanıtı

`crates/qp-mcp`: araç listesi şemadan üretilir; JSON-RPC (`initialize`, `tools/list`,
`tools/call`) işler; arkasında aynı `qp_session::Session` vardır. Uygunluk koşucusunun
`mcp` taşıması dökümlerdeki her `command` ve `query` mesajını ilgili MCP araç çağrısına
çevirir, sonucu protokol mesajına geri çevirir ve aynı beklentilerle karşılaştırır.
Sonuçlar: bkz. §4 (uyarlayıcı tamamlanınca dolduruldu).

## 2. MCP Apps üzerinde (analiz)

MCP Apps (`io.modelcontextprotocol/ui`) araçların etkileşimli arayüz döndürmesidir:
araç tanımında `_meta.ui.resourceUri = "ui://…"`, kaynak `text/html;profile=mcp-app`,
arayüz korumalı bir iframe'de çalışır ve `postMessage` üzerinden JSON-RPC ile konuşur.

Qapsule'ın bir MCP Apps barındırıcısında (sohbet istemcisi, IDE) görünmesi:

1. Araç `op_show_finding` (ya da `query_present`) `_meta.ui.resourceUri =
   "ui://qapsule/capsule"` taşır. Kaynak, **WASM çekirdeği** ve ince bir kabuğu içeren
   bir HTML paketidir (çizim çekirdekte; HTML yalnızca kap).
2. Paket içinde **ikinci bir otorite yoktur**: kabuk, barındırıcı üzerinden bizim MCP
   araçlarımızı (`tools/call`, görünürlüğü `["app"]` olan araçlar) çağırarak `qp/0`
   konuşur. Yani paket bir `qp/0` istemcisidir; devir yapılmışsa (§8.4) otorite pakettir.
3. **Yetenek:** barındırıcının `HostContext`'i (`platform`, `deviceCapabilities`,
   `containerDimensions`, `displayMode`) kabukta `hello.surface`'e çevrilir:
   `platform: mobile` → `phone`; kap genişliği < 300 px → `watch` profili;
   `displayMode: inline` → `max_depth: capsule`, `fullscreen` → `chart`.
4. **Girdi** paket içinde yerel çekirdeğe gider (kaydırma, yakınlaştırma); dışarı çıkmaz.
5. **Dikkat (kullanıcı → ajan):** kabuk, kullanıcının baktığı aralığı
   `ui/update-model-context` ile modele bildirir (son değer geçerli — bizim varlık
   durumu semantiğiyle aynı) ve aynı anda `input look` ile otoriteye. **Dikkat (ajan →
   kullanıcı):** ajanın `attend` yönergesi kabuğa `notify` olarak gelir; kabuk adımları
   oynatır. MCP Apps'te bunun standart karşılığı yok; bizim mesajımız taşınır.
6. **Değişim:** kabuk `changes` aboneliğini araç yoluyla açar; MCP Apps'te sunucudan
   arayüze kalıcı itme kanalı olmadığı için `tool-result` bildirimleri ve gerekirse
   `log` sorgusuyla yoklama (`after_seq`) kullanılır. `seq` boşluk tespitini sağlar.
7. **Güvenlik:** barındırıcı CSP kurar; paket yalnızca bildirilen alan adlarına bağlanır.
   Otorite paketin mesajlarını her istemcinin mesajı gibi güvenilmez kabul eder (§14).

Sonuç: MCP Apps bir **teslim kanalıdır**; protokolü değiştirmez. Eksik olan tek şey
sunucudan arayüze olay itmedir; bunu `seq` + `log` sorgusu karşılar.

## 3. A2UI üzerinde (analiz)

A2UI v1.0 ajandan arayüze bildirimsel bileşen akışıdır (`createSurface`,
`updateComponents`, `updateDataModel` (JSON Pointer), `deleteSurface`; geri yönde
`action`, `error {code, surfaceId, path, message}`), kilitli bir bileşen kataloğu ve
`supportedCatalogIds` pazarlığıyla.

1. **Özel katalog bileşeni.** Qapsule bir katalog yayımlar: `dev.qapsule/catalog/v0`
   içinde `QapsuleCapsule` ve `QapsuleChart` bileşenleri. Özellikleri:
   `{doc, finding, depth}` ya da `{doc, view}`. A2UI renderer'ı bu bileşeni WASM ya da
   yerel çekirdekle çizer; çevresindeki kart, düğme, metin A2UI'nin olur.
2. **Bağ:** bileşen özellikleri A2UI veri modeline `{"@path": "/qapsule/finding"}` ile
   bağlanabilir; ajan `updateDataModel` ile hangi bulgunun gösterileceğini değiştirir.
   **Doküman verisi A2UI veri modeline kopyalanmaz**: bileşen kendi `qp/0` bağlantısıyla
   otoriteye bağlanır. Aksi halde iki doğruluk kaynağı olur ve sıra/yeniden oynatma
   kaybolur (A2UI'de `seq` yok).
3. **Yetenek:** renderer `supportedCatalogIds` içinde `dev.qapsule/catalog/v0` bildirirse
   ajan bileşeni kullanır; bildirmezse ajan `present` sorgusunun `sentence`/`capsule`
   derinliğindeki metin ve işaretleri standart A2UI `Text`/`Card` bileşenleriyle
   kurar (zarif düşüş). Derinlik seçimi yine `hello.surface` ile otoritede yapılır.
4. **Eylemler:** A2UI `action` (ör. "bulguyu reddet" düğmesi) kabukta `qp/0`
   `command`'ına çevrilir (`reject_finding`); yetki ve köken otoritede. A2UI `error`
   biçimi bizim `error.path`'imizle uyumludur.
5. **Taşıma:** A2UI AG-UI, A2A uzantısı ya da MCP üzerinden akabilir; Qapsule bileşeni
   hangisi olursa olsun aynı `qp/0` bağlantısını kullanır.

Sonuç: A2UI **yüzey kompozisyonudur** (grafiğin etrafındaki arayüz); grafiğin kendisi ve
doküman protokolü A2UI'ye taşınmaz. Ajanın A2UI bileşen ağacını yazması, K21-1'deki
"ajan grameri doğrudan yazar" seçeneğinin genel arayüz karşılığıdır; ölçüm T6e.

## 4. Uyarlayıcı sonuçları

`crates/qp-mcp` (T9 alt ajanı; ayrıntı `crates/qp-mcp/NOTES.md`):

- **Araç üretimi:** `qp_schema::json_schema("ClientMsg")`'den, elle tanım olmadan, sabit
  sırada 37 araç: 22 `op_<işlem>` (işlem şeması `op` etiketi olmadan + `mode`, `base`,
  `reason`), 10 `query_<tür>`, 5 genel (`command`, `query`, `subscribe`, `unsubscribe`,
  `attend`). Her girdi şeması yalnızca eriştiği `$defs`'i taşır. Testler: her `Op` ve
  `Query` türünün bir aracı var; araç şemaları `fixtures/protocol/valid/` örneklerini kabul
  ediyor. Şemaya yeni bir işlem eklenince araç kendiliğinden çıkar.
- **MCP sürümü:** 2026-07-28 (araçlar, temel, sürümleme, keşif, stdio, kayıt sayfaları bu
  oturumda okundu); `initialize` 2025-11-25 yaşam döngüsüne göre de kabul edilir.
- **Uygunluk:** `qp conform fixtures/protocol/conformance --transport mcp` → **17/17**
  (her döküm bağlantısı aynı süreç içi oturum üzerinde ayrı bir MCP uyarlayıcısı). Aynı
  dökümler `inproc` 17/17, `serve` 17/17. T9 ölçümünde 19 döküm (16 resmî + 3 iç) için
  46 `hello`, 90 `tools/call`, 17 doğrudan adım.
- **MCP üzerinden taşınmayanlar** (doğrudan oturuma gider): `input` (ham etkileşim
  çekirdekte), `clock` (yalnızca host), `send_raw` (JSON değil), `hello`'dan önceki
  mesajlar, kimliği dizge olmayan mesajlar.
- **Kararlar ve sapmalar:** `_meta` olmayan istekler kabul ediliyor (2026-07-28 `-32602`
  ile reddetmeyi öngörür; eski ve basit istemciler için gevşek). Olaylar
  `notifications/message` ile gidiyor; bu yöntem 2026-07-28'de **kullanımdan kalkmış**
  (yalnızca istemci kayıt düzeyi istediğinde) — doğal yol `subscriptions/listen` + `log`
  sorgusu, öneri olarak bırakıldı. Protokolün istek kimliği
  `params._meta["dev.qapsule/id"]`'de; olaylar `_meta["dev.qapsule/order"]` ile
  otoritenin sırasına geri dizilebilir. `outputSchema` ince tutuldu (`tools/list` ~110 KB).
- **Sınır:** uyarlayıcı tek iş parçacıklı (`Rc<RefCell<…>>`); HTTP barındırıcısı için
  `Arc<Mutex<…>>` gerekir.
