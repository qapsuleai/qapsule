# RF1/S3.1 bağımsız denetim: `rf1/s31-watch` @ 6aab60c6 (`main` c0c73912 üstünde)

**Hüküm: GEÇTİ.** Davranış `main` ile aynı: charac 25/25 ve 11/11, kitaplar aynı, `compare.sh`
270/270. İzleme `qp-watch`'a taşındı ve mantığı aynı kaldı. Politika kolları eski sabit kollarla
birebir. Kayıt listesi ürün kodundaki her kurulum yoluna ulaşıyor. Kayıt satırı çıkarılınca otorite
`src/`'si değişmiyor ve izleme kapanıyor. B1–B6, B8 ve B9 kapandı. Aşağıdaki bulguların hiçbiri
ürün davranışını değiştirmiyor.

Uyarılar:
- **WASM kabuk paketi kapıya çok yakın:** tabana göre **+%1,90**, sınır ±%2.
- **Ek noktası sınaması (E) için bir ifade düzeltmesi gerekiyor** (B-a).

Denetçi kendi worktree'sinde çalıştı (`.claude/worktrees/audit-s31`, `--detach`). Derleme klasörü
`target/wt/audit-s31` idi. Dal değiştirilmedi, birleştirilmedi. Geçici sınama dosyası ve kayıt
satırı silme denemesi yalnız denetim worktree'sinde yapıldı ve geri alındı (`git status` temiz).

## Sonuçlar

| # | Adım | Sonuç |
|---|---|---|
| 1 | `cargo fmt --all -- --check` | temiz |
| 1 | `cargo clippy --workspace --all-targets -- -D warnings` | temiz |
| 1 | `cargo test --workspace` | 206 ikili, 1235 geçti, 0 kaldı, 9 yok sayıldı |
| 1 | S1 charac (`qp-session --test charac`) / G1 charac (`qp-designs --test charac`) | 25/25 / 11/11 |
| 1 | Kitaplar (`qp-session/tests/charac/expected`, `qp-designs/tests/charac/expected`) | `git diff main HEAD` boş; koşulardan sonra `git status` temiz |
| 1 | Yeni sınamalar (`new-tests.txt` S3.1) | 5'i de koştu ve geçti; `max_watches_binds_registry` `qp-watch` içinde geçti |
| 2 | `scripts/refactor/compare.sh` | `270 items byte-identical to the baseline` |
| 3 | Kayıt yayılımı | eksiksiz; aşağıda |
| 4 | İzleme taşıması | aynı mantık, aynı sıra; aşağıda |
| 5 | Ek noktası önizlemesi | otorite `src/` dokunulmadı; yazma yetkili için `unsupported`, izleyici için `forbidden` (B-a) |
| 6 | audit-s30 B1–B6 | kapandı; sınamaların ikisi zayıf (B-b, B-c) |
| 7 | `scripts/arch-check` | 0 hata; 2 adlı istisna; İ3 `qp-authority` **20.129 = tavan 20.129** |
| 7 | `scripts/arch-check --config-diff main` | 10 değişiklik, **0 atıfsız gevşeme**. Hepsi sıkılaşma: İ3 20.437→20.129, 4 `file.soft`/`fn.lines` kalemi, `expand_watch` silindi, `lint.allow` `qp-authority` 48→47 ve `qp-ops` 21→20. `qp-watch` `oturum` katmanına girdi |
| 7 | `scripts/arch-check --self-test` | PASS (B7 vakası dahil) |
| 7 | `cargo tree` | `qp-authority`'nin ağacında `qp-watch` yok (normal, build, dev). `qp-watch`'ın ağacında `qp-authority` yok. `qp-watch` → `qp-capsule`, `qp-ops`, `qp-plane`, `qp-schema`, `qp-subsystem` |
| 7 | Katman sırası | `qp-plane < qp-ops < qp-subsystem < qp-watch < qp-authority < qp-session` |
| 8 | WASM | aşağıda |

## 3. Kayıt yayılımı: her kurulum yolu

