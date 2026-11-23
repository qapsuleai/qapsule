# qp-plane

**Veri düzlemi** (`docs/PROTOCOL.md` §18) ve otoritenin okuyucu başına tuttuğu saf dizinler.
Düzlem (`plane`: bağlar, bağlı barlar, okuma dokümanı, sağlayıcı), değer önbelleği
(`values`: bar sütunları, çalışma çıktıları, koşul değerlendirmesi), protokol hatası kurucusu
(`error`, `suggest`: `did_you_mean`) ve saf kapsül dizinleri: okuyucu izdüşümü (`proj`),
kimlik dizini, cümle ve öğe belleği (`capidx`), cümle girdisi ve geçerlilik (`sentences`),
başlık yapı taşları (`headparts`). RF1 S2'de `qp-session`'dan saf taşımayla ayrıldı
(`rounds/RF1/PLAN.md` §3.3, §11 D2).

**Katman:** oturum (`docs/ENGINEERING.md` §2); katman içi sırada en altta, `qp-ops`'tan önce.

**Bağımlılıklar:** `qp-schema`, `qp-source`, `qp-compute`, `qp-calendar`, `qp-capsule`,
`qp-voice`, `qp-grammar` (`HeadlineParts`, `GLYPH_MIN_LEDGER_N`); `serde`, `serde_json`.

**Dışa açık yüzey (`lib.rs`):** modüller `plane`, `values`, `error`, `suggest`, `proj`,
`capidx`, `sentences`, `headparts`; kökte `DataError`, `DataOutcome`, `HostNotice`,
`did_you_mean`, `edit_distance`.

**Düzen (`src/`):** her modül bir dosya; `capidx/` kapsül öğe belleği (`items`), `values/`
serinin değerleri (`computed`: `Computed`).

**Ne değildir:** oturum değildir (`Session`'ı tanımaz); işlem açmaz (`qp-ops`); dış veri
kaynağı değildir (sağlayıcılar `qp-source`'ta); kapsül motoru değildir (`qp-capsule`).
