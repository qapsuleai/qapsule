# RP2/T7c — Yaklaşım C: yansı yalnızca kendi verisiyle çalışır, sapmayı sonradan bildirir

Tarih: 2026-11-15. Dal: `rp2/t7c-approach`. Kod:
`rounds/RP2/experiments/harness/src/approaches/c.rs` (mesajlar, otorite, ortak yardımcılar)
ve `.../approaches/c/replica.rs` (yansı). Kayıt: `approaches!(n, c);`. Düzenek
(`HARNESS.md`) değiştirilmedi; ölçümler düzeneğin varsayılan tohumlarıyla (100; S17: 20).
Bu çalışma RESEARCH §3.C'yi savunucusunun yazacağı en iyi haliyle kurar; diğer adayların
(A, B, D) dallarına bakılmadı.

## 1. Özet

- **M1 = M2 = 0 ve M3k (kesin yanlış) = 0 bütün senaryolarda**, 100 tohumda; **M5 (kaçan) = 0
  bütün senaryolarda**, S2c (yansı beslemesi 60 s geride) ve S19 (2 dk kesinti) dahil.
- Kalan tek hata türü **M3g (geçici yanlış)**: canlı bardaki geçici iddianın, yansının beslemesi
  gerçeğin ~80–140 ms gerisinde olduğu için, gösterildiği anda gerçekte birkaç on ms önce
  bozulmuş olması. Toplam 84 olay / 2 320 tohum-koşu (en çok S11: 19, S19: 15, S2a: 10).
  Tolerans 0 ms olduğu için bu yarış canlı geçici gösterimle sıfırlanamaz (§5.2).
- Aynı kod, canlı geçici gösterimi kapatan tek sabitle (`LIVE_PROVISIONAL = false`, değişke
  **C-kesin**) **M1 = M2 = M3 = M5 = M8(adet) = 0, bütün senaryolarda** verir; bedeli canlı bar
  için "geçici" işaretinin olmaması (kesin gösterim gecikmesi değişmez: p50 ≈ 110 ms).
- Protokol baytı N'nin ~¼'ü (S0: 0,78 B/s, N: 3,15 B/s); r→a yalnızca otorite yanılınca
  (S8: 0,22 B/s, S14: 0,03 B/s). Yansının mutlak işi N'ninkine yakın (+%2–6).
- Bedel ürün düzeyinde: **iki kullanıcı farklı şey görebilir** ve görüntü, belgedeki tek
  otoritenin kararından ayrışabilir (§7).

## 2. Tasarım

### 2.1 İlke

Gösterim kararını **yalnızca yansının kendi verisi** verir: kendi beslemesi + kendi
`fetch_settled` (sağlayıcının kesinleşmiş bar kaynağı, REST benzetimi). Ekrandaki her iddia,
yansının motorunun (paylaşılan `Engine`, aynı dedektör) o anki iddia kümesindedir ve yansının
görünen verisi motorun barlarıdır; görüntü her geri çağrıda motor değişikliğiyle birlikte
yeniden etiketlenir. Bu yüzden **erken (M1) ve çelişkili (M2) gösterim yapısal olarak
imkânsızdır** (testle de sınanıyor). Otoritenin iddia akışı yalnızca *sonradan*, uyuşmazlığı
işaretlemek ve bildirmek için kullanılır.

### 2.2 Etiketler (yansı)

| Durum (yansının kendi verisinde) | Etiket |
|---|---|
| Canlı bardaki iddia, besleme taze, önceki barlar onaylı, eşikten pay ≥ δ (ilk gösterimde) | `Provisional` |
| Kesinleşmiş (settled) bardaki iddia, ekranda henüz kapanmamış (besleme geride/kesik) | `Provisional` |
| Kapanmış bar(lar), iddia barına kadar bütün barlar **onaylı** | `Final` |
| Onaylı `Final`, otorite o barı geçti ve iddiayı söylemiyor, fark ≥ 2 s sürdü | `SourceDiffers` (+ `report`) |
| Yansıda yok, otorite (barı geçmiş, yansı onaylı) söylüyor, fark ≥ 2 s | `Disputed` (iddia etmez, + `report`) |
| Kapanmış ama onaysız, daha önce geçici gösterilmiş | `Provisional` ("henüz onaylanmadı") |
| Kapanmış, onaysız, hiç gösterilmemiş | gösterilmez (onay bekler) |

### 2.3 Onay (Final'ın koşulu)

Bir bağ için **onaylı sınır** `conf_t`: bu bara kadar her bar ya

1. **kesinleşmiş bardır** (`fetch_settled` ile alınmış ya da onunla bit bit karşılaştırılıp eşit
   bulunmuştur; kesintisiz önek `set_t`), ya da
2. **besleme kesinliği kanıtlanmış** (aşağıdaki denetim) ve o bara ait olabilecek **hiçbir
   olay eksik değildir**.

Eksik olay izleme: yansı `seq` defterini motorla aynı mantıkla kendisi de tutar ve her
boşluğa "eksik olay hangi barlara ait olabilir" aralığını (`lo` = boşluk anındaki besleme
cephesi, `hi` = boşluğu ortaya çıkaran olayın barı; yeni bar açan `bar` olayıysa bir önceki
bar) iliştirir. Geç gelen (sıra bozulmuş) olay boşluğu kapatır; kesinleşmiş bar o aralığı
kapsayınca boşluk çözülür. Düzeltmeler ve anlık görüntüler kaybolmadığı için cephenin
gerisindeki onaylı bar bir boşlukla geri alınmaz.

