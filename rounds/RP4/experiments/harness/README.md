# harness — RP4 ölçüm düzeneği M25 (çıkış kuralı), M26 (dedektör eşikleri)

Şartname (bağlayıcı): `SPEC.md` (bu klasör, commit `eb1a8c3`) ve `../SPLIT.md`. Bu crate
onların uygulamasıdır; şartnameyi değiştirmez. Şartnamenin açık bıraktığı noktalar en dar
yorumla uygulandı ve aşağıda **Yorumlar** başlığında tek tek yazılı.

## Bağımsızlık beyanı

Düzenek hiçbir alternatif görülmeden ve hiçbir hikâye sonucu (isabet, ECR…) okunmadan
yazıldı. İçindeki tek strateji başvuru `r3`'tür (ürünün sonuç kuralı ve dedektör seti).
Alternatif ajanlar düzeneği değiştirmez: kendi crate'lerini yazar, `Cargo.toml`'a bir path
bağımlılığı ve `src/registry.rs`'e bir satır ekler (aşağıda). Ölçüler yalnız motorun
çıktısından (hikâye olayları, karne) ve barlardan hesaplanır.

## Yapı

- Bağımsız crate: çalışma alanı üyesi değil (boş `[workspace]`), kendi `Cargo.lock`'u var;
  `crates/qp-*`'a path bağımlılığı. Kök çalışma alanının `fmt`/`clippy`/`test`'ini etkilemez.
- `src/data.rs` REST → seri (`prepare`), ikili seri biçimi, SPLIT aralıkları, keserek okuma.
- `src/sim.rs` bir sembol: dedektörler → olgular → `qp_capsule` motoru (tema `KindDir`,
  önem `Confirm`, sonuç = sınanan kural) → hikâye özeti + ECR/REV/SENS; birleşik karne.
- `src/m25.rs`, `src/m26.rs` ölçüler; `src/boot.rs` sembol kümeli bootstrap;
  `src/report.rs` tablolar; `src/registry.rs` kayıt; `src/tick.rs` fiyat adımı;
  `src/synth.rs` sentetik REST dosyaları (testler); `src/common.rs` LCG, yüzdelik, iş parçacığı.

## Komutlar

```sh
cd rounds/RP4/experiments/harness
export CARGO_TARGET_DIR=<ana ağaç>/target/wt/h      # K56
cargo build --release
H=$CARGO_TARGET_DIR/release/harness
# depo kökünden:
$H prepare --rest fixtures/local/live/rest --out fixtures/local/rp4 --universe fixtures/local/live/u100.txt
$H m25 --data fixtures/local/rp4/calib --rule r3 --period 5m            # --split calib varsayılan
$H m26 --data fixtures/local/rp4/calib --detectors r3 --period 5m [--rule r3]
$H report --out rounds/RP4/experiments                    # M25.md/csv, M26.md/csv
$H ticks --raw fixtures/local/live/raw --universe fixtures/local/live/u100.txt
$H list                                                   # kayıtlı kurallar, dedektör setleri
# yalnız son ölçüm (S), orkestratör:
$H m25 --data fixtures/local/rp4/full --rule <ad> --period 5m --split test
cargo test                                                # düzeneğin testleri
```

- `--period`: `5m`, `15m`, `1h`, `1d`.
- `m25` / `m26` sonucu `--out` altına yazar (varsayılan: yukarı doğru ilk `fixtures/local`
  → `fixtures/local/rp4/results`): `M25/<kural>.<periyot>.<split>.json`,
  `M26/<set>.<kural>.<periyot>.<split>.json`. `report --results <dir>` bunları okur.
  Sonuç dosyaları yalnız toplu sayıdır (sembol adı, fiyat yok). Süre yalnız stderr'e yazılır.
- `m25 --rule X` her zaman `X` ile başvuru `r3`'ü aynı veride koşar; farklar eşlidir.

## Veri

**`prepare --rest <dir> --out <dir> [--universe <dosya>]`** `<dir>` altındaki
`<SYM>.<5min|1hour|1day>.<başlangıç>.<bitiş>.adj.json` dosyalarını (JSON dizi; `time` epoch
ms bar açılışı, `open/high/low/close/volume`) okur. `.unadj` dosyalar okunmaz.

