# qp-program

**Programlar oturumda** (RL1; `docs/PROGRAMS-SESSION.md` §1–§3, kavramlar `docs/CONCEPTS.md`
"Hesap dili (RL1)"): hesap dilinin (`qp-ir/0`, `qp-lang`) programlarını dokümana koyan alt
sistem. Otoriteyi tanımaz; yalnız `qp-subsystem` sözleşmesi (`docs/ARCHITECTURE.md` "Yeni alt
sistem nasıl eklenir"). Kayıt: `qp-session` `compose.rs` `LIST` (izleyici listesinde yok:
izleyici program serilerini veri düzlemiyle hesaplar).

**Katman:** oturum; sırada `qp-watch`'tan sonra, `qp-read` ve `qp-authority`'den önce.

**Bağımlılıklar:** `qp-subsystem`, `qp-ops` (açılım bağlamı, yerleşik `remove`/`insert_layer`/
`insert_panel` açılımı, `apply_all`), `qp-plane` (hata kurucusu, bar serisi, veri kapısı),
`qp-schema`, `qp-lang` (denetim, sınırlar, bütçe, `try_program`, çalıştırıcı), `qp-lang-text`
(metin → ara gösterim, kaynak haritası, kanonik yazıcı), `serde_json`. Yeni dış bağımlılık yok.

**Dışa açık yüzey:** `PROGRAM` (`Programs`): `Subsystem`.

- İşlemler (`OpHandler`): `add_program` → çıktı başına `put_series` (`<örnek>.<çıktı>`,
  `study: "program"`, `program: ProgramRef` satır içi kanonik ara gösterimle; `since_t` açılım
  saati, `owner` kabul edenin ya da konuşanın adına çalıştığı kişinin ilke kimliği, `person_of`), `show` ile çıktı başına katman
  (`series` → `line`, `event` → `bar`; etiket seri kimliği, lejant onu yazar);
  `set_program_params` → aynı seriler yeni değer, `instance_id`, `since_t` ile;
  `remove_program` → çıktı başına `remove` kuralı (`in_use`). Sinyal/durum çıktısı `E_TRY_ONLY`.
- Politika: hepsi `write`; ajan `add_program` (yalnız yeni örnek) ve `set_program_params`'ı
  yalnız önerir, `remove_program` asla. Kamu dünyası ve okuyucu izdüşümü her hesap serisiyle
  aynı kuraldan geçer (varsayılan `to_public`).
- `Check` fazı: program serisine `put_series`/`set_params`/`remove` → `E_PROGRAM_SERIES`;
  olay serisi `compare` işleneni (`add_intent`, `add_finding`, `update_finding`) →
  `E_EVENT_OPERAND` (kabul edilen önerinin işlemlerine de); komutun program işlemleri, kabul
  edilen önerinin işlemleri (karar veren sahip) ya da program serisi geri getirebilen `undo`
  bir kopyada barındırıcının sınırlarıyla (`SessionView::param`) açılır: program sınırları ve
  yazılan/geri gelen her sahibin canlı bütçesi (`program.live.doc.*`). Günlük baytı her
  açılım yolunda o dünyanın kendi günlüğünden sayılır: günlük başına bir kez, sonra yalnız
  eklenen girdiler (`budget::logged`, saf önbellek; aynı günlük aynı cevap). Kabulde açılımın yazacağı sahip (karar verenin
  yazar adı ilke kimliği sayılır) karar veren bağlantının kişisi değilse `owner_unresolved`.
- Sorgular (`QueryHandler`): `check_program` (ara gösterim ya da metin; metin hataları `span`
  taşır; `program.text.*` sınırları), `try_program` (okuyucunun kapanmış barları, veri yetkisi
  kapısı), `explain_program` (kanonik metin, düğüm başına maliyet, `at` ile köken).

**Düzen (`src/`):** `subsystem` (kayıt), `ops` (işlem ve politika), `expand` (açılım), `show`
(katmanlar), `budget` (sahip başına kullanım), `check` (`Check` fazı), `guard` (seri
korumaları), `query` + `asked` (sorgular), `inst` (dokümandaki örnekler), `refuse` (program
hataları → protokol hatası, hepsi birden).

**Kurallar:** hesap kodunda model, ağ, saat, rastgelelik yok; sınırlar yalnız kayıt
defterinden (`qp_lang::Limits`). Açılım `qp_ops::Ctx`'te `param` olmadığından kayıt defteri
varsayılanlarıyla denetler; barındırıcı değeri `Check` fazında. Eksik kancalar: açılımda
`param_for` (PS §7/3); açılım bağlamında karar verenin ilke kimliği (`Ctx::approved_by` yalnız
`Author`); açılımın alt sistem durumuna erişimi (günlük baytı önbelleği bu yüzden `thread_local`); komut
başına program işlemi tavanı için kayıt defterinde anahtar yok.

**Ne değildir:** program serilerinin değerlerini hesaplamaz (`qp-plane`); otorite değildir;
yeni `ChangeOp`, koşul türü ya da koleksiyon eklemez.
