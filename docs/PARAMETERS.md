# Parametre envanteri (RG1/K2)

Durum: **envanter** (2026-11-22). Kural `docs/ENGINEERING.md` §6, karar K78. Kayıt defterinin
tipleri `qp-schema` içindedir (`qp_schema::params`); tanımlı anahtarların üretilmiş listesi
`docs/PARAMETERS.generated.md`. Bu dosya koddaki davranış sabitlerinin bugünkü yerini,
önerilen anahtarını ve kapsamını sayar. Yerler **crate + öğe yolu** ile verilir (dosya yolu
RG1 taşımalarında değişti).

Kısaltmalar: **Tür** N = adlı `const`, L = satır içi sayı ya da `Default` alanı. **Bağ**
✅ = RG1'de kayıt defterinden okunur hale geldi; — = borç (`docs/DEBT.md`). **CB** = sabit
bağlamda kullanılıyor (dizi boyu, başka `const`, desen) — bağlamak zor; **fmt** = `format!`
içinde adıyla yakalanıyor. Kapsam: sis(tem), plan, kul(lanıcı), enst(rüman), nes(ne);
**H** = satılabilir hak gibi duruyor.

**Bugün olmayanlar:** günlük soru adedi, yapay zekâ kullanım kotası, öneri ömrü (TTL) kodda
yok. Hak gibi duran mevcut değerler: gecikmeli veri dakikası, kişi başına izleme, sonuç
sınırları.

## qp-capsule

| Öğe | Değer | Ne | Anahtar | Kapsam | Tür | Bağ |
|---|---|---|---|---|---|---|
| `realize::LEDGER_MIN_N` | 20 | Karne cümlesi bu n'nin altında "az örnek" | `scorecard.min_n` | sis | N | ✅ |
| `realize` kitleli karne eşikleri | 0,6 / 0,4 | "Çoğunlukla / Yarı yarıya / Nadiren" | `scorecard.redacted.mostly_at`, `.even_at` | sis | L | — |
| `realize::Ago::vague` | 2..3, 4..7, 8..17, 18..26 (günlük), 5..20 | Belirsiz süre kovaları | `sentence.vague_ago.buckets` | sis | L (desen) CB | — |
| `engine::FRESHNESS_HALF_BARS` | 5,0 | İşaret tazelik yarı ömrü (bar); `qp_grammar::depth` kopyası | `glyph.freshness_half_bars` | sis | N | ✅ |
| `engine::glyph_spec` teyit tavanı | 1..5 | İşarette teyit sayısı | `glyph.max_confirmations` | sis | L | — |
| `engine` hikâye doğumu kademesi | 3 | Yeni hikâyenin kademesi | `story.initial_tier` | sis | L | — |
| `theme::GAP_BARS` | 10 | Aynı tür/yön birleştirme toleransı | `story.theme.gap_bars` | sis | N | ✅ |
| `theme::CONFIRM_BARS` | 1 | Akış olgusunun fiyat hikâyesini teyit penceresi | `story.theme.flow_confirm_bars` | sis | N | ✅ |
| `outcome::ATR_FAST` / `ATR_SLOW` | 14 / 100 | Yönlü / nötr bariyer ATR dönemi | `outcome.atr_fast`, `.atr_slow` | sis | N | ✅ |
| `outcome::C_DIR` / `C_NEUTRAL` | 0,3 / 0,65 | Bariyer katsayıları (k = C·√h) | `outcome.barrier.c_dir`, `.c_neutral` | sis | N | ✅ |
| `outcome::HL_S_DIR` / `HL_S_NEU` | 2,0 / 1,3 | Yüksek/düşük dokunma ölçeği | `outcome.highlow.scale_dir`, `.scale_neutral` | sis | N | — |
| `outcome::INTRADAY_MAX_STEP_MS` | 43 200 000 | Bu adım ve üstü günlük sayılır | `outcome.intraday_max_step_ms` | sis | N | ✅ |
| `outcome::base_params` | 20 / 15 / 10 bar | Anlam türüne göre ufuk | `outcome.horizon.<tür>` | sis (enst?) | L | — |
| `rank::Config::default` | 0,2 / 0,5 / 0,3, 30, 2,0, 0,02, 0,69, 0,58, 0,5 | Kademe ve önem sıralaması (9 değer) | `rank.*` | sis | L | — |
| `rank::Confirm::tier` | 2,0 / 1,0 / 0,5 | Teyit kademe kesimleri | `rank.tier.confirm_cuts` | sis | L | — |
| `audience` kademe süzgeci (+ `sentinel::SentinelAudience::tier_ok` kopyası) | Önemli ≤ 1, Dengeli ≤ 2 | Hassasiyet seviyelerinin geçirdiği kademe | `sensitivity.important.max_tier`, `.balanced.max_tier` | sis (seviyeyi kul seçer) | L | — |
| `sentinel::optional_reasons` | kademe 1 | Pozisyon dışı enstrümanda yalnız kademe-1 doğum bildirilir | `sentinel.offposition_max_tier` | sis | L | — |
| `sentinel` tekrar bastırma | 1 gün | Kapsül başına günde bir bildirim | `sentinel.repeat_hold_window` | sis | L (yapısal) | — |
| `feedrule::FEED_RULES` `rate55` | min_n 20, τ 0,55 | Akışta yazı-tura gizleme kuralı | `feed.coinflip.min_n`, `.tau` | sis | N (const dizi) CB | — |
| `feedrule::PRESENTED_FEED_RULE` | "rate55" | Gösterimin akış kuralı | `feed.rule.default` | kul | N | — |
| `market::DELAY_MINUTES` | 15 | "Canlı · gecikmeli N dk" | `market.delay_minutes` | plan **H** | N fmt | — |
| `trust::Params::default` | 5, 5, 2, 2, 1 | Mühür yolu penceresi ve konuşma eşiği | `trust.*` | sis | L | — |
| `trust` makbuz / taahhüt harf sayısı | 5 / 8 | | `trust.receipt_letters`, `.commit_letters` | sis | L | — |
| `voice::{marla,selvi}::MAX_HEADLINE` / `MAX_DETAIL` / `MAX_DETAIL_SENTENCES` / `MAX_LEDGER` | 80 / 200 / 3 / 70 | Ses sınırları (iki kopya) | `voice.max_headline`, `.max_detail`, `.max_detail_sentences`, `.max_ledger` | sis | N | — |
| `voice::selvi::MAX_GLOSS` | 160 | (kullanılmıyor) | `voice.max_gloss` | sis | N | — |
| `voice::marla::HEAD_LEVELS` / `LEDGER_LEVELS` | 6 / 3 | Geri çekilme seviyeleri | — | sis | N CB | — |
| `comment::text_ids` | ≥ 5 harf | Kimlik benzeri parçanın maskelenmesi | `comment.id_mask_min_chars` | sis | L | — |

