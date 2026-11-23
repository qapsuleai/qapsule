# RF1/S1 bağımsız denetim: `rf1/s1-charac` @ 452c5ff6

**Hüküm: KALDI** (mekanik adımların hepsi geçti; duyarlılıkta dört boşluk çıktı, biri test
kusuru). Düzeltmeler yalnız `tests/charac` ve kitaplarla sınırlı ve küçük; düzeltme sonrası
yalnız §3'ün kırmızı kalması gereken satırları ile §2 yeniden koşulmalı.

Denetçi kendi worktree'sinde (`.claude/worktrees/audit-s1`, ayrık HEAD) ve kendi derleme
klasöründe (`target/wt/audit-s1`) çalıştı. Denetlenen dala yazılmadı, birleştirme yapılmadı.
Bütün değişiklikler geçiciydi ve her denemeden sonra geri alındı (`git status` temiz).

## 1. Fark kapsamı — GEÇTİ

`git diff main...rf1/s1-charac --stat`: 34 dosya, +5465 / −0, beş commit
(3342f55b … 452c5ff6), taban `main` (6447b5c0).

- `crates/qp-session/tests/charac/{main,rig,snap,opcase,ops_doc,ops_flow,order,agent,worlds,replica,transcripts}.rs`
- `crates/qp-session/tests/charac/expected/*.txt` (22 kitap, 2573 satır, 520 KB)
- `rounds/RF1/experiments/new-tests.txt` (+21 ad)

`crates/*/src` altında ve `Cargo.toml`'da değişiklik yok.

## 2. fmt, clippy, test — GEÇTİ

- `cargo fmt --all -- --check`: temiz.
- `cargo clippy -p qp-session --all-targets -- -D warnings`: temiz.
- `cargo test -p qp-session` iki kez: ikisinde de bütün test ikilileri yeşil (`charac` 21/21,
  ≈ 9 s). Her koşudan sonra `git status --porcelain` boş: kitaplar değişmedi.
- 38 `Op` türünün her biri için en az bir başarı ve bir hata vakası var.

## 3. Duyarlılık — boşluklar var

Her değişiklik tek başına uygulandı, `cargo test -p qp-session --test charac` koşuldu, geri
alındı. İstenen altı türün (a–f) her birinde en az bir kırmızı var; ama aynı türden ikinci bir
olası hata dört yerde yeşil kaldı.

| # | Değişiklik (yeniden yazımda olası hata) | Sonuç | Kırılan test |
|---|---|---|---|
| a1 | `settle` (komut yolu): `conds` ↔ `intents` sırası | **KIRMIZI** | `order_commands_full` (`cmd.58.bar_low`) |
| a1b | `settle_data` (artımsız veri yolu): `conds` ↔ `intents` sırası | **YEŞİL** | — |
| a2 | `entry_events_in`: `finding_state` grubu `change`'ten önce | **KIRMIZI** | `order_commands`, `order_commands_full`, `transcripts_data`, `worlds_flow` |
| b1 | `add_comment`: `no series of instrument` yolu `/about/instrument` → `/about` | **YEŞİL** | — |
| b2 | `update_finding`: ileti `nothing to update` → `nothing to change` | **KIRMIZI** | `ops_findings` |
| c | `ops_to_pub`: yetkili yazarın `add_watch`'ı kamu dünyasına girmez | **KIRMIZI** | `worlds_flow` (`w.45…`) |
| d1 | `project_changes`: gizlenen olgu `evidence_ref`'i tutar | **KIRMIZI** | `worlds_flow`, `worlds_single` (`q_log_c`, `q_log_C`) |
| d2 | `project_changes`: gizli kapsülün `SetCapsuleState` kolu silinir | **YEŞİL** | — |
| e | `viewer_flush`: bağlantılar ters sırada | **KIRMIZI** | `viewer_two_readers` (`v.12.step9`) |
| f1 | `watch_visible`: ajan, kişisinin izlemesini görmez | **KIRMIZI** | `ops_watch`, `order_data`, `order_data_full`, `transcripts_surface` |
| f2 | `intent_for_reader`: ajan okuyucu kişisinin izlemesini almaz | **KIRMIZI** | f1'dekiler + `worlds_flow` |
| f3 | `match_watches`: döngü sırası (önce izleme, sonra olay) | **YEŞİL** | — |

### Bulgular ve öneriler

