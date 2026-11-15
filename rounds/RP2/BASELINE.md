# RP2/T1 — Ölçüm tabanı: bugünkü referans otorite

Bu belge, RP2'de artımlı canlı yol yazılmadan önce bugünkü referans otoritenin
(`qp_session::Session`, kod değiştirilmeden) ne kadar tuttuğunu kaydeder. RP2 kabul
ölçütü 4'ün "önce" sütunu budur.

- Kod: `rounds/RP2/experiments/tick-bench/` (bağımsız crate, çalışma alanı üyesi değil;
  `crates/*`'e yol bağımlılığı, `crates/` değiştirilmedi).
- Yeniden üretim: `rounds/RP2/experiments/tick-bench/run.sh` (ham çıktılar
  `rounds/RP2/experiments/raw/t1/` altına yazılır; yaklaşık 4 dakika).
- Bütün sayılar bu belgedeki iki koşudan (`*-run1.txt`, `*-run2.txt`) alındı.

## 1. Yöntem

**Doküman.** `fixtures/protocol/docs/thyao.json` (px + `rsi14` + `macd` + `bb20`, bir
olgu, bir bulgu, canlı kapsül `cap-overbought`). px, T6d ile aynı üreteç ve tohumla
(SplitMix64 `0x5444_3648_414e_444f`, 2 ondalık) 10 000 bara uzatıldı. Hesap serisi
sayısı toplam **5 / 50 / 500**: fikstürdeki 3 seri + `s000…` diye üretilen seriler
(sma/ema/rsi/macd/bollinger sırayla, dönemleri `i`'ye göre deterministik değişen;
`extra_study` fonksiyonu). Eklenen seriler yalnızca seri; görünüme katman eklenmedi.
Doküman özetleri: N=5 `bccc66367ce7ba32`, N=50 `ecc8cf08797779e9`, N=500 `25748b43d13caf64`.

**Tik akışı.** T6d ile aynı üreteç (tohum `0xFEED`): her benzetilmiş saniyede 1
`append_bars` + 59 `update_last_bar`. **600 tik** (10 yeni bar + 590 düzeltme). Her
tik, `feed` rolündeki bağlantıdan ayrı bir `command` mesajı (159 B) olarak
`Session::handle_raw` ile verilir; JSON ayrıştırma ölçüme dahildir. Giden satırların
serileştirmesi dahil değildir (ayrıca ölçüldü, §4.3).

**İzleyici düzenleri.** Oturumun tik maliyeti, kaç hesabın izlendiğine bağlıdır: oturum
hesapları tembel hesaplar; tikte yalnızca bir koşulun okuduğu seriler (önbellek
temizlendiği için) baştan hesaplanır. Bu yüzden üç düzen ölçüldü:

| Düzen | İçerik | Tikte baştan hesaplanan |
|---|---|---|
| `t6d` | canlı kapsül (geçersizlik: `rsi14` 70'i aşağı keser) + 1 `changes` aboneliği | `rsi14` |
| `base` (görevdeki senaryo) | `t6d` + 1 `condition` aboneliği (`macd` 0'ı yukarı keser) | `rsi14`, `macd` |
| `all` | `base` + her hesap serisine bir `condition` aboneliği (`gt 1e12`, hiç tetiklenmez; bağlantı başına 32 sınırı yüzünden `viewer` bağlantılarına dağıtıldı) | **bütün N seri** |

`all`, oturumun çekirdekle "aynı işi" (her tikte N hesabın güncel değeri) yaptığı
düzendir. `base` ve `t6d`'de oturum çekirdekten *daha az* hesap yapar ama her birini
O(n) baştan yapar.

**Çekirdek tek başına (payda).** Aynı N hesap `qp_compute::create` ile kurulur,
`Study::extend` ile 10 000 barlık geçmişe bağlanır; aynı 600 tikte yeni barda `push`,
düzeltmede `update_last`; çıktılar sütunlarda tutulur. Oturum yok, ayrıştırma yok.
Tik başına süre, döngünün toplam süresinin tik sayısına bölümüdür (7 döngünün medyanı;
tik başına zamanlayıcı ek yükü yok). Ayrıca: tik başına zamanlanmış dağılım, JSON
ayrıştırmalı hali ve "her tikte N hesabı toplu `compute_bars` ile baştan hesapla"
(`core_batch_all`). Akışlı son değerlerin toplu hesapla bit düzeyinde eşit olduğu
her N'de doğrulandı (`stream_eq_batch = true`; 10 / 100 / 1 000 çıktı).

**Anlık görüntü ve hazır olma.** `{"kind":"document"}` sorgusunun `result` mesajı,
ham ve gzip (seviye 6, T6d gibi). Hazır olma = anlık görüntüyü ayrıştır +
`Session::new` + 2 `hello` (okuyucu `arkin`/phone ve `feed`). T6d'den farklı olarak
`Study::extend` dahil değildir (T6d'de 0,13 ms'ydi). 15 tekrarın medyanı.

**Döküm.** (a) Hedefli zamanlayıcılar: oturumun tikte attığı adımlar, açık API'lerle
tek tek taklit edilip zamanlandı (ayrıştırma `Value` + `serde_path_to_error` ile tipli;
`Document::clone`; `apply_change`; eski dokümanın düşürülmesi; izlenen iki hesabın
`compute_bars` ile yeniden hesabı). (b) Örnekleyici profil: macOS `sample` (1 ms),
`tick-bench profile N 30000` döngüsünde 4 s; çağrı ağacı `sample-summary.py` ile
iç içe sayım olmadan toplandı.

**Seçicili komut.** `set_encoding` RSI katmanına iki biçimde: kimlikle
(`view:"main", layer:"l2"`) ve seçiciyle (`view:{}`, `layer:{"shows":{"study":"rsi"}}`),
`arkin` (owner), 60'ar kez dönüşümlü (genişlik değişerek, her biri uygulanır).

**Tekrarlar.** `run.sh` her yapılandırmayı iki kez koşar. Tablolarda iki koşunun
ortalaması verilir; tek tek değerler ham dosyalarda.

## 2. Ortam

Apple M4 Max, Darwin 27.0.0, rustc 1.92.0, Node v24.3.0 (`node:wasi`, preview1).
Yerel: `release` (thin LTO, cgu 1). WASM (`wasm32-wasip1`, Node altında) iki profil:
çalışma alanının `wasm` profili (opt `s`, fat LTO) ve `release` (opt 3). WASM derleme +
örnekleme 4–11 ms. İkili boyutları (`wasm-sizes.txt`: opt-s 3,8 MB, release 21,9 MB
satır tablosu hata ayıklama bilgisiyle) bench ikilisidir, motorun gömme boyutu değildir.

## 3. Tik maliyeti

### 3.1 Oturum ve çekirdek (µs/tik, p50; iki koşunun ortalaması)

| N | Hedef | Oturum `t6d` | Oturum `base` | Oturum `all` | Çekirdek | Çekirdek + ayrıştırma |
|---:|---|---:|---:|---:|---:|---:|
| 5 | yerel | 87,0 | 188,9 | 416,9 | 0,041 | 0,76 |
| 5 | WASM opt-s | 188,2 | 343,4 | 759,4 | 0,103 | 1,05 |
| 5 | WASM release | 130,9 | 282,1 | 673,7 | 0,090 | 1,90 |
| 50 | yerel | 92,3 | 196,6 | 4 033 | 0,262 | 0,92 |
| 50 | WASM opt-s | 191,8 | 352,5 | 7 267 | 0,687 | 1,67 |
| 50 | WASM release | 136,8 | 293,1 | 6 652 | 0,549 | 1,40 |
| 500 | yerel | 156,0 | 259,7 | 40 218 | 2,917 | 3,72 |
| 500 | WASM opt-s | 288,8 | 450,1 | 73 202 | 6,989 | 8,36 |
| 500 | WASM release | 211,4 | 369,4 | 66 384 | 5,630 | 6,79 |

(Çekirdek: döngü medyanı; "çekirdek + ayrıştırma": tik başına zamanlanmış ortalama.)

Ortalama ve p99 (yerel, `base`, run1 / run2):

| N | ortalama | p99 |
|---:|---|---|
| 5 | 192,4 / 191,4 | 222,1 / 220,2 |
| 50 | 219,0 / 192,6 | 238,3 / 221,3 |
| 500 | 331,7 / 259,9 | 1 265 / 314,5 |

`base` p99 WASM opt-s: 394 / 394 (N=5), 387 / 373 (N=50), 5 591 / 460 (N=500).
Bütün tablolar ham dosyalarda (`n{N}.session_*.per_tick_us`).

### 3.2 Oran: oturum / çekirdek (aynı akış, aynı N)

| N | Hedef | `t6d` | `base` | `all` |
|---:|---|---:|---:|---:|
| 5 | yerel | 2 097× | 4 551× | 10 045× |
| 5 | WASM opt-s | 1 827× | 3 334× | 7 373× |
| 5 | WASM release | 1 446× | 3 118× | 7 444× |
| 50 | yerel | 353× | 752× | 15 424× |
| 50 | WASM opt-s | 279× | 513× | 10 578× |
| 50 | WASM release | 249× | 534× | 12 116× |
| 500 | yerel | 53× | 89× | 13 788× |
| 500 | WASM opt-s | 41× | 64× | 10 474× |
| 500 | WASM release | 38× | 66× | 11 791× |

Hedef "oturumun eklediği yük ≤ 3 × çekirdek", yani oturum/çekirdek ≤ 4 (yükü
oturum − çekirdek diye okursak) ya da ≤ 3. Bugün en iyi durumda (N=500, `t6d`) 38×,
çekirdekle aynı işi yaptığında (`all`) 7 000–15 000×.

`all` düzeninde oturumun tik maliyeti, çekirdeğin her tikte N hesabı toplu baştan
hesaplamasıyla (`core_batch_all`) aynıdır: yerel 419,6 / 3 939 / 39 442 µs ortalama
(N=5/50/500) — yani `all`'da süre neredeyse tamamen O(n) yeniden hesaptır.

### 3.3 Tekrarlanabilirlik

- p50'ler iki koşu arasında her yapılandırmada %±20 içinde: oturumda en büyük fark
  %5,5 (yerel N=50 `base`, 202,0 / 191,2), çekirdek döngü medyanında %12 (yerel N=5,
  0,039 / 0,044 µs).
- Ortalamalar ve p99 tek tek sıçramalardan etkileniyor: yerel N=50 `t6d` ortalaması
  run1'de 160,4, run2'de 92,4 (p50 93,4 / 91,2); yerel N=500 `base` 331,7 / 259,9;
  WASM opt-s N=500 `base` 706,9 / 443,3. Bunlar kuyrukta birkaç yüz tikte milisaniyelik
  duraksamalar (ölçüm makinesinde başka iş); karşılaştırmada p50 kullanılmalı.
- Doküman özetleri yerel, WASM opt-s ve WASM release'te ve iki koşuda aynı:
  N=5 `3d554391f7003300`, N=50 `69226187c0ddd11f`, N=500 `db50769bc4152be4`
  (akış sonu; üç izleyici düzeni de aynı dokümanı üretir).
- Kapsülün geçersizlik koşulu ve `macd` koşulu bu akışta hiç tetiklenmedi (motor
  girdisi 0; giden satır 1 204 = 600 ack + 600 change + 4 diğer, `t6d`'de 1 200).

## 4. Zaman nereye gidiyor (oturum tiki)

### 4.1 Hedefli zamanlayıcılar (µs/tik ortalama, `base` adımlarının taklidi; run2)

| Adım | N=5 yerel | N=50 yerel | N=500 yerel | N=500 WASM opt-s | N=500 WASM release |
|---|---:|---:|---:|---:|---:|
| Ayrıştırma (`Value` + tipli) | 1,47 | 1,25 | 1,28 | 2,25 | 2,25 |
| `Document::clone` | 10,7 | 13,5 | 46,0 | 97,5 | 62,0 |
| `apply_change` | 0,16 | 0,10 | 0,09 | 0,19 | 0,23 |
| Eski dokümanı düşürme | 0,75 | 2,82 | 34,8 | 38,2 | 31,9 |
| `rsi14` yeniden hesap (10 000 bar) | 72,8 | 70,2 | 72,3 | 135,3 | 106,3 |
| `macd` yeniden hesap (10 000 bar) | 104,2 | 100,8 | 169,6* | 160,8 | 159,3 |
| Toplam | 190,0 | 188,6 | 324,0 | 434,3 | 362,0 |
| Ölçülen oturum `base` ortalaması | 191,4 | 192,6 | 259,9 | 443,3 | 369,2 |

\* run2'deki bu değer bir sıçrama içeriyor (run1: 98,5; run1 toplamı 249,4). Tersine
N=5'te run1'de `rsi14` 238,5 sıçramalı; o sütunda run2 değerleri verildi.

Okuma:

- **Yeniden hesap baskın:** N=5/50'de tikin ~%90'ı iki izlenen hesabın 10 000 bar
  üzerinde baştan hesaplanması (`rsi14` ~70 µs, `macd` ~100 µs yerel).
- **Doküman kopyası + düşürme N ile büyür:** 11 µs (N=5) → 16 µs (N=50) → 81 µs
  (N=500, kopya 46 + düşürme 35). 10 000 bar tek bir `Vec<Bar>` kopyası olduğu için
  ucuz; pahalı olan seri başına küçük ayırmalar (`instrument`, `params` BTreeMap'leri,
  dizgeler).
- **JSON ayrıştırma küçük:** tik başına ~1,3 µs yerel, ~2,3 µs WASM.
- Giden satırları serileştirme (taşıma tarafı, ölçüme dahil değil): ~0,5 µs yerel,
  ~1,3–3 µs WASM; tik başına 299 B.

### 4.2 Örnekleyici profil (yerel, `base`, `handle_raw` içindeki pay)

| Kalem | N=5 | N=500 |
|---|---:|---:|
| `settle` (izleyiciler, geçersizlik, olaylar) | %86,7 | %60,3 |
| — bunun içinde `compute_bars` | %86,2 | %60,0 |
| `expand_ops` (doküman kopyası dahil) | %9,8 | %23,4 |
| — `Document::clone` | %9,0 | %22,8 |
| `commit` (eski dokümanın düşürülmesi) | %0,7 | %14,5 |
| `check_limits` | %0,8 | %0,6 |
| JSON ayrıştırma (satır içi) | < %0,5 | < %0,5 |
| `malloc`/`free` (her yerde, kesişen) | %4,4 | %22,9 |

Profil döngüsünün kendi ölçtüğü ortalama: N=5 177,8 µs, N=500 238,5 µs (30 000 tik).
`invalidations` ve `entry_events` satır içine alınmış olduğundan ayrı görünmüyor
(`settle` − `eval_watchers` ≈ geçersizlik koşulunun `rsi14` hesabı). Ham:
`sample-n5.txt`, `sample-n500.txt`, özetleri `*.summary.txt`.

### 4.3 Tik başına kaç kopya

Kodla doğrulandı (`crates/qp-session/src`): feed tiki `resolve_selectors`'tan erken
çıkar (feed okuyamaz, seçici yok → kopya yok), `expand_ops` dokümanı **bir kez** kopyalar
(`expand.rs` `let mut scratch = doc.clone()`), `commit` eskisini düşürür ve tikte veri
değiştiği için hesap önbelleğini tamamen temizler (`cache.clear()`). Geçersizlik
tetiklenirse `engine_entry` bir kopya daha yapar (bu akışta olmadı).

## 5. Seçicili komut (`set_encoding`, RSI katmanı)

| N | Hedef | kimlikle p50 | seçiciyle p50 | fark | bir doküman kopyası |
|---:|---|---:|---:|---:|---:|
| 5 | yerel | 16,8 | 38,7 | 21,9 | 10,8 |
| 50 | yerel | 22,3 | 71,3 | 49,0 | 13,6 |
| 500 | yerel | 86,8 | 423,4 | 336,7 | 47,3 |
| 5 | WASM opt-s | 54,5 | 118,1 | 63,7 | — |
| 500 | WASM opt-s | 156,9 | 711,7 | 554,9 | — |
| 500 | WASM release | 110,6 | 599,6 | 489,0 | — |

(µs; "bir doküman kopyası" aynı oturumun dokümanının kopyalanma medyanı, yerel run1.)

- **Evet, seçicili komut dokümanı kopyalıyor.** Kodda: `resolve_selectors` komut için
  `let mut scratch = self.doc.clone()` yapar (session.rs), sonra komutun kendisi
  `expand_ops`'ta bir kopya daha alır. Tek işlemli seçicili komut = **2 kopya**, kimlikli
  = 1. Çok işlemli komutta seçiciden önceki her işlem için ayrıca `scratch.clone()`
  (`next`) yapılır.
- Ölçülen fark, bir kopyadan büyük: N=5'te ~1 kopya + ~11 µs çözümleme; N=500'de
  ~47 µs kopya + ~290 µs çözümleme. Çözümleme seri sayısıyla büyüyor (seçici her seriyi
  ve katmanı tarar). Çözüm `{"/ops/0/layer":"l2","/ops/0/view":"main"}` her N'de doğru.

## 6. Anlık görüntü ve hazır olma

### 6.1 Boyut (`result{document}`, 10 000 bar)

| N | Ham | gzip (6) | Oran | `max_message_bytes` (1 048 576) |
|---:|---:|---:|---:|---|
| 5 | 750 180 B | 210 753 B | %28,1 | sığıyor |
| 50 | 758 579 B | 211 110 B | %27,8 | sığıyor |
| 500 | 842 603 B | 214 091 B | %25,4 | sığıyor |
| T6d (3 hesap) | 749 600 B | 210 638 B | — | sığıyor |

Hesap serisi başına ~187 B ham, ~7 B gzip; anlık görüntü hâlâ ~%99 bar. Sunucuda
sorgu + serileştirme yerel 0,86–0,99 ms, gzip 19,7–21 ms (bir koşuda N=50 98 ms,
sıçrama).

### 6.2 Cihazda hazır olma (ms, medyan, iki koşunun ortalaması)

| N | Hedef | Toplam | ayrıştırma | `Session::new` | `hello` okuyucu | `hello` feed |
|---:|---|---:|---:|---:|---:|---:|
| 5 | yerel | 3,01 | 2,42 | 0,23 | 0,29 | 0,002 |
| 5 | WASM opt-s | 4,50 | 3,40 | 0,46 | 0,47 | — |
| 5 | WASM release | 3,86 | 2,92 | 0,31 | 0,44 | — |
| 50 | yerel | 8,31 | 2,42 | 0,32 | 5,47 | — |
| 50 | WASM opt-s | 12,95 | 3,42 | 0,50 | 8,70 | — |
| 50 | WASM release | 11,89 | 2,96 | 0,41 | 8,34 | — |
| 500 | yerel | 55,16 | 2,65 | 0,52 | 51,66 | — |
| 500 | WASM opt-s | 90,26 | 3,80 | 0,83 | 86,04 | — |
| 500 | WASM release | 85,89 | 3,41 | 0,71 | 82,49 | — |

(Toplam ayrı bir ölçümdür: ayrıştırma + `Session::new` + 2 `hello`; bölünmüş sütunlar
ayrı tekrarlardan.)

- N=5'te T6d ile uyumlu: yerel 3,0 ms (T6d 2,57 ms, `extend` dahil), WASM opt-s 4,5 ms
  (T6d 3,81 ms), WASM release 3,9 ms (T6d 3,23 ms). Fark büyük ölçüde okuyucu `hello`'su:
  R2'den beri `welcome.summary` seçici ipuçlarını da hesaplıyor.
- **Okuyucu `hello`'su hesap serisi sayısıyla doğrusal büyüyor** (~0,1 ms/seri yerel):
  N=500'de hazır olmanın %94'ü. Koddan çıkarım (profillenmedi): `summarize_doc` →
  `select_hints` listelenen her seri için (en çok 50) bir seçici ipucu üretir ve her aday
  seçiciyi bütün dokümana karşı çözer → 50 × N eşleştirme.

## 7. RD1/T6d ile karşılaştırma

| | T6d (3 hesap) | Bugün |
|---|---:|---:|
| Referans `Session` tik p50, yerel | 84 µs | 87,0 µs (`t6d`, N=5) |
| aynı, WASM release | 123 µs | 130,9 µs |
| aynı, WASM opt-s | 183 µs | 188,2 µs |
| Anlık görüntü ham / gzip | 749 600 / 210 638 B | 750 180 / 210 753 B (N=5) |
| Hazır olma yerel / WASM release | 2,57 / 3,23 ms | 3,01 / 3,86 ms (N=5, `extend` hariç) |

Aynı izleyici düzeninde (`t6d`) referans oturum R2'den sonra da aynı yerde (+%4–6).
Görevdeki senaryoya (`base`: ikinci bir koşul aboneliği) geçince tik iki katına çıkıyor,
çünkü her izlenen hesap tikte O(n) baştan hesaplanıyor.

## 8. RP2 hedefi için sonuçlar (ne gitmeli)

1. **Tikte tam yeniden hesap gitmeli.** Tikin %60–90'ı (`base`), `all`'da ~%100'ü
   izlenen hesapların 10 000 bar üzerinde `compute_bars` ile baştan hesaplanması.
   `commit`'in veri değişince bütün önbelleği silmesi bunun sebebi. Akışlı durum
   (`push`/`update_last`) zaten çekirdekte var ve bit-eşit (§1).
2. **Tik başına doküman kopyası ve düşürme gitmeli.** N=500'de 81 µs (yerel), WASM opt-s
   136 µs; bu tek başına çekirdeğin (2,9 µs) 28×'i. Seri sayısıyla büyüyor.
3. **İzleyici değerlendirmesi ucuzlamalı ama kaybolmamalı.** `eval_watchers` kendisi ucuz;
   pahalı olan çağırdığı `values()`. Akışlı çıktıların son iki değerinden koşul okumak
   (T6d `Live`) yeterli.
4. **JSON ayrıştırma payda için önemli.** Tik mesajını ayrıştırmak ~0,7–1,3 µs yerel
   (WASM ~1–2 µs). N=5'te bu çekirdeğin (0,04 µs) 17–30 katı: **hedef ayrıştırma dahil
   oturum ile ayrıştırmasız çekirdek arasında kurulursa küçük N'de tutturulamaz**, oturum
   hiç iş yapmasa bile. N=500'de ayrıştırma çekirdeğin ~%30'u, hedef ulaşılabilir:
   yerel ≤ ~8,8–11,7 µs, WASM opt-s ≤ ~21–28 µs (oranın 3 ya da 4 okunmasına göre).
5. Ayrıca (tik dışı ama ölçülen): seçicili komutta fazladan kopya ve seri sayısıyla büyüyen
   çözümleme (N=500'de +337 µs); okuyucu `hello`'sunda seri sayısıyla doğrusal
   `welcome.summary` (N=500'de 52 ms). İkisi de R2 kalanlarıyla (kabul ölçütü 10) ilgili.

## 9. Açık sorular

- Hedefin paydası: "çekirdeğin aynı işi tek başına yapması" ayrıştırmayı içeriyor mu?
  İçermiyorsa N=5'te bütçe ~0,12–0,16 µs; tek bir tik mesajını ayrıştırmak bile ~1 µs.
  Öneri: payda "çekirdek + aynı mesajın ayrıştırılması" (`core_with_parse`, bu
  belgede ölçüldü: yerel 0,76 / 0,92 / 3,72 µs) ya da hedef yalnız N ≥ 50 için.
- "Aynı iş" `all` gibi her hesabın her tikte güncel olması mı, yoksa yalnız izlenen/görünen
  hesaplar mı? Bugün oturum tembel (yalnız izleneni hesaplar); artımlı yolda bütün hesapları
  ilerletmek çekirdek maliyetini N ile doğrusal yapar (N=500'de 2,9 µs).

## 10. Sınırlar

- Tek makine (M4 Max); GitHub koşucusunda ölçülmedi. WASM, Node/V8 altında `node:wasi`;
  tarayıcı ve telefon değil.
- 600 tik (10 yeni bar); ortalamalar ve p99 tek tük duraksamalara duyarlı, p50 kararlı.
  Koşular sırasında makinede başka iş vardı (kuyrukta ms'lik sıçramalar).
- Döküm §4.1 oturumun içi değil, aynı adımların açık API'lerle taklididir (toplam,
  ölçülen oturum süresiyle ±%5 içinde, sıçramalı koşular hariç). §4.2 örnekleyici
  profili yalnız yerelde ve iki N için; WASM için profil yok.
- Okuyucu `hello` maliyetinin sebebi (§6.2) profillenmedi; kod okumasından çıkarım.
- Kapsül geçersizliği ve koşul aboneliği bu akışta tetiklenmedi; motor girdisi yolu
  (ek doküman kopyası) ölçülmedi.
- Eklenen hesap serileri görünümde katman değildir; katman sayısının etkisi (seçici
  çözümleme, `welcome.summary`) ölçülmedi.
- Doküman 10 000 barda; ölçek (50 000 bar) bu turda tekrar ölçülmedi (T6d: tik 397 µs).