## qp-session

| Öğe | Değer | Ne | Anahtar | Kapsam | Tür | Bağ |
|---|---|---|---|---|---|---|
| `dispute::MAX_OPEN` / `MAX_PER_CONN` / `MAX_DIGESTS` | 16 / 256 / 4 | İtiraz sınırları | `dispute.max_open_per_conn`, `.max_per_conn`, `.max_digests` | sis | N | ✅ |
| `watch::MAX_WATCHES` | 32 | Kişi başına etkin izleme (`welcome.limits`) | `watch.max_per_person` | plan **H** | N | ✅ |
| `session::default_limits` | 1 MiB, 64 işlem, 10 000 bar, 32 abonelik, 4096, 100 000 | Bağlantı sınırları (PROTOCOL §4.1); `qp_schema::protocol::default_max_bind_bars` kopyası | `protocol.limits.*` | sis (abonelik plan **H**) | L | — |
| `session::MAX_DIRECTIVES` | 16 | Kişi başına dikkat yönergesi | `attention.max_directives` | sis | N | — |
| `plane::DataPlane::new` yorum sıkılığı | `CitesNumbers` | Yorum doğrulayıcı varsayılanı | `comment.strictness.default` | nes | L | — |
| `qp_schema::capsule::UserGroup::default_sensitivity` | alışkın → Hepsi, gelecek → Önemli | Grup başına hassasiyet | `sensitivity.default.<grup>` | kul | L | — |
| `canon::MAX_PANELS` / `MAX_LAYERS` | 16 / 64 | Görünüm sınırları | `view.max_panels`, `.max_layers` | sis | N fmt | — |
| `canon::THRESHOLDS` | rsi 70/30/50, macd 0 | Adlı eşikler | `grammar.threshold.<ad>` | sis/enst | N CB | — |
| `expand::MAX_BOUND_SERIES` | 256 | Belge başına bağlı seri | `doc.max_bound_series` | sis | N fmt | — |
| `claims::MAX_CLAIMS`, `replica::MAX_CLAIMS` | 4096 (iki kopya) | Hatırlanan iddia | `claims.max_remembered` | sis | N | — |
| `capidx::PER_CAPSULE` / `ITEMS_PER_CAPSULE` / `VIEWS` | 4 / 4 / 4 | Kapsül dizini önbelleği | `cache.capsule.*` | sis | N | — |
| `summary::MAX_ITEMS` / `MAX_TEXT` | 50 / 120 | Belge özeti kesme | `summary.max_items`, `.max_text` | sis | N | — |
| `select::MAX_AMBIGUOUS` / `MAX_NOT_FOUND` | 10 / 5 | Hatada aday sayısı | `select.max_ambiguous`, `.max_did_you_mean` | sis | N | — |
| `interact::STORY_ZOOM_MIN` | 20 | Hikâye kümesine dokunuşta en az bar | `gesture.story_zoom_min_bars` | sis | N | ✅ |
| `interact::FACE_HEIGHT` | 140 | Akış yüzü yüksekliği | `face.default_height` | sis | N | — |
| `viewer::DEFAULT_LIMIT` / `MAX_LIMIT` | 40 / 200 | Görüntüleyici akış uzunluğu | `viewer.limit.default`, `.max` | sis/plan | N | — |
| `viewer::DEFAULT_FACE_WIDTH`, genişlik aralığı | 390, 64..=4096 | | `viewer.face_width`, `.width.range` | sis | N/L | — |
| `reading::HIT_WIDTH` / `HIT_HEIGHT`; yüzey başına boy | 1200 / 600; telefon 390×300, saat 200×242 | Nokta sorgusu çerçevesi | `hit.*` | sis | N/L | — |
| `present::profile_for` | yüzey başına derinlik / nokta / işaret | Sunum profili | `surface.<tür>.*` | sis | L (tablo) | — |
| `query` varsayılan görünür bar | telefon 60, saat 20, diğer 120 | | `view.default_bars.<yüzey>` | sis | L | — |
| `replayhost::DEFAULT_INSTRUMENTS` / `DEFAULT_DAYS` / `DEFAULT_HISTORY` | 30 / 260 / 120 (`qp_wasm`/`qp_shellcore::shell::DemoOptions` ve `qp_source::ReplayOptions` kopyaları) | Yeniden oynatma varsayılanları | `replay.default.*` | sis | N | — |
| `replayhost::DEFAULT_SPEED` / `MAX_SPEED` / `MAX_STEP_N` / `MAX_STEPS_PER_PUMP` | 4,0 / 1000 / 100 000 / 2000 | | `replay.speed.default` (kul), `.max`, `replay.step.max_n`, `replay.max_steps_per_pump` | sis | N (fmt: MAX_SPEED, MAX_STEP_N) | — |

