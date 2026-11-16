# R4 — Cihaz ölçümü (GOAL ölçüt 9)

Görev D, dal `r4/d`. Ölçüm ekranının denetimi ve eklenenler, masaüstü ve kısıtlanmış işlemci
ölçümü, Arkin için telefon yönergesi, K17 (telefon sınıfı hedefi) ve K45 (GPU) önerileri.
Ham çıktılar ve yöntem ayrıntısı: `rounds/R4/experiments/D.md`, sentetik medyanlar
`rounds/R4/experiments/D/device-synthetic.json`.

## 1. Ölçüm ekranı (Ölçüm sekmesi)

Başsız Chromium'da denetlendi (`shells/web/tests/device.mjs` her koşunun sonunda Ölçüm
sekmesini açar, **Özeti kopyala**'ya bir kez tıklar ve metin kutusundaki özetin çekirdeğin
JSON'u olduğunu doğrular: 72/72 koşuda geçti).

| Ne | T5'te vardı | D'de eklendi |
|----|-------------|--------------|
| Kare süresi | durağan, kaydırırken, sürüklerken: n, ort., p50, p95, en çok | **yakınlaştırırken** (`zoom`: tekerlek ve kıstırma); **p99**; **uzun kare oranı** (16,7 ve 33,3 ms'yi aşan aralık payı); 1 sn'ye kadar aralıklar sayılır (önce 250 ms üstü atılıyordu; doymuş ana iş parçacığı görünmüyordu), daha uzunları ayrı **boşluk** sayısı |
| Tik maliyeti | veri çerçevesi → yansı + yeniden boyama | p99 |
| Yüz / sahne boyama | var (derleme + piksel + `putImageData`) | p99 |
| Açılış | WASM hazır, start çerçevesi, bağlandı, akış, ilk kapsül | — |
| Paket boyutu | `pkg/sizes.json` | — |
| Bellek | yok | **JS yığını** (yalnız Chromium `performance.memory`), **WASM belleği** (çekirdek kendisi okur); son ve en çok, saniyede bir örnek |
| Cihaz bilgisi | yalnız tarayıcı kimliği | **çekirdek sayısı, cihaz belleği (Chromium), ekran, DPR, çekirdeğin boyama ölçeği**; özette kullanılan görünüm (yüz, akış, geçiş, sonuç işareti) |
| Denetim | Özeti kopyala | **Sıfırla** (kare ve süre serileri baştan; açılış, cihaz, bellek kalır); sayfada telefon için tek satır yönerge; sekme açıkken satırlar saniyede bir yenilenir |

Mimari: istatistik, biçim ve bellek okuması çekirdekte (`crates/qp-wasm/src/shell.rs`);
kabuk ham değerleri verir (`metric_device`, `metric_memory`, `metric_reset`; mimari testi
0 ihlal, izin listesi `shells/web/ARCH.md`). Test: `crates/qp-wasm/tests/metrics_d.rs`.

## 2. Ölçüm

**Makine:** Apple M4 Max (16 çekirdek), 128 GB, macOS 27.0.1. Yük ortalaması koşular
boyunca 5,0–7,2 (paralel ajanlar; 16 çekirdekte tarayıcıya boş çekirdek kalıyordu).
**Tarayıcı:** başsız Chromium 151.0.7922.34 (Playwright 1.62), Node 24.3. 390×844 CSS px.
**Paket:** `scripts/demo.sh`'ın derlediği yansılı paket (adsız): WASM ham 4.377.208 B,
gzip −9 1.288.290 B; JS ham 84.321 B, gzip 15.940 B. Gösterim barındırıcısı sıkıştırmaz:
ağdan ham 4,4 MB gider.

**Düzenek** (`node shells/web/tests/device.mjs <qp> [-- --data DIR --from T]`): belirleyici
sıra görünüm → DPR → kısıt; her yapılandırma 3 koşu, her koşu taze `qp demo` ve taze tarayıcı
bağlamı, **medyan**. Koşu: açılış → oynatma 16 adım/sn → dört evre, her biri 5 sn, evre
başında seriler sıfırlanır: **durağan** (akış, dokunma yok), **kaydırma** (tekerlekle aşağı
yukarı), **sürükleme** (ilk kapsül grafiğe açılır, yatay sürükleme), **yakınlaştırma**
(grafikte tekerlek). Kısıt `Emulation.setCPUThrottlingRate` 1× / 4× / 6×; DPR 1 ve 3
(çekirdek DPR 3'te 3× ölçekte boyar: 9 kat piksel). Görünüm: varsayılan (`vadrel`, `elikov`,
`zumlak`, `tg5`) ve `r3` (`?face=r3&feed=r3&transition=r3&outcome=r3`). Veri: sentetik
(30 enstrüman, 120 bar) ve yerel gerçek (30 sembol, `--from 2025-06-02`, varsayılan 120 bar
pencere; yalnız süre toplamları).

Başsız Chromium `requestAnimationFrame`'i 120 Hz'de verir (aralık 8,3 ms); "kare/sn" evrede
gelen kare sayısı / 5 sn (120 = hiç kayıp yok). Uzun kare = aralık > 16,7 ms.

### 2.1 Varsayılan görünüm, sentetik

| | 1× DPR1 | 1× DPR3 | 4× DPR1 | **4× DPR3** | 6× DPR1 | **6× DPR3** |
|---|---:|---:|---:|---:|---:|---:|
| İlk kapsül (ms) | 120 | 121 | 401 | **460** | 623 | **639** |
| WASM hazır / bağlandı (ms) | 20 / 41 | 20 / 41 | 68 / 154 | 68 / 150 | 136 / 246 | 121 / 230 |
| Akış durağan: kare/sn · p95 · p99 · uzun % | 120 · 9,8 · 10,2 · 0 | 119 · 9,9 · 10,3 · 0 | 117 · 10,2 · 17,1 · 1 | **110 · 10,2 · 48,1 · 2** | 111 · 10,2 · 33,0 · 2 | **102 · 9,3 · 75,5 · 3** |
| Akış kaydırma: kare/sn · p95 · p99 · uzun % | 120 · 9,7 · 10,2 · 0 | 119 · 9,9 · 10,4 · 0 | 116 · 10,2 · 24,2 · 2 | **107 · 10,2 · 58,6 · 2** | 110 · 10,2 · 48,5 · 2 | **94 · 16,7 · 91,4 · 5** |
| Grafik sürükleme: kare/sn · p95 · p99 · uzun % | 113 · 9,8 · 10,4 · 1 | 111 · 9,9 · 16,6 · 1 | 99 · 10,2 · 65,2 · 2 | **98 · 10,2 · 75,4 · 2** | 93 · 10,2 · 101,6 · 2 | **79 · 17,1 · 125,6 · 10** |
| Grafik yakınlaştırma: kare/sn · p95 · p99 · uzun % | 120 · 9,8 · 10,2 · 0 | 119 · 9,5 · 10,3 · 0 | 113 · 10,2 · 35,0 · 2 | **110 · 10,2 · 49,2 · 2** | 104 · 10,2 · 74,5 · 2 | **95 · 16,7 · 83,4 · 6** |
| Tik, 16 adım/sn: p50 / p95 (ms) | 2,0 / 4,0 | 1,4 / 11,3 | 3,3 / 9,4 | **3,3 / 35,9** | 4,9 / 14,3 | **5,0 / 54,5** |
| Yüz boyama p50 / p95 (ms) | 0,7 / 1,6 | 2,6 / 4,1 | 1,9 / 2,8 | **7,8 / 9,1** | 3,0 / 4,2 | **11,9 / 14,2** |
| Sahne (grafik) boyama p50 / p95 (ms) | 1,1 / 2,0 | 3,4 / 5,7 | 3,0 / 4,0 | **10,5 / 12,2** | 5,0 / 6,2 | **16,0 / 18,1** |
| JS yığını / WASM belleği (MB) | 2,8 / 18,8 | 2,9 / 25,9 | 3,9 / 18,8 | 3,1 / 25,9 | 3,5 / 18,8 | 3,2 / 25,9 |

### 2.2 Varsayılan görünüm, yerel gerçek veri (yalnız süre toplamları)

| | 1× DPR1 | 1× DPR3 | 4× DPR3 | 6× DPR3 |
|---|---:|---:|---:|---:|
| İlk kapsül (ms) | 97 | 98 | 384 | 573 |
| Akış kaydırma: kare/sn · p95 · p99 · uzun % | 120 · 9,8 · 10,2 · 0 | 120 · 9,8 · 10,4 · 0 | 109 · 10,2 · 58,0 · 2 | 94 · 16,8 · 84,2 · 6 |
| Grafik sürükleme: kare/sn · p95 · p99 · uzun % | 112 · 9,7 · 10,3 · 0 | 112 · 9,7 · 10,3 · 0 | 104 · 10,2 · 43,1 · 2 | 89 · 17,1 · 89,9 · 7 |
| Grafik yakınlaştırma: p95 · p99 · uzun % | 9,8 · 10,3 · 0 | 9,9 · 16,6 · 1 | 10,2 · 43,1 · 2 | 16,7 · 84,1 · 5 |
| Tik p50 / p95 (ms) | 1,9 / 4,0 | 1,6 / 12,4 | 3,3 / 32,9 | 4,9 / 50,5 |
| Yüz / sahne boyama p95 (ms) | 1,5 / 1,9 | 4,0 / 5,7 | 9,1 / 11,3 | 13,1 / 17,3 |
| JS yığını / WASM belleği (MB) | 4,0 / 17,0 | 3,5 / 26,3 | 2,9 / 26,3 | 3,0 / 26,3 |

Gerçek veri sentetikle aynı sınıfta (pencere 120 bar; T6'nın tam geçmiş ölçümü ayrı: 1,0 s
/ 4× 3,9 s).

### 2.3 `r3` görünümü ile karşılaştırma (DPR 3)

| | varsayılan 4× | `r3` 4× | varsayılan 6× | `r3` 6× |
|---|---:|---:|---:|---:|
| İlk kapsül (ms), sentetik / gerçek | 460 / 384 | 406 / 383 | 639 / 573 | 619 / 600 |
| Akış durağan kare/sn (sentetik / gerçek) | 110 / 112 | **47 / 44** | 102 / 104 | **1 / 1** |
| Akış kaydırma kare/sn (sentetik / gerçek) | 107 / 109 | **6 / 4** | 94 / 94 | **0 / 0** |
| Tik p50 / p95 (ms), sentetik | 3,3 / 35,9 | **46,4 / 52,3** | 5,0 / 54,5 | **58,9 / 67,8** |
| Yüz boyama p50 (ms), sentetik | 7,8 | 12,8 | 11,9 | 18,8 |
| Yüz boyama sayısı, 5 sn'lik evrede (≈80 tik) | ≈72 | ≈330 | ≈70 | ≈330 |
| Grafik sürükleme / yakınlaştırma kare/sn | 98 / 110 | 98 / 110 | 79 / 95 | 69 / 96 |

`r3` yüzü her tikte değişiyor (≈5 kat daha çok yüz yeniden boyanıyor); DPR 3'te 4× kısıtta
tik (≈46–49 ms) 16 adım/sn'nin 62,5 ms'lik aralığına dayanıyor ve ana iş parçacığı doyuyor:
akış durağanken 44–47, kaydırırken 4–6 kare/sn; 6×'te akış donuyor. DPR 1'de ve 1× kısıtta
`r3` bütçede. Varsayılan yüz (`vadrel`) yalnız değişince boyandığı için aynı koşulda
107–112 kare/sn. Grafik derinliğinde iki görünüm aynı (aynı grafik).

### 2.4 Ne söylüyor

- **Darboğaz grafik çizimi değil, tikte yüzlerin yeniden boyanması.** Uzun kareler (p99) tik
  anlarına denk geliyor: tik p95 4× DPR3'te 33–36 ms, 6×'te 50–55 ms (yeni kapsül gelince
  akış yeniden kuruluyor ve görünen yüzler 3× ölçekte baştan boyanıyor). 16 adım/sn en kötü
  durum; varsayılan hız 4 adım/sn'de tik sayısı dörtte bir.
- **DPR 3 maliyeti** boyamada ≈3–4 kat (9 kat piksel): sahne p95 1× 2,0 → 5,7 ms, 6× 6,2 →
  18,1 ms. Bu, çizimin (tiny-skia rasterleştirme + `putImageData`) payı.
- Grafik sürükleme/yakınlaştırmada kare akışı 4×'te sürüyor (p95 10,2 ms, uzun %2): hareket
  uçuştayken birleştiriliyor, cevap gelince bir kez boyanıyor. 6× DPR3'te uzun kare %6–10.
- Açılış: ilk kapsül 4× 0,38–0,46 s, 6× 0,57–0,64 s (yerel döngü adresinde; telefonda Wi-Fi
  üzerinden 4,4 MB sıkıştırılmamış WASM eklenir, aşağıda).
- Bellek küçük: JS yığını 3–4 MB, WASM 17–26 MB (`r3` DPR3 6× en çok 38 MB).

## 3. Benzetimin sınırı

- `setCPUThrottlingRate` yalnız sayfanın ana iş parçacığını yavaşlatır. **GPU**'yu
  (birleştirme, `putImageData` doku yüklemesi), bellek bant genişliğini, **termal
  kısılmayı** (telefon birkaç dakika sonra yavaşlar), büyük/küçük çekirdek zamanlamasını ve
  WASM derleyicisinin (Liftoff/TurboFan) telefondaki hızını benzetmez.
- **Gerçek dokunma yok:** kaydırma tekerlekle, sürükleme fareyle, yakınlaştırma tekerlekle
  yapıldı; dokunmatik olay hattı, momentum kaydırma ve iki parmak kıstırma ölçülmedi.
- **Ekran tazeleme:** başsız Chromium 120 Hz'de kare veriyor; telefonların çoğu 60 Hz (kimi
  90/120). p95 ~10 ms burada "kayıp yok" demektir; telefonda aynı iş 16,7 ms'lik aralıkta
  ölçülür.
