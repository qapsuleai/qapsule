# R3/S-D — Performans (ölçüt 17)

Dal `r3/s-perf` (`r3/int` `d7e87db` üstünde). Ürün kodu değişmedi. Yazılanlar: yeni ölçüm
örneği `crates/qp-session/examples/capsule_tick.rs`, bu dosya, ham çıktılar
`rounds/R3/experiments/S/raw/perf/`.

**Özet.** Motorun kendisi ucuz: 30 enstrümanda bar kapanışı `Engine::on_bar` 0,22 ms, açık
bar adayları 0,20 ms (S-A'nın yüklü makinedeki 1,7 ms'sinin yerine). Oturum düzeyinde ise
kapsül motoru açıkken maliyeti **motor değil, oturumun kapsül başına yaptığı işler**
belirliyor ve bu iş dokümandaki kapsül sayısıyla (geçmiş uzadıkça) büyüyor: 1.000 barlık
geçmişte (6.369 kapsül) 30 enstrümanın bar kapanışı **3,65 s** (akışla 8,0 s), yeni bir
bağlantının `hello`'su **161 s** (akışla 279 s). Canlı düzeltme tiki (açık bar) olay başına
4,4–17 µs; RP2'nin tek seri bağlı N=5 tabanı 0,78 µs. Profilde `Engine::on_bar` örneklerin
%0,1'i; zamanın %97'si `run_capsules` içindeki başlık yenileme ve okuyucu izdüşümünde.
Bunlar performans hatası olarak §6'da kanıtla önerildi (düzeltilmedi; kapsam dışı).

## 1. Yöntem

### 1.1 Oturum düzeyi: `capsule_tick`

```text
cargo run --release -p qp-session --example capsule_tick -- 30 <geçmiş> <piyasa tiki> <koşu> none,caps,flow bound
cargo run --release -p qp-session --example capsule_tick -- 30 250 600 3 none,caps inline
PER_BAR=100000000 … capsule_tick -- profile caps 30000 250 bound   # yalnız düzeltme tiki profili
```

- Doküman: 30 bar serisi (`synthetic_market(30, …, 1)`, günlük), her biri **bağlı**
  (`bind`, besleme `bist`, `window.last` 10000; geçmiş `init.data` olarak) ya da **belge içi**
  (barlar dokümanda, tikler besleme bağlantısından komut).
- Düzenler: `none` motor yok (RP2 canlı yolu, 30 seri; kıyas noktası); `caps`
  `init.capsules.series` = 30 seri (yedi varsayılan dedektör, altı tür; `EngineConfig::r3()`),
  akış yok; `flow` = `caps` + seri başına sentetik lisanslı akış (`init.capsules.flow`,
  `flow.institutional` dedektörü veri alır, mühür çalışır).
- Bağlantılar: `a` (arkin, sahip, her şeye yetkili), `b` (bora, yalnız barlar), `c` (can,
  yetkisiz); her biri `changes` ve `capsules {all}` konularına abone. Belge içi yolda ayrıca
  `f` (besleme, editör).
- Besleme (RP2 deseni, enstrüman başına): saniyede bir yeni bar + 59 düzeltme (alan düzeyinde
  `patch`, yalnız değişen alanlar); düzeltme `k/60` kapanışı açılıştan son kapanışa taşır,
  60'ıncı = son bar (S-A `engine_cost` ile aynı tik üreteci). **Piyasa tiki** = her enstrümana
  bir olay (30 olay). Yeni bar öncekini kesinleştirir → otorite o barı dedektörlerden ve
  motordan geçirir, seri başına bir motor girdisi yazar, abonelere bildirir. Açık barda
  otorite kapsül işi yapmaz (adaylar yansının işi, §20; döküm 42) — yani "her tikte aday"
  oturumda ölçülecek bir şey değil, yansının maliyeti (§2).
- Zamanlama: `Session::new` (geçmiş üzerindeki motor yetişmesi dahil) ve bağlantıların
  `hello`/`subscribe`'ı döngü dışında, ayrı raporlandı. Koşu başına taze oturum; piyasa tiki
  başına bir zamanlayıcı; kapanış tikleri (yeni bar) ve düzeltme tikleri ayrı; koşu başına
  ortalama, koşular arası **medyan [min–maks]**. Belirlenimcilik: her koşunun son doküman
  özeti aynı (`same_across_runs true`, bütün satırlarda).