## qp-grammar

| Öğe | Değer | Ne | Anahtar | Kapsam | Tür | Bağ |
|---|---|---|---|---|---|---|
| `depth::GLYPH_MIN_LEDGER_N` | 20 | İşaret güven dolgusu bu n'den (≤ akış kuralı min_n; test) | `glyph.min_ledger_n` | sis | N | ✅ |
| `depth::FRESHNESS_HALF_BARS` | 5,0 | (capsule kopyası) | `glyph.freshness_half_bars` | sis | N | ✅ |
| `depth::CHART_LOOKBACK` | 30 | Grafik derinliğinde olgu öncesi bar | `depth.chart.lookback_bars` | sis | N | ✅ |
| `compile::MIN_WIDTH` / `MIN_HEIGHT` / `MAX_DIM` / `MIN_PANEL_H` / `MAX_TICKS` | 200 / 120 / 16384 / 10 / 64 | Derleme sınırları | `render.size.*`, `layout.min_panel_h`, `axis.max_ticks` | sis | N | — |
| `compile` LOD | piksel başına 1 bar | LOD eşiği | `lod.max_bars_per_px` | sis | L | — |
| `feeds::vakiro::WEEK_DAYS` | 7 | | `feed.vakiro.week_days` | sis | N | — |
| `feeds::PRESENTED_FEED_ORDER` ve düzen | live_new / r3 | Akış varsayılanları | `feed.order.default`, `feed.layout.default` | kul | N CB | — |
| `glyph::outcome::LEGEND_MAX_WORDS`, `motion::MAX_SPOTS` | 30 / 4 | | `glyph.legend.max_words`, `motion.max_spots` | sis | N | — |
| `faces::*::MAX_CHARS`, `fotolay::BEFORE`/`MIN_BARS`, `zemre::DEFAULT_HORIZON` | 80, 60/80, 20 | Yüz tasarımları | `face.<ad>.*` | sis | N | — |
| `transitions::*` süreleri | 300–4000 ms | Geçiş süreleri (görünüşe yakın) | `motion.*` | sis | N | — |