- **Ağ:** ölçüm 127.0.0.1 üzerinden; telefonda Wi-Fi'de 4,4 MB WASM (barındırıcı sıkıştırmıyor)
  ≈0,4–0,9 s ekler (40–80 Mbit/s).
- Makine yüklüydü (yük ortalaması 5–7); kısıt oranı ile telefon sınıfı arasındaki eşleme bir
  varsayımdır (4× ≈ orta sınıf, 6× ≈ eski Android; Chrome DevTools'un "mid-tier/low-tier
  mobile" önayarları da bu aralıkta). **Telefon ölçümü bu eşlemeyi doğrulamak için gerekli.**

## 4. Telefon yönergesi (Arkin için)

### Açmak

1. Bilgisayarda: `scripts/demo.sh --lan --synthetic` (karşılaştırılabilir olsun diye sentetik;
   isterseniz ikinci tur gerçek veriyle: `scripts/demo.sh --lan`). Konsol
   `telefondan aç / open on your phone (same network): http://192.168.x.y:8390/` yazar; aynı
   adres **Ayarlar** sekmesinin altında da görünür.
2. **Belirteç girmeniz gerekmez:** sayfa her bağlantı için gizli değeri kendisi alır. Adresi
   **konsolun yazdığı gibi** (IP ile) açın; başka ad (`.local`, başka IP) barındırıcının Host
   denetimine takılır (403).
3. **Güvenlik duvarı:** macOS "qp gelen bağlantıları kabul etsin mi?" diye sorarsa **İzin
   Ver**. Sormadan bağlanamıyorsanız: Sistem Ayarları → Ağ → Güvenlik Duvarı → Seçenekler →
   `target/release/qp` için "Gelen bağlantılara izin ver". `qp` yeniden derlenince soru
   yinelenebilir.
4. Telefon **aynı Wi-Fi'de** olmalı (misafir ağı ve "istemci yalıtımı" olan ağlar engeller;
   telefonda VPN kapalı). **Uyarı:** `--lan` açıkken aynı ağdaki cihazlar gösterimi görebilir.
