# RD1 — Araştırma: var olan protokoller ve 2026–2030 bakışı

Tarih: 2026-11-14. Bu belge üç araştırma alt ajanının raporlarının sentezidir. Alt
ajanlar kaynakları bu oturumda WebFetch/WebSearch ile **güncel belgelerinden** okudu;
bellekten yazılmadı. Okunamayan ya da doğrulanamayan yerler ayrıntılı dosyalarda
"DOĞRULANMADI" diye işaretli. Ayrıntı ve tam URL listeleri:

- `research/agent-protocols.md` — MCP (2026-07-28), MCP Apps, A2UI v1.0, AG-UI, A2A v1.0
- `research/collab-dataflow.md` — LSP 3.17/3.18, Automerge/Yjs/Loro, ShareDB/json1,
  ProseMirror collab, Replicache/Zero, Figma multiplayer, Vega/Vega-Lite, Perspective,
  TradingView datafeed, RFC 6902/7386
- `research/futures-encodings.md` — 2026–2030 varsayımları, ikili kodlamalar

Araştırma, tasarımdan **önce** yapıldı (T1 → T2); protokol (`docs/PROTOCOL.md`) bu
bulguların üzerine kuruldu. Görevler (`tasks/TASKS.md`) de tasarımdan önce, ayrı bir
commit'te yazıldı (`492f79a`).

## 1. İncelenen protokoller

Ölçüt 1 en az sekiz protokol istiyor; on iki sistem incelendi. Her satırda: neyi çözüyor,
bizdeki karşılığı, ne alındı, ne alınmadı.

