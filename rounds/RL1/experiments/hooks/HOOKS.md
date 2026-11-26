# Eksik kancalar — çözümleriyle (D1 şart 3)

Kaynak: `main` @ `64e30256`. Bu bir çözümleme; kod değişmedi. Satır numaraları bu commit'e göredir.
Ölçek: `scripts/arch-check` → `qp-authority` **12.744 test dışı satır = İ3 tavanı (12.744)**.
Mandal tam dolu: çekirdeğe eklenen **her net satır** ya karar satırı (K89, tavan yükseltme) ya da
aynı değişiklikte eşit silme ister. Aşağıdaki "çekirdek satır" tahminleri bu yüzden kararın konusudur.

## 1. `ConditionKind` — `Condition`'a kol

**Bugün:** `qp_schema::Condition` (`qp-schema/src/types/document.rs:62`, `tag="kind"`,
`deny_unknown_fields`) üç kollu. `Registry::eval_condition` (`qp-subsystem/src/registry.rs:253`)
kaydı `kind_of(c)` ile arar (`:262`), ama yeni `kind` dokümana hiç giremez: serde reddeder.
Kayıtlı bir tür ancak var olan üç türden birini *gölgeleyebilir*.

**Kapalı eşlemeler (kol eklenince derleme kırılır):**

| Yer | Ne | Ext kolu ne yapmalı | Crate |
|---|---|---|---|
| `qp-plane/src/values.rs:415` `condition_series` | koşulun okuduğu seriler | `reads`'i döner (aşağıda) | plane |
| `qp-plane/src/values.rs:452` `eval_condition` | kayıtsız değerlendirme | `None` (kapalı başarısızlık) | plane |
| `qp-ops/src/live.rs:30` `cond_ids` (en çok 2) | bellek yuvaları, `closure_of` | `[reads[0], reads[1]]` | ops |
| `qp-authority/src/data/live.rs:257` `eval_slots` | yuvadan değerlendirme | `None` → kimlikle (kayda) düşer | **çekirdek +1** |
| `qp-authority/src/capsule/claims.rs:222` `eval_at` (görevde "holds_at") | kanıt, itiraz, düzeltme | ya `return None` (+1, kanıtsız) ya kayda devret (aşağıda) | **çekirdek** |

`matches!(…, Condition::Text{..})` atlamaları (`live/eval.rs:198`, `session/events.rs:267`,
`session/presence.rs:42`, `public/commit.rs:221`) ve `let-else`'ler (`qp-replica/src/tape.rs:363`,
`qp-viewer/src/tape.rs:222`, `qp-program/src/guard.rs:84`, `qp-ops/src/expand.rs:201`) kırılmaz.

**Önerilen biçim:** `Condition::Ext { ext: String, reads: Vec<String>, params: serde_json::Value }`
(`kind: "ext"`). `reads` belgede *beyan edilen* bağımlılıktır; bu sayede kapanış, hazır olma,
veri yetkisi ve kanıt beslemeleri var olan yoldan çalışır (`condition_series` üstünden):
`hears`/`cond_blocked` (`live.rs:223,232`), `claim_series` (`claims.rs:137`), `feeds_of`
(`claims.rs:555`), `unentitled` (`qp-plane/src/plane.rs:444`), açılımın yetki/bağ denetimi
(`qp-ops/src/expand.rs:198`, `expand/bind.rs:26`). Kayıt `Registry::eval_condition`'da `kind_of`
yerine `Condition::Ext{ext,..}` → `find(&self.conditions, ext)` (qp-subsystem, çekirdek değil).

**Canlı yol belleği ("etkilenmedi"):** `eval_memo` (`live.rs:492`) kayıt varken belleği zaten
atlar (`:500`, `evaluates_conditions` `registry.rs:242`). Asıl kayıp *etkilenme* süzgecinde:
- abonelik: `live/eval.rs:287–295` — `cond_ids` `[None,None]` ise `(None,_) => false`; bellek
  dolduktan sonra koşul **bir daha değerlendirilmez**. Düzeltme: Ext için
  `info.rests_on_any(changed)` (+2 satır).
