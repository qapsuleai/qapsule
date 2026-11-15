# RP2 / T4 denetim düzeltmeleri (Y1, Y2, O1–O3, D2–D4)

Dal: `rp2/t4fix` (`rp2/int` @ 34e9892 üstünde). Denetim: `rounds/RP2/audits/T4.md`.
Yazılan yerler: `crates/qp-session/` (kaynak, `tests/t4_fixes.rs`, `src/conform.rs`'de
`qp serve` koşucusu), `crates/qp-cli/` (yalnız `serve --host-frames` çıktısı ve testi),
`docs/PROTOCOL.md` (§3, §15, §18.3, §18.4, §18.5, §18.7),
`fixtures/protocol/conformance/34-entitlement-anchor-gap.json` (yeni), bu dosya.
`qp-schema`, `qp-source`, `qp-compute`, `qp-mcp`'ye dokunulmadı. **Var olan 33 dökümün hiçbiri
değişmedi**; üç taşımada geçiyorlar (aşağıda "Doğrulama").

## Bulgu → düzeltme → test

### Y1 (yüksek) — koşullar değer kehaneti

**Kural (§18.3, "Koşul yazmak veri okumaktır").** Bağlı veride değerlendirilen bir şeyi
yazmak ya da ona abone olmak, yazanın / abonenin koşulun okuduğu (hesap girdileri
üzerinden, özyinelemeli) her bağlı beslemeye veri yetkisi ister; yoksa `no_entitlement`
koşulun serisinin yolunda.

**Düzeltme.**

- Yazı tarafı: `DataPlane::unentitled(doc, principal, series)` (`plane.rs`) dokümandaki
  bağlara bakar (çözüme değil; aynı komutta konan bağ da sayılır), yalnız yetkiyi denetler.
  `expand::cond_entitled` / `condition_entitled` bunu `add_finding` ve `update_finding`
  geçersizliğine (`/ops/i/invalidation/{series,left,right}`), `add_intent` tetikleyicisine
  (`/ops/i/trigger/…`) ve `subscribe` `condition` konusuna (`/topic/condition/…`) uygular.
  `Ctx`'e her zaman bulunan `plane` alanı eklendi (bağlı seri olmayan dokümanda da, aynı
  komutta konan ilk bağ için). Öneri: önerme anında (kuru koşu) ve kabulde önerenin yetkisi.
- Bildirim tarafı (aboneye göre süzgeç; otomatik güvenlik incelemesinin bulgusu da bu):
  `live::hears(plane, doc, principal, cond)`. `condition` aboneliği yetkisiz abone için
  **değerlendirilmez** (kenar durumu korunur) — hem artımlı yol (`eval_conditions_live`)
  hem baştan yol (`eval_watchers`). `intent_triggered` yalnız tetikleyicinin beslemelerine
  yetkili `intents` abonelerine gider (`Session::intent_lines`, iki yolun ortak dağıtıcısı).
- Değerlendirme tarafı (yazı denetimini atlatan yollar için): oturumun bir kişisinin
  yazdığı bulgu geçersizliği, o kişi koşulun beslemelerine yetkili değilse
  değerlendirilmez (`invalidate_these`). Böylece "kendi belge içi serime koşul yaz, sonra
  seriyi bağlı seriye çevir" ve "bağı geri alma ile geri getir" yolları kapanır. Yazarı
  oturumun kişisi olmayan bulgular (başka yerden gelen doküman, motor) eskisi gibi
  otoritenin görüşüyle değerlendirilir.
- Kapsül durumu anlam düzleminde kalır. **Kalan sızıntı Arkin'e açık soru** (§18.3):
  yetkili bir yazarın koşulu kapsül durumuyla yetkisiz okuyucuya bir bit söyler; yetkisiz
  `editor` yetkili birinin bulgusunu `update_finding` ile belge içi seriye çevirip sonra
  seriyi bağlarsa koşul yazarın adına değerlendirilir. Seçenekler: (a) kapsül durumunu
  yetkisize gizlemek, (b) koşulun okuduğu serinin bağını yazanın yetkisini de istemek,
  (c) gevşek bırakıp lisans metnine yazmak.