- Aynı sembolün pencere dosyaları ad sırasıyla birleşir; aynı `time` ikinci kez gelirse
  atılır (ilk dosya kalır), değeri farklıysa `duplicate_conflicts` sayılır.
- K14 hizalama (`qp_calendar`, XIST): gün içi `t` = `time`; günün seans kapanışında ya da
  sonrasında başlayan bar (18:00 kapanış seansı; yarım günde 12:30) aynı günün bir önceki
  barına birleşir (yüksek/düşük genişler, kapanış onunki, hacim toplanır). `bar_at(t).open
  ≠ t` olan, seans dışı ya da işlem günü olmayan satır atılır ve sayılır. Günlük: `t` =
  günün seans açılışı (10:00 İstanbul; REST `time` yerel gece yarısıdır).
- 15 dk, hizalanmış 5 dk barlarından `ExchangeCalendar::resample(15m)` ile.
- Çıktı: `<out>/full/<periyot>/<SYM>.bars` (tüm aralık) ve **fiziksel olarak ayrı**
  `<out>/calib/<periyot>/<SYM>.bars` (yalnız `t < kesim`; sınama günleri dosyada yok).
  Her kopyada `manifest.json`: periyot başına dosya, sembol, satır, tekrar, birleşen kapanış
  barı, atılan, bar, gün, ilk/son gün; evrende verisi olmayan sembol sayısı.

**Seri biçimi** (`src/data.rs`): `"QPRP4BR1"` (8 bayt) | `count` u64 LE | `count` ×
(`t` i64, `o`, `h`, `l`, `c`, `v` f64; hepsi LE, kayıt 48 bayt). Barlar kesin artan.

**Aralıklar** (SPLIT.md; İstanbul gece yarısı, UTC ms):

| Periyot | Kalibrasyon (sayılan doğum) | Kesim (`calib` okur `t <`) | Sınama |
|---|---|---|---|
| 5 dk, 15 dk, 1 sa | 2025-11-03 → | 2026-10-19 00:00 | 2026-10-19 → 2026-11-20 |
| 1 gün | 2018-01-02 → | 2025-01-02 00:00 | 2025-01-02 → 2026-11-20 |

- `--split calib` (varsayılan): okuyucu dosyayı **keserek** okur (kesimdeki ilk barda okumayı
  bırakır; sonraki kayıtlar çözülmez), `full/` verilse bile. Sayılan hikâye: doğumu
  kalibrasyon penceresinde ve kesimden önce kapanmış. Kesime taşan hikâye açık kalır,
  sayılmaz (SPLIT).
- `--split test`: her şeyi okur (ısınma için öncesi dahil), yalnız doğumu sınama
  aralığında olan ve veri bitmeden kapanan hikâyeleri sayar. Sınama barı olmayan klasörde
  (`calib/`) hata verir.
- Alternatif ajanlarına yalnız `fixtures/local/rp4/calib/` verilir.

## Ölçüler (M25, SPEC §3)

Her periyotta, sayılan hikâyeler üzerinde iki satır: **yönlü** (`up`/`down`) ve **yönsüz**.
Bütün oranlar "sembol başına toplamların oranı"dır; aralıklar sembol kümeli bootstrap
(sembolleri yerine koyarak, 1.000 tekrar, tohum 20261121, kendi LCG'miz; aynı ağırlıklar
iki kuralda da: fark eşli), %95 yakın sıra yüzdeliği.