- Geçmiş uzunluğu (100, 250, 500, 1.000 bar) değişken olarak tarandı, çünkü maliyet kapsül
  sayısıyla büyüyor (§3). 1.000 barda yalnız 1 koşu yapılabildi (`flow` düzeni tek koşuda
  ~8 dk: bağlanma 279 s + 10 kapanış × 8 s); 500 barda 120 piyasa tiki (2 kapanış).

### 1.2 RP2 tabanı (aynı gün, aynı makine)

`rounds/RP2/experiments/tick-bench` `live` ikilisi, RP2 yöntemiyle: 10.000 bar, 600 tik
(saniyede 1 yeni bar + 59 düzeltme), µs/tik, uzun döngü medyanı (çekirdek 15, oturum 7
tekrar), iki koşunun ortalaması; `t6d` ve `all` düzenleri, yalnız yerel (WASM bu turda
koşulmadı). Ayrı hedef dizininde derlendi; derleme `tick-bench/Cargo.lock`'u güncellediği için
dosya geri alındı (değişmedi), hedef dizini silindi.

### 1.3 Makine ve yük

Apple M4 Max, 16 çekirdek, Darwin 27.0.0, rustc 1.92.0, Node 24.3.0; `--release`,
`CARGO_INCREMENTAL=0`. Görev başında yük ortalaması 17–24'tü (başka ajanlar); ölçümler yük
5–8'e indikten sonra koşuldu. Ölçüm anlarındaki 1 dk yük ortalaması (ham dosyaların
`# start/# end` satırları): RP2 tabanı 5,4–6,1; `engine_cost` 6,1–6,6; `capsule_tick`
100/250/500 bar 5,7–8,1; 1.000 bar başta 6,1, sonda 14,3 (son düzen `flow` yükselen yükte);
profiller 9–11. Ölçümler tek iş parçacıklı; 16 çekirdekte bu yük düzeyinde koşular arası
yayılım ≤ %3 (aşağıdaki min–maks), RP2 tabanı RP2 raporundan ≤ %4 sapıyor (§4).

## 2. Yalnız motor (`engine_cost`, S-A §6'nın yerine)

`cargo run --release -p qp-capsule --example engine_cost -- 20 7` (30 enstrüman × 7 dedektör,
500 bar, bar başına 20 tik, bar 60'tan itibaren). Üç bağımsız koşu (her biri 7 iç koşunun
medyanı); ham `raw/perf/engine-cost-run{1,2,3}.txt`.

| Nicelik | Piyasa (30 enstrüman), 3 koşu | Enstrüman başına | S-A (yüklü makine) |
|---|---:|---:|---:|
| Tik: dedektörler + olgu + aday | **215,3 µs** [215,1–216,6] | 7,2 µs | 1,77 / 2,51 ms |
| — 7 dedektör `update_last` + `to_fact` | 11,3 µs [11,2–11,5] | 0,38 µs | 64 / 85 µs |
| — `Engine::candidates` (açık bar) | 203,7 µs [203,4–205,5] | 6,8 µs | 1,70 / 2,41 ms |
| Tik, bar 60–199 | 190,1 µs [189,8–191,2] | 6,3 µs | 1,34 / 1,71 ms |
| Tik, bar 360–499 | 232,4 µs [232,1–236,0] | 7,7 µs | 2,13 / 3,34 ms |
| Bar kapanışı `Engine::on_bar` | **218,0 µs** [215,5–220,8] | 7,3 µs | 1,74 / 3,05 ms |
| Tek `ledger_stat` (son karne, 4.240 satır) | 13 ns | — | 7,9 µs |

