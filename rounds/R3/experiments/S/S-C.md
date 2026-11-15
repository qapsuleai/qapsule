# R3/S-C — Protokol eki (T6) ve oturum bütünleştirmesi

Dal `r3/s-session` (`r3/int` `d83bf7a` üstünde). Yazılan yerler: `crates/qp-session/`,
`crates/qp-schema/src/protocol.rs`, `crates/qp-capsule/` (yalnız mühür, karne dizini,
ünlü uyumu), `crates/qp-mcp/tests/`, `docs/PROTOCOL.md` (§5.1, §5.3, §5.4, §15, yeni §20),
`docs/CONCEPTS.md` (yalnız ekleme; koddan önce ayrı commit), `fixtures/protocol/conformance/38–43`,
`Cargo.lock`. Var olan 37 döküm, altın görseller ve explain dosyaları değişmedi; c1–c5
fikstürleri de değişmedi (aşağıda §5).

## 1. Ne kuruldu

| Parça | Yer | Özet |
|---|---|---|
| Otoritede kapsül motoru | `qp-session/src/capsule.rs` (`Runner`) | `SessionInit.capsules {series, flow}` ile açılır. Kesinleşen her bar (RP2 olgunluğu) akışkan dedektörlerden (`qp_facts::default_detectors`) ve `Engine` (`EngineConfig::r3`) içinden geçer. Bakış başına ilerleyen seri için **tek motor girdisi** (`qp`), değişimsiz de olsa: `put_fact` (yalnız geçişe yol açan olgular, `evidence_ref` damgalı), hikâye başına `put_finding` + `put_capsule` (`story`, durum eşlemesi), kapanışlarda `push_ledger` (R3 alanları). Kişinin onay/ret kararı korunur. |
| Lisanslı akış | aynı | `qp_source::synthetic_flow(barlar, seed)` yalnız otoritede; sunucu olguları; mühür yorumları. Yetki `flow/SYMBOL` (§18.3 kuralı). |
| Mühür | `qp_capsule::trust::Muhur` (+ `trust/seal.rs`) | Deney crate'inden davranış aynı taşındı (yalnız yollar ve `MuhurPath` → `Muhur`). Çapraz denetim `crates/qp-capsule/tests/muhur_crosscheck.rs`: deneyin kaynağı `#[path]` ile derlenir; 3 tohum × 6 enstrüman × 400 bar'da iletiler, makbuzlar, denetim günlüğü ve `replay` bayt eşit. |
| İzdüşüm | `capsule.rs` (`Proj`, `as_reader`, `project_doc/changes/entry`) | §20.5: lisanslı doğumlu hikâye yetkisize hiç gitmez (yalnız mühür yorumu); herkese açık hikâyeye katılan lisanslı olgu gizlenir; bar yetkisi olmayana kanıt değeri ve `evidence_ref` yok, cümle rakamsız. Sorgu/abonelik/girdi/dikkat/seçici çözümü okuyucunun dokümanında; komut önce okuyucunun dokümanında kuru koşulur (varlık kâhini yok). |
| Kitle gizliliği | aynı | Güvenlik incelemesi (iki ORTA bulgu) sonrası: bir kişinin tercihleri (pozisyon, izleme listesi, seviye) başka okuyucuya **hiçbir** yoldan gitmez (`document`, `change`, `log` değişim/ters/istek/özet); yalnız kendisi ve `host`. `set_audience`/`set_sensitivity` öneri olamaz (öneri yükü herkese giderdi). |
| Sorgular, işlemler, konu | `qp-schema::protocol`, `capsule.rs` | `capsules`, `capsule`, `ledger`; `set_audience`, `set_sensitivity` (+ değişim `put_audience`); konu `capsules` (olay `capsule`, `server_comment`); `Surface.lang` (`tr`/`en`). MCP araçları şemadan üretildi (5 yeni araç; tool sayısı testi güncellendi). |
| İtiraz → geri çekme | `dispute.rs` + `capsule_dispute`, `capsule_settled` | Motor kapsülüne itiraz: çapadan `t`'ye kesin bar; düzeltilmiş barda dedektörler baştan; olgusu artık çıkmayan hikâye `retracted` (`dispute_upheld` / `data_corrected`), karnede yok. |
| Aday | `qp_session::capsule::candidates`, `Replica::candidates` | Yansı kendi verisinden; otorite hiçbir şey göndermez. |
| Sunucu/görünüm | `provider.rs`, `present.rs`, `query.rs`, `reading.rs` | `DocProvider::glyph` motorun `glyph_spec`'i; kapsül cümlesi okuyucunun (dil, yetki) gerçekleştirici cümlesi — motor varken ya da dil EN iken; yoksa saklanan başlık. |
| `x2` | `gops.rs`, `canon.rs` (`CHANNELS`), `summary.rs` | `set_encoding` `x2`'yi koyar/kaldırır; özet `… x a..b`. |
| S-B §3 kalanı | `canon.rs` (`licensed_value`) | `show_finding` (ve her köken çözümü) lisanslı olgunun kanıt değerini görünüme sabit yazmaz; yalnız köken kalır. Ayrıca izdüşüm gizli olguya başvuran görünümü yetkisize hiç göndermez. |
| Karne dizini | `qp-capsule/src/engine.rs` (`LedgerIndex`) | Anahtar başına ve genel, kapanış zamanı sıralı önek sayımı + geri çekilen satır listesi; `ledger_stat`/`ledger_stat_all` O(log n). Tanım `ledger_stat_scan` korunur. |
| Ünlü uyumu | `realize/morph.rs` (`PALATAL_L`) | `sinyal` → `Sinyalin` (ince l: `sinyal, kontrol, rol, hal, alkol, petrol, gol`). |