- niyet ve kapsül: `live/eval.rs:369, 427` `rests_on_any` — `CondInfo::closure` `cond_ids`'ten
  kurulur (`live.rs:405–431`); boşsa hiç etkilenmez. Düzeltme: `cond_info` döngüsü (`live.rs:407`)
  ve `closure_of` (`qp-ops/src/live.rs:12`) `condition_series` üstünden (sıra var olan türlerde
  `cond_ids` ile aynı → bayt eşit; ±0 satır). Üçten çok `reads` de böylece kapanışa girer.

**Kanıt:** `eval_at`'ın kayda erişimi yok; çağıranlar `claims.rs:239` (`evidence`), `claims.rs:380`
(düzeltmede yeniden sınama), `dispute.rs:648`, `qp-replica/src/replica/judge.rs:176`. Ext kanıtı
için `ConditionKind::holds_on(c, &vals) -> Option<bool>` ve `eval_at(.., reg: &Registry)`:
çekirdekte ≈ +8–10 satır (imza + 3 çağrı + kol).

**Bar başına değerlendirme (0,1,0 kaybı):** canlı yol girdi başına bir kez, son değerde
değerlendirir (`live/eval.rs:313` `eval_memo` → `prev/now` kenarı). Bir girdide birden çok bar
kapanınca aradaki kenar kaybolur. Çözüm iki yol:
- **Çekirdek:** `ConditionKind::edges(rd, cache, c, from_bar) -> Option<Vec<(bool,u64,i64)>>`
  (varsayılan `None`), `from_bar` = `Delta::first_bar` (`qp-plane/src/values.rs:40`); üç döngüde
  (`live/eval.rs` aboneler ~313, niyetler ~372, kapsüller ~433) tek değer yerine kenar listesi,
  izleyici başına son bar zamanı. ≈ +55–70 çekirdek satır.
- **Yumuşak (çekirdek 0):** `Phase::AfterData` / `Moment::Data { seq, changed: &[(id, Delta)] }`
  (`qp-subsystem/src/phase.rs:59`) her veri girdisinden sonra alt sisteme değişen serileri ve ilk
  değişen barı verir. `qp-program` olay serisinin yeni kapanan barlarını tarar, her olay barı için
  kendi itilen konusuyla (`TopicHandler::pushing/push`) bildirir ve `Outbox::entry` ile kayıt
  yazar. Eksiği: çekirdeğin `Claims` defterine (`claims.rs` `assert`) girmez; niyet (`intent`)
  tetiklemesi yine son değerdir.

**Bayt eşitliği:** Evet. Kol eklemeli (serde), var olan dökümlerde `kind:"ext"` yok → hiçbir yeni
kol alınmaz; `cond_info` değişikliği sırayı korur; `fixtures/` ve `transcripts`'te serde'nin
"expected one of" listesi kayıtlı değil (tarandı). Üretilen JSON Şeması/TS tipleri değişir (üretilir).

## 2. `ValueSource` — neden yalnız koşulda soruluyor

`Registry::eval_condition` (`registry.rs:265–281`) tek çağıran; katman sırası
`qp-plane < qp-ops < qp-subsystem < … < qp-authority` olduğundan veri düzlemi, gramer sağlayıcısı
ve açılım kaydı göremez. Kayıtlı kaynağın kimliği dokümanda seri değilse:

