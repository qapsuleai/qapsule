# RP2 — Tutarlılık senaryo düzeneği (T7 ön koşulu)

Tarih: 2026-11-15. Kod: `rounds/RP2/experiments/harness/` (bağımsız Rust crate'i, ana
çalışma alanının üyesi değil; `qp-schema` ve `qp-compute`'a yol bağımlılığı, ayrıca
`serde`, `serde_json`, `flate2`). Tasarım `rounds/RP2/RESEARCH.md` §1.3, §4.1–4.3'ü izler.

## 1. Amaç

T7a–c'deki tutarlılık yaklaşımlarını **aynı, belirleyici** bir düzenekte ölçmek: bir otorite,
bir yansı, her birinin **kendi** piyasa verisi beslemesi ve aralarında bir protokol kanalı.
Düzenek dünyayı (kaynağın gerçek olay akışı), iki beslemeyi, kanalı, zamanlayıcıyı ve hakemi
sağlar; yaklaşım yalnızca iki bağdaştırıcı yazar. Ölçüler (M1–M13) **yalnızca** yansının
görüntüsünden, görünen verisinden ve kanal baytlarından hesaplanır; yaklaşımın kendi
beyanına bakılmaz.

Gecikme, titreme, sıra bozulması, kayıp, yineleme, kesinti ve düzeltmelerin hepsi **tohumlu**
bir üreteçten gelir; simülasyonda gerçek saat yoktur (TASKS.md notu). Aynı tohum bayt
düzeyinde aynı görüntü günlüğünü verir (§7).

## 2. Bağımsızlık beyanı

- Düzenek, hiçbir aday yaklaşım (A, B, C, D, E) kodu görülmeden ve yazılmadan, T7'den önce
  yazıldı. İçindeki tek yaklaşım **N** (kontrol tabanı, "gelince göster"); aday değildir.
- Düzenek yaklaşımların iç yapısını bilmez: protokol mesajları opak bayttır; görüntü
  anlam anahtarıyla (`ClaimKey`) okunur; hakem iddiaları kendi gerçek verisi üzerinde,
  yaklaşımın atadığı kimliklerden bağımsız sınar.
- Hakem doğruyu **dünyadan** bilir (kayıpsız gerçek bar akışı); yaklaşımlara yalnızca
  beslemeleri ve gecikmeli "kesinleşmiş bar" kaynağı (REST benzetimi) açıktır; hakemin
  bilgisi sızmaz.
- Paylaşılan motor (`engine.rs`) ve başvuru dedektörü (`detector.rs`) bütün yaklaşımlar
  için aynıdır (GOAL değişmezi: otorite ve yansı aynı motor kodunu çalıştırır). Bir
  yaklaşım bunların üstüne kendi defterini kurabilir, bar uygulamasını, RSI'ı ya da
  dedektörü yeniden yazamaz.

## 3. Bileşenler ve arayüzler

| Bileşen | Dosya | Ne yapar |
|---|---|---|
| Senaryo | `spec.rs` | `ScenarioSpec`: bütün parametreler; JSON'a/JSON'dan |
| Dünya | `world.rs` | Tohumlu, sanal zamanlı gerçek kaynak akışı: geçmiş barlar, tik tik canlı barlar, kapanış işaretleri, düzeltme, bölünme, kendiliğinden `REFRESH` |
| Besleme modeli | `feed.rs` | Dünyayı bir alıcıya (otorite ya da yansı) teslim eder: gecikme, titreme, sıra bozulması, kayıp (hedefli: barı kapatan tik, en yüksek tik), yineleme, kesinti + yeniden bağlanınca anlık görüntü, sıkıştırma, f32/f64, bozuk fiyat |
| Kanal | `channel.rs` | Opak bayt mesajları, iki yön; gecikme, titreme, sıra bozulması, yineleme, kopma pencereleri; ham + mesaj başına deflate + akış deflate bayt sayımı |
| Bağdaştırıcılar | `adapter.rs` | `AuthorityAdapter`, `ReplicaAdapter`, `Ctx`, `Setup`, `Approach` |
| Görüntü | `display.rs` | `DisplayState` = `ClaimKey → DisplayItem{label, shown_values, evidence}` |
| Motor | `engine.rs`, `series.rs` | Paylaşılan motor: barlar + `qp_compute` akışlı RSI + başvuru dedektörünün iddia kümesi, artımlı |
| Dedektör | `detector.rs` | RSI(14) 70/30 kesişimi; geçersizlik koşulu (S18); iki bağlı iddia (S16) |
| Zamanlayıcı | `sim.rs` | Tek iş parçacıklı öncelik kuyruğu `(VTime, sınıf, tohumlu eş-bozucu, ekleme sırası)` |
| Hakem | `oracle.rs` | Gerçek veri, görünen veri, görüntü günlüğü → M1–M13 |
| Senaryolar | `scenarios.rs` | S0–S19 hazır ayarları |
| Kayıt | `approaches/mod.rs` | Yaklaşım listesi (şimdilik `n`) |

### 3.1 Zaman

`VTime` = sanal mikrosaniye (`i64`). Başlangıç anlık görüntüleri 0'dan itibaren teslim
edilir; canlı faz `LIVE0 = 1 s`'de başlar; canlı bar *j*'nin açılış zamanı
`t = BASE_T + j·interval` (epoch ms, `BASE_T = 1_700_000_040_000`), geçmiş barlar bunun
öncesindedir. Bar kimliği her iki uçta **bar açılış zamanıdır** (`t`), sıra numarası değil.

### 3.2 Dünya

- **Geçmiş:** sürüklenmesi 12–24 barlık üçgen dalga izleyen rastgele yürüyüş (bar başına
  6 alt adım; fiyatlar kuruşa yuvarlı). RSI(14) düzenli olarak 70'i ve 30'u keser.