5. Telefonda: Android'de **Chrome**, iPhone'da **Safari**. (Safari JS yığınını ve cihaz
   belleğini vermez; özette o satırlar boş kalır, gerisi aynı.)

### Ölçmek (her telefonda, yaklaşık 2 dakika)

1. Telefonu şarja takılı değil ama pili %50'nin üstünde tutun; düşük güç / pil tasarrufu
   **kapalı**. Sayfayı açtıktan sonra 10 sn bekleyin.
2. Alttaki oynatma çubuğunda hızı **16×** seçin ve **▶**'ye dokunun.
3. **Ölçüm** sekmesine geçip **Sıfırla**'ya dokunun.
4. **Akış** sekmesine geçin; **10 sn hiçbir şeye dokunmayın** (durağan).
5. Akışı **10 sn** parmakla aşağı yukarı kaydırın (kaydırma).
6. Bir kapsüle dokunun (açılır), bir daha dokunun (grafik). Grafikte **10 sn** sağa sola
   sürükleyin, sonra **10 sn** iki parmakla kıstırıp açın (yakınlaştırma). **‹ Geri**.
7. **Ölçüm** sekmesine dönün, **Özeti kopyala**'ya dokunun ("Kopyalandı" yazar; pano izni
   yoksa metin kutusunda seçili gelir — elle kopyalayın).
