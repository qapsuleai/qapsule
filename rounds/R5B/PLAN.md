# R5B — Çalışma planı ve sözleşmeler

Bu belge round içi iş bölümünü ve alt görevlerin birbirine dayandığı **sözleşmeleri**
verir. Kesin protokol metni `docs/PROTOCOL.md` §25'tir (T3–T5 yazar); çelişkide şema
doğrudur, sonra §25, sonra bu belge.

## 0. Değişmez kurallar (her alt görev)

- Var olan dökümler (`fixtures/protocol/conformance/*.json`), altınlar ve hesap özeti
  **değişmez**. Yeni davranış yeni alanlarla gelir; alan yoksa eskisi gibi
  (`skip_serializing_if`). Dökümü bozan bir kural yalnız yeni bir yapılandırma ile açılır.
- Hesap ve doğrulama kodunda dil modeli, ağ, rastgelelik yok.
- Şema tek kaynak `qp-schema`; MCP araç tanımları şemadan üretilir (elle ikinci şema yok).
- `CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target` (ortak); worktree içinde
  ayrı `target/` yok.
- Her alt görev kendi dalında; `cargo fmt --check`, `cargo clippy --workspace
  --all-targets -- -D warnings`, ilgili crate testleri ve `qp conform` (inproc, serve,
  mcp) yeşil olmadan bitmiş sayılmaz.

## 1. Kimlik: insan, ajan, köken (T3/T4/T5 ortak)

`Principal.author` zaten `human | agent | engine`. Yeni:

- `Principal.on_behalf: Option<String>` — ajan kimliğinin adına çalıştığı kişi. Ajanın
  **okuyucu** yetkisi (veri yetkileri, lisans, kitle) o kişininkidir; fazlası değil.
  Ajanın kendi veri yetkisi satırı varsa ikisinin **kesişimi** alınır.
- **Ajan bağlantısı:** otorite bir bağlantıyı ajan kökenli işaretleyebilir
  (`Session::mark_agent_connection(conn)`; `qp mcp serve` her MCP bağlantısını işaretler,
  istemci bunu kapatamaz). Ajan kökeni = yazarı `agent` olan kimlik **ya da** işaretli
  bağlantı.
