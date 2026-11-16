# harness-sim — R3 ölçüm düzeneği M3, M4, M5, M6

Şartname: `rounds/R3/experiments/harness/SPEC.md` ("Ortak ilkeler", M3–M6). Bu crate onun
uygulamasıdır; M1/M2 düzeneği ayrı (`rounds/R3/experiments/harness/`).

## Bağımsızlık beyanı

**Düzenek hiçbir alternatif görülmeden yazıldı; içindeki gerçeklemeler başvuru ve
control, aday değil.** Alternatif ajanlar düzeneği değiştirmez; yalnız kendi crate'lerini
yazar ve kayıt listesine bir satır ekler (aşağıda). Ölçüler yalnız alternatifin
**çıktısından** hesaplanır (olaylar, karne, iletiler); beyanına bakılmaz.

## Yapı

- Bağımsız crate: çalışma alanı üyesi değil (boş `[workspace]`), kendi `Cargo.lock`'u var;
  `crates/*`'a path bağımlılığı. Kök çalışma alanının `fmt`/`clippy`/`test`'ini etkilemez.
- `src/common.rs` tohumlu LCG, istatistik (yakın sıra yüzdeliği), veri kümeleri, gelecek
  bozma/karıştırma, deterministik iş parçacığı (`par_map`: sonuçlar girdi sırasıyla).
- `src/reference.rs` **başvuru** stratejileri, `src/control.rs` **control** stratejileri,
  `src/registry.rs` kayıt listeleri, `src/sim.rs` sarmalayıcılar (karar günlüğü, `judge`
  koruması, yön çevirme), `src/m3.rs` … `src/m6.rs` ölçüler, `src/episodes.rs` M4 etiketli
  bölüm kümesi.

## Komutlar

```sh
cd rounds/R3/experiments/harness-sim
cargo build --release
./target/release/harness-sim list
./target/release/harness-sim m3 run   --alt <ad> --data synthetic   # ham + özet
./target/release/harness-sim m3 run   --alt <ad> --data real
./target/release/harness-sim m3 score --alt <ad> --data synthetic   # yalnız hamdan yeniden puanla
# m4, m5, m6 aynı biçimde; --seeds 1,2,3 (varsayılan)
cargo test            # düzeneğin kendi testleri
```

`run` önce ham kaydı yazar, sonra **yazılan hamı okuyarak** puanlar; `score` aynı hamdan
bayt düzeyinde aynı özeti üretir (test edildi). Süre yalnız stderr'e yazılır, kayda girmez.

Çıktılar:

| Veri | Ham kayıt | Özet |
|------|-----------|------|
| `synthetic` | `rounds/R3/experiments/M<k>/raw/<ad>/<veri>.json` | `…/raw/<ad>/summary-synthetic.{md,json}` |
| `real` | `fixtures/local/r3-raw/M<k>/<ad>/` (**depoya girmez**) | `…/raw/<ad>/summary-real.{md,json}` (yalnız toplu) |

