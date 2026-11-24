# qp-session

**Bileşim** (`docs/CONCEPTS.md` "Bileşim"): `qp/0` otoritesinin dışa açık yüzü. Otorite
çekirdeği `qp-authority`'dedir ve hiçbir alt sistemi tanımaz; standart alt sistemler
(izleme, okuma, görüntüleyici) buraya birer kayıt satırıyla girer (`rounds/RF1/PLAN.md`
§3.3 satır 8, §7). RF1 S3.0'da kayıt boştur; ilk kayıt satırı S3.1'de (`qp-watch`) gelir.

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada en üstte
(`qp-plane < qp-ops < qp-subsystem < qp-authority < qp-session`).

**Bağımlılıklar:** `qp-authority`. Sınamalar ve örnekler için (`dev-dependencies`):
`qp-schema`, `qp-source`, `qp-capsule`, `qp-compute`, `qp-grammar`, `qp-look`, `qp-designs`,
`qp-ops`, `qp-plane`; `serde`, `serde_json`, `tempfile`.

**Dışa açık yüzey (`lib.rs`):** `qp-authority`'nin kök yüzeyinin bilerek yapılmış yeniden
dışa aktarımı, aynı adlarla: `Session`, `permissions`, `confine`, `default_limits`,
`read_json`; `Replica`, `ReplicaOptions`, `document_digest`, `view_digest`; `ReplayHost`,
`demo_init`; `hit_scene`, `screen_frame`, `explain_scene`, `HIT_WIDTH`, `HIT_HEIGHT`;
`CoinFlipRule`, `CommentAudit`, `CompileOptions`, `SettleRequest`, `MAX_DISPUTE_DIGESTS`,
`MAX_OPEN_DISPUTES`, `MAX_DISPUTES_PER_CONN`; modüller `capsule`, `replica`, `viewer`,
`replayhost`, `conform`, `agent`, `voices`, `tape` (gizli). Çağıranlar (`qp-host`, `qp-mcp`,
`qp-shellcore`, `qp-cli`, `qp-wasm`, `qp-render`) yollarını değiştirmez.

**Kural:** `Deref` (S3.1'de `Session(Authority)` yeni tipiyle gelecek) yalnız bugünkü çağrı
yerlerini korumak içindir. **Yeni kod `Deref`'e yaslanmaz, `sub::<T>()` / `sub_mut::<T>()`
kullanır** (`rounds/RF1/DECISION.md`, küçük notlar).

**Düzen:** `src/lib.rs` (yalnız yeniden dışa aktarımlar); `tests/` otoritenin bütünleşme
sınamaları (S1 karakterizasyonu `tests/charac`), `examples/` yazı dökümü üreticileri. İkisi
de dışa açık yolu (`qp_session::…`) sınar.

**Ne değildir:** otorite değildir (komut yolu, günlük, iki dünya `qp-authority`'de); alt
sistem değildir; alt sistem sözleşmesi değildir (`qp-subsystem`).

**Bilinen borç:** `tests/` 39 dosya (`dir.files`).