| Ölçü | Uygulama |
|---|---|
| `n`, gün, sembol | sayılan hikâye; penceredeki barlı işlem günü ve sembol; ayrıca açık kalan |
| `p_real/p_inv/p_exp` | gerçekleşti / tutmadı / süresi doldu payı (sayılanlar içinde) |
| `ECR_own` | `invalidated` içinde: karar barından sonra `(karar, doğum+h]` (veri sonuna kırpılır) bir kapanış iddia yönünde kuralın **kendi** hedefine ulaştı mı. Hedef kayıttaki `levels` fonksiyonundan, `bars[..=doğum]` ile |
| `ECR_ref` | aynısı `r3` hedefiyle (`0,3·√h·ATR14`; yönsüzde `0,65·√h·ATR100`, `h` = türün `r3` ufku) |
| `REV` | `realized` ve yönlü: karar barından sonraki `⌈h/2⌉` bar içinde `±(kapanış − P0) < 0` |
| Ömür | `kapanış barı − doğum barı`, ortanca ve p90 (yakın sıra; aralık yok) |
| Brier | doğumdaki karne `n ≥ 20` olan hikâyeler: `mean((p̂ − y)²)`; taban: `p̂` yerine `q` |
| Kalibrasyon farkı | karne anahtarı başına `|r_k − L_k|`, `n_k` ağırlıklı (Yorumlar 3) |
| `SENS` | Yorumlar 5 |
| Gerçek − yalancı | yönleri çevrilmiş olgularla (yukarı ↔ aşağı) aynı kuralın ayrı koşusu; yönlü isabet farkı |

Karne: bir hikâyenin `ledger_at_birth`'i bütün sembollerin doğumdan **önce** kapanan
hikâyelerinden sayılır (motorun `ledger_stat`'ı); düzenek bunu birleşik karneden yeniden
hesaplar (Yorumlar 1).

## M26 (SPEC §5)

Her dedektör (doğuran olgunun dedektörü) ve havuz `(hepsi)` için: doğum; **yük** = doğum /
sembol·gün (penceredeki barlı sembol·gün); isabet = gerçekleşti / kapanan (aralıklı) ve 0,55'e
göre konumu (`above`: alt uç > 0,55; `below`: üst uç < 0,55; yoksa `contains`); açılış (günün
ilk iki barı) ve kapanış (günün son barı, kapanış seansı birleşik) doğum payı, isabeti ve
geri kalanla farkı (`d_open`, `d_close`, aralıklı); `ft` = isabet(geri kalan) − isabet(açılış
∪ kapanış); "gün içinde anlamsız" = üst uç < 0,5 ya da (nokta < 0,55 ve aralık 0,5'i
kapsıyor). Başvuru seti `r3` ile eşli farklar (dedektör adıyla eşlenir); yük oranı (nokta).
Günlükle tutarlılık: aynı set/kural/split için `1d` sonucu da varsa `report` gün içi
isabet − günlük isabeti yazar. Sonuç kuralı `--rule` (varsayılan `r3`). `1d`'de açılış/kapanış
ölçüleri yok.

## `report`

`M25.md/csv`: periyot × split grupları; yönlü ve yönsüz tablolar (alternatif değerleri
aralıklarıyla), fark tablosu, **§4 puanlaması** (kalibrasyon, yönlü satır) mekanik olarak:
(1) 5 dk `ECR_own` ≤ ½ başvuru ve farkın üst ucu < 0; (2) `ECR_ref` farkının alt ucu ≤ 0;
(3) 5 dk ve 1 sa Brier farkının alt ucu ≤ 0 ve 5 dk kalibrasyon farkı farkı ≤ +0,03 (nokta);
(4) 5 dk `p_exp` ≤ 0,40; (5) 5 dk `SENS` farkının alt ucu ≤ 0. Eksik periyot "eksik" yazılır.
Sıralama (a–d) ve "ayırt edilemedi" kararı elle verilir. `M26.md/csv`: set × kural × periyot
× split başına dedektör tablosu ve §5 puanlaması: (1) her dedektörde isabet farkının üst ucu
≥ 0, (2) havuz yük oranı ≤ 1,5, (3) havuz `ft` farkının üst ucu < 0. CSV uzun biçim:
`…,metric,alt_pt,alt_lo,alt_hi,ref_pt,ref_lo,ref_hi,diff_pt,diff_lo,diff_hi`.

## Fiyat adımı (SPEC §1)

`tick(p)`: ≤20: 0,01; ≤50: 0,02; ≤100: 0,05; ≤250: 0,10; ≤500: 0,25; ≤1000: 0,50;
≤2500: 1,00; üstü 2,50; doğum kapanışının fiyatıyla. `ticks --raw` tabloyu yerel ham
kayıtlardaki canlı `priceStep` ile karşılaştırır (yalnız toplu sayı). 2026-11-19 kayıtları
(12 dosya), u100: `priceStep` taşıyan 531 ileti, 505 uyumlu; 100 sembolün 98'i her
gözlemde uyumlu; 26 uyumsuzluğun 22'si fiyat **tam sınırdayken** (ör. 50,00: tablo `≤50` →
0,02, canlı 0,05): borsa sınırda üst bandı kullanıyor (`[alt, üst)`). Şartname `≤` dediği
için tablo değiştirilmedi; etkisi yalnız doğum kapanışı tam sınırda olan hikâyelerin `SENS`
adımı (rapora öneri). Kalan 4 uyumsuzluk iletideki fiyatın adımın verildiği andaki
fiyattan farklı olmasından olabilir (adım anlık görüntüde fiyatsız geliyor; o sembolün son
fiyatıyla eşlenir).

