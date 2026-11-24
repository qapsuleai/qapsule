# qp-read

**Okuma ve sunum** (`docs/CONCEPTS.md`; `rounds/RF1/PLAN.md` §3.3 satır 4, RF1 S3.2):
otoritenin sorguları (§5.4) ve girdi hareketleri (§4.6, §7) bir alt sistem olarak. Belgeyi ve
günlüğü değiştirmez; değişiklik komut yolundan geçer.

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada `qp-watch`'tan sonra,
`qp-authority`'den önce (`qp-plane < qp-ops < qp-subsystem < qp-watch < qp-read <
qp-authority < qp-session`). Otoriteyi tanımaz; otorite de onu tanımaz. Bileşim (`qp-session`)
bir kayıt satırıyla takar.

**Bağımlılıklar:** `qp-subsystem` (sözleşme), `qp-ops` (sunucu, sağlayıcı, seçiciler,
`bounds`, `firsts`), `qp-plane` (veri düzlemi, değer önbelleği, izdüşüm, hata kurucusu),
`qp-grammar`, `qp-designs`, `qp-look`, `qp-glyph`, `qp-voice` (sahne derleme, tasarımlar),
`qp-capsule` (`StudyMemo`), `qp-compute` (çalışma adları), `qp-source` (veri konumu),
`qp-schema`; `schemars` (`describe`), `serde`, `serde_json`.

**Dışa açık yüzey (`lib.rs`):** `Read` (alt sistem), `READ` (bileşimin kaydettiği değer),
`ReadState` (yuva), `scene_for_range` (bileşimin `Session::scene_for`'u), `explain_scene`,
`hit_scene`, `screen_frame`, `HIT_WIDTH`, `HIT_HEIGHT`.

**Kancalar:**

- Sorgu kaydı (`QueryHandler`, tel etiketi `ANY`): başka alt sistemin kaydetmediği her sorgu
  türü (`describe`, `get`, `list`, `document`, `log`, `study`, `at`, `screen`, `attention`,
  `hit`, `binding`, `capsules`, `preview`, `stories`, `capsule`, `ledger`,
  `verify_evidence`, `present`). Otorite onları okuyucunun izdüşümünde (`as_reader`) çağırır.
- Girdi kaydı (`InputHandler`, `ANY`): her hareket (`look`, `pan`, `zoom`, `tap`, `tap_at`,
  `drag`, `pinch`, `open`, `close`, `style`).
- Yuva: `ReadState` (bağlantı başına görünüm aralıkları, son görünüm, son girdi; kişi başına
  odak; girdi sayacı). Değer önbelleği ve bağlantıların görünüm durumu (`look`) çağrı
  bağlamından ödünç gelir (`SessionCtx::cache`, `SessionCtx::looks`).

**Ne değildir:** kapsül yaşamı değildir: `capsule` sorgusunun ayrıntısı, `stories` sorgusu,
kapsül öğeleri ve cümleleri otoritede kalır, görünüm (`SessionView`) üstünden sorulur.
Otoritenin kendi derlemesi (`provider`, `scene`, `scene_as_reader`) değildir: barındırıcı ve
yansı onları sorgu dışında kullanır.

**Düzen:** `src/lib.rs` (alt sistem, kancalar), `rd.rs` (okuma bağlamı `Rd`: bölünmüş ödünç,
yuva), `query.rs` + `query/view.rs` (sorgular, `input`), `reading.rs` (`screen`, `hit`),
`interact.rs` + `interact/{scenes,style}.rs` (piksel hareketleri, derinlikler), `preview.rs`,
`explain.rs`, `bindq.rs` (bağ sorguları), `capsules.rs` (`capsules`, `ledger`).
