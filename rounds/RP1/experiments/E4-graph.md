# E4 — Hesap grafiği (zincirlerde ortak alt hesap) ve içerik adresli sonuç önbelleği

Dal: `rp1/e4-graph` (taban `caab6e3`). Yürüten: orkestratör (alt ajan başlatılamadı, bkz. REPORT).

## Hipotezler (denemeden önce yazıldı)

**H4.1 — Zincir taraması önek paylaşımıyla 10 kattan fazla hızlanır.** Profil C'nin her
ailesinde ilk aşamanın yalnızca 10 farklı parametre kümesi var (`sma>ema>rsi`'de 100 farklı
`sma>ema` öneki). Bugün her varyasyon iç içe `Chain` (`Box<dyn Study>`, `Option`) ile bar bar
koşuyor: 1.000 varyasyon × 10.000 bar × 2–3 sanal çağrı. Grafik yürütücü (a) her farklı
öneki bir kez hesaplar, (b) son aşamayı aynı önekin tanımlı sonekine tek bir tarama
(şerit çekirdekleri) olarak uygular. İş: `ema>rsi` için 10 EMA + 1.000 RSI (tarama hızında),
yani A profilindeki RSI taramasına yakın (≈ 20–25 ms yerel); bugün ≈ 100 ms. `rsi>bollinger`
≈ 10 RSI + 1.000 Bollinger ≈ 45 ms (bugün 178); `sma>ema>rsi` ≈ 110 kısa hesap + 1.000 RSI
≈ 25 ms (bugün 167). Beklenen: 4–7 kat.

**H4.2 — Bit-eşitlik yapı gereği korunur.** `Chain` ikinci aşamaya, birincinin ısınması
bitene kadar hiçbir şey vermez; ikinci aşamanın ilk girdisi birincinin ilk değeridir. Bu,
ikinci aşamayı birinci çıktının tanımlı soneki üzerinde sıfırdan koşmakla aynı işlemleri
aynı sırada yapar. Çıktı sütunu, sonekin çıktısının önüne birincinin ısınma uzunluğu kadar
`NaN` eklenmiş halidir. (Varsayım: yerleşik hesapların ısınması indekse bağlı; tanımlı bir
değer sonradan `None` olmaz — `Chain` belgesindeki varsayımın aynısı.)

**H4.3 — İçerik adresli önbellek, tekrar eden sorularda hesaplamayı ortadan kaldırır; anahtar
maliyeti küçüktür.** Anahtar = girdinin bitlerinin özeti + hesap adı + parametreler.
10.000 barlık bir girdinin 64 bit kelime bazlı özetini çıkarmak ≈ 5–10 µs; bayt bazlı FNV
≈ 50–80 µs. Bir isabet, bir varyasyonu yeniden hesaplamaktan (≈ 20–40 µs) ancak sütun
kopyalanmazsa (paylaşılan sahiplik, `Arc`) ucuzdur; tek varyasyon için önbellek kazancı
küçük, bir tarama ya da zincir öneki için büyük. Önbellek determinizmi bozmaz: değer, anahtarın
fonksiyonudur; çarpışma riski 64 bit özetle ihmal edilebilir ama sıfır değil (tam eşitlik
kontrolüyle sıfırlanabilir).

## Deney 4.1 — `graph::sweep_chains` (önek paylaşımı + son aşama tarama olarak)

**Yapılan.** `crates/qp-compute/src/graph.rs`: `Stage { study, params, output }`,
`sweep_chains(chains, x, visit)`. Her aşama doğrulanır, etkin parametre bitleriyle bir
anahtar üretilir; ardışık zincirler "son aşama hariç aynı önek" ölçütüyle gruplanır; her
farklı önek bir kez (tek varyasyonlu tarama ile) hesaplanır ve yalnızca tanımlı soneki
tutulur; grubun son aşaması o sonek üzerinde tek `sweep_each` olarak koşar. Çıktı sütunu
`NaN` önek + sonek çıktısı; `warmup` = zincirin ısınması (`Chain::warmup` ile aynı).
`profiles::c_run` artık bunu kullanıyor; T1 yolu `c_run_baseline` olarak duruyor ve
`--baseline 1` ile aynı ikilide ölçülüyor.