S-A'dan ×8 hızlı. İki neden ayrılamıyor: S-A'nın yük ortalaması 135–320 idi (duvar saati
şişkin), ve S-C'nin karne dizini `ledger_stat`'ı 5.784 → 14 ns'ye indirdi (S-C §5; burada
13 ns). S-A'nın profili zamanın çoğunu `ledger_stat` taramasında gösterdiği için farkın
büyük kısmı büyük olasılıkla dizinden. Aday hesabı hâlâ tikin %95'i; karne büyüdükçe artış
artık küçük (bar 60–199 → 360–499: ×1,22; S-A'da ×1,6–2).

## 3. Oturum düzeyi: 30 enstrüman, bağlı yol

µs, medyan [min–maks]; "olay başına" = piyasa tiki / 30. Kapsül sayısı = koşu sonunda
dokümandaki kapsüller (canlı + kapanmış).

| Geçmiş | Düzen | Kapsül (son) | `Session::new` | Bağlanma (4 hello + 6 abone) | Düzeltme tiki (olay başına) | Kapanış tiki (enstrüman başına) | Bütün tikler, piyasa tiki başına (olay başına) | Koşu |
|---:|---|---:|---:|---:|---:|---:|---:|---:|
| 100 | none | 0 | 0,39 ms | 1,7 ms | 58,9 [57,4–63,3] (1,97) | 63 µs (2,1 µs) | 59,0 (1,97) | 5 |
| 100 | caps | 505 | 44 ms | **1,07 s** | 131,4 [130,4–132,0] (4,38) | **134,5 ms** [132,6–136,0] (4,5 ms) | 2.370 (79) | 5 |
| 100 | flow | 777 | 79 ms | 1,78 s | 140,6 [138,4–142,6] (4,69) | 301,3 ms [297,8–309,5] (10,0 ms) | 5.159 (172) | 5 |
| 250 | none | 0 | 0,48 ms | 1,7 ms | 60,7 [60,2–60,9] (2,02) | 65 µs (2,2 µs) | 60,8 (2,03) | 3 |
| 250 | caps | 1.483 | 181 ms | 7,10 s | 158,8 [157,5–159,8] (5,29) | 507,7 ms [501,2–509,3] (16,9 ms) | 8.619 (287) | 3 |
| 250 | flow | 2.093 | 296 ms | 12,3 s | 181,5 [179,7–183,4] (6,05) | 1.056 ms [1.038–1.072] (35,2 ms) | 17.784 (593) | 3 |
| 500 | none | 0 | 0,45 ms | 1,7 ms | 60,9 [60,6–61,6] (2,03) | 86 µs (2,9 µs) | 61,3 (2,04) | 3 |
| 500 | caps | 3.075 | 536 ms | 32,7 s | 215,4 [213,8–218,6] (7,18) | 1.289 ms [1.278–1.292] (43,0 ms) | 21.705 (724) | 3 |
| 500 | flow | 4.284 | 912 ms | 56,2 s | 254,8 [254,6–257,9] (8,49) | 2.645 ms [2.630–2.649] (88,2 ms) | 44.342 (1.478) | 3 |
| 1.000 | none | 0 | 0,93 ms | 1,6 ms | 61,1 (2,04) | 71 µs (2,4 µs) | 61,3 (2,04) | 1 |
| 1.000 | caps | 6.369 | 1,75 s | **161,5 s** | 338,4 (11,3) | **3.652 ms** (121,7 ms) | 61.202 (2.040) | 1 |
| 1.000 | flow | 8.761 | 3,08 s | **279,2 s** | 513,2 (17,1; p99 14.167) | **8.037 ms** (267,9 ms) | 134.449 (4.482) | 1 |

Belge içi yol, 250 bar, 3 koşu (`raw/perf/capsule-tick-inline-h250.txt`):

| Düzen | Kapsül | Düzeltme tiki (olay başına) | Kapanış tiki | Bütün tikler (olay başına) |
|---|---:|---:|---:|---:|
| none | 0 | 92,9 µs [90,6–96,0] (3,10 µs) | 0,10 ms | 93,1 µs (3,10) |
| caps | 1.483 | **105.197 µs** [104.362–105.255] (**3,51 ms**) | 533 ms | 112.340 µs (3.745) |

Belge içi yolda her düzeltme bir günlük girdisi olduğundan (komut), kapsül motoru açıkken
**her tik** kapsül sayısıyla doğrusal iş yapıyor: 1.483 kapsülde olay başına 3,5 ms
(motorsuz 3,1 µs; ×1.130).

Bildirimler (kontrol): bağlı `caps`/`flow` düzeninde kapanış başına seri başına bir `change`
ve hikâye geçişleri kadar `capsule` olayı; `a`, `b`, `c`'ye aynı sayıda satır (`caps`) ya da
`a`'ya biraz fazla (`flow`: lisanslı hikâyeler yetkisize gitmiyor, yerine mühür yorumu). Bağlı
yolda düzeltme tiki hiçbir bağlantıya satır üretmiyor (RP2'deki gibi; günlük girdisi yok).

## 4. RP2 tabanıyla kıyas

**Aynı gün yeniden koşulan RP2 tabanı** (yerel, µs/tik, iki koşunun ortalaması; ham
`raw/perf/rp2-live-native-run{1,2}.txt`; parantez içinde RP2 raporundaki değer):

| Yol, N | Çekirdek + çözme | Oturum `t6d` | Oturum `all` |
|---|---:|---:|---:|
| belge içi, 5 | 0,66 (0,65) | 2,04 (2,04) | 2,23 (2,16) |
| belge içi, 50 | 1,47 (1,46) | 2,65 (2,71) | 3,56 (3,58) |
| belge içi, 500 | 8,02 (8,06) | 10,58 (10,18) | 21,64 (20,71) |
| bağlı, 5 | 0,41 (0,40) | 0,78 (0,76) | 0,93 (0,92) |
| bağlı, 50 | 1,24 (1,22) | 1,31 (1,31) | 2,13 (2,06) |
| bağlı, 500 | 7,71 (7,94) | 8,78 (8,71) | 19,47 (19,27) |

Taban RP2'den ≤ %4,5 sapıyor (en büyük: belge içi N=500 `all` +%4,5); makine ve yöntem
karşılaştırılabilir.

**Kıyas (birimler):** RP2'nin tiki tek serinin bir olayı; burada olay başına değer aynı
birimde (bir enstrümanın bir olayı), piyasa tiki 30 olay.

| | Olay başına µs | RP2 bağlı N=5 `t6d` (0,78) karşısında |
|---|---:|---:|
| 30 seri, motor yok (`none`), düzeltme | 1,97–2,04 | ×2,5–2,6 |
| 30 seri, `caps`, düzeltme, 100 → 1.000 bar | 4,4 → 11,3 | ×5,6 → ×14 |
| 30 seri, `flow`, düzeltme, 100 → 1.000 bar | 4,7 → 17,1 | ×6,0 → ×22 |
| 30 seri, `caps`, bütün tikler (kapanış dahil), 100 → 1.000 bar | 79 → 2.040 | ×101 → ×2.600 |
| belge içi 30 seri, `caps`, 250 bar, düzeltme | 3.507 | (RP2 belge içi N=5: 2,04) ×1.700 |

Okuma:

- Motorsuz 30 serilik doküman olay başına ~2 µs (RP2'nin tek serisi 0,78): fark,
  `settle_live` içindeki seri sayısıyla doğrusal yürüyüşler; geçmiş uzunluğundan bağımsız
  (100–1.000 barda 1,97–2,04).
- Motor açıkken açık bar tiki (otorite kapsül işi yapmıyor) yine de kapsül sayısıyla büyüyor:
  profil (`raw/perf/profile-revision-bound.txt`, 250 bar) zamanın %38'ini `run_capsules`'ın
  "kesinleşen bar var mı" denetiminde gösteriyor (her olayda 30 izlenen seri × iki
  `find_series` dizgi karşılaştırması), kalanın büyük bölümü `settle_live`'ın satır içine
  alınmış döngülerinde (canlı kapsül dizini; ayrıştırılamadı).
- Bar kapanışında oturum motorun binlerce katı: 500 barda `Engine::on_bar` 30 enstrüman için
  0,22 ms (§2), oturumun kapanış tiki 1.289 ms (`caps`, ×5.900) / 2.645 ms (`flow`, ×12.100;
  `engine_cost` akışlı ve mühürsüz, kıyas kaba).

## 5. WASM boyutu

`cargo build -p qp-wasm --target wasm32-unknown-unknown --profile wasm` (verify adım 8 ile
aynı) ve `… --features replica`; `gzip -9`.

| Yapı | ham B | gzip -9 B | RP2 | Fark (gzip) | R2 |
|---|---:|---:|---:|---:|---:|
| varsayılan | 1.943.806 | **616.021** | 598.324 | +17.697 (+%3,0) | 595.475 (+20.546, +%3,5) |
| `--features replica` | 5.326.906 | **1.310.330** | 1.088.011 (ham 4.387.493) | +222.319 (+%20,4) | — |

- Varsayılan paketteki artış S-B'nin (alfabe + yeni işaret/derinlik kodu: ham +54.472,
  gzip +17.567) ölçtüğüyle aynı; S-C'nin oturum değişiklikleri varsayılan pakete girmiyor
  (S-C ile aynı ham boyut; gzip 616.022 → 616.021, sıkıştırma gürültüsü).
- Yansılı paket +222 KB gzip: `qp-session` artık `qp-capsule` (motor, gerçekleştirici, mühür)
  ve `qp-facts`'i getiriyor (yansının `candidates`'ı ve okuyucu cümleleri için). Ayrıştırma
  (twiggy vb.) yapılmadı.

## 6. Yorum ve öneriler

### 6.1 Maliyeti ne belirliyor

Profil (`sample`, 250 bar, `caps`; özetler `raw/perf/profile-*.txt`, fonksiyon başına
kapsayıcı örnek):

- **Bar kapanışı** (`profile-close.txt`): `run_capsules` %97,5. İçinde
  `refresh_headlines` → `story_headlines` → `reader_sentence` **%50** (her izlenen seri
  ilerlediğinde, yani kapanış başına 30 kez, dokümandaki **bütün** kapsüllerin TR cümlesi
  baştan yazılıyor: gerçekleştirici `sentence_pair` %29, kalanı olguların doğrusal aranması,
  `Fact` kopyaları ve `now_bar`'ın serinin bütün barlarını kopyalaması); `proj_for` **%45**
  (bütün kapsül ve olgular üzerinden izdüşüm kuruyor; çağıranlar `entry_events` →
  `reader_proj` %23 ve `capsule_topic_lines` %23: her motor girdisi × her bağlantı için
  baştan). `Engine::on_bar` **%0,1**, `capsule_changes` %0,1.
  Karmaşıklık: kapanış başına ~ seri × kapsül × (bağlantı + cümle); kapsüller (kapanmış olanlar
  dahil) dokümanda kaldığı için geçmişle büyüyor: kapsül başına kapanış maliyeti 100 → 500
  barda 266 → 419 µs.
- **Bağlanma** (`profile-connect.txt`): `hello` → `summarize_doc` %97; `finding_hint` %90,
  `finding_rows` %84, `finding_about` %51. Özetteki son 50 bulgunun her biri için seçici
  ipucu adayları bütün bulguları tarıyor, her bulgu için olgular doğrusal aranıyor
  (`finding_facts`: `doc.facts.iter().find`) ve dizgi kuruluyor: ~50 × aday × bulgu × olgu.
  1.000 barda 161 s (`caps`) / 279 s (`flow`): bu sürede otorite başka hiçbir şeye cevap
  veremez (tek iş parçacığı).
- **Belge içi düzeltme** (`profile-revision-inline.txt`): `proj_for` %98,8 — komutun kuru
  koşu denetimi (`session.rs` "dry run" öncesi `self.proj_for(&principal).is_some()`, %24) ve
  `entry_events`'te abone başına `reader_proj` (%75). Hiçbir izdüşüm gerekmediğinde de (yetkili
  okuyucu, akış yok) her seferinde bütün kapsüller yürünüyor.

### 6.2 Telefon sınıfı bütçe

Ölçülemedi. Burada ölçülen yerel M4 Max üzerinde otorite (sunucu tarafı) ve yalnız motor;
telefon CPU'sunda ve WASM'da yansının "her tikte aday" maliyeti ölçülmedi. Elde olan dayanak:
30 enstrüman için açık bar adayları yerelde piyasa tiki başına 0,20 ms (§2); RP2'de WASM
opt-s yerelin ~1,4–2 katıydı (farklı iş yükünde); telefon çarpanı bilinmiyor. Bu üçünü
çarpıp bir bütçe sonucu çıkarmak dayanaksız olur; tarayıcıda/telefonda `Replica::candidates`
ölçümü ayrı yapılmalı.

Otorite için ise sonuç ölçümle açık: sunucuda (M4 Max) 1.000 barlık geçmişle günlük kapanış
başına 3,7–8,0 s ve bağlantı başına dakikalar; bu hâliyle ölçeklenmiyor. Kapanış sıklığı
yüksek aralıklarda (ör. 1 dk) 30 enstrüman sürdürülemez.

### 6.3 Öneriler (performans hataları; ürün kodu değişmedi)

Hepsi davranışı değiştirmeden yapılabilir; doğrulama için artımlı = baştan (bayt) ve
dökümler 38–43 yeter.

1. **Başlıkları yenileme kapsamı** (`capsule.rs` `run_capsules` → `refresh_headlines`):
   seri başına değil, `run_capsules` sonunda bir kez; daha iyisi yalnız bu bakışta değişen
   hikâyeler ve "şimdi"ye bağlı (K26) cümlesi olan canlı hikâyeler için. Kanıt: kapanış
   profilinin %50'si; kapanmış hikâyelerin cümlesi bir daha değişmiyor mu, sözlük/K26
   açısından doğrulanmalı.
2. **İzdüşüm önbelleği** (`proj_for`, `reader_proj`): okuyucu başına, doküman yapı kuşağı
   (`struct_gen`) ve yetki değişimiyle geçersizlenen önbellek; ayrıca "her şeye yetkili ve
   başkasının kitle tercihi yok" için O(1) kısa yol. Kanıt: kapanış profilinde %45
   (`reader_proj` %23 + `capsule_topic_lines` %23), belge içi düzeltmede %98,8.
3. **Doğrusal aramalar**: `reader_sentence` (`all_facts().iter().find`), `capsule_changes`
   (`self.doc.facts.iter().any/find`), `summarize_doc` (`doc.facts.iter().find`,
   `doc.capsules.iter().find`), `finding_facts` — kimlik → indeks haritası; `now_bar` ve
   `capsule_changes`'teki `bars.to_vec()` (kapsül başına bütün bar geçmişinin kopyası) yerine
   ödünç.
4. **`hello` özeti** (`summary.rs` `select_hints` → `finding_hint`): adayların eşleştiği
   kümeyi bulgu başına baştan hesaplamak yerine tek geçişte tür/durum/olgu dizinleri; hint
   hesabını özetteki 50 öğeyle sınırlı ama tarama başına O(bulgu) olmayacak şekilde. Kanıt:
   bağlanma profilinin %90'ı; 1.000 barda 161–279 s.
5. **Açık bar denetimi** (`run_capsules` başı): izlenen seri başına iki `find_series` dizgi
   karşılaştırması yerine seri indeksi ya da bar sayısı sayacı. Kanıt: bağlı düzeltme
   profilinin %38'i (250 barda olay başına ~2 µs).
6. **Büyüme sınırı (tasarım sorusu, ana akışa):** kapanmış (invalidated/expired) hikâyeler
   dokümanda sonsuza dek kalıyor ve kapsül başına her döngüye giriyor. 1–5 yapılsa bile
   uzun yaşayan bir otoritede doküman büyür; arşiv/kesit politikası (`docs/CAPSULE.md`) ana
   akışta tartışılmalı. Bu karar değil, yalnız gözlem.
7. **Yansı paketi** +222 KB gzip (1,31 MB): Ö26/A13 (ayrı paket) sorusu güçlendi; içerik
   dökümü (twiggy) ile hangi crate'in ne getirdiği ölçülmeli.
8. **Ölçüm düzeneği:** `capsule_tick` verify'a girmedi (1.000 barda dakikalar sürüyor).
   Önerim: 1–4 yapıldıktan sonra 100 barlık kısa koşuyu (5 koşu ~40 s) gerileme denetimi
   olarak eklemek.

## 7. Ham dosyalar

`rounds/R3/experiments/S/raw/perf/`:

- `env.txt` — makine, derleyici.
- `rp2-live-native-run{1,2}.txt` — RP2 tabanı (aynı gün).
- `engine-cost-run{1,2,3}.txt` — yalnız motor.
- `capsule-tick-bound-h{100,250,500,1000}.txt`, `capsule-tick-inline-h250.txt` — oturum düzeyi.
- `profile-{close,revision-bound,revision-inline,connect}.txt` — `sample` özetleri (250 bar,
  `caps`).

Her ölçüm dosyası başında ve sonunda o anın `uptime` satırı var.