**B1 — test kusuru: `add_comment` hata vakalarından üçü adının söylediği kolu sınamıyor.**
`add_comment.err.both`, `.capsule`, `.instrument` `cites: []` ile yazılmış; yorum doğrulayıcı
önce reddediyor ve üçü de `no_cites` ile biter. Kitapta dördünün özeti aynı
(`2e6815fca8eebdd8`). `about` kolunun üç hatası (ikisi birden, olmayan kapsül, olmayan
enstrüman) kilitsiz. S4 bu kolu ayrı fonksiyona taşıyacak.
*Öneri:* üç vakada `cites: ["fnd-overbought"]` verin. Denendi: üçü de amaçlanan hataya ulaşıyor
(`invalid@/ops/0/about`, `not_found@/ops/0/about/capsule` + `did_you_mean`,
`not_found@/ops/0/about/instrument`), b1 değişikliği o zaman kırmızı olur. Ayrıca bir
kitap içinde aynı özetli iki `.err` vakası bulunursa uyaran küçük bir öz-sınama eklenebilir.
Öbür beş aynı özetli çift (`approve/show_finding.err.missing`, `delete_layer/set_encoding`,
`insert/move_panel.err.after`, `remove_layer/set_view_range.err.view`,
`undo.err.nothing/after_undo`) ortak yardımcılardan geliyor; bunlar meşru.

**B2 — artımsız veri yolunda `condition_met`/`intent_triggered` sırası kilitsiz (a1b).**
Komut yolunda ikisini birlikte üreten tek vaka var (`cmd.58.bar_low`, yalnız `_full`
kitabında; artımlı `settle_live` o vakada bu sırayı üretmiyor). `settle_data`'da ikisini aynı
veri olayında üreten vaka yok. S3.0 bu üç yolu (settle, settle_data, settle_live) tek faz
düzeneğine bağlayacak.
*Öneri:* `order_data` / `order_data_full` senaryosuna, aynı çubukta hem bir `condition`
aboneliğini hem bir `intents` niyetini tetikleyen bir veri olayı ekleyin (ör. `cmd.58`'deki
koşul + niyet çiftinin veri olayıyla tetiklenen biçimi); artımlı kitapta da aynı adım olsun.

**B3 — okuyucu izdüşümünde gizli kapsül durum değişikliği kilitsiz (d2).**
`SetCapsuleState` kolunu silmek hiçbir testi kırmıyor. Denendi: `worlds_single`'a sahibin
`reject_finding` komutunu (CAP'in bulgusu) eklemek de değişikliği yakalamadı. Tek dünyada
CAP kısıtlı okuyuculardan gizli değil; iki dünyada kısıtlı okuyucular kamu dünyasından
(`translate`) besleniyor. Bu yüzden kol bugünkü dökümlerle ulaşılamaz görünüyor.
*Öneri:* S1 yazarı iki yoldan birini seçsin. (i) Yetkili dünyada `hidden` boş olmayan bir
okuyucuyla (otorite dünyasında kalıp kısmen yetkisiz) gizli kapsülün durumunu değiştiren bir
vaka kurun: `approve_finding`/`reject_finding` ya da geri alma; `changes` ve `finding`
aboneliğiyle. Böylece `PushLedger` kolu da kilitlenir. (ii) Kurulamıyorsa bunu
`new-tests.txt`'in yanına "ulaşılamaz/savunma kolu" diye not edin ve S3.2'ye
(`as_reader` taşıması) bırakın; o zaman `public.rs:454` (`translate` içinde
`SetCapsuleState`) için bir duyarlılık denemesi ekleyin.

**B4 — izleme eşleşme sırası kilitsiz (f3).**
`match_watches`'ta döngü sırası değişince `intent_matched` olaylarının sırası değişir. Ama
hiçbir döküm bir motor bakışında iki izlemenin iki olayla eşleştiği durumu üretmiyor. S3.1
(`qp-watch`) bu fonksiyonu taşıyacak.
*Öneri:* `ops_watch` ya da `order_data`'ya iki izleme ekleyin (aynı enstrüman, biri
`max_matches: 1`). Ardından tek çubukta iki kapsül olayı üreten bir veri adımı koşun ve
`intents` abonesinin satır sırası kitaba girsin.

**B5 (küçük) — `snap.rs` farkı sırayı göstermiyor.** Uyuşmazlıkta fark `contains` ile
hesaplanıyor. Yalnız sırası değişen satırlarda test kırmızı olur ama yazdırılan fark boş
kalır. Kitabın tamamı `$CARGO_TARGET_TMPDIR/charac/` altına yazıldığı için engelleyici
değil. Satır numaralı ilk farkı yazdırmak yeter.

## 4. Yol bağımsızlığı — GEÇTİ (bir not ile)

Testler yalnız kamu yüzeyini kullanıyor; çağrıların çoğu `rig.rs`'te toplanmış.

