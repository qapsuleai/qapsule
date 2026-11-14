# T6d — Doküman devri (hand-over) mi, sunucu otoritesi mi? (K21-3)

Sınanan çalışma kararı (K21-3): "motor hem sunucuda hem cihazda çalışır, doküman
devredilebilir". Deneme kodu: `experiments/t6d-handover/` (çalışma alanı üyesi değil).
Ham çıktılar: `rounds/RD1/experiments/raw/t6d/`.

## 1. Hipotez (ölçümden önce yazıldı)

Senaryo: 10 000 barlık bir kapsül dokümanı (px + RSI(14) + MACD + Bollinger + bir olgu,
bir bulgu, canlı bir kapsül) sunucu otoritesinden cihaza devredilir; cihaz onu canlı tutar
(saniyede 1 yeni bar, saniyede 60 son-bar güncellemesi).

Alternatifler: **A** otorite devri (anlık görüntü + seq, cihazda `epoch+1` ile yeni
`Session`, hesaplar cihazda `Study::extend` ile geçmişe bağlanır); **B** sunucu otorite
kalır, cihaz ince ayna (anlık görüntü bir kez, sonra `change` olayları; hesap değerleri
sunucudan); **C** varyantlar (anlık görüntü + günlük; hesap çıktılarını anlık görüntüde
taşımak; aynada yerel hesap).

Beklentiler:

- **H1 (boyut):** 10 000 bar satır biçiminde JSON ≈ 0,7–0,8 MB ham, gzip ≈ 0,25–0,3 MB.
  Sütunlu biçim gzip'te belirgin (%20–40) küçülür. Hesap çıktılarını taşımak (7 sütun ×
  10 000) boyutu kabaca iki katına çıkarır; hesaplamaktan pahalıdır, taşınmamalı.
  Anlık görüntü `max_message_bytes` (1 MiB) sınırına yakın; 10 000+ bar sınırı aşabilir.
- **H2 (hazır olma):** cihazda ayrıştırma + `Session` kurma + üç hesabı `extend` ile
  bağlama yerelde < 30 ms, WASM'da (Node) < 100 ms. En büyük kalem JSON ayrıştırma.
- **H3 (canlı, A):** artımlı yol (akışlı `push`/`update_last` + koşul değerlendirme)
  tik başına < 10 µs. Referans `Session` ise her komutta dokümanı kopyalar ve hesap
  önbelleğini temizleyip toplu yeniden hesaplar: tik başına O(n), ~0,5–2 ms. 60/s'te
  dizüstünde sığar ama telefon sınıfında (K17, R4) israftır; cihaz otoritesi için
  artımlı bir yol gerekir.
- **H4 (canlı, B):** v0 protokolünde değer itme konusu yok; ayna her tikte `study`
  sorgusu yapmak zorunda. Tik başına ~0,3 KB `change` + ~0,5 KB sorgu/sonuç →
  ~40–60 KB/s ve sunucuda cihaz başına O(n) iş. "Değer itme" ya da aynada yerel hesap
  (belirleyicilik sayesinde) bunu ~15 KB/s'ye ve O(1)'e indirir.
- **H5 (çevrimdışı/geri dönüş):** A çevrimdışı çalışmaya devam eder ama canlı veri de
  ağdan geldiği için "canlı" kısmı zaten durur; 60 s canlı tik günlüğü (3 600 girdi)
  ≈ 1–2 MB, anlık görüntüden büyük. Sunucuda yeniden oynatma < 50 ms. §8.4'teki epoch
  kuralı soy karışıklığını *tespit eder* ama sunucu tarafında bu arada gelen veriyi
  *birleştirmez*; ayrıca devir sırasında doküman diğer istemciler (ajanlar) için
  erişilemez olur.
- **H6 (belirleyicilik):** cihaz ve sunucu aynı girdilerle bayt düzeyinde aynı dokümana
  ve bit düzeyinde aynı hesap değerlerine ulaşır; yerel ve WASM özetleri eşittir.

Ön tahmin: K21-3 "inceltilmeli": varsayılan sunucu otoritesi + cihazda yerel hesap
(ayna), tam devir yalnızca tek kullanıcılı/çevrimdışı oturumlar için.

## 2. Yöntem