- **Canlı bar:** ilk tik `bar` olayı (yeni bar); sonraki her tik `patch`, **yalnızca değişen
  alanlarla** (`c`; aşıldıysa `h`/`l`; `v` birikimli). Her olay beslemenin bir sonraki
  kaynak `seq`'ini taşır. Tik sayısı bar başına `ticks_per_bar × U(0,7; 1,3)`.
- **Yönlendirme (`Shape::Steer`):** RSI'ı hedef seviyelere götürür (Wilder özyinelemesi tersine
  çevrilerek fiyat bulunur; yalnızca yolu biçimlendirmek için — iddialar her zaman
  `qp_compute` ile yargılanır): 70 çevresinde salınım (S6, S7, S11, S12, S15, S16, S19),
  kapanış yarışı (`race`: bar içinde seviyenin `margin` altında kalır, kapanıştan
  `before_close_ms` önceki son tik `cross_p` olasılıkla seviyeyi geçirir), 74 ↔ 60
  döngüsü (S18).
- **Müdahaleler** canlı bar *j*'nin ortasında (bar başından %45 sonra):
  düzeltme (`drop_claim`, `keep_claim`, `old{bars_back}`, `create_claim`) → tek `patch`
  olayı; bölünme (`rescale`) → bütün geçmişin ölçeklenmiş **anlık görüntüsü** (gerçek
  kaynak geçmişi yeniden gönderir; bar başına yama göndermek ara durumlarda yapay iddialar
  üretiyordu); kendiliğinden `REFRESH` → anlık görüntü.
- **Kapanış (`Close`)** yalnızca hakem içindir; beslemeler teslim etmez — alıcı bir barın
  kapandığını bir sonraki bar gelince anlar.
- Dünya üretiminde yalnızca tam IEEE işlemleri ve `sqrt` var (`sin`/`exp`/`ln` yok; dalga
  üçgen).

### 3.3 Besleme modeli

Teslim **FIFO**'dur (her teslim bir öncekinden en az 1 µs sonra), yalnızca sıra bozmak
için seçilen olaylar ve yinelemeler bu kuralın dışındadır. Kayıp `seq`'te boşluk bırakır;
sıkıştırılmış akış kendi `seq`'iyle yeniden numaralanır (CME'nin sıkıştırılmış akışı gibi);
kesinti bitince (`refresh_on_reconnect`) kaynağın o anki durumu anlık görüntü olarak gelir,
arada kaçan tikler gelmez (Matriks `REFRESH`). Düzeltmeler ve anlık görüntüler rastgele
kayba uğramaz. `window_last: Some(n)` bağın `{"last": n}` penceresidir (alıcının çapası
farklı olur, S12).

`SourceEvent`:

```rust
pub enum SourceEvent {
    Data(qp_schema::data::DataEvent),                       // feed + seq + bar | patch
    Snapshot { feed: FeedKey, seq: u64, bars: Vec<Bar> },   // ilk çözüm, REFRESH, bölünme
}
```

`Snapshot` düzeneğin eklentisidir (şemada ilk barlar bağ çözümünden gelir, PROTOCOL §18.3).

### 3.4 Kanal

Mesajlar opak `Vec<u8>`. Yön başına gecikme dağılımı (`base + U(0, jitter)` + olasılıklı
sıçrama), sıra bozma, yineleme; ortak kopma pencereleri: kopukken gönderilen mesaj tutulur,
yeniden bağlanınca sırayla (ve `dup_on_reconnect_p` ile ikişer) teslim edilir; iki tarafa
`ChannelEvent::{Disconnected, Reconnected}` bildirilir. Bayt: ham, mesaj başına deflate
(ortak bağlam yok), yönün bütün akışının deflate'i (mesajlar `\n` ile).

### 3.5 Bağdaştırıcı arayüzü (kararlı; yaklaşımlar buna karşı yazılır)

```rust
pub trait AuthorityAdapter {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent);
    fn on_replica_msg(&mut self, ctx: &mut Ctx, msg: &[u8]);
    fn on_timer(&mut self, _ctx: &mut Ctx, _token: u64) {}
    fn on_fetch(&mut self, _ctx: &mut Ctx, _reply: &FetchReply) {}
    fn on_channel(&mut self, _ctx: &mut Ctx, _ev: ChannelEvent) {}
    fn work_units(&self) -> u64 { 0 }
}
pub trait ReplicaAdapter {
    fn on_source(&mut self, ctx: &mut Ctx, ev: &SourceEvent);
    fn on_authority_msg(&mut self, ctx: &mut Ctx, msg: &[u8]);
    fn on_timer(&mut self, _ctx: &mut Ctx, _token: u64) {}
    fn on_fetch(&mut self, _ctx: &mut Ctx, _reply: &FetchReply) {}
    fn on_channel(&mut self, _ctx: &mut Ctx, _ev: ChannelEvent) {}
    fn display(&self) -> &DisplayState;
    fn visible_data(&self, binding: usize) -> &[Bar];
    fn work_units(&self) -> u64;
}
pub struct Approach {
    pub name: &'static str,
    pub summary: &'static str,
    pub authority: fn(&Setup) -> Box<dyn AuthorityAdapter>,
    pub replica: fn(&Setup) -> Box<dyn ReplicaAdapter>,
}
```

`Ctx`: `now() -> VTime`, `send(Vec<u8>)` (karşı tarafa mesaj), `set_timer(at, token)`,
`fetch_settled(binding, from_t, to_t) -> id` (sağlayıcının kesinleşmiş bar kaynağından
gecikmeli çekme; cevap `on_fetch` ile: istek anında `settle_lag_ms`'den daha önce kapanmış
gerçek barlar, o ana kadarki düzeltmelerle, `f64`; `rest.delay_ms` sonra).
`Setup`: taraf, bağlar (`id`, o tarafın çözdüğü `DataBinding`, `FeedKey`, aralık),
dedektör yapılandırması, yük hesabı sayısı.

