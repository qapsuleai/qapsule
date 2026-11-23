# qp-session

**Başvuru otoritesi** `qp/0` alan protokolü için (`docs/PROTOCOL.md`). `Session` bir belgeyi,
günlüğünü, bağlantıları, abonelikleri, önerileri ve varlığı tutar; her gelen ileti
`Session::handle_raw` sınırından geçer (boyut sınırı, ayrıştırma, JSON işaretçili şema
denetimi, işleme). Belgeyi değiştiren her şey komut yolundan geçer. Hesapları `qp-compute`
ile koşar, kapsül motorunu bağlar, görünümü `qp-grammar` ile sahneye derler. Saat `clock`
iletisiyle gelir; rastgelelik yok.

**Katman:** oturum (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-ops` (işlemler), `qp-plane` (veri düzlemi, değer önbelleği, hata
kurucusu, saf kapsül dizinleri), `qp-schema`, `qp-calendar`, `qp-compute`, `qp-facts`,
`qp-capsule`, `qp-voice`, `qp-grammar`, `qp-look`, `qp-glyph`, `qp-designs`, `qp-source`;
`schemars`, `serde`, `serde_json`, `serde_path_to_error`, `tempfile`.

**Dışa açık yüzey (`lib.rs`):** `Session`, `permissions`, `confine`; `Replica`,
`ReplicaOptions`, `document_digest`, `view_digest`; `ReplayHost`, `demo_init`; `hit_scene`,
`screen_frame`, `explain_scene`; modüller `capsule`, `replica`, `viewer`, `replayhost`,
`conform`, `agent`, `voices` (`qp_voice`). RF1 S2'den beri `replay`, `apply_*`,
`ENGINE_PRINCIPAL`, `DocProvider`, `profile_for`, `canon_document`, `thresholds` `qp-ops`'ta;
`DataError`, `HostNotice`, `did_you_mean`, `edit_distance`, `headparts` `qp-plane`'de.

**Düzen (`src/`; her klasörün kendi `README.md`'si var):**

- `authority/` — otorite ve komut yolu: `Session` (`session/`), ajan politikası.
- `capsule/` — kapsül yaşamı: motor bağlama, iddialar, itiraz, yorumlar, izleme eşleşmesi.
- `data/` — canlı yol, bağ sorguları.
- `read/` — sorgu, anlamsal okuma, önizleme, açıklama, etkileşim.
- `replica/` — yansı, görüntüleyici (`viewer`), kayıt oynatıcı (`tape`).
- `host/` — yeniden oynatma barındırıcısı (`replayhost`) ve uygunluk koşucusu (`conform`).

**Ne değildir:** kabuk, uyarlayıcı (CLI/MCP/WASM), kapsül motoru, veri kaynağı, çizici.

**Bilinen borç:** crate bütçenin üstünde (`crate.lines`, bütçe 12 bin) — İ3; RF1 S3
dalgaları alt sistemlere böler (`rounds/RF1/PLAN.md` §3.3).
`tests/` 39 dosya (`dir.files`). `qp-session → qp-source` bugün `layer.up` (İ1 ile kalkar).