| Kullanılan öğe | Bölmede nereye gider | Gereken düzeltme |
|---|---|---|
| `qp_session::Session` (`new`, `handle_raw`, `data_event`, `log`, `document`, `initial_document`) | `qp-session` (bileşim; `Deref` → `qp-authority`) | yok |
| `Session::set_incremental`, `mark_agent_connection`, `is_agent_connection`, `agent_policy`, `comment_audit` | `qp-authority` yöntemleri, `Deref`/`DerefMut` ile | yok (yöntem çağrısı) |
| `qp_session::ReplayHost` (`session`, `session_mut`, `handle_raw`, `data_event`) | `qp-replayhost` | `use` (rig.rs) |
| `qp_session::conform::transcript_replay_host` | `qp-replayhost` | yol (rig.rs) |
| `qp_session::tape::{run, run_viewer}` | `qp-replica` | yol (replica.rs) |
| `qp_session::replay` | `qp-ops` | yol (rig.rs) |
| `qp_session::read_json` | `session` modülü → `qp-authority`/`qp-ops` | yol (rig.rs, transcripts.rs) |
| `qp_capsule::trust::seal::sha256`, `qp_schema::{data::DataEvent, protocol::{OutboundLine, SessionInit}}` | değişmez | yok |

`qp-session` bunları yeniden dışa verirse hiçbir düzeltme gerekmez. Vermezse yalnız 6 `use`
ya da yol satırı değişir; davranışa dokunan bir şey yok.

**Not.** `qp-replayhost` ve `qp-replica`, `qp-session`'ın üstünde ya da yanında. Testlerin
onlara ulaşması bir geliştirme bağımlılığı ister. Cargo geliştirme döngüsüne izin verir;
`arch-check` katman kuralının buna izin verip vermediği S3.3'te bakılmalı. Ayrıca kitaplar
`CARGO_MANIFEST_DIR/tests/charac/expected` altından okunuyor. Testler başka bir crate'e
taşınırsa kitaplar `git mv` ile taşınmalı, yeniden yazılmamalı.

## 5. Biçim — GEÇTİ

- En büyük dosya `ops_doc.rs` (688 satır); hepsi ≤ 800.
- 21 test adının hepsi `charac_session_` önekli ve depoda tekil. Adlar `new-tests.txt`'teki
  21 adla bire bir aynı.
- Her `Book::new` adının bir `expected/*.txt` dosyası var, artık dosya yok. Kitap içinde vaka
  adı tekilliği `Book::put`'taki `assert` ile zorlanıyor.
- `QP_CHARAC_BLESS` yalnız tam `"1"` ile yazar (`std::env::var(..).as_deref() == Ok("1")`).
  Kitap dosyası yoksa `unwrap_or_default` boş metin verir ve test kırmızı olur.
- *Öneri (S4 sonrası için):* yeniden yazım dallarında `crates/qp-session/tests/charac/expected/`
  altında bir fark çıkarsa `compare.sh` ya da `verify.sh` başarısız olsun. Böylece "refactor
  asla bless etmez" kuralı yalnız yoruma dayanmaz.

## Disk

Worktree (`.claude/worktrees/audit-s1`) ve `target/wt/audit-s1` kaldırıldı. Scratchpad'deki
mutasyon betiği ve günlükler silindi.

## Yeniden denetim (`rf1/s1-charac` @ e4d308f0)

**Hüküm: GEÇTİ.** B1–B5 kapandı. d2 kolunun ulaşılamazlık gerekçesi kodla doğrulandı.

Yöntem ilk denetimle aynı: kendi worktree'm (`.claude/worktrees/audit-s1b`, ayrık HEAD) ve
kendi derleme klasörüm (`target/wt/audit-s1b`). Denetlenen dala yazılmadı.

