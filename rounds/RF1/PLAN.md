# RF1 — Bölme planı (v2)

Durum: **v3 — onaylandı (`DECISION.md`, 2026-11-23); v2 + §11 düzeltmeleri bağlayıcı.** v1 (`eb86a055`) iki bağımsız eleştiriden geçti
(`experiments/critique-1.md` uygulanabilirlik, `critique-2.md` sonraki round'ların yükü); cevaplar
§9'da. Kod değişmedi. Ölçümler `05430e06` kodunda, `scripts/refactor/graph.py` ile (dökümler ve
eşlemeler `experiments/graph/`). Satırlar `arch-check` sayımıdır.

**v1'den en önemli fark:** v1 kapsül yaşamını, iddiaları ve izlemeyi otoriteden ayırmayı öneriyordu.
Eleştiri ve ölçüm bunun bu round'da davranışı bayt eşit tutarak yapılamayacağını gösterdi. Nedenleri
§9'da (B1–B5): döngüler, `as_reader` takası, iki dünya, yeniden giriş, olay sırası. v2'de bu
üçlü **otorite çekirdeğinde birlikte kalır**, bugünkü olay sırası aynen korunur. Dışarı çıkanlar
şunlar: saf işlemler, okuma, görüntüleyici, izleme (ilk gerçek alt sistem), yansı, barındırma. Yeni
round'lar için **alt sistem sözleşmesi** ve sabit adlı **fazlar** kurulur.

## 0. Değişmez ve yöntem

- Davranış değişmez. `scripts/refactor/compare.sh` (270 kalem) her birleştirmeden önce dalda,
  sonra `main`'de bayt eşit çıkmalı. Protokol, şema, komut satırı ve MCP yüzeyi aynı kalır.
  `qp-schema` belge yorumlarına (eskimiş olsalar bile) dokunulmaz: şema dışa aktarımına giriyorlar.
- **Taşı, sonra yeniden yaz.** Taşınan öğe `scripts/refactor/moved.py` ile "değişmedi" çıkar.
  Yeniden yazılan her öğe sayılır ve listelenir (`experiments/rewritten.md`).
- Gövdesi değişecek her yer için önce karakterizasyon sınaması yazılır, ayrı commit olarak
  değişiklikten önce girer. Sınamaların adı `experiments/new-tests.txt`'e yazılır.
- Her iş kendi dalında ve worktree'sinde yapılır. Yazan ajan ile denetçi ajan ayrıdır. Kesin
  hüküm `target/wt/<iş>` altında verilir. Aynı crate'e aynı anda tek ajan yazar.
- Dışa açık Rust yolu değişiklikleri `experiments/paths.md`'ye yazılır.

## 1. Bugünkü grafik (ölçülmüş; v1'den aynen)

### 1.1 `qp-session` (31.999 satır)

- **Bileşen.** 84 dosya; `Session`'ın 57 alanı ve 295 yöntemi var. **Güçlü bağlı bileşen 25.895
  satır** (31 grup).
- **Bileşeni bağlayanlar:**
  - `impl Session`'ın 35 dosyaya dağılması;
  - iki dünya takası (`public.rs:58-192`, 16 alan);
  - `as_reader` takası (`project.rs:452-540`);
  - `conns` (20 grup okuyor);
  - yeniden giriş zinciri (`run_capsules → engine_entry_with → committed → …`, `accept.rs:151-231`).
- **Yöntem arayüzü.** Otorite → kapsül tarafı 35 yöntem; okuma → (kapsül, otorite, veri) 9/5/3;
  görüntüleyici ← otorite 6.
- **İşlemler (`ops/*`).** Test dışı kodda `Session`'ı çağırmıyor. v1'deki "3 yöntem" ölçüm
  artığıydı: `E::build` ve bir trait yöntemi. Dışarı bağları yalnız birkaç serbest öğe:
  - `watch::{intent_visible, visible_ids, expand_watch, WATCH_PREFIX}`
  - `live::closure_of`
  - `session::ENGINE_PRINCIPAL`
  - `present`'in `DocProvider` tipi

### 1.2 `qp-grammar` (20.964)

- **Döngü A (13.207):** `compile`, `present`, `provider`, `faces`, `open`, `transitions`.
- **Döngü B:** `look` ↔ `advances`.
- **Dışarıda (5.581):** `glyph`, `stories`, `feeds`, `theme`, `detmath`, `error`.
- **Derleyiciden tasarıma gerçek çağrı noktası 5:**
  - `scene.rs:46` başlık şablonu
  - `prepare.rs:100,117` açılmış kapsül metni
  - `terms.rs:149,182` katman terimi
- **`present/depth` ve `present/motion`:** tasarımlara bakan ince sarmalayıcılar; tasarımla
  birlikte yukarı gidebilirler.

### 1.3 `qp-capsule` (13.230)

`voice/` 4,75 bin; iki yönde de kenar yok, kullandığı her öğe `pub` (eleştiri 1 doğruladı).