**Yapısal kanıt.** Çalışma alanında `qp-authority`'ye bağımlı tek crate `qp-session`. Bu,
`Cargo.toml` taramasından ve `qp_authority` adının `.rs` dosyalarındaki geçişlerinden görülüyor.
Bu yüzden `qp-session` dışındaki kod otoriteyi ancak `qp_session` yüzünden kurabilir. Bu yüzdeki
`Session`, `ReplayHost`, `Replica`, `viewer::OpenSide` ve `tape::{run, run_with, run_viewer}`
bileşimin sarıcılarıdır ve hepsi `STANDARD` verir.

Glob yeniden dışa aktarımlarda (`replica::*`, `viewer::*`, `replayhost::*`, `tape::*`) açık
`pub use crate::X` ve yerel `fn` gölgeliyor. `qp-shellcore`'daki `OpenSide::trust` →
`|o| o.trust()` değişikliği, sarıcı tipin gerçekten seçildiğini gösteriyor.

`qp_session::Authority` ve `qp_session::Subsystems::NONE` ile boş listeli kurulum ürün kodunda
**yok**. `Subsystems::NONE`'u yalnız `qp-authority`'nin kendi sınamaları kullanıyor.

**Çağrı yerleri.** Hepsi `STANDARD` alıyor:

| Yer | Kurulum | Liste |
|---|---|---|
| `qp-cli` `cmd/serve.rs:12`, `cmd/scene.rs:24` | `qp_session::Session::new` | STANDARD |
| `qp-cli` `main.rs:426`, `cmd/replay.rs:46` | `qp_session::ReplayHost::with_local_dir` | STANDARD |
| `qp-cli` `cmd/replay.rs:210/241` | `qp_session::tape::{run, run_viewer}` | STANDARD |
| `qp-host` `mcp_serve.rs:294/296` | `ReplayHost::with_local_dir` / `Session::new` | STANDARD |
| `qp-host` `demo/setup.rs:33` (`qp demo`) | `ReplayHost::with_window` | STANDARD |
| `qp-host` `livehost/rest.rs:438` `host_from_history` (`qp demo --live`; **`qp sim`**: `sim/pace.rs:296` bunu çağırır) | `ReplayHost::live` (`use qp_session::ReplayHost`, `livehost.rs:50`) | STANDARD |
| `qp-mcp` `conform.rs:384/388`, `hub.rs` (`&Authority` yalnız ödünç) | `conform::transcript_replay_host` / `Session::new` | STANDARD |
| `qp-session` `conform.rs:193/562/579` | `crate::{ReplayHost, Session}` | STANDARD |
| `qp-shellcore` `drive.rs:269/488/504`, `shell.rs:290`, `viewer.rs:282` | `Replica::with_memory`, `ReplayHost::with_local_dir`, `OpenSide::new` | STANDARD |
| `qp-wasm` `bind/replica.rs:30` (WASM bağlaması), `examples/startup.rs` | `qp_session::Replica::with_memory`, `ReplayHost::with_window` | STANDARD |
| `qp-session` `examples/*` (17), `tests/*`; `qp-mcp`, `qp-render`, `qp-wasm` sınamaları | `qp_session::…` | STANDARD |

**Otoritenin doğurduğu otoriteler.** Hepsi listeyi ebeveyninden alıyor. Elle okundu, sonra geçici
bir sınamayla gözlendi:

| Yol | Kod | Gözlenen |
|---|---|---|
| `ReplayHost::rebuild` (geri sarma) | `control.rs:265` `Session::new(init, root, self.session.subsystems())` | yeniden kurulumdan sonra `["watch"]` |
| `ReplayHost::{with_window, live}` | `setup.rs:242/308` `subs` | `["watch"]` |
| `Replica` izleyicisi | `follow.rs:133` `follower(…, self.subs)` → `Session::build_with(…, subs)` | `Some(["watch"])` |
| `OpenSide` | `open.rs:260` `Replica::with_memory(…, subs)` | kodla |
| `tape` | `tape.rs:278/279/479/530/549/565`; `viewer_deliver(…, subs)` | kodla; tape charac geçti |
| `Session::*` kurucuları | hepsi `build_with_keys(…, subs)`. `load` kaydı `start_capsules`'tan ve ilk çözümlemeden önce yapar | `["watch"]` |