Görünüş kayıtları (`look::{R4, ORNEK, OLTAN, ZEMRE}`, `stories::pavka::*`, `stories::korda::*`,
`theme::*`) tasarım verisidir; parametre değil, görünüş kaydı olarak kalır.

## qp-facts (dedektör varsayılanları; enstrüman kapsamına aday)

`VolumeSurge::WINDOW`/`MULT` 20 / 2,5 → `fact.volume_surge.*`; `RangeBreakout::WINDOW`/`ATR`/
`HOLD_BARS` 20 / 14 / 3 → `fact.breakout.*`; `breakout_seasonal::HOLD_BARS` 3; `MaStack::FAST`/
`SLOW`/`ATR`/`RISE` 20 / 50 / 14 / 5 → `fact.ma_stack.*`; `RsiDivergence::*` 14 / 2 / 5 / 40 / 20
→ `fact.rsi_div.*`; `RsiExtreme::RSI`/`UPPER`/`LOWER`/`FULL_STRENGTH` 14 / 70 / 30 / 30 →
`fact.rsi_extreme.*`; `BandSqueeze::*` 20 / 2,0 / 120 / 1,5 / 20 → `fact.squeeze.*`;
`InstitutionalFlow::WINDOW`/`MULT` 5 / 2,0; `SEASON_DAYS`/`SEASON_MIN_DAYS`/`SEASON_MIN_LEN_MS`
20 / 5 / 3 600 000 → `fact.season.*`. Hepsi N, bağ —.

## qp-compute

`MAX_PERIOD` 100 000 → `study.max_period` (fmt); `address::MAX_DEPTH` 64 → `study.max_depth`
(fmt); `late::EMA_LOOKBACK_PER_PERIOD`/`RSI_LOOKBACK_PER_PERIOD`/`MIN_LOOKBACK` 24 / 48 / 64 →
`study.lookback.*`; `sweep::share::EMA_CACHE_BYTES` 16 MiB → `cache.ema_bytes`. SIMD ayarları
(`MAX_LANES`, `SOA_LANES` …) ve kıyas profilleri dışarıda.

## qp-mcp

`INBOX_CAP` 10 000 → `mcp.inbox_cap` (sis/plan); `LIST_TTL_MS` 300 000 → `mcp.list_ttl_ms`;
`surfaces::mixed::DEFAULT_LIMIT`/`MAX_LIMIT`/`LOG_LIMIT` 8 / 50 / 8, `surfaces::task::
DEFAULT_LIMIT`/`MAX_LIMIT` 10 / 50 → `mcp.<yüzey>.limit.*` (plan **H**); `surfaces::query` bul
varsayılanları (20, 50, 32, 10; tavan 100) L; görüntü boyu `MAX_SIDE`/`MIN_SIDE` 4096 / 64 (fmt).

## qp-schema

`protocol::story::STORIES_DEFAULT_LIMIT` / `STORIES_MAX_LIMIT` 200 / 500 → `stories.limit.*` (fmt).

## qp-live, qp-matriks, qp-source (bağlantı ve akış)

