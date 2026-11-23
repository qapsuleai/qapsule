# RF1 — Bölme planı (v1)

Durum: **taslak v1, karar noktasından önce.** Kod değişmedi. Ölçümler `05430e06` kodunda,
`scripts/refactor/graph.py` ile (dökümler `experiments/graph/`). Satır sayıları `arch-check`
sayımıdır (test dışı; üretilmiş dosya sayılmaz).

## 0. Değişmez ve yöntem

- Davranış değişmez: `scripts/refactor/compare.sh` her birleştirmeden önce dalda, sonra `main`'de
  bayt eşit. Protokol, şema, komut satırı, MCP yüzeyi aynı. Rust dışa açık yolları değişebilir;
  her değişen yol `experiments/paths.md`'ye yazılır.
- **Taşı, sonra yeniden yaz.** Bir öğe ya taşınır (öğe parmak izi "değişmedi",
  `scripts/refactor/moved.py`) ya da yeniden yazılır (sayılır, listelenir). Gövdesi değişecek
  her yerde önce karakterizasyon sınaması yazılır ve **ayrı commit** olarak değişiklikten önce girer;
  sınamalar `experiments/new-tests.txt`'e adıyla yazılır (tutanakta test adları bu listeyle süzülür).
- Her iş kendi dalında ve worktree'sinde; yazan ajan ≠ denetçi ajan; kesin hüküm
  `target/wt/<iş>` klasöründe, iş bitince silinir. Aynı crate'e aynı anda tek ajan.

## 1. Bugünkü grafik (ölçülmüş)

### 1.1 `qp-session` (31.999 satır, 84 dosya)

`Session`'ın 57 alanı, 295 yöntemi var; yöntemler 35 dosyadaki `impl Session` bloklarına dağılmış.

**Güçlü bağlı bileşen: 25.895 satır, 31 grup** (yöntem çağrısı + `crate::` yolu kenarları; ayrıntı
`experiments/graph/session.txt`). Bileşen dışında 6.104 satır: `apply`, `error`, `suggest`,
`values`, `plane`, `locate`, `explain`, `host/*`, `replica/{follow,judge,tape}`.

Gruplar arası yöntem arayüzü (`experiments/graph/session-interface.txt`; farklı yöntem adı sayısı):

| Çağıran → çağrılan | Yöntem | Ne |
|---|---|---|
| otorite → kapsül | **35** | iki dünya (kamu dünyası, `in_world`, `pub_*`, `seq_to`, `commit_public` …) 17; okuyucu izdüşümü (`as_reader`, `proj_for`, `project_entry`, `withheld_for`, `lang_of`) 7; kapsül koşusu (`run_capsules`, `start_capsules`, `stories_*`) 5; iddia/itiraz/yorum denetimi ve olayları 6 |
| kapsül → otorite | 17 | `build`, `seq`, `clock`, `principal`, `responsible`, `engine_entry_with`, `entry_events*`, `handle_raw`, `note_writers`, `cond_blocked`, `data_changed`, `market_item` |
| veri (canlı yol) → otorite / kapsül | 10 / 10 | |
| okuma → kapsül / otorite / veri | 9 / 5 / 3 | |
| yansı → otorite / kapsül / okuma | 10 / 8 / 3 | yansı bir `Session` tutar (istemci aynası) |
| barındırma → otorite / kapsül | 15 / 4 | dışa açık API |
| **işlemler → otorite** | **3** | `build`, `build_with`, `document` |
| otorite → görüntüleyici / veri / okuma / ajan | 6 / 4 / 2 / 1 | |

Alanların dağılımı (en geniş): `conns` 20 grup; `runner` 8; `other` (kamu dünyası) 6;
`pending_stamps`, `pending_views`, `in_public`, `pending_pub` 4. Öbür 26 alan otoritenin kendi
alanı ya da başka bir yapıda aynı adla geçtiği için ayrı sayılmadı (`doc`, `log`, `clock` …).

