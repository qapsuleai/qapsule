# Programlar — oturuma bağlanma ve protokol (RL1)

Durum: **RL1 şartnamesi** (koddan önce; normatif). Dil `docs/PROGRAMS.md` (`qp-ir/0`; bundan
sonra **PR §n**); `docs/PROTOCOL.md` §26 buraya işaret eder; kavramlar `docs/CONCEPTS.md`
"Hesap dili (RL1)"; parametreler `docs/PARAMETERS.md` "RL1". Bağlayıcı karar: Arkin'in D1'i
(`rounds/RL1/DECISION.md`) ve RL1 şartname eleştirisi kararları
(`rounds/RL1/experiments/spec/CRITIQUE-A.md`, `CRITIQUE-B.md`).

**Değişmezler.** `qp-authority/src`'ye satır eklenmez; yeni koşul türü, yeni koleksiyon, yeni
`ChangeOp`, yeni konu yoktur. Protokole yalnız ekleme: yeni alanlar isteğe bağlıdır ve yokken
yazılmaz; var olan mesajlar, dökümler, altınlar, hesap özeti, `welcome` ve `add_study` davranışı
değişmez. `ConditionKind` ve `ValueSource` kayıt kolları bu round kullanılmaz (D1); eksik
kancalar §7'de, ayrıntısı `rounds/RL1/REPORT.md`'de (D1 şart 3).

## 1. Dokümanda program (D1)

### 1.1 Temsil: çıktı başına bir hesap serisi, ara gösterim satır içi

Yalnız **seri** ve **olay** çıktıları dokümana girer (sinyal ve durum tanımı yalnız `dene`;
PR §5). `SeriesSource::Study`'ye isteğe bağlı, eklemeli `program` alanı:

```json
{"id":"p1.kirilim","instrument":{"symbol":"THYAO","exchange":"BIST","calendar":"XIST"},
 "interval":"1h",
 "source":{"kind":"study","study":"program","inputs":["px"],"params":{},"output":"kirilim",
   "program":{"instance":"p1","program_id":"sha256:…","instance_id":"sha256:…",
     "scope":"document","owner":"arkin","since_t":1764151200000,
     "params":{"len":20},"ir":{"version":"qp-ir/0","params":[…],"nodes":[…],"outputs":[…]}}}}
```