**Ölçüm** (aynı ikili, 5–7 koşu medyanı, ms; yük ortalaması ≈ 4–5; E2 şerit çekirdekleri
henüz yok — tarama R1 çekirdekleriyle):

| Aile | Yerel aarch64 T1 → yeni | x86_64 (Rosetta) T1 → yeni | WASM (Node 24) T1 → yeni |
|------|------------------------:|---------------------------:|-------------------------:|
| ema>rsi | 99,4 → **26,7** (3,7×) | 149,5 → **32,7** (4,6×) | 219,0 → **62,4** (3,5×) |
| rsi>bollinger | 175,6 → **57,4** (3,1×) | 249,4 → **74,2** (3,4×) | 338,2 → **77,0** (4,4×) |
| sma>ema>rsi | 161,5 → **30,6** (5,3×) | 240,0 → **38,4** (6,2×) | 375,7 → **64,7** (5,8×) |

Özetler üç hedefte de değişmedi (`d60c4040e327df75`, `51d10314ec252891`, `8c536220fb4398a5`).

**Bit-eşitlik kanıtı.** `tests/graph.rs::chain_sweep_equals_nested_chain`: macd.signal → ema,
macd.histogram → rsi, bollinger.upper → rsi, bollinger.bandwidth → sma, 4 aşamalı
sma>ema>rsi>bollinger, tek aşamalı zincir, tekrar eden önek; n = 0, 1, 12, 40, 700 (ısınmadan
kısa seriler dahil); iç içe `Chain`'e karşı bit bit. Ayrıca `tests/profiles.rs` (C alt kümesi
özeti) değişmeden geçiyor.

**Kalan süre nerede.** Yeni yolda süre neredeyse tamamen son aşamanın taramasında (ema>rsi:
1.000 RSI ≈ profil A'nın RSI taraması; rsi>bollinger: 1.000 Bollinger). Yani C artık A'nın
çekirdek hızına bağlı; E1/E2 kazançları buraya doğrudan geçer.

**Karar: al.** Bit-eşit, genel (her zincir, her çıktı indeksi), 3–6 kat.

## Deney 4.2 — İçerik adresli sonuç önbelleği (`graph::ResultCache`)

**Yapılan.** Anahtar = (`content_hash(x)` 128 bit, hesap adı + etkin parametre bitleri);
değer `Arc<SweepColumns>`. `content_hash` iki bağımsız 64 bitlik kelime özeti (değer bitleri +
uzunluk), her hedefte aynı.

**Ölçüm** (yerel aarch64, `cargo bench -p qp-compute --bench profiles -- --only cache`):

| İşlem | Süre |
|-------|-----:|
| 10.000 barın özeti | 13 µs |
| rsi(14) ıska (hesap + ekleme) | 75 µs |
| rsi(14) isabet | 0,5 µs |
| 1.000 varyasyonluk RSI taraması, hepsi isabet (yeniden özet + 1.000 arama) | 0,44 ms |
| Aynı tarama, önbelleksiz | 23,3 ms |
| Bellek, 1.000 varyasyon | ≈ 80 MB |

**Değerlendirme.** Ajan aynı soruyu (ya da örtüşen bir taramayı) tekrar sorduğunda ≈ 50 kat.
Bedel bellek: 10k barlık 1.000 varyasyon 80 MB. Değer anahtarın saf fonksiyonu olduğu için
determinizmi bozmaz. 128 bitlik özet çarpışması pratikte yok sayılabilir ama sıfır değil.

**Karar: sonra.** API çekirdekte deneysel olarak duruyor (başka yol kullanmıyor). Açık
konular: bellek bütçesi ve tahliye politikası (LRU / boyut sınırı), canlı seride son barın
değişmesiyle anahtarın geçersizleşmesi (son bar hariç önek özeti + `Streaming` durumu),
belge düzeyinde (şema) bir "hesap kimliği" kavramı. Bunlar mimari karar; öneri olarak
REPORT'a.

## Öneriler (kapsam dışı)

- `docs/CONCEPTS.md`: "Hesap grafiği" (zincirlerin ortak önekleri paylaşan DAG'ı) ve
  "Hesap kimliği" (girdi özeti + hesap + parametre; önbellek anahtarı) kavramları.
- Şemada zincir tanımı (`Stage` benzeri) bugün yok; `qp-schema`'ya eklenirse `sweep_chains`
  doğrudan doküman üstünden çağrılabilir.