**Okuma:** bileşeni bir arada tutan üç şey var: (1) `impl Session`'ın dosyalara dağılması
(Rust'ta başka crate'te yazılamaz); (2) **iki dünya** düzeneği — lisanslı akış varken
otorite alanlarının (doküman, günlük, önbellekler) kamu dünyasıyla takas edilmesi
(`public.rs`, `worlds.rs`); (3) `conns` (bağlantı başına bakış durumu) her yerden okunuyor.
İşlemler (`ops/*`, 8,3 bin) bileşende yalnız birkaç serbest fonksiyonla tutunuyor:
`watch::{intent_visible, visible_ids, expand_watch, WATCH_PREFIX}`, `live::closure_of`,
`present::presenter_view`, `session::ENGINE_PRINCIPAL`.

### 1.2 `qp-grammar` (20.964 satır)

| Bileşen | Satır | Gruplar |
|---|---|---|
| Döngü A | 13.207 | `compile`, `present` (depth, gesture, motion), `provider`, `faces`, `open`, `transitions` |
| Döngü B | 2.176 | `look` ↔ `advances` (üretilmiş, sayılmaz) |
| Dışarıda | 5.581 | `glyph` 1.981, `stories` 2.243, `feeds` 700, `theme` 373, `detmath` 170, `error` 76 |

Çekirdek → tasarım kenarları (kesilecek olanlar; ayrıntı `experiments/graph/grammar.txt`):
`compile/{scene,frame,layer/prepare}` → `faces` (başlık şablonu, `HeadlineFn`);
`compile/{terms,layer/prepare}`, `provider` → `open` (`layer_term`, `field_text`, `text`,
`DeviceCheck`); `present/depth` → `faces` (`is_face`, `face_view*`), `stories` (`by_name`);
`present/motion` → `transitions` (`TRANSITIONS`, `nabiz::*`, `surek::ramp`). Çekirdek →
`look`, `glyph`, `theme` kenarları aşağı doğrudur (onlar çekirdeği tanımaz).

### 1.3 `qp-capsule` (13.230)

`voice/` 4,75 bin; çekirdek `voice`'u çağırmıyor (döngü yok). `qp-voice → qp-capsule, qp-facts`.

## 2. Katman içi sıra (K-2, `arch-check`)