| Alan | Kural |
|---|---|
| `study` | `"program"` ⇔ `program` var (değilse `invalid`). Eski okuyucu bilinmeyen hesap görür, yanlış hesaplamaz |
| `inputs` | tam bir bar serisi (bağlı ya da belge içi); bağımlılık kapanışı ve besleme yetkisi bunun üstünden, var olan yolla |
| `params` | `{}`; bağlanan değerler `program.params`'ta tiplidir (PR §2.1) |
| `output` | var olan alan: programın çıktı adı |
| `program.instance` | örneğin doküman adı (`add_program` `id`'si); seri kimliği `<instance>.<output>` |
| `program.program_id`, `instance_id` | PR §2, §2.1; açılım yazar. Kanıt doğrulamada "tanım değişti"yi "veri değişti"den ayırmanın tutamağı (RS1/R7) |
| `program.scope` | bu round yalnız `"document"` (her okuyucu görür). R7 kişisel izleme için `"personal"` ekler. **RL1'de pozisyondan türeyen seviye programa yazılmaz** |
| `program.owner` | bütçenin sayıldığı kişi (§2.3); açılım yazar, istemci yazamaz |
| `program.since_t` | örneğin başlangıç anı: açılımın saati (`qp_ops::Ctx::at`); `set_program_params` yeniler (§1.2) |
| `program.params` | bağlanmış **tam** değer kümesi (varsayılanlar dahil, anahtar sırasıyla) |
| `program.ir` | kanonik ara gösterim. **Günlükte yalnız ara gösterim**, metin hiç yok |

- `program` yoksa serileştirme bugünküyle **bayt eşittir** (`skip_serializing_if`).
- **Neden satır içi?** Her seri kendi başına yeterlidir: `qp-plane` değeri, bağımlılık kapanışı,
  besleme yetkisi, `remove`/`in_use`, geri alma, yeniden oynatma ve yansı hiçbir çapraz başvuru
  çözmeden çalışır. Paylaşılan başvuru ya her tüketiciye çözüm kuralı ya yeni koleksiyon ister;
  ikincisi otoritenin yumuşak dosyasına (`capsule/public.rs`) dokunur. Satır içi temsil otoriteye
  sıfır satır ister.
- **Günlük büyümesi sınırlıdır:** ara gösterim çıktı sayısı kadar yinelenir ve her
  `set_program_params` çıktı başına `put_series` yazar. Sınırlar: `program.max_ir_bytes`
  (8192) × `program.max_outputs` (4) = komut başına ≤ 32 KiB; sahip başına günlükteki program
  yazısı toplamı `program.live.doc.max_log_bytes` (§2.3). Kötü niyet sınaması tekrarlı
  `set_program_params` içerir.
- Program serisi yalnız program işlemleriyle yazılır: istemcinin `program` taşıyan
  `put_series`'i, program serisine `set_params` ve tek çıktısının `remove`'u `invalid` /
  `E_PROGRAM_SERIES` (denetimsiz ara gösterim ve bütçe atlatma yok).

### 1.2 Olay serisi ve canlı yol

| Çıktı | Seri değeri | Canlı bar |
|---|---|---|
| `series` | düğüm değeri | geçici değer (PR §5.3) |
| `event` | kapanmış barda olay barında **1**, öbür kapanmış barlarda **0**; `known_t ≤ since_t` olan barlarda **0** | **`null`** |

- **K13 her yerde tutar:** vektörler bar sayısı uzunluğundadır; olay serisinde canlı barın değeri
  `null`'dur (önbellekte ve telde aynı).
- **Değerlendirilebilir uzunluk.** `qp_plane::values::Computed` olay serisi için kapanmış bar
  sayısını taşır; `Computed::len()` onu döner (öbür serilerde `t.len()`), `eval_on` son iki değeri
  `main_values()[..len]` üstünde alır. Otorite tarafındaki tüketici (`claims::at_index`, `eval_at`)
  zaten `len()` kullanır: sıfır satır. `len()`'in anlamı değiştiği için `qp-plane`, `qp-grammar`,
  `qp-ops`, `qp-read` tüketicileri taranır; tam uzunluk gereken yer `t.len()` okur. Otoritede tam
  uzunluk isteyen bir tüketici çıkarsa uygulayıcı durur ve sorar.
- **Canlı yolun koşulu var olan `threshold`:** `{"kind":"threshold","series":"p1.kirilim",
  "op":"ge","value":1}`. Son değerlendirilebilir değer son kapanmış barınkidir: koşul olay barı
  kapanınca (sonraki barın ilk verisinde) doğru olur, iddianın `t`'si olay barı, olgunluk hemen
  `final` (`claims::closed`); `provisional` aşaması yok (X30/3).
- **Yalnız `threshold`.** Olay serisi `compare` işleneni olamaz ve `threshold` dışında koşula
  giremez (kapanmış bar öbür serinin canlı barıyla karşılaştırılırdı): `qp-program` `Check`
  fazında `add_intent`, `add_finding`, `update_finding` koşullarını tarar, `unsupported` /
  `E_EVENT_OPERAND`. Abonelik (`condition` konusu) alt sistemce reddedilemez; bu bir sınırdır.
- **Başlangıç anı (`since_t`).** Koşul oluşturulurken tetiklenmez (otoritede niyet ve abonelik
  ilk değerlendirmede kenar durumunu yalnız kaydeder: `eval_intents`, `eval_watchers`). Ama
  `set_program_params` var olan bir koşulun altındaki seriyi değiştirir: son kapanmış barın değeri
  0'dan 1'e dönerse örnek **var olmadan önce** kapanmış bir bar için iddia doğardı (R7'de seviye
  sürükleme, RS1'de as-of ihlali). Bu yüzden olay serisi `known_t ≤ since_t` barlarda 0'dır;
  geçmiş olaylar dokümanda görünmez, `dene` ile okunur. Belirlenimcidir (`since_t` günlükte).
  Saat bar zamanlarının ilerisindeyse (sabit saatli sınama, eski veri) olay doğmaz: dökümler
  ve sınamalar saati veriye göre kurar.
- **Bildirim birleşmesi (D1 şart 2).** Olay serisi **her** olay barını kaydeder; kanıt ve `dene`
  bar bar doğrudur. `threshold` kenar tetiklidir: art arda olay barları (1, 1, …) tek
  `condition_met`/`intent_triggered` verir.
- **Toplu eklemede kayıp (sınır).** Canlı yol koşulu bir girdide bir kez, son değerde
  değerlendirir. Tek girdide birden çok bar kapanırsa (`append_bars` yığını, yeniden çözüm,
  hızlı yeniden oynatma) aradaki 0,1,0 deseni **bildirimsiz ve iddiasız** geçer. Seri ve kanıt
  yine doğrudur. Gerçek çözüm bar başına değerlendirmedir (`ConditionKind`; §7).
- **Seans sonu gecikmesi (sınır).** Bar ancak sonraki bar gelince kapanır: seansın son barındaki
  olay ertesi seansın ilk verisinde bildirilir. Kayıtlar ve `dene` `known_t`'yi (takvim
  kapanışı) taşır; kapanışın seans kapanışı olayıyla tetiklenmesi R7 notu (§8).
- **Görünür kılma:** `check` statik `W_CONSECUTIVE_EVENTS` uyarır (PR §11.3); `dene` her olay
  çıktısı için `events`, `runs` (art arda öbekler), `notifications` (= `runs`, üst sınır),
  `consecutive` (`runs < events`) ve toplu eklemeyle kaybolabilecek tek barlık öbek sayısını
  (`isolated`) yazar. İki sınır `docs/AGENTS.md`'de de yazılır.
- Geçmiş bar düzeltmesi seriyi baştan kurar; kayıtlı iddialar var olan kuralla yeniden sınanır
  (`PROTOCOL.md` §19.3 adım 3).

### 1.3 Değerlerin hesabı (`qp-plane`)

- `values::values` `program` taşıyan hesap serisini `qp-lang` çalıştırıcısıyla hesaplar; hiza
  kaynağı bağlı serinin enstrümanından (PR §3.2). Her çıktı serisi yalnız kendi çıktısının
  bağımlılık konisini koşar (canlı bütçe PR §8.1 bunu sayar).
- **Artımlı yol:** bugünkü `Live { study: Box<dyn qp_compute::Study>, … }` yalnız kapanışı
  `push` eder; program bütün bar alanlarını ister. `Live`'a program kolu eklenir:
  `Delta::Appended` → önceki canlı barı `commit`, yeni barı `peek` (değer + olay serisinin
  değerlendirilebilir uzunluğu bir artar); `Delta::Revised` son barda → `update_last` + `peek`;
  öbür her şey (geçmiş düzeltme, kesme) → baştan kurma. Tik başına baştan koşu yok. Bit eşit.
- **Ölçüm** (kabul ölçütü 17): 500 barlık seride 1, 8 ve 32 örnekle tik başına µs (son bar
  düzeltmesi ve yeni bar), K17 bütçesine oranla; WASM'da aynısı.
- **WASM (D1 şart 1):** `qp-plane` kabuk paketindedir; `qp-lang` bağlanınca paket ölçülür. İ8
  tavanı (ön iş 1'den sonraki değer) aşılırsa **dur ve sor**; önerilecek yol özellik bayrağı ya
  da izleyicide program hesabı yok ("hesaplanmadı").

### 1.4 Çizim

`add_program` `show {view, with?, panel?}` (isteğe bağlı) çıktı başına katman ekler: `series` →
`line`, `event` → `bar` (0/1 sütunları), varsayılan yeni panel. Var olan görünüş (`tavek`);
lejantı çekirdek kurar; köken çizgi biçimi kuralı (ajanın eklediği kesik). Program yazma ekranı
yok (R7). Arayüze dokunan iş `qapsule-design` becerisiyle.

## 2. Alt sistem `qp-program`

### 2.1 Kayıtlar

Oturum katmanında yeni crate; `docs/ARCHITECTURE.md` "Yeni alt sistem nasıl eklenir". İşlem
açılımı yalnız komut yolunda gerekir: yalnız `compose.rs` `LIST`'ine (`FOLLOWER_LIST`'e değil;
izleyici program serilerini `qp-plane` üstünden hesaplar).

| Kayıt | Ne |
|---|---|
| `OpHandler` | `add_program`, `set_program_params`, `remove_program` (§3.1) → `put_series`, `insert_layer`, `delete_layer`, `delete` |
| `Check` fazı | program işlemlerinin tam denetimi (PR §11), bütçe (§2.3), `E_PROGRAM_SERIES`, `E_EVENT_OPERAND`, `E_TRY_ONLY` korumaları (komuttaki bütün işlemler taranır) |
| `QueryHandler` | `check_program`, `try_program`, `explain_program` (§3.2) |

### 2.2 Politika

| İşlem | Yetki | Öneri olabilir | Ajan |
|---|---|---|---|
| `add_program` | `write` (öneri için `propose`) | evet | **yalnız öneri** (yeni kimlikle); `mode: direct` → `forbidden`, `proposal_only` |
| `set_program_params` | `write` | evet | **yalnız öneri** |
| `remove_program` | `write` | evet | asla (`agent_cannot_write`; `remove` ile aynı sınıf) |

Ajanın eklediği program öneridir; **kararı insan verir** (`PROTOCOL.md` §25.2): var olan
`accept_proposal`/`reject_proposal`, yalnız insan kökenli komutla. MCP ucunda karar aracı yok.
Kamu dünyası ve okuyucu izdüşümü `add_study` serileriyle aynı kuraldan geçer.

### 2.3 Bütçe ve sahip

- **Sahip** (`owner`) = kabul edilen öneride karar veren insan (`Ctx::approved_by`), doğrudan
  komutta konuşan (`Ctx::speaker`). Ajan doğrudan program yazamadığından sahip her zaman bir
  insandır; ikisi de `Ctx`'te, belirlenimci.
- **Denetim yeri:** `Check` fazında, `SessionView::param` ile (PR §8.2): örnek sayısı, `Σ
  live_per_bar`, günlük baytı (sahibin program serisi yazan girdilerinin değişim baytları,
  `SessionView` günlüğünden). Öneri oluşurken de öneriyi verenin dokümanına karşı denetlenir.
- **Kabulde yeniden denetim:** `Check` fazı kabulde `accept_proposal`'ı görür, `add_program`'ı
  görmez. Bu yüzden aynı denetim açılımda (`OpHandler::expand`) da koşar; `qp_ops::Ctx`'te
  `param` olmadığından **kayıt defteri varsayılanıyla**. Aşarsa kabul `error` döner, öneri
  `pending` kalır (`PROTOCOL.md` §4.7). Eksik kanca: açılımda `param_for(principal)` (PR Açık
  sorular 1).

## 3. Protokol

Şema `qp-schema`: `Program` ve düğüm tipleri, `ProgramParamDef`, `ProgramParamValue`,
`ProgramRef`, `SeriesSource::Study.program`, `Op::AddProgram`/`SetProgramParams`/
`RemoveProgram`, `Query::CheckProgram`/`TryProgram`/`ExplainProgram`, `ProgramError`,
`ProtocolError.errors`. Şema dışa aktarımına yalnız yeni tipler eklenir.

### 3.1 İşlemler

| `op` | Alanlar (`?` isteğe bağlı) | Etki / açılım |
|------|----------------------------|---------------|
| `add_program` | `id?`, `program` (ara gösterim), `inputs` (tam bir bar serisi), `params?`, `show?{view, with?, panel?}` | Tam denetim ve bütçe, sonra çıktı başına `put_series` (`<id>.<çıktı>`, çıktı sırasıyla), `show` varsa çıktı başına `insert_layer`. `id` yoksa `prog-<seq>-<n>`; var olan kimlikle çakışan çıktı `duplicate`. Sinyal/durum çıktılı program `E_TRY_ONLY`. `ack.created`: seri kimlikleri, sonra katman kimlikleri |
| `set_program_params` | `program` (örnek adı), `params` | Değerleri birleştirerek değiştirir; aralık ve bütçe yeniden; her çıktı serisi için `put_series` (kimlik ve katmanlar korunur), yeni `instance_id` ve `since_t`. Ara gösterim değişmez |
| `remove_program` | `program` | Bütün çıktı serilerini `remove` kuralıyla siler (çizen katmanlar önce); bir çıktıya koşul ya da öğe dayanıyorsa `in_use`, `expected` = dayananlar |

- **Hatalar hepsi birden:** üst düzey hata ilk hatanın `code`, `reason`, `path`, `expected`,
  `did_you_mean`, `hint`'ini taşır; isteğe bağlı `ProtocolError.errors` bütün hataları
  (`ProgramError`, PR §11.1). Bir hata varsa hiçbir işlem uygulanmaz. Yeni hata kodu yok;
  eşleme PR §11.2; `path` iletiye göre (`/ops/0/program/nodes/3/op`), `at` ara gösterime göre.
- **Metin kabul edilmez:** işlem `source` taşımaz (`schema`). Metin yüzeyde `check_program`
  ile ara gösterime indirilir (X30/2).
- Değişimler var olan `ChangeOp`'lardır; geri alma ve günlüğün yeniden oynatılması değişmez
  (`qp_ops::apply::replay` alt sistem kodu istemez).

### 3.2 Sorgular

Yan etkisiz. Program `program` (ara gösterim), `source` (metin) ya da `instance` (örnek adı) ile,
tam olarak biri (`invalid`). `inputs` bar serisi, `params` bağlanacak değerler, `lang` (`en`
varsayılan, `tr`).

| `kind` | Alanlar | Cevap (`data.kind`) |
|--------|---------|---------------------|
| `check_program` | `program?`/`source?`, `inputs?`, `params?`, `lang?` | `program_check`: `ok`, `errors[]` (metinde `span`), `warnings[]`, `cost {per_bar, live_per_bar, bars?, total?, limit}`, `outputs [{name, kind, documentable}]`, `params`, `program_id`, `limits` (geçerli değerler), `ir` (`source` verildiyse), `reject?`. `inputs` yoksa saat ve bütçe bağdan bağımsız. Program hataları cevap içindedir |
| `try_program` | `program?`/`source?`/`instance?`, `inputs?`, `params?`, `range?`, `lang?` | `program_try` ("dene", yeniden oynatma kipi, PR §8.3): `from_t`, `to_t`; `series` (tanımlı sayısı, son değerler), `event` (`events`, `runs`, `notifications`, `consecutive`, `isolated`, ilk/son olaylar `{t, known_t}`), `signal` (sayı, sonuç dağılımı, ilk kayıtlar PR §6 biçiminde, `mode: "replay"`, `label: "simulated"`), `state` (ayrı anahtar sayısı, en sıklar), `dropped`, `cost`, `work {used, limit}`, `stopped?`. Okuyucunun veri yetkisiyle (`no_entitlement`, `no_data`) |
| `explain_program` | `program?`/`source?`/`instance?`, `inputs?`, `params?`, `at?{output, t}`, `lang?` | `program_explain`: `text` (kanonik metin), `program_id`, `instance_id?`, `params`, `cost` (düğüm başına `{id, op, clock, w}`), `outputs`; `at` ile o değerin kökeni (PR §9) |

### 3.3 Taşımalar ve yüzeyler

Süreç içi, `qp serve` ve MCP aynı mesajları taşır. MCP araçları (ince yüzey, K86/6) sorgu ve
işlem adlarıyla: `check_program`, `try_program`, `explain_program` (metin ya da ara gösterim),
`add_program` (metni önce ara gösterime indirir, komutu gönderir; MCP bağlantısı ajan kökenli,
öneri olur). Karar aracı yok. Şemadan üretilen `op_*` araçları varsa yeni işlemler de oraya düşer;
politika onları yine sınırlar. Komut satırı: `qp program check | run | explain <dosya>` (metin ya
da JSON; `run` = `try_program` çekirdeği, kapalı pencere).

**Komut satırı biçimi (RX4 uygunluğu buna dayanır):**
- `qp program check [--json] [--lang tr|en] <dosya>`: `program_check` cevabı; ret `ok:true, reject{code}`, çıkış 0.
- `qp program run [--json] [--full] --data <seri.json> <dosya>`: veri `fixtures/bars` biçiminde
  (`qp_schema::Series`, gömülü barlar). Dosya **kapalı pencere** sayılır: son bar da kapanmıştır,
  canlı bar yoktur (PR §1). `--full` olmadan `try_program` özeti; `--full` ile bar başına tam çıktı:
  `outputs` adla eşlem; seri `values` (bar başına, `null` dahil), olay `bars` (indeks, artan) ve
  `t`, sinyal `records` (PR §6 kayıt biçimi + `bar`, `exit_bar` indeksleri), durum `keys` (bar
  başına dizi ya da `null`) ve `counts`.
- `qp program explain [--json] [--lang tr|en] <dosya>`: `program_explain` cevabı.

## 4. Uygunluk dökümleri (eklenecek: 71–77)

Üç taşımada; var olan dökümler değişmez. Üreten örnek `qp-program` ya da bir sınama crate'i
altında, `-- --write` ile.

| # | Ad | Kapsar |
|---|---|---|
| 71 | `program-add` | `add_program`, `put_series` değişimleri, `ack.created`, `study` (canlı bar `null`), aynı programın iki örneği, `set_program_params` (`since_t`), `remove_program`, `in_use`, `E_TRY_ONLY` |
| 72 | `program-check` | `check_program`: hepsi birden (`at`, `span`, `did_you_mean`), alt kodlar, uyarılar, maliyet, metinden `ir`, ret; `add_program`'da `errors[]` |
| 73 | `program-try` | seri/olay/sinyal/durum özeti, `consecutive`, `isolated`, `simulated` kayıtlar, bütçeyle durma, `no_entitlement`; aynı `from_t` ile `dene` olayları = dokümandaki olay serisi (olay sayısı > 0 güvencesi) |
| 74 | `program-draw` | `show` ile katman; `screen` ve `hit`te program serisi |
| 75 | `program-event` | `threshold ge 1`: kapanmış barda `condition_met`, kanıt `final`; art arda olayda tek bildirim; çok barlı `append_bars` ile kayıp sınırı; parametre değişiminde geriye dönük tetik yok (`since_t`); geçmiş düzeltmede yeniden sınama; `compare`'de `E_EVENT_OPERAND` |
| 76 | `program-agent-proposal` | ajanın `add_program`'ı öneri, `direct` → `proposal_only`, `remove_program` → `agent_cannot_write`, insan kabulü (`approved_by` = `owner`), kabulde bütçe yeniden denetimi |
| 77 | `program-replay` | günlükten yeniden oynatma bayt eşit (program serileri dahil), geri alma, yansının aynı değerleri |

## 5. Otorite dışında dokunulan yerler

| Crate | Dosya | Neden |
|---|---|---|
| `qp-schema` | `types/series.rs`, yeni `program.rs` | `Study.program`, `ProgramRef`, ara gösterim tipleri |
| `qp-schema` | `protocol/ops.rs`, `protocol/query.rs`, `protocol/envelope.rs` | üç `Op`, üç `Query` ve cevapları, `ProtocolError.errors`, `ProgramError` |
| `qp-schema` | `params/system.rs` | RL1 kayıt defteri satırları (`docs/PARAMETERS.generated.md` yeniden üretilir) |
| `qp-compute` | `studies/` (`stdev`, `percentrank`), özet örneği | yeni çekirdekler; `STUDY_NAMES` değişmez |
| `qp-lang` (yeni) | — | denetim, çalıştırıcı, bütçe, metin dil |
| `qp-plane` | `values.rs` (`values`, `Live` program kolu), `values/computed.rs` (`len`), `error.rs` (`ProtocolError` değişmezi), `Cargo.toml` | hesap yolu |
| `qp-ops` | `expand/op.rs` (yeni işlemler `unserved` listesine, `AddWatch` gibi), `expand/describe.rs` (işlem adı, kapalı eşleme) | yeni `Op` kolları |
| `qp-ops` | `expand/op/series.rs` (`SeriesSource::Study` değişmezleri 93, 142, 236), `select.rs`, `select/rows.rs`, `summary.rs` (desenler) | yeni alan (`program: None` ya da `..`) |
| `qp-grammar`, `qp-read`, `qp-ops` | `Computed::len` tüketicileri | §1.2 tarama |
| `qp-program` (yeni) | — | alt sistem |
| `qp-session` | `compose.rs` `LIST` (+ `lists` sınaması), `Cargo.toml` | kayıt satırı |
| `qp-mcp`, `qp-cli`/`qp-host` | araçlar, `qp program` | yüzeyler |
| `scripts/arch-config.json` | `layers` | `qp-lang` `alan` sırasının başına (yalnız şema ve çekirdeğe bağlı), `qp-program` `oturum` sırasında `qp-watch`'tan sonra; `config` adımı kural değişikliği satırı ister |
| `qp-authority` | — | **sıfır** (eleştiri A kodla doğruladı: `Study` desenleri `..`, `ProtocolError` değişmezi yok, `agent.rs` joker) |

## 6. Bilinen sınırlar (özet)

Toplu eklemede 0,1,0 kaybı ve art arda olayda tek bildirim (§1.2); seans sonu gecikmesi;
`condition` aboneliğinde olay serisinin `compare` işleneni olması engellenemez; kişi başına
plan değeri çözülemez (PR Açık sorular 1); `since_t` öncesi olaylar dokümanda görünmez.

## 7. Eksik kancalar (D1 şart 3; ayrıntı `REPORT.md`)

1. `ConditionKind`: `Condition`'a kol; `qp-authority` `data/live.rs` `eval_slots` ve
   `capsule/claims.rs` `holds_at` kapalı eşlemeleri; canlı yol belleğinin türsüz koşulu yeniden
   değerlendirmesi (`cond_ids`). Bar başına değerlendirme (toplu eklemede kayıp) buna bağlı.
2. `ValueSource`: çizim, değer önbelleği, bağımlılık kapanışı, veri yetkisi yolları.
3. `param_for(principal, key)`: `SessionView` ve açılım bağlamı.
4. Alt sistemin abonelik reddi (`Subscribe` fazında ret yok).

## 8. R7 ve RS1 notları

- **R7:** çizim bağı `line` parametrelerinde `{"bind": {drawing, anchor, field}}`; eklemeli
  `line.tf?` (ara değerin periyodu; günlükte çizilen çizgi 1m tabanda farklı geometri vermesin);
  sürükleme bitince tek komut (`replace_program` ya da başvuruyla ara gösterim; günlük
  büyümesi); `scope: "personal"` (`watch_visible` kuralı); olay ve sinyal çıktısına isteğe bağlı
  `evidence: [başvuru]` (seviye, fiyat canlı kanıta); kapanışın seans kapanışı olayıyla
  tetiklenmesi; izleme görevi = `add_watch {match: {series}}` mı niyet mi (R7 öncesi karar); ajanın
  kendi izleme programına doğrudan yazıp yazamayacağı (K72 "kur ve göster").
- **RS1:** canlı sinyal kaydı PR §6 biçimiyle `mode: "live"`; iki aşamalı kırılım
  `rule.kind: "breakout" {level, hold_bars, max_bars, direction}`, olay çıktısına isteğe bağlı
  `rule`; `ClaimEvidence`'a eklemeli `program {instance_id}`, parametre değişiminden önceki
  iddiaların "tanım değişti" işareti; birden çok doküman üstünden kişi başına bütçe, örnek başına
  en çok açık kayıt, RX5 HB ile `work` tek birim; maliyet cevabına `bars_per_day`; durum
  tanımına sürekli özellikler (`features`); ortak sistem stratejilerinin kişi bütçesine
  sayılmaması (K76); `source {strategy, version}` görünen addır, toplama kimliği
  `program_id`/`instance_id`.