## Alternatif ekleme

**M25 (çıkış kuralı).** Alternatif ayrı bir crate (`rounds/RP4/experiments/M25/<ad>/`) ya
da `qp-capsule` içinde adlandırılmış bir kuraldır.

1. Crate (`[workspace]` boş, `version.workspace` gibi miras alan yok):
   ```toml
   [package]
   name = "m25-a1"
   version = "0.1.0"
   edition = "2021"
   publish = false

   [workspace]

   [dependencies]
   qp-capsule = { path = "../../../../../crates/qp-capsule" }
   qp-schema = { path = "../../../../../crates/qp-schema" }
   ```
2. `src/lib.rs` iki fonksiyon verir:
   - `pub fn outcome() -> Box<dyn qp_capsule::OutcomeRule>` (her çağrıda taze örnek);
   - `pub fn levels(s: &qp_capsule::OpenStory, bars: &[qp_schema::Bar]) -> Option<(f64, f64, Option<f64>)>`:
     doğumda sabit `(p0, hedef mesafesi, durdurma mesafesi)` (fiyat birimi, > 0). Yönlüde
     `±(kapanış − p0) ≥ hedef` kendi hedefidir; yönsüzde `|kapanış − p0| ≥ hedef`. `bars`
     tam olarak `bars[..=doğum]`'dur. `ECR_own` ve süresi dolanların `SENS` "en yakın barı"
     bunu kullanır; kural hedefini sonradan kaydırıyorsa doğumdaki değer verilir.
3. `harness/Cargo.toml` `[dependencies]` altına bir satır: `m25-a1 = { path = "../M25/a1" }`.
4. `harness/src/registry.rs` `rules()` listesine bir satır:
   `RuleEntry { name: "a1", make: m25_a1::outcome, levels: |s, b| m25_a1::levels(s, b).map(Levels::from), constants: <sabit sayısı> },`
   (`qp-capsule` içindeki bir kural için `make: || Box::new(qp_capsule::…::new(…))`.)
5. `cargo build --release && $H m25 --data fixtures/local/rp4/calib --rule a1 --period 5m`
   (ve `15m`, `1h`). `Cargo.lock` güncellenir, commit'e girer.

**M26 (dedektör eşikleri).** `pub fn detectors() -> Vec<Box<dyn qp_facts::Detector>>`
(her çağrıda taze set; ürün setiyle aynı dedektör kimlikleri, farklı eşikler); kayıtta
`DetectorEntry { name: "a1", make: m26_a1::detectors },`; koşu
`$H m26 --data … --detectors a1 --period 5m`. Dedektör kimliği ve sürümü karne anahtarına
girer; eşik değişince sürüm artırılırsa satırlar yine dedektör kimliğiyle eşlenir.

Kural: alternatif yalnız trait'i uygular; saat, ağ, rastgelelik, dosya okuma yok;
`judge` yalnız `(hikâye, bars[..=now])`'un saf fonksiyonudur (`SENS` aynı hikâyeyi
oynatılmış barlarla yeniden koşar; kendi kendini doğrulama sayacı `sens_replay_mismatch`
saf olmayan kuralı yakalar).

## Testler (`cargo test`, 15)

