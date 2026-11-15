# R3/S-A — Seçimlerin ürünleşmesi, cümle düzeltmeleri, motordan cümle, hassasiyet, motor maliyeti

Dal `r3/s-engine` (`r3/int` `dd62244` üstünde). Yazılan yerler: `crates/qp-capsule/`,
`rounds/R3/experiments/S/`, `Cargo.lock`. Deney crate'lerine ve düzeneklere dokunulmadı.

## 1. Ürünleşen seçimler

| Ürün modülü | Ölçülen crate (commit) | Seçim |
|---|---|---|
| `qp_capsule::theme::KindDir` | `M4/kinddir` (`b38b317`) | M4a tür + yön |
| `qp_capsule::outcome::Barrier` | `M5/barrier` (`ab4f689`) | M5a oynaklık bariyeri |
| `qp_capsule::rank::Confirm` | `M3/confirm` (`38147c9`) | M3c teyit merkezli |
| `qp_capsule::realize::Template` (+ `morph`, `num`, `tr`, `en`) | `M2/template` (`85b5b5d`) | M2a şablon dilbilgisi |

`EngineConfig::r3()` = `KindDir::default()`, `Barrier::default()` (`"barrier"`, ölçek 1, 1),
`Confirm::default()`. Her modülün başında köken notu var (deney, commit, `RESULTS.md`).
Kod kopyadır; tek değişiklikler: `qp_capsule::` yolları, düzenek giriş fonksiyonlarının
(`theme()`, `ranker()`, `outcome()`, `perturbations()`) kaldırılması, `ConfirmRanker` →
`Confirm`, `Realizer` yapısı → `Template`, `Barrier: Default`. M2'nin birim testleri
`realize/tests.rs` olarak taşındı (bir beklenti düzeltmeye göre genişletildi, iki test eklendi;
§2). `qp-capsule` artık `qp-compute`'a bağlı (bariyerin ATR'si, cümlenin RSI/SMA/Bollinger'ı).

### Çapraz denetim (bayt eşitliği)

