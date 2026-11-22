# RG1 — Bölme planı

Durum: **v2** (iki bağımsız eleştiriden sonra; §12 eleştiriler ve cevaplar, §13 v2 değişiklikleri —
§13 §1–§11 ile çeliştiğinde §13 geçerlidir). Kod bu plan kabul edilmeden taşınmaz.

Girdi: başlangıç tutanağı (`experiments/baseline/`), `scripts/arch-check` ilk koşusu
(`experiments/arch-first-run.txt`), `qp-session` modül grafiği (§4).

## 1. İlk ölçüm (hedef haritası)

| Kural | İlk koşu | Round sonu hedefi |
|---|---|---|
| `layer.up` (yukarı bağımlılık) | 2: `qp-render → qp-source`, `qp-session → qp-source` | **0** |
| `layer.lateral` (aynı katman) | 3: `qp-capsule → qp-facts`, `qp-cli → qp-wasm`, `qp-matriks → qp-source` | taban çizgisi + istisna isteği (§3) |
| `file.hard` (> 800 satır, `src/`) | 56 dosya | **0** |
| `file.soft` (600–800) | 17 | taban çizgisi (büyüyebilir: bölünen dosyaların parçaları) |
| `fn.lines` (> 80, test dışı) | 177 | taban çizgisi (bu round'da fonksiyon bölünmez) |
| `dir.files` (> 15) | 5 (`qp-session/src` 37, `qp-grammar/src` 21, `qp-session/tests` 39, `qp-wasm/tests` 20, `qp-compute/tests` 16) | `src/` **0**; `tests/` kesilebilir (kesme sırası 2) |
| `crate.lines` (> 12 bin, test dışı) | 3: `qp-session` 31,6 bin, `qp-grammar` 20,0 bin, `qp-capsule` 13,1 bin | saf taşımayla bölünebilen bölünür; kalan istisna isteği (§4, §7) |
| `shell.lines` (> 4 bin) | `qp-cli` 11,7 bin, `qp-wasm` 10,7 bin | **< 4 bin** (§5) |
| `name.forbidden` | 8 (`common`) | **0** |
| `readme.crate` | 12 | **0** |
| `readme.module` | 17 | yeni açılan klasörlerde 0; eskiler taban çizgisi |
| `root.logic` (`lib.rs`/`mod.rs`'te mantık) | 15 | **0** |
| `doc.size` | `DECISIONS.md`, `docs/PROTOCOL.md`, `docs/matriks/INDICATORS.md` | `DECISIONS.md` bölünür; öbür ikisi taban çizgisi |

Sayım kuralları (`scripts/arch-check` başlığı): dosya sınırı ham satır (VERIFY 7 `wc -l` ile
aynı); crate ve kabuk bütçesi `src/` altındaki **test dışı** satır (`#[cfg(test)]` bölgeleri,
`tests.rs` ve `tests/` modülleri hariç); fonksiyon sınırı yalnız test dışı kod. Katman yönü
normal ve derleme bağımlılıkları üstünden; geliştirme bağımlılıkları (testler, örnekler)
katmanlanmaz (Kural önerisi K-1).

## 2. Taşıma yöntemi (bütün alt ajanlar için bağlayıcı)

1. **Yalnız taşıma.** Bir öğe (fonksiyon, `impl` bloğu, tip, sabit, test) bütünüyle başka
   dosyaya taşınır. İzinli içerik değişiklikleri: `mod` / `use` / `pub use` satırları;
   görünürlük (`pub(super)`, `pub(crate)`, `pub`); bir `impl X { … }` bloğunu öğe sınırında
   birden çok `impl X` bloğuna ayırmak; yol düzeltmesi (`crate::a::f` → `crate::a::b::f`,
   `common::` → `support::`) — yalnız yol, başka hiçbir şey; `Cargo.toml` bağımlılık ve
   `[[test]]`/`[[example]]` satırları; modül belge yorumu (`//!`).
2. **Yasak:** fonksiyon gövdesi, ad, imza (görünürlük dışında), mantık, sabit değeri,
   biçim dizgesi, test beklentisi değişikliği; fonksiyon bölme; "küçük iyileştirme".
   Fark edilen hata düzeltilmez, alt ajanın raporuna "Fark edilenler" olarak yazılır.
3. **`git mv`** ile: büyük dosya bölünürken en büyük parça `git mv` ile yeni yerine gider
   (geçmiş izlenir), kalanlar yeni dosyalara taşınır.
4. **Bölme kalıbı** (800 üstü dosya `foo.rs`): `foo.rs` + `foo/` klasörü (2018 düzeni).
   `foo.rs` ana tipleri ve çekirdeği tutar (≤ 800, hedef ≤ 600); konu grupları
   `foo/<konu>.rs` alt modüllerine gider. Alt modül ata modülün özel öğelerini görür
   (`use super::*`); ata modülün alt modüldeki öğeye erişmesi için görünürlük `pub(super)`
   olur. `foo.rs` bir `mod.rs`/`lib.rs` değildir; bu yüzden mantık taşıyabilir. Satır içi
   test modülleri (`#[cfg(test)] mod tests { … }`) `foo/tests.rs` dosyasına taşınabilir
   (`#[cfg(test)] mod tests;`).
5. **`lib.rs` / `mod.rs` mantığı:** içindeki öğeler konuyu adlandıran bir kardeş dosyaya
   taşınır, `lib.rs`'te `mod x; pub use x::{…};` kalır; dışa açık yollar değişmez.
   `macro_rules!` (ör. `qp-session` `rd!`) `#[macro_use] mod macros;` ile ilk modül olarak.
6. **Dışa açık yollar korunur** (`pub use` ile yeniden dışa aktarma). Crate'ler arası taşınan
   öğe eski crate'ten yeniden dışa aktarılmaz (katman yönünü bozar); çağıranlar yol
   düzeltmesiyle yeni crate'i kullanır. Geçici yeniden dışa aktarmalar round sonunda
   listelenir (borç).
7. **Her alt ajan yalnız kendi crate(ler)ine yazar**, kendi dalında ve kendi worktree'sinde,
   `CARGO_TARGET_DIR=/Users/arkns/Works/Mchart/qapsule/target`. Küçük commit'ler (bir dosya
   ya da bir konu başına bir commit).
8. **Bitti sayılmak için** (o dalda): `cargo fmt --all -- --check`, `cargo clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace`, `scripts/rg1-compare.sh`
   (bayt eşit), `scripts/rg1-moved.py main` (CHANGED 0; PATH listesi raporda),
   `scripts/arch-check` çıktısında crate'in `file.hard`, `root.logic`, `name.forbidden`
   bulgusu 0. Bunu **ayrı bir denetçi alt ajan** koşar ve onaylar; sonra orkestratör
   `main`'e birleştirir ve worktree'yi kaldırır.

## 3. Katman kararları

`scripts/arch-config.json`, §2'nin makine biçimidir. İki gerçek sorun ve önerilen çözüm:

**3.1 `qp-source` (istisna isteği İ1).** §2 onu uyarlayıcılara koyar ("dış dünya: dosya");
ama içeriği veri düzlemidir: `FeedStore` (PROTOCOL §18.4 kuralları), `SourceProvider`,
`MemoryProvider`, yeniden oynatma takvimi, sentetik piyasa, canlı bar kurma. `qp-session`
bunları otoritenin parçası olarak kullanır (`replica.rs`, `claims.rs`, `session.rs`
`MemoryProvider::from_init`); dış dünyaya bakan kısım `local.rs` (64 satır) ve
`lib.rs`'teki dosya okuyucudur. Saf taşımayla yapılabilecek seçenekler:
- (a) `qp-source`'u `alan` katmanına almak (yapılandırmada `layer_overrides`, her koşuda
  "istisna" diye basılır) — **öneri**;