Kavramlar önce: `docs/CONCEPTS.md` ekleri (Okuyucu, izdüşüm, otoritede kapsül motoru, mühür,
sunucu yorumu, kapsül konusu, kitle işlemleri, ilgili kapsül, aday okuması, kart paneli,
panele göre konum) ayrı commit `8bfcf2f`.

## 2. Protokol özeti (`docs/PROTOCOL.md` §20)

- Sorgu `capsules {instrument?, stages?, open?, meaning?, since_bar?, since_t?, audience?, limit?}`
  → `CapsuleItem` (kimlik, enstrüman, tür, yön, aşama, durum, kademe, önem, teyit, güven,
  `ledger_at_birth`, `sentence {text, tr, en}`, `relevant`, `redacted`) önem sırasıyla +
  yetkisiz akış için `comments`. (`kind` alanı etiketle çakıştığı için tür süzgeci `meaning`.)
- Sorgu `capsule {capsule}` → hikâye geçmişi (neden kodlarıyla), olgular, cümle, şimdiki
  karne, `glyph`, olgu başına `evidence {open, values?, evidence_ref?, requires?, rests_on}` (K34).
- Sorgu `ledger {key?, meaning?, detector?, by_kind?}` → satırlar + toplam; motorun karnesi
  (lisanstan bağımsız, herkese aynı).
- İşlem `set_audience`, `set_sensitivity` → `put_audience`; başkasınınki yalnız `owner`;
  öneri olamaz; sistem yazmaz.
- Konu `capsules {instrument?, all?}` → `capsule` olayı (öğe + geçiş + `seq`) kitle süzgeciyle
  (izleme listesi, kademe; pozisyon/niyet geçişi), yetkisize `server_comment` (`trust: server`).
- Kapsül onayı/reddi: var olan `approve_finding` / `reject_finding` (tek yol).

## 3. Dökümler (yeni; üç taşımada geçer)

`cargo run -p qp-session --example make_capsule_transcripts -- --write` (beklentiler süreç
içi otoritenin tam cevabı; `welcome` yalnız tür + `seq`). Veri: `synthetic_market(1, 160, 1)`
THYAO günlük, besleme `bist`, akış tohumu 1.

| # | Ne gösterir |
|---|---|
| 38 | yaşam döngüsü: geçersiz (bariyer stop), lisanslı akış doğdu/geçersiz, kırılım doğdu → teyit (hacim, akış: yeni aile) → güçlendi → gerçekleşti, aşırılık doğdu; `capsules`, `capsule`, `ledger` (anahtar, tür) |
| 39 | yetki: Ana (`*`) lisanslı hikâyeyi rakamlarıyla açar; Bora (`bist/THYAO`) aynı kimliğe `not_found` alır, yerine mühür yorumları ve karne; Can (yetkisiz) rakamsız cümle ve `requires: bist/THYAO` |
| 40 | K26: 10 bar önce başlayıp biten durum, TR "… artık kısa ortalamanın altında değil", EN "no longer below"; süresi dolan uyumsuzluk |
| 41 | hassasiyet: `future` grubu yalnız kademe 1; pozisyon her kademeyi geçirir; `set_sensitivity`; başkasına yazmak `forbidden`; son doküman tercihleri değişmemiş gösterir |
| 42 | aday: bar içinde oluşup kaybolan kırılım; otorite açık barda sessiz; kapanışta tek motor girdisi, hikâye yok, karne aynı |
| 43 | geri çekme: kötü baskı → sahte kırılım → yansı itirazı haklı → `retracted` (`dispute_upheld`; aynı bara dayanan bir hikâye de `data_corrected`); ikinci itiraz `duplicate`; karnede yok |