Sentetik veri (`src/synth.rs`: XIST takviminde 5 dk barlar + ayrı 18:00 kapanış barı, ondan
1 sa; günlük gece yarısı zamanlı; pencere dosyaları üst üste biner):

- **İleriye bakmama:** `full/`'de sınama barlarının hepsi değiştirilince kalibrasyon kipinin
  M25 çıktısı bayt eşit; `full/` ile `calib/` kopyasında bayt eşit; sınama kipinde fark
  görülür (veri gerçekten değişti). Okuyucu kesimden sonraki bozuk kaydı çözmez. M26 da
  `full/` = `calib/`.
- **Belirleyicilik:** iki koşu bayt eşit (M25, M26, `prepare` çıktıları); bootstrap ağırlıkları.
- **Ürün yolu:** sembol başına koşu + birleşik karne = bütün sembollerle tek motor
  (`qp_capsule::pipeline::simulate_engine`): her kapanan hikâyenin aşaması, kapanış barı ve
  `ledger_at_birth`'i aynı. `r3` düzeyleri motorun bariyeriyle aynı (gerçekleşenlerde hedef
  karar barında ilk kez aşılır, tutmayanlarda durdurma).
- **Öz tutarlılık:** `r3`'ün `ECR_own` = `ECR_ref`; kendisiyle farkı tam 0; `SENS`'in
  oynatılmamış yeniden koşusu motoru her hikâyede yeniden üretir; pay toplamı 1.
- **Alternatif yolu:** test içi bir kural (geniş bariyer) başvuruyla eşli koşar, başvuru
  sütunu tek başına `r3` koşusuyla aynı, rapor satırı yazılır.
- Hizalama (K14): her bar takvim barının açılışında; 18:00 barı 17:55'e birleşir, günde 96
  bar; 15 dk = 3 × 5 dk; günlük 10:00. Kalibrasyon kopyasında kesim sonrası bar yok. Sınama
  kipinde 24 gün. Gün konumu (açılış/kapanış). Fiyat adımı tablosu ve sınırları.

## Süre (Apple silicon, 16 çekirdek, `--release`)

Sentetik 100 sembol × 265 gün 5 dk (`synth --symbols 100`; gerçekten çok daha fazla hikâye
doğurur): `m25 --rule r3 --period 5m` kalibrasyon 2,30 M bar / 514 bin sayılan hikâye
**11 s**, ~2,2 GB bellek; sınama kipi 7 s. Alternatifle (iki kural) yaklaşık iki katı.
Paralellik sembol başınadır; sonuçlar sembol sırasıyla toplanır, çıktı bayt eşit kalır.

## Yorumlar (şartnamenin açık bıraktığı noktalar; en dar yorum)

1. **Sembol başına koşu.** Motor her sembolde ayrı koşar (paralel). Tema kuralı yalnız aynı
   enstrüman + serinin hikâyelerini, sonuç kuralı yalnız hikâyeyi ve barlarını görür; tek
   sembollerarası nicelik karnedir (`kind/direction/detector@v` anahtarı ortak). Doğumdaki
   karne birleşik kapanışlardan motorun tanımıyla (`kapanış < doğum zamanı`) yeniden
   hesaplanır; tek motorla eşitliği test edilir. Önem/kademe (sıralayıcı) ölçülmez.
2. **Sayılan hikâye** = doğumu pencerede (`calib`: 2025-11-03 / günlük 2018-01-02 → kesim;
   `test`: kesim → 2026-11-21) ve verinin sonundan önce kapanmış. Açık kalanlar ayrıca
   sayılır. Penceredeki "gün" = barı olan işlem günü.
3. **Kalibrasyon farkı.** Sınama kipinde: karne = kesimdeki karne (kesimden önce kapananlar),
   sayılan aralık = sınama. Kalibrasyon kipinde aynı tanım kendi kendine eşit olurdu (karne
   ile sayılan hikâyeler aynı küme), bu yüzden kalibrasyon penceresi ortadaki işlem gününde
   ikiye bölünür: karne o günün açılışındaki karne, sayılan aralık o günden sonra doğanlar.
   Yalnız o andaki karnesi `n ≥ 20` olan anahtarlar girer. Bootstrap'ta `L_k` sabit, `r_k`
   yeniden örneklenir.
