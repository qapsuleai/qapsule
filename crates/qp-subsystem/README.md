# qp-subsystem

**Alt sistem sözleşmesi** (`docs/CONCEPTS.md`; `rounds/RF1/PLAN.md` §4, §11 D1/D3): otorite
(`qp-authority`) ile ona kayıtla takılan alt sistemler arasındaki tek arayüz. Otorite alt
sistemleri tanımaz; bir `Registry`, alt sistemlerin durumu (`Slots`) ve bir `Outbox` tutar,
kayıtlı alt sistemleri adlı, sabit fazlarda (`Phase`) çağırır.

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada `qp-ops`'tan sonra,
alt sistemlerden ve `qp-authority`'den önce (`qp-plane < qp-ops < qp-subsystem < qp-watch <
qp-authority < qp-session`).

**Bağımlılıklar:** `qp-schema`, `qp-plane` (veri düzlemi, değer önbelleği, hata kurucusu,
okuyucu izdüşümü `Proj`), `qp-ops` (işlem açılımı: `Ctx`, `expand_op`, `expand_ops_with`),
`qp-capsule` (`StoryEvent`: motorun bakışı); `serde`, `serde_json` (tel etiketleri).

**Dışa açık yüzey (`lib.rs`):**

- `Subsystem`: durumsuz davranış nesnesi (`&'static dyn`); her şeyin varsayılanı var.
- `SessionView` (dar okuma görünümü: doküman, okuma dokümanı, günlük, saat, kimlik,
  bağlantı `ConnView`, bağlantı listesi ve abonelikleri, okuyucunun dünyası `public_view` ve
  izdüşümü `reader_proj`, veri düzlemi, piyasa, dünya, `entitled`, `param` K78, `run_kind`
  `RunKind`, yeniden oynatma konumu), `SessionCtx { view, slots, out }` (bölünmüş ödünç).
- `Slots` (`TypeId` → kutu; tek, `PerWorld<T>`, bağlantı anahtarlı), `PerWorld`.
- `Outbox` (motor girdisi `EngineEntry`, bildirim, uyandırma; fazın kendi satırında alınanlar:
  bakışın girdisine değişim `amend`, o satırda bildirim `say`). Yazma sırası derinlik
  önceliklidir; istenen girdiler en çok `MAX_EMIT_DEPTH` düzey derine yazılır.
- `Phase` (`Check`, `Expand`, `AfterCommit`, `AfterData`, `Proposals`, `Subscribe`,
  `Unsubscribe`, `Connect`, `Close`, `Settled`, `Adjust`), `Moment` (fazın konusu;
  `AfterData` için `Data`, `Look`, `Written`). `Close`'un çağrı noktası yok: `qp/0`'da
  kapanış iletisi yok; gelince otorite fazı, sonra `Slots::drop_conn`'u çağırır.
- Kayıt kolları: `OpHandler` (tür `owns`, açılım + politika: `allowed`, `proposable`, `agent`
  `AgentOp`, `to_public`, `reader_private`), `QueryHandler`, `InputHandler`, `TopicHandler`,
  `ConditionKind` (canlı yol), `ValueSource` (D3), `ClaimKind` (D1).
- `Registry`: kayıt sırasıyla alt sistemler ve tel etiketine göre tablolar; boşken her arama
  ıskalar, hiçbir faz çağrılmaz; `evaluates_conditions` (canlı yolun belleği atlanır).
  `op_name` (işlemin tel adı).

**Ne değildir:** otorite değildir (komut yolu, günlük, iki dünya `qp-authority`'de); bir alt
sistem değildir; işlemlerin kendisi değildir (`qp-ops`). Bileşim (`qp-session`) değildir.

**Kurallar:** `dyn` trait'lerinde jenerik yöntem yok; alt sistem yalnız `Outbox` ya da açılım
üstünden yazar; kayıt sırası belirlenimcidir (`BTreeMap`, kayıt sırası).