- **İnsan kararı (Arkin'in açık koşulu):** `accept_proposal`, `reject_proposal`,
  `approve_finding`, `reject_finding` yalnız **insan kökenli** komutla olur: kimliğin
  yazarı `human` **ve** bağlantı ajan kökenli değil. Aksi `forbidden`, `reason:
  "human_only"` (rol `owner` olsa bile). Bu kural koşulsuzdur (var olan dökümlerin hiçbiri
  ajanla karar vermiyor; T3 bunu `qp conform` ile doğrular). Öneri kendi yazarınca da,
  başka ajanca da onaylanamaz. MCP yüzeyinde karar araçları **listelenmez**; genel
  `command` aracıyla gönderilse bile otorite reddeder.
- **Ajan neyi yazamaz:** ajan kökenli komutta `add_fact`, `add_finding`, `update_finding`,
  `put_series` ile bar verisi, `append_bars`, `update_last_bar`, `undo`, `set_audience`
  (başkasınınki) ve kapsül/karne/hikâye değişimleri doğuran her işlem → `forbidden`,
  `reason: "agent_cannot_write"` — öneri kipinde de (öneri olarak da kabul edilmez).
  Bu kural yalnız işaretli ajan bağlantısında ya da `init.agent_policy = "r5b"` olan
  oturumda uygulanır (RD1 dökümü 05'te `scout` bulgu öneriyor; o döküm değişmez).
- Ajanın yazabildikleri: **yorum** (`add_comment`, doğrulamalı, doğrudan), **niyet**
  (`add_watch`/`remove_watch`, doğrudan, adına çalıştığı kişi için), **öneri**
  (görünüm ve çizim işlemleri: `put_view`, `insert_layer`, `set_encoding`, `insert_panel`,
  `set_emphasis`, `set_view_range`, `add_study` …, her zaman öneri olarak; insan onaylar).

## 2. Yorum ve doğrulayıcı (T4)

### 2.1 Saf doğrulayıcı — `qp_capsule::comment` (alt görev V)

```rust
pub enum Strictness { Cites, CitesNumbers, PerSentence }   // M30 a/b/c
pub struct CommentDraft { pub lang: Lang, pub text: String,
    pub cites: Vec<String>,            // olgu / kapsül / bulgu kimlikleri
    pub external: Vec<ExternalQuote> } // dış kaynak alıntıları
pub struct ExternalQuote { pub source: String, pub quote: String, pub values: Vec<f64> }
pub struct Evidence {                  // otoritenin topladığı, okuyucuya izdüşülmüş kanıt
    pub known_ids: BTreeSet<String>,   // var olan ve okuyucunun görebildiği kimlikler
    pub hidden_ids: BTreeSet<String>,  // var olan ama okuyucunun göremediği (lisans)
    pub values: Vec<CitedValue>,       // atıf yapılan öğelerin sayıları (okuyucuya açık)
    pub withheld: Vec<f64>,            // atıf ya da enstrüman kapsamında okuyucuya kapalı sayılar
    pub symbols: Vec<String> }         // enstrüman simgeleri (sayı taramasından önce çıkarılır)
pub struct CitedValue { pub id: String, pub key: String, pub value: f64, pub decimals: u8 }
pub fn validate(d: &CommentDraft, ev: &Evidence, s: Strictness) -> Result<(), Vec<Reason>>;
pub struct Reason { pub check: Check, pub code: String, pub token: Option<String>,
                    pub sentence: Option<usize>, pub hint: String }
pub enum Check { Cites, Number, Advice, License, Sentence }
```

Dört denetim (GOAL ilke 2), hepsi deterministik:

1. **Atıf:** `cites` boş değil; her kimlik `known_ids`'te. `hidden_ids`'te ise
   `license` nedeniyle (varlığını söylemeden: `code: "not_visible"`), hiçbirinde yoksa
   `unknown_id`.
2. **Sayı:** metindeki her sayı (TR `12,5` / `1.234,5`, EN `12.5` / `1,234.5`, `%`, `x`
   kat, eksi; tarih ve saat kalıpları `12 Kas`, `14:30`, yıl `2026` ayrı ele alınır:
   saat/tarih kalıbı sayı sayılmaz; simgeler önce çıkarılır) `values` ya da `external`
   içinde aranır. Eşleşme: metindeki yazılış basamağına yuvarlanmış kaynak değer metne
   eşitse (ör. kaynak 44,3187 → "44,32" ve "44,3" geçer, "44,4" geçmez). Yüzde: kaynak
   oran 0,0123 ise "%1,23" de geçer (×100). Türetilmiş sayı (fark, oran) **geçmez**: motor
   onu `values`'a koymadıysa yazılamaz (TASKS "Sayı denetimi"; ölçüm M30'da).
   `CitesNumbers` ve `PerSentence`'ta uygulanır.
3. **Tavsiye dili:** R5 `advice` taramasının listesi
   (`rounds/R5/experiments/harness/lang/tool/src/scan.rs`) ürüne taşınır; TR/EN, Türkçe ek
   kuralıyla. Her sıkılıkta uygulanır.
4. **Lisans:** metindeki bir sayı `withheld`'teki bir değerle (aynı yuvarlama kuralıyla)
   eşleşiyor ve `values`/`external`'da yoksa → `license` (`code: "licensed_value"`). Her
   sıkılıkta uygulanır. (Okuyucunun göremediği bir sayıyı dış kaynak diye işaretleyerek
   geçirme: dış alıntının değeri `withheld` ile eşleşiyorsa da red.)
5. `PerSentence`: her cümle en az bir atıf işareti taşır (`[1]` biçimi `cites` dizinine)
   ya da sayı içermez.

`Reason.hint` ajanın düzeltmesi için kısa İngilizce cümle.

### 2.2 Protokolde yorum (alt görev S)

- `Op::AddComment { id?, about: CommentAbout, lang, text, cites, external? }`;
  `CommentAbout = {capsule} | {instrument}`. Yetki: `propose` (doğrudan uygulanır, öneri
  olmaz). Otorite `Evidence`'ı **yazarın okuyucu yetkisiyle** toplar (ajan için
  `on_behalf` kişininki), `validate` çağırır; ret → `error` `code: "comment_rejected"`,
  `reasons: [Reason…]` (makinece okunur). Kabul → `ChangeOp::PutComment`; dokümanda
  `comments: Vec<Comment>` (boşsa yazılmaz), `Comment { id, about, lang, text, cites,
  external, strictness, provenance }`. Sıkılık oturum yapılandırması:
  `init.comment_strictness` (varsayılan seçilen M30 değeri).