Sözleşme (`adapter.rs` başı): saat, işletim sistemi rastgeleliği, ortam okunmaz; sıralı
haritalar kullanılır; iki taraf veriyi ve dedektörü `Engine` ile tutar; veri yalnızca
kendi beslemesinden ve kendi `fetch_settled`'ından gelir, **kanal bar taşımaz** (K23 —
kanal opak olduğu için bu kural gözden geçirmeyle denetlenir, bayt ölçülür);
`visible_data` kullanıcının grafiğinin *şu an* gösterdiği barlardır; `work_units`
belirleyici sayaçtır (`engine.work()` + `Engine::add_work`).

### 3.6 Paylaşılan motor ve dedektör

`Engine::new(&setup)` (ya da `with_policy`): `apply(&SourceEvent) -> Applied`,
`apply_settled(b, &[Bar])`, `take_deltas() -> Vec<ClaimDelta>`, `claims()`, `claim(key)`,
`bars(b)`, `series(b)` (barlar + RSI sütunu), `has_bar`, `is_closed`, `covers(&[EvidenceRange])`,
`range`, `digest(b, from_t, to_t)` (`qp_schema::data::data_digest`), `evidence_ref(...)`
(`EvidenceRef`), `position(b)` (`DataPosition`), `last_seq`, `gaps`, `missing`, `is_stale`,
`work()`, `add_work()`.

Sıra politikası: `Strict` (PROTOCOL §18.4 harfiyen: boşlukta besleme bayatlar, anlık
görüntüye kadar hiçbir şey uygulanmaz) ve `SeqLww` (varsayılan: yineleme yok sayılır, boşluk
kaydedilir ama olaylar uygulanır; her bar alanı onu son yazan `seq`'i hatırlar, alan yalnızca
daha büyük `seq` ile değişir — geç gelen olay alanı geri sarmaz; elinde olmayan bara gelen
yama barı yamadan kurar). `SeqLww`'nin kayıpsız sıra bozulması ve yinelemede kaynağa
yakınsadığı test edildi.

Hesap, tutulan **ilk bardan** (çapa) başlar; `push` yeni bar, `update_last` canlı bar,
başka her değişiklik baştan yeniden hesap (`series.rs`; rastgele dizilerde `qp_compute::run`
ile bit eşitliği test edildi).

Başvuru dedektörü (`ClaimKey {detector, params, binding, bar_t, direction}`):

| Dedektör | Yüklem (bar *i*) | Olgunluk |
|---|---|---|
| `rsi_cross` `period=14,level=70` `up` | `rsi[i-1] <= 70 && rsi[i] > 70` | kapanmış barda `final`, son (canlı) barda `provisional` |
| `rsi_cross` `period=14,level=30` `down` | `rsi[i-1] >= 30 && rsi[i] < 30` | aynı |
| `rsi_invalidation` (S18) `level=60,upper=70,lookback=20` `down` | `rsi` 60'ı aşağı keser ve `[i-20, i-1]` içinde 70'in yukarı kesişimi var — "aşırı alım" bulgusunun başvuru geçersizlik koşulu | aynı |
| `rsi_pair_overbought` (S16) `px0+px1` `up` | aynı `t`'de iki bağın RSI'ı > 70 ve bir önceki barda ikisi birden değil | iki bar da kapanmışsa `final` |

İddianın kanıt aralığı (`EvidenceRange`): iddia barı ve önceki `period` bar.

### 3.7 Görüntü

```rust
pub enum Label { Provisional, Final, Disputed, Pending, SourceDiffers }
pub struct DisplayItem { key: ClaimKey, label: Label, shown_values: BTreeMap<String, f64>,
                         evidence: Vec<EvidenceRange> }
pub struct DisplayState { items: BTreeMap<ClaimKey, DisplayItem> }
```

**İddia eden** etiketler: `Provisional`, `Final`, `SourceDiffers` (yansının kendi değerleriyle,
"veri kaynağınız sunucudan farklı" notuyla). `Disputed` ("kaynaklar farklı" işareti) ve
`Pending` ("sunucu bir şey buldu, veri geliyor") bir şey iddia etmez. Düzenek her sanal
anın sonunda `display()`'i okur ve ardışık durumlardan görüntü günlüğünü çıkarır:

```
<vtime_us> <detector>[<params>]@<binding>:<bar_t>:<up|down> <eski etiket|-> -> <yeni etiket|-> <değerler>
```

### 3.8 Zamanlayıcı

Aynı sanal anda sınıf sırası: dünya adımı (0; gerçek önce ilerler), besleme teslimi (1),
kesinleşmiş bar cevabı (2), kanal mesajı ve kanal durumu (3), zamanlayıcı (4). Sınıf içinde
tohumlu eş-bozucu (dünya adımlarında üretim sırası). Yansı **her sanal anın sonunda** bir kez
örneklenir (ekran anlar arasında çizilir): aynı anda gelen veri ve mesaj tek bir görüntü
üretir.

## 4. Senaryolar