| Yol | Fonksiyon | Bugünkü davranış | Görmesi için |
|---|---|---|---|
| Çizim | `qp-ops/src/provider.rs:128` `DocProvider::series_kind` (`find_series`) | `None` → çizilmez | sağlayıcıya kaynak; **yansı (`qp-viewer`, WASM) kaydı taşımaz** → değer telde yok, yansı çizemez |
| Önbellek | `qp-plane/src/values.rs:346` `values` (`:350` "no series") | önbelleğe girmez; `apply_delta` güncellemez | `Cache`'e kaynak tablosu (plane'de trait) |
| Bağımlılık | `live.rs:405` `cond_info` (`pos.get` → `continue`), `qp-ops/src/live.rs:10` `closure_of` | kapanış boş → hiç etkilenmez | `ValueSource::inputs(id)` |
| Hazır olma / yetki | `plane.rs:444` `unentitled`, `plane.rs:477` `gate` (`find_series` yok → `continue`); `live.rs:463` `Ents::hears` (beslemesiz → `true`) | **açık başarısızlık: yetkisiz okuyucu duyar** (D1'deki sızıntı) | aynı `inputs` + kapalı başarısızlık |
| Kanıt | `claims.rs:555` `feeds_of` (`find_series?`) | `None` → kanıt yok (kapalı) | `inputs` |

Kaynak dokümanda bir serinin kimliğine *sahipse* (D1'in yolu: `SeriesSource::Study.program`,
`values.rs:381`) hepsi kendiliğinden çalışır: girişler beyanlı, hesap `qp-plane`'de, yansı da
hesaplar. Kayıt tabanlı `ValueSource`'u tüm yollara taşımak: çekirdek ≈ +30–50 satır, plane/ops'ta
imza değişiklikleri ve yansı için değerlerin telde taşınması (protokol değişikliği). **Önerilmez.**
Öneri: `ValueSource`'u sözleşmeden çekmek (yalnız `qp-subsystem`; kaydeden yok, bayt eşit) ve
"değer kaynağı = girdileri beyanlı doküman serisi, hesabı `qp-plane`'de" kuralını yazmak. Kaldı
kalmazsa en azından `evaluates_conditions`/`eval_condition` yolu kapalı başarısız olmalı.

## 3. Öbür eksikler

| Kanca | Yer | Bugün | Çekirdek çözüm | Yumuşak çözüm (çekirdek 0) |
|---|---|---|---|---|
| `SessionView::param` | `qp-subsystem/src/view.rs:91` varsayılan `None`; `impl SessionView for Session` (`authority/session/phases.rs:77–134`) ezmez | plan/kullanıcı katmanı hiç yok; `resolve` (`qp-schema` `params`) yalnız sistem | `param_for(principal,key)` (qp-subsystem varsayılanı) + Session'da kişi→değer tablosu + barındırıcı ayarlayıcısı ≈ +12 | `params` alt sistemi: değerler kendi yuvasında (`init`/girdi ile), öbürleri `ctx.slots` okur |
| `param_for` açılımda | `qp-ops/src/expand.rs:37` `Ctx` (alan yok) | `qp-program` açılımı kayıt defteri varsayılanıyla | `Ctx`'e alan; 9 `Ctx {}` hazırı: `worlds.rs:13,109,240`, `command.rs:123,184,251`, `intake.rs:276`, `accept.rs:29`, `live/eval.rs:39` → +9 (aşağıdakiyle tek alan) | kabulde denetimi `Check`'e taşı: `Check` `accept_proposal`'ı görür, öneri işlemlerini `view.document()`'tan okur |
| Karar verenin kimliği | `accept.rs:12` `decider: &str` elde; `Ctx` yalnız `approved_by: Option<Author>` (`:31`) | açılım kişiyi bilmez | yukarıdaki alana `decider: Option<&str>` (aynı +9; ayrı yapılırsa +9 daha) | `Check`'te bağlantının ilkesi = karar veren |
| Kamu yansısı `Check`'e ulaşmaz | `check_command` (`phases.rs:433`) yalnız `command.rs:151`'de, otoritenin dünyasında; `pub_expand` (`worlds.rs:8`) `in_world(true)` içinde yalnız açılım | `qp-program/src/check.rs:237`'deki `in_public` dalı ölü | `pub_expand` içinde `check` çağrısı ≈ +4–6; reddi sessizce `pub_side = None` yapar → kamu dökümleri değişebilir, **ölçmeden bayt eşit denemez** | bugünkü sayım (açılım her dünyada örnek+maliyet+kendi baytı) en sıkıdır (PS §6); ertele |
| Abonelik reddi | `push.rs:20–28` `dispatch_subscribe` | yerleşik `condition` konusu alt sisteme sorulmaz sanılıyordu | — | `qp-program` `"condition"` etiketli `TopicHandler` kaydeder; `subscribe` `Err` döner → abonelik düşer (`:24–26`). Tek sahip kuralı: izleme alt sistemi de isterse çakışır |

Bayt eşitliği: `Ctx` alanı ve `param_for` yalnız eklemedir; varsayılan `None` ile hiçbir dal
değişmez → dökümler/altınlar/özetler eşit (`Ctx` telde değil). Kamu `Check`'i istisnadır (yukarıda).

## 4. Özet: çekirdek satırları ve bayt eşitliği

| Değişiklik | qp-authority/src net | Bayt eşit | Yumuşak yol |
|---|---|---|---|
| A. `Condition::Ext` + canlı yol (eval_slots, etkilenme, kapanış), kanıtsız | +4 | evet | yok (kol çekirdekte eşlenir) |
| A+. Ext kanıtı (`eval_at` kayda) | +8–10 | evet | — |
| B. Bar başına kenar (`ConditionKind::edges`) | +55–70 | evet (kayıtlı tür yoksa dal alınmaz) | `AfterData` taraması + itilen konu, 0 |
| C. `ValueSource`'u tüm yollara taşımak | +30–50 (+ protokol) | yansı yüzünden hayır | doküman serisi (D1), 0 |
| C'. `ValueSource`'u çekmek | 0 | evet | — |
| D. `param_for` (SessionView) | +12 | evet | `params` yuvası, 0 |
| E. `Ctx` eki (`param_for` + `decider`, tek alan) | +9 | evet | `Check`'te, 0 |
| F. Kamu dünyasında `Check` | +4–6 | ölçülmeli | ertele |
| G. Abonelik reddi | 0 | evet | `TopicHandler "condition"` |

## 5. R7 ve RS1 hangi kancaya dayanıyor

| İhtiyaç (ROADMAP R7/RS1, PS §8) | Kanca |
|---|---|
| R7 izleme görevi = program olay serisi + `threshold` niyeti (tek kenar) | yok (bugün çalışır); toplu eklemede kayıp sınırı kalır |
| R7 "çizimden doğan izleme sessizce yanlış kurulmuyor" — hızlı yeniden oynatma/yeniden çözümde olay kaçmaz | B (ya da yumuşak B) |
| R7 ajanın "kur ve göster"i, `scope: personal` kişi bütçesi | D, E (ya da `Check` yolu) |
| R7 çizim bağı / seviye sürükleme (`set_program_params`, `since_t`) | yok |
| R7 izleme alt sistemi `condition` aboneliğini süzmek | G |
| RS1 canlı sicil: **her** olay barı kanıtlı kayıt (`mode: live`, `ClaimEvidence.program`) | B + A+ (çekirdek defterde) ya da yumuşak B (alt sistem defteri) |
| RS1 birden çok doküman üstünden kişi bütçesi, K76 ortak stratejiler | D |
| RS1 iki aşamalı kırılım `rule.kind: breakout` | olay serisi yeter (program içinde); A gerekmez |

**R7 öncesi karar için önerilen en küçük küme:**
1. **C'**: `ValueSource`'u sözleşmeden çek (0 çekirdek); değer kaynağı = doküman serisi.
2. **Bar başına**: önce **yumuşak B** (`AfterData` taraması, itilen konu, alt sistem kaydı; 0
   çekirdek). RS1 sicilinin çekirdek `Claims` defterinde olması şartsa **B + A+** (≈ +65–80, İ3
   mandalı için karar satırı) — bu, RS1'den sonraki "canlı yol alt sisteme" ayrımıyla (İ3 borcu)
   birlikte yapılmaya daha uygun.
3. **D + E**: `param_for` ve `Ctx` eki (≈ +21 çekirdek, bayt eşit) — RS1 kişi bütçesi için yumuşak
   yol yetmez (açılım yuvayı görmez); R7'ye kadar `Check` yolu ve `params` yuvası köprü olabilir.
4. **A** (`Condition::Ext`) R7 için **gerekmez**: olay serisi + `threshold` aynı işi görüyor; yalnız
   bar başına çekirdek yol (B) seçilirse onunla birlikte gelir.
5. **G** sıfır maliyetle hemen; **F** ölçüm yapılmadan ertelenir.
