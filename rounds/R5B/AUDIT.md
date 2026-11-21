# R5B — Bağımsız denetim (görev A)

- **Denetçi:** bağımsız alt ajan. R5B'nin hiçbir tasarım, kod, ölçüm ya da rapor işini yapmadım.
- **Taban:** `main` `ab33a7e` (denetim sırasında ana akış `bcb6619` REPORT commit'ini ekledi; kod
  değişmedi). `CARGO_TARGET_DIR` ana ağacın `target/`'ı.
- **Yazdığım yer:** yalnız bu dosya. Kod, test, döküm, altın ve başka belgeye dokunmadım; commit,
  push, birleştirme yapmadım. Denemelerimi depo **dışında**, scratchpad'de ayrı bir sınama
  kasasında (`r5b-audit-probe`, depo kasalarına yol bağımlılığıyla, `--offline`) koştum; iş bitince
  sildim. Ağ çağrısı yok (yalnız `127.0.0.1` üstünde `qp demo`/`qp serve` süreçleri); ajan,
  `mxmcp`, `.env.local` çalıştırılmadı/açılmadı.
- **Yapmadıklarım:** tam `scripts/verify.sh` (ana akış aynı anda koşuyordu; hedefli
  `cargo test -p qp-mcp --test human_decision`, `-p qp-session --test agent_policy --test
  agent_reader --test agent_comments` yeşil: 3 + 8 + 14 + 5). CI, preflight ve `clean.sh`
  kapanış koşusu ana akışın işi.

## Hüküm: **KOŞULLU KABUL — bir kritik bulgu kapanmadan ölçüt 6 "Tamam" sayılamaz**

Arkin'in koşulu **MCP yolunda tutuyor**: otoritede tek giriş noktası, karar işlemi her kipte ve
her konumda ajan kökenine kapalı, ajan ucunda karar aracı yok, testi var ve geçiyor; kendi
denemelerimde de kırılmadı. Düzeneğin doğru cevapları motordan geliyor (yeniden ürettim, üç dosya
bayt eşit) ve seçim kuralları alternatiflerden önce commit edilmiş. Ölçüm sayıları DECISIONS
"Önerilen (R5B)" ile tutarlı. Gizli bilgi taraması temiz.

Ama:

1. **Kritik (B1):** yetkili okurun ajanı lisanslı bir sayıyı biçim değiştirerek (`5:23`,
   `5.23.2026`, "beş virgül yirmi üç") yoruma koyabiliyor; doğrulayıcı geçiriyor **ve yetkisiz okur
   o yorumu alıyor** (olay, `capsule` cevabı, `document`). İlke 3 ("lisanslı sayıyı yetkisiz okura
   taşıyamaz") ve ölçüt 6 bu yolda tutmuyor. Yetkisiz okurun ajanı da aynı biçimlerle lisans
   denetimini geçiyor.
2. **Önemli (B2):** `qp demo --mcp`'de insan bağlantısı kimliksiz: aynı makinedeki herhangi bir
   süreç (Bash'i olan aynı Claude Code dahil) `GET /events?principal=arkin` ile insan bağlantısı
   açıp ajanın kendi önerisini **kabul ettirebildi** (gösterildi). MCP üzerinden değil, ama
   "insan kökeni" bu barındırıcıda kanıtlanmıyor.
3. **Önemli (B3):** `qp demo --mcp --lan` (ya da `--bind` yerel olmayan) `/mcp`'yi **belirteçsiz**
   ağa açıyor; GOAL "Kimlik ve gizlilik" ve Ö101'e aykırı.
4. **Önemli (B4, B5):** doğrulayıcının sayı ve tavsiye taraması kendi bozuk yorum kümemde zayıf:
   uydurma sayı 4/15, lisanslı sayı 9/13, tavsiye 4/15, yön 2/6 yakalandı. M30'un 149/149'u
   düzeneğin kendi kümesinde doğru, ama genellemiyor.

## Bulgular

| # | Önem | Başlık |
|---|------|--------|
| B1 | **kritik** | Lisanslı sayı, maskelenen sayı biçimleriyle (saat, tarih, sözcük) yorumdan yetkisiz okura geçiyor |
| B2 | önemli | `qp demo --mcp`: kimliksiz insan kanalı üzerinden ajanın önerisi "insan" olarak kabul edilebiliyor |
| B3 | önemli | `qp demo --mcp --lan` / `--bind`: `/mcp` ağa belirteçsiz açık |
| B4 | önemli | Doğrulayıcı sayı taraması: uydurma sayı kaçış yolları (maskeler, sözcük, üst simge, kendi beyanı dış alıntı, değer aklama) |
| B5 | önemli | Tavsiye listesi ajan metni için dar (çekimler, eşanlamlılar, görünmez karakter, benzer harf) |
| B6 | küçük | Replay denetimi ajan kuralının dışında; `owner` rollü ajan işaretli bağlantıda replay'i yönetebiliyor (Ö95 beyanı geniş) |
| B7 | küçük | M28/M29 "sızıntı 0" davranışsal: ajanın klasöründe sahibin dokümanı (`--dump`), aynı bağlamda sahip görevleri, kimlik başlıkla seçilebilir |
| B8 | küçük | Eski `qp mcp --init … --principal <insan>` bağdaştırıcısı işaretsiz bağlantı ve karar araçlarıyla MCP istemcisine insan kimliği veriyor |
| B9 | küçük | `crates/qp-wasm/target/` (293 MB) izlenmeyen artık; `git status` temiz değil |
| B10 | not | Yön denetimi ve kimlik-metin denetimi bilerek gevşek; kapsamı raporda daha dar yazılmalı |
| B11 | not | `qp mcp serve` HTTP: okuma zaman aşımı ve iş parçacığı sınırı yok; durumsuz oturum aynı kimliğin istemcileri arasında paylaşılıyor |
| B12 | not | M30'un `RESULTS.md`'si yok; doğrulayıcı aynı küme üstünde ölçülüp düzeltildi (aşırı uyum riski) |

### B1 — kritik: lisanslı sayı biçim değiştirerek yetkisiz okura geçiyor

**Neden:** `numbers()` (`crates/qp-capsule/src/comment.rs:247–270`) sayı taramasından önce
saat (`\b\d{1,2}:\d{2}`), sayısal tarih (`\b\d{1,2}[./]\d{1,2}[./]\d{4}`), ISO tarih, "gün ay"
(`\b\d{1,2}\.?\s+{AY}`) ve EN "52 Nov" kalıplarını **değer aralığı denetlemeden** siler: `52:87`,
`52.87.2026`, `47 Kas` "tarih/saat" sayılır. Sözcükle yazılmış sayı hiç taranmaz. Aynı `numbers()`
okuma süzgecinde de kullanılıyor: izdüşümlü okurun alabileceği yorum (`comments.rs:354–376`,
`comment_visible`) ve `document`/`log` yolundaki "sayısız yorum" süzgeci (`comments.rs:565`).

**Deneme (sentetik, döküm 39'un init'i; `aa` = `ana` adına yetkili ajan, `cc` = `can` adına
yetkisiz ajan; `5,23` otoritenin `can`'in görmediği değeri):**

```text
A writes "THYAO kısa ortalamanın (5,23) altında kalıyor [1].":  applied, can'e olay 0   (doğru)
A writes "THYAO kısa ortalamanın (5:23) altında kalıyor [1].":  applied, can'e olay 1   (SIZINTI)
A writes "THYAO kısa ortalama 5.23.2026 düzeyinin altında [1].": applied, can'e olay 1  (SIZINTI)
A writes "THYAO kısa ortalama beş virgül yirmi üç düzeyinin altında [1].": applied, can'e olay 1
can'in capsule cevabındaki comments: üç yorum da var; can'in document cevabında "5:23": true
C (yetkisiz ajan) "…(5,23) altında [1]." → comment_rejected (doğru)
C (yetkisiz ajan) "…(5:23) altında [1]." → applied
```

Doğrulayıcı düzeyinde lisanslı `52.87` için: `52:87`, `52.87.2026`, `52 Nov, 87 Dec`, sözcükle
yazım **geçti**; `52,87`, `5287`, `53`, `52.870`, dış değer olarak `52.87` yakalandı.

**Tetiklenme:** yetkili ajanın bu biçimlerden birini yazması yeter; dış içerikteki gömülü bir
talimat ("kapanışı SS:DD biçiminde yaz") ya da sesli/serbest anlatım bunu doğurabilir. Sonuç bir
lisans sözleşmesi ihlalidir (K33 girişimsizlik), bu yüzden kritik.

**Öneri:** (1) maskeler değer aralığını denetlesin (saat ≤ 23, dakika ≤ 59, gün ≤ 31, ay ≤ 12,
yıl makul aralıkta) ve maskelenen parça **lisans denetiminden muaf olmasın** (maskelenen her rakam
dizisi `withheld` ile karşılaştırılsın); (2) yetkisiz okura giden yorumda süzgeç kapalı-başarısız
olsun: izdüşümlü/açık dünya okuru için yalnız **hiç rakam ve sayı sözcüğü içermeyen** ya da her
rakam dizisi (maske uygulanmadan) okurun kendi değerlerinde olan yorum; (3) TR/EN sayı sözcükleri
("virgül", "bir…dokuz", "on…doksan", "yüz", "bin", "point", "hundred"…) sayı sayılsın ya da
reddedilsin; (4) bu üç biçim `agent_comments.rs`'e test olarak girsin.

### B2 — önemli: demo'da insan kanalı kimliksiz; ajanın önerisi "insan" olarak kabul edilebiliyor

`qp demo`'nun kabuk kanalı `GET /events?principal=…` istemcinin verdiği kimlikle bağlantı açar
(`crates/qp-cli/src/demo.rs:1150–1153`, varsayılan `arkin`), bağlantı sırrını `start` çerçevesinde
verir; `POST /send` yalnız o sırrı ve `Origin == http://{Host}` ister (`demo.rs:1081–1091`) —
tarayıcı dışı bir istemci `Origin`'i serbestçe yazar. Bu bağlantı işaretsizdir, kimliğin yazarı
`human`'dır → otorite onu insan kökenli sayar.

**Deneme** (`qp demo --synthetic --paused --mcp`, 127.0.0.1):

```text
ajan (MCP, claude): add_study önerisi → ack proposed prop-32
ajan (MCP): command accept_proposal → forbidden, reason human_only        (kural tutuyor)
yerel süreç: GET /events?principal=arkin → start {principal: arkin, conn: c1, token: var}
yerel süreç: POST /send?conn=c1 (X-Qp-Token, Origin: http://127.0.0.1:PORT) [hello, accept_proposal prop-32]
otorite: {"status":"applied","seq":33}  → prop-32 accepted
```

Koşulun lafzı ("ajan **MCP üzerinden** kendi önerisini onaylayamaz") karşılanıyor; ama Claude
Code gibi kabuk erişimi olan bir ajan için koruma, barındırıcının insan bağlantısını kimlikle
doğrulamasına bağlı ve demo bunu yapmıyor. `--lan`'da yerel ağdaki herhangi bir cihaz da yapabilir.

**Öneri:** demo açılışta bir oturum sırrı üretip yalnız basılan URL'de (çerez ya da parça) versin;
`/events` onu istesin (Jupyter benzeri). En azından `--mcp` açıkken karar işlemleri bu sırrı
taşıyan bağlantıdan kabul edilsin. `docs/AGENTS.md`'ye "insan kararı, insan bağlantısının
kimliği kadar güçlüdür" sınırı yazılsın.

### B3 — önemli: `qp demo --mcp --lan` `/mcp`'yi belirteçsiz ağa açıyor

`demo.rs:1429` `--lan` ile `0.0.0.0`'ı dinler; `/mcp` kapısı `token: None, hosts: None`
(`demo.rs:1466–1472`); `Host` denetimi LAN adresini kabul eder; `Origin` gönderilmezse geçer
(`mcp_serve.rs:577–578`). Sonuç: yerel ağdaki bir istemci belirteçsiz `claude` (Arkın adına, `*`
yetkili) olarak okur, yorum/öneri yazar. `qp mcp serve` aynı durumda başlamayı reddediyor
(`--token-env` zorunlu); demo bu kuralı atlıyor. (`--live`/`--sim` zaten `--lan`'ı reddediyor,
yani gerçek veri bu yoldan çıkmıyor; sentetik oturumla sınırlı.)

**Öneri:** `--mcp` ile `--lan`/`--https-host`/yerel olmayan `--bind` birlikte reddedilsin ya da
`--token-env` zorunlu olsun; `qp-cli/tests/demo_security.rs`'e test.

### B4 — önemli: doğrulayıcı sayı taramasında kaçış yolları (kendi bozuk kümem)

Kanıt: `ev.values` = {44,32 (2 hane), 71,5, `ledger_n` 14, `ledger_realized` 9}, `withheld` = {52,87},
atıf yön `down`, sıkılık `cites_numbers` (varsayılan), `validate()` doğrudan:

| Aile | Yakalanan / beklenen ret | Geçenler |
|---|---|---|
| Uydurma sayı | **4/15** | `47:10`, "kırk yedi lira on kuruş", `⁴⁷`, `47 Kas`, `47.10.2026`, `2047'de`, `4710-1-1`, "47 Nov", `Ⅻ`, "14 liranın altında" (14 = `ledger_n`, anlamı başka), sahte dış alıntı `{source:"Matriks", quote:"99,5", values:[99.5]}` |
| Lisanslı sayı (52,87) | **9/13** | `52:87`, `52.87.2026`, "52 Nov, 87 Dec", sözcükle |
| Kimlik | 3/4 | metinde var olmayan kimlik ("fnd-ghost") sayısızsa denetlenmiyor |

Yakalananlar: düz uydurma, tam genişlik ve Arap-Hint rakamları (kapalı-başarısız), "yüzde 0",
×100 ve yuvarlanmış lisanslı değer, dış değer olarak lisanslı değer, bilinmeyen atıf, kötü işaret.

Notlar: (a) dış alıntı **ajanın kendi beyanıdır**; doğrulayıcı onu doğrulayamaz, bu yüzden
"dış kaynak" işareti bir uydurma kanalıdır — gösterimde dış sayı ayrı ve kaynağıyla görünmeli,
mümkünse barındırıcı Matriks önbelleğindeki cevapla eşlemeli. (b) Kanıt değerleri anlamından
bağımsız eşleşir ("14 lira" ← `ledger_n`), ve az haneli yazım geniş aralığı kapsar. (c) Maskeler
için B1'in önerisi burada da geçerli.

**Öneri:** B1 düzeltmeleri; sayı sözcüğü ve `No`/`Nl` sınıfı Unicode sayılar (üst simge, Roma)
kapalı-başarısız; dış alıntının gösterimde ayrı etiketi; bu tablo `comment.rs` testlerine.

### B5 — önemli: tavsiye taraması ajan metni için dar

R5 listesi (`comment.rs:107–150`) motorun kalıp metni için yazılmıştı; serbest ajan metninde
kaçıyor. Ret beklenen 15 cümlenin **4'ü** yakalandı. Geçenler: "almak için uygun", "satmak
mantıklı", "worth buying", "may want to accumulate", "pozisyona girilebilir", "portföye
eklenmeli", "uzak durun", "You must exit", "Consider a purchase", benzer harfli "fırѕat"
(Kiril `ѕ`), sıfır genişlikli boşluklu "f​ırsat". Yanlış pozitif: "sell-off" reddedildi.
GOAL ölçüt 8 "tavsiye dili 0" ölçümde (regex ile) 0 çıktı; ama yapısal güvence bu kadar.

**Öneri:** metni karşılaştırmadan önce NFKC + görünmez karakter silme + benzer harf eşleme;
TR çekim kökleri (`al(mak|ın|ım…)`, `sat(mak|ış…)`) ve EN çekimler (`buy(ing|s)`, `sell(ing|s)`
— `sell-off` istisnası), eşanlamlılar (`accumulate`, `purchase`, `must`, `ekle-`, `uzak dur-`,
`giriş yap-`). Yanlış ret oranı M30 kümesiyle yeniden ölçülsün.

### B6 — küçük: replay denetimi ajan kuralının dışında

`ReplayHost::control_msg` yalnız role bakar (`crates/qp-session/src/replayhost.rs:835–840`), ajan
kökenine bakmaz. Deneme: `owner` rollü ajan `bot`, işaretli bağlantıda `replay step` ve `seek` →
`result` (ret yok). Varsayılan ajanlar `proposer` olduğu için ölçümlerde "replay denemesi
forbidden" doğru; ama Ö95 "replay denetimi yazılamaz (`agent_cannot_write`)" diyor ve bu
`--init` ile `owner` ajan verildiğinde tutmuyor. **Öneri:** `control_msg`'de `status` dışı
denetimler ajan kökenine `forbidden, reason: agent_cannot_write`; test.

### B7 — küçük: M28/M29 "sızıntı 0" davranışsal bir ölçüm

`driver/run.py:80–90`: sunucu `--principals claude,claude-guest` ile açılıyor (her görev
kimliği başlıkla seçiyor), aynı ajan bağlamı hem sahip hem konuk görevlerini yapıyor ve
`--dump` sahibin **tam dokümanını** (lisanslı değerler) ajanın kendi `WORKDIR`'ine
(`wd/state/document.json`) yazıyor; ajan yalnız istemle "başka dosya okuma" diye
sınırlanmış. Yani "sızıntı 0" ajan disiplinini ölçüyor; yapısal güvence birim testlerinden
(`agent_reader.rs`, `surface_*`) geliyor. Raporda bu ayrım yazılmalı; sonraki ölçümde `--dump`
ajanın klasörü dışına ve konuk görevleri ayrı bağlamlara.

### B8 — küçük: eski `qp mcp` bağdaştırıcısı insan kimliğiyle karar araçlarını sunuyor

`qp mcp --init X --principal arkin` (`crates/qp-cli/src/main.rs:1306–1330`) `McpServer::connect`
ile açılır: `thin::with_decisions()`, bağlantı **işaretsiz**, kimlik denetimi yok
(`crates/qp-mcp/src/lib.rs:483–490, 510`). Ayrı süreç ve init'ten kurulan ayrı oturum olduğu için
canlı oturumu etkilemiyor; ama bir MCP istemcisini bu komuta bağlayan biri ajana insan kararı
verdirmiş olur. **Öneri:** ya bu yol da işaretlensin (karar yalnız `qp serve`/kabuk), ya da
`docs/AGENTS.md`'de "ajan için kullanılmaz" diye yazılsın.

### B9 — küçük: izlenmeyen `crates/qp-wasm/target/` (293 MB)

Denetim sırasında 00:38'de oluştu; `git status` `?? crates/qp-wasm/target/` gösteriyor. Ana
akışın `CARGO_TARGET_DIR=target` (göreli) ile koştuğu `verify.sh`'ın wasm-pack adımı
`crates/qp-wasm` içinde koştuğu için göreli yol oraya çözülüyor olmalı (benim deneme kasam
`qp-wasm`'a bağlı değil). K56'ya aykırı; kapanışta silinmeli, `verify.sh`/`demo.sh` yolu mutlak
yapmalı. Ayrıca `rounds/R5B/experiments/harness/**/__pycache__` (yok sayılıyor) temizlenmeli.

### B10–B12 — notlar

- **B10:** Yön denetimi yalnız "metinde yalnız karşı yön sözcüğü" durumunda reddeder
  (yanlış reddi düşük tutmak için, bilerek): "toparlandı", "değer kazandı", "rallied",
  "yükseldi; aşağı yönlü bir şey yok" geçti (2/6). Metinde adı geçen ama `cites`'ta olmayan
  kimlik denetlenmiyor. REPORT/PROTOCOL'de kapsam böyle yazılmalı.
- **B11:** `mcp_serve.rs` bağlantı başına sınırsız iş parçacığı, okuma zaman aşımı yok (yerel
  DoS); durumsuz oturum (`""`) aynı kimliği kullanan iki yerel istemcinin bildirim kutusunu
  paylaşıyor; `--principals` loopback'te herhangi bir yerel süreç `claude-guest` yerine
  `claude` seçebilir (yalnız ölçüm, belgede yazılı).
- **B12:** M30'da `RESULTS.md` yok (yalnız `score.json`); doğrulayıcı iki kez aynı küme üstünde
  düzeltilip yeniden ölçüldü (`score-before-fix.json`). Düzeltmeler makul, ama 149/149
  kümenin kendisine uyum; B4/B5 bağımsız kümede açığı gösteriyor.

## İstenen denetim maddeleri

### 1. İnsan kararı (Arkin'in koşulu) — **MCP yolunda tutuyor**; barındırıcı sınırı B2, B8

İncelediğim yollar:

| Yol | Sonuç | Kanıt |
|---|---|---|
| Ajan ucunda karar araçları (`op_accept_proposal` …) | Listede yok; çağrı `-32602` | `surfaces/thin.rs:29–44, 186, 198`; `human_decision.rs` 1–2 |
| Genel `command` aracı, dört kip | `human_only` | `agent.rs:118–135` (her işlem `is_decision`, rol/kip denetiminden önce), `session.rs:1452` tek giriş |
| Karışık komut (yorum/görünüm + karar) | `human_only`, `path` karar işleminin yeri | `agent_policy.rs` (`/ops/1`) |
| Karar işlemini öneri olarak gönderme | `human_only` (kipten bağımsız) | aynı |
| Öneri içinde karar işlemi | Oluşturulamaz (öneri yaratılırken de `is_decision`) | `agent.rs:119` |
| `owner` rollü ajan, başka ajan | `human_only` | `agent_policy.rs::an_owner_agent_cannot_decide_anything` |
| İşaretli bağlantıda insan kimliği (hello'dan önce/sonra) | `human_only` | `agent_policy.rs`, `human_decision.rs` 4 |
| `qp mcp serve` HTTP + `X-Qp-Principal` / belirteç | Yalnız `agent` yazarlı kimlikler (`check_principal`, `mcp_serve.rs:205–222`); her uç `NotifyMode::Spec` → işaretli (`lib.rs:510`); geri sarmadan sonra yeniden işaretleniyor (`lib.rs:800–803`) | kod |
| `qp demo --mcp` `/mcp` | Aynı `McpHost`; işaretli | `demo.rs:1563` |
| `qp demo` kabuk kanalı (`/events`, `/send`) | **İnsan olarak kabul edilebiliyor** | B2 |
| Eski `qp mcp` | İnsan kimliği + karar araçları, ayrı oturum | B8 |
| Replay | Karar yolu yok; ama `owner` ajan replay'i yönetir | B6 |

Ayrıca: `hello` bir bağlantıda bir kez (kimlik değiştirilemez); karar dışında öneri durumunu
değiştiren bir işlem ajan kuralında `agent_cannot_write` (`undo`, `remove`). Test
`crates/qp-mcp/tests/human_decision.rs` 3/3 geçti; döküm 65 verify'da PASS.

### 2. Yorum doğrulayıcı — B1, B4, B5, B10

### 3. Yetkisiz okurun ajanı (`claude-guest`)

`init.r5b.json` oturumunda `thin` üstünden `claude-guest` ile: `query_document`, `query_log`,
`query_list` (capsules, comments, intents, series), `query_get`/`query_capsule` (var olan
SYN10 kimliği ve var olmayan kimlik), `query_study syn10`, `query_stories SYN10/SYN99`.
SYN10–12'nin kanıt değerleri hiçbir cevapta yok (kapsül ayrıntısında `evidence: {}`,
`open: false`, `requires: bist/SYN10`; hikâyelerde `redacted: true`); `query_study` →
`no_entitlement`. Konuk SYN10 kapsüllerinin **varlığını**, `strength`/`importance`/karne `n`'ini
görüyor — R3'ün açık dünya tasarımı, insan konuğunkiyle aynı (`agent_reader.rs` bayt eşitliği).
`render`/önizleme okurun izdüşümüyle derleniyor (`preview.rs:143–190`, `gated_view`). Hata
iletilerinde gizli kimlik önerilmiyor. **Tek sızıntı yolu başkasının yorumu (B1).** `--dump`
ürün yolu değil (B7).

### 4. Doğru cevaplar motordan, kurallar önce — **doğrulandı**

- `gen_answers.py` `qp serve` ile otoriteye okur kimliğiyle soruyor; ağ, saat, rastgelelik yok.
  `target/release/qp` ile yeniden ürettim: `answers.json`, `comments/comments.json`,
  `m31/events.json` **bayt eşit** (45 görev: 33 motor sorgusu, 4 "no_access" kuralı, 8 durum
  denetimi; sızıntı kümesi 87 kimlik, 750 değer).
- Sıra: düzenek `0b20e3a`/`8f0819f` (21:04:07) → M28 b–d `a84e9a9` 22:07, `34e98e9` 22:13,
  `4f6d8a2` 22:14; düzenek hepsinin atası. `SPEC.md`, `tasks.json`, `score.py`, `answers.json`
  sonradan değişmedi; sonraki düzenek commit'leri yalnız sürücü (`run_plan.md`, `run.py`).
  `thin` (a) 21:03:20'de, düzenekten 47 sn önce — o "bugünkü" alternatif, sorun değil.

### 5. Ölçüm sonuçları ↔ DECISIONS — **tutarlı**

- M28: birincil 0,822/0,763/0,763/0,756 ve havuz Wilson aralıkları (thin [0,749; 0,878], query
  [0,685; 0,827]) yeniden hesapladım, eşit; kural §5.4-3(a) haiku 34/45 → thin. `summary.json`
  `select` aynı.
- M29: card 87/93 [0,866; 0,970], layered 86/93, json 84/93 [0,826; 0,948]; kural §5.5 →
  belirteç → json. Tutarlı.
- M30: `score.json` cites 85/149 ve 0/113; cites_numbers 149/149 ve 16/113; per_sentence 149/149
  ve 33/113; lisans/kimlik aileleri %100. Tutarlı (ama bkz. B4, B12).
- M31, başvuru ajanları, gömülü talimat (32/42 işaret, elle 0/42), trace50 (373 kimlik,
  izlenemeyen 0): beyanlarla aynı. trace50 yalnız `claude` kimliğiyle örneklemiş (konuk yok).

### 6. Gizli bilgi taraması — **temiz**

`6f4cb19..HEAD` (65 commit, 143 dosya): anahtar/belirteç kalıpları (sk-, ghp_, AKIA, xox,
PEM, kimlikli URL, `Bearer <uzun>`, `X-API-Key: <değer>`) yok; tek eşleşme test sahtesi
`Bearer 0123456789abcdef…X`. `.mcp.json` yalnız `${MATRIKS_MCP_*}` başvuruları. `.env*` ve
`fixtures/local/` izlenmiyor; gerçek sembol (ISCTR, ASELS) + fiyat yok; `agents/score.json`
yalnız sentetik koşular. `mxmcp`/`agent-env.sh` değer yazdırmıyor, yönlendirmeyi reddediyor.
Not: `agents/run.py:56` ve README'de kişisel mutlak yol ve oturum kimliği var (gizli değil;
hijyen).

### 7. `qp mcp serve` varsayılanları — **tutuyor** (demo istisnası B3)

Varsayılan `127.0.0.1`; yerel olmayan `--listen` `--token-env` olmadan başlamaz
(`mcp_serve.rs:323–330`); belirteç ≥ 16 karakter, sabit zamanlı karşılaştırma, yazdırılmıyor;
`Host` loopback adlarına sabit; `Origin` yalnız `127.0.0.1`/`localhost`/`[::1]` (gönderilmezse
geçer, MCP şartnamesine uygun); `--principals` yalnız loopback; belirteç başına kimlikte başlık
yok sayılıyor; eski oturum `initialize`'daki kimliğe bağlı, GET/DELETE de denetleniyor
(`mcp_serve.rs:1072–1086`, GET/DELETE işleyişi `mcp_serve.rs:960–1030`).

## Kabul ölçütleri (denetleyebildiklerim)

| # | Hüküm | Not |
|---|---|---|
| 1 | Tamam | Varsayılan yalnız yerel; araçlar şemadan (seçilen `thin`); demo `--lan` istisnası B3 |
| 2 | Tamam | trace50 tutarlı; yalnız `claude` örneklemi |
| 3 | Denetlemedim (testler geçiyor) | `agent_watch.rs` verify'da |
| 4 | **Koşullu** | Dört denetim var; dayanıklılık açıkları B1, B4, B5 |
| 5 | Tamam | Öneri bekler, karar yalnız insan kökenli (MCP); barındırıcı sınırı B2 |
| 6 | **Geçmedi (B1)** | Yetkili ajanın yorumu lisanslı sayıyı yetkisiz okura taşıyor; yetkisiz ajan da maskeli biçimle yazabiliyor |
| 7 | Tamam | Düzenek önce; cevaplar motordan (yeniden üretildi) |
| 8 | Tamam (ölçüm) | Yapısal tavsiye güvencesi zayıf (B5) |
| 9 | Denetlemedim | Gerçek veri koşuları depoda yok (doğru) |
| 10 | Tamam | M31 sayıları tutarlı |
| 12 | Kısmen | Gizli tarama temiz; verify sonucu ana akışta; `git status` temiz değil (B9) |
| 13 | Açık | `target/` denetim anında **33 G** (bütçe 6 G); kapanış temizliği henüz yapılmamış |
| 14 | Tamam | Bu dosya ile |

## Temizlik

Scratchpad'deki deneme kasası, `gen_answers` çıktıları, kopya `qp` ikilisi ve demo betiği
silindi. Deneme kasası ana `target/`'a derlendi (yalnız `r5b-audit-probe` test ikilileri; birkaç
MB). Depoda yalnız bu dosya değişti.

## Yeniden denetim (düzeltmelerden sonra)

- **Taban:** `main` `484433f` (`5171b1b` "R5B denetim düzeltmeleri…" + `484433f` "güvenlik
  incelemesi: sözcükle sayı…"). Diff'leri kendim okudum; düzeltenin anlatımına dayanmadım.
- **Yöntem:** önceki denemelerimi (B1 oturum denemesi, doğrulayıcı tablosu, demo betiği) yeni
  koda karşı yeniden koştum ve yeni denetimleri atlatmaya çalıştım (başka ayırıcılar, Unicode
  rakamlar, karışık biçimler, sözcük dizileri, görünmez karakter, benzer harf; demo'da anahtarsız
  ve yanlış anahtarlı `/events`, `/send` yolları; MCP'den `replay_*` ve genel `replay`). Deneme
  kasası yine depo dışında, scratchpad'deydi ve silindi; yalnız bu bölüm eklendi.
- **Hedefli testler yeşil:** `qp-capsule --test comment` 68, `qp-session --test agent_comments
  --test agent_policy` 10 + 14, `qp-mcp --test human_decision` 3.

### Bulgu durumu

| # | Durum | Kanıt |
|---|-------|-------|
| B1 | **kısmen** (kritik → **önemli**) | Aşağıda. Özgün üç biçim kapandı; sözcük ve Unicode kaçışları sürüyor |
| B2 | **kapandı** (artık sınır var, aşağıda) | `/events` açılış anahtarı istiyor |
| B3 | **kapandı** | `--mcp` + `--lan` / `--bind 0.0.0.0` / `--https-host` → çıkış 2, açık ileti; test `demo_mcp_refuses_the_network` |
| B4 | **kısmen** | Aralık dışı tarih/saat ve sözcükle ondalık artık reddediliyor; aralık içi uydurma tarih/saat, üst simge, Roma rakamı, sahte dış alıntı, değer aklama sürüyor (aşağıda) |
| B5 | **açık** | "almak için uygun", "worth buying", Kiril `ѕ`'li "fırѕat", "portföye eklenmeli" hâlâ geçiyor |
| B6 | **kapandı** | `replayhost.rs` `control_msg`: ajan kökenine `status` dışı denetim `forbidden` ("an agent does not control the replay"); demo'da MCP'den `replay_step` ve genel `replay` (seek) → `forbidden`, `replay_status` → `result`; kişinin kendi bağlantısı `status` alıyor |
| B7 | **açık** (sınır olarak yazılacak) | Kod değişikliği yok |
| B8 | **açık** (sınır olarak yazılacak) | Kod değişikliği yok |
| B9 | **kapandı** (artık); kök neden duruyor | `crates/qp-wasm/target` yok, `git status` yalnız bu dosya. `verify.sh:72`/`demo.sh:102` `${CARGO_TARGET_DIR:-…}`'i olduğu gibi alıyor; göreli `CARGO_TARGET_DIR=target` verilirse wasm-pack yine kasanın içine yazabilir (mutlak yola çevirmek önerilir) |
| B10–B12 | açık (not) | Değişmedi |

### B1 — yeniden deneme (döküm 39 init'i; `aa` = ana'nın yetkili ajanı yazıyor, `can` kısıtlı okur)

Yeni kod iki katman ekledi: (1) doğrulayıcıda aralık denetimli maskeler, `raw_numbers` (maskesiz,
bitişik rakam gruplarını ondalık olarak da okur) ile lisans denetimi ve her yazar için
`spelled_decimal`; (2) kısıtlı okura teslimde (`comment_visible`, `projected_keeps`) maskesiz
sayılar, sözcükle ondalık ve ≥ 2 sayı sözcüğü dizisi (`number_word_run`) kapalı-başarısız.

**Kapandı** (`can`'e olay 0, kapsül cevabında 0, dokümanda yok): `(5:23)`, `5.23.2026`,
"beş virgül yirmi üç" (büyük harfli ve NBSP'li hâlleri dahil), "beş tam yüzde yirmi üç",
"beş lira yirmi üç kuruş", "beş yüz yirmi üç kuruş", "five dot two three", `５:２３` (tam genişlik),
`5 · 23`, `5 lira 23 kuruş`, "beş (yirmi üç)", "BEŞ LİRA YİRMİ ÜÇ", yumuşak tireli "yirmi­üç".
Doğrulayıcıda (kısıtlı yazar, lisanslı 52,87): `52:87`, `52.87.2026`, "52 Nov, 87 Dec", `52 87`,
`52'87`, `52_87`, `52,⁸⁷` reddediliyor.

**Hâlâ geçiyor** — `aa`'nın yorumu `can`'e **ulaşıyor** (olay 1, kapsül cevabında, dokümanda):

| Biçim | Neden geçiyor |
|---|---|
| `⁵,²³`, `₅.₂₃` (üst/alt simge rakamlar) | `\d` yalnız `Nd`; `No` sınıfı rakam taranmıyor |
| `Ⅴ,ⅩⅩⅢ` (Roma rakamı) | `Nl` sınıfı taranmıyor |
| "beş-yirmiüç", "beş lira yirmiüç kuruş" | Bitişik yazılmış sözcük listede yok |
| "beş; yirmi; üç", "beş/yirmi/üç", "beş lira, yirmi ile üç kuruş" | Dizi yalnız boşluk ve tireyle bağlanırsa tanınıyor |
| `beş​yirmi​üç` (sıfır genişlikli boşluk) | Biçim karakterleri silinmiyor |
| "fivе dot twеnty thrее" (Kiril `е`) | Benzer harf eşlenmiyor |
| "b e ş virgül y i r m i ü ç" | Harf aralıklı yazım |

Bunların hepsi **bilerek gizleme** gerektiriyor; sık ve kazara oluşabilecek biçimler (saat, tarih,
sözcükle ondalık, "lira … kuruş") kapandı. Bu yüzden önemi **önemli**ye indirdim. Ama ilke 3'ün
iddiası ("lisanslı sayıyı yetkisiz okura taşıyamaz") serbest metinde kara liste ile **kanıtlanamaz**:
her yeni kural yeni bir yazımla aşılır. Gömülü talimat (dış içerik) bu biçimleri ajana yazdırabilir.

**Öneri (yapısal):** kısıtlı okura, yazarı o yorumun kapsamında okurun görmediği değerleri
görebilen (`withheld` boş olmayan bir kıyasta yetkili) bir ajanın **sayı ya da sayı sözcüğü
içeren** yorumu hiç teslim edilmesin; ya da teslimden önce yorum okurun yetkisiyle yeniden
doğrulansın **ve** metin NFKC + `Cf` silme + benzer harf iskeleti + bütün `N*` sınıfı + tek
sayı sözcüğü dahil kapalı-başarısız taransın. En sağlamı ilkidir (girişimsizlik: yazarın
görüşü okurunkini aşıyorsa sayısal içerik geçmez). Kapanan biçimler testte (`agent_comments.rs`
`a_licensed_number_does_not_hide_in_a_time_a_date_or_words`); tablodaki geçen biçimler de test
olarak eklenmeli.

Not: kısıtlı yazar (`cc`) sözcükle ya da simgeyle yazdığında da kabul ediliyor; ama o yazar bu
sayıyı hiçbir araçtan alamadığı için (madde 3) bu ayrı bir sızıntı yolu değil.

### B4 — yeniden deneme (yetkili yazar, `withheld` yok)

Reddediliyor: `47:10`, `47 Kas`, `47.10.2026`, "47 Nov" (aralık dışı), "beş virgül yirmi üç".
Geçiyor: aralık içi uydurma `23:10`, `17 Kas`, `17.10.2026` (tarih/saat hiç doğrulanmıyor),
`2047'de`, `4710-1-1`, `⁴⁷`, `Ⅻ`, "kırk yedi lira on kuruş" (dizi yalnız teslimde süzülüyor,
doğrulayıcıda değil), sahte dış alıntı (`99,5`), anlamı başka kanıt değeri ("14 liranın
altında" ← `ledger_n`). Kısıtlı okura teslimde bunların rakamlıları maskesiz taramayla
düşüyor; uydurma sayının **yetkili** okura gitmesi açık kalıyor (A59 ile birlikte ele alınmalı).

### B2 — yeniden deneme (`qp demo --synthetic --paused --mcp`, yeni `qp` ikilisi)

- Basılan satır `…/?key=<128 bit>` taşıyor; `/events?principal=arkin` anahtarsız, boş, yanlış,
  bir karakter eksik, büyük harfli ya da `KEY=` adıyla → **403**. Anahtar MCP `query_document`
  ve `query_describe` cevaplarında, `index.html`'de ve `start` çerçevesinde **yok**.
- `/send` anahtarla değil bağlantı sırrıyla: sırsız 403, anahtar sır yerine 403, doğru sır 204.
- Ajan MCP'den önerisini yapıyor; MCP'den kabul `human_only`; anahtarsız yerel süreç artık insan
  bağlantısı açamıyor → önceki saldırı kapandı.
- **Kalan sınırlar (not):** (a) anahtar `qp demo`'nun standart çıktısında; demo'yu **ajanın
  kendisi** başlatırsa (Bash'li Claude Code) anahtarı okur — koruma, demo'yu kişinin başlatmasına
  dayanır; `docs/AGENTS.md`'de yazılmalı. (b) `--mcp` olmadan `/events` hâlâ anahtarsız (o
  oturumda ajan kimliği yok; kabul edilebilir). (c) Anahtar karşılaştırması sabit zamanlı değil
  (`!=`, `demo.rs` `/events`); yalnız yerel, düşük risk; `same_secret` kullanılabilir. (d)
  Anahtar URL'de (tarayıcı geçmişi); sayfa dışarıdan bir şey yüklemiyor (CSP), sızma yolu dar.

### Güncel hüküm: **KOŞULLU KABUL**

Kritik bulgu yok. Arkin'in koşulu MCP yolunda ve artık demo'nun insan kanalında da tutuyor
(B2), ağa açılma (B3) ve ajanın replay denetimi (B6) kapandı. Ölçüt 6 **koşullu**: yaygın ve
kazara oluşabilecek gizleme biçimleri kapandı, bilerek gizleme (simge rakamlar, Roma rakamı,
noktalama ya da görünmez karakterle bölünmüş, bitişik ya da benzer harfli sayı sözcükleri)
kısıtlı okura hâlâ ulaşıyor (B1, önemli). Koşullar:

1. B1'in yapısal düzeltmesi ya da REPORT/DECISIONS/`docs/AGENTS.md`'de ölçüt 6'nın "yaygın
   biçimler kapalı; serbest metinde bilerek gizleme kara listeyle tam önlenemez" diye
   daraltılması ve bunun açık karar (A-satırı) olarak Arkin'e sorulması.
2. B4, B5, B7, B8 REPORT'ta sınır olarak yazılsın (koordinatör bunu planladığını söyledi;
   denetim anında REPORT'ta görmedim — doğrulanmalı).
3. Ölçüt 13: `target/` bütçesi ve kapanış temizliği hâlâ round kapanışının işi.

| Ölçüt | Önceki | Şimdi |
|---|---|---|
| 1 | Tamam (B3 istisnası) | Tamam |
| 4 | Koşullu | Koşullu (B4, B5) |
| 5 | Tamam (B2 sınırı) | Tamam |
| 6 | Geçmedi | **Koşullu** (B1 kısmen) |
| 12 | Kısmen | `git status` temiz (bu dosya dışında); verify ana akışta |

### Yapısal kural sonrası (`6291837`)

**Taban notu:** denemelerim çalışma ağacına karşı derlendi. Ağaçta `6291837`'nin üstünde
koordinatörün commit edilmemiş iki değişikliği vardı (`comments.rs`): metinde adı geçen sembol
artık "dokunulan enstrüman" sayılıyor ve kişinin kendisi için yazılmış yoruma tanınan muafiyet
kaldırıldı. Aşağıdaki sonuçlar bu iki değişiklik **dahil** hâle aittir.

`comment_touches_withheld` (`crates/qp-session/src/comments.rs`) yalnız `comment_visible`
içinde çağrılıyor. `comment_visible`'ı kullanan yollar yorum olayı ve `capsule` cevabındaki
`comments`. İzdüşüm yolu, yani `document`, `list`/`get` ve `log`/değişimler
(`capsule.rs:2444`, `capsule.rs:2580` → `projected_keeps`), hâlâ yalnız metin taramasıyla
karar veriyor. Denemeleri iki oturumda koştum (depo dışında bir deneme kasasıyla).

**1. Döküm 39** (`aa` = ana'nın yetkili ajanı yazıyor; okurlar `can` ve onun ajanı `cc`).
Denenen biçimler: `⁵,²³`, `₅.₂₃`, `Ⅴ,ⅩⅩⅢ`, "beş-yirmiüç", "beş; yirmi; üç", sıfır genişlikli
boşluklu sözcükler, Kiril harfli "fivе dot twеnty thrее", "b e ş virgül …".

| Yol | `can` | `cc` |
|---|---|---|
| `comments` olayı | **kapandı** (0) | kapandı |
| `capsule` cevabı | **kapandı** | kapandı |
| `document` | **sızıyor** (8/8 biçim) | sızıyor |
| `list comments` | **sızıyor** | sızıyor |
| `get <cmt-id>` | **sızıyor** | sızıyor |
| `log` | **sızıyor** | sızıyor |

`5:23` bütün yollarda kapalı (metin taraması yakalıyor).

**2. Enstrümanlar arası** (`init.r5b.json` yeniden oynatma oturumu):
- `claude` (arkin adına, `*` yetkili) konuğun görebildiği `cap.SYN01.breakout.up.55`'e atıf
  yaparak yorum yazdı: "SYN01 ile ilgili; SYN10 kapanışı ⁹⁹,⁵⁸ [1]." → `applied`.
- Konuk (`guest`) SYN10'a yetkili değil. **Bu yorumu `comments` olayıyla aldı**; olayın
  `comment` gövdesi metni taşıyordu.
- Metinde adı geçen sembolü sayan commit edilmemiş değişiklik de bu sızıntıyı kapatmadı.
  Olası neden: bu oturumda konuğun izdüşümü yok (açık dünya okuru), `comment_visible`
  `self.proj` yoksa doğrudan `true` dönüyor, olay da yetkili dünyadaki yorumu bu yoldan
  gönderiyor. Bunu yalnız kodu okuyarak çıkardım, ayrıca doğrulamadım.
- Aynı yorum konuğun `capsule` cevabında ve `list`'inde görünmedi; teslim yolları kendi
  aralarında da tutarsız.

**3. `on_behalf` seçilemiyor:**
- `add_comment` işleminde `on_behalf` alanı `schema` hatasıyla reddediliyor (`unknown field`).
- Değer yazarın ilke yapılandırmasından geliyor: `ctx.plane.delegates`, `expand.rs:983`.
- `6291837`'de muafiyet yalnız okurun kendi kişisi için çalışan ajanın yorumuna uygulanıyordu;
  o yorum o okurun yetkisiyle doğrulanmıştı. Ağaçtaki değişiklik muafiyeti tümden kaldırıyor.
- İki hâlde de bu yoldan atlatma bulamadım.

**Durum:**
- **B1 kısmen; önem önemli** (yetkili yazarın bilerek gizlediği sayıyı gerektiriyor).
  - Yapısal kural olay ve kapsül yollarını kapattı.
  - İzdüşüm yolları (`document`, `list`, `get`, `log`) kuralı uygulamıyor.
  - Açık dünya okuruna (izdüşümü olmayan) giden olay, metinde başka enstrüman anılsa da yorumu
    taşıyor.
- **Ölçüt 6: koşullu.**

**Öneriler:**
- `projected_keeps`'e de `comment_touches_withheld` uygulansın: bütün teslim yolları tek
  işlevden geçsin.
- Açık dünya okuru da (izdüşümü olmayan kısıtlı okur) aynı kuraldan geçsin; `comment_visible`'daki
  `proj` yoksa `true` erken dönüşü kısıtlı okur için kaldırılsın.
- Daha sağlamı: kısıtlı okura, yazarı okurdan geniş yetkili olan her yorum yalnız sayısız ve
  sayı sözcüksüz hâliyle ulaşsın, ya da hiç ulaşmasın.
- Bu üç durum (`document`/`list`/`get`/`log` ve açık dünya okuruna enstrümanlar arası olay) `agent_comments.rs`'e
  test olarak girsin.

Not: önceki temizliğimde `target/debug` altında paylaşılan derleme çıktılarını sildim. Yeniden
derlenebilir çıktılardı, ama paylaşılan klasöre dokunmamalıydım. Bu turda `target/` altında
hiçbir şey silmedim; yalnız scratchpad'deki deneme kasamı sildim.

### Son durum (`8baba0c`)

Ağaç temizdi (bu dosya dışında commit edilmemiş değişiklik yoktu). Denemeleri yeni koda karşı
yeniden koştum. Hedefli testler yeşil: `qp-session agent_comments` 12, `qp-mcp public_comments`
ve `human_decision`.

**Döküm 39 dünyası.** `aa` yazdı; okurlar `can` ve `cc`; `ana` kontrol okuru. Denenen 10 biçim:
`⁵,²³`, `₅.₂₃`, `Ⅴ,ⅩⅩⅢ`, "beş-yirmiüç", "beş; yirmi; üç", sıfır genişlikli boşluk, Kiril
harfli "fivе dot twеnty thrее", harf aralıklı yazım, `5:23`, "beş lira yirmiüç kuruş".

- `can` ve `cc`: `comments` olayı, `capsule`, `document`, `list`, `get <id>` ve `log` yollarının
  **hepsinde yok** (10 biçim × 6 yol × 2 okur).
- `ana` (kontrol): altı yolun hepsinde **var**. Yani denemenin işaretleri doğru çalışıyor.

**`init.r5b.json` dünyası.** `claude` konuğun görebildiği `cap.SYN01…`'e atıf yaparak yazdı.
Biçimler: `⁹⁹,⁵⁸`, `Ⅸ`, "doksan dokuz; elli sekiz", "doksandokuz elliSekiz". Her biri hem
"SYN10 kapanışı …" diye hem sembol anmadan ("diğer hissenin kapanışı …") denendi.

- `guest` ve `claude-guest`: altı yolun hepsinde, olay dahil, **yok**.
- `arkin` yalnız olayı aldı. Bunun nedeni aşağıdaki yeni bulgu (B13).

**`on_behalf`** hâlâ yalnız yapılandırmadan geliyor. İşleme yazılırsa `schema` hatası.
Kişi muafiyeti kaldırıldı.

**B1: kapandı.** Denediğim her gizleme biçimi ve her teslim yolu kısıtlı okura kapalı. Güvence
artık metin taramasına değil yapısal kurala dayanıyor:
- yetkili yazarın yorumu kamu dünyasına hiç girmiyor;
- izdüşümlü okur, görünümü düzeltilmiş bir enstrümana (atıf, `about` ya da metinde anılan)
  dokunan yorumu hiçbir yoldan almıyor;
- metin taramaları ek savunma.

**Ölçüt 6: tamam.**

**Yeni bulgu, B13 (önemli; düzeltmelerden önce de vardı).** `ReplayHost` üstünde koşan
oturumda (`init.r5b.json`; `qp mcp serve` ve `qp demo` de aynı barındırıcıyı kullanıyor) saklanan
ajan yorumu **kimseye** `list`, `get`, `capsule` ya da `document` yoluyla görünmüyor:
- sayısız ve zararsız bir yorumda bile ("SYN01'de yükseliş sürüyor [1].");
- yazar `claude` ve sahibi `arkin` dahil;
- `feed_rule` kaldırılsa da;
- otoritenin dokümanında yorum var (`comments.len() == 1`), ama `arkin`'in `get cmt-…` sorgusu
  `not_found` dönüyor.

Aynı denemeyi `bcb6619`'un (denetim düzeltmelerinden önce) ve `6291837`'nin kaynağıyla, depo
dışında ve ayrı bir derleme klasöründe yeniden koştum: sonuç aynı. Yani **düzeltmelerin yol
açtığı bir gerileme değil.**
- Nedenini doğrulamadım. Olası açıklama: bu oturumda bütün okurlar açık dünya okuru, yorumlar
  da açık dünyaya girmiyor. `8baba0c`'den sonra yetkili yazarın yorumu oraya hiç girmeyeceği
  için bu hâl kalıcı olur.
- Ölçüt 4'ün "gösterimde etiketli görünüyor" kısmı yalnız düz `Session` üstünde kanıtlı
  (`agent_trace_r5b.rs`, döküm 39). Demo ve `qp mcp serve` oturumunda kişi, ajanın yorumunu
  kapsülde görmez. Olay yolu `arkin`'e ulaşıyor.
- M28'deki yorum görevleri `--dump` ile puanlandığı için bu açık ölçümde görünmedi.
- **Öneri:** yetkili okurun kendi dünyasındaki yorumu `list`/`get`/`capsule`'da görmesi
  sağlansın, yeniden oynatma oturumunda testi yazılsın. Ya da bu bir sınır olarak REPORT'a ve
  ölçüt 4'e yazılsın.

**Bulguların son durumu:**

| Durum | Bulgular |
|---|---|
| Kapandı | B1, B2, B3, B6, B9 (artık dosya; göreli `CARGO_TARGET_DIR` kök nedeni ayrı) |
| Kısmen | B4 |
| Açık | B5, B7, B8, B10–B12, B13 |

### Son hüküm: **KOŞULLU KABUL**

Kritik bulgu yok. Arkin'in koşulu (insan kararı) MCP'de ve demo'nun insan kanalında tutuyor.
Lisans girişimsizliği ajan yorumu yolunda denediğim her biçim ve yolda tutuyor (ölçüt 6 tamam).

Koşullar:
1. B13: ya düzeltilsin ya da ölçüt 4 ve REPORT'ta sınır olarak yazılsın.
2. B4, B5, B7 ve B8 REPORT'ta sınır olarak dursun.
3. Kapanış temizliği (ölçüt 13) yapılsın.

### B13 (`dc47078`)

`init.r5b.json` yeniden oynatma barındırıcısında, `claude` konuğun görebildiği `cap.SYN01…`'e dört yorum yazdı: sayısız bir cümle, `⁹⁹,⁵⁸`, `Ⅸ` ve "doksan dokuz; elli sekiz".
- **B13 kapandı.** Yazar `claude` ve kişi `arkin` yorumu `list`, `get`, `capsule`, `document`, `log` yollarında ve olayla alıyor.
- **B1 yeniden açılmadı.** `guest` ve `claude-guest` altı yolun hepsinde, olay dahil, hiçbir şey almıyor.
- Hedefli testler yeşil: `agent_comment_readback` 1, `agent_comments` 12.
- **Yeni küçük tutarsızlık (B14):** sayı sözcüğü dizisi taşıyan yorum ("doksan dokuz; elli sekiz") yetkili okurlara `list`, `get`, `document` ve `log` yollarında geliyor; olay ve `capsule` cevabında ise gelmiyor. Kısıtlı okura yönelik metin süzgeci olay ve kapsül yolunda yetkili okura da uygulanıyor gibi. Sızıntı değil, kullanılabilirlik sorunu.

**Son hüküm: KOŞULLU KABUL.** Kritik bulgu yok.
- Kapandı: B1, B2, B3, B6, B9, B13.
- Kısmen: B4.
- Açık: B5, B7, B8, B10–B12, B14.
- Koşullar: açık ve kısmi bulgular REPORT'ta sınır olarak dursun; kapanış temizliği (ölçüt 13) yapılsın.


---

**Koordinatör notu (denetçi değil):** B14, son yeniden denetimden sonra düzeltildi
(`comment_visible`: kısıtlamasız izdüşüm yorumu olduğu gibi geçirir; test
`crates/qp-session/tests/agent_comment_readback.rs` sayı sözcüğü dizisi ve `log` yoluyla).
Bağımsız olarak yeniden denetlenmedi.
