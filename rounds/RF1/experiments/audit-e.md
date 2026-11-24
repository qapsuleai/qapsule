# RF1 / E — bağımsız denetim (ek noktası belgesi ve sınaması)

Dal: `rf1/e-extension` @ `ca2b8f1d` (taban `main` @ `a512d2fb`, 5 commit). Denetçi kendi
worktree'sinde (`.claude/worktrees/audit-e`, ayrık HEAD), `CARGO_TARGET_DIR=target/wt/audit-e`.
Denetlenen dala yazılmadı, birleştirme yapılmadı. Mutasyonlar geri alındı (`git status` temiz).

## Hüküm: KALDI

Yapı ve kapılar yeşil; otoritenin `src/`'si ve şema değişmedi. İki açık istenen denetimde
eksik var: (B1) `Tally`'nin `OpHandler`'ının çağrıldığı sınamada gösterilmiyor — kayıt
dağıtımı bozulunca Tally sınamaları yeşil kalıyor; (B2) D2 tablosunun `qp-authority`
listesi eksik — derleyicinin zorladığı iki dosya (`Document` yapı değişmezleri) yok, "1 zorunlu"
gerçekte 3. İkisi de küçük düzeltme; düzeltildikten sonra B1 mutasyonu (aşağıda M1) ve B2 grep'i
yeniden koşulup GEÇTİ verilebilir.

## 1. Kapsam

- `git diff --stat main...rf1/e-extension -- crates/qp-authority/src crates/qp-schema`: **boş**. ✓
- Bütün fark (7 dosya, +568/−3): `crates/qp-authority/tests/contract_fixes.rs` (+35, audit-s31 B-c),
  `crates/qp-session/src/lib.rs` (+3, `AuthorityReplica`), `crates/qp-session/tests/extension/`
  (`main.rs`, `rig.rs`, `tally.rs`), `docs/ARCHITECTURE.md` (+94/−3), `rounds/RF1/experiments/new-tests.txt` (+5).
  Kapsam dışı dosya yok.

## 2. Kapılar

| Adım | Sonuç |
|---|---|
| `cargo fmt --all -- --check` | yeşil |
| `cargo clippy --workspace --all-targets -- -D warnings` | yeşil |
| `cargo test --workspace` | 1240 geçti, 0 kaldı, 9 yok sayıldı; 5 yeni sınama geçti |
| `scripts/refactor/compare.sh` | 270/270 bayt eşit |
| `scripts/arch-check` | 0 hata; 2 adlı istisna; İ3 `qp-authority` 20129 = tavan (büyümedi) |
| `scripts/arch-check --config-diff main` | 0 değişiklik, 0 dayanaksız gevşeme |

## 3. Sınamalar gerçek mi? Mutasyonlar

| # | Mutasyon (geçici, geri alındı) | `ext_tally_counts…` | `ext_tally_a_replica…` | `ext_without_the_watch_line…` | `ext_…no_subsystem_crate` |
|---|---|---|---|---|---|
| M1 | `qp-subsystem` `Registry::op` hep `None` (kayıtlı işlem dağıtımı kırık) | **ok** | **ok** | FAILED | ok |
| M2 | `phases.rs` `emit`: motor girdileri yazılmaz (`entries.take(0)`) | FAILED | FAILED | ok | ok |
| M3 | `qp_ops::apply::replay` yazarı `qp` olan girdileri atlar | FAILED | ok | ok | ok |

- **Motor girdisi (Outbox):** gerçek. Üç `tally n` girdisi, yazar `qp`, `request` boş, dokümanda
  `tally` anlatısı `based_on` 3; M2 kırmızı.
- **Yeniden oynatma:** gerçek. `document_digest` serileştirilmiş dokümanın tamamının FNV-1a'sı
  (`qp-authority/src/replica.rs:70`), özet değil; M3 kırmızı. Yansı sınaması da tam doküman özetiyle
  karşılaştırıyor ve `seq` eşitliğini istiyor.
- **Sorgu ve konu:** `describe` yanıtında `tally: 3` yerleşik yanıtta olmayan bir alan; `proposals`
  konusunda yazarı `qp` olan 3 `change` bildirimi — ikisi de Tally'nin işleyicisinin çalıştığını gösteriyor.
