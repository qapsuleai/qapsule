# qp-replica

**İstemci yansısı** (`docs/PROTOCOL.md` §19; RP2/T8): otoritenin dokümanını `document` +
`change` olaylarından birebir izleyen, bağlı serileri kendi sağlayıcısından ve kendi yetkisiyle
çapadan çözen, gösterim kuralını (§19.4) kendi verisinde uygulayan taraf (`Replica`), ve
otorite ↔ yansı tutarlılık senaryosu (`tape`, `qp replica-tape`).

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada `qp-authority`'den sonra,
`qp-viewer`'dan önce (RF1 S3.3, `rounds/RF1/PLAN.md` §3.3 satır 6). Otoritenin
**istemcisidir**: izleyici otoritesini kendisine verilen listeyle kurar (bileşimin `FOLLOWER`'ı)
ve onu yalnız otoritenin gizli dar yöntemleriyle değiştirir (`build_follower`, `follow_start`,
`take_provider`, `mirror_apply`, `mirror_commit`, `follow_data`, `follow_resync`,
`eval_parts`; `rounds/RF1/experiments/paths.md` S3.3). Bu yöntemlerin otorite dışındaki tek
çağıranı `replica/follower.rs` içindeki `Follower` tipidir (RL1/Ö2, A71); başka her dosyadan
çağrı `scripts/arch-check` kuralı `calls.restricted` ile hatadır. Otorite onu tanımaz.

**Bağımlılıklar:** `qp-authority`, `qp-subsystem` (`SessionView`: okuma dokümanı), `qp-ops`
(özetler `digest`), `qp-plane`, `qp-capsule`, `qp-designs`, `qp-compute` (senaryonun bağımsız
kâhini), `qp-source`, `qp-schema`, `serde`, `serde_json`.

**Dışa açık yüzey (`lib.rs`):** `Replica`, `ReplicaOptions`, `DisplayItem`, `Shown`,
`document_digest`, `view_digest`, `fnv1a64`; gizli `tape`. Çağıranlar `qp_session::replica::*`,
`qp_session::{Replica, ReplicaOptions, document_digest, view_digest}` yollarını kullanır.

**Düzen:** `src/lib.rs` (bildirim), `replica.rs` (yansı) ve `replica/` (`follow`, `follower`, `judge`),
`tape.rs` (tutarlılık senaryosu).

**Ne değildir:** otorite değildir; motor girdisi yazmaz; görüntüleyici değildir (`qp-viewer`).