## 2. Katman içi sıra (A2; `rf1/a-tools` dalında, denetimde)

- `arch-config.json`'da her katmanın bir `order` listesi var. Aynı katmanda `a → b` kenarı yalnız
  `b` sırada `a`'dan önceyse geçer. Yapılandırmanın kendisi `layer.order` kuralıyla denetlenir.
- İ2'nin 5 kalemi kapandı.
- `fn.lines` ve `file.hard` istisnaları tavanlı.
- `verify.sh`'ta `config` adımı var: `main`'e göre yapılandırma ve taban çizgisi farkı. Karar
  numarası anmayan gevşetme kırmızıya düşer.

## 3. Hedef crate'ler

### 3.1 Gramer (alan; sıra: `qp-source < qp-facts < qp-look < qp-glyph < qp-grammar < qp-designs < qp-capsule < qp-voice`)

| Crate | İçerik | Tahmini satır | Bağımlı |
|---|---|---|---|
| `qp-look` | görünüş kaydı ve tipleri (`look/*`, `advances` üretilmiş), `theme`, gramer hatası (`error`), `detmath` | ≈ 2,8 bin | qp-schema |
| `qp-glyph` | anlam işareti çizimi (`glyph/*`) | ≈ 2,0 bin | qp-look |
| `qp-grammar` | **derleyici:** `compile/*` (alfabe dahil), `present/{depth,gesture}` çekirdeği, `provider`; tasarım kaydının arayüzü (`Designs`: 5 çağrı noktasının fonksiyonları) ve o noktaların tipleri (`HeadlineFn`, `DeviceCheck`) | **≈ 7,5 bin → R7'ye ≈ 4,5 bin pay** | qp-look, qp-glyph, qp-calendar |
| `qp-designs` | **adlı tasarımlar:** yüzler, açılmış kapsül, hikâye çizelgeleri, akış düzenleri, geçişler, hareket tasarımları (`present/motion`), `depth`'in tasarım sarmalayıcıları; varsayılan kayıt | ≈ 9,4 bin | qp-grammar |

**Enjeksiyon dar tutulur:** yalnız 5 çağrı noktası enjekte edilir.

- **Biçim:** `Designs` bir fonksiyon göstergesi tablosudur (`dyn` yok).
- **Kim verir:** `CompileOptions`'ın derleyici dışından gelen tek alanıdır. Bugünkü ≈ 70
  `CompileOptions::new` çağrısı aynı kalır. Bunun için `qp-designs` bir kurucu sunar; çağıranların
  yolu `qp_designs::options(…)` olur. Bu yol değişiklikleri listelenir.
- **Türetmeler:** `Debug`/`PartialEq` türetmeleri korunur. Eşitlik için tablo kimliği (adı)
  karşılaştırılır.
- **WASM:** varsayılan paket bugün hikâye, geçiş ve hareket kodu taşımıyor. Tablo yalnız 5
  fonksiyonu gösterdiği için bunlar pakete girmez. G2b'de ölçülür; ±%2 dışına çıkarsa durulur.
- **Doğrulama:** `cargo tree -p qp-grammar` tasarım crate'ini göstermez.
- **Kopya kayıt yok:** `FACES`, `TRANSITIONS`, `PRESENTED_*` tasarımla birlikte yukarı gider.
  Derleyicinin bunlara başvurusu 5 noktadan ibarettir.

**R7 işaretleri** (seviye etiketi, kırılım çentiği, kanıt bandı, niyet etiketi, tutamaç)
derleyicinin alfabesine girer. Alfabe bugün 926 satır; pay ≈ 4,5 bin. Çizim türlerinin
geometrisi gramerin altında bir alan crate'inde doğar (§5).

**`prepare` (885, İ6).** Adlandırılmış adımlara bölünür:

- `layer_source`
- `capsule_texts`
- `check_encoding`
- `mark_rows` (işaret başına kol fonksiyonu)
- `finish`

Aradaki değişkenler `PrepState`'te toplanır. Karakterizasyon, katman işareti (8) × veri kaynağı ×
görünüş başına `Prepared` dökümüdür.

### 3.2 Ses

`qp-voice`, `voice/*`'ın saf taşımasıdır (4,75 bin). Sonrasında `qp-capsule` ≈ 8,5 bin.

### 3.3 Oturum (oturum katmanı; sıra aşağıdan yukarı)