- **Kod:** `experiments/t6d-handover/src/main.rs` (tek ikili; yerel ve `wasm32-wasip1`).
  Yeniden üretmek için `experiments/t6d-handover/run.sh` (ham çıktıları
  `raw/t6d/` altına yazar). Bağımlılıklar: `qp-schema`, `qp-session`, `qp-compute`
  (yol ile, değiştirilmeden), `serde_json`, `flate2` (gzip, seviye 6).
- **Doküman:** `fixtures/protocol/docs/thyao.json` (px + `rsi14` + `macd` + `bb20`, bir
  olgu, bir bulgu `fnd-overbought`, canlı kapsül `cap-overbought`); px, sabit tohumlu
  (SplitMix64, `0x5444_3648_414e_444f`) rastgele yürüyüşle 10 000 bara uzatıldı, fiyatlar
  2 ondalık. Özet `doc.digest = bac5776765c047fb` (yerel ve WASM aynı).
- **Canlı akış:** 60 s × 60 tik/s = 3 600 tik; her saniye 1 `append_bars` + 59
  `update_last_bar` (tohum `0xFEED`). Her tik ayrı bir `command` (feed rolü).
- **A (devir):** sunucu `Session` (epoch 0) `document` sorgusuyla anlık görüntüyü verir;
  cihaz onu ayrıştırır, `Session::new(epoch 1)` kurar, üç hesabı `Study::extend` ile
  geçmişe bağlar. Canlı veri iki yolla uygulandı:
  (a) referans `Session::handle_raw` (bugünkü kod);
  (b) "A'nın en iyi hali": artımlı cihaz otoritesi (`Live`, deneme kodunda): değişim
  `apply_change` ile yerinde, günlük girdisi referansla aynı, hesaplar
  `push`/`update_last` ile O(1), canlı kapsülün geçersizlik koşulu akışlı çıktılar
  üzerinde. (b)'nin referansla **aynı doküman baytlarını ve aynı günlüğü** ürettiği
  özetle doğrulandı.
- **B (ayna):** sunucu otorite kalır; cihaz anlık görüntüyü bir kez alır, `changes`
  aboneliğinin `change` olaylarını kendi kopyasına `apply_change` ile uygular. qp/0'da
  değer itme konusu olmadığı için hesap değerleri her tikte üç `study` sorgusuyla
  (`range: {last: 1}`) alınır. Ek olarak ölçüldü: **B-push** (qp/0'da olmayan, sıkı bir
  `values` olayı, sunucuda artımlı motor) ve **B+** (ayna + cihazda yerel hesap;
  belirleyicilik sayesinde sunucu değer göndermez).
- **C:** (1) hesap çıktılarını anlık görüntüde taşımak; (2) anlık görüntü + günlük
  (geri devirde ölçüldü: günlük mü, son anlık görüntü mü gönderilir).
- **Çevrimdışı/geri dönüş:** cihaz 20 düzenleme komutu yazar (10 `add_study`,
  5+1 `set_params`, 4 `remove`), isteğe bağlı olarak aynı 60 s canlı akışla. Geri
  devirde sunucu cihaz günlüğünü `qp_session::replay` ile handover anlık görüntüsüne
  uygular. Ayrıca: sunucunun bu arada biriktirdiği akışın yeniden uygulanması; kural
  çiğnenip sunucu yazmaya devam ederse çatallanma; B'de çevrimdışı komutların `base` ile
  kuyruklanması (§8.2).
