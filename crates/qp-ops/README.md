# qp-ops

**İşlemler:** dokümanın saf fonksiyonları. Protokol işlemlerinin yeniden oynatılabilir
değişikliklere açılımı (`expand`, gramer işlemleri `gops`, izleme işlemleri `watch`),
seçiciler (`select`, `locate`), kanonik biçim (`canon`), özet (`summary`), değişiklik
uygulama ve yeniden oynatma (`apply`: `replay`, `ENGINE_PRINCIPAL`), sunucu (`present`) ve
derleyicinin veri kaynağı olarak doküman (`provider`: `DocProvider`, `HeadlineSource`);
canlı yolun saf yardımcıları (`live`). RF1 S2'de `qp-session`'dan saf taşımayla ayrıldı
(`rounds/RF1/PLAN.md` §3.3, §11 D2).

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada `qp-plane`'den sonra,
`qp-session`'dan önce.

**Bağımlılıklar:** `qp-plane`, `qp-schema`, `qp-source`, `qp-compute`, `qp-capsule`,
`qp-grammar`; `serde`, `serde_json`.

**Dışa açık yüzey (`lib.rs`):** modüller `apply`, `canon`, `expand`, `gops`, `live`,
`locate`, `present`, `provider`, `select`, `summary`, `watch`; kökte `replay`, `apply_all`,
`apply_change`, `find_collection`, `keys_overlap`, `touched_ids`, `touched_key`,
`touched_keys`, `ENGINE_PRINCIPAL`, `canon_document`, `thresholds`, `profile_for`,
`DocProvider`, `HeadlineSource`.

**Düzen (`src/`; alt klasörlerin kendi `README.md`'si var):** `expand/` (`expand_op` ve
yardımcıları), `canon/`, `select/`, `summary/`; öbürleri tek dosya.

**Ne değildir:** oturum değildir (`Session`'ı, bağlantıları, günlüğü tanımaz; okuyucunun
başlık parçalarını `HeadlineSource` üstünden alır); veri düzlemi değildir (`qp-plane`);
sahne derlemez (`qp-grammar`).

**Bilinen borç:** `src/expand/op.rs` tek fonksiyonlu 800 üstü dosya (`expand_op`, İ6); S4'te
kol başına fonksiyonlara bölünecek.
