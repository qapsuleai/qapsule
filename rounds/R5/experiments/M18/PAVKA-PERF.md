# M18 — `pavka` sahne kurulumunun hızlandırılması (yalnız başarım)

Dal `r5/m18-pavka-perf` (`r5/m18-pavka` d62207a'dan). Tasarım değişmedi: aynı girdiyle sahne
tarifi önceki kodla bayt eşit (aşağıda kanıt). "Kümelemeyi erken başlat" görsel bir değişiklik
olduğu için yapılmadı. Bu not yorum ya da seçim içermez; yalnız değişikliği ve ölçümü verir.

## Neden (ölçülen)

M18 yeniden koşusunda `pavka`'nın JIT kapalı sürüklemedeki sahne kurma p95'i 11 ms (50 hikâye),
`korda` 6, `r4` 4 ms; boyama geçişinin kurma dışında kalan kısmı üçünde yakın (≈ 9 ms). Fark
çekirdeğin sahne kurmasındaydı. Kurmanın parçaları WebKit 26.5'te `useJIT=false` ile (K52'nin
`PHONE` ayarındaki motor; `microbench/wkbench.mjs`, WASM `wasm` profili, 533 sentetik satır,
`microbench/microbench.rs`) ayrı ayrı ölçüldü (ortanca, ms):

| parça | önce | sonra |
|---|---:|---:|
| cihazda her kurulumda `first_facts` (1.600 bulgu, 533 satır: `BTreeSet`/`BTreeMap`, kimlikler kopyalanıyor) | 10,1 | kümesiz sahnede 0; kümeli sahnede 1,5 |
| aynısı, kimlikleri kopyalamadan (`BTreeMap<&str, &str>`; denendi, alınmadı) | 9,4 | — |
| `pavka::extend` 5 / 50 / 200 hikâye görünürken | 0,4 / 1,2 / 3,5 | 0,3 / 0,9 / 2,7 |

En büyük kalem `first_facts` idi: hikâye satırlarının bulgularından ilk olgulara bir harita her
sahne kurulumunda baştan, sıralı kümelerde dize karşılaştırmasıyla kuruluyordu; oysa bu harita
yalnız küme çiplerinin sabitleri için okunuyor.

## Değişiklik

1. `qp-session/src/viewer.rs`: `FirstFacts` — ilk olgu araması ilk sorulduğunda bir kez yapılır
   (`OnceCell`), belgenin kimlikleri kopyalanmaz, kümeler sabit (rastgele tohumsuz) FNV-1a
   karmasıyla tutulur. Cevap `first_facts(doc, rows).get(f).cloned()` ile her `f` için aynı
   (aynı bulgular aynı sırayla geçilir, aynı süzgeç, aynı "son olan kazanır" kuralı). Kümesi
   olmayan bir sahne aramayı hiç yapmaz. Cihaz (`open_scene_with`; arama derlemeden sonra
   kuruluyor, derleme belgeyi değiştirmiyor) ve otorite (`interact.rs` `story_extend`) aynı yolu
   kullanır. `first_facts` duruyor (eşitlik testi ona karşı).
2. `qp-grammar/src/stories/pavka.rs` (`extend`), aynı değerleri daha az işle:
   - şeridin uçları yerleştirmede bir kez hesaplanıp saklanıyor (çizimde yeniden `slot_x` yok);
   - doğum işaretinin noktaları doğrudan ilkel içine yazılıyor (ara liste ve kopya yok);
   - şerit katmanının kimliği, etiketi ve küme sabitinin anahtarı `format!` yerine birleştirme
     ile kuruluyor (aynı dize);
   - sağ kenardaki "canlı" sözcüğü her canlı şerit için değil bir kez sığdırılıyor (aynı girdi).
3. Yapılmayan: "aynı girdide (aralık, liste sürümü, odak, ayar, çerçeve) şerit yerleşimini yeniden
   kurma" önbelleği. Sürüklemede her kurulum yeni bir görünen aralığa denk geliyor (sayfa sahneyi
   yalnız çekirdek bir değişiklik bildirince ister: `main.js` `paintPass` → `take_changes`) ve
   şeritlerin piksel aralıkları, dolayısıyla paketleme, aralığa bağlı; bu yüzden böyle bir önbellek
   sürüklemede isabet etmezdi. Kurulum başına çağrı sayısını ayrıca ölçmedim (akıl yürütme, kod
   okumasından).

## Bayt eşitlik kanıtı