| # | Protokol (sürüm, kaynak) | Neyi çözüyor | Bizde karşılığı | Alındı | Alınmadı (neden) |
|---|--------------------------|--------------|-----------------|--------|------------------|
| 1 | **MCP** 2026-07-28 — https://modelcontextprotocol.io/specification/2026-07-28 (changelog, basic, server/tools, resources, patterns/subscriptions, patterns/mrtr, versioning) | Ajan ↔ araç/kaynak. Bu sürümde **durumsuz**: `initialize` yok, her istek `_meta` içinde sürüm ve istemci yeteneklerini taşır; `server/discover`; `subscriptions/listen`; MRTR (`InputRequiredResult`); Tasks uzantı oldu. | Komut ve sorgu araç olarak; `describe` ≈ `server/discover`; abonelik ≈ `subscriptions/listen` | Araç tanımlarını **şemadan üretmek** (`inputSchema`/`outputSchema`); iki katmanlı hata (protokol hatası ile model düzeltmesi için `isError: true` araç hatası) — bizde `error` kodları + `path` + `did_you_mean`; yeteneklerin istemciden bildirilmesi | Durumsuzluğu çekirdek protokole almak (bizim doküman canlı, sıralı ve aboneli; durum otoritede). SSE yeniden oynatmanın kaldırılması bizim değişim günlüğü ihtiyacına ters; günlük bizim işimiz. |
| 2 | **MCP Apps** (`io.modelcontextprotocol/ui`, kararlı 2026-01-26) — https://modelcontextprotocol.io/extensions/apps/overview , ext-apps `specification/2026-01-26/apps.mdx` | Araçların etkileşimli HTML arayüz döndürmesi: `ui://` kaynak, `text/html;profile=mcp-app`, `postMessage` üzerinden JSON-RPC (`ui/initialize`, `ui/notifications/tool-result`, `ui/update-model-context`) | Gömme kenarı: kabuk bir `ui://` paketinde çalışır; `HostContext` ≈ bizim `Surface` | `HostContext` alanları (platform, cihaz yetenekleri, kap boyutu, görüntü kipi) **Yetenek** mesajını şekillendirdi (`surface.kind`, `screen`, `input`, `max_depth`); "kullanıcının gördüğü bağlamı modele bildirme" fikri **Dikkat**'in kullanıcı yönü | HTML'yi protokolün parçası yapmak (biz çizimi çekirdekte yaparız; HTML yalnız gömme kabı). |
| 3 | **A2UI** v1.0 (kararlı 2026-11-11) — https://a2ui.org/specification/v1.0-a2ui/ , https://a2ui.org/specification/v0.9.1-a2ui-extension-specification/ | Ajan → arayüz: `createSurface`, `updateComponents` (düz bitişiklik listesi, kilitli katalog), `updateDataModel` (JSON Pointer), `deleteSurface`; arayüz → ajan `action`, `error {code, surfaceId, path, message}`; `supportedCatalogIds` pazarlığı | Görünüm (R2 grameri) ve **Değişim**; A2UI üzerinde kapsülümüz özel bir katalog bileşeni olabilir | Hatanın **yol göstererek** bildirilmesi (`path`); kataloğun pazarlıkla seçilmesi (bizde `features`); veri modeli ile bileşenin ayrılması (bizde doküman ile varlık durumu) | Genel amaçlı bileşen kataloğu (form, buton): bizim gramerimiz finansal anlam taşır, düğme dizmez. Ajanın bileşen ağacını doğrudan yazması K21-1 ile ölçüldü (`experiments/T6e`). |
| 4 | **AG-UI** (CopilotKit) — https://docs.ag-ui.com/concepts/events , /state , /interrupts | Ajan ↔ ön yüz olay akışı: `StateSnapshot`/`StateDelta` (RFC 6902), araç çağrıları, `interrupt`/`resume` (insan onayı: `reason`, `responseSchema`, `expiresAt`) | **Değişim** ve **Öneri** | Anlık görüntü + fark ayrımı (bizde `document` sorgusu + `change`); interrupt'ın `expiresAt`'i → öneri süre aşımı | Sıra numarasız fark akışı (boşluk tespiti yok; bizde `seq`); sohbet merkezli olay modeli; zayıf `RunError`; JSON Patch'in kendisi (bkz. 12). |
| 5 | **A2A** v1.0.0 — https://a2a-protocol.org/latest/specification/ | Ajan ↔ ajan: Agent Card (yetenek keşfi), görev durumları (`INPUT_REQUIRED`, `AUTH_REQUIRED`), akış güncellemeleri, `A2A-Version` başlığı | Çok ajanlı çalışmada köprü; Agent Card ≈ `describe` | Yetenek keşfinin ayrı bir okuma olması (`describe`); sürümün her mesajda değil bağlantıda bildirilmesi (bizde `hello`) | Opak ajan ve "görev bitti" modeli: canlı, ortak yazılan bir dokümana uymuyor. |
| 6 | **LSP** 3.17 / 3.18 — https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/ , .../3.18/... | Editör ↔ dil sunucusu: `initialize` yetenek pazarlığı, sürümlü dokümanlar, artımlı eşitleme, `$/cancelRequest`, `ContentModified`, `$/progress`, değişim açıklamaları | **Yetenek** (`hello`/`welcome`), sürüm (`seq`), hata kodları | Bağlantı başında yetenek pazarlığı; sürüm = uygulanmış değişim sayısı (bizde `seq`); belgenin sahibinin açıkça tek olması (açık doküman istemcinindir) → tek otorite ve **devir** fikrinin dayanağı; "içerik değişti" türü hata → `conflict` | Metin aralığına dayalı düzenlemeler (biz anlamsal işlemle çalışırız). |
| 7 | **CRDT: Automerge** 2.x/3.x, **Yjs**, **Loro** — https://automerge.org/docs/reference/concepts/ , /merge-rules/ , /blog/automerge-3/ , https://docs.yjs.dev/api/undo-manager , https://github.com/dmonad/crdt-benchmarks | Merkezsiz çok yazarlı belge; çevrimdışı birleşme reddetmeden yakınsar | Eşzamanlılık alternatifi (K21-2) | Değişimin kökeni: yazar, sıra, zaman, ileti (Automerge change); **kökene göre geri alma** (Yjs `UndoManager.trackedOrigins`) → bizde "konuşanın kendi son girdisi"; değişimlerin idempotent olması | Aynı anahtarda "keyfi ama deterministik" kazanan (iki ajanın yazısından rastgele birini seçer); eşzamanlı eklemelerin ikisinin de kalması (çift gösterge). Ölçüm: `experiments/T6a`. |
| 8 | **OT: ShareDB** (json0/json1), **ProseMirror collab** — https://share.github.io/sharedb/ , https://github.com/ottypes/json1 , https://prosemirror.net/docs/guide/#collab | Merkezî sıralama + dönüştürme; ProseMirror: merkezî otorite, sürüm = kabul edilmiş adım sayısı, istemci adımlarını yeniden temele oturtur | Tek otorite + sıralı günlük | ProseMirror'un modeli: otorite sırası, kabul edilmeyen adımın istemcide yeniden temellenmesi → bizde `base` + `conflict` + yeniden gönderme | Genel OT dönüşüm matrisleri: json1 hâlâ önizleme; README'si milyon civarı bulanık testte yakınsama hatası bulunduğunu söylüyor. İşlem alfabesi büyüdükçe N² dönüşüm. |
| 9 | **Figma multiplayer** (2019, arşiv kopyası), **Replicache/Zero** — https://madebyevan.com/figma/how-figmas-multiplayer-technology-works/ , https://doc.replicache.dev/concepts/how-it-works , https://zero.rocicorp.dev/docs/mutators | Sunucu otoriteli çok yazarlılık: özellik düzeyinde "son yazan kazanır" (sunucu sırası), istemci tahmini; Replicache/Zero: aynı mutator istemcide ve sunucuda, geri sar ve yeniden oynat | Otorite, iyimser istemci, devir | Özellik/öğe düzeyinde son yazan kazanır + sunucu sırası (bizde öğe düzeyinde dokunulan kimlikler); "geri alma kendi geçmişini düzenler, eski durumu geri yüklemez" → ters değişimin yeni girdi olarak yazılması; **aynı mutator iki uçta** → motor sunucuda ve cihazda aynı Rust kodu (K21-3) | Figma'nın CRDT/OT'yi neden almadığı gerekçesi bizim durumumuza da uyuyor; ama Replicache'te sunucu uygulaması istemciden farklı olabilir — bizde belirleyicilik aynı kodu şart koşar. |
| 10 | **Vega / Vega-Lite** — https://vega.github.io/vega/docs/api/view/ , /signals/ , https://vega.github.io/vega-lite/docs/selection.html , /parameter.html | Bildirimsel görselleştirme veri akışı: sinyaller, `changeset` ile toplu insert/remove, tek darbe (`runAsync`), seçimler/parametreler | Hesap serilerinin bağımlılık grafiği; etkileşim durumu | Toplu değişimin tek darbede uygulanması (bizde bir komut = bir girdi = bir yayın); **geçici etkileşim durumunun (sinyal, seçim) kalıcı belgeden ayrılması** → bizde varlık durumu (presence) günlüğe girmez | İfade dizgeleri (güvenilmez girdide kod yürütme yüzeyi); elle `run()`. |
| 11 | **TradingView Charting Library datafeed**, **Perspective** — https://www.tradingview.com/charting-library-docs/latest/connecting_data/Datafeed-API/ , https://perspective-dev.github.io/ | Canlı finansal veri: anlık görüntü (`getBars`) + akış (`subscribeBars`), yalnızca son barın güncellenmesi ya da yeni bar eklenmesi, geçmiş düzeltmesi için önbellek sıfırlama | `append_bars` / `update_last_bar`, `document` anlık görüntüsü | Son bar güncelle / yeni bar ekle ayrımı (R1'deki `BarEvent` ile aynı) | Sıra numarası ve boşluk tespitinin olmaması (bizde `seq`); Perspective'in delta/Arrow ayrıntıları doğrulanamadı. |
| 12 | **JSON Patch** (RFC 6902), **JSON Merge Patch** (RFC 7386/7396) — https://www.rfc-editor.org/rfc/rfc6902 , https://www.rfc-editor.org/rfc/rfc7386 | Genel belge farkı | Değişim biçimi alternatifi (`experiments/T6c`) | RFC 6902 `test` işlemi ön koşul olarak → bizde `base` | Dizi indeksli yolların eşzamanlılıkta kayması; niyetin kaybolması ("bulguyu reddet" üç anonim `replace` olur); alan kurallarının denetlenememesi; Merge Patch `null` yazamaz, dizileri bütün değiştirir. Yalnızca içe/dışa aktarım ve hata ayıklama sınırında. |