8. Aynı sayfayı `?face=r3&feed=r3&transition=r3&outcome=r3` ekiyle açıp 2–7'yi bir kez daha
   yapın (`r3` karşılaştırması; adresin sonuna ekleyin).
9. İlk açılış için sayfayı sekmeyi kapatıp yeniden açın (yenilemek önbelleği kullanır); ilk
   açılış süresi özetin `open_ms.first_capsule` satırıdır.

### Özeti nereye

Her özeti `rounds/R4/experiments/D/phone-<model>-<görünüm>.json` adıyla kaydedin (ör.
`phone-pixel4a-default.json`) ya da ana akış oturumuna yapıştırın. Özet yalnız süre, bayt,
cihaz bilgisi ve tarayıcı kimliği taşır; piyasa verisi taşımaz. Gerçek veriyle alınan
özetler yine de `fixtures/local/` altına konur (commit edilmez), rapora toplamı girer.

### Hangi telefonlar

- **Kendi telefonunuz** (orta-üst sınıf; referans).
- **Eski bir Android** (zorunlu): 4–6 yaşında, orta ya da alt sınıf (ör. Snapdragon 6xx/7xx
  ya da Helio G serisi, 3–4 GB RAM, 60 Hz). K17'nin taban sınıfı bunun üstüne kurulur.