- **Dilbilgisi düzeyi** (`crates/qp-grammar/tests/story_pavka_equal.rs`): d62207a'daki
  `extend` olduğu gibi `tests/pavka_ref/mod.rs`'te duruyor (yalnız yollar ve kullanılmayan kayıt
  öğeleri farklı). Test aynı girdiyi eski ve yeni fonksiyona verip sahneleri hem `PartialEq` hem
  `serde_json::to_vec` baytlarıyla karşılaştırıyor: 5 / 50 / 200 hikâye (kısa ve uzun ömürlü 200)
  ve 533 hikâyelik liste, toplam 13 görünen aralık (birkaç bardan tümüne) × odak 6 (ortadaki,
  canlı, kapanmış, aralık dışı, yok, bilinmeyen) × görünüş 2 (`r4`, `ornek`) × tema 4 (varsayılan,
  `dark`, `light`, bilinmeyen) × kapanmışlar gösterili/gizli × `tr`/`en` × çerçeve yüksekliği 3
  (640, 400, 200 = hikâye kısmı yok) ve yarı bulguların ilk olgusu bilinmiyor. **7.488 sahnenin
  hepsi eşit**; 1.456'sında küme var, 2.496'sı erken dönüş.
- **Cihaz düzeyi, uçtan uca** (`crates/qp-wasm/tests/story_pavka_equal.rs`): gösterimin
  görüntüleyici çekirdeği (`story: pavka`, bir kapsül grafiğe açık), demo dünyası (adım 160; `tr`,
  `en` × varsayılan/`light`/`dark`; açılış aralığı, iki yakınlaştırma, tümü; her birinde
  kapanmışlar gösterili ve gizli; sonra odak başka hikâyeye) ve yoğun dünya (geçmiş 1000, gün 1200,
  adım 200; `tr` varsayılan, `en` `light`; 35–40 şerit ve 7–9 küme). Her sahnenin `scene_digest`'i
  (sahne JSON baytları üstünde FNV-1a) aynı test d62207a ağacında `QP_PAVKA_RECORD=1` ile
  kaydedildi (`tests/data/pavka_scene_digests.txt`, 76 sahne, 46 farklı); yeni kodla **76'sı da
  aynı**. Bu test cihazdaki `FirstFacts` yolunu da kapsıyor (yoğun dünyada kümeler var).
- **`FirstFacts` birim testi** (`viewer.rs`): listelenmiş/olgusuz/yinelenen kimlikler (sonra
  olgusuz, sonra başka olgulu), listelenmemiş, belgede olmayan ve kimsenin olmadığı kimlikler için
  eski haritayla aynı cevap.

## K52 (M18'in yöntemiyle, aynı oturumda)