| # | Crate | İçerik | Tahmini | Tür |
|---|---|---|---|---|
| 1 | `qp-ops` | **işlemler:** açılım (`expand`, `gops`), seçiciler (`select`, `locate`), kanonik biçim, özet, değişim uygulama ve `replay`, hata kurucusu (`error`, `suggest`), değer önbelleği, veri düzlemi (`plane`), sunucu (`present`), veri sağlayıcının tipi; ayrıca izleme ve canlı yolun saf yardımcıları (§1.1) ve `ENGINE_PRINCIPAL` | ≈ 10,2 bin | saf fonksiyonlar |
| 2 | `qp-subsystem` | **alt sistem sözleşmesi** (§4): `Subsystem`, `SessionView`/`SessionCtx`, fazlar, tipli durum yuvaları (`Slots`, `PerWorld<T>`), işlem kaydı (`OpHandler`: açılım + politika), sorgu, girdi ve konu kaydı | ≈ 0,8 bin | yeni |
| 3 | `qp-watch` | **izleme niyeti** (`watch.rs`): ilk gerçek alt sistem, açılımı ve politikasıyla kayıt üstünden | ≈ 0,4 bin | alt sistem |
| 4 | `qp-read` | **okuma ve sunum:** `query`, `interact`, `reading`, `preview`, `explain`, bağ sorguları (`bindq`), kapsül sorguları (`capsule/queries`) | ≈ 3,9 bin | alt sistem (sorgu ve girdi) |
| 5 | `qp-authority` | **otorite çekirdeği:** alım, dağıtım, komut yolu, kabul, uzlaşma, saat, varlık durumu, olaylar, günlük, bağlantılar, ajan politikası; iki dünya ve `as_reader`; kapsül yaşamı (`capsule/*`), iddia/itiraz/yorum, canlı yol (`data/live`), veri sağlayıcı; kayıt üstünden alt sistem çağrıları (fazlar) | **≈ 11,5 bin** (saf kapsül dizinleri `capidx`, `headparts`, `sentences` ≈ 1,3 bin aşağı iner; inmezse ≈ 12,8 bin) | otorite |
| 6 | `qp-replica` | istemci yansısı (`Replica`, `follow`, `judge`, `tape`); kendi otoritesini `qp-read` + `qp-watch` kaydıyla kurar | ≈ 1,7 bin | istemci (yeniden yazma: iç alanlara erişim → `#[doc(hidden)] pub` yöntem) |
| 7 | `qp-viewer` | görüntüleyici bağlantıları (`replica/viewer*`; açık dilim `Replica` tutar) | ≈ 1,4 bin | alt sistem (konu, bağlantı, veri fazı) |
| 8 | `qp-session` | **bileşim:** `pub struct Session(Authority)` + `Deref`/`DerefMut`; kurucular standart kaydı yapar (izleme, okuma, görüntüleyici); `sub::<T>()`/`sub_mut::<T>()` | ≈ 0,3 bin | bileşim |
| 9 | `qp-replayhost` | `ReplayHost`, `conform` | ≈ 1,9 bin | istemci |

**Saf kapsül dizinleri nereye iner:** `capidx`, `headparts` ve `sentences` önbellek ve dizin
tipleridir. Otoriteyi tanımıyorlar; `capidx → capsule` tek kenar. Bunlar `qp-capsule`'a (alan;
sonrasında ≈ 9,8 bin) ya da `qp-ops`'a iner. Seçim S2'de bağımlılık ölçümüyle yapılır.

**12 bin hedefi.** Ölçülmüş eşleme (`map-v2b.json`): çekirdek 12.842 satır, okuma, görüntüleyici
ve izleme dışarıda. Saf dizinler inince ≈ 11,5 bin.

**Çekirdek yine 12 bini aşarsa:** **tek adlı istisna** (`qp-authority`) olur. Tavan ölçülen değer
alınır ve 13.000'i geçemez. Bunu karar noktasında soruyorum (§10, soru 1).

**Bileşim.** `Session::new(…)` ve `session.handle_raw(…)` çağrıları değişmez. Otorite hiçbir alt
sistemi tanımaz. Kayıtlı olmayan alt sistemin kancası çağrılmaz; yansı kendi otoritesini kendi
kaydıyla kurar.

## 4. Alt sistem sözleşmesi (`qp-subsystem`)

Eleştirilere göre yeniden kuruldu (B3–B5, Y1–Y5). Kurallar:

1. **Davranış durumsuz, durum otoritede.** Alt sistem nesnesi `&'static dyn Subsystem`'dir. Her
   alt sistemin durumu otoritenin sahip olduğu tipli yuvada durur (`Slots`; `TypeId` → kutu).
   Dünya başına durum `PerWorld<T>` tipindedir ve iki dünya takasında otorite onu da takas eder.
   Bunun için `swap_world`'e tek satır eklenir. Böylece kayıttan alt sistem çekip almak ya da
   `RefCell` gerekmez. Yeniden giriş güvenlidir, çünkü yuva her çağrıda bölünmüş ödünçle verilir.
2. **Bağlam bölünmüş ödünçtür:** `SessionCtx<'a> { view: &'a dyn SessionView, slots: &'a mut Slots, out: &'a mut Outbox }`.
   - `SessionView` okuma yöntemleridir: doküman, günlük, saat, kimlik, bağlantı bakışı, veri
     düzlemi, piyasa durumu, `param` (K78), `entitled`, `run_kind`, yeniden oynatma konumu.
   - Yazma yalnız `Outbox` üstündendir: motor girdisi isteği, bildirim, uyandırma.
   - Otorite, fazın yazma adımında `Outbox`'ı bugünkü sırayla uygular.
   - Hiçbir yöntem jenerik değildir (`dyn` uyumlu). Kamu dünyasında koşmak bir bayraktır:
     otorite fazı iki dünya için ayrı çağırır.
   - `Host` adı kullanılmaz; barındırma kavramıyla çakışıyor.
