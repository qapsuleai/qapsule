# R5B / T1 — Araştırma: ajan bağlantısı (MCP), araç tasarımı, güvenlik, doğrulama

Erişim tarihi (tüm kaynaklar): 2026-11-21. Bu belge `rounds/RD1/EMBEDDING.md` §1'in
güncellemesidir; orada yazılanı tekrarlamaz. "Doğrulanamadı" = birincil kaynakta
bulunamadı ya da yalnız dolaylı kaynakta görüldü.

## Özet (10 satır)

1. En güncel MCP sürümü **2026-07-28**; durumsuz (oturum ve `initialize` yok), her istek sürümü ve yetenekleri `_meta` ile taşır.
2. Olay yolu değişti: `resources/subscribe` ve HTTP GET akışı kalktı; yerine `subscriptions/listen` geldi. Bu akışta yalnız liste değişimi ve `notifications/resources/updated` gider.
3. **`notifications/message` bir istekle ilişkili ve istek `logLevel` verdiyse gider; dinleme akışında gitmez; Logging özelliği kullanımdan kaldırıldı (deprecated).** `qp-mcp`'nin olayları `notifications/message` ile itmesi 2026-07-28'de uyumsuz; düzeltilmeli.
4. Streamable HTTP: `Origin` doğrulaması ZORUNLU (geçersizse 403), yerelde yalnız `127.0.0.1`'e bağlanma ve kimlik doğrulama ÖNERİLİR. Yerel sunucu için resmi öneri: `stdio` ya da HTTP'de yetki belirteci / Unix soketi.
5. Araç ek açıklamaları (`readOnlyHint` vb.) istemci için **güvenilmez ipucudur**; güvenlik buna dayanamaz.
6. Claude Code `.mcp.json` içinde `${VAR}` ve `${VAR:-varsayılan}` genişletmesi `command`, `args`, `env`, `url`, `headers` alanlarında çalışır.
7. Ajan araç tasarımı: az sayıda görev düzeyi araç; `response_format`; sayfalama; anlamlı ad. Araç sayısı arttıkça seçim doğruluğu düşer (ölçümler bölüm 3'te, çoğu 2025 ön baskısı).
8. Kod modu (Anthropic, Cloudflare) büyük araç kümelerinde belirteç kazandırır ama güvenli bir kum havuzu ister; Qapsule için ikincil seçenek.
9. Dolaylı talimat saldırısına kesin çözüm yok; sağlam olan mimari kuraldır: yetkiyi veriden ayır, "Rule of Two" (Meta), CaMeL, Dual LLM. Qapsule'ın "ajan kendi önerisini onaylayamaz" kuralı bu sınıfa girer ve motor tarafında zorlanmalı.
10. Yorumdaki sayıları kaynağa karşı deterministik denetleme yöntemleri var (tam eşleşme + biçim normalizasyonu + hesap yeniden doğrulama; "Proof-Carrying Numbers"); tavsiye dili için Türkiye'de SPK çerçevesi (genel tavsiye ile kişiye özel danışmanlık ayrımı) esas alınmalı; yapay zekâya özgü SPK kuralı doğrulanamadı.

---

## 1. Güncel MCP şartnamesi

### 1.1 Sürüm ve temel değişiklikler

Bulgular:
- En son yayımlanmış sürüm `2026-07-28` (`/specification/latest` bu sürüme çözülüyor). Önceki: `2025-11-25`.
- Durumsuz: `initialize` / `notifications/initialized` yok; her istek `_meta` içinde
  `io.modelcontextprotocol/protocolVersion`, `.../clientCapabilities` (ve önerilen `.../clientInfo`) taşır.
  Sunucu `server/discover` UYGULAMAK ZORUNDA (sürümler, yetenekler, kimlik).
- Protokol düzeyinde oturum ve `Mcp-Session-Id` başlığı kaldırıldı. Çağrılar arası durum için sunucunun ürettiği
  açık **tutamaç** (handle) araç argümanı olarak taşınır. (Qapsule için: doküman/oturum kimliği zaten araç argümanı.)
- Her sonuçta zorunlu `resultType` (`complete` ya da `input_required`).
- `ping`, `logging/setLevel`, `notifications/roots/list_changed` kaldırıldı. Günlük düzeyi istek başına `_meta["io.modelcontextprotocol/logLevel"]`.
- Kullanımdan kaldırılanlar (en az 12 ay pencere): Roots, Sampling, Logging; HTTP+SSE taşıması; Dinamik İstemci Kaydı (DCR).
- Görevler (Tasks) çekirdekten çıktı, resmî uzantı `io.modelcontextprotocol/tasks`. MCP Apps ve "Skills over MCP" uzantı olarak anılıyor.
- `tools/list`, `resources/list`, `resources/read` sonuçlarında zorunlu `ttlMs` ve `cacheScope`; araçlar belirleyici sırada dönmeli (önbellek ve istem önbelleği için).
- Hata kodları: `-32020` HeaderMismatch, `-32021` MissingRequiredClientCapability, `-32022` UnsupportedProtocolVersion; kaynak yok = `-32602`.

Kaynaklar:
- https://modelcontextprotocol.io/specification/latest
- https://modelcontextprotocol.io/specification/2026-07-28/changelog.md

### 1.2 Araçlar: açıklamalar, çıktı şeması, hata

Bulgular:
- `ToolAnnotations` alanları ve varsayılanları (şema kaynağından `schema/2026-07-28/schema.ts`):
  `title`; `readOnlyHint` (varsayılan `false`); `destructiveHint` (varsayılan `true`, yalnız `readOnlyHint==false` iken anlamlı);
  `idempotentHint` (varsayılan `false`); `openWorldHint` (varsayılan `true`). Varsayılanlar kötümser: ek açıklama yoksa araç "yazar, yıkıcı olabilir, dış dünyaya açık" sayılır.
- Şartname: istemci, ek açıklamaları **güvenilmez** saymalı (güvenilir sunucudan gelmedikçe). Yani ip ucu, yetki değildir.
- `outputSchema` verilirse sunucu şemaya uyan `structuredContent` DÖNMELİ; istemci doğrulamalı. `structuredContent` artık herhangi bir JSON değeri olabilir. Geriye uyum için aynı JSON metin bloğunda da dönmeli (`qp-mcp` bunu zaten yapıyor).
- Hata ikiye ayrılır: protokol hatası (JSON-RPC; bilinmeyen araç, bozuk istek) ve araç yürütme hatası (`isError: true`; modelin kendini düzeltmesi için). İstemci, ikincisini modele vermeli. (`qp-mcp`'nin `error` → `isError` eşlemesi doğru.)
- Araç listesi bağlantıya göre değişemez; yalnız sunucuya sunulan yetkiye (kapsamlara) göre değişebilir.
- Güvenlik bölümü: sunucu girdiyi doğrulamalı, erişim denetimi uygulamalı, hız sınırlamalı, çıktıyı arındırmalı; istemci hassas işlemde onay istemeli, araç girdisini kullanıcıya göstermeli, sonucu modele vermeden doğrulamalı. Araç çağrısında "insan döngüde" olması ve reddedebilmesi (SHOULD).
- `x-mcp-header`: araç parametresini `Mcp-Param-*` başlığına yansıtır (ara katman için). Qapsule'a gerek yok; hassas değer için kullanılmamalı.

Kaynaklar:
- https://modelcontextprotocol.io/specification/2026-07-28/server/tools
- https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/schema/2026-07-28/schema.ts (satır 1916 civarı, `ToolAnnotations`)

### 1.3 Kaynaklar ve abonelik, bildirim yolu

Bulgular:
- `resources` yeteneği: `listChanged`, `subscribe`. Kaynak okuma `resources/read`; URI şablonları (`resources/templates/list`, RFC 6570) var. Özel URI şeması serbest (RFC 3986'ya uygun). `qapsule://doc/<id>/log?after=<seq>` biçimi buna uyuyor.
- `resources/subscribe`/`unsubscribe` **kaldırıldı**. Yerine `subscriptions/listen`: istemci filtre yollar
  (`toolsListChanged`, `promptsListChanged`, `resourcesListChanged`, `resourceSubscriptions: [uri…]`); sunucu önce
  `notifications/subscriptions/acknowledged` gönderir (kabul ettiği alt küme), sonra yalnız istenen türleri yollar. Her bildirim `_meta["io.modelcontextprotocol/subscriptionId"]` taşır (= listen isteğinin id'si).
- HTTP'de dinleme akışı, `subscriptions/listen` POST yanıtının açık kalan SSE akışıdır. Sunucu yeniden başlarsa istemci yeniden dinlemeli; stdio'da bağlantı yenilenince istemci `subscriptions/listen`'i yeniden göndermeli. **SSE yeniden oynatma (`Last-Event-ID`) yok**; kopan akışta olay kaybolur. Bu, bizim `seq` + `log` sorgusu tasarımının neden doğru olduğunun şartname düzeyinde kanıtıdır.
- `notifications/resources/updated` yalnızca URI taşır (içerik yok); istemci `resources/read` ile okur. Yani "değişti" sinyali + okuma modeli.
- **`notifications/message` (Logging):** yalnızca ilgili isteğin yanıt akışında gider ve yalnız istek `logLevel` verdiyse; dinleme akışında GİTMEZ; özellik "Deprecated". Sonuç: `qp-mcp`'nin abonelik olaylarını `notifications/message` ile yollaması (RD1 §1.1'de "olaylar MCP bildirimi olarak") 2026-07-28 istemcileri için yanlış kanaldır. Eski sürümlerde (2025-11-25) çalışır; modern uçta olay taşıyıcısı `notifications/resources/updated` + okuma olmalı.
- Claude Code'un `resources/updated` ya da `subscriptions/listen` ile modeli uyandırıp uyandırmadığı: **doğrulanamadı** (belgede yalnız `list_changed` ve `@sunucu:protokol://yol` ile kaynak anma anlatılıyor). Kaynak yoluyla uyarı yerine, ajanın `query`/`log` ile yoklamasına güvenilmeli; abonelik "ek rahatlık" sayılmalı.

Kaynaklar:
- https://modelcontextprotocol.io/specification/2026-07-28/basic/patterns/subscriptions
- https://modelcontextprotocol.io/specification/2026-07-28/server/resources.md
- https://modelcontextprotocol.io/specification/2026-07-28/server/utilities/logging (sürüm notları `changelog.md` 4. ve 5. madde)
- https://code.claude.com/docs/en/mcp (list_changed, kaynak anma bölümleri)

### 1.4 Taşımalar: Streamable HTTP ve stdio

Streamable HTTP (2026-07-28):
- Tek uç nokta, her mesaj ayrı POST. Yanıt JSON ya da isteğe bağlı SSE. GET ve DELETE uç noktada yok (eski istemciye `405`). `Mcp-Session-Id` yok sayılır, üretilmez.
- Zorunlu istek başlıkları: `MCP-Protocol-Version` (gövdedeki `_meta` sürümüyle aynı, değilse `400` + `HeaderMismatch`), `Mcp-Method`, `Mcp-Name` (`tools/call`, `resources/read`, `prompts/get`). İstemci `Accept: application/json, text/event-stream` yollar.
- **Güvenlik (MUST/SHOULD):** (1) sunucu tüm bağlantılarda `Origin` başlığını DOĞRULAMALI, geçersizse `403`; (2) yerelde yalnız localhost'a (127.0.0.1) bağlanmalı, 0.0.0.0'a değil; (3) tüm bağlantılar için uygun kimlik doğrulama uygulamalı. Gerekçe: DNS yeniden bağlama (rebinding) ile uzak sitelerden yerel sunucuya erişim.
- `X-Accel-Buffering: no` ve SSE yorum satırı (`:`) ile canlı tutma önerilir.

stdio:
- İstemci alt süreç olarak başlatır; stdin/stdout'ta satır sınırlı JSON-RPC; stdout'a başka bir şey yazılmaz (günlük stderr'e). İptal `notifications/cancelled`. Kapanış: stdin kapanınca çıkmak. Geri uyum yoklaması: önce `server/discover`, olmazsa `initialize` ("legacy").

Yetkilendirme:
- İsteğe bağlı. HTTP için OAuth 2.1 (kaynak sunucu rolü, RFC 9728 korunan kaynak üstverisi, RFC 8707 `resource`, belirteç hedef kitle doğrulaması; belirteç sorgu dizesinde olmaz; `Authorization: Bearer`). **stdio uygulamaları bu şartnameyi İZLEMEMELİ, kimlik bilgisini ortamdan almalı.**
- Güvenlik en iyi uygulamaları, yerel sunucular için: `stdio` kullan (erişimi yalnız istemciyle sınırlar); HTTP ise yetki belirteci iste ya da erişimi kısıtlı Unix alan soketi/IPC kullan; DNS rebinding ile açık unutulmuş yerel sunucu açıkça tehdit olarak sayılıyor. Durum tutamacı (handle) sahipliği her çağrıda denetlenmeli, tahmin edilemez olmalı.

Kaynaklar:
- https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http.md
- https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio.md
- https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization.md
- https://modelcontextprotocol.io/docs/tutorials/security/security_best_practices (bölüm "Local MCP Server Compromise", "State Handle Hijacking")

### 1.5 Elicitation ve insan onayı

Bulgular:
- 2026-07-28'de sunucu istemciye kendi isteğini gönderemez; "çok turlu istek" (MRTR) ile `tools/call` yanıtı `resultType: "input_required"` döner, içinde `inputRequests` (`elicitation/create`, kipler `form` ve `url`); istemci kullanıcıdan alıp aynı isteği `inputResponses` + `requestState` ile yeniden yollar (yeni JSON-RPC id).
- Yanıt eylemleri: `accept` / `decline` / `cancel`. İstemci hangi sunucunun sorduğunu açıkça göstermeli; reddetme/iptal seçeneği olmalı. Form kipinde parola, anahtar vb. istenmez (URL kipi kullanılır).
- Claude Code elicitation'ı destekler: terminalde diyalog gösterir; 2026-07-28 bağlantılarında `{form:{}, url:{}}` bildirir; eski sürümlerde de v2.1.287+ aynısı; bir `Elicitation` kancası ile otomatik yanıtlanabilir (yani yapılandırmayla atlanabilir).
- Kullanılabilir mi? Teknik olarak evet, ama **Qapsule'ın insan onayı için ana mekanizma olmamalı**: (a) elicitation aynı araç çağrısı içinde, çağıran istemci üzerinden yanıtlanır; kullanıcı ile ajan aynı istemcide ise ajanın yapılandırması (kanca) yanıtı otomatikleştirebilir; (b) Qapsule'da onay, dokümanın içindeki "öneri" nesnesinde ve **başka bir kişi** tarafından, motorun komut yolunda verilir. Elicitation en fazla kabukta kolaylık (ör. "öneri bekliyor, kullanıcıya göster") olabilir.

Kaynaklar:
- https://modelcontextprotocol.io/specification/2026-07-28/client/elicitation.md
- https://modelcontextprotocol.io/specification/2026-07-28/server/tools (InputRequiredResult)
- https://code.claude.com/docs/en/mcp ("Respond to MCP elicitation requests")

### Qapsule için sonuç (bölüm 1)

- S1.1 Modern uçta olay taşıyıcısı: `subscriptions/listen` + `notifications/resources/updated` (URI = `qapsule://doc/<id>/log`) ve sinyalden sonra `resources/read ?after=<seq>`. `notifications/message` yalnız eski sürüm uyumluluğu (2025-11-25/2025-06-18) için kalsın; modern uçta yalnızca istek `logLevel` verirse ve yalnız o isteğin akışında kullanılsın. (Bu, T-görevlerinde `qp-mcp` düzeltmesi olarak planlanmalı; bu araştırma kod değiştirmez.)
- S1.2 Olay kaybı kabul edilen bir durum: her bildirimde `seq` yok; yalnız "N'e ulaşıldı" sinyali ve `log` sorgusu ayrıntıyı verir. Şartname bunu güçlendiriyor (yeniden oynatma yok).
- S1.3 Her araç için ek açıklama yaz (varsayılanlar kötümser olduğu için okuma araçlarına `readOnlyHint: true`, `openWorldHint: false`, `destructiveHint: false` açıkça verilmeli) ama **güvenliği ek açıklamaya bağlama**: salt-okunur/yazar ayrımı sunucuda, ajanın ilke (principal) yetkisiyle zorlanmalı.
- S1.4 `outputSchema` + `structuredContent` + metin kopyası şeması sürdürülsün; `ttlMs`/`cacheScope` (okuma araçları için `private`) ve araç listesinin belirleyici sırası eklensin.
- S1.5 HTTP varsayılanları: yalnız `127.0.0.1`/`::1`'e bağlan; `Origin` varsa izin listesiyle (varsayılan boş = yalnız `Origin` yok olan istekler) doğrula, yoksa `403`; `Host` başlığını da doğrula (DNS rebinding için `Origin` yokken ikinci savunma; bu madde şartnamede değil, benim önerim); sabit zamanlı karşılaştırmayla `Authorization: Bearer` belirteci iste, belirteç ortam değişkeninden okunsun, günlüğe yazılmasın; sorgu dizesinde asla; GET/DELETE `405`; `0.0.0.0` yalnız açık bayrakla.
- S1.6 Varsayılan taşıma `stdio` (yetkilendirme şartnamesi: ortamdan kimlik); HTTP isteğe bağlı ve bu varsayılanlarla.
- S1.7 OAuth Qapsule'ın yerel kullanımında gereksiz; HTTP belirteci yeterli. Uzak/çok kullanıcılı sunum gündeme gelirse RFC 9728 + `resource` + audience doğrulaması gerekir (kapsam dışı notu).
- S1.8 İnsan onayı elicitation'a bağlanmasın; `op_accept_proposal` komutu motorda, başka ilkeden gelen çağrıyla ve kimlik denetimiyle yürüsün (bölüm 4).

---

## 2. Claude Code ve Claude masaüstünde MCP yapılandırması

### 2.1 Claude Code

Bulgular (resmî belge):
- Kapsamlar: `local` (varsayılan, `~/.claude.json`, yalnız o proje), `project` (proje kökünde `.mcp.json`, sürüm kontrolünde paylaşılır), `user` (`~/.claude.json`, tüm projeler). Öncelik: local → project → user → eklenti → claude.ai bağlayıcıları; girdi alan alan birleşmez, bütün giriş seçilir.
- `.mcp.json` biçimi: `{"mcpServers": { "<ad>": {...} }}`. Taşıma türleri: `"type": "http"` (önerilen; `streamable-http` takma ad), `"sse"` (deprecated), `"ws"`, stdio (`type` yok; `command`, `args`, `env`). **`url` var ama `type` yoksa yapılandırma hatasıdır** (stdio sanılır). Sunucu başına `"timeout"` (ms) alanı var.
- Ortam değişkeni genişletmesi: `${VAR}` ve `${VAR:-varsayılan}`; alanlar: **`command`, `args`, `env`, `url`, `headers`**. Tanımsız ve varsayılansız ise yapılandırma yüklenir, uyarı verilir ve `${VAR}` metni olduğu gibi kullanılır. Boş dizge "tanımlı" sayılır (varsayılana düşmez). Uzak `url`/`headers` içinde `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` gibi kimlik bilgisi değişkenleri boş okunur; `ANTHROPIC_BASE_URL` genişler.
- `headersHelper`: üstbilgiyi her bağlantıda çalıştırılan bir komuttan almak için; proje `.mcp.json`'daki helper yalnız klasör güvenildikten sonra çalışır, kimlik değişkenleri verilmeden. (Yalnız bahsi; Qapsule için statik `headers` yeterli.)
- Proje kapsamlı `.mcp.json` sunucuları etkileşimli oturumda onay ister (`claude mcp reset-project-choices` ile sıfırlanır); `claude -p` ve Agent SDK oturumlarında sorulmadan yüklenir (`--strict-mcp-config`, `disabledMcpjsonServers` ile engellenir). **Sonuç: `.mcp.json`'a bir komut konursa betik ortamında onaysız çalışabilir; depodaki `.mcp.json` yalnız bizim betiğimizi göstermeli.**
- Komut biçimi: `claude mcp add --transport http <ad> <url> [--header "Authorization: Bearer ..."] [--scope local|project|user]`; stdio: `claude mcp add --env KEY=değer --transport stdio <ad> -- <komut> <argümanlar>` (`--env`'den hemen sonra ad gelirse ad çift sanılır; `--` sonrası olduğu gibi geçer). Ayrıca `claude mcp add-json <ad> '<json>'` (sarmalayıcı anahtar olmadan).
- Çıktı sınırları: varsayılan en çok 25.000 belirteç (`MAX_MCP_OUTPUT_TOKENS`), 10.000 üstünde uyarı; 50.000 karakter üstü başarılı metin sonucu dosyaya kaydedilir; araç, `tools/list` girdisinde `_meta["anthropic/maxResultSizeChars"]` ile kendi eşiğini yükseltebilir.
- Araç araması varsayılan açık: çok sayıda MCP aracı varsa bağlama hepsi yüklenmez, aranır (`ENABLE_TOOL_SEARCH`); bir sunucu için `alwaysLoad: true` ile araçlar hep görünür kalır.
- `list_changed` desteklenir; elicitation desteklenir (bölüm 1.5).
- Claude Code'un 2026-07-28 ile `server/discover` yoklamasını nasıl yaptığının ayrıntısı ("MCP client runtimes" bölümü): **doğrulanamadı** (sayfanın o bölümü okunan parçalarda yer almadı); yalnız 2026-07-28 bağlantılarında elicitation bildirdiği doğrulandı.

Örnek `.mcp.json` (gizli değer yok; belirteç ortamdan):

```json
{
  "mcpServers": {
    "qapsule": {
      "command": "${QAPSULE_BIN:-./target/release/qp}",
      "args": ["mcp", "--principal", "agent", "--surface", "headless",
               "--doc", "${QAPSULE_DOC:-default}"],
      "env": { "QAPSULE_TOKEN": "${QAPSULE_TOKEN}" }
    }
  }
}
```

HTTP varyantı (yerel, belirteçli):

```json
{
  "mcpServers": {
    "qapsule-http": {
      "type": "http",
      "url": "http://127.0.0.1:${QAPSULE_PORT:-7421}/mcp",
      "headers": { "Authorization": "Bearer ${QAPSULE_TOKEN}" }
    }
  }
}
```

Eşdeğer komut: `claude mcp add --transport http qapsule-http http://127.0.0.1:7421/mcp --header "Authorization: Bearer $QAPSULE_TOKEN"` (kabuk değişkeni komut satırında genişler; bu yüzden `.mcp.json` yolu, belirteci kabuk geçmişinde bırakmadığı için tercih edilir).

Kaynaklar:
- https://code.claude.com/docs/en/mcp

### 2.2 Claude masaüstü

Bulgular:
- Dosya: macOS `~/Library/Application Support/Claude/claude_desktop_config.json`; Windows `%APPDATA%\Claude\claude_desktop_config.json`; Linux `~/.config/Claude/claude_desktop_config.json`. Ayarlar → Developer → Edit Config ile açılır; değişiklikten sonra uygulama tamamen kapatılıp açılmalı.
- Biçim: `mcpServers` altında `command` + `args` (+ `env`). Belgedeki örnekler yalnız stdio. Yollar mutlak olmalı. Günlükler `~/Library/Logs/Claude/mcp*.log`; stdio sunucunun stderr'i `mcp-server-<AD>.log`'a gider.
- Bu belgede `${VAR}` genişletmesi ya da `url`/`headers` (uzak sunucu) alanı **belgelenmemiş**; uzak sunucular için ayrı "Connect to remote servers" sayfası var (okunmadı: **doğrulanamadı**). Masaüstü için güvenli varsayım: yalnız stdio, mutlak yol, gizli değeri `env` içine elle değil, betiğin ortamından okutmak.

Örnek:

```json
{
  "mcpServers": {
    "qapsule": {
      "command": "/Users/<kullanıcı>/Works/Mchart/qapsule/scripts/qp-mcp-stdio.sh",
      "args": ["--principal", "agent"]
    }
  }
}
```

Kaynaklar:
- https://modelcontextprotocol.io/docs/develop/connect-local-servers

### Qapsule için sonuç (bölüm 2)

- S2.1 Depoya konacak `.mcp.json` yalnız stdio yoluna işaret eden, ortam değişkenli sürüm olsun; `url`'li girişte `type: "http"` yazılsın (yoksa hata).
- S2.2 Betiğin adı sabit ve depoda olsun (`scripts/…`); `.mcp.json` içine gizli değer ve kullanıcıya özel mutlak yol girilmesin (`${VAR:-varsayılan}` yeterli). Belirteç `.env.local`'dan değil, çalıştıran kabuğun ortamından gelsin (CLAUDE.md kuralı).
- S2.3 `claude -p`/SDK oturumları onaysız yükler: ajan oturumu başlatan betik `--strict-mcp-config` ve açık `--mcp-config` ile yalnız Qapsule'ı yüklesin.
- S2.4 Sonuç boyutu: araç yanıtlarını varsayılan 25.000 belirteç sınırının çok altında tut (öneri: 2.000–4.000); büyük çıktı gerekirse `anthropic/maxResultSizeChars` yerine sayfalama.
- S2.5 Çok araç yok → araç araması devreye girmez; az sayıda araç için `alwaysLoad: true` önerilebilir (düşük gecikme, her tur görünür).
- S2.6 Masaüstü için tek örnek: stdio sarmalayıcı betiği; uzak/HTTP masaüstü yapılandırması doğrulanamadı, vaat edilmesin.

---

## 3. Ajan araç tasarımı: kanıt

### 3.1 Anthropic: "Writing effective tools for agents"

Bulgular:
- "Daha çok araç her zaman daha iyi sonuç vermez." Ham API uçlarını sarmalamak yaygın hata; görev düzeyi araçlar tercih edilir (`schedule_event`, `search_logs`, `get_customer_context` gibi birleştirilmiş araçlar).
- Ad alanı/önek (`asana_search`, `jira_search`) seçimi iyileştirir; önek mi sonek mi tercihi modele göre değişir, ölç.
- `response_format` sabit listesi ("concise"/"detailed"): yazının örneğinde ayrıntılı yanıt 206, kısa 72 belirteç (yaklaşık 1/3).
- Sayfalama, aralık seçimi, filtre, kırpma, makul varsayılanlar; Claude Code araç yanıtlarını varsayılan 25.000 belirteçle sınırlıyor. Hata iletileri eyleme dönük olmalı.
- Düşük düzey kimlikler (UUID) yerine anlamlı alanlar; ad çözümlemek geri çağırma kesinliğini artırdı ve halüsinasyonu azalttı; teknik kimlik, takip çağrısı için "detailed" kipte kalsın.
- Araç açıklaması "yeni işe başlayan birine" anlatır gibi; belirsiz olmayan parametre adı (`user_id`, `user` değil); küçük açıklama düzeltmeleri büyük fark yaratabilir.
- Değerlendirme: gerçekçi çok adımlı görevler + doğrulanabilir sonuç; doğruluk, süre, çağrı sayısı, belirteç, araç hatası izlenir; bekletilmiş test kümesi; transkriptleri ajana analiz ettirip araçları iyileştirme.

Kaynak: https://www.anthropic.com/engineering/writing-tools-for-agents

### 3.2 Araç sayısı ve doğruluk (ölçümler)

Bulgular:
- Anthropic "Advanced tool use" (2025-11-24): Tool Search Tool bağlam tüketimini ~77K'dan ~8,7K belirtece indirdi (%85); MCP değerlendirmesinde doğruluk Opus 4'te %49 → %74, Opus 4.5'te %79,5 → %88,1. Programatik araç çağrısı belirteci 43.588 → 27.297 (%37). Araç kullanım örnekleri karmaşık parametre doğruluğunu %72 → %90 yaptı (kuruluşun iç testi).
- RAG-MCP (arXiv 2505.03275, Mayıs 2025): tüm kataloğu istemle veren taban çizgisinde seçim doğruluğu %13,62; geri çağırmayla seçilen araçlarla %43,13; istem belirteci %50'den fazla azaldı (özet metni doğrulandı). Araç sayısına göre eğri (5 araçta %85 → 20 araçta %45) yalnız ikincil özetten geldi: **doğrulanamadı**.
- MCPVerse (arXiv 2508.16260): en büyük ölçekte bile en iyi model (Claude-4-Sonnet) ortalama 44,2 doğruluk; çoğu modelde MCP sayısı artınca düşüş; en büyük kurulum ~147K belirteçlik şema (arama sonucundan; ayrıntı doğrulanmadı).
- Bir uygulamacı ölçümü (dev.to "we gave llms 150 tools", hakemsiz): 25–150 araç arasında altı model; düşüş 25–50 arasında başlıyor; bir model %86,7 → %76,7. OpenAI API'sinde istek başına 128 araç sınırı.
- Sonuç: yön tutarlı (araç arttıkça doğruluk ve belirteç maliyeti kötüleşir), ama sayısal eşikler çalışmaya ve modele bağlı; evrensel bir "N araç" eşiği **doğrulanamadı**.
- Küçük modeller: araç sayısı kontrollü, küçük modele özgü 2026 ölçümü bulunamadı (**doğrulanamadı**). MCP Bridge makalesinde 8B RL-hizalı model kendi kıyasında 120B tabandan iyi (%69,7 / %58,7), ama bu tek makale ve araç sayısı değişmiyor. Dolaylı çıkarım: küçük modellerde şema belirsizliği ve araç sayısı daha da pahalı; önlem olarak az araç + katı şema + sabit liste (enum).

Kaynaklar:
- https://www.anthropic.com/engineering/advanced-tool-use
- https://arxiv.org/abs/2505.03275
- https://arxiv.org/pdf/2508.16260
- https://dev.to/craigtracey/we-gave-llms-150-tools-heres-what-broke-2jaj (hakemsiz, yalnız ipucu)
- https://arxiv.org/pdf/2504.08999 (MCP Bridge)

### 3.3 Kod modu ve kod yürütme

Bulgular:
- Anthropic "Code execution with MCP" (2025-11-04): araçlar dosya ağacı gibi sunulur, ajan yalnız gereken tanımı okur; veri kodda süzülür (10.000 satırlık tablodan 5 satır modele gider); örnek iş akışında 150.000 → 2.000 belirteç (%98,7; yazının kendi örneği, genel ölçüt değil). Ara sonuçlar yürütme ortamında kalır; hassas veri belirteçlenebilir. Uyarı: ajan kodunu çalıştırmak "uygun kum havuzu, kaynak sınırı ve izleme" ister.
- Cloudflare "Code Mode" (2025-09-26): MCP şemasını TypeScript API'sine çevirip tek bir "kod çalıştır" aracı verir; V8 yalıtımı (isolate) ile kum havuzu, internet yok, API anahtarını gözetmen tutar. Yazı nicel ölçüm vermiyor (yalnız nitel iddia).
- Qapsule açısından: değerli ama (i) ek güvenlik yüzeyi, (ii) küçük modellerde güvenilirlik belirsiz. Hesap kodunda LLM ve ağ yok kuralı ayrı; kod modu istemci/ajan tarafının tercihi olur, motor değişmez.

Kaynaklar:
- https://www.anthropic.com/engineering/code-execution-with-mcp
- https://blog.cloudflare.com/code-mode/

### Qapsule için sonuç (bölüm 3): araç yüzeyi adayları

Aday A (önerilen başlangıç, ~8 araç, görev düzeyi; hepsi `qp/0` komut/sorgu yoluna iner):

| Araç | Ek açıklama | Ne yapar |
|---|---|---|
| `qp_orient` | okuma | Oturum özeti: aktif doküman, görünüm, `seq`, neler değişti (son N olay), bekleyen öneriler. Ajanın ilk çağrısı. |
| `qp_read` | okuma | `what` ∈ {study, series, capsule, finding, comments, …}, `response_format` ∈ {concise, detailed}, `limit`/`cursor`. Varsayılan `concise`. |
| `qp_changes` | okuma | `after_seq` ile günlük; sayfalı; boşluk tespiti için `seq`. Abonelik yoksa yoklama yolu. |
| `qp_watch` | okuma (yan etkisi: abonelik kaydı) | Niyet/abonelik açma-kapama (`subscribe`/`unsubscribe`, filtre). |
| `qp_comment` | yazar, yıkıcı değil | Yorum yaz; motor sayıları ve atıfları deterministik doğrular; reddi yapılandırılmış hata (`isError`) döner. |
| `qp_propose` | yazar, yıkıcı değil, idempotent | Öneri oluştur (`ack.status = proposed`); kabul bu araçla yapılamaz. |
| `qp_attend` | yazar, yıkıcı değil | Ajan → kullanıcı dikkat yönergesi (RD1 `attend`). |
| `qp_command` | yazar | Atomik çok işlemli ileri düzey çıkış kapısı; varsayılan yetkide kapalı ya da yalnız `proposed` kipli. |

- S3.1 Ham `op_<işlem>` araçlarının tamamını ajan yüzeyine koyma (RD1'deki makinece üretilen yüzey, test ve uygunluk için kalsın; ajan profili yukarıdaki küçük listeyi açsın, `tools/list` ilke yetkisine göre değişebilir: şartname bunu açıkça izin veriyor). Bu, "az sayıda görev düzeyi araç" kanıtıyla uyumlu.
- S3.2 `qp_accept_proposal`/kabul işlemi ajan profilinin araç listesinde HİÇ bulunmasın (bölüm 4).
- S3.3 Yanıt biçimi: `response_format: "concise" | "detailed"`, varsayılan `concise` (kısa özet + kimlikler yalnız `detailed`'de); `concise` içinde de insan okur ad (ör. "AKBNK 1G, 14 Ağu") ve gerekirse `ref` alanı; sayfalama `cursor`/`limit`, sayfa üst sınırı sabit; kırpılmışsa `truncated: true` ve nasıl daraltılacağı yazılı.
- S3.4 Belirteç bütçesi: tek yanıt hedefi ≤ 2.000–4.000 belirteç (Claude Code sınırı 25.000, uyarı 10.000; bu hedef benim önerim). Şema ve açıklamalar kısa, parametre adları belirsizliksiz, sabit liste (enum) bol.
- S3.5 Araç açıklamaları: ne zaman kullanılır / kullanılmaz, örnek çağrı (kanıta göre karmaşık parametrelerde örnek doğruluğu artırıyor), hata iletisinde `did_you_mean` (zaten var).
- S3.6 Değerlendirme: Anthropic yöntemi (gerçekçi çok adımlı görev + doğrulanabilir sonuç + belirteç/çağrı/hata sayısı) küçük bir "ajan değerlendirme kümesi" olarak R5B'de kurulabilir; küçük ve büyük model ayrı ölçülmeli (küçük model verisi literatürde yok: doğrulanamadı, kendi ölçümümüz gerekir).
- S3.7 Kod modu şimdilik kapsam dışı; `qp_read`'in `detailed` ve sayfalama çıktısı veri süzmeyi sunucuda yapar (kod yürütmeden benzer kazanç).

---

## 4. Dış içerikten talimat saldırıları ve savunmalar

### 4.1 Sınıflandırma ve ölçüm kümeleri

Bulgular:
- OWASP LLM01:2025: **doğrudan** (kullanıcı girdisi) ve **dolaylı** (model harici içerik okur; içinde gömülü talimat davranışı değiştirir). OWASP: tam önleme olmayabilir; azaltma listesi: rol/kapsam kısıtı, çıktı biçimini deterministik kodla doğrulama, girdi/çıktı süzme, **en az yetki** (belirteç ve işlevler uygulama kodunda), **yüksek riskli eylem için insan onayı**, harici içeriği ayırıp etiketleme, karşı-test.
- InjecAgent (arXiv 2403.02691): 1.054 test durumu, 17 kullanıcı aracı, 62 saldırgan aracı; ReAct-GPT-4 saldırılara %24 oranında kırılgan, "hacking prompt" ile neredeyse iki katı.
- AgentDojo (arXiv 2406.13352): 97 gerçekçi görev, 629 güvenlik test durumu; güncel modeller saldırı olmadan bile birçok görevde başarısız; mevcut saldırılar bazı güvenlik özelliklerini bozuyor, hepsini değil.
- Not: AgentDojo/InjecAgent 2024 çalışmaları; güncel model sonuçları için tablolar okunmadı (**doğrulanamadı**), bu yüzden sayılar eski modellere aittir.

Kaynaklar:
- https://genai.owasp.org/llmrisk/llm01-prompt-injection/
- https://arxiv.org/abs/2403.02691
- https://arxiv.org/abs/2406.13352

### 4.2 Savunmalar

Bulgular:
- **Spotlighting** (Microsoft, arXiv 2403.14720): girdiyi işaretleme/kodlama ile talimattan ayırt ettirme; saldırı başarı oranını >%50'den <%2'ye indirdi (GPT ailesi, görev kaybı az). Olasılıksal savunma: model uyum gösterirse çalışır.
- **CaMeL** (arXiv 2503.18813): kontrol akışını güvenilir kullanıcı isteğinden türetir, güvenilmeyen veri kontrol akışını değiştiremez; yetenek (capability) etiketleriyle veri sızması engellenir; AgentDojo'da görevlerin %77'sini "kanıtlanabilir güvenlikle" çözer (korumasız %84). Yani ~7 puan faydadan feragat.
- **Tasarım örüntüleri** (Beurer-Kellner ve ark., arXiv 2506.08837): Action-Selector, Plan-Then-Execute, LLM Map-Reduce, Dual LLM, Code-Then-Execute, Context-Minimization. Ortak ilke: ajan güvenilmeyen girdiyi işledikten sonra o girdi sonuç doğuran eylemi tetikleyememeli.
- **Meta "Agents Rule of Two"** (2025-10-31): bir oturumda üç özellikten en çok ikisi: (A) güvenilmeyen girdiyi işler, (B) hassas sisteme/özel veriye erişir, (C) durumu değiştirir ya da dışarıyla iletişim kurar. Üçü gerekiyorsa yeni bağlam ya da insan denetimi.
- **"Lethal trifecta"** (Simon Willison, 2025-06-16): özel veri + güvenilmeyen içerik + dış iletişim birleşimi veri çalmayı mümkün kılar; tek güvenli yol bu birleşimden kaçınmak; koruma ürünlerine güvenilmez.
- MCP şartnamesi araç çıktısını sunucu arındırsın, istemci modele vermeden doğrulasın diyor; ek açıklamalar güvenilmez; araç çağrısında insan döngüde olsun (SHOULD).

Kaynaklar:
- https://arxiv.org/abs/2403.14720
- https://arxiv.org/abs/2503.18813
- https://arxiv.org/html/2506.08837
- https://ai.meta.com/blog/practical-ai-agent-security/
- https://simonwillison.net/2025/Jun/16/the-lethal-trifecta/

### 4.3 Qapsule'a uygulama: mimari güvence

Qapsule'da ajanın okuduğu içerik güvenilmeyen veridir: haber metni, kullanıcı yorumları, belge başlıkları, sembol adları. Eşleme:
- (A) güvenilmeyen girdi: VAR (yorum/metin alanları, dış veri).
- (B) hassas veri: belge ve kullanıcı çalışması VAR (ama hesap yok, para hareketi yok).
- (C) durumu değiştirme: `qp_comment`, `qp_propose`. Dış iletişim: Qapsule'ın ağ çıkışı yok (hesap kodunda ağ yok), ajanın başka araçları ajan istemcisinin sorumluluğu.
- Qapsule yüzeyinde ajan yalnız **öneri** üretir; değişikliği uygulayan kabul adımıdır ve insan ilkesindedir. Bu, "Rule of Two"daki (C) özelliğini ajan için daraltır: ajan kalıcı karar veremez.

### Qapsule için sonuç (bölüm 4)

- S4.1 **Yapısal kural, istemde değil motorda:** `accept_proposal` yalnız `principal.kind == human` (insan yüzeyi, kimlik doğrulanmış) olan çağrılarda ve `proposed_by != caller` iken geçerli olsun. Ajan profilinin araç listesinde bu işlem yok; yine de ham `command` yolunda denenirse motor `forbidden` (açıklayıcı hata) ile reddetsin. Ajan kendi önerisini onaylayamaz kuralı test edilsin (ajan kimliğiyle, başka ajan kimliğiyle ve sahte `principal` alanıyla).
- S4.2 İlke (principal) kimliği istemcinin gövdesindeki alana değil, bağlantının yapılandırmasına bağlı olsun (stdio'da başlatma bayrağı, HTTP'de belirteçten); istemcinin gönderdiği `principal`/`surface` değerleri yok sayılsın (şartname: sunucu istemci kimliğine doğrulamasız güvenmemeli; elicitation bölümü).
- S4.3 Dış kaynaklı metni veri olarak etiketle: araç sonuçlarında harici/kullanıcı metni ayrı bir alana (`untrusted: true` ya da `data` altında) konsun, `content[0].text` içinde de açık sınırlayıcıyla ("<harici-metin>…") sarılsın (spotlighting benzeri; olasılıksal, tek başına yeterli sayılmaz). Talimat gibi görünen içerik süzülmez ama etiketlenir.
- S4.4 Yetki ayrımı: okuma araçları ile yazma araçları ayrı yetkiler; varsayılan ajan profili "okuma + yorum + öneri". `qp_command` varsayılan kapalı.
- S4.5 Yazma sonuçları ajana kısıtlı döner (yalnız `ack`/`error`); yazılan metinde dış bağlantı ya da görsel URL'si yasak (veri sızdırma kanalı: örüntü ilkesi "çıktı sızıntı yaratmamalı").
- S4.6 Hız ve boyut sınırları (şartname: sunucu hız sınırlamalı): yorum/öneri uzunluğu, saniyedeki yazma sayısı; ardışık reddedilen yazmada ajan oturumu kısıtlansın.
- S4.7 Her ajan eylemi günlükte (kim, hangi araç, hangi `seq`), değişmez kayıt; denetim insan tarafında okunabilir.
- S4.8 Doğrulama testi: AgentDojo/InjecAgent tarzı küçük bir iç küme (haber metninde gömülü "öneriyi kabul et" gibi talimatlar) hazırlanıp motorun yapısal reddi ölçülsün; ölçüt "model direndi mi" değil "motor engelledi mi" olsun.

---

## 5. LLM çıktısında kaynak/atıf doğrulaması

### 5.1 Sayısal temellendirme yöntemleri

Bulgular (çoğu ön baskı; yalnız özet/çıkarım okundu, bulgu sayıları doğrulanmadı):
- **Proof-Carrying Numbers (PCN)** (arXiv 2509.06902): model çıplak sayı yazmaz; sayı aralığı yapılandırılmış bir iddiaya bağlanır, doğrulayıcı bildirilmiş bir politikayla (tam eşitlik, yuvarlama, takma ad, tolerans) kaynak iddiaya karşı denetler; doğrulama modelde değil **sunumda (renderer)**; doğrulanmayan sayı varsayılan "doğrulanmamış". Özetteki ilke: "güven yalnız kanıtla kazanılır". (Doğruluk/tamlık kanıtı yazarın iddiası; doğrulanmadı.)
- **VeNRA** (arXiv 2603.04663): model destekleyici alıntıyı verir, karakter ofsetine eşlenir; tam alıntı tutmazsa en uzun sayısal belirteçle aranır; metrik adı örtüşmesi için eşik 0,30; yazarlar tek sayılar için bulanık dizge eşlemesinin işe yaramadığını bildiriyor.
- **Finansal özetlemede hata sınıfları** (arXiv 2404.06162): uydurma sayı, yuvarlama hatası, aritmetik hata, bağlam uyumsuzluğu; "1M" ve "1.000.000" gibi eşdeğer biçimler açıklamada toplanır.
- **FinGround** (arXiv 2604.23588): cevabı atomik iddialara böler; hesaplamalı iddiaları tablolara karşı aritmetikle yeniden doğrular; genel dedektörlerin hesaplamalı hataların yaklaşık %43'ünü kaçırdığı iddiası (yazarların, doğrulanmadı).
- Ortak çıkarım (deterministik uygulanabilir): (1) sayıyı metinden **belirleyici bir ayrıştırıcıyla** çıkar (binlik/ondalık ayracı, %, bps, K/M/B, tarih); (2) her sayıya kaynak alanı (ref) bağla; (3) kaynak değerle **tolerans politikasına göre** karşılaştır (tam, yuvarlama basamağı, göreli tolerans); (4) türetilmiş sayıyı (yüzde değişim, fark) motorun hesabıyla yeniden hesapla; (5) bağlanamayan sayı → "doğrulanamadı", reddet ya da işaretle.

Kaynaklar:
- https://arxiv.org/abs/2509.06902
- https://arxiv.org/pdf/2603.04663
- https://arxiv.org/pdf/2404.06162
- https://arxiv.org/pdf/2604.23588
(Son üçü yalnız arama özetinden alındı; özet dışı içerik okunmadı.)

### 5.2 Tavsiye dili (finansal tavsiye) denetimi

Bulgular:
- Türkiye çerçevesi (SPK; hukuk bürosu özetlerinden, kanun metni ve SPK rehberine karşı **doğrulanmalı**): yatırım danışmanlığı, müşterinin mali durumu ve risk/getiri tercihleri dikkate alınarak sermaye piyasası araçlarına ilişkin kişiye özel yönlendirici tavsiyedir (SPKn m.37 çerçevesinde); genel yatırım tavsiyesi/araştırma/piyasa analizi danışmanlık sayılmaz (m.45/2 atfı); danışmanlık yalnız izinli kuruluşlar tarafından verilebilir; izinsiz faaliyet SPKn m.109 kapsamında; ücretli, müşteriye özel al/sat sinyalleri izinsiz danışmanlık sayılabilir.
- SPK'nın yapay zekâ çıktısına özgü açık bir kuralı: **doğrulanamadı**.
- Dolayısıyla Qapsule'ın güvenli çizgisi: kişiye özel yönlendirme ("al", "sat", "sen şunu yap", hedef fiyat/miktar/stop önerisi, kullanıcının portföyüne atıf) üretmemek; tanımlayıcı gözlem ve koşullu, genel dil.
- Deterministik denetim için yöntem olarak kural/sözlük + yapı denetimi akla yakın (OWASP: çıktı biçimini deterministik kodla doğrula); ölçüm kümesi/benchmark bulunamadı (**doğrulanamadı**).

Kaynaklar:
- SPK Yatırım Hizmetleri ve Kuruluşları Rehberi: https://spk.gov.tr/data/61e496a11b41c60d1404d6b2/9%20Yat%C4%B1r%C4%B1m%20Hizmetleri%20ve%20Kurulu%C5%9Flar%C4%B1%20Rehberi%20(i-SPK.37.10)%20v3.pdf (arama sonucunda listelendi, içeriği okunmadı)
- https://www.alomaliye.com/2022/04/07/yatirim-hizmetleri-ve-faaliyetleri-ile-yan-hizmetler-iii-37-1-c/ (arama sonucu, ikincil)
- https://bilalalyar.av.tr/?p=73701 (hukuk bürosu; ikincil)

### Qapsule için sonuç (bölüm 5)

- S5.1 Yorum doğrulaması (motor, deterministik): yorum gövdesinde sayı yalnız yapılandırılmış **sayı atıfı** olarak yazılabilsin (örn. `{{ref:finding/17.value}}` ya da `ref` + metin içinde değer); motor (a) metindeki çıplak sayıları ayrıştırır, (b) atıfsız sayıyı reddeder ("kaynağı yok"), (c) atıflı sayıyı bildirilmiş tolerans politikasıyla (tam / yuvarlama / bps / yüzde) kaynakla karşılaştırır, (d) türetilmiş değeri kendi hesabıyla yeniden hesaplar, (e) tutmazsa `isError` ile `expected`/`did_you_mean` döner. Bu PCN'in "doğrulama sunumda/motorda" ilkesine uyar; modelin kendi beyanına güvenmez.
- S5.2 Sunumda doğrulanmamış sayı hiç gösterilmesin ya da "doğrulanmadı" işaretiyle gösterilsin (fail-closed).
- S5.3 Biçim normalizasyonu: yerel ondalık/binlik (Türkçe `1.234,56`), `%`, `bps`, `K/M/B`/`mn/mlr`, tarih ve saat dilimi; tek bir deterministik ayrıştırıcı (`f64`, kural gereği) ve altın testler.
- S5.4 Tavsiye dili: kural tabanlı denetim, iki kademeli: (1) yasak kalıplar (emir kipi + enstrüman: "al/sat/tut", "tavsiye ederim", hedef fiyat, stop/boyut önerisi, "portföyünüz/size göre") motorda ret; (2) izin verilen biçim: gözlem + koşullu ("X olursa Y anlamına gelir") + "yatırım tavsiyesi değildir" kısa ibaresi (ibare tek başına koruma sayılmaz; kaynaklardan biri bunu tartışıyor, doğrulanmadı). Denetim açıklamalı hata dönsün; sözlük sürümlü ve test edilebilir. Hukuki geçerlilik için hukuk danışmanı onayı gerekli (bu araştırmanın kapsamı dışı).
- S5.5 Öneri (proposal) ile yorum (comment) ayrımı dilde de korunsun: ajan "öneri" nesnesinde yalnız bir değişiklik tarif eder (örn. "bu çalışmaya şu göstergeyi ekle"), alım-satım yönü değil.

---

## Genel öneriler (T1 dışı, ana akışa not)

- `crates/qp-mcp`: modern uçta `notifications/message` yerine `subscriptions/listen` + `resources/updated`; `ttlMs`/`cacheScope`, araç sırası, ek açıklamalar; HTTP taşıması varsa `Origin`/`Host`/belirteç/`127.0.0.1` varsayılanları (S1.1–S1.6).
- Ajan profili için küçük araç kümesi (bölüm 3 aday A) ve ilke kimliğinin bağlantı yapılandırmasından alınması (S4.2).
- Motorda atıflı sayı doğrulayıcı ve tavsiye dili denetçisi (S5.1–S5.4).
- Doğrulanamadı listesi: Claude Code'un `resources/updated` ile modeli uyandırması; Claude Code "client runtimes" ayrıntısı; masaüstünde uzak (`url`) giriş; araç sayısı eşiği; küçük model ölçümü; SPK'nın yapay zekâya özgü kuralı; güncel modellerle AgentDojo/InjecAgent sayıları.