- Okuma: `capsule` sorgusunun cevabında `comments` (okuyucunun görebileceği: atıfları
  okuyucuya görünmeyen yorum ona gitmez; sayı içeren yorum okuyucunun lisansı yetmiyorsa
  gitmez). `comments` konusu (abonelik).

## 3. Niyet (T3, alt görev S)

Var olan `Intent { text, trigger: Condition }` korunur. Ek alan `watch: Option<Watch>`:

```json
{"for":"arkin", "match":{"instrument":"SYN03"|null, "holdings":true|false,
  "meaning":["breakout"]|null, "direction":"up"|null,
  "stages":["born","weakened","invalidated","retracted"]|null, "negative":true|false},
 "expires_t": <epoch ms>|null, "max_matches": 1|null}
```

- `add_watch {id?, for?, text, match, expires_t?, max_matches?}` → `put_intent`
  (`watch` dolu). `remove_watch {intent}` → `set_intent_active false` + durum `removed`.
- Yaşam döngüsü: `active → matched (max_matches dolunca) | expired (clock ≥ expires_t) |
  removed`. Durum `Intent.state` (yoksa eski davranış).
- Eşleşme motorda: her hikâye geçişinde (kapsül motoru günlüğe yazdıktan sonra, sabit
  sırayla) etkin niyetler sınanır; `holdings: true` → kişinin kitlesindeki pozisyonlar.
  Eşleşme → `intent_matched` olayı (`intents` konusu) `{intent, capsule, stage, bar, t}`,
  okuyucuya izdüşülmüş (lisanslı hikâye yetkisize sunucu yorumu olarak). Aynı girdi →
  aynı olaylar (bayt eşit).
- Sınır: kişi başına en çok 32 etkin niyet (`welcome.limits.max_watches`); aşınca
  `limit`. Niyetle ilgili kapsül K31/K42 kuralıyla hassasiyetten bağımsız gelir (var olan
  `relevant` yolu).

## 4. Öneri ve ekransız görünüm (T5)

- Öneri yolu var (§4.7). Yeni: (a) §1'deki insan kararı kuralı; (b) bekleyen önerinin
  **önizlemesi**: `preview {proposal, width?, height?}` sorgusu → önerinin işlemlerini
  dokümanın bir kopyasında açıp etkilediği görünümün sahne tarifi özetini verir (doküman
  değişmez); (c) MCP'de `render` aracı: görünümü (ya da önizlemeyi) PNG'ye çizer
  (`qp-render`), MCP `image` içeriği olarak döner.

## 5. MCP sunucusu (T2, alt görev M)

- `qp mcp serve --stdio` ve `qp mcp serve --http [--port N]` (varsayılan bağ adresi
  `127.0.0.1`; başka adres yalnız `--listen <addr> --token-env <VAR>` ile, istekte
  `Authorization: Bearer`). HTTP'de `Origin` başlığı varsa yalnız `http://127.0.0.1`/
  `http://localhost` kabul (DNS rebinding). Streamable HTTP: `POST /mcp` JSON cevap; `GET
  /mcp` SSE ile bildirimler.