Yöntem M18'inki: `main`'deki `glance.py k52 --charts` / `k52chart.mjs`, aynı dünya
(`qp demo --synthetic --days 2600 --history 2500`, SYN02, M18'in `states.json`'u ve hikâye
listesi), aynı yakınlaştırma kuralıyla 5 / 50 / 200 hikâyeye, 5 s oynatma + 6 s ±40 px sürükleme.
Dört yapı ayrı ayrı `git archive` kopyasından derlendi (`qp` release + `scripts/demo.sh`
paketi, `CARGO_TARGET_DIR=target/wt/m18-pp-<ad>`): `pavka-old` = d62207a (M18'de ölçülen),
`pavka-new` = 0ba7dde (bu dal), `korda` = `r5/m18-korda` bc8fc37, `r4` = `main` 717c7f2
(hikâye çizelgesi yok). JIT kapalı (`phone`) 5 tekrar, iki geçiş (ikincisi ters sırada, yük
kaymasını ayırmak için); JIT açık tahmini (`phone-jit-est`) 3 tekrar (M18'in ilk koşusundaki
gibi). Makine başka ajanlarla paylaşılıyordu; yük her yapının penceresinde kaydedildi
(`load.log`, `load2.log`, `loads.py`). Ham sonuçlar `pavka-perf/results/`, günlükler
`pavka-perf/measure*.log`, tablo `pavka-perf/table.py`.

### JIT kapalı (`phone`): sürüklemede uzun kare payı (K52: ≤ %10)

| yapı | geçiş | yük (1 dk, ortanca) | uzun kare payı 5 / 50 / 200 (ortanca) | 50 hikâyede tek tek | 200 hikâyede tek tek | ≤ %10 |
|---|---|---:|---|---|---|---|
| `pavka-old` | 1 | 12,4 | 0 / **0,131** / 0,081 | 0,175, 0,154, 0,131, 0,115, 0,097 | 0,104, 0,097, 0,081, 0,069, 0,064 | hayır (50) |
| `pavka-old` | 2 | 8,2 | 0 / **0,129** / 0,088 | 0,172, 0,149, 0,129, 0,118, 0,10 | 0,098, 0,096, 0,088, 0,083, 0,081 | hayır (50) |
| `pavka-new` | 1 | 9,9 | 0 / 0,058 / 0,064 | 0,046, 0,076, 0,09, 0,058, 0,053 | 0,072, 0,086, 0,055, 0,064, 0,053 | evet |
| `pavka-new` | 2 | 14,3 | 0 / 0,073 / 0,068 | 0,115, 0,058, 0,054, 0,081, 0,073 | 0,086, 0,095, 0,053, 0,068, 0,054 | evet |
| `korda` | 1 | 9,2 | 0 / 0 / 0 | 0 × 5 | 0 × 5 | evet |
| `korda` | 2 | 6,2 | 0 / 0 / 0 | 0 × 5 | 0, 0, 0,006, 0, 0 | evet |
| `r4` | 1 | 4,7 | 0 / 0 / 0 | 0 × 5 | 0, 0, 0,003, 0,003, 0 | evet |
| `r4` | 2 | 7,2 | 0 / 0 / 0 | 0, 0, 0, 0,003, 0 | 0 × 5 | evet |

(M18'in yeniden koşusu, aynı yöntem, başka oturum: `pavka` 0 / 0,135 / 0,086; `korda` 0 / 0 / 0;
`r4` 0 / 0 / 0.) Tek tek değerlerde `pavka-new` 50 hikâyede 10 koşunun 1'inde %10'un üstünde
(0,115, 2. geçiş); `pavka-old` 10 koşunun 9'unda.

### JIT kapalı (`phone`): süreler (ms, ortanca; 5 / 50 / 200)

| yapı | geçiş | görünen | tik p95 | sahne boyama p95 (sürükleme) | sahne kurma p95 | boyama geçişi p95 | sürüklemede kare p99 | `PHONE_BIAS` ile sahne p95 |
|---|---|---|---|---|---|---|---|---|
| `pavka-old` | 1 | 6 / 55 / 208 | 1 / 1 / 1 | 11 / 18 / 23 | 9 / 12 / 15 | 13 / 20 / 25 | 20 / 33 / 39 | 20,4 / 33,3 / 42,6 |
| `pavka-old` | 2 | 6 / 55 / 208 | 1 / 1 / 1 | 11 / 18 / 22 | 9 / 11 / 14 | 13 / 20 / 24 | 20 / 32 / 37 | 20,4 / 33,3 / 40,7 |
| `pavka-new` | 1 | 6 / 55 / 208 | 1 / 1 / 1 | 4 / 13 / 16 | 3 / 6 / 8 | 7 / 15 / 19 | 19 / 27 / 31 | 7,4 / 24,1 / 29,6 |
| `pavka-new` | 2 | 6 / 55 / 208 | 1 / 1 / 1 | 4 / 13 / 16 | 3 / 6 / 9 | 7 / 15 / 19 | 19 / 28 / 32 | 7,4 / 24,1 / 29,6 |
| `korda` | 1 | 6 / 55 / 206 | 1 / 1 / 1 | 5 / 12 / 12 | 3 / 5 / 6 | 8 / 15 / 15 | 19 / 21 / 20 | 9,3 / 22,2 / 22,2 |
| `korda` | 2 | 6 / 55 / 206 | 1 / 1 / 1 | 5 / 12 / 12 | 3 / 6 / 6 | 8 / 15 / 15 | 19 / 21 / 22 | 9,3 / 22,2 / 22,2 |
| `r4` | 1 | 6 / 55 / 208 | 1 / 1 / 1 | 3 / 9 / 10 | 2 / 4 / 5 | 7 / 12 / 13 | 19 / 19 / 18 | 5,6 / 16,7 / 19,4 |
| `r4` | 2 | 6 / 55 / 206 | 1 / 1 / 1 | 4 / 9 / 10 | 3 / 4 / 5 | 7 / 12 / 13 | 19 / 19 / 19 | 7,4 / 16,7 / 18,5 |

Öteki alanlar (sürüklemede boyanan sahne sayısı `scene_drag_n`, koşu koşu sahne p95, adımlar)
`results/*.json`'da.

### JIT açık tahmini (`phone-jit-est`, 3 tekrar; K52'nin bağlayıcı koşulu)

| yapı | yük | görünen | tik p95 | sahne boyama p95 | sürüklemede kare p99 | uzun kare payı | geçti |
|---|---:|---|---|---|---|---|---|
| `pavka-old` | 4,6 | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 2 / 2 | 18 / 19 / 18 | 0 / 0 / 0 | evet |
| `pavka-new` | 3,6 | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 1 / 1 | 19 / 19 / 18 | 0 / 0 / 0 | evet |
| `korda` | 6,5 | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 1 / 1 | 19 / 19 / 18 | 0 / 0 / 0 | evet |
| `r4` | 8,4 | 6 / 52 / 219 | 1 / 1 / 1 | 1 / 1 / 2 | 18 / 19 / 18 | 0 / 0 / 0 | evet |

## Yeniden üretim

`pavka-perf/build.sh <ağaç> <ad>` (her yapı), `pavka-perf/prep.py` (M18'in `states.json`'u +
hikâye listesi, `designs.json`), `pavka-perf/measure.sh` (geçiş 1 + JIT açık),
`pavka-perf/measure2.sh` (geçiş 2, ters sıra), `python3 pavka-perf/table.py run-phone:phone ...`.
Betikler geçici klasör yollarıyla yazıldı (ölçüm `rounds/R5/experiments/harness/glance`'in
`main` kopyasından koşuldu). Mikro ölçüm: `microbench/microbench.rs` (`cdylib`, `qp-grammar`'a
bağlı; `run`, `ff`, `ff2`, `ff3` dışa aktarımları) ve `microbench/wkbench.mjs <wasm> 0 <ad>`.