**Testler** (`crates/qp-session/tests/t4_fixes.rs`):

| Test | Ne gösterir |
|------|-------------|
| `y1_binary_search_learns_nothing` | Denetimin saldırısı: `31-no-entitlement` init'i, `editor` 20 deneme (`px gt X`, kapsüllü bulgu). Her deneme `no_entitlement` `/ops/0/invalidation/series`; X dışında 20 cevap **aynı** (bilgi yok). `rsi14` (hesap üzerinden), `compare` sağ yanı (`/right`), `update_finding`, `mode: propose` da reddediliyor; yetkili owner yazabiliyor; yeniden oynatma eşit |
| `y1_condition_subscription_and_intent_refused` | `viewer` `subscribe condition px` → `no_entitlement` `/topic/condition/series`; `editor` `add_intent` (`rsi14`) → `/ops/0/trigger/series`; belge içi seri herkese açık |
| `y1_rebinding_does_not_open_an_oracle` | `editor` ve owner aynı koşulu kendi belge içi serilerine yazar, sonra seriyi THYAO'ya bağlar: owner'ın kapsülü `invalidated`, `editor`'ünkü `live` kalır (veri olayından sonra da) |
| `y1_notifications_are_gated_per_subscriber` | Owner'ın bağladığı seride `viewer`'ın (bağlanmadan önce yazdığı) koşul aboneliği ve `intents` aboneliği: veri olayında owner `condition_met` + `intent_triggered` alır, `viewer` hiçbir şey almaz. Artımlı ve baştan yolda aynı |

Döküm 34 (adım 4–7) aynı kuralı üç taşımada gösterir.

### Y2 (yüksek) — çapa damgasının geri alınması

**Düzeltme.** `expand::anchor_stamps(entry)`: yazarı `qp`, her değişimi `anchor_t`'li bir
bağlı serinin `put_series`'i, tersi aynı serinin `anchor_t`'siz hali olan girdi çapa
damgasıdır. `expand_undo` böyle bir girdiyi hedef alan geri almayı (owner dahil)
`undo_conflict` ile reddeder: "entry N is the authority's anchor stamp of bound series
`px`: an anchor is reset only by a command that rewrites the binding (§18.3), never by
undo", ipucu: bağı `put_series` ile yeniden yaz ya da bağı koyan girdiyi geri al. Başka
bir yol aynı bağa farklı çapa damgalamaz: çapa yalnız `anchor_t`'siz bağda damgalanır,
`anchor_t`'yi silen tek değişim bağı yeniden yazan komut (ya da aşağıdaki O2 geri alması,
o da bağı önceki çapasıyla ya da hiç geri koymaz).

**Test.** `y2_anchor_stamp_cannot_be_undone`: `32-verify-evidence` init'i; `undo seq 1` →
`undo_conflict`, ileti "anchor" içerir, `anchor_t` aynı; 5 veri olayından sonra (denetimin
senaryosu) yine aynı; yeniden oynatma eşit. Döküm 34 adım 9.

### O2 (orta) — owner kendi `put_series`'ini geri alamıyordu

**Düzeltme.** `expand_undo` çatışma taramasında, hedef girdinin koyduğu ya da yeniden
yazdığı bağlı seri için arkasından gelen çapa damgası (damgaladığı bağ, hedefin koyduğu
bağla `anchor_t` dışında aynıysa — `plane::same_but_anchor`) çatışma sayılmaz. Geri alma
hedefin tersini uygular: yeni seri silinir (damga onunla gider), yeniden yazılan seri
**önceki bağıyla ve önceki çapasıyla** geri gelir (düzlem onu çapadan yeniden çözer, yeni
damga yok). Damgadan sonra başka biri seriyi değiştirdiyse çatışma kalır.