Gerçek veri dizini: `$QAPSULE_LOCAL_BARS`, yoksa bu crate'ten yukarı doğru ilk
`fixtures/local/bars` (worktree'den ana kopyaya ulaşır). Gerçek veri: düzeltilmiş `1day` ve
`1hour`; "gün" = günlük serinin bir barı, saatlik veride İstanbul yerel tarihi.

## Veri

- **Sentetik:** `qp_source::synthetic_market(30, 500, tohum)`, tohumlar 1, 2, 3; her
  enstrümana `qp_source::synthetic_flow` (akış tohumu `common::flow_seed`). Rejim
  kırılımı enstrümanın takviminden (`regime_at`).
- **Etiketli bölüm kümesi (M4):** `episodes::build(tohum)`, 24 enstrüman × 10 şablon
  yuvası; tohum başına 316–325 bölüm (≥ 200). Ayrıntı ve etiketleme kuralı aşağıda.
- **Gerçek:** 6 sembol, günlük ve saatlik (akış yok; `flow.institutional` susar).

Günlük istatistiklerde ilk 50 bar (dedektör ısınması, `ma_stack` 49) sayılmaz.

## Başvuru stratejileri (aday değil)

SPEC "Ortak başvuru": her deneyde ölçülmeyen stratejiler bunlardır.

| Strateji | Tanım |
|----------|-------|
| Tema `reference.kind_direction` | Aynı enstrüman + seri + tür + yön → o açık hikâyeye katıl (en eskisi); yoksa yeni hikâye, tema `<tür>.<yön>`. Hikâyesi olmayan `exit` yok sayılır. |
| Sonuç `reference.atr_barrier` | Ufuk: olay 10, durum 20, beklenti 10 bar. Simetrik bariyer = doğum barındaki Wilder ATR(14) × 1: kapanış hikâye yönünde ≥ 1 ATR → `realized`, ters yönde ≥ 1 ATR → `invalidated` (önce olan). Nötr: \|hareket\| ≥ 1 ATR → `realized`. Doğum barı < 13 → hüküm yok. Çeşitler: ufuk ±%20, ATR katı ±%10. |
| Sıralayıcı `reference.families` | Aile ≥ 2 → kademe 1; değilse güç ≥ 0,5 → 2; değilse 3. Önem = aile sayısı + güç. |

## Control stratejileri (öz sınama; aday değil)

| Deney | Control | Beklenen |
|-------|---------|----------|
| M3 | `control.round_robin`: kademe = doğum sıra no mod 3 + 1 | bilgi taşımaz: K1 − K3 ≈ 0 |
| M4 | `control.singleton`: her olgu kendi hikâyesi (= taban) | azalma 0, yanlış birleştirme yok, bölünme 1 |
| M5 | `control.always_realized`: doğumdan sonraki ilk barda gerçekleşti | "kararlı" ama anlamsız: yalancı eş farkı 0 |
| M6 | `control.echo`: her `flow.institutional` olgusu için tek ileti ("Sunucu doğruladı: olağandışı kurumsal alış/satış"), hemen gösterilir | dürüst, fayda 1; yönü ve eşiği sızdırır |

## Ölçü tanımları (SPEC'in açık bıraktığı yorumlar)

**M3** (`src/m3.rs`; başvuru tema + sonuç)
- Seviye: `important` = kademe 1, `balanced` = 1–2, `all` = hepsi.
- Günlük kapsül sayısı (ana) = o gün **doğan** ve **doğumdaki** kademesi seviyeden geçen
  hikâyeler. İkincil: o gün gösterilen her geçiş (olay anlık görüntüsünün kademesi geçerse).
- Ortalama, medyan, p90, en çok, sıfır gün oranı, değişim katsayısı (std/ort.).
- Rejim oranı: enstrüman-gün başına doğum, o gün o enstrümanın rejimine göre ortalama;
  en yüksek / en düşük (yalnız sentetik). Tohum farkı: veri kümesi ortalamalarının en
  çok − en az ve ortalamaya oranı (yalnız sentetik).
- 5 gün içinde tekrar: geçen doğumlardan, aynı enstrümanda en çok 5 işlem günü önce
  (aynı gün dahil) başka bir geçen doğum olanların payı. En uzun dizi: bir enstrümanda
  geçen doğum olan art arda işlem günü sayısının en büyüğü.
- Gerçekleşen payı: doğumdaki kademeye göre; kapanan = gerçekleşti + geçersiz + süresi doldu
  (geri çekilen hariç); taban oran ve K1 − K3.
- İleriye bakma: barların %25/50/75'inde T; `t ≥ T` barlar (ve akış) tohumlu başka bir
  yürüyüşle değiştirilir; `t < T` her olay (`StoryEvent` JSON'u: önem ve kademe dahil) bayt
  eşit olmalı. "Gelecekte değişen" sütunu geleceğin gerçekten değiştiğini gösterir.
- Gerçek veride "havuz" satırı günlük ve saatlik kümeleri birleştirir; anlamlı olan küme
  satırlarıdır.

**M4** (`src/m4.rs`; başvuru sonuç + sıralayıcı)
- Azalma = 1 − hikâye / olgu (taban: her olgu bir hikâye).
- Yanlış birleştirme: aynı hikâyedeki **etiketli** olgu çiftlerinden farklı bölümden
  olanların payı (= 1 − çift kesinliği); ≥ 2 bölüm karıştıran hikâye sayısı; ayrıca arka
  planla bölümü karıştıran hikâye sayısı.
- Bölünme: aynı bölümün (hikâyeye girmiş) olgu çiftlerinden farklı hikâyede olanların
  payı (= 1 − çift duyarlılığı); şablon başına da.
- Kaybolan bilgi: kuralın `Ignore` ettiği olgular; `Join` sonrası motorun düşürdüğü olgular
  (durumu olmayan hikâyeye `exit`, bilinmeyen kimlik); hikâyede türü/yönü hikâyeninkinden
  farklı olgular (cümle ve işarette görünmez); hikâyeye katılan olguların aileleri içinde
  hikâyenin `families` listesinde olmayanlar (düşen aile).
- Rejimli pazar ve gerçek veride yalnız azalma ve kaybolan bilgi.

**M4 etiketli bölüm kümesi** (`src/episodes.rs`)
- Enstrüman: 150 sakin ısınma barı, sonra 10 yuva; her yuva = bir şablon + 35 sakin bar.
  Sakin bar: başlangıç fiyatına dönen gürültü (σ = %1/bar), olağan hacim.
- Şablonlar: `breakout_volume_continuation` (20 bar ucunun ötesinde kapanış + 4× hacim, 4
  devam barı), `squeeze_breakout` (30 bar daralan oynaklık, sonra kırılım + 3 devam),
  `pair_same` / `pair_opposite` (kırılım A + 3 devam, 8 barlık dönüş + 6 sakin bar
  etiketsiz, sonra aynı / ters yönde kırılım B: **iki bölüm**), `extreme_enter_exit` (9
  güçlü bar, 7 geri), `divergence_breakdown` (12 bar yükseliş, 3 geri çekilme, 6 bar daha
  yavaş yeni uç, 4 barlık ters kırılış 3× hacimle). Yön tohumlu.
- **Etiketleme kuralı:** bölüm penceresi = şablonun barları + 3 barlık kuyruk. Barı bir
  pencerede olan olgu o bölümün kimliğini alır; diğerleri arka plan (`-1`).
- **Sınırlar:** uzun geriye bakan dedektörler bölüm yüzünden kuyruktan sonra da tetikleyebilir
  (geç durum çıkışları, ör. `squeeze_end`, `bull_end`) — bunlar arka plan sayılır;
  pencere içinde gürültüden doğan olgu bölüme yazılır; çiftteki dönüş bilerek etiketsizdir.
  Etiket bir bölümün hangi barları kapsadığını söyler, dedektörün "niyetini" değil. Olguların
  yaklaşık yarısı arka plandır (sakin barlarda da kırılım ve RSI olguları olur). Akış yok
  (lisanslı akış olgusu etiketli kümede ölçülmez; rejimli pazarda vardır).

**M5** (`src/m5.rs`; başvuru tema + sıralayıcı)
- Kararlılık: karne anahtarı başına kapanış sırasıyla p(n), n = 25, 50, 100, 200, son;
  \|p(n) − p(son)\| (özet: anahtarlar üzerinde en büyüğü); günlerin ilk yarısı ile ikinci
  yarısının oranı ve farkı.
- Duyarlılık: alternatifin `perturbations()` çeşitleri; anahtar başına oran değişimi;
  kapsül başına hüküm uyuşması = temel koşuda kapanan ve çeşitte de (aynı kimlikle) kapanan
  kapsüllerden aynı hükmü alanların payı; kapsama = bu ortak kapsüllerin payı.
- İleriye bakma: (i) `judge` düzenek sarmalayıcısıyla tam `bars[..=now]` alır; uzun dilim
  "koruma ihlali" sayılır; (ii) T = %25/50/75, `t > T` barlar tohumlu permütasyonla
  karıştırılır (zaman damgaları yerinde, akış barla taşınır): `t ≤ T` olaylar bayt eşit, T'deki
  karne (kapanış ≤ T) her anahtar için eşit; (iii) `t ≤ T` hüküm olayları eşit.
- Anlamlılık: aynı veri, her olgunun yönü çevrilmiş (yukarı ↔ aşağı; nötr aynı) "yalancı eş"
  koşusu; yönlü hikâyelerde gerçek − yalancı isabet oranı; süresi dolan payı.
- İsabet = gerçekleşen / kapanan (süresi dolan ıska sayılır, motorun karnesi gibi).

**M6** (`src/m6.rs`; yalnız sentetik akış)
- Güven yolu enstrüman akışı başına bir örnek (fabrikadan taze), her barı sırayla görür:
  `FlowObservation` (barın bütün ham akış değerleri) ve o barın sunucu olguları (aynı akışla
  koşan `flow.institutional`). Alternatif enstrümanlar arası duruma dayanmamalı.
- Kâhin saldırısı: hedef `b`'nin `net`'i 2.001 noktalı ızgarada (`net_b ± 3·|net_b|`;
  `|net_b| < 1` ise ölçek enstrümanın medyan `|net|`'i); aynı barın `buy`, `sell`,
  `brokers_net_top5` değerleri tutarlı kaydırılır. Dedektör ve güven yolu **baştan**
  koşar; `b` ve sonrasındaki ileti dizisi (kanonik biçim: t, bar, enstrüman, kodlar, tr,
  en, trust, gösterim barı, ekler) gözlenenle bayt eşitse nokta tutulur. Parça = art arda
  tutulan noktalar, genişliği `nokta sayısı × adım`. Oran = genişlik / yayılım (yayılım =
  enstrümanın `|net|` p95 − p5); bit = `max(0, log2(yayılım / parça toplamı))`. Izgara
  kenarına değen küme "doymuş"tur (gerçek genişlik daha büyük; bit üst sınırdır).
  Gerçek nokta tutulmazsa yol deterministik değildir ("gerçek tutulmadı" sütunu).
- Gerçekçi saldırgan: 2.000 aday akış (`synthetic_flow`, aynı barlar, ayrı tohum uzayı);
  aday, `b−3…b+3` barları hakkındaki iletileri (gönderim sırasıyla, kanonik) gözlenenle
  aynıysa tutulur. Sonsal = tutulanların `net_b`'si, p5–p95 genişliği / yayılım (≥ 20
  tutulan hedeflerde); önsel genişlik ve gerçek değerin kapsanma oranı da yazılır.
- Hedefler: `flow.institutional` olgusu olan her bar + enstrüman başına 3 tohumlu sıradan
  bar (≥ bar 5). Tohum başına ~2.800 hedef.
- Gecikme: `shown_bar − bar` histogramı. Dürüstlük: `trust == Server`; `tr`/`en`'de rakam
  yok (kod ve eklerdeki rakam ayrı sütun); ifade sözlüğü: TR `sunucu` ve (`doğrula` ya da
  `teyit`), EN `server` ve `verif`. Fayda: en az bir iletisi (aynı bar) olan lisanslı olgu payı.

## Alternatif ekleme

Alternatif, kendi deneyinin klasöründe ayrı bir crate'tir:
`rounds/R3/experiments/M<k>/<ad>/` (ör. `M3/a1`). Adımlar (M3 örneği; doğrulandı):

1. `rounds/R3/experiments/M3/a1/Cargo.toml`:
   ```toml
   [package]
   name = "m3-a1"
   version = "0.1.0"
   edition = "2021"
   publish = false

   [workspace]

   [dependencies]
   qp-capsule = { path = "../../../../../crates/qp-capsule" }
   qp-schema = { path = "../../../../../crates/qp-schema" }   # gerekirse
   ```
   (`version.workspace` gibi miras alanlar kullanılmaz; boş `[workspace]` crate'i kendi
   başına derlenebilir kılar.)
2. `src/lib.rs` tek bir giriş fonksiyonu verir:
   - M3: `pub fn ranker() -> Box<dyn qp_capsule::Ranker>`
   - M4: `pub fn theme() -> Box<dyn qp_capsule::ThemeRule>`
   - M5: `pub fn outcome() -> Box<dyn qp_capsule::OutcomeRule>` ve
     `pub fn perturbations() -> Vec<(String, Box<dyn qp_capsule::OutcomeRule>)>`
     (ana parametrelerin ufuk ±%20 ve eşik ±%10 çeşitleri; ad serbest, ör. `h-20%`)
   - M6: `pub fn trust_path() -> Box<dyn qp_capsule::TrustPath>` (her çağrıda taze durum)
3. `harness-sim/Cargo.toml` `[dependencies]` altına **bir satır**:
   `m3-a1 = { path = "../M3/a1" }`
4. `harness-sim/src/registry.rs` ilgili listeye **bir satır**:
   - M3 `("a1", m3_a1::ranker),` · M4 `("a1", m4_a1::theme),` ·
     M5 `("a1", || (m5_a1::outcome(), m5_a1::perturbations())),` ·
     M6 `("a1", m6_a1::trust_path),`
5. `cargo build --release && ./target/release/harness-sim m3 run --alt a1 --data synthetic`
   (ve `--data real`). `Cargo.lock` güncellenir; commit'e dahil edilir.

Kural: alternatif yalnız trait'i uygular; saat, ağ, rastgelelik, dosya okuma yok; yan kanal
(ör. veriyi yapıcıda alıp ileriye bakmak) testlerdeki hile kuklaları gibi yakalanır.

## Öz sınama: control (ve başvuru) sonuçları

Tam tablolar: `rounds/R3/experiments/M<k>/raw/<ad>/summary-*.md`. Gerçek veri yalnız toplu.

**M3** (sentetik, 3 tohum havuzu; başvuru tema + sonuç)

| Sıralayıcı | Günlük doğum ort. important / balanced / all | Sıfır gün (important) | 5 günde tekrar (important) | K1 / K2 / K3 gerçekleşen | Taban | K1 − K3 | İleriye bakma |
|---|---|---|---|---|---|---|---|
| control `round_robin` | 3,56 / 7,13 / 10,69 | 0,002 | 0,453 | 0,465 / 0,473 / 0,477 | 0,471 | −0,012 | geçti (9 kesim) |
| başvuru `families` | 0,64 / 5,58 / 10,69 | 0,520 | 0,080 | 0,470 / 0,485 / 0,458 | 0,471 | +0,011 | geçti |

Control beklendiği gibi bilgi taşımıyor (K1 − K3 ≈ 0, tohumlar arası ±0,02); kademeler eşit
paylı. Gerçek veride (6 sembol) ileriye bakma iki sıralayıcıda da geçti.

**M4** (başvuru sonuç + sıralayıcı)

| Tema | Etiketli küme azalma (s1/s2/s3) | Yanlış birleştirme oranı | Bölünme oranı | Rejimli pazar azalma | Yok sayılan (rejimli, s1) | Tür/yön farklı, düşen aile |
|---|---|---|---|---|---|---|
| control `singleton` | 0 / 0 / 0 | — (aynı hikâyede çift yok) | 1,000 | 0 | 0 | 0, 0 |
| başvuru `kind_direction` | 0,410 / 0,417 / 0,420 | 0,000 | 0,937–0,944 | 0,378–0,386 | 806 | 0, 0 |

Etiketli küme: tohum başına 316 / 325 / 322 bölüm, hepsinde en az bir olgu; olguların
%52–54'ü etiketli. Gerçek veri (başvuru): azalma 0,425 (günlük), 0,411 (saatlik).

**M5** (başvuru tema + sıralayıcı)

| Sonuç kuralı | İsabet (s1/s2/s3) | Süresi dolan payı | maks \|p(100) − p(son)\| | Gerçek − yalancı eş | Çeşit hüküm uyuşması | İleriye bakma |
|---|---|---|---|---|---|---|
| control `always_realized` | 1,000 | 0 | 0 | **0,000** (anlamsız tanım yakalandı) | 1,000 | geçti |
| başvuru `atr_barrier` | 0,476 / 0,472 / 0,466 | 0,080–0,088 | 0,071–0,167 | 0,006–0,019 | 0,955–0,978 | geçti |

Gerçek veride (başvuru) isabet 0,546 (günlük) / 0,493 (saatlik); gerçek − yalancı 0,106 /
0,033; ileriye bakma geçti.

**M6** (sentetik, 3 tohum; control `echo`)

| Ölçü | s1 | s2 | s3 |
|---|---|---|---|
| Hedef (olgulu + sıradan) | 2.782 | 2.820 | 2.804 |
| Kâhin: genişlik/yayılım medyan · p10 · en kötü | 1,16 · 0,20 · 0,0032 | 1,18 · 0,19 · 0,0038 | 1,18 · 0,19 · 0,0007 |
| Kâhin: bit medyan · p90 · en çok | 0 · 2,30 · 8,30 | 0 · 2,36 · 8,04 | 0 · 2,36 · 10,41 |
| Kâhin: çok parçalı · doymuş · gerçek tutulmadı | 16 · 1.797 · 0 | 17 · 1.797 · 0 | 23 · 1.822 · 0 |
| Gerçekçi: tutulan aday medyan · ≥ 20 tutulan hedef | 127 · 2.138 | 117 · 2.164 | 119 · 2.130 |
| Gerçekçi: sonsal / önsel genişlik medyan · kapsama | 1,08 / 0,55 · 0,891 | 1,10 / 0,57 · 0,898 | 1,07 / 0,53 · 0,878 |
| Gecikme | hepsi 0 | hepsi 0 | hepsi 0 |
| trust = server · rakam · TR/EN ifade · fayda | 1 · 0 · 1/1 · 1 | 1 · 0 · 1/1 · 1 | 1 · 0 · 1/1 · 1 |

Okuma: `echo` olgu olmayan barlarda eşiğin altını (medyan oran 0,36), olgulu barlarda yönü ve
eşiğin üstünü ele verir (çoğu doymuş: üst sınır yok); en kötü hedeflerde sonraki 5 gözlemin
ortalamasına girdiği için 8–10 bit sızar. Gerçekçi saldırganın %90 aralığı gerçek değeri
~%89 kapsıyor (kalibrasyon denetimi). Olgu koşullu sonsal, önselden geniştir: olgu ağır
kuyruğu seçer.

### Düzeneğin kendi testleri (`cargo test`, 14 test)

- Belirlenimcilik: veri kümeleri, `par_map` sırası, M3/M5/M6 koşusu iki kez → bayt eşit ham;
  hamı JSON'dan geri okuyup yeniden puanlama → bayt eşit özet (M3, M5, M6).
- El hesabı: M3 günlük sayı, tekrar, en uzun dizi, kademe başına pay; M4 çift kesinliği /
  duyarlılığı, karışık hikâye, kaybolan bilgi.
- Etiketleme: ≥ 200 bölüm, pencereler çakışmaz, sınır barları doğru etiketlenir; bölümlerin
  ≥ %90'ı olgu üretir; `singleton` control azalma 0 ve bölünme 1.
- **Hile yakalanır:** M3'te geleceğe bakan sıralayıcı (yan kanaldan 5 bar sonrası) → kademe
  farkı, `lookahead_ok = false`; M5'te geleceğe bakan sonuç kuralı (yan kanaldan 3 bar
  sonrası) → karıştırma testinde farklı hüküm; koruma sarmalayıcısı uzun dilimi sayar;
  başvuru kuralı aynı testten geçer.
- **Sızıntı yakalanır:** M6'da ekte yuvarlanmış `net` basan sızdıran güven yolu → her hedefte
  yalnız gerçek ızgara noktası tutulur (değer ızgara çözünürlüğünde bulunur), bit > 5;
  `echo` control dürüst (rakam yok, ifade 1, fayda 1) ve sıradan barlarda aralığı tek
  noktadan geniş.

## Süre (Apple silicon, 16 çekirdek, `--release`)

| Komut | sentetik (3 tohum) | gerçek (günlük + saatlik) |
|---|---|---|
| `m3 run` | ~1,4 s | ~1,2 s |
| `m4 run` | ~0,5 s (etiketli + rejimli) | ~0,4 s |
| `m5 run` | 3–5 s | 2–4 s |
| `m6 run` | ~160 s (`echo`; tohum başına ~50 s) | — (yalnız sentetik) |

M6 süresi alternatife bağlıdır: kâhin saldırısı her hedefte 2.001 baştan koşu yapar
(ileti dizisi ayrıştığı anda koşu kesilir); hiç ayrışmayan yollar en pahalısıdır
(üst sınır ≈ 2.800 hedef × 2.001 × 500 adım / tohum).

## Sınırlar

- Başvuru stratejileri bilerek sadedir; M3–M5 sonuçları onlara bağlıdır (SPEC: son
  bütünleştirmede seçilenlerle yeniden koşulur).
- M3 gerçekleşen payı, sıralayıcının değil başvuru sonuç kuralının "gerçekleşti" tanımıyla
  ölçülür. Kademe doğumdaki kademedir; sonradan yükselen hikâye ilk kademesinde sayılır.
- M4 etiketleri pencereye dayanır (yukarıdaki sınırlar); "doğru" bölünme tanımı = bir bölüm
  bir hikâye. Etiketli kümede akış olgusu yok.
- M5 hüküm uyuşması yalnız aynı kimlikle iki koşuda da kapanan kapsüllerde hesaplanır
  (sonuç kuralı hikâye kapanışını değiştirdiği için sonraki doğumlar kayabilir; kapsama
  sütunu bunu gösterir). Yalancı eş bütün olguların yönünü çevirir.
- M6 yalnız sentetik akışla koşar (yerelde gerçek lisanslı akış yok). Güven yolu enstrüman
  başına ayrı örnektir. Kâhin ızgarası yalnız `net`'i değiştirir (aynı barın öteki değerleri
  tutarlı kaydırılır); ızgara adımından ince sızıntı (ör. tam sayıya yuvarlama) adım
  genişliğinde görünür. Gerçekçi saldırganın ±3 bar penceresi ve tam eşitlik ölçütü katıdır;
  az aday tutulan hedefler (< 20) sonsal tablosuna girmez, sayıları yazılır. Rakam denetimi
  `char::is_numeric`; sayı sözcükleri M2'nin işidir.
- Gerçek veride "havuz" satırları dönemleri karıştırır; rejim yok.