RSI özyinelemeli olduğundan **geçici** iddia da önceki kapanmış barlara dayanır: canlı bardaki
iddia ancak `conf_t ≥ bar_t − aralık` iken gösterilir. (İlk ölçümde S6/S8'deki bütün M3g'ler
buradan çıkıyordu: kapanış tikini kaybetmiş bir barın üstündeki canlı bar yanlış kesişim
gösteriyordu; bu kuralla S6/S8 M3 = 0.)

### 2.4 Kesinleşmiş kaynağı yoklama

Yansı her bağ için **her bar bir kez** `fetch_settled(b, set_t + aralık, ∞)` ister. Bir barın ne
zaman çekilebilir olduğunu kendisi öğrenir: kaynak barı kapanıştan sabit bir süre sonra
verdiği için `istek_zamanı − (bar_t + aralık)` her bar ve bağ için aynıdır; isabet bu değerin
üst, ıska alt sınırını verir (bağlar arasında ortak `Tracker`). Sınırlar gevşekken (> 5 s)
250 ms aralıkla yoklar, sonra ikiye bölerek 20 ms'ye sıkıştırır; yakınsayınca her bar tek
istekle, kesinleşir kesinleşmez (kapanış + 2 s + 0,8 s yanıt) gelir. Aynı yoklama dört iş görür:

- **Denetim (audit).** Cephenin gerisinde ve eksiksiz bir besleme barı kesinleşmiş barla
  bit bit karşılaştırılır. İlk eşleşme bağı `Exact` yapar (hızlı yol açılır); herhangi bir
  uyuşmazlık `Inexact` yapar (f32 fiyat — S15/S15k —, bozuk fiyat): o bağda yalnız kesinleşmiş
  barlara güvenilir, onaysız barlar kesinleşmiş değerle değiştirilir, canlı geçici iddiaya ek
  pay (0,05 RSI) istenir. İstekten sonra beslemenin değiştirdiği bar (düzeltme yarışı)
  karşılaştırılmaz ve üstüne yazılmaz.
- **Onay.** Boşluklu barlar kesinleşmiş barla değiştirilip onaylanır (S6, S8: kapanış tiki
  kaybı → Final ≈ kapanış + 2,8 s).
- **İleri bakış.** Besleme kesinleşmiş kaynağın gerisindeyse (S2: 5 s, S2c: 60 s, S16 bağ 1)
  kesinleşmiş barlar beslemenin önüne eklenir; beslemenin bu barlara ait bayat olayları
  atlanır (cephedeki, `set_t` gerisindeki olay = bayat tekrar; cephenin gerisindeki olay =
  düzeltme, uygulanır). Bu durumda bağ "geride" işaretlenir ve beslemenin bayat canlı barındaki
  geçici iddialar gösterilmez.
- **Kesinti.** Besleme sustuğunda (≥ max(5 s, 8 × ortalama olay aralığı)) kesinleşmiş barlar
  gelmeye devam eder; donmuş canlı bardaki geçici iddialar gizlenir. Dönüşteki `REFRESH`
  anlık görüntüsü eldekiyle birleştirilir (aynı veri döneminde kesinleşmiş barlar korunur;
  bölünme gibi farklı dönemde her şey değiştirilir ve yeniden denetlenir).

Başlangıçta yansı, beslemenin ilk anlık görüntüsünü beklemeden otoritenin hello'sundaki çapadan
itibaren kesinleşmiş geçmişi çeker (S2c'de anlık görüntü 60 s geç gelir; bu olmadan geçmiş
iddiaları 217/255 kaçıyordu).

### 2.5 Geçici gösterimin payı

Yansının canlı barı gerçeğin gerisindedir; gösterim anında gerçek iddiayı bozmuş olabilir. Bu
yüzden yeni bir geçici iddia ilk kez ancak değer eşiği **δ** kadar geçmişse gösterilir
(gösterildikten sonra iddia tuttukça kalır):

`δ = 1,5 × ortalama |ΔRSI| (canlı bar olayı başına) × 250 ms / ortalama olay aralığı`
(+ 0,05 `Inexact` bağda; ilk 8 canlı olaydan önce gösterilmez).

