# RD1 — Bağımsız Denetim (T11)

- **Denetçi:** bağımsız denetçi alt ajan (round'un hiçbir işini yapmadı).
- **Tarih:** 2026-11-15
- **Denetlenen:** `main` @ `2265a04` ("Merge RD1 …"), temiz kopya:
  `git clone /Users/arkns/Works/Mchart/qapsule <scratchpad>/audit/repo && git checkout 2265a04`.
  Bütün komutlar bu kopyada çalıştırıldı. Gerçek depoya yazılan tek dosya bu dosyadır.
- **Makine:** Apple silicon (aarch64), macOS; `qp` = kopyada `cargo build -p qp-cli --release`.

## Sonuç: **KOŞULLU KABUL**

Round'un çekirdeği sağlam ve iddia edildiği gibi çalışıyor: `verify.sh` temiz kopyada yeşil,
17 döküm üç taşımada geçiyor, yeniden oynatmanın bayt eşitliği **kendi yazdığım bağımsız
bir Python oynatıcısıyla** 7 günlükte doğrulandı, yetki/sınır/yol hapsi saldırgan mesajlara
karşı doğru davranıyor, şartname yalnızca kendisi ve şemayla okunarak üç yeni görev
yapılabildi. Ajan yazabilirlik ham kayıtları yeniden puanlamayla bire bir tutuyor ve v2r'nin
35 denemesi bugünkü ikiliyle bayt bayt yeniden üretiliyor.

Kabulü koşullu kılan noktalar (ayrıntı "Bulgular"da):

1. **T6a, T6b, T6c deney kodları birleşik `main`'de derlenmiyor** (protokol tipleri ölçümden
   sonra değişti); raporlar bunu ve hangi commit'te yeniden üretileceğini söylemiyor. (Orta)
2. **`base` değeri günlüğün ucundan büyük olduğunda sessizce kabul ediliyor** (ör. `seq` 0'da
   `base: 999`), bu da çakışma denetimini kapatıyor; devirde (epoch değişimi) tam olarak
   kaçınılmak istenen durum. (Orta)
3. **Ajan yazabilirlik ölçümünde alt ajanlara verilen yönerge kayıtlarda yok**; "yalnızca
   şartname ve şema gördü" iddiası kayıtlardan kanıtlanamıyor, yalnızca yönergeye dayanıyor
   ve ajanların çalıştırdığı `run.sh` gizli düzeneğin (`harness/ergo.py`) mutlak yolunu
   gösteriyor. (Orta, yöntem)
4. `REPORT.md`'deki birkaç ifade abartılı (MCP "17/17", denetimden önce "13/13 ✅",
   `STATUS.md`'nin henüz var olmayan `AUDIT.md`'yi kanıt göstermesi). (Düşük)