**Kapsam.** `452c5ff6..e4d308f0` tek commit, 10 dosya, +388/−6:
- yeni `extra.rs` (4 test);
- `ops_flow.rs` (B1'in 3 vakası);
- `snap.rs` (B5);
- 4 yeni kitap (`order-both`, `order-both-full`, `watch-cross`, `worlds-public-owner`);
- `new-tests.txt` (+4 ad).

`main...e4d308f0` boyunca `crates/*/src` ve `Cargo.toml` değişmedi. Var olan kitaplarda
`git diff 452c5ff6 e4d308f0 -- '*/expected/*'` yalnız `ops-people.txt`'in 3 satırını
değiştiriyor: `add_comment.err.{both,capsule,instrument}`. Üçü artık amaçlanan hataya ulaşıyor:
- `invalid@/ops/0/about`
- `not_found@/ops/0/about/capsule`
- `not_found@/ops/0/about/instrument`

Başka satır değişmedi.

**fmt, clippy, test.**
- fmt temiz; `clippy -p qp-session --all-targets -D warnings` temiz.
- `cargo test -p qp-session` iki kez koştu, iki koşuda da başarısız ikili yok (charac 25/25,
  ≈ 7,6 s).
- Her koşudan sonra `git status` temiz.
- En büyük dosya hâlâ 688 satır.
- 25 test adı tekil ve `new-tests.txt`'teki 25 `charac_session_` adıyla aynı.

**Duyarlılık.** Her değişiklik tek başına uygulandı, test koşuldu, geri alındı.

| # | Değişiklik | Sonuç | Kırılan test |
|---|---|---|---|
| a1b | `settle_data`: `conds` ↔ `intents` | **KIRMIZI** | `order_both_full` (`both.15.bar2`) |
| b1 | `add_comment` instrument hata yolu → `/about` | **KIRMIZI** | `ops_people` (`add_comment.err.instrument`) |
| f3 | `match_watches` döngü sırası | **KIRMIZI** | `watch_cross_order` (`x.22.step18`) |
| t1 | `translate`: `SetCapsuleState` hep düşer (`public.rs:454`) | **KIRMIZI** | `worlds_public_owner` (`po.17.approve`) |
| t2 | `translate`: `PushLedger` hep düşer | **KIRMIZI** | `worlds_public_owner` |
| d2 | `project_changes`: gizli kapsülün `SetCapsuleState` kolu silinir | yeşil (beklenen, aşağıda) | — |

B5 da kapandı: uyuşmazlıkta artık konumuyla ilk farklı satır yazdırılıyor (`want`/`got`).
Yukarıdaki çıktılar bunu gösteriyor.

**d2 ulaşılamazlık gerekçesi — ONAYLANDI.** `project_changes` otorite dünyasında iki yerden
çağrılıyor: `events.rs:350` (değişiklik bildirimi) ve `read/query.rs:337` (`log` sorgusu,
`as_reader` içinde). Gerekçe adım adım:

1. **Kapsül nerede gizlenir.** Bir kapsül `Proj.hidden`'a yalnız `proj_scratch` içinde girer
   (`reader.rs:203-227`). Koşul: hikâyenin baş olgusu lisanslı olacak ve `flow_ok` yanlış
   olacak. `flow_ok` yanlışsa runner o enstrüman için bir akış okuyor (`flow_feed` Some) ve
   okuyucu o akışa yetkili değil. Lisanslı olmayan baş olguda yalnız olgu kimlikleri gizlenir
   (`hidden_here`), kapsül kimliği gizlenmez.
2. **Akış varsa kamu dünyası da vardır.** Runner bir akış okuyorsa `has_flow` doğrudur
   (`capsule.rs:279`). O zaman `start_public` kamu dünyasını kurar (`other` Some,
   `capsule.rs:455` → `public.rs:133`).
3. **Gizli kapsül görebilen okuyucu kamu görünümlüdür.** `public_view(p)` doğrudur, çünkü
   okuyucu akış beslemelerinden en az birine yetkisiz (`public.rs:210-220`). Bu yüzden otorite
   dünyasında bildirim alamaz (`events.rs:332`, `mine`). `as_reader` da onu kamu dünyasına
   geçirir (`project.rs:460`).
4. **Kamu dünyasında da gizli kapsül oluşmaz.** Kamu dünyasının belgesine lisanslı baş olgulu
   kapsül girmez; `translate` gizlileri düşürür.
5. **Otorite dünyasında kalan okuyucu her akışa yetkilidir.** Bu yüzden onun için hiçbir kapsül
   gizlenmez.
6. **Tutarlılık.** Yetki sağlayıcıdan canlı okunur ve iki karar (`hidden`, `public_view`) aynı
   anda aynı yetkiyle verilir. Önbellek `entitlements_hold` ile bozulur.

Sonuç: bu kol (ve aynı nedenle `PushLedger` ile `Delete` gizli kolları) bugün ulaşılamaz bir
savunma kolu. Gerçek koruma `translate` tarafında; t1 ve t2 artık onu kilitliyor.

*Küçük öneri (engellemez):* gerekçe dalda yazılı değil. S3.2'de `as_reader` taşınırken
kaybolmaması için `project_changes`'teki kolun yanına ya da `extra.rs`'teki B3 yorumuna bir
satır eklenebilir. Bu bir `src` değişikliği olacağından S3.2'ye bırakılmalı.

**Disk.** `.claude/worktrees/audit-s1b` ve `target/wt/audit-s1b` silindi.