- **B1 — işlem işleyicisi kanıtlanmıyor (orta).** `Tally`'nin `OpHandler::expand`'i yerleşik
  `expand_op`'a devrediyor; sayım `AfterCommit`'te `entry.request` üstünden `SET_VIEW_RANGE.owns(op)`
  doğrudan çağrılarak yapılıyor. Kayıt dağıtımı olmasa da yerleşik açılım aynı değişimi üretiyor ve
  sayım aynı çıkıyor: M1'de iki Tally sınaması yeşil. Yani "komut alır" (GOAL §5, PLAN §7b "kayıt
  üstünden bir `Op` türü alır") Tally için gösterilmiyor; dağıtımı bugün yalnız (a) `add_watch`
  sınaması yakalıyor. Düzeltme önerisi: Tally'nin açılımı ayırt edilebilir bir iz bıraksın
  (ör. yerleşik değişimlere ek bir değişim ya da `ctx` üstünden bir işaret) ve sınama o izi
  arasın; ya da sayımı `request` yerine açılımın ürettiği izden yapsın. Kabul: M1 altında Tally
  sınaması kırmızı.
- **B4 — (c) sınamasının Cargo taraması dar (düşük).** Yalnız `qp-watch =`/`qp-watch=` ile başlayan
  satırları arıyor; `qp-watch.workspace = true` ya da `qp-watch.path = …` biçimini kaçırır.
  `arch-check` katman sırası alttaki crate'e bağımlılığa izin verdiği için (`qp-watch` sırada
  `qp-authority`'den önce) bu sınama tek bekçi. Öneri: satırın ilk anahtarını `.`/`=`/boşlukta
  kesip karşılaştırmak ya da `cargo metadata` ile denetlemek.
- (B-c) `a_registered_claim_kind_keeps_the_limited_guards`: iki `limited` kolu (`MAX_DISPUTE_DIGESTS`,
  `MAX_DISPUTES_PER_CONN`) ve başka bağlantının sınırlanmadığı sınanıyor; yalnız `tests/`. ✓
- PLAN §7a'nın "satır geri gelince döküm bayt eşit" yarısı ek sınamada değil `compare.sh`'ta
  karşılanıyor (270/270); kabul edilebilir, raporda böyle anılmalı.

## 4. Belge doğruluğu (10 iddia)

| # | İddia | Kod | Sonuç |
|---|---|---|---|
| 1 | `Check`, `Connect`, `Subscribe`, `Unsubscribe`, `Adjust` → `authority/session/phases.rs`; `Expand` | `phases.rs:244,263,305,317,405`; `expand_ops` `phases.rs:48` → `Registry::expand_ops` | ✓ |
| 2 | `AfterCommit` → `accept.rs` | `accept.rs:228` | ✓ |
| 3 | `AfterData`: `Data` → `settle.rs`; `Look`, `Written` → `capsule/run.rs` | `settle.rs:130`; `run.rs:121,125,146` (`look_phase`/`written_phase`, tanım `phases.rs:181,195`) | ✓ |
| 4 | `Proposals` → `clock.rs`; `Settled` → `capsule/dispute.rs`; `Close` çağrılmaz | `clock.rs:151`; `dispute.rs:253` → `settled_phase`; `phase.rs:31` | ✓ |
| 5 | Kayıt: `compose.rs` `LIST`'e bir satır | `compose.rs:14-16` | ✓ |
| 6 | `qp-session/Cargo.toml`'a bağımlılık satırı gerekir | `Cargo.toml:15` `qp-watch`; `LIST` `qp_watch::WATCH`'u adıyla anıyor, satırsız derlenmez | ✓ |
| 7 | `session.sub::<T>()` / `sub_mut` | `compose.rs:109,114` | ✓ |
| 8 | Aynı `op`/`kind` iki kez kaydedilemez; kayıtlı işlem yerleşikten önce sorulur | `registry.rs:26` `claim`; `registry.rs:207-219` | ✓ |
| 9 | Motor girdisi yazar `qp`, `MAX_EMIT_DEPTH` | `outbox.rs:5,38` (8) | ✓ |
| 10 | D2 `qp-ops`/`qp-authority` kapalı eşlemeler | aşağıda | kısmen ✗ (B2) |

