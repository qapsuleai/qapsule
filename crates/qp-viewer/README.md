# qp-viewer

**Görüntüleyici bağlantıları** (`docs/PROTOCOL.md` §22; RP3/M15, K48): cihaz bir
görüntüleyicidir, hesap yeri değil. Otoritenin **görüntüleyici alt sistemi** (`viewer` konusu,
`visible` iletisi, her girdiden sonra itilen güncellemeler: akış, ekrandaki hazır yüzler,
açık kapsülün dilimi, veri abonelikleri, piyasa durumu), cihazın açık kapsül tarafı
(`OpenSide`: dilimden başlatılmış bir yansı) ve iki tarafın derlediği sahne, dilim ve sınama
işlevleri (`face_scene`, `open_scene`, `slice`, `check_story`, `trust_text`).

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada `qp-authority`'den (ve S3.3
sonunda `qp-replica`'dan) sonra, `qp-session` bileşiminden önce (RF1 S3.3,
`rounds/RF1/PLAN.md` §3.3 satır 7). Otorite onu tanımaz: bileşim (`qp-session`) standart
listesine bir satırla takar (`VIEWER`); izleyici otoritelerin listesinde (`FOLLOWER`) yoktur
(izleyiciye bağlantı konuşmaz).

**Bağımlılıklar:** `qp-authority` (açık taraf ve dışa açık sahne işlevleri `Session`'ın
`provider`, `item_headline_parts`, `row_stamp` yöntemleriyle derler; yansı), `qp-subsystem`
(sözleşme), `qp-ops` (sağlayıcı, ilk olgular), `qp-plane` (hata kurucusu, bağlı beslemeler),
`qp-grammar`, `qp-designs`, `qp-look`, `qp-facts`, `qp-capsule`, `qp-compute` (kayıt
senaryosu), `qp-source`, `qp-schema`, `serde`, `serde_json`.

**Dışa açık yüzey (`lib.rs`):** `Viewer`, `VIEWER` (alt sistem); `OpenSide`, `open_provider`;
`face_scene`, `open_scene`, `open_scene_with`, `scene_digest`, `story_series`, `look_of`,
`OpenExtra`; `slice`, `check_story`, `StoryCheck`, `TrustState`, `trust_text`; `first_facts`,
`FirstFacts`; `DEFAULT_LIMIT`, `MAX_LIMIT`, `DEFAULT_FACE_WIDTH`; gizli `tape`
(`run_viewer`, `ViewerOutcome`). Çağıranlar `qp_session::viewer::*` yolunu kullanır.

**Kancalar** (`qp_subsystem::TopicHandler`, itilen konu; RF1 S3.3):

- `subscribe`: tekil abonelik ve `limit`/`width` denetimi, bağlantı başına yuva `ViewerConn`
  (`Slots::put_conn`). İlk olay onaydan sonra `push(all)`.
- `pushing`/`pushes`/`push`: her girdiden sonra okuyucunun dünyasında (okuma görünümü ve
  kayıtla: akış `capsules` sorgusunu sorgu işleyicisinden ister) neyin değiştiği; otorite
  satırı ve sırayı (`seq_to`) kurar.
- `visible`/`visible_result`: `visible` iletisi (kaydı, sonra okuyucunun dünyasında yanıt).
- `claim_sub`: açık enstrümanın beslemelerine dayanan iddia görüntüleyici aboneliğine de gider.
- `data`: barındırıcının bu bağlantıya sunduğu beslemeler (`Session::viewer_data`).
- `Unsubscribe` fazı: abonelik düşünce yuva kalkar.

**Düzen:** `src/lib.rs` (bildirim), `viewer.rs` (sahneler, dilim, sınama, güven) ve
`viewer/` (`open`: yüz ve açılış sahneleri, açık taraf; `updates`: alt sistem), `tape.rs`
(görüntüleyici tutarlılık senaryosu, `qp replica-tape --viewer`).

**Ne değildir:** kapsül motoru (cihaz akış için motor koşmaz), çizici, kabuk.