- Oturum kaynağı: `--source synthetic|local|sim`; sentetik varsayılan (yeniden oynatma,
  K44); `--speed`, `--running`. Kimlik: `--principal <id>` (yazarı `agent` olmalı; değilse
  başlamaz). Sunucu varsayılan olarak yalnız bu kimliği sunar; farklı `X-Qp-Principal` →
  403. Birden çok kimlik yalnız açıkça: `--principals a,b` (yalnız loopback) ya da belirteç
  başına kimlik `--token-env a=VAR1,b=VAR2` (kimlik belirteçten, başlık yok sayılır); eski
  oturumda kimlik `Mcp-Session-Id`'ye bağlıdır (güvenlik incelemesi, `r5b/m-principal`). Her MCP bağlantısı ajan kökenli işaretlenir (§1).
- Araç yüzeyi `--surface <ad>` ile seçilir; yüzeyler `crates/qp-mcp/src/surfaces/`
  altında, her biri tek dosya, `surfaces::registry()`'de bir satır. Araç girdi şemaları
  `qp-schema` tiplerinden (`schemars`) üretilir.
- Ölçüm günlüğü: `--log <dosya>` her araç çağrısını `{run, tool, args_bytes,
  result_bytes, ok, ms}` satırı olarak yazar (ölçüm için; ürün değil).
- `qp demo --mcp`: gösterim barındırıcısı aynı oturumu `/mcp` ucunda da sunar; insan
  web kabuğunda, ajan MCP'de aynı dokümanı okur ve yazar.

## 6. Araç yüzeyi alternatifleri (M28)

| Ad | Tarif |
|----|-------|
| `thin` (a) | Protokolü yansıtan araçlar (bugünkü üretilmiş liste) + yeni işlemler. |
| `task` (b) | 6–9 görev düzeyi araç: ne değişti, kapsülü açıkla, enstrüman hikâyeleri, bul, izle, yorum yaz, öner, çiz. |
| `query` (c) | Tek sorgu aracı (küçük seçici dili) + tek yazma aracı. |
| `mixed` (d) | Görev düzeyi araçlar + genel `query` kaçış aracı. |

Her alternatif ayrı alt ajanın, ayrı dalda (`r5b/m28-<ad>`), yalnız kendi
`surfaces/<ad>.rs` dosyasına yazdığı uygulamadır; birbirini görmez. M29 (kapsülün
veriliş biçimi: `json`, `card`, `layered`) seçilen yüzeyde parametre olarak ölçülür.

## 7. Düzenek (H) — alternatiflerden önce commit

`rounds/R5B/experiments/harness/`: görev kümesi (40+), doğru cevap üretici (motordan:
`qp serve`/MCP `thin` yüzeyiyle sorgu), puanlama (deterministik betik), bozuk/doğru yorum
kümesi, gömülü talimat örnekleri (≥ 10), ölçüm sürücüsü (alt ajan istemi + `qmcp`
istemcisi). Ayrıntı `harness/SPEC.md`.

## 8. İş sırası

1. T0 ✓, T1 (araştırma) ve H (düzenek) paralel; H commit'i M28 dallarından önce.
2. V (doğrulayıcı), S (otorite: kimlik, yorum, niyet, önizleme, §25), M (MCP sunucusu)
   paralel, ayrı dallarda; birleştirme sırası V → S → M.
3. M28 a–d (dört alt ajan, ayrı dallar) → ölçüm (üç model boyu) → seçim; M29.
4. M30 (doğrulayıcı sıkılığı), R1–R3 başvuru ajanları, M31, Matriks bağlamı ve gömülü
   talimat sınaması.
5. T6 gösterim izi, T7 `docs/AGENTS.md`, rapor, temizlik, bağımsız denetim.