Koşullar: (1) T6a–c için ya kodu güncel tiplere uyarlamak ya da her deney raporuna "şu
commit'te yeniden üretilir" satırı eklemek; (2) `base > seq`'i `invalid` yapmak ya da şartnamede
davranışı tanımlamak (v1'e not olarak da olabilir — ana akış kararı); (3) T7 yönergesinin
tam metnini `ergonomics/raw/*/AGENTS.md`'ye eklemek; (4) aşağıdaki "REPORT'ta abartılı ya da
yanlış" listesini düzeltmek.

## VERIFY tablosu (14 adım)

**Toplam: 12 PASS, 2 koşullu PASS (8, 9), 0 FAIL.**

| # | Kontrol | Sonuç | Kanıt |
|---|---------|-------|-------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | **PASS** | Temiz kopyada çıkış 0, `verify: OK`, 13/13 adım PASS (fmt, clippy, test, schema "22 schemas, deterministic", fixtures, golden "byte-identical", wasm "gzip -9 475242 B", hygiene, browser "headless Chromium matches golden", compute, digest "arm64, x86_64, wasm32: 7b1e99d40889f83c", protocol "47 examples as expected", conform "17 transcripts x inproc, serve, mcp"). Toplam 141 s. Önceki adımlar değişmeden geçiyor; 12 ve 13 eklenmiş. |
| 2 | `scripts/preflight.sh`; CI | **PASS** | `preflight: TAMAM` (hesap `qapsuleai`, yazma izni, commit e-postası). `scripts/qgh run list --branch main --limit 1` → run `43640428929` "Merge RD1 …" `completed success` (3m33s). Denetim başladığında aynı run `in_progress` idi; tamamlanmasını bekledim. |
| 3 | `fixtures/protocol/` örnekleri üretilen şemaya göre | **PASS** | verify adım 12'ye ek olarak **bağımsız bir doğrulayıcıyla** (Python `jsonschema`, scratchpad venv) `qp schema export` çıktısına karşı: `valid` 27/27 geçti, `invalid` 20/20 reddedildi; 17 döküm `Transcript` şemasına uygun. |
| 4 | Döküm koşucusu, süreç içi | **PASS** | `qp conform fixtures/protocol/conformance --transport inproc` → `17 transcript(s), 17 passed, 0 failed`. Koşucunun ayırt ediciliği: bir dökümde `status` değerini ve beklenen mesaj sayısını bozdum → üç taşımada da FAIL (doğru yol ve neden ile). |
| 5 | Aynı dökümler `qp serve` üzerinden | **PASS** | `--transport serve` 17/17; `--transport mcp` 17/17 (MCP için not: Bulgu D2). |
| 6 | Günlük baştan oynatılır → bayt eşit | **PASS** | Kendi oynatıcım (`replay.py`, yalnızca `PROTOCOL.md` §2/§5.3 ve `ChangeOp` şema açıklamalarından yazıldı; uygulamaya bakılmadı). `qp serve` ile mesajları sürdüm, `log` ve `document` sorgularıyla çıktıyı aldım, `changes`'i başlangıç dokümanına uyguladım, otoritenin **ham** `document` baytlarıyla karşılaştırdım. Sonuç: dökümler 05, 06, 07, 10, 13, 14 ve kendi 15 girdilik dizim (16 farklı `ChangeOp` türü: `put_series/view/fact/finding/capsule/intent/narrative/proposal`, `insert_layer`, `delete_layer`, `set_x_domain`, `set_capsule_state`, `push_ledger`, `pop_ledger`, `delete`, `append_bars`, `set_last_bar`; sil+geri al, karar geri alma, öneri kabulü dahil) — **7/7 bayt eşit**. Bir koşul: başlangıç dokümanı otoritenin kanonik hali olmalı (Bulgu D1). `truncate_bars` bu denemede sınanmadı. |
| 7 | Yalnız şartname + şemayla üç yeni görev | **PASS** | Aşağıda "VERIFY 7". Üç görev de bir keşif çalıştırmasından sonra ilk gerçek denemede başarılı; 9 belirsizlik kaydedildi. |
| 8 | T6 ölçümlerinden en az ikisi | **koşullu PASS** | T6d (`main`'de) ve T6e (`main`'de) yeniden koşuldu: rapora ±%20 içinde. T6b `main`'de **derlenmiyor**; kendi commit'inde (`50eb96d`) koşuldu, rapora ±%12 içinde. T6a ve T6c de `main`'de derlenmiyor (Bulgu O1). Ayrıntı: "VERIFY 8". |
| 9 | Ajan yazabilirlik ham kayıtları | **koşullu PASS** | Kayıtlar var ve tutarlı: `score.py` ile v1 15/20 → 17/20 (düzelen 2/5), v2 14/20 → 18/20 (4/6), v2r (`QP_ERGO_VARIANT=renamed`) 6/20 → 19/20 (13/14) — `RESULTS.md` ile aynı; üç `score.json` bayt bayt yeniden üretildi. v2r'nin 35 denemesi bugünkü `qp` ile yeniden çalıştırıldı: **35/35 tam çıktı bayt bayt aynı**. Ama "yalnızca şartname ve şema gördü" kayıtlardan anlaşılamıyor (Bulgu O3). |
| 10 | Üç yüzey, aynı bulgu, üç derinlik | **PASS** | Döküm 11 geçiyor; ayrıca kendim: `present fnd-overbought` → watch `sentence` {text,state}, phone `capsule` {+glyphs, confidence, spark}, desktop `chart` {+view, facts}, voice `sentence`. |
| 11 | MCP üzerinden üç döküm | **PASS** | `--transport mcp` 17/17; 11 döküm hiç "direct" adım içermeden yalnız MCP araç çağrılarıyla geçiyor (02, 04, 05, 07, 08, 10, 11, 13, 14, 15, 16). Ayrıca `qp mcp` stdio duman testi: `initialize` → welcome `_meta`, `tools/list` → 37 araç, `tools/call query_at` → doğru `result`. |
| 12 | `RESEARCH.md` kaynak bağlantıları | **PASS** | 12 protokolün her satırında URL var; `research/*.md` içinde 95 bağlantı ve "DOĞRULANMADI" işaretleri. İki kaynağı yokladım: MCP `specification/2026-07-28` (durumsuz istekler, istek başına yetenek pazarlığı — raporla uyumlu) ve A2UI v1.0 (`createSurface`, `updateComponents`, `updateDataModel` — uyumlu). Küçük not: A2UI sayfası "Current", "son güncelleme 2026-06-08" diyor; `RESEARCH.md`'deki "kararlı 2026-11-11" tarihini sayfada görmedim. |
| 13 | `qp-compute`, `qp-render`, `fixtures/golden` | **PASS** | `git diff --stat eebffd2 2265a04 -- crates/qp-compute crates/qp-render fixtures/golden` → boş. Hesap özeti `7b1e99d40889f83c` aynı; altın görsel bayt eşit. |
| 14 | Gizli bilgi taraması; eski klasörler | **PASS** | İzlenen dosyalarda `ghp_`, `github_pat_`, `sk-ant-`, `sk-…`, `AKIA…`, `BEGIN … KEY`, `xox[bp]-`, `AIza…` → yalnızca `rounds/R1/AUDIT.md` (desen adlarını anlatan metin, değer yok). `.env`/`secret`/`.pem`/`.key` adında izlenen dosya yok. `scripts/qapsule-gh.env` izleniyor (`eccdc63`, `.gitignore` istisnası); talimat gereği **içeriğini okumadım**, verify `hygiene` adımı (izinli dört anahtar kuralı) PASS. `eebffd2..2265a04` farkında depo dışı yol yok. `../Services` ve `../Docs` klasör mtime'ları 2026-11-14 00:27/00:28 (round öncesi); `find -newer rounds/RD1/GOAL.md` iki klasörde boş. |

## Kabul ölçütleri (GOAL.md, 13)

**Toplam: 10 ✅, 3 ⚠️, 0 ❌.**

| # | Ölçüt | Durum | Gerekçe |
|---|-------|:-----:|---------|
| 1 | ≥ 8 protokol, güncel belgelerden | ✅ | 12 sistem; her biri için neyi çözüyor / karşılık / alındı / alınmadı ve URL. İki kaynak yoklandı, uyumlu. |
| 2 | 2026–2030 varsayımları | ✅ | V1–V9, her biri için kanıt ve "gerçekleşmezse protokolde ne değişir". |
| 3 | `PROTOCOL.md` v0 | ✅ | 8 tür (§4.1–4.8, her biri JSON örnekli), durum modeli, otorite, köken, yetki, yetenek, sürümleme, hata modeli, güvenlik. Belirsizlikler VERIFY 7'de. |
| 4 | Tek kaynak, üretilen JSON Schema, geçerli/geçersiz örnekler | ✅ | Tipler `qp-schema::protocol`; elle yazılmış şema yok (izlenen `*.schema.json` yalnızca `ergonomics/raw/*/packet/` anlık kopyaları). Bağımsız doğrulayıcı 27/20 doğru. Not: "Öneri" için ayrı bir geçersiz örnek yok (`client_msg.bad_mode` dolaylı karşılıyor). |
| 5 | ≥ 12 dilden bağımsız döküm | ✅ | 17 döküm, `Transcript` şemasına uygun. |
| 6 | Dökümler geçer; yeniden oynatma bayt eşit; geri alma; öneri karne/köken | ✅ | 17/17; bayt eşitlik bağımsız oynatıcıyla 7/7; geri alma (kendi, başkasının, karar girdisi, sil+geri al yerinde) ve öneri ret/kabul kendi dizimde ve döküm 05/06/07'de doğrulandı. |
| 7 | İki taşıma | ✅ | inproc 17/17, serve 17/17. |
| 8 | Çelişkili kararlar (a–e) alternatifleriyle | ⚠️ | Beş deney ve raporu var, alternatifler ciddi kurulmuş (Automerge + Loro; CBOR/MsgPack/postcard/sütunlu; JSON Patch korumalı varyantları). T6b, T6d, T6e sayıları yeniden üretildi. Ama T6a/T6b/T6c kodu birleşik `main`'de derlenmiyor ve raporlar sabit commit belirtmiyor (O1); T6e'nin `presenter_fn` ölçümü `pub(crate)` fonksiyonun **kopyası** üzerinde (her koşuda eşitliği denetleniyor; kabul edilebilir, not). |
| 9 | Ajan yazabilirlik | ⚠️ | ≥ 20 görev, taze alt ajanlar, tasarım değişikliği, yeniden ölçüm, önce/sonra tablosu var ve sayılar yeniden üretiliyor. Zayıflıklar: yönerge kaydı yok, erişim kısıtı yalnızca talimatla; tek koşu, tek model (sonnet), n = 20; v1 → v2 ilk deneme iyileşmedi (15 → 14, rapor bunu dürüstçe söylüyor). |
| 10 | Yetenek uzlaşması; anlatım çizelgesi | ✅ | Dört yüzey üç derinlik; §11 ve döküm 13. |
| 11 | MCP uyarlayıcısı; A2UI ve MCP Apps analizi | ✅ | 37 araç şemadan; ≥ 3 döküm yalnız MCP araçlarıyla; `EMBEDDING.md` §2, §3. "17/17" ifadesi abartılı (D2). |
| 12 | `R2_INPUT.md` | ✅ | Gereksinimler, önerilen kapsam, ana akış kararları bölümleri var. |
| 13 | REPORT, AUDIT, STATUS, ROADMAP, CONCEPTS, ARCHITECTURE, DECISIONS | ⚠️ | Hepsi güncellenmiş; `DECISIONS.md` "Önerilen (RD1)" var. Ama `2265a04`'te `AUDIT.md` yoktu; `STATUS.md`/`ROADMAP.md` round'u denetimden önce "Tamamlandı, 13/13" ilan ediyor ve `STATUS.md` `AUDIT.md`'yi kanıt gösteriyor; `REPORT.md` "Denetim" bölümü boş. Bu dosyayla ve REPORT'un güncellenmesiyle kapanır. |

## CLAUDE.md kuralları

| Kural | Sonuç | Kanıt |
|-------|-------|-------|
| `NORTH_STAR.md` değişmez | ✅ | `git diff eebffd2 2265a04 -- NORTH_STAR.md` boş. |
| Hesap/oturum kodunda LLM, ağ, rastgelelik, saat yok | ✅ | `qp-session`, `qp-mcp`, `qp-schema/src`, `qp-compute/src` içinde `SystemTime`, `Instant`, `chrono::Utc/Local`, `rand`, `getrandom`, `reqwest`, `hyper`, `std::net`, `uuid`, `HashMap/HashSet`, `env::var` → eşleşme yok. Not: `qp-session`'ın normal bağımlılığı `tempfile` ve `std::process` yalnızca `conform.rs`'deki döküm koşucusunda (otorite durumunu etkilemiyor; mimari hijyen notu, D6). |
| Şemanın tek kaynağı `qp-schema` | ✅ | JSON Schema `qp schema export` ile üretiliyor; iki dışa aktarım aynı. |
| Dokümanı değiştiren her şey komut yolundan | ✅ | `Session`'ın dışa açık yüzeyi okuma erişimcileri + `handle*`; süre aşımı ve geçersizleşme `qp` yazarlı günlük girdileri; `input` dokümana dokunmuyor (kendi denemelerimde doğrulandı). |
| Değerler `f64` | ✅ | `qp-session` ve `protocol.rs`'de `f32` yok; şemada `format: double`. |
| Kapsam: `qp-compute`/`qp-render` değişmez, altın görsel güncellenmez | ✅ | VERIFY 13. |
| `CLAUDE.md` değişikliği | bilgi | `eccdc63` (round kurulum commit'i) üç kural ekledi ve `scripts/qapsule-gh.env`'i izlemeye aldı — round'un işi değil, Arkin'in kurulum değişikliği olarak görünüyor. |

## VERIFY 7 — yalnızca şartname ve şemayla üç yeni görev

Yöntem: önce yalnız `docs/PROTOCOL.md` ve `qp schema export` çıktısını okudum (uygulamaya,
testlere, `ergonomics/` ve `tasks/` içeriğine bakmadan; `tasks/TASKS.md`'ye yalnızca G01–G20
ile çakışmamak için görev başlıklarını görmek üzere baktım). Mesajları
`qp serve --init fixtures/protocol/init/thyao.json` ile çalıştırdım. Önce bir keşif dosyası
(`list series`, `study px last 2`, `study macd histogram 178–182`, `get main`) çalıştı.

| Görev | Kim | Doğal dil | Sonuç |
|-------|-----|-----------|-------|
| N1 | scout + feed | "Fiyat son kapanışın (124,69) %5 altına inerse bana haber ver; besleme yeni bar getirsin, sonra son barı güncellesin." | ✅ `condition` aboneliği (`threshold px lt 118.4555`) + `changes`; feed `append_bars` (c=120) → yalnız `change`; `update_last_bar` (c=117) → `change` ve ardından `condition_met {bar:200, seq:2}` (kenar tetiklemeli, doğru sıra). Kendi hatam (c=116 < l=116,5) `invalid "bar 0: inconsistent OHLC"` ile reddedildi. |
| N2 | scout → arkin | "EMA(50)'yi fiyat paneline ekle (öneri); arkin reddetsin, not yazsın; reddedilenleri ve arkin'in günlüğünü göster." | ✅ `ack proposed prop-1 created [ema50]`, `proposal` olayları (pending, rejected), `get ema50` → `not_found` (doküman değişmedi), `list proposals state rejected` → `decided_by arkin`, not; `log author arkin` doğru; scout'un `accept_proposal`'ı → `forbidden`. |
| N3 | arkin | "MACD histogramının 178. barda sıfırın altında olduğunu olgu olarak kaydet, ona dayanan bir bulgu yaz ve sunucuyla göster; olguyu silmeyi dene; gösterimi geri al." | ✅ `created [fact-hist-neg, fnd-weak, cap-1-2]`, `show_finding` → `view-2-0`, `remove fact` → `in_use expected [fnd-weak]`, telefon `present` → `capsule`, `undo` → `undoes: 2`, görünüm silindi. |

Karşılaşılan belirsizlikler:

1. **Atanan kimliklerin `n`'i** girdideki bütün oluşturulan öğeleri sayıyor (olgu 0, bulgu 1,
   kapsül 2 → `cap-1-2`). §5.1 bunu söylüyor ama örnek (`fnd-6-0`, `cap-6-1`) tek tür sayaç
   gibi okunabiliyor; ben `cap-1-1` bekledim. `ack.created` kullanılınca sorun yok.
2. **`condition` konusunun alanı** şartnamede yalnızca "`condition`" diye geçiyor; bir
   `Condition` nesnesi olduğunu şemadan öğrendim.
3. **`at` sorgusunda `bar` ve `t` ikisi de yoksa** ne olduğu yazmıyor (son bar dönüyor).
4. **`present` sporu (`spark`)** "ilk olgunun serisinden" diyor ama hangi çıktıdan dediği yok:
   olgu `macd.histogram` iken spark `macd.macd` çıktısını verdi.
5. **`proposal` olayındaki `author`** ret olayında da öneren (`scout`); karar vereni taşımıyor.
   Şartname olay alanlarını tanımlamıyor.
6. **`push_ledger`'ın nereye yazdığı** (`ledger.entries`), §5.3'te yok; `Ledger` şemasından.
7. **Yeniden oynatmanın "başlangıç dokümanı"** dosyanın kendisi değil, otoritenin yüklediği
   kanonik hali (D1).
8. **Feed rolünün `welcome`'u** `read:false` iken `chart` profili ve `max_points:10000` bildiriyor;
   anlamsız ama zararsız.
9. `update_last_bar` hatasında ileti "bar 0" diyor (tek bar için sıra numarası), yol
   `/ops/0/bar` — ilk bakışta 0. barla karıştırılabiliyor.

## VERIFY 8 — T6 ölçümlerinin yeniden üretimi

| Deney | Nasıl | Rapor | Denetim | Fark |
|-------|-------|-------|---------|------|
| T6d (`main`) | `cargo build --release`, `t6d-handover 10000 60 15` | anlık görüntü 749 600 B / gzip 210 638 B; hazır 2,568 ms; ref oturum tik başı 110,8 µs; artımlı 0,18 µs; hızlanma 604× ("460–700×") | 749 620 B / 210 660 B; 2,639 ms; 94,1 µs; 0,20 µs; 483× | boyut ≈0 (20 B fark: sonradan eklenen `now` alanı); süreler ±%15; hızlanma −%20 sınırda, raporun aralığında. Determinizm özetleri eşit. |
| T6e (`main`) | `t6e-presenter bench` | `presenter_fn` 125 ns; +serialize 459–500; `show_finding` 6 583–6 917 ns; present chart 5 958–6 083; capsule 2 750–2 875; 1 000 koşuda 1/1/1; özet `b6566f6d79eeff63` | 125; 459; 6 875; 5 917; 2 875; 1/1/1; `b6566f6d79eeff63` | ±%2 |
| T6b (`50eb96d`) | `main`'de derleme hatası (`ServerMsg` alanı `now` eksik, 5 hata); deneyin kendi commit'inde worktree, `cargo run --release` | tik JSON 234 B, postcard 78 B; RSI 10k JSON 321 991 B; MACD 10k JSON 733 728 B, enc 870 µs dec 1 030 µs; qcol dec 7,76 µs | 234; 78; 321 991; 733 728, 878 / 1 062 µs; 8,66 µs | boyut aynı; süreler ±%12 |

Not: T6e `bench` raw klasöre yazıyor; `raw/t6e/present-query*-out.json` dosyaları v2'den önce
üretilmiş (`now` alanı yok), yani depodaki ham çıktılar güncel koddan bayatlamış (D5).

## Saldırgan mesajlar (`qp serve`)

| Deneme | Sonuç |
|--------|-------|
| 2 MB'lık mesaj | `limit "line exceeds 1052672 bytes"` (ayrıştırmadan önce), süreç ayakta |
| 200 ve 100 000 derinlikli JSON | `parse "recursion limit exceeded"`, çökme yok |
| Feed bulgu yazmaya çalışıyor (`auto` ve `propose`) | `forbidden` (iki kipte); feed `document` okuyamıyor (`forbidden`) |
| Proposer `undo` (hedefsiz, `seq`'li, `mode: propose`) | `forbidden` ("cannot be proposed") |
| Editor, arkin'in girdisini geri almaya çalışıyor | `forbidden "only an owner may undo someone else's entry"`; kendi girdisini geri alabiliyor |
| `put_series` kaynağı `file` (`/etc/passwd`, `../../..`), `matriks` | `unsupported`, `expected [inline, study]` |
| Init'te doküman yolu `../../../etc/hosts`, `/etc/hosts`, `a/../b`, mutlak yol, köke dışarıyı gösteren sembolik bağ | hepsi reddedildi ("must be relative and stay inside the root" / "leaves the root"); aynısı döküm `init`'i ile üç taşımada |
| Viewer yazma; arkin'in `clock`'u; `hello`'da `role: owner`; işlemde `provenance` | `forbidden`; `forbidden`; `schema unknown field role`; `schema unknown field provenance` |
| Sınırlar: 65 işlem, 5 000 baytlık dizge, 10 001 bar, 33 abonelik, geçersiz kimlik `../../x`, yinelenen abonelik kimliği | `limit` / `limit` / `limit` / `limit` / `invalid` (desen) / `duplicate` |
| Dev bar indeksi (u64 max), ters aralık, `zoom factor` 0 ve −1e308, `pan bars` i64 min, `1e400` | `invalid` / `invalid` / `invalid` / kırpıldı / `parse number out of range` |
| `period` 0, −5, 1,5, 1e18 | `invalid`; ama 1e18 için ileti "must be an integer >= 1, got 1000000000000000000" — üst sınır söylenmiyor, ileti yanlış (D3); 100 000 kabul ediliyor |
| **`base: 999` (günlük `seq` 0'dayken)** | **`applied`** — çakışma denetimi devre dışı (O2) |

## Bulgular (önem sırasıyla)

Yüksek önemde bulgu yok.

### Orta

**O1. T6a, T6b, T6c deney kodları birleşik `main`'de derlenmiyor; yeniden üretilebilirlik
raporlarda belirtilmemiş.**
Yeniden üretim: temiz kopyada `cd experiments/t6b-encoding && cargo build --release` → 5 hata
(`missing field now in ServerMsg`); `t6a-concurrency` 28, `t6c-changes` 60 hata (`ChangeOp`
`index` alanı, `PopLedger` struct varyantı, `LogEntry.reason`). Nedeni: deneyler ölçüldükten
sonra protokol tipleri değişti (v2 `now`, `index`, birim varyantların boş yapıya çevrilmesi).
T6b kendi commit'inde (`50eb96d`) yeniden üretildi ve tutuyor; T6a/T6c denenmedi. Öneri:
her deney raporuna "yeniden üretim: `git checkout <commit>`" satırı, ya da deneyleri güncel
tiplere uyarlamak; `verify.sh`'ye en azından `cargo check` için isteğe bağlı bir adım.

**O2. `base`, günlüğün ucundan büyükse sessizce kabul ediliyor.**
Yeniden üretim: `qp serve --init fixtures/protocol/init/thyao.json`'a
`hello arkin` ve `{"type":"command","id":"b","base":999,"ops":[{"op":"add_intent","id":"x","text":"x"}]}`
→ `ack applied seq 1`. `base > seq` anlamsızdır (istemci var olmayan bir geleceği görmüş
olamaz); en olası neden başka bir epoch'tan (devir öncesi otorite) kalan `seq`'tir. §8.4 "bir
`base` her zaman bağlantının kendi epoch'una aittir" diyor ama otorite bunu denetlemiyor; bu
durumda çakışma denetimi tam olarak gerekli olduğu yerde kapanıyor. Öneri: `base > seq` →
`invalid` (`expected: ["<= seq"]`) ve döküm.

**O3. Ajan yazabilirlik ölçümünde "alt ajanlar yalnızca şartname ve şemayı gördü" kayıtlardan
kanıtlanamıyor.**
`raw/v1/AGENTS.md` "Hepsine aynı yönerge verildi (aşağıda)" diyor ama dosya tablodan sonra
bitiyor; yönerge metni hiçbir turda kayıtlı değil. Kısıt yalnızca talimatla; ajanların
kullandığı `packet/run.sh` gizli düzeneğin mutlak yolunu
(`/Users/arkns/Works/Mchart/qapsule/rounds/RD1/ergonomics/harness/ergo.py`) içeriyor ve o
dosya gizli kurulumu ve denetleyiciyi barındırıyor; ajan araç çağrısı kayıtları (hangi
dosyaları okudukları) saklanmamış. Notlarda başka dosya okunduğuna dair iz bulmadım ve sayılar
yeniden üretiliyor (v2r 35/35 bayt eşit), yani sahtecilik izi yok; sorun kanıtın biçimi.
Diğer yöntem zayıflıkları: tek koşu, tek model, n = 20 (rapor ±1–2 görevin gürültü olduğunu
kabul ediyor); "ilk deneme" tek atışlık düzenekte keşfi cezalandırıyor (rapor söylüyor);
v1 ve v2'de üç görevin ilk denemesi bayt bayt aynı (basit görevler için olağan).
Öneri: yönergenin tam metnini kaydetmek; paket dışı yolu gizleyen bir sarmalayıcı; alt ajan
araç çağrısı dökümünü saklamak.

### Düşük

**D1. Bayt eşitliği yalnızca otoritenin kanonik başlangıç dokümanına karşı tutuyor.**
`fixtures/protocol/docs/thyao.json`'daki `macd.params` sırası `fast, slow, signal`; otorite
yüklerken (`BTreeMap`) `fast, signal, slow` yapıyor. Ham dosyaya oynatınca baytlar farklı
(yapısal olarak eşit). §2 "başlangıç dokümanı"nın kanonik serileştirme olduğunu söylemeli
(ya da `document` sorgusuyla seq 0'da alınmalı). Ayrıca §5.3 `push_ledger`'ın `ledger.entries`'e
yazdığını ve yeni koleksiyonların sırasını söylemiyor.

**D2. MCP "17/17" abartılı.** MCP döküm koşucusu stdio değil, süreç içi JSON-RPC nesnesi;
`01-handshake` hiç araç çağrısı yapmıyor (5 hello + 1 direct); 6 döküm toplam 14 adımı MCP'yi
atlayarak doğrudan oturuma veriyor (`input`, `clock`, `send_raw`, `hello` öncesi). `qp mcp` stdio
yalnızca duman testinden geçiyor. Ölçüt (≥ 3 döküm) yine de karşılanıyor.

**D3. Yanıltıcı hata iletisi:** `set_params period: 1e18` → "must be an integer >= 1, got
1000000000000000000"; değer kuralı karşılıyor, gerçek neden (üst sınır) söylenmiyor. Ajanın
"hatadan düzeltme" iddiası için önemli.

**D4. İzlenen `.pyc`:** `rounds/RD1/ergonomics/harness/__pycache__/ergo.cpython-313.pyc` depoda;
`score.py` çalıştırınca değişiyor ve çalışma ağacını kirletiyor. `.gitignore`'a `__pycache__/`.

**D5. Bayat ham çıktılar:** `raw/t6e/present-query*-out.json` `now` alanından önceki biçimde;
`t6e-presenter bench` yeniden yazınca fark çıkıyor. T6d ham baytları da 20 B eski.

**D6. Mimari hijyen:** `qp-session` (otorite crate'i) normal bağımlılık olarak `tempfile` ve
`std::process` kullanıyor (döküm koşucusu `conform.rs`); koşucuyu ayrı bir modül/özellik
arkasına almak otorite çekirdeğini saf tutar.

**D7. Küçük ölçüm farkı:** WASM gzip 475 242 B (rapor 475 126 B; +116 B, ortam farkı olabilir).

## REPORT.md'de abartılı ya da yanlış olanlar

1. "**Durum: Tamamlandı. On üç kabul ölçütünün on üçü kanıtlarıyla karşılandı**" ve ölçüt 13 ✅
   "`AUDIT.md`" — `2265a04`'te `AUDIT.md` yoktu; ölçüt denetimden önce ✅ işaretlenmiş.
   `STATUS.md` ve `ROADMAP.md` de "Tamamlandı, 13/13" diyor ve `STATUS.md` `AUDIT.md`'yi kanıt
   gösteriyor. Bu denetime göre: 10 ✅, 3 ⚠️.
2. Ölçüt 11 ve Ölçümler: "`qp conform --transport mcp` **17/17**" — doğru ama bir kısmı MCP'yi
   atlıyor (D2); "MCP üzerinden 11 döküm tamamen, 6'sı kısmen" demek daha doğru.
3. Ölçüt 8 ✅ ve T6 tabloları — sayılar tutuyor, ama T6a/b/c kodunun birleşik `main`'de
   derlenmediği söylenmiyor (O1).
4. Ölçüt 9: "Uygulamayı görmemiş, yalnızca şartname ve şemayı okuyan" — iddia makul ama ham
   kayıtlardan doğrulanamıyor (O3); RESULTS.md "kayıtlarda `raw/<tur>/AGENTS.md`" diyor, orada
   yönerge yok.
5. "`git diff main` boş" (qp-compute, qp-render, golden) — doğru (VERIFY 13).
6. Ölçümler "WASM modülü 475.126 B gzip" — bu kopyada 475.242 B (D7, önemsiz).
7. "Denetim (Denetçi alt ajan tamamlandığında dolduruldu.)" — bölüm boş; bu dosyaya göre
   doldurulmalı.

## Denetçinin kendi araçları (scratchpad, depoya girmedi)

`<scratchpad>/audit/replay/replay.py` (bağımsız oynatıcı), `<scratchpad>/audit/v7/*.jsonl`
(VERIFY 7 mesaj dizileri), `<scratchpad>/audit/adv/*.jsonl` (saldırgan mesajlar),
`<scratchpad>/audit/t6/` (T6 yeniden koşu çıktıları), `<scratchpad>/audit/verify.log`.