- (b) `qp-source`'u ikiye bölmek (veri düzlemi aşağı, dosya okuma yukarı): `MemoryProvider::
  from_init` ve `local_replay_feeds` dosya okur ve `Session::new_inner` gövdesi onu çağırır;
  dosya okumayı dışarıdan vermek gövde değişikliği ister (bu round'da yasak).
(a) ile `qp-session → qp-source` aşağı, `qp-matriks → qp-source` aşağı olur.

**3.2 `qp-render → qp-source`.** Tek kullanım `bars::synthetic_bars`, `qp_source::
synthetic_series`'e aynen ileten bir sarmalayıcı. Çözüm: sarmalayıcı `qp-render`'dan
kaldırılır, çağıranlar (`qp-wasm` 3 yer; `qp-render` örnek ve testleri) yol düzeltmesiyle
`qp_source::synthetic_series`'i çağırır; `qp-source`, `qp-render`'ın geliştirme
bağımlılığına iner. Davranış aynı (aynı fonksiyon). `qp-render`'ın dışa açık yüzeyinden bir
fonksiyon düşer (protokol, şema, CLI, MCP değişmez) — raporda yazılır.

**3.3 Yan bağımlılıklar (istisna isteği İ2 / Kural önerisi K-2).** §2 "yana bağımlılık
hata verir" der. Aynı katman içinde zincir gerçekten var: `qp-capsule → qp-facts` (kapsül
olgudan kurulur). Kabukların inceltilmesi uyarlayıcı katmanında yeni yan bağımlılıklar açar
(§5: `qp-host → qp-mcp, qp-live, qp-matriks, qp-shellcore`). `arch-check` bunları
`layer.lateral` olarak raporlar; taban çizgisine adıyla girer (mandal: yenisi hata). Öneri
(K-2): §2'ye "aynı katmanda bildirilmiş sırayla (döngüsüz) bağımlılık serbest" eklenmesi ya
da katmanların alt katmanlara ayrılması. Karar Arkin'in.

`qp-cli → qp-wasm` yan bağımlılığı kabuk inceltmesiyle (§5) kalkar.

## 4. `qp-session` (M2)

**Ölçüm.** 37 dosya, 31,6 bin test dışı satır. Modül başvuru grafiği (`crate::x`,
`super::x`): 23 modül **tek bir güçlü bağlı bileşen** (25,9 bin satır: `session`, `capsule`,
`expand`, `select`, `canon`, `viewer`, `replayhost`, `query`, `live`, `replica`, `capidx`,
`public`, `summary`, `dispute`, `gops`, `comments`, `claims`, `story`, `headparts`, `watch`,
`provider`, `present`, `agent`). Nedeni yapısal: `Session` tipinin `impl Session` blokları bu
dosyalara dağılmıştır ve Rust'ta tipin kendi `impl`'i başka crate'te yazılamaz. Bileşenin
dışında kalanlar: yapraklar `suggest`, `error`, `values`, `plane`, `apply`, `locate`
(≈ 2,5 bin) ve tepeler `interact`, `conform`, `tape`, `preview`, `explain`, `reading`,
`bindq` (≈ 3,7 bin).

**Karar.** Crate bölünmesi saf taşımayla 12 bin bütçesine indiremez (bileşen tek başına
25,9 bin); bu yüzden **konu klasörleri** + crate bütçesi için **istisna isteği İ3**
(gerekçe: yukarıdaki grafik; çözüm yolu: `Session`'ı alt sistemlere bölen bir tasarım
round'u — RF). Yaprakları ayrı crate'e almak bütçeyi tutturmaz ve yeni bir katman sorusu
açar; yapılmaz.

**Hedef düzen** (`crates/qp-session/src/`, her klasör ≤ 15 dosya, her dosya ≤ 800, her
klasörde kısa `README.md`):

| Klasör | Konu | Bugünkü dosyalar |
|---|---|---|
| `authority/` | otorite ve komut yolu: oturum, komut, uygulama, günlük, hatalar, öneri | `session.rs` (bölünür), `apply.rs`, `error.rs`, `suggest.rs`, `values.rs`, `agent.rs` |
| `ops/` | işlem genişletme ve gramer işlemleri | `expand.rs` (bölünür), `gops.rs`, `canon.rs` (bölünür), `select.rs` (bölünür), `locate.rs`, `summary.rs` (bölünür) |
| `capsule/` | kapsül yaşamı: motor bağlama, dizin, iddialar, itiraz, karne, yorumlar, izleme, hikâye | `capsule.rs` (bölünür), `capidx.rs` (bölünür), `claims.rs`, `dispute.rs`, `comments.rs`, `watch.rs`, `story.rs`, `headparts.rs`, `public.rs` (bölünür) |
| `data/` | veri düzlemi ve bağ: düzlem, sağlayıcı, canlı, bağ sorgusu | `plane.rs`, `provider.rs`, `live.rs` (bölünür), `bindq.rs` |
| `read/` | okuma ve sunum: sorgu, sunum, okuma, önizleme, açıklama, etkileşim | `query.rs` (bölünür), `present.rs`, `reading.rs`, `preview.rs`, `explain.rs`, `interact.rs` (bölünür) |
| `replica/` | yansı ve görüntüleyici | `replica.rs` (bölünür), `viewer.rs` (bölünür), `tape.rs` |
| `host/` | yeniden oynatma barındırıcısı ve uygunluk | `replayhost.rs` (bölünür), `conform.rs` |

Dışa açık yollar (`qp_session::Session`, `qp_session::capsule::…`, `qp_session::viewer::…`,
`qp_session::replayhost::…`, `qp_session::replica::…`, `qp_session::tape`,
`qp_session::conform`, `qp_session::headparts`, `qp_session::agent`) `lib.rs`'te `pub use`
ile korunur (ör. `pub use host::replayhost;`). `lib.rs` yalnız `mod` ve `pub use`; `rd!`
makrosu `macros.rs`'e. Testler (`tests/`, 39 dosya): `tests/common` → `tests/support`;
konu klasörlerine `[[test]]` satırlarıyla dağıtılması kesilebilir (kesme sırası 2).

Bu crate'e **tek alt ajan** yazar (M2); boyutu yüzünden iki aşamada çalışır: (i) klasörler ve
`lib.rs` (dosya taşıma; içerik aynı), commit; (ii) 800 üstü dosyaların bölünmesi, dosya
başına commit.

## 5. Kabukların inceltilmesi (M1)

**`qp-wasm`** (10,7 bin → < 4 bin). `wasm_bindgen` yalnız `lib.rs` bağlamalarında ve
`shell.rs`'teki `performance.now()` saatindedir; kabuk çekirdeği saf Rust'tır. Yeni crate
**`qp-shellcore`** (katman: uyarlayıcılar — oturum yansısına bağlı; "kabuk çekirdeği",
CONCEPTS: `ShellCore`): `shell.rs` + `shell/` (`agent`, `bench`, `freq`, `live`, `motion`,
`paint`, `paint_canvas`, `paint_gpu`), `gallery.rs`, `drive.rs`, `viewer.rs`. `qp-wasm`'da
yalnız `wasm_bindgen` bağlamaları kalır (`lib.rs` → `bind/` altında konu dosyaları:
`compute`, `render`, `shell`, `gallery`, `replica`, `spike`), `qp-wasm` `qp-shellcore`'u
yeniden dışa aktarır (`pub use qp_shellcore::{shell, gallery, viewer, drive}` — kabuk →
uyarlayıcı, aşağı). `performance.now()` saati `qp-shellcore`'da `cfg(target_arch =
"wasm32")` ile `wasm-bindgen` bağımlılığıyla kalır (gövde aynı). WASM boyutu ve özet aynı
kalmalı (§9).

**`qp-cli`** (11,7 bin → < 4 bin). Yeni crate **`qp-host`** (katman: uyarlayıcılar —
"barındırma": canlı barındırıcı, Matriks kaydı ve inceleme, işlem karşılaştırması,
hızlandırılmış benzetim, demo HTTP sunucusu, MCP sunucusunun bağlanması): `livehost.rs`,
`live.rs`, `live_trades.rs`, `sim.rs` + `sim/tests.rs`, `demo.rs`, `demo_static.rs`,
`mcp_serve.rs`. `qp-cli`'de yalnız `clap` tanımları ve komutların `qp-host` /
alt crate'lere bağlanması kalır (`main.rs` → `main.rs` + `cmd/` altında komut başına dosya).
`clap` argüman yapıları (`DemoArgs`, `SimArgs`, `ServeArgs`, `LiveCommand`, `FeedArgs`)
komut satırının parçasıdır: **`qp-cli`'de kalır**; `qp-host` fonksiyonları onları alıyorsa
yapılar `qp-host`'a taşınır ve `clap` türetmesi oraya gider (komut satırı arayüzü değişmez;
`qp --help` çıktısı tutanağa eklenir, §9). Hangisinin daha az değişiklik istediğini alt
ajan ölçer ve raporlar.

Yan bağımlılıklar (§3.3): `qp-host → qp-mcp, qp-live, qp-matriks, qp-shellcore`;
`qp-shellcore` → `qp-session`, `qp-render`, `qp-grammar`, `qp-capsule`, `qp-source` (aşağı).

**3.2'deki `qp-render` düzeltmesi** bu alt ajana verilir (`qp-wasm` çağrı yerleri onun).

Bu iş `qp-cli`, `qp-wasm`, `qp-render` ve iki yeni crate'e yazar; **M2 ile aynı anda**
koşabilir (`qp-session`'a yazmaz), M3/M4 ile çakışmaz.

## 6. `qp-schema` (M3)

`protocol.rs` (4.916) → `protocol/` mesaj ailesine göre: `envelope` (hello/welcome/error/
ack), `ops` (işlem tipleri), `query` (sorgular ve cevapları), `notify` (bildirim ve
konular), `data` (veri olayları, bağ), `replay`, `viewer`, `capsule` (kapsül mesajları),
`agent`, `story` (var). Dışa açık yollar `qp_schema::protocol::X` `protocol.rs`'te (ya da
`protocol/mod.rs` yerine `protocol.rs` + `protocol/`) `pub use` ile korunur. `types.rs`
(808) iki konuya. `tests/common` → `tests/support`. **Üretilen şema bayt eşit**
(`qp schema export` tutanakta; `schemars` adları tip adından gelir, modül yolundan değil —
ilk bölmeden sonra hemen `rg1-compare` ile sınanır).

## 7. Kalan crate'ler (M4; crate başına bir alt ajan)

| Crate | İş |
|---|---|
| `qp-grammar` (20,0 bin) | 800 üstü 13 dosya; `src/` 21 dosya → konu klasörleri (`look/`, `layer/`, `compile/`, `present/` [depth, advances, draw, motion]…); `stories/mod.rs` mantığı; `tests/common`. Tasarım klasörleri (`stories`, `open`, `transitions`, `glyph`, `faces`, `feeds`) ayrı crate'e saf taşımayla alınabiliyorsa (döngü yoksa) alınır ve ölçülür; alınamıyorsa crate bütçesi için **istisna isteği İ4** (grafik kanıtıyla). |
| `qp-capsule` (13,1 bin) | `voice/selvi.rs`, `voice/marla.rs` (veri tablosu ağırlıklı: tabloları ayrı dosyaya, kesme sırası 1 değil — 800 sınırı için zorunlu), `engine.rs`, `comment.rs`, `realize/elapsed.rs`; `lib.rs`, `voice/mod.rs`, `realize/mod.rs` mantığı; `tests/common`. `voice/` ayrı crate'e saf taşımayla alınabiliyorsa bütçe tutar; alınamıyorsa **istisna isteği İ5**. |
| `qp-mcp` | `surfaces/query.rs` (2.550), `task.rs`, `mixed.rs`; `lib.rs` (1.056) mantığı; `common.rs` → konu adı. Araç tanımları bayt eşit (tutanak `mcp/`). |
| `qp-live` | `conn.rs`, `mock.rs` |
| `qp-matriks` | `feed.rs`; `lib.rs` mantığı |
| `qp-source` | `livebar.rs`; `lib.rs` mantığı |
| `qp-compute` | `lib.rs`, `studies/mod.rs` mantığı; `tests/common.rs`; `tests/` 16 dosya |
| `qp-facts`, `qp-calendar` | `lib.rs` mantığı; `tests/common` (`qp-facts`) |
| `qp-render` | `lib.rs` mantığı; `examples/common` (M1'den sonra; 3.2 M1'de) |

## 8. Sıra ve eşzamanlılık

```
dalga 1 (paralel, crate'ler ayrık):  M2 qp-session | M1 kabuklar (+qp-render, iki yeni crate)
                                      | M3 qp-schema | M4a qp-grammar | M4b qp-capsule | M4c qp-mcp
dalga 2 (paralel):                    M4d küçük crate'ler (live, matriks, source, compute, facts, calendar)
                                      | K1 parametre tipi (qp-schema, M3'ten sonra)
dalga 3:                              R README'ler | L lint | K2 envanter | K3 bağlama (≥ 10 sabit)
                                      | M5 testler (kesilebilir) | M6 veri tabloları (kesilebilir)
her an (kod dışı):                    D1 DECISIONS bölünmesi | W recovered dalları
sonda:                                Z taban çizgisi | C karşılaştırma | D bağımsız denetim | X kapanış
```

- Aynı crate'e aynı anda iki alt ajan yazmaz. M1 `qp-cli`, `qp-wasm`, `qp-render` ve yeni
  crate'lere; M4d `qp-render`'a dokunmaz (`examples/common` M1'de).
- Birleştirme sırası: dalga 1 bittikçe tek tek; her birleştirmeden sonra `main`'de
  `rg1-compare` + `arch-check`. Çakışma `Cargo.lock` ve `Cargo.toml`'da beklenir (yeni
  crate'ler) — orkestratör çözer.
- `main`'de tam `scripts/verify.sh` her dalgadan sonra.

## 9. Tutanak ve karşılaştırma

`scripts/rg1-snapshot.sh` 105 kalem üretir (şema, 12 görünüm × görsel/açıklama/sahne,
hesap çıktıları ve dört özet, üç taşıma + görünüm uygunluk dökümleri, dört yansı bandı,
yeniden oynatma akışı, dört MCP yüzeyi ve RD1 uyarlayıcısının `initialize`+`tools/list`
cevabı, kabuksuz sürüş özetleri, görünüş kayıtları, iki puanlayıcı testi, test sayısı,
bütün kayıtlı fikstürlerin SHA-256'sı). Tarayıcı altınları `verify.sh` adım 18 ile kayıtlı
dosyalara karşı. **Eklenecek:** `qp --help` ve her alt komutun `--help` çıktısı (komut
satırı arayüzü değişmez; §5'te `clap` yapıları taşınabilir). Bu kalem ilk taşımadan önce
tutanağa eklenir (tutanak yeniden alınır ve commit edilir).

Performans (`scripts/rg1-perf.sh`): K17 `sweep_bench` medyanları, K52 `metrics.mjs` (CPU
×4), WASM ham/gzip (adım 8 yapısı ve kabuk paketi). Round sonunda bir kez yeniden ölçülür
(K82); WASM ±%2.

## 10. Parametre kayıt defteri (K1–K3)

`qp-schema/src/params/` (yeni modül): `ParamDef` (anahtar, tip, birim, aralık, varsayılan,
kapsam listesi, kim değiştirir, hak mı, açıklama, eklendiği sürüm), `ParamScope` (sistem,
plan, kullanıcı, enstrüman, nesne), `ParamValue`, çözümleme sırası (nesne > kullanıcı >
plan > sistem) saf fonksiyon; belge `qp params doc` değil — yeni CLI komutu açılmaz
(komut satırı değişmez): belge bir test/örnek (`cargo run -p qp-schema --example
params_doc`) ile `docs/PARAMETERS.generated.md`'ye üretilir ve bir test belgenin güncel
olduğunu sınar. Şemaya (`qp schema export`) **girmez** (dışa aktarılan şema bayt eşit
kalmalı). Sistem varsayılanları tablosu `qp-schema` dışında, sabitin sahibi crate'te değil,
tek yerde: `qp-schema/src/params/system.rs` (anahtar → varsayılan; kaynak sabitle eşitliği
test sınar). Bağlama (K3): sabitin kullanıldığı yerde değer `params::system::get("…")`
yerine tipli erişimciden okunur (`params::SENTINEL_REPEAT_HOLD.default_u32()` gibi) —
**bu bir gövde değişikliğidir** ve bu round'un tek izinli gövde değişikliği türüdür
(GOAL §3 "en az 10 sabit kayıt defterinden okunur hale gelir"): yalnız sabit ifadesi
erişimciyle değişir, eşitliği hem derleme zamanı hem test sınar; rg1-moved bunları
"CHANGED" diye listeler, denetçi tek tek bakar.

## 11. Riskler

| Risk | Önlem |
|---|---|
| Taşıma sırasında `#[cfg(...)]` kapılı öğe yanlış dosyada derlenmeden kalır (ör. `qp-wasm` `replica` özelliği, `wasm32` hedefi) | Denetçi `cargo clippy --workspace --all-targets --all-features` değil (spike özellikleri derlenmeyebilir) — `verify.sh` adım 8 (wasm32) ve `--features replica` derlemeleri + `rg1-compare` (wasm32 özet) |
| Glob `use super::*` ile ad gölgelenmesi davranışı değiştirir | Yalnız aynı ad iki yerden gelirse olur; derleyici belirsizliği hata verir; `rg1-compare` + testler |
| `impl` taşınırken özel yöntem görünmez olur | Görünürlük `pub(super)`; derleyici yakalar |
| Test sayısı değişir (taşınan test düşer) | Tutanakta `tests.count`; `rg1-compare` eşitlik ister |
| WASM boyutu / özet değişir (crate sınırı inline kararlarını etkiler) | `profile.wasm` `lto = "fat"`; `rg1-compare` wasm32 özeti; `rg1-perf` boyutu ±%2 |
| Paylaşılan `target/` worktree yolunu ikiliye gömer (R5B §6, `local_bars_dir`) | Tutanak ve karşılaştırma yalnız ana ağaçtan derlenen `qp` ile; denetçi worktree'de koşar ama yerel veriye bakan kalem yok (sentetik) |
| Disk (çok worktree) | Her birleştirmeden sonra worktree kaldırılır; dalga sonunda `du -sh target` |

## 12. Eleştiriler ve cevaplar

İki eleştirmen ayrı ayrı okudu (A: Rust'ta uygulanabilirlik; B: yönetişim ve kapsam).
Tam metinler orkestratör oturumunda; özet ve cevap:

| # | Eleştiri | Ağırlık | Cevap |
|---|---|---|---|
| B1 | `expand_op` (903) ve `layer::prepare` (883) tek başına 800'ü aşıyor; fonksiyon bölmek yasak | engelleyici | Kabul. Bu iki fonksiyon yalnız kendini taşıyan iki dosyaya gider; `file.hard` için adıyla **istisna isteği İ6** (`arch-config.json` `exceptions`). Fonksiyon bölmek bu round'da yapılmaz. |
| B2, A8 | `layer.lateral`'ı taban çizgisine koymak ve `qp-source`'u yapılandırmayla alana almak kuralı oyuna getirir | engelleyici | Kabul. `layer.lateral` artık kesin kural. Yan bağımlılıklar ve `qp-source` katmanı yalnız **adıyla istisna isteği** olarak (`exceptions`, `layer_overrides`) durur, her koşuda "EXCEPTION … Arkin onayı bekliyor" diye basılır; raporda ölçüt 4 "yukarı 0; yan N — İ1/İ2 onayına bağlı" diye yazılır, "karşılandı" diye değil. Saf taşıma bunları kaldıramaz (§3). |
| B3 | `--write-baseline` yeni kalemi ve sayı artışını sessizce yazıyor | önemli | Düzeltildi: yeni kalem `--allow-new` ister, değer ve sayı artışı her durumda reddedilir. |
| B4 | `fn.lines` anahtarında satır numarası | önemli | Düzeltildi: `yol::ad` (+ aynı ad için `#k`). |
| B5 | `arch-check` kandırılabilir: başlıktaki "üretilmiştir", `build.rs`, `#[path]`, `cfg(test)`'siz `tests.rs`, `lib.rs`'te satır içi `mod {}` | önemli | Düzeltildi: üretilmiş dosya yalnız yapılandırmada adıyla listelenirse muaf; `build.rs` taranır; `src/` dışına çıkan `#[path]` kesin hata (`path.outside`); `tests.rs`/`tests/` ancak `#[cfg(test)] mod` ile bildirilmişse test sayılır; satır içi `mod {}` `root.logic`. Öz-sınamada her biri için örnek. |
| B6, A1 | `rg1-moved` satır kümesi karşılaştırıyor: öznitelik/belge değişikliği, `if a {` satırı, sıra değişikliği, `qp_x::f` → `qp_y::g` görünmüyor | önemli | Yeniden yazıldı: **öğe parmak izi** — her `fn` (gövdesiyle), tip, sabit, makro üstündeki öznitelik ve belge yorumlarıyla birlikte önce ve sonra çoklu küme olarak eşit olmalı. Yol yalnız son bölüm aynı kalırsa PATH. Eklenen `#[allow(clippy::…)]` ALLOW diye listelenir. Sahte değişikliklerle sınandı (gövde, belge, son ad yakalandı). |
| B7, A12 | K3 bağlama "CHANGED 0" ile çelişiyor; sabit bağlamlarda erişimci çağrılamaz | önemli | §13.7. |
| B8 | Tutanak zayıf: uygunluk yalnız PASS satırı; MCP yalnız `tools/list`; test adları yok | önemli | Düzeltildi (kod taşınmadan önce, `26c513d`): 82 dökümün `qp serve` üstünden **gerçek cevap akışı**, dört yüzeyde sabit `tools/call` istekleri, test adları. 225 kalem. |
| B9 | Ortak `target/`: iki worktree birbirinin `qp`'sini koşabilir | önemli | Düzeltildi: anlık görüntü kilitli, `qp` ve wasm paketi özel geçici klasöre kopyalanır, karşılaştırma klasörü worktree başına. Son hüküm her birleştirmeden sonra `main`'de. |
| B10 | Dalga 3 aynı crate'lere paralel yazıyor | önemli | §13.8: L ve K3 sıralı; R yalnız README yazar. |
| B11 | Lint mekanizması yok | önemli | §13.6. |
| B12 | CI adımı; D1 karşılaştırma betiği; yeni crate evrakı | önemli | CI kapanışta (`preflight.sh`, `qgh`). D1 yapıldı (`1957752`, `scripts/rg1-decisions-check.py`: 352 satır eşit, yeni kırık başvuru 0). Yeni crate'ler: README, `ARCHITECTURE.md` satırı, CONCEPTS ("barındırma"), `docs/decisions/rounds/RG1.md` önerilen karar. Dal kapısı: `arch-check`'te ilgili crate için yeni bulgu yok (taban çizgisi round sonunda yazılır). |
| B13 | Yalınlık | küçük | M5 (test klasörleri) ve M6 (veri tabloları) **baştan kesildi** (kesme sırası 1, 2); yalnız 800 sınırı için gereken bölme yapılır. |
| A2 | `common::` → `support::` CHANGED sayılıyordu | önemli | Yeni `rg1-moved`'de modül yeniden adlandırma PATH (son bölüm aynı). |
| A3 | §3.2 çağrı yerleri yanlış (`qp-wasm` değil; `qp-render` testleri, örnekleri, birim testi) | önemli | §13.3. |
| A4 | `capsule/capsule.rs`, `replica/replica.rs` `module_inception` ve yol sorunu; klasör dosya sayısı | önemli | §13.2. |
| A5 | `clap` yapıları `qp-host`'a gitmeli (`demo(&DemoArgs)`, `value_parser = crate::sim::…`) | önemli | Kabul: yapılar `qp-host`'a, `qp-host` `clap`'a bağlı; `#[command(version)]` `main.rs`'te kalır (`--help` ağacı tutanakta). |
| A6 | Kütüphaneye geçen ikili kodu yeni lint'ler tetikleyebilir | önemli | Görünürlük en dar (`pub` yalnız crate dışı gerekiyorsa); tetiklenen clippy uyarısı öğeye `#[allow(clippy::…)]` ile (ALLOW listesi, taban çizgisine). |
| A7 | Varsayılan WASM yapısı: `qp-shellcore` yalnız `replica` özelliğiyle; `drive` kapısı; `wasm-bindgen` `=0.2.130` | önemli | Kabul (§13.4). |
| A9 | `include_str!("../../../fixtures/…")` derinleşince kırılır; `env!("CARGO_PKG_VERSION")` yeni crate'te değişir | küçük | Göreli yol dizgesi düzeltmesi izinli ve listelenir; her yeni crate `version.workspace = true`. |
| A10 | `schemars` `schema_id` modül yolunu içerir; ad çakışmasında 2, 3 eki karşılaşma sırasıyla | küçük | İki `Op` aynı modül yoluna konmaz; ilk bölmeden hemen sonra `rg1-compare`. |
| A11 | M1 tek ajan için çok büyük | küçük | M1a (`qp-wasm` → `qp-shellcore`), sonra M1b (`qp-cli` → `qp-host`; `qp-host` `qp-shellcore`'u kullanır). |
| A12 | 800 üstü test dosyaları izlenmiyor; belge başvuruları eskir | küçük | `test.file` kuralı izliyor (taban çizgisi). Belge başvuruları R adımında güncellenir. |

## 13. v2 değişiklikleri (bağlayıcı)

**13.1 Doğrulama kapısı (her taşıma dalı).** Ayrı denetçi alt ajan o dalda koşar:
`cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
`cargo test --workspace`; `scripts/rg1-compare.sh` (225 kalem bayt eşit);
`scripts/rg1-moved.py main <dal> --list` → **CHANGED 0** (PATH ve ALLOW listesi rapora; izinli
istisnalar adıyla: §13.3'teki test, göreli `include_str!` yolu); `scripts/arch-check --json`
ile o crate'te `file.hard` (İ6 dışında), `root.logic`, `name.forbidden`, `path.outside`, `layer.up`
bulgusu 0 ve başka kuralda bulgu sayısı `main`'dekinden fazla değil (dosya bölmenin doğal
sonucu olan `file.soft` ve `fn.lines` anahtar değişimleri hariç). Orkestratör birleştirir,
`main`'de `rg1-compare` + `arch-check`, worktree'yi kaldırır.

**13.2 `qp-session` klasör kalıbı.** Var olan dışa açık modül adları (`capsule`, `replica`,
`viewer`, `replayhost`, `tape`, `conform`, `agent`, `headparts`) klasör kökü olur: `capsule.rs`
+ `capsule/` (çocuklar: `claims`, `dispute`, `comments`, `watch`, `story`, `capidx`, `public`,
`headparts` ve `capsule.rs`'in parçaları — 15'i aşarsa `capsule/engine/`, `capsule/index/`
gibi alt klasörler). `replica.rs` + `replica/` (`viewer`, `tape` ve parçalar). Öbür konu
klasörleri (`authority/`, `ops/`, `data/`, `read/`, `host/`) `mod.rs` yerine `authority.rs` gibi
yalnız bildirim taşıyan kök dosya kullanabilir — ama `root.logic` yalnız `lib.rs`/`mod.rs`'e
bakar; bildirimden başka şey taşıyan kök dosya yine de 800 sınırına tabi. **İç yollar
değişmez:** `lib.rs` taşınan modülleri eski adlarıyla yeniden bildirir (`use capsule::claims;`
gibi `pub(crate) use`), böylece gövdelerdeki `crate::claims::…` yolları dokunulmadan çalışır;
dışa açık yollar `pub use` ile (`#[doc(hidden)] pub use replica::tape;`). Klasörlerde
`README.md` (3–6 satır). `#[cfg(test)]` satır içi testler parça dosyalarla birlikte ya da
`x/tests.rs`'e. `include_str!` göreli yolu derinleşen dosyada düzeltilir (listelenir).

**13.3 `qp-render → qp-source`.** `bars::synthetic_bars` sarmalayıcısı kaldırılır. Çağıranlar:
`qp-render/tests/golden.rs:271` ve beş örnek (`gen_sample`, `bench`, `frame_100k`,
`spike_vello_cpu`, `spike_vello_gpu`) `qp_source::synthetic_series(…)` çağırır (`qp-source`
geliştirme bağımlılığı). Birim testi `synthetic_is_valid_and_stable` `qp-source/src/synthetic.rs`
testlerine `synthetic_series` ile taşınır (test sayısı ve adı aynı kalır). Bunlar adıyla
listelenen izinli CHANGED'dir (ad değişikliği: aynı fonksiyon, sarmalayıcı düştü).
`qp_render::synthetic_bars` dışa açık yüzeyden düşer (raporda).

**13.4 M1a `qp-wasm` → `qp-shellcore`.** Yeni crate `crates/qp-shellcore` (`version.workspace`,
`[lints] workspace = true`, README): `shell.rs` + `shell/`, `gallery.rs`, `drive.rs`
(`#[cfg(not(target_arch = "wasm32"))]`), `viewer.rs`. Bağımlılıklar: `qp-schema`,
`qp-session`, `qp-source`, `qp-calendar`, `qp-capsule`, `qp-grammar`, `qp-render`,
`qp-compute` (gerekirse), `fontdue`, `serde`, `serde_json`; `wasm-bindgen = "=0.2.130"` yalnız
`cfg(target_arch = "wasm32")` hedef bağımlılığı. `qp-wasm`: `qp-shellcore` isteğe bağlı,
`replica` özelliğiyle; `lib.rs` `pub use qp_shellcore::{shell, gallery, viewer}` (+ `drive`
kapısıyla) — `crate::shell::…` yolları bağlamalarda değişmez. `qp-wasm/lib.rs` (1.296)
bağlamaları `bind/` altında konu dosyalarına; `lib.rs` yalnız bildirim. 800 üstü
`shell.rs`, `paint_canvas.rs`, `paint_gpu.rs`, `gallery.rs`, `drive.rs` §2.4 kalıbıyla bölünür.
WASM özeti ve boyutu: `rg1-compare` (wasm32 özeti) + `wasm32` derlemesi (`verify.sh` adım 8).

**13.5 M1b `qp-cli` → `qp-host`** (M1a birleştikten sonra). Yeni crate `crates/qp-host`:
`livehost`, `live`, `live_trades`, `sim` (+ `sim/tests.rs`), `demo`, `demo_static`, `mcp_serve`
ve `clap` argüman yapıları (`clap` bağımlılığı); `qp-shellcore`'u doğrudan kullanır
(`qp_wasm::shell` → `qp_shellcore::shell` yol düzeltmesi), böylece `qp-cli → qp-wasm` kalkar.
Bu dosyaların 800 üstü olanları bölünür. `qp-cli/main.rs` (1.387) `main.rs` + `cmd/` konu
dosyalarına. `include_str!`/`include_bytes!` göreli yolları (ör. `demo_static.rs`) düzeltilir ve
listelenir.

**13.6 Lint (L, taşımalardan sonra, tek alt ajan, sıralı).** Kök `Cargo.toml`
`[workspace.lints.clippy]`: `unwrap_used`, `expect_used`, `panic`, `too_many_lines`,
`cognitive_complexity` = `warn` (verify `-D warnings` ile hata); `clippy.toml`:
`too-many-lines-threshold = 80`, `allow-unwrap-in-tests`, `allow-expect-in-tests`,
`allow-panic-in-tests` (sürüm destekliyorsa; desteklemiyorsa test modüllerine
`#![allow]`). Mevcut ihlaller öğe başına `#[allow(clippy::…)]` ile (betikle, `clippy
--message-format=json` çıktısından; `rg1-moved` ALLOW). `arch-check` yeni kural `lint.allow`:
crate başına `#[allow(clippy::unwrap_used|expect_used|panic|too_many_lines|cognitive_complexity)]`
sayısı taban çizgisinde (mandal); yeni bir izin kalemi hata. Böylece yeni ihlal derlemeyi kırar,
eskiler sayısıyla görünür. Bu kesme sırası 3'tür: ihlaller düzeltilmez.

**13.7 Parametre bağlama (K3).** Gövdeye dokunulmaz: bağlanacak her değer önce (ayrı
commit) adlandırılmış `const` ise doğrudan, değilse zaten adlı sabit olanlar seçilir. Bağlama
yalnız sabitin **ilklendiricisini** değiştirir: `const HOLD: u32 =
qp_schema::params::SENTINEL_REPEAT_HOLD.default_u32();` (`const fn` erişimci). Kayıt defteri
`qp-schema/src/params/` (tip, çözümleme, sistem varsayılanları tablosu; şema dışa aktarımına
girmez). Her bağlanan sabit için sahibi crate'te test: değer eski değişmeze eşit (f64
`to_bits`). `rg1-moved` bu `const` satırlarını CHANGED listeler; denetçi tek tek bakar (izinli
tür). En az 10 sabit, mümkünse tutanak çıktılarına ulaşanlardan. Belge
`docs/PARAMETERS.generated.md`, `qp-schema` örneği üretir, test güncelliğini sınar.
Çözümleme sırasında `enstrüman` kapsamının yeri ENGINEERING'de yok → Kural önerisi.

**13.8 Sıra (v2).**

```
dalga 1 (paralel):  M2 qp-session | M3 qp-schema | M4a qp-grammar | M4b qp-capsule | M4c qp-mcp
                    | M1a qp-wasm → qp-shellcore (+qp-render düzeltmesi)
dalga 2:            M1b qp-cli → qp-host | M4d küçük crate'ler | K1 parametre tipi (M3 sonrası)
dalga 3 (sıralı):   R README + belge başvuruları → L lint → K2 envanter → K3 bağlama
sonda:              Z taban çizgisi + istisnalar → C → D denetim → X kapanış, CI
kesildi:            M5 test klasörleri, M6 veri tabloları
```