Otorite içinde liste almadan otorite kuran başka yol yok. Tarananlar: `Session::new`, `with_*`,
`build*`, `Replica::*`, `OpenSide::new`, `ReplayHost::*`, ayrıca `self.session =` atamaları.

## 4. İzleme taşıması

**Eşleme** (`main:capsule/watch.rs` ile `qp-watch/src/matching.rs`, satır satır):

- `Matched`, `negative`, `holds`, `matches` aynen taşındı.
- `match_watches` aynı. Yalnız iki fark var:
  - `self.doc` / `self.clock` yerine `view.document()` / `view.clock()`. Bunlar `&self.doc` ve
    `self.clock`; `in_world` takası doküman alanını değiştirir, saati değiştirmez. Eski kodda da
    böyleydi.
  - `expect("on")` yerine `let … else continue`. `on(i)` `watch.is_some()`'ı zaten şart koşuyor,
    bu yüzden fark erişilemez.
- `watch_lines` için denetlenenler:
  - Bağlantı sırası: `conn_ids()` bağlantıları `self.conns` sırasıyla verir.
  - Abonelik sırası: `subs(cid)`, `c.subs` sırası.
  - Kamu görünümü: eski ifade `self.other.is_some() && public_view(..)`. Yeni `view.public_view`.
    `Session::public_view` `other.is_none()` ise zaten `false` döndüğü için eşdeğer.
  - Kişi: `person_of` ile `delegates.get(..).unwrap_or(principal)` aynı.
  - İzdüşüm ve olay alanları aynı.

**Sıra** (`capsule/run.rs`, `main` ile fark):

- Değişimler: `capsule_changes` → `look_phase(&events, false)`. Ardından `in_world(true)` içinde
  kamu `capsule_changes` → `look_phase(&events_pub, true)`.
- Satırlar: `entry_events` → `capsule_topic_lines` → `written_phase(seq)`. Eski sıra da buydu.
- `Pending` tekli yuvada (`one`). `swap_world` yalnız `PerWorld` yuvalarını takas ettiğinden iki
  dünyanın bakışları aynı yuvaya yazar: `full`, sonra `public`.
- `written_phase` eşleme yokken de `pub_of(seq)` çağırıyor. Bu çağrı saf ve gözlenemez.
- Girdi yazılmadığında (`engine_entry_with` → `None`) eski kod eşlemeleri atıyordu. Yeni kodda
  `Pending` bir sonraki `Look{public:false}` ile eziliyor ve gözlenebilir fark yok.
- `watch-cross`, `order-data`, `order-data-full`, `ops-watch` ve `worlds-flow` kitapları geçti.

**Açılım** (`main:qp-ops/src/watch.rs` `expand_watch` ile `qp-watch/src/ops.rs`): denetim sırası,
hata kodları ve iletileri, `new_id` ve `prov` aynı. `unreachable!` yerine `invalid` hatası geldi;
kayıt yalnız `owns` olan işlemi verdiği için bu yola erişilemez.

**Politika** (eski sabit kollarla tek tek):

| Kalem | `main` | `qp-watch` `WatchOp` | Eşit |
|---|---|---|---|
| `allowed` | `AddWatch \| RemoveWatch => perms.read` | `perms.read` | ✓ |
| `proposable` | yasak listesinde → `false` | `false` | ✓ |
| `agent` | `AddComment \| AddWatch \| RemoveWatch => Direct` | `Direct`; `qp_session::agent::agent_op` önce `STANDARD` işleyicisine sorar, otoritenin `agent_check`'i kayda sorar | ✓ |
| `to_public` | `other => Some(other.clone())` | alan alan kopya. Örüntülerde `..` yok, bu yüzden derleyici her alanın kopyalandığını garanti eder; `Clone` türetmesiyle aynı değer | ✓ |
| `reader_private` | `matches!(AddWatch\|RemoveWatch) \|\| reg.op(op).reader_private()` | `reg.op(op).reader_private()` → `true` | ✓ |
| `owns` | (tel adı) | `matches!(op, Op::AddWatch{..})` / `RemoveWatch` | ✓ |

