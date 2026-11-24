# qp-replayhost

**Yeniden oynatma barındırıcısı** (`docs/PROTOCOL.md` §21; R4/T2, K44): bir otoriteyi
yeniden oynatma kaynağı (`qp_source::Replay`) üstünde koşturan barındırıcı. Otoritenin
veri düzlemini yeniden oynatmanın ilk çubuklarından besler, her adımda sanal saati
(`host_clock`) ve adımın veri olaylarını (`host_data_event`) verir, `replay` iletisini
(hız, duraklat, sürdür, adım, atla, durum) yanıtlar, kalan her iletiyi otoriteye aynen geçirir.
Geri gitmek otoriteyi init'inden yeniden kurar (`epoch + 1`).

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada `qp-authority`'den sonra,
`qp-session` bileşiminden önce (RF1 S3.3, `rounds/RF1/PLAN.md` §3.3 satır 9). Otoritenin
**istemcisidir**: oturumlarını kendisine verilen alt sistem listesiyle kurar
(`Subsystems`; bileşimin `qp_session::ReplayHost`'u standart listeyi verir); otoriteye yalnız
dışa açık ve birkaç gizli (`#[doc(hidden)]`) yöntemle ulaşır (`seq_to`,
`conn_agent_origin`; `rounds/RF1/experiments/paths.md` S3.3). Otorite onu tanımaz.

**Bağımlılıklar:** `qp-authority` (otorite), `qp-subsystem` (`SessionView::conn`),
`qp-source` (yeniden oynatma, beslemeler), `qp-plane` (hata kurucusu, yeniden oynatma
takvimi), `qp-schema`, `serde_json`.

**Dışa açık yüzey (`lib.rs`):** `ReplayHost`, `demo_init`, `interval_ms`, varsayılanlar
(`DEFAULT_*`, `MAX_*`). Çağıranlar `qp_session::replayhost::*` ve `qp_session::{ReplayHost,
demo_init}` yollarını kullanır (bileşim aynı adlarla yeniden dışa aktarır).

**Düzen:** `src/lib.rs` (bildirim), `replayhost.rs` (barındırıcı) ve `replayhost/`
(`setup`: beslemeler, genişletilmiş init, pencere, canlı; `control`: iletiler ve oynatma
denetimi; `recorded`: teslim edilen veri olaylarının kaydı). `tests/headparts.rs`: başlık yapı
taşları, demo yeniden oynatması üstünden.

**Ne değildir:** kaynak sağlayıcı (beslemeler `qp-source`'tan), kabuk, uygunluk koşucusu
(`conform`, bileşimde).
