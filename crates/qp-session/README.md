# qp-session

**Başvuru otoritesi** `qp/0` alan protokolü için (`docs/PROTOCOL.md`). `Session` bir belgeyi,
günlüğünü, bağlantıları, abonelikleri, önerileri ve varlığı tutar; her gelen ileti
`Session::handle_raw` sınırından geçer (boyut sınırı, ayrıştırma, JSON işaretçili şema
denetimi, işleme). Belgeyi değiştiren her şey komut yolundan geçer. Hesapları `qp-compute`
ile koşar, kapsül motorunu bağlar, görünümü `qp-grammar` ile sahneye derler. Belirlenimci:
saat `clock` iletisiyle gelir, rastgelelik yok, çıktıda karma tablo yok.

**Katman:** oturum (`docs/ENGINEERING.md` §2).

**Bağımlılıklar:** `qp-schema`, `qp-calendar`, `qp-compute`, `qp-facts`, `qp-capsule`,
`qp-grammar`, `qp-source`, `schemars`, `serde`, `serde_json`, `serde_path_to_error`,
`tempfile`. Geliştirme: `tempfile`.

**Dışa açık yüzey (`lib.rs`):** `Session`, `ENGINE_PRINCIPAL`, `permissions`, `confine`;
`replay`, `apply_all`, `apply_change`; `Replica`, `ReplicaOptions`, `document_digest`,
`view_digest`; `ReplayHost`, `demo_init`; `DocProvider`; `hit_scene`, `screen_frame`,
`explain_scene`; `did_you_mean`; modüller `capsule`, `replica`, `viewer`, `replayhost`,
`conform`, `agent`, `headparts`, `voices` (`qp_capsule::voice`).

**Düzen (`src/`; her klasörün kendi `README.md`'si var):**

- `authority/` — otorite ve komut yolu: `Session` (`session/`), uygulama, hatalar, öneri.
- `ops/` — işlem genişletme (`expand/`), kanonik biçim, seçiciler, konum bulma, özet.
- `capsule/` — kapsül yaşamı: motor bağlama, dizin, iddialar, itiraz, yorumlar, izleme.
- `data/` — veri düzlemi, belge sağlayıcı, canlı yol, bağ sorguları.
- `read/` — sorgu, sunucu, anlamsal okuma, önizleme, açıklama, etkileşim.
- `replica/` — yansı, görüntüleyici (`viewer`), kayıt oynatıcı (`tape`).
- `host/` — yeniden oynatma barındırıcısı (`replayhost`) ve uygunluk koşucusu (`conform`).

**Ne değildir:** kabuk ya da uyarlayıcı değildir (CLI, MCP, WASM bağlamaları üst
katmanlardadır); kapsül motoru, veri kaynağı ya da çizici değildir.

**Bilinen borç:** crate ≈ 31,9 bin satır (`crate.lines`, bütçe 12 bin) — istisna isteği İ3
(`rounds/RG1/PLAN.md` §4). Tek işlevlik 800 üstü `src/ops/expand/op.rs` (`expand_op`) — İ6.
`tests/` 39 dosya (`dir.files`). `qp-session → qp-source` bugün `layer.up` (İ1 ile kalkar).