`qp conform fixtures/protocol/conformance`: **43/43** inproc, serve, mcp.

## 4. Testler ve yetki taraması

`crates/qp-session/tests/capsules.rs`:

- `replaying_the_log_reproduces_the_capsule_history` (ölçüt 4): 100 bar canlı akış; iki koşu
  günlük ve çıktı bayt eşit; artımlı = baştan (bayt eşit); `replay(başlangıç, günlük)` =
  doküman (bayt); >20 hikâye; her karne satırı kapanmış hikâyenin.
- `entitlement_sweep_no_licensed_value_reaches_an_unentitled_reader` (VERIFY 10): Bora (bar
  yetkili, akış yetkisiz) ve Can (yetkisiz) 7 konuya abone; 100 bar; Ana lisanslı bulguyu
  `show_finding` ile görünüme koyar; sonra her ikisi `describe, document, log, capsules
  (±audience), ledger, binding, screen`, her koleksiyonun `list full`, dokümandaki **her**
  kimlik için `get, capsule, present (desktop, phone), verify_evidence`, seçicili `present`
  ve lisanslı bulguya `approve_finding` gönderir. **Sonuç:** Bora'ya giden hiçbir satırda
  lisanslı olgu/hikâye/bulgu kimliği ve lisanslı kanıt değeri (>5 farklı değer arandı) yok;
  mühür yorumları ve karne var. Can'a giden hiçbir satırda R3 olgularının kanıt değeri, veri
  özeti (`evidence_ref.digest`) ve kapsül cümlesinde rakam yok; kanıt erişimi kapalı,
  `requires`/`rests_on` dolu. Ana lisanslı hikâyeyi rakamlarıyla açıyor.
- `a_candidate_that_disappears_leaves_no_trace` (VERIFY 9): açık bar 143'te
  `candidates` `cap.THYAO.breakout.up.143`'ü (aşama `candidate`) verir; yama sonrası yok;
  kapanışta tek motor girdisi; günlükte `breakout.up.143` ve `candidate` hiç geçmez; karne
  satırı yok.
- `k26_ended_state_past_tense_tr_and_en`, `the_engine_never_changes_the_audience` (100 bar,
  motor girdilerinde `put_audience` yok, tercihler aynı), `a_persons_audience_reaches_nobody_else`
  (öneri yolu dahil: öneri kipinde `forbidden`, öneri oluşmaz; B ve owner'a hiçbir satırda
  sırlar yok; kişi kendininkini, host hepsini görür).
- `crates/qp-capsule/tests/ledger_index.rs`: dizin = satır taraması (geri çekmeli, her anahtar,
  her zaman ve uç zamanlar).

## 5. Ölçümler

**Karne dizini** (`cargo run --release -p qp-capsule --example ledger_index`;
`synthetic_market(30, 500, 1)` + akış; Apple M4 Max sınıfı, yük ortalaması ~7–20):

| | Önce | Sonra |
|---|---:|---:|
| olay JSON özeti (11.951 olay, 9.957.165 B) | `b62df7d557c4e722` | `b62df7d557c4e722` |
| karne satırları özeti (4.240) | `7a831f94d277363d` | `7a831f94d277363d` |
| olgular özeti (8.405) | `2c1ab65d49188b25` | `2c1ab65d49188b25` |
| `ledger_stat` (14.000 çağrı, 7 anahtar) | 5.784 ns/çağrı | 14 ns/çağrı |
| `simulate` duvar saati | 0,245 s | 0,146 s |

Var olan ileriye bakma ve yeniden oynatma testleri (`tests/r3.rs`) ve S-A benzetim eşitliği
değişmeden geçiyor.