4. **Brier tabanı `q`** = aynı satırın (yönlü/yönsüz) kalibrasyon aralığındaki gerçekleşme
   oranı (kalibrasyonda doğup kesimden önce kapanan hikâyeler; kalibrasyon kipinde örneklem içi).
   Brier ve taban aynı hikâyeler üzerinde (karnesi `n ≥ 20`). Süresi dolan `y = 0`.
5. **`SENS`.** Sayılan her kapanmış hikâyede: oynatılan bar = karar barı (gerçekleşti/tutmadı;
   yönsüzde "genişlemedi" kararı ufuk barıdır); süresi dolanlarda ufkun son barı **ve**
   pencerede kapanışı kuralın kendi hedefine ya da durdurmasına (yönsüzde hedefe) en yakın
   bar (eşitlikte erken olan). Her bar için kapanış `±1`, `±2` adım (`tick(P0)`), yüksek/düşük
   kapanışı kapsayacak kadar genişler. Kural doğumdan itibaren motorun sırasıyla (önce
   `judge`, sonra ufuk) yalnız o hikâye için yeniden koşar; `judge`'a hikâye, motorun o barda
   gösterdiği haliyle verilir (kapanış dışı olayların anlık görüntüsü, o bara kadar katılan
   olgular; orijinal kapanıştan sonrası için son açık hali — sonradan katılacak olgular
   bilinemez). "Aşaması değişen" = oynatılmamış yeniden koşunun son aşamasından farklı
   (gerçekleşti/tutmadı/süresi doldu/veri bitti). Yeniden koşu sembol başına taze bir kural
   örneğiyle yapılır.
6. **`ECR`** yalnız `invalidated` hikâyelerde; hedef düzeyi kuralın `levels`'ından (doğumda
   sabit); düzeyi olmayan hikâye (`None`, ör. ısınma) payda dışı, `ecr_skipped` sayılır.
   Pencere veri sonunda (kalibrasyon kipinde kesimde) kırpılır. Yönsüzde hedef `|kapanış − P0|`.
7. **`REV`** yalnız yönlü `realized`; "geçti" kesin eşitsizlik (`< P0` yukarı iddiada).
   `P0` her zaman doğum kapanışı (kuralın `p0`'ı değil).
8. **Yalancı eş**: bütün olguların yönü çevrilir (R3/M5), aynı kural, aynı sayma; yalnız
   yönlü satır. "Günlükte R3 başvurusuyla karşılaştırma" `report`'ta değil, rapor yazımında
   (R3/M5 RESULTS sayıları) yapılır.
9. **M26**: isabet = gerçekleşti / kapanan (süresi dolan ıska, motorun karnesi gibi). Açılış
   ve kapanış barı sembolün kendi verisinde günün ilk iki / son barıdır (eksik bar varsa
   ona göre). "Yanlış tetikleme azalması" havuz `ft` farkıyla ölçülür (dedektör başına
   satırlar da var). Yük oranı noktadır (şartname "en çok 1,5 katı").
10. **Gösterim**: pencerenin bitiş günü kesimden bir önceki takvim günü olarak yazılır
    (kalibrasyonda 2026-10-18 Pazar; son işlem günü 2026-10-16).
11. **REST satırları**: `high`/`low`, `open`/`close`'u kapsamıyorsa genişletilir; fiyatı ≤ 0
    ya da sonlu olmayan satır atılır (sayılır). Akış verisi yok (`flow.institutional` susar).

## Sınırlar

- Ölçüler motorun ürettiği hikâyeler üzerindendir; sonuç kuralı hikâye kapanışını
  değiştirdiğinden (tema kuralı açık hikâyeye katar) iki kuralın hikâye kümeleri birebir
  aynı değildir; eşleme sembol düzeyindedir (bootstrap), hikâye düzeyinde değil.
- `SENS` oynatmadan sonra sonradan katılacak olguları bilemez (Yorumlar 5).
- Yönsüz hikâyelerin hükmü ATR(100) ister: sembolün ilk 100 barında doğan yönsüz hikâye
  hüküm almadan süresi dolar (motor davranışı; ısınma pencerenin başında kalır).