Deney crate'leri kendi `[workspace]` köküdür; kök çalışma alanı üyesinin bağımlılığı
olamıyorlar ("multiple workspace roots" hatası, denendi). Bu yüzden denetim ayrı bir crate:
`rounds/R3/experiments/S/crosscheck/` (kendi `Cargo.lock`'u). Tek bir `qp-capsule` kopyasına
hem ürün modülleri hem deney crate'leri bağlanır.

| Denetim | Sonuç |
|---|---|
| Cümle: M2 düzeneğinin 73 durumu (`harness m2 inputs` → `crates/qp-capsule/tests/fixtures/m2_inputs.json`, sentetik), TR + EN | **59/73 bayt eşit; farklı 14'ü tam olarak §2'deki düzeltme listesi** (test listeyi birebir sınar) |
| Cümle düzeltmesinden önce (aynı kod, düzeltmesiz) | 73/73 bayt eşit |
| `simulate`, `synthetic_market(30, 500, s)` + sentetik akış, s = 1, 2, 3; ürün `EngineConfig::r3()` ↔ deney (kinddir, barrier, confirm) | olaylar, karne satırları, olgular JSON **bayt eşit**: s1 11.951 olay / 4.240 karne / 8.405 olgu (olay JSON'u 9,96 MB); s2 11.805 / 4.169 / 8.318; s3 11.700 / 4.090 / 8.259 |
| Kural adları | (`kinddir`, `barrier`, `confirm`) ikisinde aynı |
| Dolaylı: hassasiyet benzetiminin piyasa geneli `important` satırı (§5) | M3'ün bütünleşik ölçümüyle aynı: 1,79 / 2 / 4 / 8, sıfır gün 0,150; `balanced` 3,78 |

Çalışma alanı içinde: `crates/qp-capsule/tests/r3.rs::realizer_equals_measured_m2_template_except_documented_fixes`
ürün cümlecisini `tests/fixtures/m2_sentences.json`'a karşı sınar (ölçülen 73 cümle + ürünün
farklı 14'ü); bu dosyanın ölçülen crate'in güncel çıktısı olduğunu `crosscheck` testi sınar.

Yeniden koşum (denetçi için):

```text
cargo test --manifest-path rounds/R3/experiments/S/crosscheck/Cargo.toml          # 4 test
cargo run --release --manifest-path rounds/R3/experiments/S/crosscheck/Cargo.toml -- sim
cargo run --release --manifest-path rounds/R3/experiments/S/crosscheck/Cargo.toml -- diff
cargo test -p qp-capsule --test r3
```

Not: `qp-capsule`'e `qp-compute` bağımlılığı eklendiği için `harness/` ve `harness-sim/`
`--locked` olmadan derlenince kendi `Cargo.lock`'larına tek satır (`qp-capsule` → `qp-compute`)
yazar; düzenekler değiştirilmedi, kilit dosyaları geri alındı.

## 2. Cümle düzeltmeleri (yalnız bunlar değişti)

**(i) Akış yuvası (M2b bulgusu).** Kurumsal net akış koşulunda (`subject = net`) durum ve
beklenti cümleleri "net akış ortalamanın altında/altına indi" diyordu; aşağı yönlü akışta bu
"satış zayıftı" diye okunur. Artık yön sözcükle söylenir ve ölçüt "her zamankinden güçlü /
stronger than usual"dır: TR "kurumsal net satış 30 bardır her zamankinden güçlü (son günlerin
ortalaması 452.000)", "… her zamankinden güçlü hâle geldi; … yeniden olağan düzeyine geriledi;
artık her zamankinden güçlü değil", "… 3 bar içinde her zamankinden güçlü hâle gelirse … teyit
olur"; EN "net institutional selling … has been stronger than usual (recent average 452,000)",
"… became stronger than usual; it eased back to usual …; no longer stronger than usual", "If net
institutional selling … becomes stronger than usual … within 3 bars". Yukarı yönde "net alış /
net buying". Olay cümlesi zaten doğruydu ("net satış … ortalamayı belirgin biçimde aştı"),
değişmedi. Rakamsız ailede parantez düşer, rakam çıkmaz (test).

**(ii) Aşırılığın sonucu (M5 RESULTS).** Bariyer kuralında `extreme` için *gerçekleşti* =
"uzama sürdü" (fiyat hikâyenin yönünde bariyere ulaştı), *geçersiz* = "uzama sürmedi" (fiyat geri
döndü). Eski not "Beklenen sonuç gerçekleşti / The expected outcome came true" aşırı alımda
"dönüş beklentisi tuttu" diye okunabiliyordu. Yeni not: TR "Uzama sürdü: fiyat bugün
yükselmeye devam etti; sinyal gerçekleşti sayıldı." / "Uzama sürmedi: fiyat bugün geri düştü;
sinyal gerçekleşmedi sayıldı; artık geçerli değil."; EN "The stretch continued: the price kept
rising today, so it counts as played out." / "The stretch did not continue: the price fell back
today, so it counts as not played out; it is no longer valid." (aşırı satımda düşmeye /
geri yükseldi, falling / rose back).

**(iii) Bütünleştirme (M2 durumlarında etkisiz).** `realize` günlük seriyi `.1d` ile tanıyordu;
motorun seri kimlikleri `.1day` ile biter (`qp-source` periyotları). `.1day` da günlük sayılır
("bugün", "dün"). 73 durumun hiçbiri değişmez (çapraz denetim).

Testler: `realize::tests::net_flow_condition_says_buying_or_selling_stronger_than_usual`,
`realize::tests::extreme_outcome_says_whether_the_stretch_continued`; `time_markers_per_class`
biten durum için "crossed" yanında "became"i de kabul ediyor (akış cümlesinde "crossed" yok).

### Değişen durumlar (14; tam liste)

Akış: s45, s46, s47, s48 (durum), s69, s70, s71, s72 (beklenti). Aşırılık: s19 (olay, geçersiz),
s42, s44 (durum, gerçekleşti), s65 (beklenti, geçersiz), s66, s67 (beklenti, gerçekleşti).
Önce/sonra metinleri: `crosscheck diff` (aşağıda dört örnek; tamamı `crates/qp-capsule/tests/fixtures/m2_sentences.json`, `measured` ↔ `product_changes`).

- s48 TR önce: "EREGL'de kurumsal net akış 30 bardır son günlerin ortalamasının (452.000) altında; şu an 442.960. …"
  sonra: "EREGL'de kurumsal net satış 30 bardır her zamankinden güçlü (son günlerin ortalaması 452.000); şu an 442.960. …"
- s47 EN before: "About a week ago, net institutional flow in BIMAS crossed below the recent average; it is no longer below the recent average. …"
  after: "About a week ago, net institutional selling in BIMAS became stronger than usual; it is no longer stronger than usual. …"
- s44 TR önce: "… Aşırı alım sürüyor. Beklenen sonuç bugün gerçekleşti. …" sonra: "… Aşırı alım sürüyor. Uzama sürdü: fiyat bugün yükselmeye devam etti; sinyal gerçekleşti sayıldı. …"
- s65 EN before: "… The expectation was invalidated today and is no longer valid. …" after: "… The stretch did not continue: the price rose back today, so it counts as not played out; it is no longer valid. …"

## 3. Motordan cümle (K26, K33)

`qp_capsule::sentence::sentences_at(engine, realizer, capsule, bars, now, reader) -> Option<Sentences>`
(`Sentences { tr, en, still_holds, current, redacted, stage, … }`):

- Hikâye, motorun olay günlüğünde `now`'a kadarki son geçişteki hâliyle; olgular `now`'a kadar
  katılanlar. `now`'dan sonraki bar okunmaz (test: `bars[..=now]` ile aynı sonuç).
- `condition_now(lead, facts, bars)`: koşul konusu `bars[..=now]`'dan `qp-compute` ile:
  `rsi` → RSI(14) (dedektörün aynı çalışması; giriş barında dedektörün `value` kanıtına bit
  düzeyinde eşit, test), `close`, `close_dev` ve `bandwidth` → Bollinger(20, 2). Eşik lead
  olgunun kanıtından; hareketli anahtarlar (`sma20`, `sma50`, `middle`, `half_width`) `now`'da
  yeniden hesaplanır. Durumun kendi `exit` olgusu katıldıysa `still_holds = false`.
  `within_bars` taşıyan durum koşulu (sıkışma: "20 bar içinde bant dışına kapanış") tutma
  koşulu değil çözülme beklentisidir: cümle o koşulsuz kurulur, `still_holds` = çıkış yok.
  Kapanmış (geçersiz/süresi doldu/geri çekildi) durum hiçbir zaman "sürüyor" değildir.
- `Reader { entitlements }` (§18.3 biçimi: `*`, `SYMBOL`, `feed/SYMBOL`, `feed/*`):
  lisanslı lead olgu yetkisiz okuyucuya `redacted = true` (rakam yok); `current` gizlenir.

K26 uçtan uca (`tests/r3.rs::k26_rsi_crossed_70_now_lower_is_past_tense_no_longer`, gerçek
dedektörler + `EngineConfig::r3()`):

> THYAO'da RSI 19 bar önce 70'in üstüne çıktı; şu an 52,6, artık 70'in üstünde değil. Aşırı alım
> sona erdi. Uzama sürdü: fiyat 17 bar önce yükselmeye devam etti; sinyal gerçekleşti sayıldı.
> Henüz karnesi yok.
>
> 19 bars ago, THYAO's RSI crossed above 70; it is now 52.6, no longer above 70. The overbought
> state has ended. The stretch continued: the price kept rising 17 bars ago, so it counts as
> played out. No track record yet.

Aynı hikâye giriş barında: "THYAO'da RSI bugün itibarıyla 70'in üstünde; şu an 72,4. Aşırı alım
sürüyor." Lisanslı akış: yetkisiz okuyucuda cümlelerde hiç rakam yok, "rakamlar kurumsal akış
lisansı gerektiriyor"; `flow/*` yetkili okuyucuda rakamlar var (test).

## 4. İleriye bakma ve yeniden oynatma (gerçek dedektörler + seçilen yapılandırma)

`tests/r3.rs` (sentetik 8 × 260, akışlı):

- `look_ahead_real_detectors_chosen_config` (denetçi notu N4): 3 kesim (%25/50/75) × 2 bozma
  (kesimden itibaren barları ve akışı başka bir yürüyüşle değiştirme; kesimden sonrakileri
  karıştırma). Kesimden önceki olaylar JSON bayt eşit; kesimden önce doğanların
  `ledger_at_birth`'i eşit; kesimdeki karne (her anahtar için `ledger_stat(key, T)`) eşit;
  gelecek gerçekten değişti (tüm olaylar farklı). Geçti.
- `history_replay_reproduces_capsule_history` (ölçüt 4): iki koşu bayt eşit (olgular ve motor
  anlık görüntüsü); kaydedilen olgular taze motora bar bar oynatılınca anlık görüntü ve olaylar
  bayt eşit. Geçti.

## 5. Hassasiyet benzetimi (ölçüt 12)

`crates/qp-capsule/examples/sensitivity.rs` → `S/sensitivity.{md,json}` (sentetik) ve
`S/sensitivity-real.{md,json}` (yerel gerçek veri, yalnız toplu). Belirleyici: iki koşu aynı
SHA-1 (`sensitivity.json` `7f8acd93…`, `sensitivity.md` `2ee5611c…`).

```text
cargo run --release -p qp-capsule --example sensitivity -- synthetic   # iki kez; shasum aynı
cargo run --release -p qp-capsule --example sensitivity -- real        # yerel veri varsa
```

Kitle: izleme listesi 10 (SYN01–SYN10), pozisyon 3 (SYN01, SYN04 açığa, SYN12 listenin
dışında), etkin niyet 1 (SYN07); evren 11 enstrüman. 3 tohum × 450 sayılan işlem günü (ilk 50
bar ısınma) = 1.350 gün havuz. Günlük kapsül = evrende o gün doğan ve geçen hikâye (M3 tanımı).

| Seviye (kitle, doğum/gün) | Ort. | Medyan | p90 | En çok | Sıfır gün | Pozisyon/niyetle geçen doğum (havuz) |
|---|---:|---:|---:|---:|---:|---:|
| `important` | 1,59 | 1 | 3 | 7 | 0,20 | 1.229 / 2.143 |
| `balanced` | 2,06 | 2 | 4 | 9 | 0,13 | 849 / 2.779 |
| `all` | 3,23 | 3 | 6 | 11 | 0,04 | 0 / 4.357 |

Grup varsayılanı: `future` → `important` 1,59/gün; `habituated` → `all` 3,23/gün. Sistem
seviyeyi değiştirmedi (her geçişte kitle önce/sonra assert). Pozisyon/niyetle geçen olaylar
(`important`): 3.177; bunların 617'si `invalidated`, 381'i `weakened`. Gerçek veri (6 sembol,
günlük 2.169 + saatlik 495 gün): `important` 1,88/gün ortalama, medyan 1, sıfır gün 0,41;
geçen doğumların 4.220/5.006'sı pozisyon/niyet geçişi (evrenin 4/6'sı pozisyon/niyet).