## 5. Ek noktası önizlemesi

`compose.rs`'teki `&qp_watch::WATCH,` satırı geçici olarak silindi. Sonuçlar:

- `git diff` yalnız `crates/qp-session/src/compose.rs | 1 -` gösterdi. `git diff --quiet HEAD --
  crates/qp-authority` temiz çıktı.
- Geçici sınama, `qp_session::Session` (fikstür `agent_watch.rs`'in init'i, ajan politikası
  yok):

  | Kişi | Satır varken | Satır yokken |
  |---|---|---|
  | `host` (yazma yetkisi var) | `applied` | **`unsupported`**: "`add_watch` is not served here: no subsystem owns it" |
  | `ana` (`viewer`) | `applied` | **`forbidden`**: "role `viewer` may not `add_watch`" (B-a) |

- Yayılım gözlemi satır yokken: `subsystems()` oturumda, barındırıcıda, geri sarmadan sonra ve
  yansıda `[]` döndü. Satır varken dördü de `["watch"]` döndü.
- `agent_watch` 5/5 kaldı (ajan: `forbidden`, `agent_cannot_write`). Charac 8/25 kaldı:
  `watch_cross_order`, `ops_watch`, `order_data`, `order_data_full`, `worlds_flow`,
  `transcripts_surface`, `agent_marked_connection`, `agent_policy_r5b`. Kitaplar yeniden
  yazılmadı.
- Satır geri gelince charac 25/25, `agent_watch` 5/5. `git status` temiz.

## 6. audit-s30 düzeltmeleri