"Gerçekçi" ağ tabanı (aksi yazılmadıkça): otorite beslemesi 20 ms + U(0, 20) ms, yansı
beslemesi 80 ms + U(0, 60) ms, kanal her yönde 40 ms + U(0, 30) ms. Ortak varsayılanlar:
1 bağ, 1 dk bar, 400 geçmiş bar, 60 canlı bar (+ sonuncuyu kapatan açılış tiki), bar başına
ortalama 30 tik, oynaklık %0,4, canlı fiyat adımı 0,01, serbest yürüyüş, REST 800 ms gecikme
ve 2 s kesinleşme, Δ_miss 30 s, Δ_stale 5 s, engelleme eşiği 5 s, düzeltme penceresi 30 s,
geçici yanlışlık toleransı 0 ms, boşaltma 90 s, 100 tohum. Tam değerler: `harness spec <S#>`.

| # | Parametreler (tabandan farkı) | Neyi sınar |
|---|---|---|
| S0 | gerçekçi taban, başka hiçbir şey (RESEARCH'te yok; karşılaştırma için) | Ağ asenkronluğunun tek başına N'ye etkisi |
| S1 | gecikme yok, kayıp yok | Kimlik; bütün yaklaşımlar N ile aynı göstermeli |
| S2a / S2 / S2c | yansı beslemesi sabit 0,5 s / 5 s / 60 s (+%5 titreme) geride; protokol hızlı | Erken gösterim |
| S3 | kanal 3 s + U(0, 1) s; yansı beslemesi 20 ms + U(0, 20) ms | Gecikme |
| S4 | kanal 50 ms + U(0, 400) ms, %2 olasılıkla ~2 s sıçrama, %10 sıra bozma (≤1,5 s), %5 yineleme (≤1 s); 300–330 s kopma, yeniden bağlanınca %30 yineleme | `seq` tamponu, tekdüzelik, yineleme |
| S5 | bar başına 60 tik; yansı beslemesinde %20 sıra bozma (≤0,8 s) | Canlı barda farklı ara durumlar |
| S6 | 70'te yönlendirme + kapanış yarışı; yansı beslemesinde %3 rastgele kayıp, en yüksek tikin %30, barı kapatan tikin %50 kaybı | Kalıcı bar farkı |
| S7 | bar başına 3600 tik (60/s), 8 canlı bar; RSI bar içinde 70 ± 2,5 salınır (bar başına 6 salınım) | Geçici iddia, yanıp sönme, geri çekme |
| S8 | 70'te kapanış yarışı (margin 1, son tik kapanıştan 150 ms önce, %50 geçer); iki beslemede de barı kapatan tikin %25 kaybı | Kesinleşme doğruluğu |
| S9 | 1200 geçmiş bar; canlı bar 6: (a) `drop_claim`; 11: (b) `keep_claim`; 16: (c) 1000 bar önce; 21, 31, 41: (d) `create_claim` (eklendi: iddia doğuran düzeltme); 26: (a); 36: (b) | Geri çekme, önek kırılganlığı, M8, M11 |
| S10 | canlı bar 8'de bütün geçmiş ×0,5 (anlık görüntü olarak) | Yüklem ayakta, bütün özetler değişir |
| S11 | 70'te salınım; yansı beslemesi 240–270 s kesik (dönüşte `REFRESH`); kanal 250–265 s kopuk | Toplu doğrulama |
| S12 | 3000 geçmiş bar; yansı penceresi son 300 bar; gecikme yok; 70 ± 1,5 | Farklı hesap başlangıcı (bilinçli bağ hatası) |
| S12s | S12, yansı penceresi son 60 bar (eklendi) | Çapa farkının yükleme ulaştığı durum |
| S13 | yansı beslemesinde %10 yineleme (≤2 s); kanalda %10 yineleme (≤3 s) | Eşgüçlülük |
| S14 | otorite beslemesinde %2 bozuk fiyat (±%1), %5 kayıp, barı kapatan tikin %30 kaybı; yansınınki doğru | Otoritenin yanlışını kim göstermez |
| S15 | yansı fiyatları f32, otorite f64; gecikme yok; 70 ± 1 | Bit eşitliğinin hiç olmadığı durum |
| S15k | S15 bıçak sırtında (eklendi): canlı fiyat yuvarlanmaz, her yönlendirilmiş bar RSI'ı 70'in 1e-6 yakınında kapatır | f32 farkının yükleme ulaştığı durum |
| S16 | 2 bağ, iki bağlı iddia açık; 72 ± 2; yansının bağ 1 beslemesi 5 s geride | Konum vektörü |
| S17 | 50 bağ, bağ başına bar başına 1200 tik (toplam ~1000 tik/s), bağ başına 10 ek hesap (toplam 500), 6 canlı bar, 300 geçmiş bar; **20 tohum** | M9, M10 |
| S18 | geçersizlik dedektörü açık; RSI 74'te 3 bar ↔ 60'ta 5 bar (60 ± 2,5, bar başına 4 salınım) | Bulgu geçersizliği, bayat gösterim |
| S19 | 70'te salınım; yansı beslemesi 300–420 s (2 dk) kesik, dönüşte `REFRESH` | Bekleme durumunun dürüstlüğü |

Kesinti ve kopma pencereleri canlı fazın başına göredir.

## 5. Ölçüler (uygulandığı biçimiyle)

Tanımlar: **görünende tutuyor** = iddia, yansının görünen verisi üzerinde (RSI yansının ilk
görünen barından) başvuru dedektörünün iddia kümesinde. **Gerçekte tutuyor** = gerçek barlarda
(RSI dünyanın bar 0'ından) iddia kümesinde; **gerçekte kesin** = tutuyor ve barı gerçekte
kapanmış. **Gösterim** = bir iddianın iddia eden bir etikete geçişi ya da iki iddia eden
etiket arasında geçişi.

| # | Uygulama |
|---|---|
| M1 erken | Gösterimde: iddianın barı (çift iddiada her iki bar) görünen veride yok, ya da görünen veride ondan önce `period`'dan az bar var (kanıt penceresi görünmüyor), ya da etiket `Final` ve bar görünen veride kapanmamış |
| M2 çelişkili | İddia eden etiket ekrandayken iddia görünende tutmuyor; bölüm başına bir sayılır, süreleri toplanır (ms) |
| M3 yanlış | Gösterimde: `Final` (ya da gerçekte kapanmış bardaki `SourceDiffers`) ve gerçekte kesin değil → M3k; `Provisional` (ya da canlı bardaki `SourceDiffers`) ve gerçekte tutmuyor **ve** son `live_grace_ms` içinde tutmamış → M3g (varsayılan tolerans 0: RESEARCH'teki katı tanım) |
| M4 geri çekilen geçici | `Provisional` → iddia etmeyen/yok, barın gerçek kapanışından önce (hata sayılmaz) |
| M5 kaçan | Sonda gerçekte kesin; kesinleştiği an `τ0 = max(since, 0)` ve `τ0 + Δ_miss ≤ son`; `[τ0, τ0 + Δ_miss]` içinde hiç iddia eden etiketle ekranda olmamış. "uygun" = bu koşula giren iddia sayısı |
| M6 gecikme | İlk `Provisional` gösterim − iddianın gerçek canlı barda ilk tuttuğu an; ilk kesin gösterim − gerçekte kesinleştiği an. Yalnız canlı faz iddiaları, yalnız gösterim anında doğruysa. p50/p95/p99, bütün tohumlar havuzlanır |
| M7 bayat | İddia eden etiket ekrandayken gerçekte yanlış olduğu süre (`Final` için: gerçekte kesin değil); Δ_stale'den uzun bölümler ayrıca sayılır. Gösterim anında yanlış olan da (M3) bu süreye girer |
| M8 yanlış engelleme | Gerçekte kesin + görünende kesin + iddia edilmiyor: süre toplanır, eşikten (5 s) uzun bölümler sayılır |
| M9 bayt | Yön başına mesaj, ham bayt, mesaj başına deflate, akış deflate. Özet: a→r ham bayt / gerçekte kesinleşmiş farklı iddia sayısı; canlı saniye başına ham, mesaj başına deflate, akış deflate; r→a ham |
| M10 yansı işi | `work_units()` (belirleyici; motor: hesap adımı × bar, dedektör değerlendirmesi, özetlenen bar) / yansı geri çağrı sayısı. Duvar saati ayrıca (`m10_wall_ns`, belirleyici değil, günlüğe girmez) |
| M11 düzeltme | Her düzeltme/bölünme anında gerçekteki üyeliği değişen iddialar; ekran gerçeğe uyana kadar geçen süre (gösteriliyor ⇔ doğru); pencere (30 s) içinde, barı düzeltmeden önce kapanmış ve değişmemiş iddialardaki ekran değişimleri ("gereksiz"); çözülmeyenler |
| M12 yanıp sönme | İddia eden ↔ etmeyen geçiş sayısı; toplam ve tek iddiadaki en çok |
| M13 uyuşmazlık süresi | `Disputed` ya da `SourceDiffers` etiketinde geçen süre |

"Hata" (tablodaki "Hatalı tohum") = M1 + M2 + M3 > 0. **Canlı hata** = canlı faz başladıktan
(`VTime ≥ LIVE0`) sonra olan M1/M2/M3 olayları; başlangıçtaki toplu gönderimi (otoritenin
bütün geçmiş iddiaları, yansının ilk anlık görüntüsünden önce gelebilir) ayırmak için.

Hakemin artımlı iddia kümeleri (gerçek ve görünen) her koşunun sonunda hata ayıklama
derlemesinde toplu yeniden hesapla karşılaştırılır (`Oracle::self_check`; `cargo test`
bütün hazır ayarlarda çalıştırır).

## 6. Kullanım

```bash
cd rounds/RP2/experiments/harness
cargo build --release
./target/release/harness list
./target/release/harness run --approach n --scenario all --seeds 100     # tablo + harness-results-n.json
./target/release/harness run --approach n --scenario S7 --scenario S8 --seeds 20 --out s78.json
./target/release/harness log --approach n --scenario S8 --seed 84         # görüntü günlüğü + ölçüler
./target/release/harness determinism --approach n --scenario S7 --seeds 3
```

`--seeds` verilmezse senaryonun varsayılanı (100; S17: 20). Varsayılanı 100'den az olan
hazır ayar (S17) `--seeds`'i kendi varsayılanıyla sınırlar. `--seed-base B` tohumları
`B..B+N` yapar; `--threads T` paralel tohum sayısı (sonuçları değiştirmez; test edildi).
JSON çıktısı her senaryo için `spec`, `summary` ve tohum başına `runs` içerir (en kötü tohum
`summary.worst_seed`).

### 6.1 Özel senaryo (denetçi için)

```bash
./target/release/harness spec S9 > mine.json      # bir hazır ayarı JSON olarak al
# düzenle: ör. "replica_feed": {"delay": {"base_ms": 700, "jitter_ms": 300}, "reorder_p": 0.2, ...}
./target/release/harness run --approach n --spec mine.json --seeds 100
```

Eksik alanlar varsayılanla dolar (`{"name":"x","replica_feed":{"delay":{"base_ms":700}}}`
geçerli bir senaryodur); bilinmeyen alan reddedilir. Birden çok `--spec` ve `--scenario`
birlikte verilebilir.

### 6.2 Yaklaşım ekleme

1. `src/approaches/<ad>.rs`: iki bağdaştırıcıyı yaz; iki taraf da `Engine` kullansın.
2. Aynı dosyada `pub fn register() -> Approach { Approach { name: "<ad>", summary: "...",
   authority: |s| Box::new(...), replica: |s| Box::new(...) } }`.
3. `src/approaches/mod.rs` içindeki tek satıra modül adını ekle:
   `approaches!(n, <ad>);` — bu kadar (makro modülü bildirir ve kayda ekler).
4. `harness run --approach <ad> --scenario all`.

Örnek: `src/approaches/n.rs` (≈120 satır). Ayrı dallardaki yaklaşımlar birleşirken yalnız bu
satır çakışır; çözümü listeyi birleştirmektir.

## 7. Belirleyicilik

- Rastgelelik tek kaynaktan: SplitMix64, tohum + akış adı (`world/<b>`, `feed/a/<b>`,
  `feed/r/<b>`, `chan/a2r`, `chan/r2a`, `sched`). Normal dağılım 12 tekdüze sayının toplamı
  (Irwin–Hall); `ln`/`exp`/`sin` yok.
- Simülasyonda duvar saati yok; `HashMap` yok (`BTreeMap`/`BTreeSet`); sayılar günlüğe en kısa
  geri dönüşlü biçimde yazılır.
- Testler (`tests/harness.rs`): her hazır ayarda iki tohum (S17'de bir) iki kez koşulur →
  görüntü günlüğü baytları ve ölçüler (duvar saati hariç) aynı; farklı tohum farklı günlük;
  1 ve 4 iş parçacığı aynı sonuç.
- CLI kanıtı (`harness determinism --scenario S7 --scenario S11 --scenario S9 --seeds 3`):

```
S7 seed 0: 6466 lines, hash 6e48d0c5ba524b58 / 6e48d0c5ba524b58 -> identical
S7 seed 1: 3485 lines, hash 70114456b6f63482 / 70114456b6f63482 -> identical
S7 seed 2: 2475 lines, hash 60dd3ae89afa5944 / 60dd3ae89afa5944 -> identical
S11 seed 0: 346 lines, hash b41150ecdf65aed8 / b41150ecdf65aed8 -> identical
S11 seed 1: 365 lines, hash 2766c2ef9834b99b / 2766c2ef9834b99b -> identical
S11 seed 2: 414 lines, hash 89ab419b378e1803 / 89ab419b378e1803 -> identical
S9 seed 0: 56 lines, hash bcc7bace61e4fcd0 / bcc7bace61e4fcd0 -> identical
S9 seed 1: 47 lines, hash 020e54fa60197826 / 020e54fa60197826 -> identical
S9 seed 2: 108 lines, hash 70d99e63504dcfbb / 70d99e63504dcfbb -> identical
```

Sınır: aynı tohumun **farklı platformlarda** aynı günlüğü vermesi beklenir (dünyada yalnız tam
IEEE işlemleri; `qp_compute` hedefler arası bit-eşit), ama bu oturumda yalnız bir makinede
(aarch64 macOS) denendi.

## 8. Kontrol tabanı N — sonuçlar

N: otorite paylaşılan motoru kendi beslemesinde çalıştırır, iddia kümesindeki her değişikliği
anında JSON olarak yollar (`show` iddia + olgunluk, `hide`); yansı kendi beslemesini yalnız
grafik için tutar ve gelen her şeyi geldiği anda gösterir, hiçbir şeyi sınamaz.

Komut: `harness run --approach n --scenario all --seeds 100` (S17: 20 tohum). Kısaltmalar:
M3k/M3g = kesin/geçici yanlış; M6 değerleri ms; M7 saniye (parantezde Δ_stale'i aşan bölüm);
M9 = a→r ham bayt/iddia · ham B/s · akış-deflate B/s.

| Senaryo | Tohum | Hatalı tohum | Canlı hata (tohum) | M1 | M2 (adet / s) | M3k | M3g | M4 | M5 kaçan/uygun | M6g p50/p95 ms | M6k p50/p95 ms | M7 s (>Δ) | M8 | M9 B/iddia · B/s · akış-deflate B/s | M10 iş/olay | M11 p50 ms/çözülmeyen/gereksiz | M12 top/en çok | süre s |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| S0 | 100 | 97 | 511 (87) | 1057 | 1306 / 37 | 0 | 2 | 250 | 0/1202 | 85/101 | 85/102 | 21 (0) | 0 | 942 · 3.1 · 0.3 | 2.4 | –/0/0 | 1702/12 | 0.1 |
| S1 | 100 | 0 | 0 (0) | 0 | 0 / 0 | 0 | 0 | 250 | 0/1202 | 0/0 | 0/0 | 0 (0) | 0 | 942 · 3.1 · 0.3 | 2.4 | –/0/0 | 1702/12 | 0.0 |
| S2a | 100 | 100 | 553 (87) | 1212 | 1453 / 614 | 0 | 2 | 250 | 0/1202 | 85/101 | 85/102 | 21 (0) | 0 | 942 · 3.1 · 0.3 | 2.4 | –/0/0 | 1702/12 | 0.1 |
| S2 | 100 | 100 | 593 (87) | 1239 | 1466 / 6699 | 0 | 2 | 250 | 0/1202 | 85/101 | 85/102 | 21 (0) | 0 | 942 · 3.1 · 0.3 | 2.4 | –/0/0 | 1702/12 | 0.1 |
| S2c | 100 | 100 | 1039 (87) | 1600 | 1551 / 74790 | 0 | 2 | 250 | 0/1202 | 85/101 | 85/102 | 21 (0) | 0 | 942 · 3.1 · 0.3 | 2.4 | –/0/0 | 1702/12 | 0.1 |
| S3 | 100 | 67 | 330 (67) | 0 | 241 / 536 | 0 | 89 | 238 | 0/1202 | 3548/3972 | 3561/3964 | 539 (0) | 0 | 942 · 3.1 · 0.3 | 2.4 | –/0/0 | 1702/12 | 0.1 |
| S4 | 100 | 72 | 292 (70) | 64 | 319 / 1943 | 0 | 15 | 248 | 0/1202 | 285/1323 | 310/1594 | 1965 (2) | 0 | 942 · 3.1 · 0.3 | 2.4 | –/0/0 | 1702/12 | 0.1 |
| S5 | 100 | 99 | 793 (90) | 1093 | 1569 / 171 | 0 | 13 | 504 | 0/1215 | 83/103 | 86/101 | 42 (0) | 0 | 1824 · 6.2 · 0.3 | 2.5 | –/0/0 | 2225/16 | 0.1 |
| S6 | 100 | 100 | 3607 (100) | 2691 | 2770 / 3059903 | 0 | 0 | 0 | 0/3028 | 85/102 | 84/102 | 0 (0) | 0 | 516 · 4.3 · 0.4 | 2.5 | –/0/0 | 3028/1 | 0.1 |
| S7 | 100 | 100 | 23869 (100) | 1175 | 16988 / 242 | 0 | 7560 | 15833 | 0/1232 | 84/100 | 87/103 | 282 (0) | 0 | 87453 · 2244.6 · 30.3 | 1.8 | –/0/0 | 32898/196 | 2.3 |
| S8 | 100 | 100 | 6469 (100) | 2881 | 3206 / 1188848 | 330 | 1906 | 1160 | 1045/3028 | 85/103 | 85/102 | 598504 (1634) | 386 | 2488 · 20.9 · 0.7 | 2.2 | –/0/0 | 4635/8 | 0.2 |
| S9 | 100 | 100 | 672 (100) | 2925 | 3467 / 97 | 0 | 6 | 151 | 0/3467 | 85/103 | 85/102 | 33 (0) | 0 | 542 · 5.6 · 0.6 | 9.2 | 85/0/0 | 4243/8 | 0.1 |
| S10 | 100 | 96 | 568 (87) | 1058 | 1360 / 39 | 0 | 4 | 289 | 0/1206 | 84/102 | 87/101 | 24 (0) | 0 | 978 · 3.3 · 0.3 | 2.9 | –/0/0 | 1784/12 | 0.1 |
| S11 | 100 | 100 | 9865 (100) | 2936 | 8740 / 574 | 0 | 43 | 6492 | 0/2559 | 85/102 | 86/102 | 644 (9) | 0 | 3934 · 28.0 · 1.1 | 2.5 | –/0/0 | 15545/11 | 0.2 |
| S12 | 100 | 100 | 0 (0) | 7425 | 7500 / 27682500 | 0 | 0 | 5131 | 0/9674 | 0/0 | 0/0 | 0 (0) | 0 | 1128 · 30.3 · 1.7 | 2.0 | –/0/0 | 19936/10 | 0.3 |
| S12s | 100 | 100 | 948 (76) | 8074 | 9074 / 30158561 | 0 | 0 | 5131 | 0/9674 | 0/0 | 0/0 | 0 (0) | 0 | 1128 · 30.3 · 1.7 | 1.8 | –/0/0 | 19936/10 | 0.3 |
| S13 | 100 | 97 | 534 (87) | 1060 | 1310 / 5261 | 0 | 18 | 253 | 0/1202 | 84/103 | 86/100 | 5244 (9) | 0 | 942 · 3.1 · 0.3 | 2.3 | –/0/0 | 1712/14 | 0.1 |
| S14 | 100 | 98 | 1063 (91) | 1050 | 1439 / 61962 | 39 | 389 | 476 | 60/1202 | 102/31132 | 88/106 | 61937 (114) | 60 | 871 · 2.9 · 0.3 | 2.4 | –/0/0 | 2135/11 | 0.1 |
| S15 | 100 | 0 | 0 (0) | 0 | 0 / 0 | 0 | 0 | 3935 | 0/2559 | 0/0 | 0/0 | 0 (0) | 0 | 2873 · 20.4 · 0.8 | 2.2 | –/0/0 | 10429/11 | 0.2 |
| S15k | 100 | 100 | 1489 (100) | 0 | 1489 / 2721586 | 0 | 0 | 0 | 0/2568 | 0/0 | 0/0 | 0 (0) | 0 | 2849 · 20.3 · 0.7 | 2.2 | –/0/0 | 2568/1 | 0.2 |
| S16 | 100 | 100 | 2092 (100) | 2778 | 3423 / 9232 | 0 | 9 | 847 | 0/2767 | 86/102 | 87/104 | 72 (0) | 0 | 1707 · 13.1 · 0.6 | 2.4 | –/0/0 | 4461/15 | 0.2 |
| S17 | 20 | 20 | 3268 (20) | 5879 | 8189 / 190 | 0 | 628 | 2338 | 0/8025 | 85/103 | 89/103 | 137 (0) | 0 | 3523 · 3926.5 · 91.8 | 12.4 | –/0/0 | 12711/73 | 43.1 |
| S18 | 100 | 100 | 7131 (100) | 3098 | 6923 / 190 | 0 | 18 | 4268 | 0/3097 | 85/102 | 85/103 | 362 (0) | 0 | 3174 · 27.3 · 1.3 | 2.1 | –/0/0 | 11633/12 | 0.2 |
| S19 | 100 | 100 | 10105 (100) | 3154 | 8770 / 4205 | 0 | 35 | 6492 | 0/2559 | 85/102 | 86/102 | 550 (0) | 0 | 3934 · 28.0 · 1.1 | 2.5 | –/0/0 | 15545/11 | 0.2 |

### 8.1 Her senaryo N'yi düşürüyor mu?

İlk denetim (RESEARCH §4.3): N'de hata sıfırsa senaryo zayıftır.

- **S1: hata 0 (beklenen).** Gecikme yokken N doğru gösterir; M4 (geri çekilen geçici) > 0
  doğal.
- **S0, S2a/S2/S2c, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12s, S13, S14, S15k, S16, S17,
  S18, S19: N düşüyor**, canlı fazda da (canlı hata > 0). Belirgin olanlar: S2c'de M2
  süresi 74 790 s (yansı 60 s geride); S6'da kalıcı bar farkıyla süreklenen çelişki; S7'de
  7560 geçici yanlış ve tek iddiada 196 yanıp sönme; S8'de 330 kesin yanlış ve 1045 kaçan;
  S11/S19'da kesinti boyunca erken ve çelişkili gösterim; S14'te otoritenin yanlışı (39 kesin,
  389 geçici) yansıya taşınıyor; S16'da geride kalan bağın iddiaları.
- **Senaryoya özgü etkisi tabanla (S0) aynı düzeyde kalanlar — açıklama:**
  - **S10 (bölünme):** ×0,5 ikinin kuvveti olduğu için ölçekleme tamdır, RSI bitleri
    **aynı** kalır, iddia kümesi değişmez; N hiçbir şeyi karşılaştırmadığı için bölünmeden
    etkilenmez. N'deki hatalar S0'daki ağ asenkronluğundandır (canlı hata 568 / S0'da 511).
    S10'un işi "veri özeti değişti ama yüklem ayakta" ayrımını ölçmektir; bu yalnız özet
    karşılaştıran bir yaklaşımda (A) M8/M13 olarak görünür. Bilinçli olarak güçlendirilmedi.
  - **S9 (düzeltmeler):** N iddia düşüren düzeltmeyi gizleyerek uygular (gizlemek hata
    değildir); hatayı yalnız iddia **doğuran** düzeltme üretir — bu yüzden (d) `create_claim`
    eklendi. N'nin S9'daki canlı hataları S0'dan daha yüksektir ama farkın bir kısmı
    1200 barlık geçmişin daha çok başlangıç iddiası taşımasından gelir. M11 (85 ms,
    çözülmeyen 0, gereksiz 0) S9'un ayırt edici ölçüsüdür.
  - **S13 (yineleme):** N'nin `hide`'dan sonra gelen yinelenmiş `show`'u eski iddiayı yeniden
    gösterir (M7 > Δ_stale 9 bölüm, S0'da 0); sayıca S0'a yakın çünkü serbest yürüyüşte canlı
    iddia azdır.
- **S12: canlı hata 0, başlangıç hatası çok (M1 7425).** Pencere dışında kalan geçmiş
  iddiaları (yansıda barı yok) N gösterir — gerçek bir bağ hatası. Canlı fazda hata yok,
  çünkü 300 barlık pencerede Wilder etkisi (13/14)^300 ≈ 2·10⁻¹⁰ kadar söner: bitler farklı,
  yüklem eşiğe 10⁻⁸ yakın olmadıkça aynı. Yükleme ulaşan çapa farkı için **S12s** eklendi
  (son 60 bar; canlı hata 948).
- **S15: hata 0 — açıklama ve güçlendirme.** f32 fiyat farkı ~10⁻⁷ bağıl; kuruşa yuvarlı
  fiyatlarla RSI farkı 10⁻⁵'in altında kalır, 70 ± 1 salınımda yüklemi hiç çevirmez. S15'in
  etkisi bit düzeyindedir (özet ve değer eşitliği karşılaştıran B-katı/A: sürekli
  uyuşmazlık, M8/M13); N bunu göremez. Yüklem düzeyinde etki için **S15k** eklendi
  (yuvarlamasız, kapanış 70 ± 10⁻⁶): canlı hata 1489, 100/100 tohum.

## 9. Çalışma süresi

Apple M4 Max, 16 çekirdek, `--release`, N ile: bütün hazır ayarlar 100 tohum (S17: 10)
**25,7 s duvar** (246,9 s işlemci); S17 tek başına 20 tohum 43,1 s; S7 100 tohum 2,3 s;
diğerleri 100 tohumda ≤ 0,3 s. `cargo test` (hata ayıklama denetimleriyle) ≈ 55 s.
Yaklaşımların yansı işi arttıkça süre artar; S17 bu yüzden 20 tohumla sınırlı.

## 10. Bilinen sınırlar

- Görüntü sanal anların sonunda örneklenir; aynı anda gösterilip gizlenen iddia görünmez.
- M1'in kanıt penceresi denetimi yalnız iddia barının ve önceki `period` barın görünür
  olmasına bakar; RSI'ın bütün geçmişe bağlılığını (çapa) M2 yakalar.
- Başlangıçta otoritenin geçmiş iddiaları yansının ilk anlık görüntüsünden önce gelebilir;
  bu gerçek bir erken gösterimdir ve sayılır. Canlı fazı ayrıca görmek için "canlı hata".
- Bölünme anlık görüntü olarak gelir; bar başına yama biçimi denenip bırakıldı (ara
  durumlarda yapay iddialar üretiyordu — yaklaşımların değil düzeneğin yapıntısı).
- Yaklaşımın kanal üzerinden bar taşımaması (K23) düzenekçe zorlanmaz (kanal opak); bayt
  ölçüsü ve gözden geçirme ile denetlenir.
- Dünya yalnız başvuru dedektörünün iddialarını üretir; bulgu/kapsül modeli yalnız S18'deki
  başvuru geçersizlik koşuluyla temsil edilir.

## Öneriler (kapsam dışı; yapılmadı)

- Denetçi (T11) kendi senaryosunu `--spec` ile eklemeli; düzeneğin kendi hazır ayarlarına
  güvenmemeli.
- Farklı bir platformda (x86_64 Linux) belirleyicilik denemesi: aynı tohum → aynı günlük özeti.