**Test.** `o2_own_put_series_undoes_through_its_anchor`: `put_series px3` (seq 2), damga
seq 3; damganın geri alınması yine `undo_conflict`; `undo` → `applied`, `undoes: 2`, `px3`
yok, yeniden oynatma eşit. Sonra `px` penceresi `last 100` ile yeniden yazılır (yeni çapa),
`undo` → eski çapa geri gelir, `at px bar 250`'nin `t`'si yeniden yazmadan öncekiyle aynı,
yeniden oynatma eşit. Döküm 34 adım 10–11 (`undoes: 3`).

### O1 (orta) — boşluk barındırıcıya bildirilmiyordu

**Düzeltme.** `DataPlane::apply_event` bağ başına `stale`'e **geçişte** bir
`HostNotice` üretir: `Gap { series, feed, expected, got }` ya da (büyüme sınırı)
`Stale { series, feed, reason }`. Yeni `Session::host_data_event(ev) ->
Result<DataOutcome { lines, notices }, DataError>`; `Session::data_event` imzası
korundu (yalnız `lines`; MCP koşucusu ve T5 testleri değişmedi, birleştirme çatışması
azaldı). `handle_host_frame` artık `DataOutcome` döner; `qp serve --host-frames` her
bildirimi bildirimlerden önce `{"host":{"gap":{…}}}` / `{"host":{"stale":{…}}}` satırı
olarak yazar. Bildirim hata değildir (olay alındı; uygulanacak bir şey yok).
`qp conform`'un `serve` taşıması `host` satırlarını atlar (bağlantı çıktısı değil; süreç içi
taşımalar da görmez). §3, §15, §18.4'e yazıldı.

**Test.** `o1_gap_is_reported_to_the_host` (`seq` 7 → `Gap {px, expected 1, got 7}`,
çerçeve biçimi bayt bayt; bağ `stale`; `stale` iken sonraki olayda bildirim yok;
`refresh_bindings` → `ready`; `handle_host_frame` aynı bildirimi döner).
`crates/qp-cli/tests/protocol.rs::serve_reports_a_gap_to_the_host`: gerçek `qp serve
--host-frames` süreci ilk satırda `{"host":{"gap":…}}` yazar, sonra `batch_end`; `binding`
sorgusu `stale`. Döküm 34 adım 12–13.

### O3 (orta) — veri değişmeden `changed`