| Bulgu | Durum | Sınama |
|---|---|---|
| B1 `eval_memo` kaydı atlıyor | kapandı: `reg.evaluates_conditions()` doğruysa bellek atlanıyor (`data/live.rs`) | `memo_registry::a_registered_condition_kind_is_asked_past_a_fresh_memo`: taze bellekle iki çağrının ikisi de kayda gidiyor. Birim sınaması `eval_memo`'yu doğrudan çağırıyor; otorite üstünden canlı yol sınaması yok (audit-s30 bunu istemişti) |
| B2 uyandırma temizlenmiyor | kapandı: `advance_clock`'ta `now >= wake` → `None`, faz çağrısından önce | `a_wake_the_clock_reached_is_spent`, otorite üstünden (`host_clock`) |
| B3 derinlik | kapandı: `MAX_EMIT_DEPTH = 8`; sıra derinlik öncelikli ve belgelendi (`outbox.rs`) | `requested_entries_stop_at_the_emit_depth`: komut + 8 yankı, sıra `echo n+1`, `echo n+2`. Sınırı aşan istekler **sessizce** düşüyor (B-d) |
| B4 ölü kalemler | kısmen: `Close` ve `Moment::Command` belgelendi; `drop_conn`, `RunKind`, `param`, `replay_position` aynı (kullanım round'una bağlı) | — |
| B5 `dispute_guard` | kapandı: biçimsiz özet → `invalid`; görülmüş → `duplicate`; bağlantı ya da iddia sınırı → `limited`; hüküm `seen`'e yazılıyor. `MAX_OPEN` uygulanmıyor: kayıtlı tür eşzamanlı hüküm verir, açık itiraz oluşmaz | `a_registered_claim_kind_keeps_the_dispute_guards`: `invalid` ve `duplicate` sınanıyor, **`limited` sınanmıyor** (B-c) |
| B5 `viewer_unsubscribed` | kapandı: konu reddinden sonra çağrılıyor | `a_refused_subscription_leaves_nothing_behind` **boş iddia**: reddedilen konu `changes` olduğu için görüntüleyici hiç kaydedilmiyor; `viewer_data("a").is_none()` düzeltme olmasa da doğru (B-b) |
| B6 kopya gövde | kapandı: tek gövde `qp_ops::expand::expand_ops_with`; `expand_ops` ve `Registry::expand_ops` onu kullanıyor | charac, compare |
| B8 İ3 tavanı | kapandı: 20.437 → 20.129 = ölçülen | arch-check |
| B9 README | kapandı: bağımlılıklar ve düzen doğru | — |

B7 `main`'de c0c73912 ile kapanmıştı. `--self-test`'teki B7 vakası geçiyor.

## 7. Yapı

- İ3: ölçülen 20.129, tavan 20.129.
- Çekirdekteki net değişim **−25 satır** (S3.0 denetiminde 20.154). `capsule/watch.rs` (≈ 200
  satır) çıktı. Yerine `Subsystems`, `look_phase`/`written_phase`, `dispute_guard`, `subs`
  parametreleri, B1–B3 ve yeni `SessionView` yöntemleri girdi. 13.000 sınırına uzaklık değişmedi.
  Beklenen bir durum, çünkü S3.1'in payı küçüktü.
- Otoritede izlemeye dair kalanlar bilinçli. Bayt eşitliği bunları gerektiriyor ya da bunlar
  dokümanın izdüşümüne ait:
  - `limits.max_watches` (welcome, `create.rs`);
  - `intent_for_reader` (dokümanın izdüşümü);
  - ajan ret ipucundaki "watches (`add_watch`)" metni (`agent.rs:210`) ve modül belgesi
    (`agent.rs:12`).
  - `qp-ops` `watch.rs` (`on`, `person_of`, `watch_visible`, `MAX_WATCHES`) ortak saf kısım
    olarak kaldı.

## 8. WASM

Aynı derleme klasörüyle (`target/wt/audit-s31`). Kabuk paketi `perf.sh` komutuyla ölçüldü; çıktı
geçici bir klasöre yazıldı.

| Ölçü | Taban | HEAD 6aab60c6 | Fark | `main` c0c73912 (denetçi) | S3.1'in kendi payı |
|---|---|---|---|---|---|
| varsayılan ham | 2.345.821 | 2.354.884 | **+%0,39** | 2.354.892 | −8 B |
| varsayılan gzip -9 | 769.836 | 773.647 | **+%0,50** | 773.655 | −8 B |
| kabuk paketi ham | 8.789.414 | 8.956.356 | **+%1,90** | 8.913.176 | +43.180 B (+%0,48) |
| kabuk paketi gzip -9 | 2.432.388 | 2.474.582 | **+%1,73** | 2.459.941 | +14.641 B (+%0,60) |

İkisi de ±%2 içinde. Ancak kabuk paketinin payı ham boyutta yalnız **%0,10** (≈ 8,8 KB) kaldı.
S3.2 (`qp-read`) ve S3.3 (yansı, görüntüleyici) bu payı aşacaktır. Kayıt tablolarının vektöre
çevrilmesi ve `to_public`'in alan alan kopyalanması zaten bu kapı için yapıldı.

## Bulgular

**B-a (orta; plan §7 sınaması E(a) için): kayıtsız işlem yetkiden önce reddediliyor.**
- **Ne oluyor:** Satır çıkınca `add_watch` yazma yetkisi olana `unsupported` döndürüyor. İzleyiciye
  `forbidden` ("role `viewer` may not `add_watch`") dönüyor, ajana `forbidden`/`agent_cannot_write`.
- **Neden:** `allowed()` kayıtta karşılık bulamayınca `_ => perms.write` koluna düşüyor. Ajan
  politikası da açılımdan önce koşuyor.
- **Etki:** Ürün yolunda yok, çünkü bileşim her zaman `STANDARD` ile kuruyor. Ama E(a) "`add_watch`
  bilinmeyen işlem hatası verir" diyor; sınama izleyiciyle yazılırsa yanlış hatayı kilitler, ileti
  de yanıltıcı olur.
- **Öneri:** E(a) yazma yetkili bir kişiyle (`host`/`owner`) yazılsın ve `unsupported` beklesin.
  Ya da `allowed`/`agent_check` kayıtsız bir alt sistem işlemi için önce `unsupported` dönsün.
  Bunun için hangi işlemlerin "alt sistem işlemi" olduğunu otoritenin bilmesi gerekir.

**B-b (düşük): B5 abonelik sınaması boş iddia.** `a_refused_subscription_leaves_nothing_behind`
`changes` konusunu reddediyor. `viewer_unsubscribed` yalnız `viewer` konusunda iş görür, bu yüzden
sınama düzeltme olmadan da geçer. Kayıtlı bir `viewer` konu işleyicisinin reddiyle yazılırsa
gerçekten sınar; bu S3.3'ün işi.

**B-c (düşük): `dispute_guard`'ın `limited` kolu sınanmıyor.** `MAX_PER_CONN` ve `MAX_DIGESTS` yolu
kapsanmıyor. `MAX_OPEN`'ın bilerek dışarıda bırakıldığı kodda yazılı değil.

**B-d (düşük): derinlik sınırında sessiz düşme.** `MAX_EMIT_DEPTH`'i aşan motor girdisi istekleri
iz bırakmadan atılıyor. Belgede yazıyor, ama hata ayıklamak için bir sayaç ya da günlük notu
olmalı. Belirlenimciliği bozmaz.

**B-e (düşük; gelecek): `Outbox::amend` / `say` yanlış anda sessizce düşüyor.** Bir alt sistem
`Look` ya da `Written` dışındaki bir anda `amend` ya da `say` kullanırsa istek emit adımında
düşüyor (belgede yazıyor). Daha kötüsü: bir `amend` emit'ten önce gelen bir `Look`'a denk gelirse
o bakışın girdisine karışır. Bugün tek kullanıcı `qp-watch` ve yalnız doğru anlarda yazıyor. Öneri:
`debug_assert` ya da anı bilen bir `amend`.

**B-f (düşük): kapı açık.** `qp_session` `Authority` ve `Subsystems`'i (dolayısıyla
`Subsystems::NONE` ve `Default`) dışa açıyor. Ürün kodunda kullanılmıyor, ama gelecekte boş listeli
kurulumu engelleyen bir şey yok. Öneri:
- `Subsystems`'ten `Default` kaldırılsın;
- `qp-session` dışında `Subsystems::NONE`'u yasaklayan bir arch-check kuralı ya da sınama eklensin.