`qp_live::backoff::BackoffPolicy::default` (2 s, ×2, 60 s, 6, 60 s) → `live.backoff.*`;
`qp_live::auth::AuthPolicy::default` (300 s, 30 s, 15 dk, 5, 3600 s, 10 s) → `live.auth.*`;
`qp_live::config::MqttSettings::default` (30 s, 10 s) → `live.mqtt.*`; bağlantı döngüsü (200 ms,
2 s, 1); `qp_live::halt::DEFAULT_SERVER_CLOSE_LIMIT`/`WINDOW` 2 / 600 s; `qp_live::rest::MIN_GAP`
700 ms, `TRADE_RETRIES` [5 s, 20 s] (CB), zaman aşımı 60 s; `qp_live::mqtt::MAX_PACKET` 16 MiB;
`qp_matriks::feed::DEFAULT_FINALIZE_GRACE_MS` 2000 (clap varsayılanı da), `MAX_LEAD_MS` 60 000,
işlem tekilleştirme penceresi 30 dk, bar aralıkları 1m/5m/1h/1d (sis/enst), borsa farkı 180 dk
(enst — K74: ülkeye özgü değer parametreye); `qp_source::livebar::LiveConfig::default` seans
öncesi/sonrası 30 dk (enst); `qp_source::MAX_FILE_BYTES` 256 MiB (fmt); sentetik piyasa modeli
(`flow::INSTITUTIONAL_PROB`, `regimes::*`).

## qp-render

`retained::LAYER_MIN_PRIMS` / `FULL_SHARE_PERMILLE` 16 / 600; `DEFAULT_WIDTH`/`HEIGHT` 1200 / 600,
`cpu::MAX_DIM` 16384, `cpu::MAX_TEXT_PX` 512 → `render.*`.

## Barındırma ve kabuk (`qp-host`, `qp-shellcore`, RG1'de `qp-cli`/`qp-wasm`'dan taşındı)

`sim::CLOSED_PAUSE` 2 s; `demo::VIEWER_FLUSH_MS` 250; `livehost::RestTradeHistory::
MAX_WINDOW_DAYS` 15; `livehost::MAX_SAMPLES` 200 000; HTTP MCP zaman aşımları 120 s / 15 s.
`shell::FEED_LIMIT` 40, `TAP_SLOP` 8 px, `WHEEL_FACTOR` 1,25, `LONG_FACTOR`/`VERY_LONG_FACTOR`
1,5 / 2,5, `REFRESH_MIN_SAMPLES` 20, `OPEN_PLAUSIBLE_MS` 60 000, `FACE_HEIGHT` 240 → `ui.*`
(kullanıcı kapsamına aday).

## Tek anahtarda birleşmesi gereken kopyalar

- Karne "20": `qp_capsule::realize::LEDGER_MIN_N`, `qp_grammar::depth::GLYPH_MIN_LEDGER_N`,
  `FEED_RULES` rate55 `min_n` (RG1'de iki ayrı anahtar: `scorecard.min_n`, `glyph.min_ledger_n`;
  birleştirme anlam kararıdır, ana akışa).
- Tazelik yarı ömrü (capsule + grammar): RG1'de tek anahtar `glyph.freshness_half_bars`.
- Ses sınırları (`marla`, `selvi`); iddia tavanı (`claims`, `replica`); hassasiyet kesimleri
  (`audience`, `sentinel`); yeniden oynatma varsayılanları (üç yer); `max_bind_bars` (iki yer).

## RL1 — hesap dili (yeni anahtarlar)

Şartname `docs/PROGRAMS.md` §8, `docs/PROGRAMS-SESSION.md` §2.3. Bu anahtarlar koddaki bir sabitin envanteri değil, RL1'de
**doğrudan kayıt defterine** (`qp_schema::params::system::REGISTRY`) girecek tanımlardır;
koda gömülü karşılıkları olmaz. Biçim üretilmiş tablonunkidir (`docs/PARAMETERS.generated.md`).
Alt sistem değeri `SessionView::param(key)` ile okur; `None` ise varsayılan geçerlidir.
Kişi başına (plan) değer bu round'da çözülemez (`docs/PROGRAMS.md` Açık sorular 1). `program.live.doc.*`
kapsamı doküman içinde sahip başınadır; dokümanlar arası kişi başına sayım RS1'in işidir.