**D2 grep'i (yeniden koşuldu).** `ChangeOp::`/`Collection::` geçen dosyalar: `qp-authority` 18,
`qp-ops` 7, `qp-plane` 1, `qp-watch` 2. Doğrulananlar: `apply_change` 24 kol, `touched` 23 kol
(`ChangeOp` 24 tür; `touched_key`'in `other =>` kolu var), `collection_items` 9 kol = 9 `Collection`
türü, `coll_name`, `expand_remove`, `find_collection`, `all_ids` yerinde; `capsule/public.rs`
`item_json` zinciri derleyicinin zorlamadığı liste (bugün `comments`'i de içermiyor). Belgede
sayılmayan `events.rs`, `public/commit.rs`, `reader.rs`, `worlds.rs` eşlemeleri joker kollu ya da
tek tür süzgeci: zararsız.

**B2 — `qp-authority` listesi eksik (orta).** Tablonun ilk satırı (`Document` alanı) iki
`qp-authority` dosyasını derleyiciyle zorunlu kılar; `Document`'ta `Default` yok ve iki tam yapı
değişmezi var:
- `crates/qp-authority/src/replica/viewer.rs:331` — görüntüleyicinin açık tarafı için doküman
  (her alan elle; yeni koleksiyonun açık tarafa gidip gitmeyeceği aynı zamanda gizlilik kararı).
- `crates/qp-authority/src/capsule/queries.rs:373` — `ledger_now`'ın boş dokümanı.

Bu yüzden "bugün 1 zorunlu + 1 yumuşak" yerine "3 zorunlu (`read/query.rs`, `replica/viewer.rs`,
`capsule/queries.rs`) + 1 yumuşak" olmalı. (Başka crate'te `Document {…}` değişmezi yok.)

Belgenin geri kalanı (kod düzeni, protokol çizimi, bilinen sınırlar B-a/B-e/B-f, kapalı
`Query`/`Topic`/`Gesture`/`Event` notu) kodla tutarlı.

## 5. `qp_session::AuthorityReplica`

- Gerekli değil: `qp-authority`, `qp-session`'ın normal bağımlılığı; bütünleşik sınama onu
  doğrudan `qp_authority::Replica` olarak kullanabilir (`rig.rs` zaten `qp_subsystem`, `qp_source`,
  `qp_ops`, `qp_schema`'yı doğrudan kullanıyor). Daha dar seçenek: `qp_session::Replica::with_list`.
- Kabul edilebilir: var olan `Authority` (`qp_authority::Session`) ve `Subsystems` takma adlarıyla
  aynı desen; belge (§8 "Barındırıcı API'si") bu yolu anlatıyor; `qp-session/src`'ye +3 satır
  (K89 tavanı yalnız `qp-authority` için; arch-check yeşil).
- **B3 — listelenmemiş (düşük).** `rounds/RF1/experiments/paths.md` ("her birleştirmede güncellenir")
  E bölümü/yeni yol satırı içermiyor; `AuthorityReplica` yalnız `ARCHITECTURE.md`'de geçiyor. GOAL
  ölçüt 14 "dışa açık Rust yolu değişiklikleri listeli" için `paths.md`'ye "E — yeni yollar:
  `qp_session::AuthorityReplica`" eklenmeli (ya da takma ad kaldırılıp sınama `qp_authority::Replica` kullanmalı).

## Bulgular özeti

| # | Ağırlık | Bulgu | Düzeltme |
|---|---|---|---|
| B1 | orta | Tally'nin `OpHandler`'ı çağrıldığı gösterilmiyor; M1'de Tally sınamaları yeşil | açılım ayırt edilebilir iz bıraksın; M1 altında kırmızı |
| B2 | orta | D2 `qp-authority` listesi eksik: `replica/viewer.rs`, `capsule/queries.rs` (`Document` değişmezleri) | tabloya ekle; "3 zorunlu + 1 yumuşak" |
| B3 | düşük | `AuthorityReplica` `paths.md`'de yok; takma ad zorunlu değil | `paths.md`'ye satır ya da takma adı kaldır |
| B4 | düşük | (c) Cargo taraması `qp-watch.workspace`/`.path` biçimini kaçırır | anahtar ayrıştırma ya da `cargo metadata` |

## Temizlik

Worktree `.claude/worktrees/audit-e` ve `target/wt/audit-e` silindi; scratchpad günlükleri silindi.

## Yeniden denetim

Dal: `rf1/e-extension` @ `8536e567` (4 düzeltme commit'i: `aebec96b` B1, `1c1af96e` B2, `1f7a61f6` B3,
`8536e567` B4). Kendi worktree'si (`.claude/worktrees/audit-e2`, ayrık), `CARGO_TARGET_DIR=target/wt/audit-e2`.
Dala yazılmadı; mutasyonlar geri alındı (`git status` temiz, `Cargo.lock` geri alındı).

### Hüküm: GEÇTİ

B1–B4 kapandı; kapılar yeşil.

### Kapılar

| Adım | Sonuç |
|---|---|
| `git diff --stat main...rf1/e-extension -- crates/qp-authority/src crates/qp-schema` | boş |
| Bütün fark | 6 dosya, +623/−3; `qp-session/src` artık `main` ile aynı |
| fmt / clippy `-D warnings` | yeşil / yeşil |
| `cargo test --workspace` | 1240 geçti, 0 kaldı, 9 yok sayıldı; 5 yeni sınama geçti |
| `compare.sh` | 270/270 bayt eşit |
| `arch-check` | 0 hata, 2 adlı istisna, İ3 20129 (değişmedi) |
| `--config-diff main` | 0 değişiklik, 0 dayanaksız |

### Mutasyonlar (aynı üç bozma)

| # | `ext_tally_counts…` | `ext_tally_a_replica…` | `ext_without_the_watch_line…` | `ext_…no_subsystem_crate` |
|---|---|---|---|---|
| M1 `Registry::op` → `None` | **FAILED** | **FAILED** | FAILED | ok |
| M2 `emit` motor girdisi yazmaz | FAILED | FAILED | ok | ok |
| M3 `replay` `qp` girdilerini atlar | FAILED | ok | ok | ok |

- **B1 kapandı.** Tally'nin `expand`'i yerleşik açılımdan sonra `tally-op` anlatısını (iz)
  ekler; sayım `entry.changes`'teki izden yapılır, `request`'ten değil. Sınama girdi başına iz
  sayısını `[1, 1, 1, 0]` (üç `set_view_range`, bir `set_emphasis`) olarak ister. M1 altında iki
  Tally sınaması artık kırmızı.
- **B2 kapandı.** Tabloya `replica/viewer.rs` (açık taraf, gizlilik kararı notuyla) ve
  `capsule/queries.rs` eklendi; "3 zorunlu + 1 yumuşak". Grep yeniden koşuldu: `*/src` altında
  `Document { … }` yapı değişmezi yalnız `qp-authority/src/replica/viewer.rs:331` ve
  `capsule/queries.rs:373` (`project.rs:218` dönüş tipi, `reply.rs:350` `QueryResult` türü);
  `ChangeOp::`/`Collection::` geçen `qp-authority` dosya sayısı 18, ilk denetimle aynı. Liste dürüst.
- **B3 kapandı.** `pub use qp_authority::Replica as AuthorityReplica` kaldırıldı; sınama
  `qp_authority::Replica`'yı doğrudan kullanıyor; depoda `AuthorityReplica` geçen yer kalmadı
  (yalnız bu rapor); `ARCHITECTURE.md` §8 güncel. Yeni dışa açık yol yok, `paths.md` satırı gerekmez.
- **B4 kapandı.** (c) bağımlılıkları `cargo metadata --no-deps --offline` ile okuyor (normal,
  dev, build; yeniden adlandırmada `name` paket adıdır). Denendi: `qp-watch.path = "../qp-watch"`
  (noktalı) ve `[dev-dependencies.qp-watch]` (tablo) biçimlerinin ikisinde de sınama kırmızı
  ("qp-authority depends on subsystem crate qp-watch").

### Not (bulgu değil)

- İz anlatısı (`tally-op`) dokümana yazılır; yalnız sınamadadır, ürün yolunu etkilemez.
- (c) sınaması alt süreçle `cargo metadata` çağırır (`--offline`); ağ istemez.

### Temizlik

`.claude/worktrees/audit-e2` ve `target/wt/audit-e2` silindi; scratchpad günlükleri silindi.