- **Süreler:** her ölçüm `reps = 15` (geri oynatma 7) tekrar, medyan; tik başına
  ölçümlerde ortalama/p50/p99. Yerel iki koşu (`native-run1/2.txt`) ±%20 içinde
  (yalnızca referans `Session` tik ortalaması run1'de kuyruk gürültüsüyle 110,8 µs,
  run2'de 84,2 µs; p50 ikisinde 84 µs).
- **Ortam:** Apple M4 Max, Darwin 27.0, rustc 1.92.0, Node v24.3.0 (`node:wasi`,
  preview1). WASM iki profille: `release` (opt 3) ve çalışma alanının `wasm` profili
  (opt `s`, fat LTO). Ağ simüle edilmedi; aktarım süreleri bant genişliğinden hesaplandı.

## 3. Sonuçlar

### 3.1 Aktarım boyutu (10 000 bar, sunucudan cihaza)

| Biçim | Ham | gzip | Not |
|---|---:|---:|---|
| `result{document}` mesajı, satır bar JSON (bugünkü) | 749 600 B | 210 638 B | `max_message_bytes` = 1 048 576 → sığıyor |
| — bunun barsız kısmı (görünüm, olgu, bulgu, kapsül) | 2 459 B | 928 B | doküman = %99,7 bar |
| Sütunlu bar (`t,o,h,l,c,v` dizileri) | 489 554 B | 170 289 B | gzip −%19 |
| Sütunlu + `t` farkları (Δt) | 439 602 B | 136 241 B | gzip −%35 |
| C1: hesap çıktıları da taşınırsa (7+1 sütun × 10 000) | +1 942 518 B | +768 955 B | anlık görüntünün gzip'te 3,65 katı |

- 50 000 barda mesaj 3 836 801 B (gzip 1,11 MB) → **1 MiB sınırını aşıyor**; satır
  JSON'da sınır ≈ 14 000 barda aşılır. `put_series`/`append_bars` ayrıca
  `max_bars = 10 000` sınırına takılır. Yani bugünkü qp/0'da büyük bir dokümanın
  anlık görüntüsü tek mesajla taşınamaz (parçalı/akışlı anlık görüntü gerekir).
- Ağ maliyeti hesaplamayı ezer: 210 KB gzip 10 Mbit/s'te ≈ 170 ms, 50 Mbit/s'te
  ≈ 34 ms; aşağıdaki tüm işlemci maliyetleri 3–4 ms.

### 3.2 Sunucuda serileştirme, cihazda hazır olma

| Adım (medyan, ms) | Yerel aarch64 | WASM release | WASM opt-s |
|---|---:|---:|---:|
| Sunucu: `document` sorgusu + JSON | 0,84 | 1,69 | 3,13 |
| Sunucu: gzip (seviye 6) | 19,4 | 31,1 | 32,7 |
| Cihaz: anlık görüntüyü ayrıştır | 2,31 | 2,86 | 3,37 |
| Cihaz: `Session::new` (epoch 1) + 2 `hello` | 0,12 | 0,24 | 0,26 |
| Cihaz: 3 hesabı `Study::extend` ile bağla | 0,13 | 0,12 | 0,15 |
| Cihaz: 3 hesabı tüm çıktılarıyla hesapla (`run`) | 0,25 | 0,42 | 0,45 |
| C1: taşınan hesap çıktılarını ayrıştır | 1,98 | 2,93 | 2,98 |
| **A toplam (ayrıştır + oturum + extend)** | **2,57** | **3,23** | **3,81** |
| **B toplam (yalnızca ayrıştır)** | 2,31 | 2,86 | 3,37 |

- A ile B'nin hazır olma farkı ≈ 0,3–0,5 ms; ikisinde de maliyetin %90'ı JSON ayrıştırma.
- C1 (çıktıları taşımak) **hem daha büyük hem daha yavaş**: çıktıları ayrıştırmak
  (1,98 ms) onları hesaplamaktan (0,25 ms) 8 kat pahalı; üstelik akışlı durum yine
  `extend` ister. Hesap çıktıları anlık görüntüye konmamalı.
- 50 000 barda A toplamı 12,5 ms (yerel); doğrusal.

### 3.3 Canlı tutma (3 600 tik, 60/s)

| Tik başına maliyet | Yerel | WASM release | WASM opt-s |
|---|---:|---:|---:|
| A(a) referans `Session` (p50) | 84 µs | 123 µs | 183 µs |
| A(a) — 60/s'te bir çekirdeğin payı | %0,5–0,7 | %0,76 | %1,1 |
| A(b) artımlı (ayrıştırmasız) | 0,18 µs | 0,23 µs | 0,27 µs |
| A(b) artımlı + mesaj ayrıştırma (p50 / p99) | 0,75 / 0,96 µs | 1,13 / 2,50 µs | 1,17 / 2,21 µs |
| B cihaz: `change` ayrıştır + uygula + 3 sonuç ayrıştır | 4,0 µs | 6,5 µs | 7,2 µs |
| B sunucu: akış (doküman başına, referans) | 86 µs | 131 µs | 189 µs |
| B sunucu: 3 `study` sorgusu (**cihaz başına**) | 214 µs | 362 µs | 367 µs |
| B+ sunucu: cihaz başına ek iş (bildirimi serileştir) | 0,24 µs | 0,38 µs | 0,74 µs |

- Referans `Session` tik başına **O(n)**: 10 000 barda 84 µs, 50 000 barda 397 µs
  (her komutta doküman kopyası + hesap önbelleği temizlenip RSI'nin toplu yeniden
  hesabı). Artımlı yol 50 000 barda da 0,31 µs. Hız farkı 460–700×.
- Dizüstünde 60/s'te referans yol bile çekirdeğin %1'inden az; ama B'de sorgular cihaz
  başına O(n) olduğu için sunucu cihaz başına çekirdeğin %1,8'ini (50 000 barda %8,6)
  harcar → ~55 cihaz/çekirdek. B+ ile cihaz başı sunucu işi 0,24 µs/tik.

| Ağ (cihaz yönü) | Bayt/tik | Bayt/s |
|---|---:|---:|
| A: akış verisi cihaza (`command`, feed) | 159 | 9 540 |
| B: `change` olayları | 249 | 14 945 |
| B v0: + 3 `study` sonucu (aşağı) | +672 | 55 236 aşağı |
| B v0: 3 `study` sorgusu (yukarı) | 272 | 16 320 yukarı |
| B-push (varsayımsal `values` olayı) | +544 | 47 556 aşağı |
| B+ (ayna + yerel hesap) | 249 | 14 945 |

- A da ağsız canlı kalamaz: akış verisi cihaza bir yerden gelmek zorunda. A ile B+
  arasındaki ağ farkı ≈ 5 KB/s (159'a karşı 249 B/tik); B v0 ise 3,7 kat fazla.
- Varsayımsal değer itme bile (544 B/tik) v0 sorgusundan çok az kazanç sağlıyor:
  JSON'da 8 değerin adı ve zarfı veriden büyük. Asıl kazanç cihazda hesaplamakta.

### 3.4 Çevrimdışı ve geri dönüş

**A (cihaz otorite, epoch 1), çevrimdışı:** cihaz yazmaya, sorgulamaya, hesaplamaya
devam eder (ölçüldü: 20 düzenleme + isteğe bağlı yerel akış hatasız uygulandı). Ancak
gerçek kullanımda akış verisi ağdan geldiğinden "canlı" kısım çevrimdışında zaten durur;
kalan kazanç düzenleme ve okuma.

**B (ayna), çevrimdışı:** cihaz donar: yazamaz, değer alamaz (v0'da değerler sunucudan).
B+ ise elindeki son veriyle hesap ve okuma yapabilir; yazılar `base` ile kuyruklanır
(§8.2).

Geri devir (cihaz → sunucu) boyutları ve süreleri (yerel; WASM değerleri `raw/t6d/`):

| Gönderilen | Girdi | Ham | gzip | Sunucuda | Özet eşit |
|---|---:|---:|---:|---|---|
| Günlük, yalnızca düzenleme | 20 | 12 571 B | 1 070 B | ayrıştır 0,04 + `replay` 0,015 ms | evet |
| Günlük, düzenleme + 60 s yerel akış | 3 620 | 1 759 367 B | 95 204 B | ayrıştır 4,9 + `replay` 0,17 ms | evet |
| — sıkıştırılmış günlük (aynı bar için ara `set_last_bar`'lar atıldı) | 140 | 66 357 B | 5 695 B | — | evet |
| Son anlık görüntü (karşılaştırma) | — | 755 496 B | 211 925 B | ayrıştır ≈ 2,3 ms | — |

- 60 s'lik canlı günlük ham olarak anlık görüntünün 2,3 katı ama gzip'te 0,45 katı
  (girdiler çok benzer). Ara tikleri atmak günlüğü 26 kat küçültür, belge baytları
  değişmez; ancak denetim izi ve `inverse` zinciri kaybolur, `seq` yeniden numaralanır.
- `replay` hesap çalıştırmadığı ve doküman kopyalamadığı için çok ucuz (3 620 girdi
  0,17 ms). Aynı akışı referans `Session` üzerinden komut olarak yeniden uygulamak
  305 ms sürer (tik başına O(n)).

**Sunucu bu arada akışı biriktirdiyse** (§8.4'e göre yazamaz) geri devirden sonra yeni
otorite (epoch 2) biriktirilenleri uygular:

| Durum | Saf yeniden uygulama | `t` ile tekilleştirme |
|---|---|---|
| Cihazın çevrimdışında akışı yoktu | 3 600 tik, 0 hata, 305 ms | 0 atlandı, 0 hata |
| Cihaz aynı kaynaktan akışı yerelde aldı | 3 541 hata (`invalid`: bar zamanı geride/uyuşmuyor) | 3 541 atlandı, 0 hata |

**Kural çiğnenirse (çatallanma):** sunucu epoch 0'da akışı ve bir editörün
`set_params macd` yazısını almaya devam ederken cihaz epoch 1'de düzenleme + akış
yazdı. Doküman özetleri ayrıştı (`7ef1…` / `01db…`); iki soyun birden dokunduğu
kimlikler `["macd", "px"]`. §8.4'te birleştirme yok; hangi doküman kazanırsa öbür
taraftaki bu iki öğenin değişikliği sessizce kaybolur.

**B'de çevrimdışı kuyruk (`base`, §8.2):** sunucu bu arada 3 600 akış girdisi ve
editörün `macd` değişikliğini aldı; cihaz 20 komutu dönüşte gönderdi.

| Kuyruk biçimi | Uygulanan | `conflict` | Diğer hata |
|---|---:|---:|---:|
| Hepsi tek `base` (kopuş anındaki seq) | 1 | 11 | 8 (`not_found`, zincirleme) |
| Öğe başına `base` (cihaz kendi ack'lerinin seq'ini kullanır) | 19 | 1 (gerçek çakışma: `macd`) | 0 |

- Tek `base` ile cihaz **kendi** önceki kuyruk yazısıyla çakışıyor (ilk `add_study`
  `main` görünümüne dokunuyor, sonraki her `add_study` onunla çakışıyor). Öğe başına
  `base` ile yalnızca gerçek çakışma kalıyor ve çözümü §8.2'deki gibi istemciye düşüyor.
  Bu, A'nın devriyle elde edilen çevrimdışı yazma yeteneğinin devir olmadan da
  elde edilebildiğini gösteriyor; üstelik doküman bu sürede diğer istemcilere
  (ajanlara) açık kalıyor.

### 3.5 Belirleyicilik

| Karşılaştırma | Sonuç |
|---|---|
| A(a) referans cihaz vs A(b) artımlı cihaz: doküman baytları | eşit (`82c59895ce97b470`) |
| A(a) vs A(b): günlük baytları | eşit (`717b9f1a76c97b67`) |
| B sunucu vs B aynası vs A cihazı: doküman | üçü eşit (`82c59895ce97b470`) |
| Akışlı hesap (push/update_last, 3 600 tik) vs toplu `compute_bars`, 10 060 bar × 8 çıktı | bit düzeyinde eşit (`af779fd08f6f97a4`) |
| B'de sunucudan gelen son değerler vs A cihazının değerleri | 8/8 bit eşit |
| Geri devirde `replay` vs cihaz dokümanı (tam ve sıkıştırılmış günlük) | eşit |
| Yerel aarch64 vs WASM release vs WASM opt-s: yukarıdaki bütün özetler | aynı |

Devir için gereken ön koşul (iki tarafın aynı girdilerle aynı baytlara ulaşması) tam
olarak sağlanıyor; bu, B+ (aynada yerel hesap) için de aynı derecede geçerli.

## 4. §8.4 epoch kuralı yeterli mi?

Kural: eski otorite anlık görüntü + `seq` verir ve yazmayı durdurur; yeni otorite
`epoch + 1` ile başlar; bağlantılar kapanır, istemciler `hello` ile yeniden bağlanır;
`base` bağlantının kendi epoch'una aittir.

Yeterli olduğu yer: normal akışta iki yazarı önler ve `base`'in hangi soya ait olduğunu
belirsizlikten kurtarır.

Yetersiz kaldığı yerler (ölçümle görülen):

1. **Devir sırasında gelen veri.** Eski otorite yazamaz; akış ve ajan yazıları ya yeni
   otoriteye yönlendirilmeli ya da biriktirilmeli. Biriktirme ve geri devirde yeniden
   uygulama tanımlı değil; saf uygulama aynı kaynaklı veride 3 541 hata verdi, `t` ile
   tekilleştirme hatasız. Kural şartnamede yok.
2. **Erişilebilirlik.** Cihaz otoriteyken çevrimdışı olursa doküman herkes için
   yazılamaz (sunucudaki ajanlar dahil). Kayıp/kapalı cihazda doküman kilitli kalır.
   Kiralama (lease) ve süre sonunda sunucunun geri alması (epoch + 2) yok; geri alınırsa
   cihazın o arada yazdıkları çatallanmış olur ve bunun ne olacağı tanımsız.
3. **Çit (fencing) yok.** Eski otoritenin yazmayı gerçekten durdurduğunu kanıtlayan bir
   mekanizma yok; çatallanma deneyinde iki soy aynı öğelere (`macd`, `px`) dokundu ve
   epoch bunu yalnızca *tespit eder*, birleştirmez.
4. **Günlük sürekliliği.** Yeni otoritenin günlüğü `seq = 1`'den başlıyor (referans
   `Session::new`); geri devirde sunucu cihaz günlüğünü kendi günlüğüne eklerken
   yeniden numaralamalı. Konum aslında `(epoch, seq)`; bu şartnamede açık değil.
   `NOTES.md` #33'teki "başka epoch'tan `base`" denetimi de komutta epoch olmadığı için
   uygulanamıyor (yeniden `hello` zorunluluğu bunu dolaylı çözüyor).
5. **Mesaj sınırı.** Anlık görüntü tek `result` mesajıdır; ≈ 14 000 barın üstünde
   `max_message_bytes`'ı aşar. Parçalı anlık görüntü tanımı yok.

## 5. Hipotezlerin değerlendirmesi

| | Beklenti | Ölçüm | Sonuç |
|---|---|---|---|
| H1 | 0,7–0,8 MB / gzip 0,25–0,3 MB; sütunlu −%20–40; çıktılar ~2× | 0,75 MB / 0,21 MB; −%19 / −%35 (Δt); çıktılar gzip'te 3,65× | doğrulandı (çıktılar beklenenden de pahalı) |
| H2 | yerel < 30 ms, WASM < 100 ms | 2,6 ms / 3,2–3,8 ms | doğrulandı, beklenenden ~10× iyi |
| H3 | artımlı < 10 µs; referans 0,5–2 ms, O(n) | 0,18 µs; referans 84 µs, O(n) doğrulandı (50k'da 397 µs) | kısmen: referans tahminimden 10× hızlı ama O(n) |
| H4 | v0 40–60 KB/s, sunucu O(n)/cihaz; push/yerel ~15 KB/s | v0 55 KB/s aşağı + 16 KB/s yukarı; push 48 KB/s; yerel hesap 15 KB/s | push için yanlış; yerel hesap için doğru |
| H5 | canlı günlük 1–2 MB, replay < 50 ms, epoch kuralı yetersiz | 1,76 MB ham ama gzip 95 KB; replay 0,17 ms; 5 eksik | doğrulandı |
| H6 | bit/bayt eşitliği, yerel = WASM | bütün özetler eşit | doğrulandı |

## 6. Sonuç

- **"Motor hem sunucuda hem cihazda çalışır" doğrulandı.** Doküman ve hesap değerleri
  sunucuda, cihazda ve WASM'da bayt/bit düzeyinde aynı; cihazda hazır olma 3–4 ms,
  canlı tutma artımlı yolla tik başına < 2 µs. Bunu yapmanın maliyeti ağdır
  (anlık görüntü 136–211 KB gzip), işlemci değil.
- **"Doküman devredilebilir" teknik olarak çalışıyor ama ölçülen kazancı dar.**
  Devrin sağladığı şeylerin çoğu devirsiz de elde ediliyor:
  - canlı görüntü ve yerel hesap → B+ (ayna + yerel hesap): ağda A'dan yalnızca
    ~5 KB/s fazla, sunucuda cihaz başına 0,24 µs/tik;
  - çevrimdışı yazma → §8.2 `base` kuyruğu (öğe başına `base` ile 19/20 uygulandı,
    1 gerçek çakışma).
  Devrin bedeli ise ölçüldü: devir boyunca doküman diğer yazarlara kapalı, sunucuda
  biriken veri için tanımsız yeniden uygulama, kural çiğnenirse sessiz kayıp, büyük
  dokümanda mesaj sınırı.
- **B v0 (değerleri sorgulayan ince ayna) en kötü seçenek:** 3,7× ağ ve sunucuda cihaz
  başına O(n) iş. Değer itme (`values` konusu) bunu az iyileştiriyor; asıl çözüm aynada
  yerel hesap.
- **Referans `Session` canlı veri için O(n):** hangi alternatif seçilirse seçilsin,
  otoritede artımlı bir canlı yol gerekir (deneme bunun mümkün ve referansla bayt-eşit
  olduğunu gösterdi). Sunucu otoritesinde bu, cihaz sayısıyla çarpıldığı için daha da
  önemli.

## 7. K21-3 için öneri: **incelt**

Önerilen yeni ifade:

> Motor hem sunucuda hem cihazda çalışır ve aynı girdilerle bayt/bit düzeyinde aynı
> sonucu verir. **Varsayılan:** otorite sunucuda kalır; cihaz aynadır ve hesapları
> yerelde yapar (değer taşınmaz, yalnızca `change` olayları). Çevrimdışı yazılar §8.2
> `base` kuyruğuyla gönderilir. **Otorite devri** yalnızca dokümanın başka yazarı ya da
> canlı ajanı yokken, açıkça istenen bir "çevrimdışına al" ya da yalnızca yerel doküman
> için yapılır; süreli kira (lease) ile, geri devir günlükle (ara tikler sıkıştırılarak)
> ve sunucuda biriken akış `t` ile tekilleştirilerek yeniden uygulanır.

Bu ayrım, (2) "belge başına tek otorite" kararını korur ve devri istisnai, sınırları
belirli bir işleme çevirir.

## 8. Sınırlar

- Gerçek telefon ölçülmedi. Bütün süreler Apple M4 Max'te; WASM, Node/V8 altında
  `node:wasi` ile koşuldu (tarayıcı değil; iOS JavaScriptCore değil). K17 telefon
  sınıfı R4'e kalıyor. Kaba bir 5–10× telefon çarpanı bile hazır olmayı ≈ 40 ms'nin,
  artımlı tiki ≈ 20 µs'nin altında tutar; referans `Session` tiki ise ≈ 1–2 ms'ye çıkar.
- Ağ simüle edilmedi (gecikme, kayıp, yeniden bağlanma süresi yok); aktarım süresi bant
  genişliğinden hesaplandı.
- Veri sentetik (rastgele yürüyüş, 2 ondalık); gzip oranları gerçek veride değişebilir.
  Tek bar serisi, tek doküman, tek cihaz; çok cihazlı sunucu yükü cihaz başı maliyetten
  çıkarıldı, ölçülmedi.
- Kapsülün geçersizlik koşulu her tikte değerlendirildi ama koşul hiç doğru olmadı
  (`engine_entries = 0`); motorun kendi yazdığı girdi yolu canlı ölçümde çalışmadı.
- Artımlı cihaz otoritesi (`Live`) deneme kodudur: yalnızca bar tiklerini ve
  `threshold`/`compare` koşullarını kapsar; niyetler, koşul abonelikleri ve öneriler
  yok. Referansla eşitliği yalnızca bu senaryoda doğrulandı.
- Öğe başına `base` kuyruğu ve `t` ile tekilleştirme istemci tarafı deneme
  kurallarıdır; şartnamede değiller.

## 9. Öneriler (kapsam dışı; yapılmadı)

- `qp-session`'a artımlı canlı yol: `append_bars`/`update_last_bar` için doküman
  kopyasız uygulama ve akışlı hesap önbelleği (deneme 460–700× hız, aynı baytlar).
- §8.4'e: kira/süre ve geri alma, devir sırasında gelen veri için yönlendirme ya da
  biriktirme + `t` ile tekilleştirme, `(epoch, seq)` günlük konumu, parçalı anlık
  görüntü (ya da `max_message_bytes` dışında bir anlık görüntü kanalı).
- §8.2'ye: istemcinin kendi önceki yazılarıyla çakışmaması için öğe başına `base`
  (ya da bağlantının kendi girdilerini `base` denetiminden hariç tutma) açıkça yazılsın.
- Anlık görüntüde bar verisi için sütunlu + Δt biçimi (gzip −%35) kodlama deneyiyle
  birlikte değerlendirilsin; hesap çıktıları anlık görüntüye konmasın.
- Ekransız/ince yüzeyler (saat) için gerekiyorsa sıkı bir `values` aboneliği; ekranlı
  cihazlarda yerel hesap tercih edilsin.

## Ham çıktılar

`raw/t6d/`: `env.txt`, `native-run1.txt`, `native-run2.txt`, `wasm-release.txt`,
`wasm-opt-s.txt` (+ `*.stderr.txt`: WASM derleme+örnekleme ≈ 4,5 ms), `wasm-sizes.txt`
(release 3,99 MB, opt-s 2,60 MB; deneme ikilisi, motorun gömme boyutu değil),
`native-50k-bars.txt` (ölçek denemesi).