### Bütün protokollerde ortak boşluk → özgün kısım (K18)

Hiçbir protokol şunları birlikte sağlamıyor; bunlar Qapsule protokolünün özgün kısmıdır:

1. **Sıralı, boşluksuz, yeniden oynatılabilir değişim günlüğü** — her girdide köken,
   onaylayan, istek, değişim ve **ters değişim**; yeniden oynatma hesap kodu ve saat
   gerektirmez (değişimler tam öğe taşır).
2. **İstek / değişim ayrımı:** ajan anlam düzeyinde yazar (`reject_finding`), günlük
   ilkel ve tam değişim tutar (`set_capsule_state` + `push_ledger`). Yeniden oynatma
   motor sürümünden bağımsızdır; denetim izi niyeti kaybetmez.
3. **Öneri dokümanın içindedir:** bekleyen, reddedilen ve süresi dolan yazılar da iz olarak
   kalır; kimlikler öneri anında atanır.
4. **İki yönlü dikkat:** kullanıcının baktığı yer (girdiden) ve ajanın baktırmak istediği
   yer ve sırası (yönerge) — hiçbir protokolde yok.
5. **Anlamsal abonelik:** bulgu durumu, koşul (kenar tetiklemeli), niyet; içeriksiz
   "değişti" sinyali (MCP) değil.