- Varsa bir **iPhone** (Safari/WebKit farklı WASM motoru) ve bir **120 Hz** ekranlı telefon.
- Her telefonda modeli, ekran tazeleme hızını ve pil durumunu not edin; termal etkiyi görmek
  için bir telefonda ölçümü 5 dakika oynatmadan sonra bir kez daha yapın.

## 5. Öneri — K17 telefon sınıfı hedefi (geçici)

Telefon sınıfı iki basamak; her biri bir kısıt benzetimine karşılık gelir ve **telefonda
doğrulanana kadar geçicidir** (telefon ölçümü gelince hedef telefon sayısıyla yazılır,
benzetim yalnız CI'da gerileme izler). Ölçüm koşulu: 390 CSS px genişlik, DPR 3, sentetik
30 enstrüman, oynatma 16 adım/sn, Ölçüm ekranının sayıları.

| Ölçü | Orta sınıf telefon (≈ 4× DPR3) | Taban: eski Android (≈ 6× DPR3) | 4× ölçülen | 6× ölçülen |
|------|---:|---:|---:|---:|
| İlk kapsül (yerel ağ, 120 bar) | < 1,0 s | < 1,5 s | 0,46 s | 0,64 s |
| Akış kaydırma kare p95 | ≤ 16,7 ms | ≤ 16,7 ms | 10,2 | 16,7 |
| Akış kaydırma uzun kare (>16,7 ms) oranı | ≤ %5 | ≤ %10 | %2 | %5 |
| Grafik sürükleme/yakınlaştırma kare p95 | ≤ 33 ms | ≤ 33 ms | 10,2 | 17,1 |
| Grafik uzun kare oranı | ≤ %5 | ≤ %10 | %2 | %6–10 |
| Tik p95 (16 adım/sn) | ≤ 33 ms | ≤ 62 ms (tik aralığı) | **35,9** | 54,5 |
| Sahne boyama p95 | ≤ 16,7 ms | ≤ 33 ms | 12,2 | 18,1 |
| WASM belleği | ≤ 64 MB | ≤ 64 MB | 26 | 26 |

Varsayılan görünüm hedefleri tutuyor, **tik p95 orta sınıfta sınırda (35,9 > 33)** — ilk iş
tikte yüzlerin yeniden boyanmasının hafifletilmesi (Öneriler). `r3` görünümü DPR 3'te orta
sınıfı tutmuyor (akış 4–47 kare/sn); yalnız seçenek olarak kalır.

## 6. Öneri — K45 GPU kararı

**Ertelenmeye devam.** Gerekçe: grafik derinliğinde çizim (sahne derleme + rasterleştirme +
`putImageData`) orta sınıf benzetiminde p95 12,2 ms — 60 Hz kare bütçesinin (16,7 ms) %73'ü,
ama her karede değil yalnız hareket cevabında çiziliyor; kare akışı p95 10,2 ms, uzun kare %2.
Uzun karelerin kaynağı çizici değil, tikte akış yüzlerinin toplu yeniden boyanması (bkz.
§2.3: aynı grafik iki görünümde aynı, akış ise `r3`'te doyuyor). Taban sınıfta (6× DPR3)
sahne p95 18,1 ms bütçeyi aşıyor; burada GPU'dan önce ucuz CPU seçenekleri var: hareket
sürerken 2× ölçekte boyamak (DPR 3'te piksel 9 → 4 kat; DPR 1→3 farkı ölçümde ≈3 kat
maliyet), değişmeyen yüzü yeniden boyamamak, kirli bölge.

**Tetikleyici** (biri telefonda görülürse GPU yolu açılır, önce yukarıdaki CPU seçenekleri
denenip yeniden ölçülerek): orta sınıf telefonda grafik sürükleme/yakınlaştırma **kare p95
> 33 ms** ya da uzun kare oranı > %10, ya da **sahne boyama p95 > 16,7 ms**; taban sınıfta
sahne boyama p95 > 33 ms.

## 7. Paket ve ayrı yükleme (A13 / Ö26 için girdi)

Sayfanın paketi (yansılı, adsız) gzip 1,29 MB, ham 4,38 MB; barındırıcı sıkıştırmadan ham
gönderiyor. Yerel döngüde WASM indirme açılışın küçük payı (4×'te WASM hazır 68 ms, ilk
kapsül 460 ms), telefonda Wi-Fi'de ham 4,4 MB ≈0,4–0,9 s ekler. Ayrı yükleme (T6 ÖK-T6-4)
yerine önce **barındırıcıda sıkıştırma** (gzip/br önceden sıkıştırılmış dosya, `Accept-Encoding`
ile; yeni bağımlılık istemez: `demo.sh` `.wasm.gz` üretebilir) daha ucuz; telefon ölçümünde
`open_ms.wasm_ready` bunu gösterecek.