**B-g (not): "bir kayıt satırı" iki satır.** Yeni alt sistem `compose.rs`'te bir satıra ek olarak
`qp-session/Cargo.toml`'da bir bağımlılık satırı da istiyor. Plan §7.5'in ifadesi buna göre
güncellenebilir.

**B-h (not): ad gölgeleme.** `tape.rs` `run_viewer` içinde `let subs = a.viewer_data(..)` parametre
`subs: Subsystems`'i gölgeliyor. Tipler farklı olduğu için derleyici yanlış kullanımı yakalıyor,
ama okunması zor. `feeds` gibi bir ad önerilir.

## Öneriler (kapsam dışı; yapılmadı)

1. E sınamasını yazma yetkili kişiyle ve `unsupported` beklentisiyle yazın, ya da kayıtsız alt
   sistem işlemini yetkiden önce reddedin (B-a).
2. S3.3'te B5 abonelik sınamasını kayıtlı bir `viewer` konusuyla yeniden yazın; `limited` kolunu
   sınayın (B-b, B-c).
3. WASM kabuk paketinde ham boyutta yalnız %0,10 pay kaldı. S3.2'den önce ya kabuk paketinin
   kendi büyümesine bakın ya da kapının tabanını yeniden konuşun. Bu bir karar konusu.
4. `Subsystems::NONE`'u `qp-session` dışında kapatın (B-f).
5. B4 kalanları (`drop_conn`, `RunKind`, `param`, `replay_position`) kullanım round'una bağlı kalsın.

## Disk

- **Silinenler:**
  - `.claude/worktrees/audit-s31` ve `.claude/worktrees/audit-s31-main` (yalnız `main`'in WASM
    ölçümü için açıldı; `git worktree remove` ile kaldırıldı);
  - `target/wt/audit-s31` (debug, wasm32, `compare.sh` ağacı);
  - scratchpad'deki `audit-s31/` (geçici sınama, iki `pkg`, dökümler), `fmt.txt`, `clippy.txt`,
    `test.txt`.
- **Bilerek bırakılanlar:** oturum scratchpad'inde denetçinin yazmadığı dosyalar (`e*.py`,
  `final*.log`, `moved.txt`, `wasm.sh` …). Başka bir iş tarafından yazıldılar; dokunulmadı.