## 6. Motor maliyeti (ölçüt 17, yalnız motor tarafı)

`crates/qp-capsule/examples/engine_cost.rs` (tanım dosyanın başında). 30 enstrüman × 7 dedektör
(6 tür), akışkan; tik = her dedektör `update_last` (barın ilk tiki `push`) + `to_fact` +
`Engine::candidates`; bar kapanışı = `Engine::on_bar`. Bar başına 20 tik, 500 bar, bar 60'tan
itibaren ölçülür; koşu başına ortalama, koşuların medyanı.

```text
cargo run --release -p qp-capsule --example engine_cost -- 20 7   # bar başına tik, koşu
```

Makine: Apple M4 Max sınıfı, 16 çekirdek, rustc 1.92, `--release`. **Uyarı: ölçüm sırasında
makine başka ajanların koşularıyla aşırı yüklüydü (yük ortalaması 135–320, 16 çekirdekte);
duvar saati süreleri şişkin ve gürültülüdür (koşular arası ×1,9).** Ham çıktı
`S/raw/engine-cost.txt`. Aşağıdaki tablo iki oturumun medyanı (7 koşu, yük ~135–230 / 5 koşu,
yük ~230–320):

| Nicelik | Piyasa (30 enstrüman) | Enstrüman başına |
|---|---:|---:|
| Tik: dedektörler + olgu + aday | 1,77 ms / 2,51 ms | **59 µs / 84 µs** |
| — 7 dedektör `update_last` + `to_fact` | 64 µs / 85 µs | **2,1 µs / 2,8 µs** |
| — `Engine::candidates` (açık bar) | 1,70 ms / 2,41 ms | 57 µs / 80 µs |
| Tik, bar 60–199 (küçük karne) | 1,34 ms / 1,71 ms | 45 µs / 57 µs |
| Tik, bar 360–499 (büyük karne) | 2,13 ms / 3,34 ms | 71 µs / 111 µs |
| Bar kapanışı `Engine::on_bar` | 1,74 ms / 3,05 ms | **58 µs / 102 µs** |
| Tek `ledger_stat` (son karne, 4.240 satır) | — | 7,9 µs (ikinci oturum) |