6. **Yetenek → derinlik:** yüzey kendini tanıtır, otorite anlam zoom'u derinliğini seçer
   (cümle / kapsül / grafik). MCP Apps `HostContext` boyut bildirir ama derinlik kavramı yok.
7. **Saat bir girdidir:** otoritenin saati yoktur; süre aşımı ve zaman damgaları `clock`
   mesajından gelir; aynı girdi aynı sonucu verir.
8. **Varlık durumu ile doküman ayrımı:** saniyede onlarca kaydırma kalıcı günlüğe girmez,
   ama ajanlar onu okuyabilir (AI-native testi 4).

## 2. 2026–2030: protokolün dayandığı varsayımlar

Her varsayım için: bugünkü kanıt (ayrıntı ve kaynaklar `research/futures-encodings.md`
Bölüm A) ve **gerçekleşmezse ya da tersine dönerse protokolün hangi kısmı değişir**.

| # | Varsayım | Kanıt (2025–2026) | Gerçekleşmezse / tersine dönerse protokolde ne değişir |
|---|----------|-------------------|--------------------------------------------------------|
| V1 | **Model cihaza iniyor** (telefon, saat üstünde küçük model) | Telefonda orta (Apple Foundation Models çerçevesi, Gemini Nano / ML Kit GenAI); saat ve gözlükte zayıf (Wear OS'ta kanıt yok) | Otoritenin cihazda çalışma gerekçesinin yarısı (yerel ajan) düşer; **devir** (§8.4) yalnızca çevrimdışı okuma için kalır; `headless` profili sunucu ajanlarına iner. Tersine (model her yerde) → cihazda ajan da `hello` ile bağlanır; protokol değişmez, çünkü ajan zaten aynı mesajları kullanıyor. |
| V2 | **Ajanlar arayüz üretiyor** (üretken UI) | Güçlü: A2UI 1.0 kararlı (2026-11-11), MCP Apps resmi uzantı (2026-01-26), AI SDK üretken UI | Gerçekleşmezse gömme katmanı (`EMBEDDING.md`) küçülür, kendi görüntüleyicimiz tek kabuk olur. Tersine aşırı gerçekleşirse (ajan her pikseli yazar) → K21-1'in "sunucu" katmanı tartışmaya açılır; ölçüm `experiments/T6e`. İki rakip model (A2UI ve MCP Apps) birleşmezse ikisine de uyarlayıcı gerekir. |
| V3 | **Ekranlar çeşitleniyor** (saat, gözlük, ses, araba) | Ekransız (ses) güçlü; ekranlı gözlük zayıf (IDC 2026 Ç2: ekransız %70, ekranlı %14) | Gerçekleşmezse yetenek uzlaşması iki profile çöker (masaüstü, telefon); `voice`/`watch` profilleri ölü kalır ama zararsız (ek alan). Gözlük gelirse yeni `surface.kind` eklenir (ek değişiklik, §12). |
| V4 | **Çok ajanlı çalışma** (bir belgede birden çok ajan) | Standart var (A2A 1.0), güvenilirlik düşük | Gerçekleşmezse tek yazar + önerici modeline iner; `base`/`conflict` ve kökene göre geri alma az kullanılır ama protokol aynı kalır. Tersine (onlarca ajan) → otorite darboğaz olabilir: belge bölümlenmesi (alt belgeler, ayrı otoriteler) ve abonelik süzgeçleri gerekir. |
| V5 | **LLM maliyeti düşüyor, gecikme belirsiz** | Token fiyatı yılda kabaca 10–50× düşüyor (ikincil kaynak); saniye başına token eğilimi çelişkili | Gecikme düşmezse K21-1 (model çizim döngüsünde değil) daha da güçlenir. Gecikme ve maliyet sıfıra yaklaşırsa ajanın grameri doğrudan yazması ucuzlar, ama **belirleyicilik** (aynı belge → aynı görünüm) ve denetim yine sunucuyu gerektirir; sunucu küçülür, kalkmaz (T6e). |
| V6 | **Finansta yapay zekâ denetimi sıkılaşıyor** | AB Yapay Zekâ Yasası yüksek risk yükümlülükleri ertelenmiş görünüyor (Aralık 2027 / Ağustos 2028; nihai kabul doğrulanamadı); ESMA 2024-05 rehberi; SPK taslağı bulunamadı | Sıkılaşırsa köken alanları zorunlu ve imzalı olur (`LogEntry`'ye imza/özet zinciri eklenir — ek alan); günlüğün saklama ve silme kuralları gerekir. Gevşerse köken isteğe bağlı kalabilir ama ürün ilkesi (karne, kanıt) zaten istiyor. |
| V7 | **Local-first / eşitleme motorları yaygınlaşıyor** | Zayıf-orta | Gerçekleşirse istemcide yazıp sonra birleşme beklentisi artar → CRDT tartışması geri gelir (T6a sonuçları ve hibrit önerisi). Gerçekleşmezse tek otorite daha da sadeleşir. |
| V8 | **WebGPU/WASM her yerde** | Güçlü (~%84 tarayıcı kapsamı, tahmin) | Gerçekleşmezse cihazda motor çalışmaz → devir yerine **uzak çizim** profili (sunucu PNG/sahne gönderir) korunur; `Profile`'a "uzak sunum" alanı eklenir. |
| V9 | **Ajan-araç standardı MCP'de birleşiyor** | Güçlü (2026-07-28 sürümü, Apps uzantısı) ama sıcak yol dışında | Başka bir standart kazanırsa yalnızca uyarlayıcı (`qp-mcp`) değişir: araçlar şemadan üretildiği için yeni bir standarda üretici yazmak yeter; çekirdek protokol etkilenmez (kalıcı çekirdek, değiştirilebilir kenar — ilke 10). |

## 3. Kodlama (sıcak yol) — literatür özeti

`research/futures-encodings.md` Bölüm B (kaynak: djkoloski/rust_serialization_benchmark,
2026-11-05 sonuçları; crates.io sürüm bilgileri). JSON'a göre ("log" veri kümesi):
postcard serileştirme 9,1×, çözme 2,4×, boyut 0,40; minicbor 7,4× / 1,9× / 0,45;
ciborium çözmede JSON'dan yavaş (0,62×). bincode 3.0.0 bir "mezar taşı" sürümü
(RUSTSEC-2025-0141; geliştirme durdu). İkili biçimlerin hepsi f64'ü ham 8 bayt yazar;
serde_json'ın tam gidiş-dönüşü `float_roundtrip` ister (depoda açık). FlatBuffers, Cap'n
Proto ve Protobuf ayrı şema dili istediği için "şemanın tek kaynağı Rust tipleri" kuralıyla
çelişir. Ölçülecek adaylar: CBOR (minicbor) ve postcard; sütunlu bar yükü ayrıca.
Ölçüm sonucu: `experiments/T6b-encoding.md`.

## 4. Tasarıma etkisi (kısa)

| Karar | Dayanak |
|-------|---------|
| Kendi zarfımız (`type` etiketli JSON) + JSON-RPC eşlemesi | MCP/LSP/A2A JSON-RPC kullanıyor; eşleme 1:1 olduğu için çekirdek mesajları sade tutup kenarda eşledik. Ajan yazımı için daha kısa. |
| Tek otorite + sıralı günlük + öğe düzeyinde `base` | Figma, ProseMirror, Replicache; CRDT'nin anlamsal anomalileri. Ölçüm T6a. |
| Anlamsal istek, ilkel değişim | Replicache mutator'ları; JSON Patch'in niyet ve indeks zayıflığı. Ölçüm T6c. |
| Kökene göre geri alma, ters değişim yeni girdi | Yjs `trackedOrigins`, Figma geri alma ilkesi. |
| Varlık durumu ayrı | Vega sinyalleri, Yjs awareness, MCP Apps model bağlamı. |
| Hata: `code` + `path` + `expected` + `did_you_mean` + `hint` | MCP `isError` (modelin kendini düzeltmesi), A2UI `error.path`. Ajan yazabilirlik ölçümüyle sınandı (`ergonomics/`). |
| `hello` ile yetenek ve sürüm | LSP `initialize`, MCP Apps `HostContext`, A2A Agent Card. MCP'nin durumsuz yönü bilerek alınmadı. |
| Epoch ile devir | LSP'nin belge sahipliği; kaynaklarda hazır devir modeli bulunamadı (araştırma açık sorusu 2) → kendi tasarımımız. |