| Key | Kind | Unit | Range | Default | Scopes | Changed by | Entitlement | Description | Since |
|---|---|---|---|---|---|---|---|---|---|
| `program.max_nodes` | int | count | 1.. | 200 | system | release only | no | Nodes per program at most (`E_TOO_MANY_NODES`). | RL1 |
| `program.max_window` | int | bars | 1..100000 | 2000 | system | release only | no | Largest `period`, `n` or `bars` in a program (`E_WINDOW_LIMIT`); at most `study.max_period`. | RL1 |
| `program.max_outputs` | int | count | 1.. | 4 | system | release only | no | Outputs per program at most (`E_TOO_LARGE`). | RL1 |
| `program.max_params` | int | count | 0.. | 16 | system | release only | no | Parameters per program at most (`E_TOO_LARGE`). | RL1 |
| `program.max_ir_bytes` | int | bytes | 256.. | 8192 | system | release only | no | Canonical IR size at most; bounds log growth (`E_TOO_LARGE`). | RL1 |
| `program.max_cost_per_bar` | int | unit/bar | 1.. | 666 | system | release only | no | Static cost per bar of one program at most (`E_COST_LIMIT`); ≈ RX4's 2 000 000 units for 3000 bars. | RL1 |
| `program.signal.max_horizon_bars` | int | bars | 1.. | 500 | system | release only | no | Largest `max_bars` of a signal outcome rule (`E_BAD_RULE`). | RL1 |
| `program.check.max_errors` | int | count | 1.. | 50 | system | release only | no | Errors returned by one check at most; the rest is `truncated`. | RL1 |
| `program.live.doc.max_instances` | int | count | 0.. | 32 | system, plan | admin | yes | Per document, per owner: program instances at most (`E_BUDGET`). | RL1 |
| `program.live.doc.max_cost_per_bar` | int | unit/bar | 0.. | 2000 | system, plan | admin | yes | Per document, per owner: sum of the live cost per bar of the owner's instances at most (`E_BUDGET`). | RL1 |
| `program.live.doc.max_log_bytes` | int | bytes | 0.. | 4194304 | system, plan | admin | yes | Per document, per owner: bytes of program changes in the log at most; bounds log growth by repeated edits (`E_BUDGET`). | RL1 |
| `program.try.max_bars` | int | bars | 1.. | 20000 | system, plan | admin | yes | Closed bars one try or replay run reads at most (the latest ones). | RL1 |
| `program.try.max_work` | int | unit | 1.. | 50000000 | system, plan | admin | yes | Work of one try or replay run at most; the run stops with `stopped: budget`. | RL1 |
| `program.try.max_items` | int | count | 1..100 | 10 | system | release only | no | Values, bars or records listed per output in a try summary. | RL1 |
| `program.work.record` | int | unit | 0.. | 16 | system | release only | no | Work charged per emitted record (event bar, signal record, state key). | RL1 |
| `program.work.open_record` | int | unit | 0.. | 1 | system | release only | no | Work charged per open signal record per bar. | RL1 |
| `program.work.read` | int | unit | 0.. | 1 | system | release only | no | Work charged per output cursor read. | RL1 |
| `program.text.max_source_chars` | int | chars | 1.. | 65536 | system | release only | no | Characters of a program text at most (`E_TOO_LARGE`); eight times `program.max_ir_bytes` for comments and spelling. | RL1 |
| `program.text.max_token_chars` | int | chars | 1.. | 1024 | system | release only | no | Characters of one token (name, number, string) of a program text at most (`E_TOO_LARGE`). | RL1 |
| `program.text.max_depth` | int | count | 1.. | 64 | system | release only | no | Expression nesting of a program text at most, parentheses included (`E_TOO_LARGE`). | RL1 |
| `program.max_ops_per_command` | int | count | 1..64 | 8 | system | release only | no | Program operations (`add_program`, `set_program_params`, `remove_program`) in one command at most (`E_TOO_LARGE`). | RL1 |

Dil sabitleri (kutu kenarı 1…8, durum boyutu 1…4, düğüm ağırlıkları) parametre değildir;
değişmeleri dil sürümünü değiştirir (`docs/PROGRAMS.md` §8.4).