3. **Fazlar adlıdır ve sabittir;** her fazın toplama ve yazma adımı ayrıdır (B5). Fazlar:
   - `Check` (komut denetimi)
   - `Expand` (`OpHandler`)
   - `AfterCommit`
   - `AfterData` (toplama: koşul ve kapsül değerlendirmesinden önce; yazma: bugünkü sırada)
   - `Proposals` (saat: öneri süresi)
   - `Subscribe`/`Unsubscribe`, `Connect`/`Close`
   - `Adjust` (K88 kurumsal işlem planı; RS1'de kullanılır)

   Kayıt sırası belirlenimcidir. Her faz, otorite kodunda **bugün o işin yapıldığı satırda**
   çağrılır. Çekirdekte kalan iş, yani kapsül, iddia ve canlı yol, bugünkü sırayla aynen koşar.
4. **İşlem kaydı açılım ve politikadır** (Y1). `OpHandler` üç şey verir:
   - açılım;
   - yetki sınıfı (`allowed`), önerilebilirlik (`proposable`), ajan sınıfı (`agent_op`);
   - kamu dünyası eşlemesi ve okuyucu gizliliği.

   Otoritedeki her `match op` kayda düşen bir varsayılan kol alır. Kayıtta karşılığı yoksa
   bugünkü hata döner.
5. **Kapalı enum'lar.** Yayılım ölçüldü (eleştiri 2): `ChangeOp` 23 dosya, `Op` 14, `Event` 13,
   `QueryResult` 14, `Topic` 11, `Condition` 10. Bu round'da otorite tarafında `Op`, `Query`,
   `Topic` ve `Condition` (canlı yol değerlendiricisi; RL1'in `qp-program`'ı buradan bağlanır)
   dağıtıcıları kayda düşen varsayılan kol alır. `ChangeOp` uygulaması `qp-ops`'ta kalır. Şema
   değişikliği yok.
6. **Barındırıcı yüzü.** Bugünkü ayarlayıcılar (`set_coin_flip_rule` …) çekirdekte kalır. Yeni
   round'ların ayarlayıcıları kendi crate'lerinde `impl` edilir; bileşim üstünden
   `session.sub_mut::<T>()` ile erişilir. `Session`'a yöntem eklenmez.
7. **Uygulanan ve taslak kancalar.**
   - Uygulanan kancalar: alt sistemlerin (izleme, okuma, görüntüleyici) ve ek noktası sınamasının
     kullandıkları. Hepsi gerçek kodla sınanır.
   - Taslak kancalar: `Adjust` ve `Condition` kaydı. Tipleri ve çağrı noktaları kurulur, birer
     sınamayla davranışı kilitlenir. Kullanım sonraki round'da.

## 5. Sonraki round'lar nereye oturur (eleştiri 2'ye göre güncellendi)

| Round | Alan katmanı | Oturum alt sistemi | Kayıt satırı |
|---|---|---|---|
| RL1 | `qp-lang`: hesap dili, ara gösterim, maliyet (K79; ENGINEERING §2 "hesap dili" alanda) | `qp-program`: `add_program` / `check_program` için `OpHandler` (politika dahil); maliyet bütçesi `Check` fazında `param`/`entitled` ile; canlı değerlendirme, canlı yolun `Condition` kaydına bağlanarak (paralel bir yol değil) | 1 |
| R7 | `qp-drawing`: çizim geometrisi ve türleri; gramerin altında, böylece gramer çizebilir ve K88 taşıyabilir | `qp-watch` büyür: izleme görevleri, yaşam döngüsü, niyet ve netleştirme. `qp-drawings`: çizim nesneleri, çizim → koşul → olay. "Kur ve göster" `qp-read`'de. İşaretler `qp-grammar` alfabesine (≈ 4,5 bin pay) | 1–2 |
| RS1 | `qp-ledger`: sicil hesabı, iddia + sonuç, tutma yüzdesi (ENGINEERING §2'de sicil alandadır; ad `Ledger` kavramından) | `qp-ledger`'ın oturum alt sistemi: `AfterCommit`, `AfterData`. K88 için `Adjust` fazı: her alt sistem bir taşıma planı verir, otorite tek motor girdisi ve tek bildirim yazar | 1 |
| RA1 | — | `qp-prep` (hazırlık görevi). Ortak analizin üretimi ve tekilleştirme belgeler arası olduğu için barındırma katmanında (`qp-host` ya da yeni bir uyarlayıcı); oturumda yalnız üyelik ve kişisel katman (`qp-membership`). Zaman aralığı okuması `qp-read`'e (≈ 8 bin pay) | 1–2 |

Ölçüt: yeni bir `Op` kolu eklenirken dokunulan otorite dosyası 0 olmalı. Şemaya kol eklemek
round'un kendi işidir. Ek noktası sınaması bunu var olan bir `Op` türüyle gösterir (§7).

## 6. Sıra, risk, geri dönüş

| Adım | İş | Tür | Risk |
|---|---|---|---|
| Ö | P8/P9 (`rf1/o-design`, denetimde) → **tutanak v2** (yalnız `design/check.txt` ve yeni fikstürün özeti; kod değişmeden) | araç | düşük |
| A1/A2 | `rf1/a-tools` (denetimde) | araç | düşük |
| V | `qp-voice` | saf taşıma | düşük |
| **K** | **karar noktası** | | |
| G1 | gramer karakterizasyonu: işaret × kaynak × görünüş; yüz, açılmış kapsül, hikâye, geçiş kareleri | test | — |
| G2a | `qp-look`, `qp-glyph` aşağı | taşıma | düşük |
| G2b | `Designs` tablosu + 5 nokta + `qp_designs::options` | yeniden yazma (küçük) | orta: WASM, çağıran yolları |
| G2c | `qp-designs` yukarı (tasarımlar + `motion` + `depth` sarmalayıcıları) | taşıma | düşük |
| G3 | `prepare` adımlara | yeniden yazma | orta |
| S1 | oturum karakterizasyonu: `expand_op` kol başına (başarı/hata); faz başına bildirim dizisi (komut → günlük → bildirim; iki dünyalı ve okuyuculu dökümler) | test | — |
| S4 | `expand_op` → kol başına fonksiyon | yeniden yazma (gövdeler aynen) | düşük |
| S2 | `qp-ops` aşağı (+ saf yardımcılar, `ENGINE_PRINCIPAL`, sağlayıcı tipi); saf kapsül dizinleri aşağı | taşıma + küçük yeniden yazma | düşük |
| S3.0 | `qp-subsystem` + çekirdekte fazlar, yuvalar, `swap_world` satırı; henüz alt sistem yok, davranış aynı | yeni kod | **orta** |
| S3.1 | `qp-watch` (ilk alt sistem) | yeniden yazma | orta |
| S3.2 | `qp-read` | yeniden yazma (≈ 99 `rd!` kullanımı bölünmüş ödünçle) | **orta-yüksek** |
| S3.3 | `qp-replica`, `qp-viewer`, `qp-session` bileşimi, `qp-replayhost` | yeniden yazma | **orta-yüksek** |
| E | ek noktası belgesi + sınama | belge + test | düşük |
| Q, A3 | kesilebilirler (GOAL kesme sırası) | | |

**En riskli üç adım:**

1. **S3.3, yansı ve görüntüleyici.** Yansı oturumun iç alanlarına yazıyor (`replica.rs:229-241`).
   Görüntüleyici `Replica` tutuyor. Yansının kendi kaydıyla kurulması, görüntüleyicisiz otoritenin
   bugünküyle aynı davranmasını ister.
2. **S3.2, okuma.** 50 kenar çekirdeğe gidiyor; `rd!` ve `&mut cache` bölünmüş ödünç gerektiriyor.
3. **S3.0, faz düzeneğinin ilk kurulumu.** Bildirim sırası bugünküyle bire bir kalmalı. Bunu S1
   karakterizasyonu kilitler.

**Geri dönüş:** her adım kendi dalında. Bir alt sistem dalgası tutanağı bayt eşit tutamazsa o
alt sistem çekirdekte kalır. Çekirdek 12 bini aşarsa §10 soru 1 devreye girer. Kesin sınır: tek
istisna, tavan 13.000.

**Kapı:** her dalga sonunda `compare.sh` bayt eşit, `verify.sh` yeşil, `arch-check` yeşil.
Ayrıca her dalgada WASM boyutu bir kez ölçülür (`perf.sh`'ın WASM kısmı) ve ±%2 içinde kalmalı.

## 7. Yeni alt sistem nasıl eklenir (`docs/ARCHITECTURE.md`'ye girecek)

1. Oturum katmanında yeni bir crate açılır ve `arch-config.json` sırasına eklenir. Bağımlılıkları
   `qp-subsystem`, `qp-ops` ve gerekiyorsa sırada kendinden önceki alt sistemlerdir. Otoriteye
   bağımlı olmaz.
2. Durum tipi bir yuvaya girer (`PerWorld<T>` ise iki dünyada ayrı tutulur). Davranış
   `impl Subsystem` ile yazılır: işlem kaydı (`OpHandler`), fazlar, sorgu, girdi ve konu.
3. Günlüğe yazma yalnız açılım ya da `Outbox` motor girdisiyle olur. Yeniden oynatma
   değişimlerden çalışır, alt sistem kodu istemez (`qp_ops::replay`).
4. Yansıya giden kısım protokol mesajıdır; yansı kendi otoritesini aynı kayıtla kurar.
5. `qp-session` bileşimine **bir kayıt satırı** eklenir. Otorite crate'i değişmez.

**Sınama (E), iki parça:**

- **(a) Gerçek alt sistem.** `qp-watch`'ın kayıt satırı bileşimden çıkarılınca `add_watch`
  "bilinmeyen işlem" hatası verir. Satır geri gelince bugünkü döküm bayt eşit çıkar. Otoritenin
  `src/`'si iki durumda da aynıdır.
- **(b) Yalnız testte yaşayan `Tally`.** Kayıt üstünden bir `Op` türü alır, sayısını motor
  girdisiyle günlüğe yazar ve `replay` aynı dokümanı verir. Kendi konusuyla bildirim gönderir.
  `git diff --stat` otorite crate'inde değişiklik göstermez.

## 8. Ölçüt eşlemesi

| GOAL ölçütü | Planda |
|---|---|
| 7 | §3.1 |
| 8 | §3.2 |
| 9 | §3.3; istisna olursa §10 soru 1 |
| 10 | G3, S4 |
| 11 | İ2 → §2; İ3–İ6 → §3; İ1 → Q (kesilebilir) |
| 12 | §7 |

## 9. Eleştiriler ve cevaplar

Eleştiri 1: uygulanabilirlik (`critique-1.md`, 19 bulgu). Eleştiri 2: sonraki round'ların yükü
(`critique-2.md`, 19 bulgu). Kısaltmalar: B = birinci, Y = ikinci.

| Bulgu | Ağırlık | Cevap |
|---|---|---|
| B1 önerilen sınırlar döngülü (izleme ↔ kapsül, iddia ↔ kapsül, görüntüleyici ↔ yansı) | yüksek | **Kabul.** `graph.py --map` eklendi. v1 eşlemesi tek döngü çıktı (`map-v1.json`). v2'de iddia ve kapsül çekirdekte birlikte. İzleme ↔ kapsül kenarı fazla çevrildi: `run.rs`'in `match_watches` çağrısı `AfterData` fazına, izlemenin `reader_proj` çağrısı `SessionView`'e. Görüntüleyici yansının üstüne alındı (`map-v2b.json`: kalan kenarlar yalnız S2/S3'te kesilecek olanlar). |
| B2 `as_reader` yerleşim kancası değil, doküman takası | yüksek | **Kabul.** Takas otoritede kalır. Kapsül yaşamı da çekirdekte. |
| B3 iki dünya, alt sistem durumunu da takas ediyor; `in_world<R>` dyn değil | yüksek | **Kabul.** `PerWorld<T>` yuvası, otorite takası; jenerik yöntem yok (§4.1–4.2). |
| B4 yeniden giriş zinciri | yüksek | **Kabul.** Durumsuz alt sistem + otoritedeki yuvalar + bölünmüş ödünç (§4.1). Zincirin kendisi çekirdekte kalıyor. |
| B5 olay sırası kayıt döngüsüyle korunamaz; `on_clock` yanlış tarif | yüksek | **Kabul.** Adlı fazlar, toplama/yazma ayrı, her faz bugünkü satırında. Saat fazı yalnız öneri süresi (`Proposals`). |
| B6 işlemler → otorite "3 yöntem" ölçüm artığı | orta | Kabul; §1.1 düzeltildi (gerçekte 0). |
| B7 `present` `DocProvider` (`&Session`) kullanıyor | orta | Kabul. Sağlayıcının tipi ve trait'i `qp-ops`'a iner, `&Session` tutan uygulaması çekirdekte kalır (S2). |
| B8 S2b yansı ve barındırıcı düşük riskli taşıma değil | orta | Kabul. S3.3'e alındı, "yeniden yazma, orta-yüksek". |
| B9 `rd!` ve `&mut cache` bölünmüş ödünç | orta | Kabul. S3.2 riski yükseltildi. |
| B10 `Conn`/`Sub` alt sistem tipi taşıyor | orta | Kabul. Bağlantı başına alt sistem durumu yuvaya (`Slots`, bağlantı anahtarlı) gider. Bugünkü alanlar (`Sub.memo/watch/stories`) çekirdeğin alt sistemleri için yerinde kalır. |
| B11 gramer enjeksiyonu ≈ 70 `CompileOptions::new`; türetmeler; WASM | orta | **Kabul.** Enjeksiyon 5 noktaya daraltıldı; fonksiyon göstergesi tablosu; `depth` sarmalayıcıları ve `motion` taşınıyor (§3.1). |
| B12 WASM tek yönlü küçülme de ±%2 dışı | orta | Kabul. Kapı iki yönlü; kayma olursa durup sorarım. |
| B13 şema belge yorumu tuzağı | orta | Kabul. `qp-schema` belge yorumlarına dokunulmaz; eskimişler "Fark edilenler"e (§0). |
| B14 geri dönüşün tetiği yok | orta | **Kabul.** Tetik: dalga tutanağı bayt eşit tutamazsa o alt sistem çekirdekte kalır. Çekirdek > 12 bin → tek istisna; şimdiden soruyorum (§10). |
| B15–B19 (`src` içi testler, `pub(crate)` → `pub`, `Host` adı, `advances` taşıması `config-diff`, dinamik çağrı maliyeti) | düşük | Kabul. Testler kodla birlikte taşınır ya da üst crate'in `tests/`'ine gider. `pub` yapılanlar `paths.md`'ye. `Host` → `SessionView`. `advances` taşıması `rule_changes` gerekçesiyle `DECISION.md`'yi anar. Faz çağrısı K17/K52 ölçümüyle denetlenir. |
| Y1 işlem politikası otoritede | yüksek | **Kabul.** `OpHandler` = açılım + politika (§4.4). |
| Y2 niyet/izleme değerlendirmesi otoritede | yüksek | **Kısmen.** `eval_watchers`/`eval_intents` canlı yolun parçası ve çekirdekte kalır. Bu round'da `qp-watch`'a yalnız `watch.rs` (izleme kaydı ve işlemleri) gider. R7'nin izleme görevleri `qp-watch`'ta doğar ve değerlendirmeye `Condition` kaydı + `AfterData` fazıyla bağlanır. Otoriteye dokunmaz. |
| Y3 iki dünya ↔ alt sistem durumu | yüksek | B3 ile aynı. |
| Y4 kapalı enum yayılımı ölçülmemiş | yüksek | **Kabul.** Ölçüldü (§4.5). Otorite tarafındaki dağıtıcılara kayıt kolu. Ölçüt: yeni kol için otorite dosyası 0. |
| Y5 abonelik ve bağlantı kancası yok | yüksek | **Kabul.** `Subscribe`/`Unsubscribe`/`Connect`/`Close` fazları, konu kaydı. Görüntüleyici bunları gerçekten kullanacak. |
| Y6 `in_world<R>` jenerik; ödünç | orta | B3/B4 ile aynı. |
| Y7 K88 tek alt sistemin işi değil | orta | **Kabul.** `Adjust` fazı: alt sistem başına plan, tek motor girdisi, tek bildirim (taslak kanca). |
| Y8 barındırıcı yüzü (ayarlayıcılar) | orta | **Kabul.** `sub::<T>()`/`sub_mut::<T>()` bileşimde (§4.6). |
| Y9 kurulum ve bağlam (`build(init)`, `limits`, `param`, `entitled`, `run_kind`, yeniden oynatma konumu, uyandırma) | orta | **Kabul.** `Subsystem::init` ve `SessionView` yöntemleri (§4.2). |
| Y10 adlar: `Host` çakışması, `qp-record` → `qp-ledger`, `qp-shared` belirsiz; kavramlar sözlükte yok | orta | **Kabul.** `SessionView`, `qp-ledger`, `qp-membership`. `docs/CONCEPTS.md`'ye kod yazılmadan önce eklenecekler: alt sistem, alt sistem sözleşmesi, faz, bileşim, işlem kaydı, tasarım kaydı (`Designs`), canlı yol (çekirdekte kalıyor ama adı var). "Kapsül yaşamı" crate'i yok, girmez. |
| Y11 katmanlar: sicil ve hesap dili alanda; çizim geometrisi gramerin altında | orta | **Kabul.** §5 güncellendi. |
| Y12 R7 gramer payı ≈ 3,3–4,2 bin gerekebilir, 4,5 bin hedef | orta | **Kabul.** `depth`/`motion` yukarı çıkınca ≈ 4,5 bin (§3.1). Ölçümle teyit G2c sonunda. |
| Y13 RL1 canlı değerlendirme canlı yola bağlanmalı | orta | **Kabul.** `Condition` kaydı (§4.5, §5). |
| Y14 RA1 ortak analiz belgeler arası | orta | **Kabul.** Barındırma katmanına (§5). |
| Y15 yansı ve barındırıcı iç alanlara uzanıyor | orta | B8 ile aynı. |
| Y16 olay sırası, fazlar | orta | B5 ile aynı. |
| Y17 `query` `QueryResult` dönmeli | düşük | Kabul. |
| Y18 `&dyn Designs` `CompileOptions`'a girmesin | düşük | Kabul (§3.1: fonksiyon göstergesi tablosu, kimlikle eşitlik). |
| Y19 `Tally` en kolay yolu sınıyor | düşük | **Kabul.** E'ye gerçek alt sistemin (`qp-watch`) kaydını çıkarma/geri koyma sınaması eklendi (§7a). |

## 10. Arkin'e sorular (karar noktası)

1. **Çekirdek istisnası.** Plan çekirdeği ≈ 11,5 bine indirmeyi hedefliyor. Saf kapsül dizinleri
   inemezse ya da okuma/görüntüleyici dalgası çekirdekte kalırsa çekirdek 12,8–14 bin olur.
   Bu durumda `qp-authority` için **tek adlı istisna** kabul ediliyor mu? Tavan ölçülen değer
   olur, 13.000'i geçemez; aşarsa yeniden sorarım.
2. **Kapsül yaşamı çekirdekte kalıyor.** GOAL "işler kendi durumunu taşıyan alt sistemlere
   ayrılır" diyor. v2'de kapsül yaşamı, iddia/itiraz/yorum ve canlı yol bu round'da alt sisteme
   çevrilmiyor: bayt eşitliği tutmanın güvenli yolu bu. Bu ayrımı ileri bir RF'ye bırakmak kabul
   mü? Yeni round'lar sözleşme üstünden doğar; çekirdeğe kayıt satırı dışında dokunmaz.
3. **Crate adları:** `qp-ops`, `qp-subsystem`, `qp-authority`, `qp-watch`, `qp-read`,
   `qp-viewer`, `qp-replica`, `qp-replayhost`, `qp-look`, `qp-glyph`, `qp-designs`, `qp-voice`.
   Uygun mu?

## 11. Karar noktasından sonra (v3; bağlayıcı)

Arkin v2'yi onayladı (`DECISION.md`). Üç sorunun cevabı ve dört düzeltme plana şöyle işlendi.

**Cevaplar:**

- **Çekirdek istisnası.** `qp-authority` tek adlı istisnadır; tavanı ölçülen değerdir ve
  13.000'i geçemez. Tavan mandallıdır.
- **Çekirdek donuktur.** Sonraki round'lar çekirdeğe kayıt satırı dışında dokunmaz.
- **Kalan ayrım borç olarak yazılır.** Kapsül yaşamı, iddia/itiraz/yorum ve canlı yolun
  ayrılması `docs/DEBT.md`'ye adıyla girer. Aday zamanı: RS1'den sonra.
- **Çekirdek 13.000'i aşarsa yeniden sorulur.** Bu, okuma ya da yansı dalgası çekirdekte kalırsa
  olur. Soruyla birlikte hangi dalganın neden tutmadığı ve çekirdeğin yeni boyutu getirilir.
- **Ad notu.** `qp-drawing` (alan) ile `qp-drawings` (oturum) adlarını R7 ayırt edilir biçimde
  verecek. Bu round'un işi değil.

**D1 — sicilin kapısı.** İki taslak kanca açılır; ikisinin de tipi, çağrı noktası ve birer
sınaması olur:

- **`Settled` fazı (sonuç kesinleşti).** Bugün sonucun yazıldığı satırda çağrılır
  (`capsule/settled.rs`, iddianın ve kapsülün sonucu).
- **İddia türü kaydı.**

Bayt eşitliğini tutmuyorsa bu kancalar yapılmaz. Bunun yerine raporda "RS1 çekirdeğin şu
dosyalarına dokunacak" listesi adıyla verilir.

**D2 — yeni doküman nesnesi.**

- `ChangeOp` için kayıt kurulmaz.
- `docs/ARCHITECTURE.md`'ye "Yeni doküman nesnesi nasıl eklenir" bölümü yazılır: dokunulacak
  dosyaların tam listesi, hedef "otorite 0 dosya".
- `qp-ops`'ta pay en az 2,5 bin olmalı:
  - Veri düzlemi ile değer önbelleği (`plane`, `values`; gerekirse `present`) ayrı bir crate'e
    ayrılır: `qp-plane`. Ayrım S2'de `graph.py --map` ile ölçülür.
  - Hata kurucusu (`error`, `suggest`) iki crate'in de altında kalır: ya `qp-plane`'de ya da
    ölçüme göre en alttaki crate'te.
  - Payı en az 2,5 bin tutmuyorsa nedeni ölçümle gösterilir.

**D3 — değer kaynağı.** Veri düzleminde değer kaynağı için bir kayıt kolu açılır (taslak kanca).
Bir alt sistem "bu adresin değerini ben veririm" diyebilir. RL1'in program çıktısı buradan girer.

**D4 — sıra.**

- Dalga sırası: S3.0 → S3.1 (`qp-watch`) → **E** → S3.2 (`qp-read`) → S3.3 (yansı,
  görüntüleyici, bileşim, barındırma).
- `Tally` sınaması işlem kaydının yanında bir fazı, bir sorguyu ve bir konuyu da kullanır.

**Küçük notlar:**

- **`Deref` yalnız çağrı yerlerini korumak için.** Yeni kod `Deref`'e yaslanmaz, `sub::<T>()`
  kullanır. Bu, `qp-session` README'sine yazılır.
- **Boş kaydın maliyeti.** Kayıt boşken faz çağrılarının maliyeti S3.0 sonunda bir kez ölçülür:
  K17 ve K52.
- **Sözlük önce.** `docs/CONCEPTS.md` adları kod yazılmadan önce girer: alt sistem, alt sistem
  sözleşmesi, faz, bileşim, işlem kaydı, tasarım kaydı, canlı yol, değer kaynağı.
- **Geri dönen dalgalar raporda.** Her biri nedeniyle ve bir sonraki denemenin neye ihtiyaç
  duyacağıyla yazılır.
