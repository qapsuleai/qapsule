# Karar Günlüğü — Önerilen (RF1) — Arkin'in onayını bekliyor (2026-11-25)

<!-- dizin: DECISIONS.md; RF1 bölme turunun önerdiği satırlar -->

RF1 bu satırları **önerir**; kesinleştirmez. Round içinde Arkin'in verdiği kararlar
`rounds/RF1/DECISION.md`'de kendi sözleriyle duruyor: plan onayı, S4 dağıtıcısı, S3.0 araç izni
ve İ8. Kanıt `rounds/RF1/REPORT.md`, `AUDIT.md`, `experiments/`.

| # | Önerilen karar | Kanıt |
|---|----------------|-------|
| Ö119 | **Oturum katmanında katman içi sıra:** `qp-plane < qp-ops < qp-subsystem < qp-watch < qp-read < qp-authority < qp-replica < qp-viewer < qp-replayhost < qp-session`. Otorite çekirdeği (`qp-authority`) hiçbir alt sistem crate'ine bağımlı değildir (`cargo tree`; ek noktası sınaması). | `arch-config.json`; `docs/ENGINEERING.md` §2; audit-s33. |
| Ö120 | **Alt sistem sözleşmesi `qp-subsystem`:** `Subsystem` (durumsuz `&'static dyn`), `SessionView` / `ReadView`, `SessionCtx` (bölünmüş ödünç), `Slots` / `PerWorld`, `Outbox`, adlı fazlar (Check, Expand, AfterCommit, AfterData, Proposals, Subscribe/Unsubscribe, Connect, Settled, Adjust), kayıtlar (`OpHandler` açılım + politika, sorgu/girdi/konu, `ConditionKind`, `ValueSource`, `ClaimKind`). Yeni alt sistem bileşime (`qp-session` `compose.rs`) bir kayıt satırıyla girer; otorite crate'i değişmez. | `docs/ARCHITECTURE.md` "Yeni alt sistem nasıl eklenir"; ek noktası sınaması `qp-session/tests/extension`. |
| Ö121 | **Bileşim iki liste tutar:** `STANDARD` (otorite: izleme, okuma, görüntüleyici) ve `FOLLOWER` (izleyiciler — yansı, açık taraf — okuma ve görüntüleyici olmadan; izleyiciler sorgu/girdi almaz). Yeni alt sistemin yazarı izleyicide gerekip gerekmediğine karar verir. | DECISION.md uygulama notu; audit-s32 adım 2, audit-s33. |
| Ö122 | **Çekirdek donuktur** (DECISION.md, Arkin): `qp-authority` İ3 tavanı 12.744, mandallı; kapsül yaşamı, iddia/itiraz/yorum ve canlı yolun ayrılması borçtur (`docs/DEBT.md`), aday RS1'den sonra. | REPORT §3. |
| Ö123 | **Gramer bölünmesi:** `qp-look` (görünüş, tema, hata), `qp-glyph` (anlam işareti), `qp-grammar` (derleyici; tasarım kaydı `Designs` fonksiyon tablosu, 5 çağrı noktası), `qp-designs` (adlı tasarımlar; varsayılan kayıt `qp_designs::options`). `qp-grammar` tasarım tarafını tanımaz. R7 işaretleri derleyici alfabesine (pay ≈ 4,3 bin). | G1–G3 denetimleri. |
| Ö124 | **`qp-voice`** ayrı crate (`qp_capsule::voice` → `qp_voice`). | audit-v. |
| Ö125 | **İstisnalar kapandı:** İ2 (katman içi sıra), İ4 (`qp-grammar` 7.710), İ5 (`qp-capsule` 8.467), İ6 (`prepare`, `expand_op` bölündü). Kalanlar İ1 (`qp-source`, kesildi) ve İ3 (`qp-authority` 12.744). Yeni: **İ8** kabuk WASM paketi (Arkin'in kararı: ham ve gzip en çok +%2,5; ölçülen +%2,45 / +%2,17). | `arch-config.json`; DECISION.md İ8. |
| Ö126 | **Araçlar kalıcı:** `scripts/refactor/` (`snapshot`, `compare`, `perf`, `moved` — `use`, `impl` başlığı, öğe makrosu, `cfg` denetimleriyle —, `graph --map`); `arch-check` katman içi sıra, istisna tavanları, `--config-diff` (`verify.sh` adımı `config`; karar atfı yeni olmalı; yeni crate'e taşınan istisna tanınır). | audit-a (iki tur), audit-s30 B7. |

## Açık (Arkin'e)

| # | Soru | Öneri |
|---|------|-------|
| A70 | İ8: kabuk WASM paketinde (`--no-opt`) crate'ler arası jenerik kopyalar ≈ 39 KB; tavana 4,8 KB kaldı. R7'de izleyici yoluna eklenecek küçük kod bile tavanı aşar. | Kabuk paketinde `wasm-opt` (ya da `codegen-units`/`lto` ayarı) için kısa bir ölçüm işi, R7'den önce ya da R7'nin ilk işi. |
| A71 | Gizli izleyici yöntemleri (`follow_start`, `mirror_apply`, `mirror_commit`, `follow_data`, `follow_resync`, `take_provider`) komut yolu dışından yazıyor ve `DerefMut` yüzünden her barındırıcıdan çağrılabilir (audit-s33 Y1). | `Follower(Session)` tipi: yazan yöntemler yalnız orada. RS1'den önce küçük bir RF işi. |
| A72 | Kesilenler: `arch-check` sözdizimi ağacı (K-10), `qp-source` dosya okuması (İ1), geçici yeniden dışa aktarmalar, `unimplemented`/`unreachable` lint'leri (K-11). | Sonraki RF ya da RL1'in ön işi; lint'ler küçük, önce o. |
| A73 | Kapalı enum'lar (`Query`, `Topic`, `Event`, `ChangeOp`) yeni tür için şema değişikliği ister; alt sistem yalnız var olanı üstlenebilir. RL1 ve R7 şemaya kol ekleyecek. | Şema kolu round'un işi; otorite tarafında dokunulacak dosya 0 (D2 tablosu: 0 zorunlu + 1 yumuşak). |
