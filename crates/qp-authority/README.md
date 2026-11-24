# qp-authority

**Otorite çekirdeği** (`Authority` kavramı; tip adı bugün `Session`). **Başvuru otoritesi** `qp/0` alan protokolü için (`docs/PROTOCOL.md`). `Session` bir belgeyi,
günlüğünü, bağlantıları, abonelikleri, önerileri ve varlığı tutar; her gelen ileti
`Session::handle_raw` sınırından geçer (boyut sınırı, ayrıştırma, JSON işaretçili şema
denetimi, işleme). Belgeyi değiştiren her şey komut yolundan geçer. Hesapları `qp-compute`
ile koşar, kapsül motorunu bağlar, görünümü `qp-grammar` ile sahneye derler. Saat `clock`
iletisiyle gelir; rastgelelik yok.

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada `qp-subsystem`'den sonra,
`qp-session` bileşiminden önce. Hiçbir alt sistem crate'ini tanımaz. RF1 S3.0'da
`qp-session`'ın içeriği saf taşımayla buraya geldi; çağıranlar `qp-session` bileşiminin
yeniden dışa aktardığı yolları kullanır (`rounds/RF1/PLAN.md` §3.3).

**Bağımlılıklar:** `qp-ops` (işlemler), `qp-plane` (veri düzlemi, değer önbelleği, hata
kurucusu, saf kapsül dizinleri), `qp-schema`, `qp-calendar`, `qp-compute`, `qp-facts`,
`qp-capsule`, `qp-voice`, `qp-grammar`, `qp-look`, `qp-glyph`, `qp-designs`, `qp-source`;
`qp-subsystem` (alt sistem sözleşmesi); `schemars`, `serde`, `serde_json`,
`serde_path_to_error`.

**Dışa açık yüzey (`lib.rs`; `qp-session` bunları aynı adlarla yeniden dışa aktarır):** `Session`, `permissions`, `confine`; `Replica`,
`ReplicaOptions`, `document_digest`, `view_digest`; `ReplayHost`, `demo_init`; `hit_scene`,
`screen_frame`, `explain_scene`; modüller `capsule`, `replica`, `viewer`, `replayhost`,
`agent`, `voices` (`qp_voice`). RF1 S2'den beri `replay`, `apply_*`,
`ENGINE_PRINCIPAL`, `DocProvider`, `profile_for`, `canon_document`, `thresholds` `qp-ops`'ta;
`DataError`, `HostNotice`, `did_you_mean`, `edit_distance`, `headparts` `qp-plane`'de.

**Düzen (`src/`; her klasörün kendi `README.md`'si var):**

- `authority/` — otorite ve komut yolu: `Session` (`session/`), ajan politikası.
- `capsule/` — kapsül yaşamı: motor bağlama, iddialar, itiraz, yorumlar, izleme eşleşmesi.
- `data/` — canlı yol, bağ sorguları.
- `read/` — sorgu, anlamsal okuma, önizleme, açıklama, etkileşim.
- `replica/` — yansı, görüntüleyici (`viewer`), kayıt oynatıcı (`tape`).
- `host/` — yeniden oynatma barındırıcısı (`replayhost`); uygunluk koşucusu (`conform`) S3.0'da
  bileşime (`qp-session`) taşındı.

**Ne değildir:** kabuk, uyarlayıcı (CLI/MCP/WASM), kapsül motoru, veri kaynağı, çizici.

**Bilinen borç:** crate bütçenin üstünde (`crate.lines`, bütçe 12 bin) — İ3; RF1 S3
dalgaları okuma, yansı, görüntüleyici ve barındırmayı dışarı alır (`rounds/RF1/PLAN.md` §3.3).
Bütünleşme sınamaları ve örnekler `qp-session`'da (`tests/` 39 dosya, `dir.files`).
