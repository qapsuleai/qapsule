# Ajan Protokolleri Araştırması (RD1)

Tarih: 2026-11-14. Tüm bilgiler aşağıdaki sayfalardan, bu oturumda WebFetch/WebSearch ile okunarak alındı; hafızadan yazılmadı. Okuyamadığım veya doğrulayamadığım yerler "DOĞRULANMADI" diye işaretli.

Bizim tasarımın mesaj aileleri (kısaltma): Command, Change, Query, Subscription, Input, Proposal, Attention, Capability; ayrıca versiyonlama, hata modeli, yetki/denetim izi, kimlik, zaman.

## 0. Özet tablo ve genel sonuçlar

| Bizim ihtiyaç | En yakın karşılık | Not |
|---|---|---|
| Command | MCP `tools/call` (+ A2A `SendMessage`, AG-UI tool call) | MCP'de tek yön: client -> server. Biz insan ve ajan için aynı. |
| Change (sıralı, tekrar oynatılabilir) | AG-UI `StateDelta` (RFC 6902) / A2UI `updateDataModel`+`updateComponents` / A2A `TaskArtifactUpdateEvent` | MCP'nin karşılığı yok; yalnızca `notifications/resources/updated` (içeriksiz "değişti" sinyali). |
| Query | MCP `resources/read`, `resources/templates/list`, `tools/call` + `outputSchema` | Anlamsal sorgu için `outputSchema`'lı tool en doğrusu. |
| Subscription | MCP `subscriptions/listen` (yeni) | Filtre = `resourceSubscriptions` URI listesi; koşul/bölge filtresi yok. |
| Input | MCP Apps `ui/*` hiçbiri değil; A2UI `action`; AG-UI yok | Ham dokunma/scroll protokolleri yok. Bu bizim çekirdeğimizde kalmalı. |
| Proposal | MCP Tasks `input_required` + MRTR `InputRequiredResult`; AG-UI `interrupt`/`resume`; A2A `INPUT_REQUIRED` | Üçü de "bekleyen yazma"nın değil "bekleyen girdi"nin modeli. Accept/reject/expire kısmen var (`expiresAt`, `ttl`). |
| Attention | Doğrudan karşılık YOK | En yakın: MCP Apps `ui/update-model-context` (kullanıcı -> model bağlamı), `ui/message`. |
| Capability | MCP `_meta.io.modelcontextprotocol/clientCapabilities` + `server/discover`; MCP Apps `HostContext`; A2UI `supportedCatalogIds`; A2A Agent Card | Hepsinin ortak fikri: yüzey kendini tanıtır. HostContext cihaz boyutunu bile taşır. |
| Versiyon | MCP: her istekte `_meta` protocolVersion; A2A: `A2A-Version` header | |
| Hata modeli | MCP: protokol hatası vs. `isError:true` tool hatası; A2UI: `error` {code,surfaceId,path,message} | Kendini düzeltme için ikinci model doğru. |
| Zaman/canlı | MCP `ttlMs`/`cacheScope`, AG-UI olay akışı, A2A stream | Narration timeline için hiçbirinde karşılık yok. |

Temel sonuçlar:
1. MCP 2026-07-28 revizyonu **stateless**. `initialize` el sıkışması, `Mcp-Session-Id`, `ping`, `resources/subscribe` kalktı. Oturum durumu yok; durum gerekirse açık, sunucunun ürettiği handle (ör. `basket_id`) tool argümanı olarak geçer. Bu bizim "doküman = canlı belge" modelimizle uyumlu: handle = DocumentId + revision.
2. Hiçbir protokol "sıralı, tekrar oynatılabilir Change log" sağlamıyor. En yakını AG-UI `StateSnapshot`+`StateDelta` (sıra garantisi var, yeniden eşitleme için snapshot isteme var, ama revision/sequence numarası yok) ve MCP'nin tersine, MCP 2026-07-28 SSE yeniden oynatmayı (`Last-Event-ID`) bilerek kaldırdı. Change protokolü bizim kendi işimiz.
3. Attention ve Input için hazır standart yok. Bunlar bizim alan protokolümüzün özgün kısmı.
4. Taşıma: bizim protokolü MCP üstünde taşımak mümkün ve doğal (bkz. her bölümdeki "Üstünde taşıma" notu ve en sondaki sentez).

---

## 1. MCP (Model Context Protocol)

### 1.1 Kaynaklar (gerçekten okunanlar)
- https://modelcontextprotocol.io/specification/latest -> yönlendirdi: **2026-07-28** revizyonu (şema `schema/2026-07-28/schema.ts`). Bugünkü "latest".
- https://modelcontextprotocol.io/specification/2026-07-28/changelog.md (2025-11-25'ten farklar)
- https://modelcontextprotocol.io/specification/2026-07-28/basic.md
- https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning.md
- https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/mrtr.md
- https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/subscriptions.md
- https://modelcontextprotocol.io/specification/2026-07-28/server.md
- https://modelcontextprotocol.io/specification/2026-07-28/server/tools.md
- https://modelcontextprotocol.io/specification/2026-07-28/server/resources.md
- https://modelcontextprotocol.io/specification/2026-07-28/client (Roots sayfası döndü; Roots deprecated)
- https://modelcontextprotocol.io/specification/2026-07-28/client/elicitation.md
- https://modelcontextprotocol.io/extensions/tasks/overview.md
- Web araması sonucu 2025-11-25 changelog özeti (sayfa doğrudan okunmadı, arama özeti): https://modelcontextprotocol.io/specification/2025-11-25/changelog

Okunmadı (kapsam dışı kaldı): `server/discover` sayfası, sampling sayfası, progress/cancellation/pagination/caching sayfaları, authorization sayfaları, ext-tasks repo'sundaki tam şema. Bunlarla ilgili cümleler changelog/özet sayfalarından.