**Değişen cümleler (ünlü uyumu, M2'nin 73 durumu):** 7 TR cümlesi, yalnız "Sinyalın" →
"Sinyalin": s07, s09, s14, s23, s29, s35, s46 (EN değişmedi). s46 S-A'nın 14'ü arasındaydı;
yeni 6'sı ile `product_changes` 20. Örnek s09: "… yukarı kırılım. Sinyal**ın** 1 barlık ömrü
bugün sonuçsuz doldu …" → "… Sinyal**in** 1 barlık ömrü …". `m2_sentences.json` crosscheck
`fixture` komutuyla yeniden üretildi (`measured` aynı).

**c1–c5:** cümle motor çalışırken ya da dil EN iken gerçekleştiricinin; fikstür
dokümanlarında motor yok, dil TR → saklanan başlık; c1–c5 metinleri ve altınları değişmedi
(yeniden üretilmedi).

## 6. Doğrulama

`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `QP_VERIFY_BROWSER=0 scripts/verify.sh` (`CARGO_INCREMENTAL=0`):
**verify: OK** (76 s, ikinci koşu; ilk koşu yalnız `clippy`'de iki döngü indeksi uyarısıyla
düştü, düzeltildi). Bütün adımlar PASS; `browser` SKIP. `conform` 43 döküm × inproc, serve,
mcp; `views` 12 görünüm ×2 bayt eşit; `wasm` ham 1.943.806 / gzip 616.022 B (S-B ile aynı:
varsayılan paket oturumu içermez); `digest` `7b1e99d40889f83c`. `git diff r3/int -- fixtures/`
yalnız 6 yeni dosya (38–43); var olan dökümler, altınlar ve explain dosyaları değişmedi.

Bağımsız denetim bu görevde yapılmadı (CLAUDE.md: denetim ayrı bir denetçi alt ajanla;
koordinatörün güvenlik incelemesi iki ORTA bulgu verdi, ikisi de düzeltildi: kitle
tercihlerinin başka okuyuculara gitmesi ve öneri yolu).

## 7. Açık konular ve öneriler

- **Crosscheck testi:** `rounds/R3/experiments/S/crosscheck` (yazma alanım dışında) değişen
  durum listesini 14 olarak sabitliyor (`CHANGED`); ünlü uyumu düzeltmesiyle 20 oldu. O
  crate'in `CHANGED` listesine s07, s09, s14, s23, s29, s35 eklenmeli (ana akış).
- **Otorite yeniden başlarsa** motor durumu (dedektörler, karne kayıtları, sayaçlar) bellekte;
  dokümandan geri kurulmaz. Yeniden başlatmada bağlı seri baştan koşulursa aynı hikâyeler
  aynı kimliklerle yeniden yazılır (belirleyici), ama günlük ikinci kez yazar. Kalıcılık
  (motor anlık görüntüsü ya da günlükten yeniden kurma) ayrı iş.
- **Akış besleme sözleşmesi:** gerçek barındırıcı için `FlowSource` arayüzü yok; yalnız test
  sentetik akışı (`init.capsules.flow`). Akış her kesinleşme turunda barlardan baştan üretilir
  (önek kararlı; O(n) / tur).
- **Seq kanalı:** `seq` tektir; motor girdisi her bar kapanışında yazıldığı için lisanslı
  hikâyenin varlığı seq ritminden çıkmaz; ama bir girdinin **değişim sayısı** izdüşümde boş
  görünse de girdinin kendisi görünür. Özet gizli kimlik içerirse `…`.
- **Karışık hikâyenin kademesi:** lisanslı olgu herkese açık hikâyeyi teyit ettiyse yetkisiz
  okuyucu hikâyeyi lisanslı olgudan önceki kademesiyle görür; kitle süzgeci bu kademeyle
  çalışır (lisanslı teyit yetkisizin itilmesini değiştirmez). Gerçekleşme/geçersizlik
  geçişleri yine görünür (fiyat verisinden).
- **K34 ile M6 kuralı:** lisanslı doğumlu hikâyenin kanıtı yetkisize "gerektirir" notuyla
  değil, yalnız mühür yorumunun ekleriyle (`entitlement: flow/*`, `evidence:
  flow.institutional/net`) söylenir — M6 bütünleştirme kuralı varlığın ayrıca gitmesini
  yasakladığı için. Ana akışın onayı gerekir.
- **Kitle gizliliği:** `host` herkesin tercihini görür, `owner` başkasınınkini yazabilir ama
  okuyamaz (bilinçli karar). Yansı aynaladığı dokümanı izdüşümle alır; akış yetkisi olmayan ya
  da başka kişilerin tercihleri olan dokümanda yansı ile otorite doküman özetleri farklıdır.
- **Aday yansıda** açık dedektörlerle (akış yok) ve kesin barlardan baştan kurulan motorla
  hesaplanır; otoritenin akışla teyit ettiği hikâyelere katılım adayda görünmez.
- **İtirazın kapsamı:** kesin bar cevabı yalnız düzeltilmiş barı ve sonrasını etkiler;
  düzeltmeden sonra doğan hikâyeler ve sonuç kuralının bozuk barla verdiği kararlar ayrıca
  sınanmaz (yalnız olguları sınanır).
- **Sensitivity önerisi** (S-A §7): pozisyon geçişinin bütün aşamaları geçirmesi aynen kaldı.
