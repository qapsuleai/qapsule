# qp-watch

**İzleme niyeti** (`docs/CONCEPTS.md`; `docs/PROTOCOL.md` §25.5): otoritenin ilk gerçek alt
sistemi (`rounds/RF1/PLAN.md` §3.3 satır 3, RF1 S3.1). Kişinin hikâye geçişleri üstüne
bıraktığı yapılı niyetler; motor onları belirlenimci eşler.

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada `qp-subsystem`'den sonra,
`qp-authority`'den önce (`qp-plane < qp-ops < qp-subsystem < qp-watch < qp-authority <
qp-session`). Otoriteyi tanımaz; otorite de onu tanımaz. Bileşim (`qp-session`) bir kayıt
satırıyla takar.

**Bağımlılıklar:** `qp-subsystem` (sözleşme), `qp-ops` (`Ctx`, izlemenin ortak saf kuralları
`qp_ops::watch`: `on`, `watch_visible`, `person_of`, `MAX_WATCHES`), `qp-plane` (hata kurucusu),
`qp-capsule` (`StoryEvent`), `qp-schema`.

**Dışa açık yüzey (`lib.rs`):** `Watch` (alt sistem), `WATCH` (bileşimin kaydettiği değer).

**Kancalar:**

- İşlem kaydı (`OpHandler`): `add_watch`, `remove_watch` — açılım (`ops.rs`, `qp-ops`'tan
  taşındı) ve politika: okuyabilen yazar (`allowed`), öneri olamaz (`proposable`), ajan doğrudan
  yazar (`agent`), başkasının isteği okuyucuya gitmez (`reader_private`), kamu dünyasına aynen.
- `AfterData` fazı, `Moment::Look` (her dünyada bir kez): etkin izlemeler motorun bakışındaki
  hikâye geçişleriyle eşlenir; sayı ve durum değişimleri aynı motor girdisine girer
  (`Outbox::amend`). `Moment::Written`: girdi yazıldı; `intent_matched` olayları o satırda
  gider (`Outbox::say`).
- Yuva: `Pending` (bakış ile yazılış arasındaki eşleşmeler).

**Ne değildir:** dokümandaki izleme öğelerinin gizliliği (başkasının izlemesini kim okur)
değildir: o, dokümanın izdüşümü olarak otoritede kalır (her dokümanda, bu alt sistem kayıtlı
olsun olmasın, geçerli). Koşullu niyetlerin (`add_intent`, tetik) değerlendirmesi değildir:
canlı yol, çekirdekte.

**Düzen:** `src/lib.rs` (bildirim), `subsystem.rs` (alt sistem, faz), `ops.rs` (işlemler),
`matching.rs` (eşleme, olay satırları).