### 1.2 Çözdüğü sorun
LLM uygulamalarını (host/client) dış veri ve araçlarla (server) JSON-RPC 2.0 üstünde standart biçimde bağlamak. Üç sunucu ilkeli: Resources (uygulama kontrollü), Prompts (kullanıcı kontrollü), Tools (model kontrollü). İstemci özelliği: Elicitation (Sampling ve Roots 2026-07-28'de **deprecated**, SEP-2577).

### 1.3 Mesaj modeli (2026-07-28)
**Zarf.** JSON-RPC 2.0: Request (`id` string|int, null olamaz), Result (`result.resultType` zorunlu: `"complete"` | `"input_required"`; uzantılar yeni değer ekleyebilir, örn. Tasks `"task"`), Error (`code`,`message`,`data`), Notification.

**Stateless.** "all the information needed to process a request is contained in the request itself." Her istek `_meta` içinde taşır:
- `io.modelcontextprotocol/protocolVersion` (zorunlu), `io.modelcontextprotocol/clientCapabilities` (zorunlu), `io.modelcontextprotocol/clientInfo` (SHOULD), `io.modelcontextprotocol/logLevel` (opsiyonel), `progressToken`, `traceparent`/`tracestate`/`baggage` (OpenTelemetry).
- Sunucu sonuçlarda `io.modelcontextprotocol/serverInfo` döner. Eksik zorunlu alan -> `-32602`, HTTP 400.
- `server/discover`: sunucu desteklediği sürümleri, capability'leri, kimliğini bildirir (MUST implement).

**Versiyonlama.** El sıkışma yok. Sürüm uyuşmazlığı -> `UnsupportedProtocolVersionError` kod `-32022`, `data: { supported: [...], requested: "..." }`; client uygun sürümle yeniden dener. Eski (initialize'lı) sürümlerle "dual-era" uyumluluk matrisi tanımlı. Uzantılar `capabilities.extensions` haritasında (`io.modelcontextprotocol/ui`, `io.modelcontextprotocol/tasks` gibi); "bir taraf desteklemiyorsa çekirdek davranışa dön ya da reddet".

**Hata modeli (iki katman).**
- Protokol hataları (JSON-RPC): bilinmeyen tool, bozuk istek. Kodlar: `-32700`, `-32600..-32603`; MCP'ye ayrılmış aralık `-32020..-32099`: `-32020 HeaderMismatch`, `-32021 MissingRequiredClientCapability` (`data.requiredCapabilities` eksikleri listeler), `-32022 UnsupportedProtocolVersion`. Kaynak bulunamadı: `-32602` (eskiden `-32002`).
- **Tool yürütme hataları**: `result.isError: true` + açıklayıcı `content` metni. Spec'in gerekçesi: "Input validation errors ... should be returned as Tool Execution Errors ... to enable model self-correction" (SEP-1303, 2025-11-25). Örnek metin: "Invalid departure date: must be in the future. Current date is 08/08/2025." Yani hata mesajı düzeltme için gereken bağlamı (güncel tarih) içeriyor.

**Tools.** `tools/list` (sayfalama, `ttlMs`/`cacheScope` önbellek, deterministik sıra SHOULD), `tools/call` {`name`,`arguments`}. Tool alanları: `name`, `title`, `description`, `icons`, `inputSchema`, `outputSchema` (JSON Schema 2020-12; SEP-2106 ile her anahtar kelime serbest), `annotations` (uyarı: "untrusted unless from trusted servers"; okunan sayfada alan isimleri listelenmedi, `readOnlyHint` vb. DOĞRULANMADI bu revizyonda). Sonuç: `content[]` (text/image/audio/`resource_link`/embedded `resource`, her biri `annotations`: `audience`, `priority`, `lastModified`) ve `structuredContent` (artık **herhangi bir JSON değeri**; `outputSchema` varsa uyması MUST; geriye uyum için JSON'u TextContent olarak da dön SHOULD). `notifications/tools/list_changed`.

**Resources.** `resources/list`, `resources/read` -> `contents[]` (`text` | `blob`), `resources/templates/list` (RFC 6570 URI template, `uriTemplate`), `annotations`. Kapasite `resources: {listChanged, subscribe}`.

**Subscriptions (yeni).** `resources/subscribe`/`unsubscribe` ve HTTP GET kalktı; yerine tek `subscriptions/listen` (uzun ömürlü POST-yanıt akışı). Filtre `notifications`: `toolsListChanged`, `promptsListChanged`, `resourcesListChanged` (boolean), `resourceSubscriptions` (URI dizisi). Sunucu ilk mesaj olarak `notifications/subscriptions/acknowledged` yollar (kabul ettiği alt küme); sonraki her bildirimde `_meta["io.modelcontextprotocol/subscriptionId"]` = listen isteğinin id'si. Bildirim: `notifications/resources/updated` {`uri`} (içerik yok; client `resources/read` ile yeniden okur). İptal: SSE kapat / `notifications/cancelled`. Sunucu zarifçe kapatırken `resultType:"complete"` ile listen isteğine yanıt verir. **Yeniden oynatma yok**: "A broken response stream loses the in-flight request; clients MUST re-issue it" (SSE `Last-Event-ID` kaldırıldı); stdio'da bağlantı koparsa client listen'i yeniden gönderir, sunucu abonelik durumu tutmaz.

**MRTR (Multi Round-Trip Requests, SEP-2322).** Sunucu -> client istekleri (elicitation, sampling, roots) artık sunucu başlatmalı değil. Sunucu `tools/call`, `resources/read` veya `prompts/get` yanıtı olarak `InputRequiredResult` döner: `{resultType:"input_required", inputRequests:{ anahtar: {method:"elicitation/create", params:{...}} }, requestState:"opaque"}`. Client cevabı toplar, **yeni id ile** aynı isteği `inputResponses` + aynen geri yankılanan `requestState` ile tekrarlar. `requestState` saldırgan girdisi sayılır; HMAC/AEAD, principal, TTL ve istek özeti ile korunmalı (SHOULD). Tek kullanımlık garantisi protokolde yok; sunucu uygulamalı.

**Elicitation.** `elicitation/create` iki mod: `form` (düz nesne, yalnız ilkel alanlar: string/number/integer/boolean/enum/çoklu enum; `requestedSchema`) ve `url` (hassas/out-of-band akış). Yanıt `action`: `accept` (+ `content`) | `decline` (açık ret) | `cancel` (seçim yapmadan kapatma). Bu üçlü bizim Proposal accept/reject/dismiss için iyi bir model. `elicitationId` ve `notifications/elicitation/complete` 2026-07-28'de **kaldırıldı** (sonuç, isteği yeniden deneyerek öğrenilir).

**Tasks uzantısı** (`io.modelcontextprotocol/tasks`, çekirdekten çıkarıldı, SEP-2663). Sunucu, uzun işte `CreateTaskResult` (`resultType:"task"`; `taskId`, durum, `ttlMs`, `pollIntervalMs`) döner; client opt-in'i capability ile yapar, sunucu istek başına karar verir. Yöntemler: `tasks/get` (polling; terminalde `result` veya `error`), `tasks/update` (`inputResponses`; client -> server girdi), `tasks/cancel` (kooperatif). `tasks/list` ve bloklayan `tasks/result` kaldırıldı. Durumlar: `working`, `input_required`, `completed`, `failed`, `cancelled` (son üçü terminal). `notifications/tasks` ile itme (listen üzerinden opt-in; her bildirim tüm görev durumunu taşır). Task ID kalıcı handle: istemci çökse bile kaldığı yerden polling.

**Güvenlik/kimlik.** OAuth tabanlı yetkilendirme (HTTP). Liste uçları isteğe sunulan yetkiye göre değişebilir ("credentials are per-request input"). Denetim için: spec "Log tool usage for audit purposes" der (SHOULD); protokolde audit trail mesajı yok. İnsan onayı: tool çağrılarında "SHOULD always be a human in the loop".

### 1.4 Bizim tasarımda karşılığı
- Command = `tools/call`. İnsan ve ajan aynı komutu kullanır; MCP bunu sağlamaz (MCP tool'u model-controlled), biz kimliği (actor) komut zarfına koyarız.
- Query = `tools/call` + `outputSchema` + `structuredContent`, ya da `resources/read` + `resources/templates/list` (ör. `qapsule://doc/{id}/study/{sid}?from=&to=`).
- Subscription = `subscriptions/listen` modeli (opt-in filtre, acknowledge, subscriptionId).
- Proposal = MRTR `InputRequiredResult` + `elicitation/create` (form modu) ya da Tasks `input_required`; yanıt `accept/decline/cancel`.
- Capability = `clientCapabilities` + `extensions` + `server/discover`.
- Versiyon = `_meta` protocolVersion + `UnsupportedProtocolVersionError` (bizde: `supported` listesi).
- Hata (kendini düzeltme) = `isError:true` tool hatası modeli.
- Zaman = `ttlMs`/`cacheScope` (tazelik ipucu) kısmen.

### 1.5 AL
1. Per-request `_meta` zarfı (protokol sürümü + capability + client kimliği + trace context). Stateless olması headless/watch/voice için büyük avantaj: yüzey bağlantısı kopsa da hiçbir şey kaybolmaz.
2. İki katmanlı hata: protokol hatası (kod aralığı politikası dahil: ayrılmış aralık) ve alana-özgü, kendini düzeltmeye yönelik hata (`isError`). Biz bunu zenginleştiririz: hata `data` içinde `expected`, `got`, `valid_range`, `suggested_fix`, güncel bağlam (MCP örneğindeki "Current date is ..." gibi).
3. `outputSchema` + `structuredContent` (JSON Schema 2020-12). Tip kaynağımız Rust (`qp-schema`) ve JSON Schema üretilir, bu doğrudan uyumlu.
4. `subscriptions/listen` kalıbı: opt-in filtre, ilk mesaj `acknowledged` (kabul edilen alt küme), her bildirimde `subscriptionId`, graceful close.
5. `accept/decline/cancel` üçlüsü, `requestState` mantığı (sunucu durumsuz kalabilir), `ttlMs`.
6. Task handle (kalıcı, polling + push, `input_required`), uzun süren ajan işleri ve narration için.
7. Uzantı müzakeresi (`capabilities.extensions`, ters-DNS ön ek, `io.` ayrılmış). Biz `qapsule/...` uzantısını aynı şekilde tanımlayabiliriz.
8. Dikkat çeken: `annotations.audience: ["user","assistant"]` ve `priority`. Bir bulgunun kime hitap ettiğini ve önemini işaretlemek için.
9. OpenTelemetry `traceparent` anahtarları: denetim izine ücretsiz bağlanma noktası.

### 1.6 ALMA ve neden
1. MCP'nin **Change/replay** eksikliği: `notifications/resources/updated` içeriksiz ve yeniden oynatma (SSE resume) bilerek kaldırıldı. Bizim sıralı, tekrar oynatılabilir Change akışımız MCP'de karşılıksız; ayrı tasarlanmalı. Kaybolan akış = yeniden `read` varsayımı bizim için yeterli değil (watch/voice anlatımında kayıp olay anlatımı bozar).
2. MCP'nin `resources/updated` için tek granülerlik **URI**'dir. "Bir bulgu/bölge/koşul değişti" tarzı anlamsal abonelik için tek URI yetmez; biz kendi filtre diline ihtiyaç duyarız.
3. Sampling/Roots/Logging (deprecated) alınmaz. Sampling bizim ilkemizle de çelişir: hesap kodunda LLM yok.
4. `form` modu elicitation şeması çok kısıtlı (düz, ilkel). Proposal'ın taşıdığı "ne değişecek" içeriği için yetersiz; Proposal'ı ayrı, tipli bir nesne olarak taşımalı, elicitation'ı yalnızca onay UI'sı için kullanmalı.
5. Tool `annotations` (hint'ler) "güvenilmez" kabul ediliyor; yetki kararını bunlara bağlamayız. Yetki bizim komut katmanında.
6. `clientInfo`/`serverInfo` "güvenlik kararları için kullanılmamalı" (doğrulanmamış). Kimlik/yetki için bunlara güvenmeyiz.
7. Model-controlled tool'lar insana yönelik değil: insan girdisi (touch/zoom) MCP istek-yanıt maliyetiyle taşınmamalı (Input çekirdekte).

### 1.7 Bizim protokolü MCP üstünde taşımak
- **Tool'lar olarak**: `qp.command(...)`, `qp.query(...)`, her biri `inputSchema`/`outputSchema` ile (şema Rust'tan üretilir). Tek bir genel `command` tool'u + tipli birkaç kısa tool, model keşfini kolaylaştırır. Tool listesi deterministik sırada ve önbelleklenebilir (`ttlMs`).
- **Resources olarak**: `qapsule://doc/{id}` (belge anlık görüntüsü), `qapsule://doc/{id}/findings`, şablonlar `.../study/{sid}{?from,to}`. Ajanın bağlama eklemesi için "application-controlled" doğası uygun.
- **Subscription**: `subscriptions/listen` + `resourceSubscriptions` ile "değişti" sinyali; asıl Change verisi için **Change'i resource olarak okutmak**: `qapsule://doc/{id}/changes?since=<revision>`. Yani MCP bildirimi = "revizyon N'e ulaştı", ayrıntı `resources/read` ile (cursor/`since` sayesinde replay bizde, MCP'de değil). Alternatif: listen akışına özel bildirim yöntemi eklemek şema dışı (uzantı gerek).
- **Proposal**: tool çağrısı `InputRequiredResult` + `elicitation/create` ile "onaylıyor musun?" ya da Tasks `input_required`. Uzun bekleyen öneriler için Tasks (kalıcı handle, `ttlMs` = expire).
- **Capability**: `capabilities.extensions["qapsule/surface"] = {kind:"watch", ...}` istemci tarafında; sunucuda `server/discover`.
- **Hata**: `isError:true` + `structuredContent` içinde bizim zengin hata nesnemiz.
- **Kimlik/denetim**: `_meta` içinde bizim anahtarlarımız (`dev.qapsule/actor`, `dev.qapsule/causedBy`), `traceparent` korunur.
- Sınırlama: MCP tek yönlü istek (client -> server) modelli; sunucu -> client "bak şuraya" (Attention) için kalıcı kanal `subscriptions/listen` ile mümkün ama o yalnızca tanımlı bildirim türlerini taşır. Attention agent -> kullanıcı yönü bir bildirim uzantısı ya da MCP Apps `ui/*` ile gerekir.

---

## 2. MCP Apps (SEP-1865, uzantı `io.modelcontextprotocol/ui`)

### 2.1 Kaynaklar
- https://modelcontextprotocol.io/extensions/apps/overview.md
- https://raw.githubusercontent.com/modelcontextprotocol/ext-apps/main/specification/2026-01-26/apps.mdx (spec, "Stable (2026-01-26)", oluşturma 2025-11-21; okuma aracı özetledi, tam metin değil)
- Web araması: https://modelcontextprotocol.io/seps/1865-mcp-apps-interactive-user-interfaces-for-mcp (SEP sayfası; doğrudan okunmadı, arama özeti: SEP Final 2026-01-28, spec 2026-01-26)
- Desteklenen host'lar (overview sayfasından): Claude, Claude Desktop, VS Code GitHub Copilot, Microsoft 365 Copilot, Goose, Postman, MCPJam, Archestra.AI.
- Not: spec metnini bir özetleme aracı üzerinden okudum; alan adlarını resmi `apps.mdx`'ten çapraz doğrulamak gerekir (özellikle `HostContext` alt alanları).

### 2.2 Çözdüğü sorun
Metin yanıtı yetmez; sunucu, sohbet içinde sandbox'lı, etkileşimli HTML arayüzü (grafik, form, dashboard) sunabilsin; arayüz host üzerinden aynı sunucunun tool'larını çağırabilsin.

### 2.3 Model
**UI nasıl teslim edilir.** Tool tanımında `_meta.ui.resourceUri` ("ui://..." URI). Host UI'yı tool çağrılmadan önce önyükleyebilir. Kaynak MIME: `text/html;profile=mcp-app`, `resources/read` ile `text` veya `blob`. Eski düz anahtar `_meta["ui/resourceUri"]` kullanımdan kalkıyor. Tool `_meta.ui.visibility`: `["model","app"]` varsayılan; `"app"`-yalnız tool'lar ajan listesinde görünmez, yalnız uygulama çağırır.

**Kaynak üst verisi** `_meta.ui`: `csp` {`connectDomains`, `resourceDomains`, `frameDomains`, `baseUriDomains`}, `permissions` {`camera`,`microphone`,`geolocation`,`clipboardWrite`}, `domain`, `prefersBorder`. CSP yoksa kısıtlayıcı varsayılan (`connect-src 'none'`).

**Geri konuşma.** Sandbox'lı iframe içinden **postMessage üzerinde JSON-RPC** ("MCP'nin kendi lehçesi"). Yöntemler (birebir):
- Yaşam döngüsü: `ui/initialize` (View -> Host istek), `ui/notifications/initialized`, `ui/resource-teardown` (Host -> View).
- Host -> View bildirimleri: `ui/notifications/tool-input` (en çok bir kez), `ui/notifications/tool-input-partial` (akış), `ui/notifications/tool-result`, `ui/notifications/tool-cancelled`, `ui/notifications/host-context-changed`.
- View -> Host istekleri: `ui/message` (sohbete mesaj ekle), `ui/open-link`, `ui/request-display-mode`, `ui/update-model-context` (her çağrı öncekini ezer), bildirim `ui/notifications/size-changed`.
- Standart MCP yöntemleri view'a açık: `tools/call`, `resources/read`, `notifications/message`, `ping`.
- Web host'lar farklı origin'de sandbox proxy kullanır (`ui/notifications/sandbox-proxy-ready`, `sandbox-resource-ready`).

**HostContext** (host'un view'a bildirdiği yüzey bilgisi): `theme` (`light`/`dark`), `displayMode` (`inline`/`fullscreen`/`pip`), `availableDisplayModes`, `locale` (BCP 47), `timeZone` (IANA), `platform` (`web`/`desktop`/`mobile`), `deviceCapabilities` (`touch`, `hover`), `safeAreaInsets`, `containerDimensions`, `toolInfo`, `styles`, `userAgent`. Host capability'leri: `openLinks`, `serverTools.listChanged`, `serverResources.listChanged`, `logging`, `sandbox`. Host, istemci capability'sinde `extensions["io.modelcontextprotocol/ui"] = {mimeTypes:[...]}` bildirir.

**Güvenlik.** Sandbox, CSP'yi host kurar, host `initialized` almadan view'a hiçbir şey göndermez, host view mesajlarını doğrular/loglar, önceden bildirilmiş kaynaklar numaralanabilir/hashlenebilir.

### 2.4 Bizim tasarımda karşılığı
- Capability (yüzey tanıtımı) <- `HostContext` + host capabilities. En yakın hazır örnek: yüzey cihaz sınıfı, dokunma/hover, güvenli alan, boyut, görüntüleme modu.
- Command: View'dan `tools/call` (app-only tool'lar).
- Attention (kullanıcı -> ajan): `ui/update-model-context` (kullanıcı ne görüyor/yaptı; modele bağlam), `ui/message`.
- Change (ajan -> UI): `ui/notifications/tool-result` / `tool-input-partial` (akış) ve app'in tool polling'i.
- Ana fark: MCP Apps UI'yı **sunucudan HTML olarak gönderir**; biz "çizim çekirdekte (Rust), kabuklar ince" dediğimiz için HTML UI göndermek zorunda değiliz.

### 2.5 AL
1. `ui://` kaynak + tool `_meta.ui.resourceUri` bağlama deseni (tool <-> görünüm ilişkisi, önyükleme).
2. `visibility: ["model","app"]` ayrımı: bazı komutlar yalnız insan arayüzü (app) içindir, ajan listesinde görünmez; veya tersi. Yetki sınırı olarak kullanışlı.
3. `HostContext` alan seti (theme, locale, timeZone, platform, deviceCapabilities, safeAreaInsets, containerDimensions, displayMode) bizim Capability mesajının çekirdeği için iyi başlangıç.
4. `ui/update-model-context` "ezen" semantiği (son durum geçerli): Attention için de uygun (kullanıcının şu anki bakışı = son değer).
5. "Önce host `initialized` almadan göndermeme" ve host'un view mesajlarını doğrulayıp loglama ilkesi: denetim izi.
6. Metin-yalnız fallback zorunluluğu: sunucu "meaningful text content" sağlamalı; ekransız/sesli yüzeyler için aynı fikir.

### 2.6 ALMA ve neden
1. HTML/iframe teslimi: bizim kabuklar ince, çizim Rust çekirdekte; çekirdeğin çıktısını HTML bundle olarak taşımak WASM/yerel çizim hedefimizle çelişir. Yalnızca "MCP Apps host'unda görünmek" istersek ayrı bir uyumluluk katmanı olarak (bkz. 2.7).
2. Ham etkileşimi (zoom/pan) postMessage JSON-RPC ile taşımak: yüksek frekanslı Input için uygun değil; Input çekirdekte kalmalı.
3. `ui/message` (sohbete mesaj ekleme) sohbet merkezli; biz belge merkezliyiz.
4. Spec "Stable" diyor ama düz `ui/resourceUri` anahtarı "GA'dan önce kaldırılacak" der (spec içi tutarsızlık; capability örneğinde `2024-11-05` sürümü geçiyor). Alan adlarını sabitlemeden önce yeniden doğrula.

### 2.7 Üstünde taşıma / gömme
- Qapsule grafik görünümü bir MCP Apps host'unda (Claude, VS Code vb.) göstermek için: tool `show_chart` + `_meta.ui.resourceUri = "ui://qapsule/chart"`; HTML bundle içinde **WASM çekirdek** (CSP `resourceDomains` + `connectDomains` ile kendi veri kaynağına). Bu bundle çekirdeği çalıştırır, app içinden `tools/call` ile **bizim** MCP tool'larına (visibility `["app"]`) konuşur: Input çekirdekte (WASM içinde) yerel kalır, yalnızca Command/Query/Subscription dışarı çıkar.
- Host `HostContext`'i bizim Capability mesajımıza eşlenir (`platform`, `deviceCapabilities`, `containerDimensions`, `displayMode`).
- `ui/update-model-context` Attention (kullanıcı bakışı) için kullanılabilir; yönlendirme (ajan -> kullanıcı bakışı) için `ui/notifications/tool-result` ile yeni viewport durumu gönderilir (standart yok; bizim şema).

---

## 3. A2UI (Agent-to-User Interface, Google)

### 3.1 Kaynaklar
- https://a2ui.org/ (v1.0 etiketli ana sayfa)
- https://a2ui.org/specification/v1.0-a2ui/ (okundu; araç 100.000 karakterde kesti, yaklaşık 16.000 karakter okunmadı; yanıtta "renderer-to-agent" bölümü kısmen eksik)
- https://a2ui.org/specification/v0.9.1-a2ui-extension-specification/ (A2A üstünde taşıma)
- Web araması (doğrudan okunmayanlar): https://a2ui.org/specification/v0.9-a2ui/ , https://a2ui.org/reference/messages/ , https://a2ui.org/client_to_server_actions (fetch 404), https://a2ui.org/specification/v1.0/ (404). Kaynak: github.com/google/A2UI (doğrudan açılmadı).
- Sürüm durumu: ana sayfa "v1.0"; v0.9 ve v0.9.1 de yayında. A2A extension spec'i v0.9.1'de. v1.0 ile v0.9 arasında adlandırma değişiklikleri var (aşağıda).

### 3.2 Çözdüğü sorun
Ajanın, istemcinin kayıtlı ve onaylı **bileşen kataloğu** (Card, Button, DatePicker ...) içinden, çalıştırılabilir kod göndermeden **bildirimsel JSON** ile UI üretmesi; native renderer'lar (React, Lit, Angular, Flutter, SwiftUI, Compose); güvenli ve markaya uygun.

### 3.3 Mesaj modeli (v1.0)
Her mesaj `"version": "1.0"` + tam bir mesaj anahtarı taşır. Terimler "agent-to-renderer"/"renderer-to-agent".

Ajan -> renderer:
- `createSurface` {`surfaceId`, `catalogId`, `sendDataModel` (varsayılan false), `components`, `dataModel`, `metadata`}. (v1.0'da tek mesajda hem bileşen hem veri olabilir; `theme` v1.0'da geçersiz.)
- `updateComponents` {`surfaceId`, `components` (düz liste)} — yüzey önce var olmalı.
- `updateDataModel` {`surfaceId`, `path` (JSON Pointer, varsayılan `/`), `value`} — `null` o yolu siler.
- `deleteSurface` {`surfaceId`}.
- Zarfta ayrıca `callRendererFunction`, `agentFunctionResponse` (çift yönlü fonksiyon RPC; v1.0 ile yeni).

Renderer -> ajan:
- `action` {`name`, `surfaceId`, `sourceComponentId`, `timestamp`, `context` (zorunlu), `userMessage`, `metadata`} (v0.8'de `userAction` idi, v0.9'da `action`).
- `error` {`code`: `VALIDATION_FAILED` | `UNALLOWED_PARENT` | `UNALLOWED_CHILD`, `surfaceId`, `path` (JSON Pointer), `message`}. Spec: ajan bunu LLM'in kendini düzeltmesi için kullanmalı.
- `callAgentFunction`, `rendererFunctionResponse`.

**Bileşen modeli**: düz liste + id referanslı komşuluk listesi (adjacency list), her bileşende `id` ve `component` türü; tam bir `id:"root"` olmalı (gelene kadar güncellemeler tamponlanır). Katalog `allowedParents`/`allowedChildren` ile bileşimi kısıtlar. Bir bileşen `@catalogId` ile farklı katalogdan olabilir.

**Veri bağlama**: `{"@path": "/yol"}` (RFC 6901 JSON Pointer; `/` ile başlayan mutlak, koleksiyon şablonunda görece), fonksiyon çağrısı `{"@call": "ad","args":{}}`, `ChildList` ya statik id dizisi ya da `{componentId, path}` şablonu, `formatString` içinde `${...}`, `@index(offset:n)`. Eylem türleri: `event` (ajana gider) ve `functionCall` (yerel). `requiresUserActivation` olan fonksiyonlar yalnız kullanıcı jestiyle çalışır; `agentOnly` olanlar UI'ya bağlanamaz.

**Capability/katalog müzakeresi**: renderer `rendererCapabilities.supportedCatalogIds` bildirir; ajan `inlineCatalogs` gönderebilir. Aynı sürümde olmalı. Katalogda `protocolVersion` (v1.0'da zorunlu).

**Akış**: JSON nesne akışı (JSONL örneği; mesaj başına bir satır); aktarım gereksinimi: sıralı, güvenilir teslim, çerçeveleme, metadata. Bağlamalar: **AG-UI (standart)**, **A2A (uzantı)**, **MCP (tool çağrıları, tool çıktıları veya resource subscription)**, SSE+JSON-RPC, WebSocket, REST (akışsız).

**A2A üstünde (v0.9.1 uzantı spec'i)**: uzantı URI `https://a2ui.org/a2a-extension/a2ui/v0.9.1` (sürüm URI'de); Agent Card `capabilities.extensions` içinde `params`: `supportedCatalogIds`, `acceptsInlineCatalogs`; yük `DataPart`, `metadata.mimeType = "application/a2ui+json"`, `data` = A2UI mesaj dizisi; istemci yeteneği `message.metadata["a2uiClientCapabilities"]`; UI durum eşitleme `message.metadata["a2uiClientDataModel"]` (surface bazlı, `sendDataModel` açıksa her mesajda). Mesajlar sırayla işlenir, hatalı mesaj loglanıp devam edilir, tek mesaj atomik, renderer tüm liste işlenmeden yeniden çizmemeli.

### 3.4 Bizim tasarımda karşılığı
- Change (çekirdek -> yüzey) = `updateDataModel` (JSON Pointer ile kısmi güncelleme) + `updateComponents` (ağaç). Bizde "doküman değişen kısım".
- Query/Command ters yönde: `action`/`callAgentFunction`.
- Capability = `supportedCatalogIds` + `inlineCatalogs`.
- Hata (kendini düzeltme) = `error` {code,surfaceId,path,message}: yol işaretleyen yapısal hata; bizim hata modeli için iyi örnek.
- Çok yüzeyli yaşam döngüsü = `createSurface`/`deleteSurface` (bizim "görünüm"/"panel").

### 3.5 AL
1. Yapısal doğrulama hatası: `code` + `path` (JSON Pointer) + `message` -> ajan nereyi düzelteceğini bilir. Biz aynı biçimi komut/şema hataları için kullanırız.
2. Katalog kavramı (kayıtlı, kısıtlı bileşim): ajanın yalnız önceden bilinen, güvenli şeyleri ifade edebilmesi. Bizde karşılığı: Command kataloğu (tipli komut ailesi) ve bulgu/çizim türleri kataloğu; "ajan keyfi çizim kodu göndermez".
3. Düz liste + id referansı (adjacency list) ve akışla artımlı kurulum: LLM'in üretmesi kolay, parça parça gönderilebilir.
4. JSON Pointer tabanlı veri güncellemesi, `null` = sil.
5. Veri modeli eşitleme bayrağı (`sendDataModel`) fikri: yüzeyin durumunu ajana ancak istenirse göndermek (gizlilik/trafik).
6. Sürümün uzantı URI'sinde taşınması (A2A) ve katalogun kendi `protocolVersion`'ı: sürüm = kimlik.
7. `requiresUserActivation`: yalnız insan jestiyle çalışan eylemler, `agentOnly`: ajana ait işlevler. Authority modeli için iyi örnek (kim tetikleyebilir).
8. İşleme semantiği: sırayla, hata loglanıp devam; atomiklik tek mesaj düzeyinde; liste bittikten sonra çizim.

### 3.6 ALMA ve neden
1. UI **ağacını** ajandan almak: bizde çizim/etkileşim çekirdekte. Ajan "grafik bileşeni ağacı" kurmaz; Command ile belgeyi değiştirir, çekirdek çizer. A2UI'nin bileşen kataloğu genel amaçlı form/kart UI'ı için; mum grafiği/gösterge çizimi için uygun değil.
2. `action` olayı = yüksek seviyeli UI olayı (düğme tıklandı). Ham dokunma/scroll Input'u için uygun değil.
3. `callAgentFunction`/`callRendererFunction` çift yönlü RPC: bizde Command/Query zaten bunu karşılar; ikinci bir RPC katmanı karmaşıklık.
4. Sürüm oynaklığı: v0.8 -> v0.9 -> v1.0 arasında adlandırma kırıcı değişiklikleri (`userAction` -> `action`, `@` önekleri, envelope adları). Bağımlılığı şemaya sabitlemeyiz; yalnızca fikir alırız veya doğrudan gömme gerekirse sürümü sabitleriz.
5. Veri bağlama ifade dili (`@call`, `formatString`) bir mini dil; "hesap kodunda rastgelelik/LLM yok, deterministik" ilkemizle çelişmez ama hesap ifadesi için ayrı, kendi (Rust) ifade modelimiz var; ikinci bir dil eklemeyiz.

### 3.7 Üstünde taşıma / gömme
- **A2UI içine Qapsule gömmek**: A2UI katalogunda **özel bir bileşen** `QapsuleChart` tanımla (`catalogId` = bizim katalog); bileşenin özellikleri `{documentId, revision, viewportRef}` ve veri modeli bağları (`@path`) ile bağlanır. Renderer bu bileşeni WASM/native çekirdekle çizer. Etkileşim A2UI `action` olarak yalnızca yüksek seviyeli olaylarda (ör. "bulguya git") döner. Böylece A2UI'nin form/kart parçaları ile bizim grafik tek yüzeyde yaşar.
- **Change'i A2UI üstünde taşımak**: `updateDataModel` {path:"/doc/studies/3/params", value} kısmi güncellemeyi taşıyabilir; ancak sıra numarası/revision A2UI'de yok, bizim `metadata`ya (ya da değer içine) koymamız gerekir. Kayıp/yeniden oynatma bizde.
- A2UI'nin MCP bağlaması tanımlı ama ayrıntı okunan sayfada yoktu (DOĞRULANMADI): "tool çağrıları, tool çıktıları veya resource subscription".

---

## 4. AG-UI (CopilotKit, Agent-User Interaction Protocol)

### 4.1 Kaynaklar
- https://docs.ag-ui.com/introduction (sürüm numarası sayfada yok)
- https://docs.ag-ui.com/concepts/events (okunan sürüm: olaylar PascalCase; `THINKING_*` kullanımdan kalktı, 1.0.0'da silinecek)
- https://docs.ag-ui.com/concepts/state
- https://docs.ag-ui.com/concepts/tools
- https://docs.ag-ui.com/concepts/interrupts
- https://docs.ag-ui.com/concepts/architecture (RunAgentInput alanları sayfada tam listelenmedi; capability discovery yok)
- Not: olay adlarını docs `StateSnapshot` gibi PascalCase yazıyor; tel biçimindeki `type` değerleri (ör. `STATE_SNAPSHOT`) bu oturumda doğrulanmadı. Kodlarken `type` sabitlerini repo'dan doğrula.

### 4.2 Çözdüğü sorun
Ajanlar uzun süren, ara çıktı akıtan, yapısal+yapısal olmayan girdili, alt-ajan çağıran, UI'yı nondeterministik denetleyen sistemler; istek/yanıt modeli yetmez. AG-UI: kullanıcı uygulaması ile herhangi bir ajan backend'i arasında **olay tabanlı, çift yönlü** bağlantı (HTTP SSE, ikili protokol; WebSocket/webhook de mümkün). Katman ayrımı: AG-UI = ajan <-> kullanıcı, MCP = ajan <-> araç/veri, A2A = ajan <-> ajan. Üretken UI için A2UI ve MCP Apps destekleniyor.

### 4.3 Olay modeli (docs'tan)
Her olayda `type`, `timestamp`, `rawEvent`, `metadata`; çoğunda opsiyonel `subagentRunId`.
- **Yaşam döngüsü**: `RunStarted` {`threadId`,`runId`,`parentRunId`,`input`}, `RunFinished` {`outcome`: `success` | `interrupt` {`interrupts`}, `result`}, `RunError` {`message`,`code`}, `StepStarted`/`StepFinished` {`stepName`}.
- **Metin**: `TextMessageStart` {`messageId`,`role`}, `TextMessageContent` {`delta`}, `TextMessageEnd`, `TextMessageChunk`.
- **Tool çağrısı**: `ToolCallStart` {`toolCallId`,`toolCallName`,`parentMessageId`}, `ToolCallArgs` {`delta` kısmi JSON}, `ToolCallEnd`, `ToolCallResult` {`messageId`,`toolCallId`,`content`,`role`}, `ToolCallChunk`.
- **Durum**: `StateSnapshot` {`snapshot`} (istemci mevcut durumu **tamamen değiştirir**, birleştirmez); `StateDelta` {`delta`: RFC 6902 JSON Patch dizisi, sırayla, "atomically (all or none)"; tutarsızlık görülürse taze snapshot istenebilir}; `MessagesSnapshot` {`messages`} (rol bazında hep-ya-da-hiç).
- **Aktivite**: `ActivitySnapshot` {`messageId`,`activityType`,`content`,`replace`}, `ActivityDelta` {`patch`}.
- **Akıl yürütme**: `ReasoningStart`, `ReasoningMessageStart/Content/End/Chunk`, `ReasoningEnd`, `ReasoningEncryptedValue`.
- **Özel**: `Raw` {`event`,`source`}, `Custom` {`name`,`value`}. **Alt ajan**: `SubagentStarted/Finished/Error`. Taslak: `MetaEvent` {`metaType`,`payload`}.

**Tool'lar**: frontend tool'ları `RunAgentInput.tools` ile ajana verilir (`name`,`description`, JSON Schema parametreler); backend tool'ları ayrı. Sonuç `role:"tool"` mesajı; hata için `error` alanı ("without it, a tool that failed is indistinguishable from one that succeeded").

**Interrupt / insan döngüsü**: `RunFinished.outcome = {type:"interrupt", interrupts:[...]}`; her interrupt: `id`, `reason` (`tool_call` | `input_required` | `confirmation`, özel olanlar ad alanlı), `message` (insan okur), `toolCallId`, `responseSchema` (devam yükü için JSON Schema), `expiresAt` (ISO-8601 TTL; bayat devam -> `RunError`), `metadata`. Devam: aynı `threadId` üzerinde yeni run, `RunAgentInput.resume[]`: `interruptId`, `status` (`resolved`|`cancelled`), `payload`, `metadata`. Kurallar: tek `resume` dizisi tüm açık interrupt'ları kapsamalı ("Partial resumes are not supported"); bekleyen interrupt yeni girdiyi bloklar; devam tekrar oynatmaya güvenli olmalı; `RunFinished` öncesi `StateSnapshot`+`MessagesSnapshot` yayımlanır (çerçeve checkpoint'ine bağımlılık yok); `editedArgs` "full replacement, not partial merge". Örnek: üç paralel onaydan ikisi `{approved:true}`, biri cancelled.

**İnsan döngüsü ortak durumla**: "Humans can correct or guide the agent by modifying state properties"; ajan bir eylemi durumda önerir, kullanıcı "approve, reject, or modify it before execution".

### 4.4 Bizim tasarımda karşılığı
- Change = `StateDelta` (JSON Patch, sıralı) + `StateSnapshot` (yeniden eşitleme). **En yakın hazır eşleşme.**
- Proposal = interrupt (`reason: confirmation`, `responseSchema`, `expiresAt`, `resume` `resolved`/`cancelled`, `editedArgs`). Accept/reject/expire kavramlarının üçü de var.
- Command (ajan -> UI) = frontend tool; (UI -> ajan) = durum değiştirme veya yeni run girdisi.
- Subscription: akışın kendisi (run boyunca); koşullu abonelik yok.
- Zaman: olay `timestamp`, adımlar `StepStarted/Finished` (anlatım zaman çizelgesine kaba karşılık).
- Hata: `RunError` {`message`,`code`} (zayıf; yapısal düzeltme ipucu yok), tool `error` alanı.

### 4.5 AL
1. Snapshot + delta ikilisi, "tutarsızlıkta snapshot iste" kuralı, deltaların atomik (hep ya da hiç) uygulanması.
2. Interrupt/resume modeli: `id` (idempotensi/denetim), `reason`, `message` (evrensel fallback UI: ekransız/sesli yüzeyde okunur), `responseSchema`, `expiresAt`, `status: resolved|cancelled`, "resume güvenle tekrar oynatılabilir", `editedArgs` = tam yerine koyma. Proposal'ın doğrudan şablonu.
3. Metin akışının `Start/Content/End` kalıbı ve kısmi tool argüman akışı (`ToolCallArgs` delta): sesli anlatım ve canlı komut oluşumu.
4. Aktivite olayları (`ActivitySnapshot/Delta`): sohbet geçmişine girmeyen geçici durum ("analiz ediliyor") için; Attention yönlendirmesi benzeri geçici sinyaller için fikir.
5. `Custom`/`Raw` kaçış kapakları ve ara katman (middleware) fikri: yeni olay türü eklemeden uzantı.
6. Alt ajan atfı (`subagentRunId`): denetim izinde "kim yaptı".

### 4.6 ALMA ve neden
1. Sohbet-merkezli yapı (`messages`, `role`, thread): bizim birincil nesne belge; mesaj geçmişi değil.
2. `StateDelta` yalnızca JSON Patch: **revision/sequence numarası yok**, "tutarsızlık görülürse" tespiti istemciye bırakılmış. Bizim sıralı ve tekrar oynatılabilir Change için monoton `revision` ve `base_revision` gerekir; AG-UI bunu vermiyor.
3. JSON Patch'in yol-tabanlı dizi indeksleri (`/series/3/...`) eşzamanlı yazmada kırılgan: kimlik tabanlı adresleme (stabil ID) tercih edilmeli.
4. Interrupt "bekleyen girdi bloklar" kuralı (run bitip yeni run başlıyor): canlı belgede eşzamanlı birden çok Proposal'ın yaşaması gerekir; AG-UI'nin "tüm açık interrupt'ları tek `resume` ile kapat, kısmi yok" kuralı bizim modelimiz için fazla katı.
5. `Reasoning*` olayları ve şifreli değerler: ajan düşünce akışı bizim çekirdek ilgi alanı değil.
6. `RunError` zayıf (yalnızca mesaj+kod); bizim hata modeli zengin olmalı.
7. Sürüm numarası ve kararlılık belirsiz (draft olaylar, kullanımdan kalkan `THINKING_*`), bağımlılık riski.

### 4.7 Üstünde taşıma
- AG-UI, A2UI'nin standart taşıma katmanı (A2UI spec: "AG-UI: the standard binding"). Bizim protokol AG-UI üstünde şöyle taşınır: `StateSnapshot`/`StateDelta` = belge durumu; `Custom` olayları = Attention/Input özetleri; interrupt = Proposal. Fakat AG-UI ajan <-> UI arasındadır; çekirdek (engine) ile UI arasında değil. Bizim motor-UI protokolü için gerekli değil, "bir ajan çerçevesi UI'ya konuşurken" köprü olarak anlamlı (CopilotKit kullanan bir host'ta Qapsule gömmek).

---

## 5. A2A (Agent2Agent) — kısa

### 5.1 Kaynaklar
- https://a2a-protocol.org/latest/specification/ (3 parça okundu: 0-100k, 100k-200k, 200k-257k karakter; sürüm **1.0.0**, önceki 0.3.0/0.2.6/0.1.0)
- Not: A2A okuma aracı özetlerinde `message/send` gibi eski adlar doğrulanmadı; JSON-RPC bağlamasında güncel adlar aşağıdaki gibi.

### 5.2 Çözdüğü sorun
Birbirinden bağımsız (opak) ajanların birlikte çalışması: keşif, görev devri, uzun süren iş, çok turlu iletişim. (MCP = araç/veri; A2A = ajan-ajan; spec Ek B'de "birlikte çalışırlar" der.)

### 5.3 Model
- Kavramlar: **Agent Card** (JSON, kimlik/yetenek/beceri/uç nokta/kimlik doğrulama; `.well-known/agent-card.json`), **Message** (`role` user|agent, `Part`'lar), **Task** (sunucu üretimli `id`, `contextId` ile konuşma gruplama), **Part** (tam biri: `text`|`raw`|`url`|`data`; `mediaType`, `filename`, `metadata`), **Artifact** (`artifactId`, `name`, `description`, `parts`, `metadata`, `extensions`; "sonuçlar Mesajda değil Artifact'ta").
- Görev durumları: terminal `COMPLETED`, `FAILED`, `CANCELED`, `REJECTED`; kesilmiş `INPUT_REQUIRED`, `AUTH_REQUIRED`; diğer `SUBMITTED`, `WORKING`, `UNSPECIFIED`.
- JSON-RPC yöntemleri (Bölüm 9): `SendMessage` (Task veya Message döner; varsayılan bloklar, `returnImmediately` ile hemen), `SendStreamingMessage`, `GetTask`, `ListTasks` (cursor sayfalama), `CancelTask`, `SubscribeToTask` (ilk olay güncel Task olmalı), `CreateTaskPushNotificationConfig`/`Get`/`List`/`Delete`, `GetExtendedAgentCard`. Ayrıca gRPC ve HTTP+JSON bağlamaları (`POST /message:send` vb.).
- Akış olayları: `StreamResponse` tam biri: `task`|`message`|`statusUpdate` (`TaskStatusUpdateEvent`)|`artifactUpdate` (`TaskArtifactUpdateEvent`: `append`, `lastChunk`). Olaylar üretim sırasıyla; akış terminal durumda kapanır; her akış aynı olayları alır.
- Push bildirim: sunucu istemcinin webhook'una POST; `url`,`token`,`authentication`; `capabilities.pushNotifications` gerekli; SSRF korumaları.
- Agent Card alanları: `name`, `description`, `supportedInterfaces` (`url`,`protocolBinding`,`tenant`,`protocolVersion`), `provider`, `version`, `documentationUrl`, `capabilities` (`streaming`, `pushNotifications`, `extensions`, `extendedAgentCard`), `securitySchemes`, `securityRequirements`, `defaultInputModes`, `defaultOutputModes`, `skills` (`id`,`name`,`description`,`tags`,`examples`,`inputModes`,`outputModes`), `signatures` (kart imzalama), `iconUrl`. Kimliği doğrulanmış "extended card" daha fazla beceri açabilir.
- Versiyon: `A2A-Version` header'ı (Major.Minor), desteklenmezse `VersionNotSupportedError` (-32009); uzantılar `A2A-Extensions` header (virgülle ayrılmış URI'ler), `required:true` uzantı desteklenmezse `ExtensionSupportRequiredError` (-32008). Çok kiracılık `tenant`.
- Hata kodları: `TaskNotFoundError` -32001, `TaskNotCancelableError` -32002, `PushNotificationNotSupportedError` -32003, `UnsupportedOperationError` -32004, `ContentTypeNotSupportedError` -32005, `InvalidAgentResponseError` -32006, `ExtendedAgentCardNotConfiguredError` -32007, `ExtensionSupportRequiredError` -32008, `VersionNotSupportedError` -32009.
- Güvenlik: her işlemde yetki denetimi, sonuçları çağıranın yetki sınırına daraltma, yetkisiz kaynağın varlığını sızdırmama; görev içi yetki `AUTH_REQUIRED`.

### 5.4 Bizim tasarımda karşılığı
- Capability = Agent Card (yüzeyin kendini tanıtması: `skills`, `defaultInputModes/OutputModes`, `capabilities`). `inputModes/outputModes` mime listeleri cihaz/ekran sınıfı için ilginç.
- Proposal/uzun iş = Task + `INPUT_REQUIRED`/`AUTH_REQUIRED`.
- Subscription = `SubscribeToTask` + push config.
- Artifact = bir "bulgu/anlatım çıktısı" olarak (ör. rapor), `append`/`lastChunk` ile akış.

### 5.5 AL
1. Agent Card fikri (iyi bilinen yerde, imzalanabilir, genişletilmiş kart kimlik doğrulayana daha fazlasını gösterir): bizde "Capability" mesajı, yüzeyin ve motorun karşılıklı tanıtımı.
2. `contextId` ile ilişkili işleri gruplama: bizde belge/oturum/anlatım bağlamı.
3. Yetkisiz kaynağın varlığını sızdırmama kuralı (authority modeli).
4. Kodlu hata listesi + sürüm/uzantı hataları (`VersionNotSupported`, `ExtensionSupportRequired`).
5. "Sonuçlar Artifact'ta, Message iletişim için" ayrımı: Query yanıtı ile açıklama/anlatım ayrımı.

### 5.6 ALMA ve neden
1. A2A ajan <-> ajan: opak ajan varsayımı (iç durumu paylaşmaz). Bizde belge herkese açık ortak durum; bu varsayım ters.
2. Görev merkezli yaşam döngüsü (submitted -> completed): canlı belge sürekli, "tamamlanan görev" yok.
3. Push webhook (sunucu -> sunucu HTTP) mobil/saat/ekransız yüzeyler için uygun taşıma değil.
4. Sürüm belirsizliği: spec içinde v1.0 kırıcı değişiklikleri (`kind` ayırıcı yerine üye-adı sarmalayıcıları) ve örneklerde hâlâ `A2A-Version: 0.3`; tutarsızlık, hedef sürümü seçmeden önce doğrula.

### 5.7 Üstünde taşıma
- A2UI, A2A üstünde taşınır (extension URI + `DataPart`, `application/a2ui+json`). Bizim protokolü doğrudan A2A `DataPart`'ı olarak (`application/vnd.qapsule+json`) taşımak mümkün, ancak A2A bizim için yalnızca "başka ajanlarla işbirliği" köprüsü.

---

## 6. Bizim protokolün taşıma/gömme analizi için sentez (sonraki analiz girdisi)

1. **MCP üstü (en doğal, birincil)**: Command/Query = tool'lar (`outputSchema`), belge/bulgu görünümleri = resource + template, abonelik = `subscriptions/listen` ("revizyon N") + replay'i `resources/read ...changes?since=N` ile biz sağlarız (MCP replay'i bilerek kaldırdı), Proposal = MRTR/elicitation ya da Tasks, Capability = `clientCapabilities.extensions`, hata = `isError` + yapısal veri. Eksikler: Change akışının sıra/replay garantisi, Attention (özellikle ajan -> kullanıcı), Input, koşullu/anlamsal abonelik, anlatım zaman çizelgesi. Bunlar `qapsule/*` uzantısı olarak tanımlanmalı (uzantı müzakeresi mekanizması hazır).
2. **MCP Apps üstü (görünürlük için)**: Qapsule'ı Claude/VS Code gibi host'larda görsel olarak göstermek istediğimizde `ui://qapsule/chart` + WASM çekirdek; app içinden yalnızca `app`-visibility tool'ları ile belge protokolüne konuşulur; HostContext -> bizim Capability'ye eşlenir; Input WASM içinde yerel. Bu, ayrı protokol değil bir **teslim kanalı**.
3. **A2UI içine gömme (genel yüzeylerde)**: özel bileşen `QapsuleChart` + veri modeli bağları; çevresindeki form/kart A2UI olur. A2UI katalogu "ajanın kurduğu UI" için; bizim görünüm yine çekirdek çizer. Taşıma: AG-UI (standart) veya A2A uzantısı veya MCP.
4. **AG-UI**: ajan çerçevesi (ör. CopilotKit) ile bir web kabuğu arasında köprü; Change = `StateDelta`, Proposal = interrupt. Motor-yüzey protokolünün yerine geçmez.
5. **Ortak desenler (tüm protokollerde tekrarlanan, bizim şemaya almaya değer)**: (a) yüzey kendini tanıtır (HostContext/Agent Card/supportedCatalogIds/clientCapabilities); (b) snapshot + kısmi güncelleme; (c) yapısal doğrulama hatası yol işaretiyle; (d) üçlü yanıt `accept/decline/cancel` + TTL; (e) kalıcı handle (taskId/interruptId/requestState); (f) uzantı/sürüm müzakeresi her istekte; (g) hassas olanı kanal dışına alma (URL elicitation).
6. **Hiçbir yerde olmayanlar (bizim özgün protokol alanı)**: monoton revision'lı, replay edilebilir Change log; Attention (iki yönlü: kullanıcının baktığı yer + ajanın yönlendirmesi, sıralı); anlamsal Subscription (bulgu/bölge/koşul); ham Input'un çekirdekte kalması; yetki + denetim izi mesajı (MCP sadece "SHOULD log" der, A2A sadece yetki denetimi); anlatım zaman çizelgesi.

## 7. Açık noktalar / doğrulanmadı listesi
- MCP `server/discover` sonuç şeması ve sampling sayfaları okunmadı; tool `annotations` alan adları 2026-07-28 sayfasında listelenmedi.
- MCP Apps spec metni bir özetleme aracı üzerinden okundu; alan adları resmî `apps.mdx` ile çapraz kontrol edilmeli. Spec "Stable" diyor ama düz `ui/resourceUri` "GA'dan önce kaldırılacak" diyor.
- A2UI v1.0 sayfasının son ~16.000 karakteri okunmadı; `a2ui.org/client_to_server_actions` ve `/specification/v1.0/` 404 verdi; A2UI-MCP bağlamasının ayrıntısı doğrulanmadı.
- AG-UI olay `type` sabitleri (ör. `STATE_SNAPSHOT`) ve sürüm numarası doğrulanmadı; `RunAgentInput` alan listesi sayfada yoktu.
- A2A: v1.0 kırıcı değişiklikleri ve örneklerdeki `A2A-Version: 0.3` tutarsızlığı; Task/TaskStatus tablosu ilgili parçada okunmadı.