Piyasa tiki başına ortalama 11,1 aday hikâye; sonda 43 açık hikâye, 11.951 olay.

**Bunlar yalnız motor maliyetidir;** oturum düzeyindeki ölçüm (komut yolu, doküman, abonelik,
mesaj) sonra yapılacak. RP2 tabanı (`rounds/RP2/REPORT.md`, oturum düzeyi, tek seri): bağlı
N=5 0,76 µs/tik (`t6d`, yerel), N=50 1,31 µs, N=500 8,71 µs; belge içi N=5 2,04 µs. Kıyas
birimleri farklı: RP2'nin tiki tek serinin bütün bağlı hesapları, buradaki enstrüman-tiki
yedi dedektör + aday hikâye hesabı.

Okuma: dedektör tarafı ucuz (enstrüman-tiki ~2 µs, RP2'nin tek seri N=5 oturum tiki
0,76 µs ile aynı düzey). Maliyetin %95'ten fazlası `Engine::candidates`/`on_bar` içinde ve
karne büyüdükçe artıyor (bar 60–199 → 360–499: ×1,6–2). Örnekleyici profil (`sample`):
zamanın çoğu `run_bar`'ın sıralama adımındaki karne taramasında (`ledger_stat`: dokunulan her
hikâye için bütün karne satırları üzerinde anahtar karşılaştırması, `memcmp`) ve ardından
`BTreeMap` klonlamasında (`candidates` sayaçları kopyalıyor). Tek tarama 4.240 satırda
~8 µs (yüklü makinede). Seçilen stratejiler (kinddir, barrier, confirm) bu maliyetin küçük
bir kısmı. Öneri §7: anahtar başına önek sayımı; davranış değişmeden tarama O(log n) olur.
Yüksüz makinede yeniden ölçülmeli (komut yukarıda; denetçi aynı komutla koşabilir).