**Düzeltme** ("sayıyı motor söyler", §18.5). `add_fact` hesap serisinde istemcinin
`evidence.value` / `evidence.prev`'ini motorun ana çıktısıyla (olgunun barında / bir önceki
barda) bit düzeyinde karşılaştırır (`expand::check_evidence_values`); farklıysa `invalid`
`/ops/i/evidence/value` (ya da `prev`), `expected`: motorun değeri (en kısa geri dönüşlü
yazım); motorun değeri yoksa (ısınma, bar 0'da `prev`) `invalid`. Bağlı seride okuma
dokümanı (komutun başındaki hali), belge içi seride doküman kullanılır; aynı komutta bağlı
seri üstüne konan hesap serisi denetlenmez (belgelendi). Diğer anahtarlar ve bar
serisindeki olgular denetlenmez.

**Döküm etkisi: yok.** Değer taşıyan iki döküm (30: `value 36.65288105373857`; 32: `value
48.41675196259`, `prev 42.43789494505598`) zaten motorun bitlerini yazıyor; kural uygulandı,
33 döküm değişmeden geçiyor. Bayrak ya da gevşetme gerekmedi.

**Test.** `o3_evidence_values_are_the_engines` (yanlış `value` → `invalid` + `expected`
motorun değeri; `prev + 1e-9` → `invalid` `/ops/0/evidence/prev`; bar 0 → `invalid`; doğru
bitler + `level` → `applied`; bar serisinde `value` denetlenmez) ve
`o3_inline_study_checked_too` (veri düzlemi olmayan belge içi doküman).

### D2 — bağlı olgunun `value` kökeni

**Düzeltme.** Katman sabitinin `fact`/`value` kökeni, olgunun serisi bağlı bir seriye
dayanıyorsa değeri okuma dokümanından hesaplar, yazanın yetkisiyle süzer
(`canon::bound_fact_value`; yetkisiz → `no_entitlement` `/…/origin/<k>/fact`). Bunun için
`canon::Site`'a ve `canon_view`/`new_layer`'a `reads: Option<Reads>` (okuyucu + yazan)
eklendi (`Ctx::reads()`). Yükleme sırasında (`canon_document`, veri yok) bağlı olgunun
değeri bilinemez: yazılı sabit korunur (`OriginValue::Unknown`), sabit yoksa `invalid`.
Böylece kaydedilip yeniden yüklenen doküman geçersiz olmaz.

**Test.** `d2_value_origin_of_a_bound_fact`: owner `rsi14` bar 100 olgusuna `value` kökenli
`rule` ekler → `applied`, sabit `study` sorgusunun değeriyle bit düzeyinde aynı; yetkisiz
`editor` aynı katmanı ekleyemez (`no_entitlement`); yeniden oynatma eşit.

### D3 — `check_bind` oturumun sınırını kullanmalı

**Düzeltme.** `check_bind(…, max_bind_bars)` artık `ctx.plane.max_bind_bars`'ı (oturumun
`limits.max_bind_bars`'ından kurulur) kullanır, `default_limits()`'i değil. Bugün sınırlar
yapılandırılamadığı için ayrı bir test yok (ikisi aynı değer); var olan
`bound_series_validation` testi sınırı denetlemeye devam ediyor.

### D4 — seçici karalama kopyası bağlı barları kopyalıyordu

**Düzeltme.** Seçicili komutun seçici çözümü ve karalama kopyası okuma dokümanından değil
dokümanın kendisinden (`self.doc`) yapılır: seçiciler bar okumaz (§17), `add_fact` barları
`ctx.data` üzerinden okur. Bağlı dokümanda karalama kopyası artık bar içermez (T9'un
"tek kopya, yerinde geri alma" yaklaşımı aynen duruyor). Sorgulardaki seçiciler kopya
yapmadığı için okuma dokümanında kaldı.

**Test.** `d4_selector_command_on_a_bound_document`: bağlı dokümanda iki işlemli komut
(`add_fact` + seçiciyle o olguyu alan `add_finding`), kimlikli komutla aynı doküman,
yeniden oynatma eşit. `selector_copies` testleri değişmeden geçiyor.

### D1, D5 — yapılmadı

- D1 (T4.md'ye ölçüm aralığı ve makine yükü notu): T4.md bu görevin yazma kapsamında
  değil; öneri olarak kalıyor.
- D5 (iki depoda kısmi uygulama): D-T4-1 (besleme başına tek depo) ile çözülecek; bu
  görevde dokunulmadı.

## Değişmezler

- Artımlı canlı yol korunuyor: `tests/live.rs` özellik testi (artımlı = baştan, bayt
  düzeyinde) yeşil; yeni koşul süzgeci iki yolda aynı yerde (değerlendirme öncesi).
- Yeniden oynatma eşitliği her yeni testte denetleniyor.
- 33 eski döküm, altın görseller, hesap özeti değişmedi. Döküm 34 eklendi (üç taşımada
  geçiyor).

## Doğrulama

`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `QP_VERIFY_BROWSER=0 scripts/verify.sh` — sonuçlar commit
iletisinde ve alt ajan raporunda.

## Öneriler

- Y1 kalan sızıntı için Arkin kararı (§18.3 açık soru; a/b/c).
- `Session::data_event`'i ileride kaldırıp yalnız `host_data_event`'i bırakmak (barındırıcı
  bildirimi gözden kaçırmasın); bugün T5/T8 ve MCP koşucusuyla çatışmamak için korundu.
- Yeniden çözümü otomatikleştirmek: `Gap` bildirimi geldiğinde barındırıcının
  `refresh_bindings`'i çağırması için örnek barındırıcı (`qp serve`'de bayrak?) — kapsam dışı.