`arch-config.json`'a her katman için **bildirilmiş sıra** gelir: `"order": ["qp-facts",
"qp-capsule", …]`. Aynı katmanda `a → b` ancak `b` sırada `a`'dan önceyse geçer; sıra dışı ya
da bildirilmemiş yan kenar kesin hata. Sıra listesi döngü içeremez (liste olduğu için doğal
olarak). İ2'nin 5 kalemi bu sırayla kapanır, istisna listesinden silinir. Ayrıca: fonksiyon boyu
istisnalarına tavan (`fn.lines` mandalı istisna kalemlerinde de), `verify.sh`'a `main`'e göre
`arch-config.json` / `arch-baseline.json` farkını basan adım (`config-diff`; yeni istisna
satırı karar numarası taşımıyorsa kırmızı), öz-sınamaya sıra ihlali ve sahte sıra örnekleri.

## 3. Hedef crate'ler

### 3.1 Gramer (alan katmanı; sıra aşağıdan yukarı)

| Crate | İçerik | Tahmini satır | Bağımlı |
|---|---|---|---|
| `qp-look` | görünüş kaydı ve tipleri (`look/*`, `advances` üretilmiş), tema (`theme`), gramer hatası (`error`), `detmath` | ≈ 2,8 bin | qp-schema |
| `qp-glyph` | anlam işareti çizimi (`glyph/*`) | ≈ 2,0 bin | qp-look |
| `qp-grammar` | derleyici: `compile/*` (alfabe dahil), `present/{depth,gesture,motion}` çekirdeği, `provider`; **tasarım kaydı arayüzü** (`Designs`) ve tasarım tiplerinin sözlüğü (`FaceDesign`, `HeadlineFn`, `DeviceCheck`, `StoryDesign`, `MotionDesign`, `Frame`, `LiveMarks`) | **≈ 8,4 bin → R7'ye ≈ 3,6 bin pay** | qp-look, qp-glyph, qp-calendar |
| `qp-designs` | adlı tasarımlar: yüzler (`faces`), açılmış kapsül (`open`), hikâye çizelgeleri (`stories`), akış düzenleri (`feeds`), geçişler (`transitions`), hareket tasarımları; `Designs`'ın varsayılan kaydı `qp_designs::REGISTRY` | ≈ 8,5 bin | qp-grammar |

**Enjeksiyon:** derleyici adla tasarım bulacağı her yerde `&dyn Designs` alır (`CompileOptions`
içinde ya da derinlik fonksiyonlarına parametre; derleyicinin kendi varsayılanı yok — tasarım
tarafını tanımaz). `Designs` yöntemleri bugünkü serbest fonksiyonların birebir karşılığı:
`headline_template`, `headline_text`, `is_face`, `face_view`, `face_view_in`, `layer_term`,
`voice_terms_in`, `field_text`, `open_text`, `story`, `transition`, `motion`. Çağıranlar
(`qp-session`, `qp-shellcore`, `qp-cli`, `qp-render` testleri) `&qp_designs::REGISTRY` verir.
`DeviceCheck`, `HeadlineFn` gibi tipler çekirdeğe iner (sözlük); örnekleri yukarıda kalır.
`cargo tree -p qp-grammar` tasarım crate'ini göstermez (VERIFY 10).

**R7 işaretleri** (seviye etiketi, kırılım çentiği, kanıt bandı, niyet etiketi) çekirdeğin
alfabesine (`qp-grammar/src/compile/alphabet/`) girer: ≈ 3,6 bin pay. Görünüş değerleri
`qp-look`'ta (2,8 bin; 9 bin pay).

**`prepare` (885 satır, İ6):** sırayla şu adlandırılmış adımlara: `layer_source`
(veri kaynağı ve satırlar), `capsule_texts` (ses/başlık/açılmış kapsül metinleri),
`check_encoding` (opaklık, genişlik, x/x2 kuralları), `mark_rows` (işarete göre `match`; her
kol kendi fonksiyonu), `finish` (çapa, etiket, `Prepared`). Aradaki değişken yığını bir
`PrepState` yapısında. Karakterizasyon: katman türü (8 işaret) × veri kaynağı (seri, olgu,
bulgu, sabit) × görünüş başına `Prepared` dökümü (`Debug` çıktısının özeti, `tests/prepare_*`).

### 3.2 Ses (alan)

`qp-voice` = `qp-capsule/src/voice/*` saf taşıma (≈ 4,75 bin). Sıra: `qp-facts < qp-capsule <
qp-voice`. `qp-capsule` ≈ 8,5 bin. Çağıranlar (`qp-session` `voices` yeniden dışa aktarımı,
`qp-shellcore`) yol düzeltmesiyle. Onaydan önce yapılabilir (GOAL §2).

### 3.3 Oturum (oturum katmanı; sıra aşağıdan yukarı)

| # | Crate | İçerik | Tahmini | Durum / sahibi |
|---|---|---|---|---|
| 1 | `qp-ops` | **işlemler:** işlem açılımı (`expand`, `gops`), seçiciler ve anlamla başvuru (`select`, `locate`), kanonik biçim (`canon`), özet (`summary`), değişim uygulama ve yeniden oynatma (`apply`), protokol hatası kurucusu (`error`, `suggest`), değer önbelleği (`values`), veri düzlemi (`plane`), sunucu (`present`) | ≈ 10,0 bin | saf fonksiyonlar; durum yok. Kendi dışından tek ihtiyaç bugün `watch` ve `live`'dan 5 serbest fonksiyon: işlem açılımı kaydına (aşağıda) dönüşür |
| 2 | `qp-subsystem` | **alt sistem sözleşmesi:** `Subsystem` trait'i, `Host` arayüzü (otoritenin alt sisteme verdiği dar görünüm), kayıt tipi (`Registry`), olay ve kanca tipleri, işlem açılımı kaydı (`OpExpander`) | ≈ 0,6 bin | yeni |
| 3a | `qp-watch` | izleme niyeti: `watch` + işlem açılımı kolları (`AddWatch`, `RemoveWatch`) kayıtla | ≈ 0,5 bin | alt sistem; R7 burada büyür |
| 3b | `qp-claims` | iddia, itiraz, yorum: `claims`, `dispute`, `comments` (kendi durumu: `claims`, `settle`, `comment_audit`) | ≈ 2,1 bin | alt sistem |
| 3c | `qp-capsule-life` | kapsül yaşamı: motor bağlama (`run`, `capsule`), okuyucu izdüşümü (`reader`, `project`, `capidx`), hikâye (`story`), karne sorguları (`queries`, `settled`), başlık parçaları (`headparts`, `sentences`); kendi durumu: `runner`, önbellekler (`fact_ix`, `cap_ix`, `sentences`, `item_memo`, `projs`, `views`, `head_state`, `headlines`, `real_facts`), `coin_flip*`, `trust_keys`, `audience_seen` | ≈ 5,0 bin | alt sistem; **en riskli** |
| 3d | `qp-livepath` | canlı yol: koşulların ve kapsüllerin artımlı değerlendirmesi (`data/live`), bağ sorguları (`bindq`), doküman veri sağlayıcısı (`provider`) | ≈ 1,6 bin | alt sistem (veri olayı ve saat kancası) |
| 3e | `qp-read` | okuma ve sunum: `query`, `interact`, `reading`, `preview`, `explain` | ≈ 3,2 bin | alt sistem (sorgu ve girdi kancası) |
| 3f | `qp-viewer` | görüntüleyici bağlantıları (otorite tarafı: `replica/viewer*`) | ≈ 1,4 bin | alt sistem (abonelik kancası) |
| 4 | `qp-authority` | **otorite:** `Session` çekirdeği — oluşturma, alım (`intake`), dağıtım, komut yolu, kabul, uzlaşma, saat, varlık durumu, olaylar; günlük; bağlantılar; ajan politikası (`agent`); **iki dünya** (kamu dünyası: `public`, `public/commit`, `worlds`) — günlüğün kime hangi sırayla gittiği otoritenin işidir; kayıt üstünden alt sistem çağrıları | ≈ 5,6 bin | otorite |
| 5 | `qp-session` | **bileşim:** `Session` = otorite + standart alt sistemlerin kaydı (yalnız kayıt satırları ve dışa açık yolların yeniden dışa aktarımı) | ≈ 0,2 bin | bileşim kökü |
| 6 | `qp-replica` | istemci yansısı (`Replica`, `follow`, `judge`, `tape`) — bir `Session` tutar | ≈ 1,6 bin | otoritenin istemcisi |
| 7 | `qp-replayhost` | yeniden oynatma barındırıcısı (`ReplayHost`) ve uygunluk koşucusu (`conform`) | ≈ 1,9 bin | otoritenin istemcisi |

Toplam ≈ 33 bin (bugün 32,0 bin; sözleşme ve kayıt ≈ +1 bin). **12 bin üstü crate kalmaz.**
Çağıranlar (`qp-host`, `qp-mcp`, `qp-shellcore`, `qp-cli`, `qp-wasm`, `qp-render` testleri)
`qp_session::Session`'ı aynı yoldan kullanmaya devam eder; `Replica`, `ReplayHost`, `conform`,
`tape`, `viewer` yeni crate yollarına taşınır (liste `experiments/paths.md`).

**`qp-session` bileşimi nasıl:** `qp_session::Session` bir yeni tip: `pub struct Session(qp_authority::Authority)`,
`Deref`/`DerefMut` ile otoritenin yöntemlerini açar; kurucuları (`new`, `new_with_trust_keys`,
`with_provider_and_trust_keys`, …) otoriteyi standart kayıtla kurar. Böylece `Session::new(…)`
ve `session.handle_raw(…)` çağrıları değişmez, otorite hiçbir alt sistemi tanımaz.

## 4. Otorite arayüzü

```text
trait Subsystem {                        // qp-subsystem
    fn name(&self) -> &'static str;
    fn expanders(&self) -> &[&dyn OpExpander] { &[] }          // komut: işlem açılımı
    fn check(&mut self, host: &dyn Host, cmd: &CommandView) -> PResult<()> { Ok(()) }
    fn after_commit(&mut self, host: &mut dyn HostMut, entry: &LogEntry) -> Vec<Notice> { … }
    fn on_clock(&mut self, host: &mut dyn HostMut, at: i64) -> Vec<Notice> { … }
    fn on_data(&mut self, host: &mut dyn HostMut, ev: &DataEvent) -> Vec<Notice> { … }
    fn query(&self, host: &dyn Host, q: &Query) -> Option<PResult<Value>> { None }
    fn project(&self, host: &dyn Host, reader: &Reader, out: &mut Outgoing) { }   // yerleşim
    fn as_any(&self) -> &dyn Any;  fn as_any_mut(&mut self) -> &mut dyn Any;
}
trait Host {          // otoritenin verdiği dar görünüm (okuma)
    fn document(&self) -> &Document;  fn log(&self) -> &[LogEntry];  fn clock(&self) -> i64;
    fn principal(&self, conn: &str) -> Option<&Principal>;  fn conn_view(&self, conn: &str) -> Option<&ViewState>;
    fn plane(&self) -> &DataPlane;  fn market(&self) -> Option<&MarketStatus>; …
}
trait HostMut: Host { // yazma yalnız komut yolundan
    fn engine_entry(&mut self, changes: Vec<ChangeOp>, reason: &str) -> PResult<Seq>;
    fn notify(&mut self, to: Audience, msg: Notify);
    fn in_world<R>(…)  // kamu dünyası: otoritenin
}
```

- **Komut:** otorite `Op`'u açarken kayıttaki `OpExpander`'lara sorar (op türüne göre); açılım
  ve denetim alt sistemin, değişimin günlüğe yazılması otoritenin. Protokol değişmez: `Op`
  şemada kapalı bir enum; bugün hangi kol nerede açılıyorsa yalnız yeri değişir.
- **Sorgu:** `query` adını tanıyan alt sistem cevaplar; tanıyan yoksa bugünkü hata.
- **Olay:** `after_commit` (günlük girdisinden sonra; iddia/yorum/kapsül olayları), `on_data`
  (veri olayı; canlı yol), `on_clock` (saat adımı; kapsül koşusu, uzlaşma).
- **Yerleşim:** `project` okuyucuya giden mesajın alt sisteme ait parçasını kurar (okuyucu
  izdüşümü, gizli kalanlar).
- **Kayıt sırası belirlenimcidir** (bileşimdeki satır sırası); alt sistem çağrı sırası bugünkü
  kod sırasıyla aynı kurulur — karakterizasyon sınaması bunu kilitler.
- Alt sistemler birbirini **katman içi sırayla** tanıyabilir (ör. `qp-read → qp-capsule-life`),
  otoriteyi tanımaz; birbirinin durumuna `Registry::get::<T>()` ile erişir (bileşim çalışma
  zamanında verir).

Arayüzün kesin yöntem listesi S3'te her alt sistem taşındıkça ölçümle kurulur; yukarıdaki
iskelet en dar başlangıçtır. Her yeni `Host` yöntemi `experiments/host-api.md`'de gerekçeli.

## 5. Sonraki round'lar nereye oturur

| Round | Crate | Otoriteye ne eklenir |
|---|---|---|
| RL1 | `qp-lang` (alan: hesap dili, ara gösterim, maliyet) + `qp-program` (oturum alt sistemi: `add_program` / `check_program` açılımı, maliyet bütçesi denetimi `check`, canlı değerlendirme `on_data`) | `qp-session` bileşimine 1 kayıt satırı; şemaya `Op` kolları (şema round'un işi) |
| R7 | `qp-drawing` (çizim nesneleri, çizim → koşul → olay), `qp-watch` büyür (izleme görevleri, yaşam döngüsü, niyet ve netleştirme), "kur ve göster" `qp-read`'de ya da `qp-drawing`'de; işaretler `qp-grammar` alfabesine (3,6 bin pay), görünüş değerleri `qp-look`'a | 1–2 kayıt satırı |
| RS1 | `qp-record` (sicil: iddia + sonuç, yeniden oynatma kaydı, tutma yüzdesi; `after_commit` ve `on_clock`), kurumsal işlem taşıması `on_data` kancasıyla `qp-record`'da | 1 kayıt satırı |
| RA1 | `qp-prep` (hazırlık görevi), `qp-shared` (ortak analiz ve üyelik), zaman aralığı okuması `qp-read`'e (3,2 bin; ≈ 8 bin pay) | 1–2 kayıt satırı |

## 6. Sıra, risk, geri dönüş

| Adım | İş | Tür | Risk | Geri dönüş |
|---|---|---|---|---|
| Ö | P8 çizilen (ayrı fikstür), P9 `design-check`'te; Playwright klasörleri | araç | düşük | dal |
| A1 | `scripts/refactor/` kalıcı (yapıldı: snapshot, compare, perf, moved, graph); `moved.py`'a `use`, `impl` başlığı, öğe düzeyi makro, `cfg` | araç | düşük | dal |
| A2 | `arch-check` sıra kuralı, `config-diff`, `fn.lines` istisna tavanı, öz-sınama | araç | düşük | dal |
| V | `qp-voice` | saf taşıma | düşük | dal |
| **K** | **karar noktası** | | | |
| G1 | gramer karakterizasyonu (katman türü × kaynak × görünüş; yüz, açılmış kapsül, hikâye, geçiş kareleri) | test | — | — |
| G2a | `qp-look`, `qp-glyph` aşağı (saf taşıma; döngüsüz) | taşıma | düşük | dal |
| G2b | `Designs` arayüzü + çağrı yerleri (≈ 25 yer yeniden yazılır) | yeniden yazma | **orta**: derleme sırasını değiştirmeden aynı fonksiyonu çağırmalı | dal; tutanak |
| G2c | `qp-designs` yukarı (taşıma) | taşıma | düşük | dal |
| G3 | `prepare` adımlara | yeniden yazma | orta | karakterizasyon |
| S1 | oturum karakterizasyonu: işlem türü başına `expand_op` (26 kol × başarı/hata), alt sistem başına olay sırası (komut → günlük → bildirim dizisi), iki dünyalı dökümler | test | — | — |
| S2a | `qp-ops` aşağı: 5 serbest fonksiyon `qp-ops`'a ya da açılım kaydına; `ENGINE_PRINCIPAL` sabiti `qp-ops`'a | taşıma + küçük yeniden yazma | düşük | dal |
| S2b | `qp-replica`, `qp-replayhost` yukarı (dışa açık API kullanırlar) | taşıma | düşük | dal |
| S4 | `expand_op` 26 kol → kol başına fonksiyon; dağıtıcı `match` | yeniden yazma (gövdeler aynen) | düşük | karakterizasyon |
| S3.0 | `qp-subsystem` + otoritede kayıt; alt sistem yok, davranış aynı | yeni kod | orta | dal |
| S3.1 | `qp-viewer` (arayüz 6 yöntem) | yeniden yazma (`self.x` → `host.x()`/kendi durumu) | orta | tutanak |
| S3.2 | `qp-livepath`, `qp-read` | yeniden yazma | orta | tutanak |
| S3.3 | `qp-watch`, `qp-claims` | yeniden yazma | orta | tutanak |
| S3.4 | `qp-capsule-life`; iki dünya otoritede kalır | yeniden yazma | **yüksek** | **geri dönüş: bu dalga tutmazsa kapsül yaşamı otoriteyle aynı crate'te kalır** (`qp-authority` ≈ 10,6 bin + kalanlar; gerekirse tek adlı istisna, tavan ≈ 13 bin < 32 bin, ölçümle Arkin'e) |
| E | ek noktası belgesi + örnek alt sistem sınaması | belge + test | düşük | — |
| Q | `qp-source` dosya okuma (kesilebilir), geçici yeniden dışa aktarmalar (kesilebilir), eskimiş yorumlar | | | |
| A3 | `unimplemented`/`unreachable` lint (kesilebilir); sözdizimi ağacı (kesilebilir, ilk kesilen) | | | |

**En riskli üç adım:** (1) S3.4 kapsül yaşamının otoriteden ayrılması (35 yöntemlik arayüz,
iki dünya takası, `RefCell` önbellekleri; olay sırası tek bir yerde kayarsa yanıt akışı değişir);
(2) G2b tasarım kaydının enjeksiyonu (çok çağrı yeri; derleyicinin varsayılan tasarım seçimi
yerinden oynamamalı); (3) S3.0–S3.1 kayıt düzeneğinin ilk kurulumu (olay ve bildirim sırasının
bugünküyle bire bir kalması; WASM boyutunda `dyn` kaynaklı büyüme).

Ölçüt kapısı: her dalga sonunda `compare.sh` bayt eşit, `verify.sh` yeşil, `arch-check` yeşil;
WASM ±%2 her dalgada bir kez (`perf.sh`'ın WASM kısmı) — kayma görülürse dalga durur.

## 7. Yeni alt sistem nasıl eklenir (taslak; `docs/ARCHITECTURE.md`'ye girecek)

1. Oturum katmanında yeni crate (`qp-<ad>`), `arch-config.json` sırasına eklenir; bağımlılık:
   `qp-subsystem`, `qp-ops`, gerekiyorsa sıradaki alt sistemler.
2. Durumunu taşıyan tip `impl Subsystem`: işlem açılımı (`OpExpander`), denetim, olay
   (`after_commit`, `on_data`, `on_clock`), sorgu, yerleşim.
3. Günlüğe yazmak yalnız `HostMut::engine_entry` (ya da komutun açılımı) ile; yeniden oynatma
   değişimden çalışır, alt sistem kodu istemez (`apply::replay`).
4. Yansı: alt sistemin istemciye gidenleri `project` ile; yansı yalnız protokol mesajı görür.
5. `qp-session` bileşimine **bir kayıt satırı**. Otorite crate'i değişmez.

Sınama (E): yalnız `qp-session/tests/`'te yaşayan `Tally` alt sistemi — bir `Op` türünü sayar
(açılım kaydı üstünden komut alır), sayıyı motor girdisiyle günlüğe yazar, `replay` aynı
dokümanı verir; `git diff --stat` otorite crate'inin `src/`'sinde değişiklik göstermez.

## 8. Ölçüt eşlemesi

GOAL 7 → §3.1; 8 → §3.2; 9 → §3.3 (12 bin üstü yok; geri dönüşte tek istisna); 10 → G3, S4;
11 → İ2 (§2 sırası), İ3–İ6 (§3), İ1 (Q, kesilebilir); 12 → §7.

## 9. Eleştiriler ve cevaplar

(eleştiriler `experiments/critique-1.md`, `critique-2.md`; cevaplar aşağıda)