## 7. Açık sorular ve öneriler (kapsam dışı, yapılmadı)

- **Karne taraması doğrusal:** `Engine::ledger_stat` her dokunulan hikâye için bütün karne
  satırlarını tarıyor; maliyet karne büyüdükçe artıyor (§6). Anahtar başına zaman sıralı önek
  sayımı (ikili arama, `retracted_at` ayrı tutulur) ile davranış aynı kalarak O(log n)'e iner.
  Bu bir motor değişikliğidir; ayrı görev ve kendi bayt eşitlik testiyle yapılmalı.
- **Pozisyon/niyet geçişi baskın:** `important` seviyede geçen doğumların %57'si (gerçekte %84)
  pozisyon/niyet geçişi; §9.8 "kapsül … kademeden bağımsız geçer" bütün aşamaları geçiriyor.
  Yalnız `weakened`/`invalidated`/`realized` gibi aşamaların ya da kademe ≤ 2'nin geçmesi
  düşünülebilir (K31 "özellikle geçersizleşme"); karar ana akışın.
- **Ölçülen şablonda TR ünlü uyumu hatası:** "Sinyalın ömrü" (doğrusu "Sinyalin"; *sinyal*
  ince l ile okunur). Bayt eşitliği için düzeltilmedi; düzeltme bir sözcük istisnası.
- **Karne cümlesi aşırılıkta:** "Geçmişte bu sinyal 140 seferin 62'sinde beklendiği gibi
  sonuçlandı" aşırılıkta da "uzama sürdü" anlamına gelir; aşama notu artık açık, karne
  cümlesi de aynı biçimde açıklanabilir (14 durumun dışına çıkmamak için yapılmadı).
- **Sıkışma dedektörünün koşulu:** `squeeze.bandwidth` durumunun `condition`'ı tutma koşulu
  değil, çözülme beklentisi (`within_bars = 20`). `sentence` bunu ayırıyor; sözleşmede
  (`MeaningCondition`) "durumun tutma koşulu" ile "beklenti" ayrı alan olabilir.
- `docs/CONCEPTS.md`: `Reader` (okuyucunun veri yetkileri) ve `sentences_at` sözlüğe; `DECISIONS.md`:
  `EngineConfig::r3` = M2a/M3c/M4a/M5a satırı ve cümle düzeltmeleri — ana akışta.
- Akış hikâyelerinde `current` bar verisinden hesaplanamaz (lisanslı akış barda yok);
  `still_holds`/`current` için akış beslemesinin `sentences_at`'e geçirilmesi R3 sonrası.