Yani "beslemenin 250 ms'lik gecikme bütçesinde RSI eğilimle ne kadar geri gelebilir". Seyrek
tikli piyasada δ çok küçüktür (S0 p50 gecikme 124 ms), hızlı salınımda büyür (S7: 3600 tik/bar,
M3g 7560 → 0, M4 15 833 → 738; bedeli S7'de geçici gecikme p50 9,6 s).

### 2.6 Otorite ve protokol

Otorite paylaşılan motoru kendi beslemesinde çalıştırır ve gönderir (JSON, mesaj numaralı):

```json
{"n":7,"a":{"px0":1699976040000},"w":{"px0":1700000100000},
 "add":["rsi_cross[period=14,level=70]@px0:1700000100000:up"],"del":[]}
```

- `a` (hello, bir kez): bağ başına **hesap başlangıcı** (otoritenin ilk barı). Bağ meta
  verisidir, piyasa verisi değildir. Yansı çapası farklıysa (S12/S12s: son 300/60 bar) eksik
  geçmişi kendi `fetch_settled`'ıyla tamamlar (ya da fazlasını keser); bu yapılana kadar o bağda
  hiçbir şey göstermez.
- `w`: otoritenin son kapanmış barı (kesin iddia kümesi buraya kadar tam).
- `add`/`del`: yalnız **kesin** iddia kümesindeki değişiklik (anlam anahtarı metni). Otoritenin
  geçici iddiaları gönderilmez: C'de yansı onları hiçbir şey için kullanmaz.

Yansı → otorite: `{"type":"report","kind":"divergence","items":[{"claim":…,"local":true,
"authority":false,"sealed_t":…}]}` — her anahtar için durum değiştikçe bir kez. Otorite rapor
alınca o bağın barlarını (son çektiğinden rapordaki bara kadar) kendi kesinleşmiş kaynağından
çekip uygular ("otorite kayda geçer"); kendi verisi düzelirse kesin kümesi değişir, yansıdaki
`SourceDiffers`/`Disputed` kalkar (S8, S14).

Kanal sırasız/yinelenmiş teslim edebilir (S4, S13): yansı mesajları `n` sırasıyla uygular,
yinelemeyi atar. **Kanal piyasa verisi taşımaz (K23):** yalnız bar zamanları (çapa, filigran,
anahtardaki `bar_t`) ve anlam anahtarları.

## 3. Neden C'nin en iyi hali

Savunucunun vaadi "erken = 0, çelişkili = 0, gecikme en düşük, ağ kesintisinde de çalışır".
Saf C (kendi beslemesinde ne bulursa göster) yalnız ilk ikisini tutar; S6/S8'de kendi
kayıplı beslemesine dayanarak **kesin yanlış** gösterir, S2c'de 60 s geç kalır, S15k'de f32 ile
bıçak sırtında yanılır, S12s'de kısa çapayla otoriteden ayrışır. Buradaki sürüm bunların her
birini **kendi verisinden çıkmadan** kapatır:

- kesinleşmiş kaynak yansının *kendi* verisidir (her cihazın REST'e erişimi var); onayla M3k = 0;
- beslemenin kesinliği varsayılmaz, ölçülür (denetim) — f32/bozuk beslemede hızlı yol kendiliğinden
  kapanır;
- geride kalan ya da kesik beslemede kesinleşmiş kaynak ileri bakış/yedek olarak kullanılır —
  M5 = 0;
- otoriteye yalnız gösterilmiş bir şeyin etiketini *sonradan* değiştirmek ve rapor için bakılır;
  2 s'lik tolerans aynı düzeltmenin iki uçta farklı anda gelmesini yutar.

## 4. Sonuçlar

Komutlar (hepsi varsayılan tohumlar):

```bash
cd rounds/RP2/experiments/harness && cargo build --release
./target/release/harness run --approach c --scenario all          # C
# C-kesin: replica.rs içinde LIVE_PROVISIONAL = false ile derlenip aynı komut
./target/release/harness run --approach n --scenario all          # N (karşılaştırma)
```

Kısaltmalar: M3k/M3g = kesin/geçici yanlış; M6 ms (bütün tohumlar havuzlanmış); M7 saniye
(parantezde Δ_stale'i aşan bölüm); M8 adet (parantezde toplam süre, s); M9 = a→r ve r→a ham
bayt/canlı saniye; M10 = yansı iş birimi/olay; M13 saniye. "Hatalı tohum" = M1+M2+M3 > 0
(parantezde en kötü tohum, yeniden oynatma için).

### 4.1 C (varsayılan: canlı geçici gösterim açık)

| Senaryo | Tohum | Hatalı tohum (en kötü) | M1 | M2 | M3k | M3g | M4 | M5 kaçan/uygun | M6g p50/p99 ms | M6k p50/p99 ms | M7 s (>Δ) | M8 (s) | M9 a→r B/s · r→a B/s | M10 | M12 top/en çok | M13 s |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| S0 | 100 | 2 (#46) | 0 | 0 | 0 | 2 | 204 | 0/1202 | 124/31671 | 111/139 | 22 (0) | 0 (823) | 0.78 · 0.00 | 0.8 | 1610/11 | 0 |
| S1 | 100 | 0 | 0 | 0 | 0 | 0 | 204 | 0/1202 | 0/31536 | 0/0 | 0 (0) | 0 (843) | 0.78 · 0.00 | 0.8 | 1610/11 | 0 |
| S2a | 100 | 10 (#17) | 0 | 0 | 0 | 10 | 203 | 0/1202 | 518/32059 | 513/525 | 97 (0) | 0 (400) | 0.78 · 0.00 | 0.8 | 1610/11 | 0 |
| S2 | 100 | 1 (#99) | 0 | 0 | 0 | 2 | 6 | 0/1202 | 34744/63326 | 5134/5248 | 10 (0) | 0 (0) | 0.78 · 0.00 | 0.9 | 1214/6 | 0 |
| S2c | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | 35188/63380 | 61670/62809 | 0 (0) | 0 (0) | 0.78 · 0.00 | 0.4 | 1202/1 | 0 |
| S3 | 100 | 1 (#54) | 0 | 0 | 0 | 1 | 204 | 0/1202 | 35/31575 | 30/40 | 6 (0) | 0 (3672) | 0.78 · 0.00 | 0.8 | 1610/11 | 0 |
| S4 | 100 | 2 (#46) | 0 | 0 | 0 | 2 | 204 | 0/1202 | 124/31671 | 111/139 | 22 (0) | 0 (910) | 0.78 · 0.00 | 0.8 | 1610/11 | 0 |
| S5 | 100 | 8 (#87) | 0 | 0 | 0 | 9 | 362 | 0/1215 | 224/30824 | 120/850 | 62 (0) | 0 (737) | 0.78 · 0.00 | 1.2 | 1941/16 | 0 |
| S6 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3028 | 112/2957 | 2809/3597 | 0 (0) | 0 (845) | 1.08 · 0.00 | 3.9 | 3028/1 | 0 |
| S7 | 100 | 0 | 0 | 0 | 0 | 0 | 738 | 0/1232 | 9639/48908 | 119/140 | 33 (0) | 0 (823) | 2.17 · 0.00 | 2.1 | 2708/13 | 0 |
| S8 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3028 | 113/140 | 120/2901 | 0 (0) | 0 (823) | 1.10 · 0.22 | 2.0 | 3028/1 | 438 |
| S9 | 100 | 4 (#41) | 0 | 0 | 0 | 4 | 123 | 0/3467 | 120/18631 | 108/140 | 39 (0) | 0 (2506) | 1.21 · 0.00 | 3.0 | 4187/9 | 0 |
| S10 | 100 | 3 (#9) | 0 | 0 | 0 | 3 | 241 | 0/1206 | 126/20200 | 114/140 | 26 (0) | 0 (823) | 0.78 · 0.00 | 0.9 | 1688/11 | 0 |
| S11 | 100 | 18 (#54) | 0 | 0 | 0 | 19 | 5831 | 0/2559 | 126/37296 | 111/30109 | 631 (0) | 0 (823) | 1.01 · 0.00 | 1.0 | 14223/9 | 0 |
| S12 | 100 | 0 | 0 | 0 | 0 | 0 | 4527 | 0/9674 | 0/24198 | 0/0 | 0 (0) | 0 (566) | 2.07 · 0.00 | 1.9 | 18728/9 | 0 |
| S12s | 100 | 0 | 0 | 0 | 0 | 0 | 4527 | 0/9674 | 0/24198 | 0/0 | 0 (0) | 0 (65) | 2.07 · 0.00 | 1.9 | 18728/9 | 0 |
| S13 | 100 | 2 (#46) | 0 | 0 | 0 | 2 | 210 | 0/1202 | 120/31671 | 110/140 | 22 (0) | 0 (823) | 0.78 · 0.00 | 0.8 | 1622/11 | 0 |
| S14 | 100 | 2 (#46) | 0 | 0 | 0 | 2 | 204 | 0/1202 | 124/31671 | 111/139 | 22 (0) | 0 (823) | 0.79 · 0.03 | 0.8 | 1610/11 | 61 |
| S15 | 100 | 0 | 0 | 0 | 0 | 0 | 2982 | 0/2559 | 0/31257 | 2802/3586 | 0 (0) | 0 (1002) | 1.01 · 0.00 | 5.6 | 8523/9 | 0 |
| S15k | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/2568 | – | 2802/3586 | 0 (0) | 0 (3009) | 1.01 · 0.00 | 5.6 | 2568/1 | 0 |
| S16 | 100 | 2 (#45) | 0 | 0 | 0 | 2 | 205 | 0/2767 | 32505/64104 | 5075/5247 | 51 (4) | 0 (825) | 1.63 · 0.00 | 1.3 | 3177/7 | 0 |
| S17 | 20 | 5 (#3) | 0 | 0 | 0 | 6 | 379 | 0/8025 | 2127/40120 | 114/140 | 35 (0) | 0 (6208) | 105.62 · 0.00 | 12.4 | 8783/14 | 0 |
| S18 | 100 | 5 (#25) | 0 | 0 | 0 | 5 | 3816 | 0/3097 | 123/26617 | 110/139 | 416 (0) | 0 (1294) | 1.20 · 0.00 | 0.9 | 10731/10 | 0 |
| S19 | 100 | 15 (#7) | 0 | 0 | 0 | 15 | 5688 | 0/2559 | 125/48939 | 112/62813 | 616 (0) | 0 (823) | 1.01 · 0.00 | 1.0 | 13935/9 | 0 |

### 4.2 C-kesin (değişke: canlı geçici gösterim kapalı, `LIVE_PROVISIONAL = false`)

| Senaryo | Tohum | Hatalı tohum (en kötü) | M1 | M2 | M3k | M3g | M4 | M5 kaçan/uygun | M6g p50/p99 ms | M6k p50/p99 ms | M7 s (>Δ) | M8 (s) | M9 a→r B/s · r→a B/s | M10 | M12 top/en çok | M13 s |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| S0 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 111/139 | 0 (0) | 0 (823) | 0.78 · 0.00 | 0.8 | 1202/1 | 0 |
| S1 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 0/0 | 0 (0) | 0 (843) | 0.78 · 0.00 | 0.8 | 1202/1 | 0 |
| S2a | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 513/525 | 0 (0) | 0 (400) | 0.78 · 0.00 | 0.8 | 1202/1 | 0 |
| S2 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | 34746/63326 | 5134/5248 | 0 (0) | 0 (0) | 0.78 · 0.00 | 0.9 | 1202/1 | 0 |
| S2c | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | 35188/63380 | 61670/62809 | 0 (0) | 0 (0) | 0.78 · 0.00 | 0.4 | 1202/1 | 0 |
| S3 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 30/40 | 0 (0) | 0 (3672) | 0.78 · 0.00 | 0.8 | 1202/1 | 0 |
| S4 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 111/139 | 0 (0) | 0 (910) | 0.78 · 0.00 | 0.8 | 1202/1 | 0 |
| S5 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1215 | – | 120/850 | 0 (0) | 0 (741) | 0.78 · 0.00 | 1.2 | 1215/1 | 0 |
| S6 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3028 | 2958/2966 | 2809/3597 | 0 (0) | 0 (2295) | 1.08 · 0.00 | 3.9 | 3028/1 | 0 |
| S7 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1232 | – | 119/140 | 0 (0) | 0 (823) | 2.17 · 0.00 | 2.1 | 1232/1 | 0 |
| S8 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3028 | – | 120/2901 | 0 (0) | 0 (823) | 1.10 · 0.22 | 2.0 | 3028/1 | 438 |
| S9 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3467 | – | 108/140 | 26 (0) | 0 (2506) | 1.21 · 0.00 | 3.0 | 3941/3 | 0 |
| S10 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1206 | – | 114/140 | 0 (0) | 0 (823) | 0.78 · 0.00 | 0.9 | 1206/1 | 0 |
| S11 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/2559 | 52515/62844 | 111/30109 | 0 (0) | 0 (823) | 1.01 · 0.00 | 1.0 | 2559/1 | 0 |
| S12 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/9674 | – | 0/0 | 0 (0) | 0 (566) | 2.07 · 0.00 | 1.9 | 9674/1 | 0 |
| S12s | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/9674 | – | 0/0 | 0 (0) | 0 (65) | 2.07 · 0.00 | 1.9 | 9674/1 | 0 |
| S13 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 110/140 | 0 (0) | 0 (826) | 0.78 · 0.00 | 0.8 | 1202/1 | 0 |
| S14 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 111/139 | 0 (0) | 0 (823) | 0.79 · 0.03 | 0.8 | 1202/1 | 61 |
| S15 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/2559 | – | 2802/3586 | 0 (0) | 0 (5118) | 1.01 · 0.00 | 5.6 | 2559/1 | 0 |
| S15k | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/2568 | – | 2802/3586 | 0 (0) | 0 (3009) | 1.01 · 0.00 | 5.6 | 2568/1 | 0 |
| S16 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/2767 | 53499/64402 | 5075/5247 | 0 (0) | 0 (825) | 1.63 · 0.00 | 1.3 | 2767/1 | 0 |
| S17 | 20 | 0 | 0 | 0 | 0 | 0 | 0 | 0/8025 | – | 114/140 | 0 (0) | 0 (6208) | 105.62 · 0.00 | 12.4 | 8025/1 | 0 |
| S18 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3097 | – | 110/139 | 0 (0) | 0 (1294) | 1.20 · 0.00 | 0.9 | 3097/1 | 0 |
| S19 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/2559 | 57080/63598 | 112/62813 | 0 (0) | 0 (823) | 1.01 · 0.00 | 1.0 | 2559/1 | 0 |

C-kesinde "Provisional" yalnız kesinleşmiş bardaki iddialar için (besleme geride/kesikken ekranda henüz kapanmamış bar); M6g sütunu yalnız bunları ölçer.

### 4.3 N (kontrol tabanı, aynı oturumda yeniden koşuldu)

| Senaryo | Hatalı tohum | M1 | M2 | M3k | M3g | M5 | M6g p50/p99 | M6k p50/p99 | a→r B/s |
|---|---|---|---|---|---|---|---|---|---|
| S0 | 97 | 1057 | 1306 | 0 | 2 | 0/1202 | 85/104 | 85/105 | 3.15 |
| S1 | 0 | 0 | 0 | 0 | 0 | 0/1202 | 0/0 | 0/0 | 3.15 |
| S2a | 100 | 1212 | 1453 | 0 | 2 | 0/1202 | 85/104 | 85/105 | 3.15 |
| S2 | 100 | 1239 | 1466 | 0 | 2 | 0/1202 | 85/104 | 85/105 | 3.15 |
| S2c | 100 | 1600 | 1551 | 0 | 2 | 0/1202 | 85/104 | 85/105 | 3.15 |
| S3 | 67 | 0 | 241 | 0 | 89 | 0/1202 | 3548/4025 | 3561/4029 | 3.15 |
| S4 | 72 | 64 | 319 | 0 | 15 | 0/1202 | 285/2877 | 310/30128 | 3.15 |
| S5 | 99 | 1093 | 1569 | 0 | 13 | 0/1215 | 83/106 | 86/104 | 6.16 |
| S6 | 100 | 2691 | 2770 | 0 | 0 | 0/3028 | 85/107 | 84/107 | 4.34 |
| S7 | 100 | 1175 | 16988 | 0 | 7560 | 0/1232 | 84/107 | 87/107 | 2244.62 |
| S8 | 100 | 2881 | 3206 | 330 | 1906 | 1045/3028 | 85/107 | 85/106 | 20.93 |
| S9 | 100 | 2925 | 3467 | 0 | 6 | 0/3467 | 85/106 | 85/107 | 5.57 |
| S10 | 96 | 1058 | 1360 | 0 | 4 | 0/1206 | 84/106 | 87/106 | 3.28 |
| S11 | 100 | 2936 | 8740 | 0 | 43 | 0/2559 | 85/107 | 86/106 | 27.96 |
| S12 | 100 | 7425 | 7500 | 0 | 0 | 0/9674 | 0/0 | 0/0 | 30.31 |
| S12s | 100 | 8074 | 9074 | 0 | 0 | 0/9674 | 0/0 | 0/0 | 30.31 |
| S13 | 97 | 1060 | 1310 | 0 | 18 | 0/1202 | 84/106 | 86/103 | 3.15 |
| S14 | 98 | 1050 | 1439 | 39 | 389 | 60/1202 | 102/43210 | 88/2028 | 2.91 |
| S15 | 0 | 0 | 0 | 0 | 0 | 0/2559 | 0/0 | 0/0 | 20.42 |
| S15k | 100 | 0 | 1489 | 0 | 0 | 0/2568 | 0/0 | 0/0 | 20.33 |
| S16 | 100 | 2778 | 3423 | 0 | 9 | 0/2767 | 86/106 | 87/106 | 13.12 |
| S17 | 20 | 5879 | 8189 | 0 | 628 | 0/8025 | 85/106 | 89/107 | 3926.48 |
| S18 | 100 | 3098 | 6923 | 0 | 18 | 0/3097 | 85/106 | 85/107 | 27.30 |
| S19 | 100 | 3154 | 8770 | 0 | 35 | 0/2559 | 85/107 | 86/106 | 27.96 |

## 5. Senaryo senaryo

### 5.1 Hatasız olanlar

- **S1** (gecikme yok): C ≡ gerçek; hata 0, M6 0. Gösterilen kümede N'den farkı yalnız
  δ kuralının bazı canlı geçicileri hiç göstermemesi (M4 204, N 250).
- **S3** (protokol 3–4 s geride): C protokolü gösterim için beklemediği için gecikme N'nin
  ~3,5 s'si yerine **30–40 ms** (M6k p99 40 ms). Tek M3g olayı canlı yarış (§5.2).
- **S4, S13** (kanal titremesi, sıra bozma, yineleme, kopma): kanal gösterimi etkilemiyor;
  değerler S0 ile aynı. Mesaj sırası `n` ile korunuyor; `SourceDiffers` hiç oluşmadı (M13 0).
- **S6** (yansı beslemesi kayıplı, kapanış tikinin %50'si): **hata 0** (N: 2691 erken, 2770
  çelişkili). Kapanış tiki kaybolan bar boşluk taşıdığı için Final kesinleşmiş barla gelir
  (M6k p50 2,8 s); canlı geçici, önceki bar onaylanana kadar gösterilmez.
- **S8** (kapanış yarışı, iki uçta %25 kapanış tiki kaybı): **hata 0, kaçan 0** (N: 330 kesin
  yanlış, 1045 kaçan). Otorite kendi kayıplı beslemesiyle yanlış/eksik iddia ettiğinde yansı
  `SourceDiffers`/`Disputed` gösterip rapor ediyor (M13 438 s / 100 tohum; r→a 0,22 B/s); otorite
  kesinleşmiş barları çekip kendini düzeltince işaret kalkıyor.
- **S12, S12s** (farklı çapa): hello'daki çapa ile eksik 2700/2940 bar kesinleşmiş kaynaktan
  tamamlanıyor; **hata 0** (N: 7425/8074 erken). Bedeli: yansının görünen verisi bağın
  penceresinden geniş (3000 bar) ve başlangıç ~0,9 s geç (M8 süresi, eşiği aşmıyor).
- **S14** (otoritenin beslemesi hatalı): yansı otoritenin yanlışını **göstermez** (N: 39 kesin,
  389 geçici yanlış); otoritenin söylediği ama yansının onaylı verisinde olmayan iddialar
  `Disputed` (iddia etmez), tersi `SourceDiffers` olarak işaretlenip raporlanıyor (M13 61 s).
  Tek M3g olayı canlı yarış.
- **S15, S15k** (yansı fiyatı f32): ilk denetim bağı `Inexact` yapıyor; Final yalnız kesinleşmiş
  barlarla (M6k p50 2,8 s), canlı geçicide ek pay. **S15k'de hata 0** (N: 1489 çelişkili);
  bıçak sırtındaki geçiciler hiç gösterilmiyor (M6g "–").
- **S2c** (yansı beslemesi 60 s geride): kesinleşmiş kaynak beslemenin önünde; geçmiş ve
  kapanmış barlar oradan, **kaçan 0** (saf C'de 217/255). Bayat canlı bar gösterilmiyor. Final
  ancak beslemenin bir sonraki barı ekrana gelince (M1 kuralı: Final için bar görünen veride
  kapanmış olmalı) → M6k p50 61,7 s; o arada iddia kesinleşmiş barda `Provisional` olarak
  ~2,8 s'de ekranda (M6g ölçüsü iddianın canlı barda ilk tuttuğu andan sayar: p50 35 s).
- **S7** (60 tik/s salınım): δ kuralı M3g'yi 0'a, yanıp sönmeyi tek iddiada 196'dan 13'e indiriyor;
  bedeli geçici gecikme (p50 9,6 s).

### 5.2 Kalan M3g: canlı geçici yarışı (S0, S2a, S2, S3, S4, S5, S9, S10, S11, S13, S14, S16, S17, S18, S19)

Hepsi aynı olay (düzeneğin bir kopyasına hata ayıklama çıktısı eklenerek tek tek doğrulandı,
düzeneğin kendisine dokunulmadan): yansı iddiayı kendi verisinde görür görmez gösterir, ama
gerçek iddiayı 10–80 ms önce bozmuştur (ör. S11 #3: gerçek 977,524 s'de bozuldu, yansı
977,534 s'de gösterdi). Yansının beslemesi gerçeğin 80–140 ms (S2a: 500 ms) gerisinde ve
`live_grace_ms = 0` olduğu için, canlı geçici gösterim yapan **hiçbir** yaklaşım bunu kendi
verisinden kesinlikle sıfırlayamaz; δ yalnızca olasılığı düşürür. Denendi: bütçe 300 ms ve
katsayı 2 ile 20 tohumda S11 5 → 4, S19 4 → 3, S2a/S10 değişmedi; S7'de geçici gecikme p50 ≈10 s → 20 s; seyrek
tikli dünyada (S11/S19: bar başına 30 tik) kalan olay tek bir uçuştaki tikten geliyor ve onu
yakalayacak pay geçici gösterimi neredeyse kapatıyor. Bu yüzden ayar 250 ms / 1,5'te bırakıldı ve
seçim ürün kararına bırakıldı (§7): canlı geçici isteniyorsa C (84 olay / 2 320 koşu), M3 = 0
şartsa C-kesin.

S11/S19'daki olayların kesintiyle ilgisi yok (aynı dünya tohumları — #7, #12, #16 — iki
senaryoda da hatalı; kesinti penceresinde gösterim yok). S16'daki iki olay aynı anda bağ 1'in
iddiası ve ona dayanan çift iddia (tek yarış, iki anahtar). S2'de bağ "geride" işaretlenmeden
önceki ilk barlarda bayat beslemenin geçicisi gösterilebiliyor (2 olay); işaret oturunca kalkıyor.

### 5.3 Düzeltme ve bölünme

- **S9**: düzeltme beslemeden gelir, motor yeniden hesaplar, görüntü aynı geri çağrıda izler:
  M11 p50 109 ms, çözülmeyen 0, gereksiz değişim 0. İddia doğuran düzeltme (d) onaylı önek
  üstünde olduğu için doğrudan `Final`. 4 M3g canlı yarış.
- **S10**: bölünme anlık görüntüsü farklı veri dönemi olarak algılanır (ilk ortak bar eşit
  değil): veri değiştirilir, kesinleşmiş önek sıfırlanıp yeniden denetlenir. ×0,5 RSI'ı
  değiştirmediği ve besleme kesin olduğu için ekranda hiçbir şey değişmez (M8 0, gereksiz 0).

### 5.4 Kesinti (S11, S19)

Besleme sustuğunda kesinleşmiş barlar her bar gelmeye devam eder; donmuş canlı bardaki geçici
iddia gizlenir; kapanan barların iddiaları kesinleşmiş barda ~2,8 s'de gösterilir: **kaçan 0**
(2 dk kesintide bile). M6k p99 (S19: 62,8 s) kesinti sırasındaki son kesinleşmiş barın ekranda
"kapanmamış" kalmasından (bir sonraki kesinleşmiş bar gelene kadar `Provisional`). M7 (bayat)
S11/S19'da ~6,2 s/tohum: canlı geçici iddianın bar kapanırken gerçekte geri çekilip yansıda
beslemesi gelene kadar kalması (çoğu < 200 ms) ve kesinti başında susma eşiğine (≥ 5 s) kadar
donmuş canlı barda kalan geçiciler; hiçbiri Δ_stale'i aşmadı.

## 6. Maliyet

- **Protokol (M9).** a→r: S0'da 0,78 B/s (N 3,15), iddia başına 232 B (N 942); akış deflate
  0,13 B/s. S7'de 2,17 B/s (N 2245: C geçici iddiaları göndermez). S17'de (50 bağ) 105,6 B/s
  (N 3926), çoğu filigran. r→a yalnız uyuşmazlıkta (S8 0,22, S14 0,03 B/s).
- **Yansı işi (M10).** M10 geri çağrı başına iştir ve C'nin yansısı N'den çok geri çağrı alır
  (1 s'lik bakım zamanlayıcısı, yoklama yanıtları), bu yüzden oran (S0 0,8; N 2,4) yanıltıcıdır.
  **Mutlak iş** (100 tohum toplamı): S0 455 506 (N 447 608, +%2), S7 6,24 M (N 5,89 M, +%6),
  S17 89,94 M (N 89,82 M). Ek iş etiket hesabı ve kesinleşmiş barla değiştirilen barların yeniden
  hesabı; en yüksek oran S15/S15k (5,6: her bar kesinleşmiş barla değiştirilip yeniden hesap).
  Duvar saati S17'de olay başına ~2,9 µs.
- **Kesinleşmiş kaynak (REST).** Bağ başına bar başına ~1 istek (yakınsadıktan sonra), başlangıçta
  birkaç yoklama. S12'de 2700 barlık tek geri doldurma. Bu, düzenekte ölçülmeyen gerçek bir
  maliyettir (sağlayıcı istek sınırı: §8).
- **Gecikme.** Kesin: besleme kesinse ~besleme gecikmesi (S0 p50 111 ms), boşluk/f32'de
  kapanış + ~2,8 s, geride beslemede beslemenin gecikmesi. Geçici: seyrek tikte ~besleme
  gecikmesi, hızlı salınımda δ yüzünden saniyeler.
- **Başlangıç.** Hello + ilk denetim gelene kadar (~0,9 s; S3'te ~4 s) gösterim yok: M8 süresi
  (eşik 5 s, adet 0).

## 7. Sınırlar ve ürün soruları

- **İki kullanıcı farklı şey görür.** Her yansı kendi verisini gösterir; beslemesi geride ya da
  kayıplı olan kullanıcı aynı iddiayı saniyeler sonra ya da (onaylanana kadar) hiç görmez.
  Otorite ile yansı ayrıştığında yansı kendi onaylı verisini gösterir (`SourceDiffers`) —
  belgedeki tek otorite (K21-2) görüntüde delinir. Ürün açısından sorulması gereken: "Ekranda
  sunucunun karnesinde olmayan bir olgu görmek kabul mü?" (S14'te bu doğru olgudur, S8'de de;
  ama ajanlar ve karne otoritenin olgularını kullanır.)
- **Otoritenin bulduğunu yansı kendisi bulamazsa göstermez.** `Disputed` yalnız işarettir;
  ajanın yazdığı bulgu ve dayandığı olgu yansıda yoksa C onu göstermez (RESEARCH §3.C).
  Verisiz/ekransız istemciye C hiçbir şey vermez.
- **Canlı geçicinin katı doğruluğu yok** (§5.2). Geçici gösterim istenirse M3g'nin küçük ama sıfır
  olmayan bir oranı kalır; ya `live_grace_ms` > 0 (ör. besleme gecikmesi kadar) bir ürün kararı
  olur ya da C-kesin seçilir.
- **Kesinleşmiş kaynağa bağımlılık.** M3k = 0 ve M5 = 0, her cihazın sağlayıcının kesinleşmiş bar
  ucuna (Matriks REST) erişmesine ve bunun beslemeyle aynı gerçeği vermesine dayanır; istek sınırı,
  yetki ve gecikmesi gerçek üründe ölçülmedi. Düzenek REST'i her zaman ve gecikmesiz-hatasız
  sunuyor.
- **Hello'ya bağımlılık.** Yansı başlangıçta otoritenin çapasını bekler (S12'nin bağ hatasını
  düzeltmek için). Çapa şemada bağın parçası olursa (RESEARCH §1.2/5) bu mesaj gereksizleşir.
- **Geride beslemede düzeltme.** Besleme "geride"yken cephedeki bayat olaylar atlanır;
  kesinleşmiş yoklama yalnız yeni barları ister. Geride bir beslemede eski bir barın düzeltmesi
  ancak besleme onu getirdiğinde görülür (senaryolarda bu birleşim yok).
- **Ayarlar.** δ (250 ms, 1,5), susma eşiği (5 s / 8 aralık), uyuşmazlık toleransı (2 s) ve
  yoklama sabitleri 20 tohumluk geliştirme koşularında seçildi; son ölçüm 100 tohumla (farklı
  tohumlar dahil) yapıldı. Düzeneğe ve senaryolara dokunulmadı.

## 8. Belirleyicilik

Yaklaşım saat, işletim sistemi rastgeleliği ve ortam okumaz; yalnız `BTreeMap`/`BTreeSet`;
`f64` ortalamalar belirleyici. Kanıt:

- `cargo test` içinde `approaches::c::tests::deterministic_and_never_early_or_contradictory`:
  S4, S6, S11, S12s, S14, S15k, S16 × 2 tohum, iki kez → görüntü günlükleri bayt düzeyinde aynı,
  M1 = M2 = M3k = 0.
- `harness determinism --approach c --scenario S7 --scenario S11 --scenario S9 --scenario S16 --seeds 3`:

```
S7 seed 0: 1401 lines, hash 2b29eeee2671edfe / 2b29eeee2671edfe -> identical
S7 seed 1: 1947 lines, hash 2863ad586f2cdeef / 2863ad586f2cdeef -> identical
S7 seed 2: 302 lines, hash a40e7fd0a521ea7d / a40e7fd0a521ea7d -> identical
S11 seed 0: 320 lines, hash deda94bf827aeb73 / deda94bf827aeb73 -> identical
S11 seed 1: 346 lines, hash 2dbf32b700529ca2 / 2dbf32b700529ca2 -> identical
S11 seed 2: 390 lines, hash 67749cc839a4c99c / 67749cc839a4c99c -> identical
S9 seed 0: 51 lines, hash df733273b0270d70 / df733273b0270d70 -> identical
S9 seed 1: 46 lines, hash 75f88566f6456c31 / 75f88566f6456c31 -> identical
S9 seed 2: 104 lines, hash 364c1a90174a5978 / 364c1a90174a5978 -> identical
S16 seed 0: 27 lines, hash 7e5a6b5851be6df5 / 7e5a6b5851be6df5 -> identical
S16 seed 1: 61 lines, hash 280e48ddad7e91b9 / 280e48ddad7e91b9 -> identical
S16 seed 2: 77 lines, hash 826fcc365d604bb7 / 826fcc365d604bb7 -> identical
```

Crate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` temiz
(düzeneğin kendi testleri — oracle'ın hata ayıklama öz denetimi dahil — geçiyor).

## 9. Düzenek hakkında notlar (değiştirilmedi)

- Engelleyen bir hata bulunmadı.
- `M8` süresi başlangıçtaki toplu gösterimi de sayıyor: yansı hello + ilk denetimi beklediği
  için her tohumda geçmiş iddialar ~0,9 s "engellenmiş" görünüyor (S0: 100 tohumda 823 s; adet 0).
  Canlı fazı ayrı raporlamak (canlı hata gibi) yaklaşımları karşılaştırmayı kolaylaştırır.
- `M6g` iddianın canlı barda gerçekte *ilk* tuttuğu andan ölçülür; bar içinde birkaç kez belirip
  kaybolan iddiada (S7) gecikme ilk belirişe göre büyük çıkar. Bu tanım kararlı; yorumlanırken
  hesaba katılmalı.
- `run --out` verilmezse çalışma dizinine `harness-results-<ad>.json` yazılıyor; dal temizliği
  için `.gitignore`'a eklenmesi önerilir (yapılmadı).

## Öneriler (kapsam dışı; yapılmadı)

- `live_grace_ms` için ürün kararı (RESEARCH açık soru S1'in devamı): canlı geçici gösterimde
  besleme gecikmesi kadar bir tolerans kabul edilirse C'nin kalan M3g'si tanım gereği kalkar.
- Bağın hesap başlangıcı (çapa) şemaya girerse hello'daki `a` alanı kalkar.
- Gerçek sağlayıcıda kesinleşmiş bar ucunun gecikmesi, istek sınırı ve beslemeyle bit eşitliği
  ölçülmeli (C, D ve kesinleşmeye dayanan her yaklaşım için ortak varsayım).
- Denetçi (T11) C'yi `--spec` ile, özellikle "geride besleme + düzeltme" ve "REST gecikmesi
  yüksek" birleşimleriyle sınamalı.
