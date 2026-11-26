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
`qp-voice`, `qp-grammar` (`HeadlineParts`, `GLYPH_MIN_LEDGER_N`), `qp-lang` (RL1: program
serilerinin çalıştırıcısı); `serde`, `serde_json`. Yalnız sınamada: `qp-session` (canlı oturumda
program olay serisi ve `threshold`).

**Dışa açık yüzey (`lib.rs`):** modüller `plane`, `values`, `error`, `suggest`, `proj`,
`capidx`, `sentences`, `headparts`; kökte `DataError`, `DataOutcome`, `HostNotice`,
`did_you_mean`, `edit_distance`.

**Düzen (`src/`):** her modül bir dosya; `capidx/` kapsül öğe belleği (`items`), `values/`
serinin değerleri (`computed`: `Computed`), akışkan durum ve artımlı güncelleme (`stream`),
program serileri (`program`).

**Program serileri (RL1, `docs/PROGRAMS-SESSION.md` §1.2–§1.3):** `program` taşıyan
`SeriesSource::Study` `qp-lang` çalıştırıcısıyla hesaplanır: saklı ara gösterim kayıt defteri
sınırlarıyla yeniden denetlenir (`Limits::default()`, bağlı değerler, girdinin periyodu), hiza
girdinin takviminden, yalnız adı verilen çıktının konisi koşar. Değer önbelleği, bağımlılık
kapanışı (`inputs`), yetki ve kanıt `add_study` serileriyle aynı yoldan. Olay serisi: kapanmış
barda 1/0, `known_t ≤ since_t` barlarda 0, canlı barda `null` (K13: tam uzunluk);
`Computed::len()` olay serisinde kapanmış bar sayısıdır, koşullar son kapanmış barı okur. Artımlı:
yeni bar `push`, canlı bar düzeltmesi `update_last`, öbürü baştan kurma; baştan kurmayla bit eşit.
Tik maliyeti `examples/tick_bench.rs` (`rounds/RL1/experiments/wasm/TICK.md`).

**Ne değildir:** oturum değildir (`Session`'ı tanımaz); işlem açmaz (`qp-ops`); dış veri
kaynağı değildir (sağlayıcılar `qp-source`'ta); kapsül motoru değildir (`qp-capsule`).
