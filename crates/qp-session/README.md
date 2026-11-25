# qp-session

**Bileşim** (`docs/CONCEPTS.md` "Bileşim"): `qp/0` otoritesinin dışa açık yüzü. Otorite
çekirdeği `qp-authority`'dedir ve hiçbir alt sistemi tanımaz; standart alt sistemler buraya
birer kayıt satırıyla girer (`src/compose.rs`, `LIST`; `rounds/RF1/PLAN.md` §3.3 satır 8,
§7). Kayıtlı: `qp-watch` (izleme niyeti, S3.1), `qp-read` (okuma ve sunum, S3.2),
`qp-viewer` (görüntüleyici bağlantıları, S3.3).

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada en üstte
(`… < qp-read < qp-authority < qp-replica < qp-viewer < qp-replayhost < qp-session`).

**Bağımlılıklar:** `qp-authority` (otorite), `qp-subsystem` (sözleşme), `qp-watch`, `qp-read`,
`qp-viewer` (alt sistemler), `qp-replica`, `qp-replayhost` (otoritenin istemcileri), `qp-source` (sağlayıcı tipleri, kurucuların imzası); uygunluk koşucusu (`conform`)
için `qp-ops`, `qp-plane`, `qp-schema`, `serde`, `serde_json`, `tempfile`. Sınamalar ve
örnekler için (`dev-dependencies`): `qp-capsule`, `qp-compute`, `qp-grammar`, `qp-look`,
`qp-designs`, `schemars`.

**Bileşim (`src/compose.rs`):** `STANDARD` (standart alt sistemler, kayıt sırasıyla),
`FOLLOWER` (izleyicilerin listesi: `STANDARD` eksi okuma ve görüntüleyici, sıra aynı; RF1 S3.2,
S3.3) ve onlarla kurulan otoriteler. Her biri `qp-authority`, `qp-replica`, `qp-viewer` ya da
`qp-replayhost` tipini sarar (`Deref`/`DerefMut`; bugünkü çağrı
yerleri değişmez) ve aynı adlı kurucularla `STANDARD`'ı (izleyicilere `FOLLOWER`'ı) verir.
İzleyici oturumu (`Replica::session_mut`, `OpenSide::replica_mut`) sorgu ya da girdi almaz:
`handle_raw` ile gönderilirse `unsupported` döner.

- `Session` (`new`, `new_with_trust_keys`, `with_provider`, `with_provider_and_trust_keys`;
  `sub::<T>()`, `sub_mut::<T>()`, `into_inner`);
- `ReplayHost` (`new`, `with_local_dir`, `with_window`, `live`);
- `Replica` (`new`, `with_memory`); `OpenSide` (`viewer::OpenSide::new`): `FOLLOWER`;
- `tape::{run, run_with, run_viewer}` (otorite `STANDARD`, yansı ve açık taraf `FOLLOWER`); `agent::agent_op` (standart alt sistemlerin işlemleri
  kendi işleyicileriyle).

Bir otoritenin doğurduğu otorite (barındırıcının yeniden kurduğu oturum, yansının izleyicisi,
görüntüleyicinin açık tarafı) listeyi ondan alır (`qp_authority::Subsystems`).

**Dışa açık yüzey (`lib.rs`):** yukarıdakiler; `Authority` (`qp_authority::Session`: alt
sistemsiz otorite; barındırıcının ve yansının içindeki oturumun tipi), `Subsystems`; ve
`qp-authority`'nin kök yüzeyinin bilerek yapılmış yeniden dışa aktarımı, aynı adlarla:
`permissions`, `confine`, `default_limits`, `read_json`; `ReplicaOptions`, `document_digest`,
`view_digest` (`qp-replica`); `demo_init` (`qp-replayhost`); `hit_scene`, `screen_frame`, `explain_scene`, `HIT_WIDTH`,
`HIT_HEIGHT`; `CoinFlipRule`, `CommentAudit`, `CompileOptions`, `SettleRequest`,
`MAX_DISPUTE_DIGESTS`, `MAX_OPEN_DISPUTES`, `MAX_DISPUTES_PER_CONN`; modüller `capsule`,
`replica`, `viewer`, `replayhost`, `agent`, `voices`, `tape` (gizli; bunların beşi yukarıdaki
sarıcıları da taşır), `compose`, `conform`.

**Kural:** `Deref` yalnız bugünkü çağrı yerlerini korumak içindir. **Yeni kod `Deref`'e
yaslanmaz, `sub::<T>()` / `sub_mut::<T>()` kullanır** (`rounds/RF1/DECISION.md`, küçük
notlar). Barındırıcının (`ReplayHost::session`) ve yansının içindeki oturum `Authority`'dir;
ikisini birlikte tutan çağıran `&Authority` alır.

**Düzen:** `src/lib.rs` (yeniden dışa aktarımlar, sarıcı modüller), `src/compose.rs`
(bileşim), `src/conform.rs` (uygunluk yazı dökümü koşucusu, S3.0'da otoriteden taşındı);
`tests/` otoritenin bütünleşme sınamaları (S1 karakterizasyonu `tests/charac`), `examples/`
yazı dökümü üreticileri. İkisi de dışa açık yolu (`qp_session::…`) sınar.

**Ne değildir:** otorite değildir (komut yolu, günlük, iki dünya `qp-authority`'de); alt
sistem değildir; alt sistem sözleşmesi değildir (`qp-subsystem`).

**Bilinen borç:** `tests/` 39 dosya (`dir.files`).
