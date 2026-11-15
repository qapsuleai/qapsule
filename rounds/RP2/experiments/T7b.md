# RP2/T7b — Yaklaşım B: kanıt değerleri (ölçüm)

Tarih: 2026-11-15. Dal: `rp2/t7b-approach`. Kod:
`rounds/RP2/experiments/harness/src/approaches/b.rs` (+ `approaches/mod.rs`'te tek satır).
Düzenek değiştirilmedi. Tasarım kaynağı `rounds/RP2/RESEARCH.md` §3.B; ölçüler ve senaryolar
`rounds/RP2/experiments/HARNESS.md` §4–5. Bu belge B'yi **en güçlü savunucusunun kuracağı
haliyle** kurar ve ölçer; diğer yaklaşımların (A, C, D) kodu görülmedi.

## 1. Özet

- **Tüm senaryolarda (S0–S19, eklenenler dahil; S17 20 tohum, diğerleri 100) M1 = M2 = M3 = 0**,
  canlı hata 0, hatalı tohum 0. N aynı tohumlarda 22/24 senaryoda düşüyor (yalnız S1 ve S15'te hatasız).
- Kaçan (M5) yalnız S4'te: 2/1202 (30 s'lik protokol kopması). Yanlış engelleme (M8, > 5 s
  bölüm) yalnız S4'te 2.
- Bedel gecikmedir: yansının verisi eksik ya da boşluklu olduğunda iddia, yansı kendi
  kesinleşmiş barını (REST) çekene kadar bekler: S6/S8/S14'te kesin gösterim p95 ≈ 2,9 s
  (N: ~0,1 s ama yanlış/çelişkili). Gecikmesiz ağda B, N ile aynı hızda (S0 p50 111 ms,
  N 85 ms; fark yansı beslemesinin 80–140 ms gecikmesi — B yansının kendi barını bekler).
- Bayt: iddia başına 221 B ham JSON (N: 516–87 453 B), canlı saniye başına 0,7–5,9 B
  (S17: 251 B/s, N: 3 927 B/s). Canlı bar iddiaları gönderilmediği için S7'de N'nin 1/390'ı.
- Seçilen B = **B-yüklem + "kesinleşmiş-geçici" + veri hijyeni + itiraz + doldurma**
  (§3). Varyantlar (§6): B-katı, canlı geçici, çekirdek B.

## 2. Tasarım

### 2.1 Mesajlar (JSON, Ö4)

Otorite → yansı: her geri çağrının değişiklikleri tek bir dizi mesajında.

```json
[{"op":"claim","n":57,"k":"rsi_cross[period=14,level=70]@px0:1700000280000:up",
  "m":"final","ok":true,"anchor":{"px0":1699976040000},
  "pts":[["px0",1700000220000,68.47995938601073],["px0",1700000280000,70.98631673221242]]}]
```

(221 bayt boşluksuz; ölçülen M9 iddia başına 221 B ile aynı.)

| Alan | Anlam |
|---|---|
| `n` | Otoritenin tekdüze işlem numarası. Yansı iddia başına en büyük `n`'i tutar (anahtar başına son-yazan-kazanır): kanalda sıra bozulması, yineleme, yeniden bağlanmada tekrar zararsız. |
| `k` | İddianın anlamla kimliği (`ClaimKey::text`: dedektör, parametreler, bağ, bar zamanı `t`, yön). Bar numarası yok — düzlemler arası kimlik zamandır (PROTOCOL §18.3). |
| `m` | Otoritenin verisinde bar durumu: `provisional` (canlı) / `final` (kapanmış). |
| `ok` | Otoritenin verisi iddia barına kadar onarılmamış sıra boşluğu içermiyor (§2.3). |
| `anchor` | Hesap başlangıcı: bağ başına otoritenin ilk bar zamanı (RESEARCH §1.2/5). |
| `pts` | **Kanıt değerleri**: `[seri, t, değer]` — dedektörün RSI(`period`) çıktısı, iddia barında ve ondan önceki barda, bağ başına (çift iddiada 4 nokta). Eşik `k` içindeki parametrede. |

`{"op":"retract","n":58,"k":"…"}` (82 B): otorite iddiayı artık ileri sürmüyor.
Yansı → otorite: `{"op":"dispute","k":"…"}` (75 B; "yeniden çektiğim verim iddianı
doğrulamıyor") ve `{"op":"check","k":"…"}` ("temiz verim bu kesin iddiayı tutuyor, sen
söylemiyorsun"). Kanalda bar yok (K23): yalnız iddianın dayandığı birkaç sayı.

Seçilen B'de (`prov = Settled`) otorite **yalnız kesin iddiaları** gönderir; canlı bar
iddiaları kanala hiç girmez (yansı onları zaten göstermez, §2.2). Kesin iddia değerleri
düzeltmeyle değişirse yeniden gönderilir.

### 2.2 Yansının durum makinesi (iddia başına)

Girdi: otoritenin son işlemi, yansının kendi motorundaki iddia (aynı `Engine`, aynı
dedektör), yansının veri hijyeni. Her yansı geri çağrısının sonunda etkilenen iddialar
yeniden değerlendirilir (motor değişimleri, bekleyen iddialar, hijyen değişince bağın
bütün iddiaları); ekran böylece her sanal anın sonunda yansının verisiyle tutarlıdır.

```
                       otorite: claim (n)
                              │
          ┌── kanıt barları (pts'deki t'ler) yansıda yok / hesap başlangıcı uzatılıyor
          ▼
     [Pending] ──3 s──► kendi REST'inden doldur (çapadan ya da son bardan → t+1 bar), 5 s'de bir yokla
          │
          ▼ barlar var
   yansının kendi iddiası?
     ├─ var, kapanmış, otorite final ─┬─ güvenilir (yansı temiz ∨ ok) ─► [Final]  (yansının değerleriyle)
     │                                └─ değil (iki uç da boşluklu) ──► [Pending] (onarım sürüyor)
     ├─ var, yansıda canlı bar, otorite final
     │       ├─ bar t yansıda REST'ten kesinleşmiş değerle var ∧ ok ∧ güvenilir ─► [Provisional]
     │       └─ değil ─► [Pending] (yansının kapanışını bekle; 3 s sonra doldur)
     ├─ yok, bar yansıda henüz kapanmadı ─► [Pending]
     ├─ yok, yansının verisi boşluklu ─► [Pending] (onarım sürüyor)
     └─ yok, yansı temiz ─► kendi REST'inden [çapa..t] yeniden çek (bar t kesinleşene kadar yinele)
             ├─ şimdi tutuyor ─► [Final]
             └─ hâlâ tutmuyor ─► [Disputed] + "dispute" → otorite kendi aralığını yeniden çeker, düzeltirse retract

  otorite retract / yansının iddiası kaybolur ─► hemen gizle (otoritenin geri çekmesini beklemeden)
  yansıda temiz kesin iddia, otorite 3 s içinde söylemedi ─► "check" (otorite yeniden çeker)
```

- **Yüklem karşılaştırması (B-yüklem):** gösterim koşulu yansının motorunda iddianın
  var olmasıdır — bu, dedektörün yükleminin yansının verisi üzerinde doğru olmasıdır.
  Ekranda her zaman yansının değerleri; otoritenin değerleri yalnız iddia etmeyen
  durumlarda (`Pending`, `Disputed`, `src0…`) görünür. Kanıt ile ekran çelişemez (M2 = 0
  yapısal).
- **Etiket = iki olgunluğun en küçüğü:** `Final` yalnız yansıda bar kapanmış **ve**
  otorite `final` derken (M1 = 0 yapısal). `Provisional` yalnız "otorite kesin dedi, yansının
  grafiği barı henüz kapatmadı ama barın kesinleşmiş değerini REST'ten aldı" durumunda —
  değerler gerçek bar olduğu için canlı tik yarışı yoktur (M3g = 0).
- **Güven kuralı (iki kaynaktan biri temiz):** yansının ve otoritenin yüklemi birlikte
  tutuyorsa ve uçlardan en az birinin verisi boşluksuzsa, o ucun kapanmış barları gerçek
  barlarla aynıdır, yüklemi gerçektir; ikisi birden aynı kapanış tikini kaybettiyse
  (S8'de %6) aynı yanlış veriyle "anlaşabilirler" — bu durumda onarım beklenir.
- **Zaman aşımları:** veri bekleme 3 s → kendi REST'inden doldurma; 5 s'de bir yoklama
  (en çok 40); onarım ilk deneme 2 s (kesinleşme gecikmesi), sonra 1 s'de bir (en çok 20);
  itiraz için yeniden çekme, iddia barı kesinleşmemişse 1 s'de bir yinelenir. Bekleme
  süresince ekranda dürüst `Pending` ("sunucu bir şey buldu, veri geliyor").

### 2.3 Veri hijyeni (iki uçta; PROTOCOL §18.4 "boşluk doldurma barındırıcının işi")

- **Şüpheli bar:** kaçan bir `seq`, bir barın son alınan olayıyla **başka** bir barın ilk
  alınan olayı arasındaysa o barın kapanış değeri kaybolmuş olabilir → bar şüpheli. Bar
  içindeki kayıplar (sonraki tikler `c`'yi yeniden yazar) şüpheli değildir. Geç gelen olay
  boşluğu doldurursa şüphe kalkar.
- **Onarım:** şüpheli barlar kendi kesinleşmiş kaynağından çekilir (`fetch_settled`),
  `Engine::apply_settled` ile uygulanır ve "REST" olarak işaretlenir (güvenilir).
- **Otorite doğrulaması:** otorite her kapanan barı 2,5 s sonra kesinleşmiş kaynağa karşı
  doğrular (bozuk fiyatı — boşluk bırakmayan hata, S14 — da düzeltir). Bunun yansıya
  gecikme maliyeti yoktur (güven kuralı "yansı temiz ∨ ok").
- **Eski tik süzgeci:** REST'ten yazılmış bir barın tiki, besleme o barı geçmeden gelirse
  (besleme geride) motora verilmez; besleme sonraki bara geçtikten sonra gelen yama
  düzeltmedir ve uygulanır. **Eski anlık görüntü süzgeci:** `seq`'i uygulanmış olandan küçük
  anlık görüntü (gecikmiş yinelenen başlangıç görüntüsü) uygulanmaz.
- **Hesap başlangıcının uzatılması:** iddianın `anchor`'u yansının ilk barından eskiyse
  yansı `[anchor, ilk bar)` aralığını kendi REST'inden çeker ve başa ekler (PROTOCOL §18.3:
  "aynı bar numaralarını isteyen yansı bağı `from_t: anchor_t` ile çözer"); bu sırada bağın
  iddiaları `Pending` (S12'de ≈ 0,8 s). 8+ değişen bar tek seferde (motorun anlık görüntü
  yoluyla, tek yeniden hesap) uygulanır.

Hiçbir hijyen adımı protokol kanalından veri geçirmez; hepsi yansının/otoritenin **kendi**
beslemesi ve kendi kesinleşmiş kaynağıdır (adapter sözleşmesi).

## 3. Neden bu B'nin en iyi hali

1. **B-yüklem, B-katıdan iyidir** (§6): katı karşılaştırma f32 kaynakta (S15) ve farklı
   hesap başlangıcında sürekli uyuşmazlık verir; yüklem aynı doğruluğu (M1–M3 = 0) daha
   az beklemeyle verir. Katı eşitliği RESEARCH de "savunucunun tercihi değil" diye anar.
2. **Canlı bar iddiası göstermemek** (`Settled`): RESEARCH'teki B canlı iddiayı "yansının
   canlı barında yüklem doğruysa geçici" gösterir. Ölçüm (§6, `b-live`) bunun M3g'yi
   sıfırlayamadığını gösterir: yansının bilgisi her zaman ≥ 80 ms eskidir, 60 tik/s'de
   gerçek canlı bar o arada döner (gerçeklik toleransı 0 ms). B'nin şartı "yanlış = 0"
   olduğuna göre canlı tik yarışına girmemek doğru seçimdir; yerine, beslemesi geride
   kalan yansıda **kesinleşmiş değerle** geçici gösterim konur (S2c ve S19'da kaçanı
   sıfırlayan şey bu).
3. **Hijyen + güven kuralı**: kanıt değerleri tek başına "iki uç aynı yanlış veriyle
   anlaşıyor" durumunu (ikisi de aynı kapanış tikini kaybetti) yakalayamaz; boşluk takibi
   bunu kesin olarak sınıflar ve onarım kendi REST'iyle yapılır.
4. **İtiraz ve kontrol**: otoritenin yanlışı (S14) yalnız gösterilmemekle kalmaz;
   `dispute` / `check` + otoritenin doğrulaması onu birkaç saniyede düzeltir (S14'te kaçan
   0; N'de 60).
5. **`anchor` alanı**: RESEARCH'in "hesap başlangıcı" uyarısını mesajın kendisi taşır;
   yansı farkı görüp kendini düzeltir (S12/S12s'de 0 hata, 0 kaçan; N'de 7 425 / 8 074 erken).

## 4. Sonuçlar (B, varsayılan tohumlar)

Komut: `harness run --approach b --scenario all` (S17: 20 tohum, diğerleri 100).
M6 ms; M8 = 5 s'yi aşan bölüm sayısı / toplam süre; M9 = a→r ham bayt/iddia · ham B/s ·
mesaj-deflate B/s · akış-deflate B/s; r→a ham B/s.

| Senaryo | M1 | M2 | M3k | M3g | M4 | M5 kaçan/uygun | M6g p50/p95/p99 | M6k p50/p95/p99 | M7 s (>Δ) | M8 (s) | M9 a→r | r→a B/s | M10 | M11 p50/çöz./gereksiz | M12 top/en çok | M13 s |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| S0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 111/137/139 | 0 (0) | 0 (1) | 221 · 0,7 · 0,2 · 0,16 | 0 | 2,5 | – | 1202/1 | 0 |
| S1 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 0/0/0 | 0 (0) | 0 (0) | 221 · 0,7 · 0,2 · 0,16 | 0 | 2,5 | – | 1202/1 | 0 |
| S2a | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 513/524/525 | 0 (0) | 0 (0) | 221 · 0,7 · 0,2 · 0,16 | 0 | 2,5 | – | 1202/1 | 0 |
| S2 | 0 | 0 | 0 | 0 | 0 | 0/1202 | 36224/62926/63810* | 5130/5238/5248 | 0 (0) | 0 (0) | 221 · 0,7 · 0,2 · 0,16 | 0 | 4,5 | – | 1208/3 | 0 |
| S2c | 0 | 0 | 0 | 0 | 0 | 0/1202 | 36224/62926/63810* | 61670/62893/62974 | 0 (0) | 0 (0) | 221 · 0,7 · 0,2 · 0,16 | 0 | 4,5 | – | 1208/3 | 0 |
| S3 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 3522/3987/4025 | 0 (0) | 0 (4182) | 221 · 0,7 · 0,2 · 0,16 | 0,24 | 2,5 | – | 1202/1 | 0 |
| S4 | 0 | 0 | 0 | 0 | 0 | **2**/1202 | – | 301/1532/30255 | 0 (0) | **2** (441) | 221 · 0,7 · 0,2 · 0,16 | 0 | 2,5 | – | 1202/1 | 0 |
| S5 | 0 | 0 | 0 | 0 | 0 | 0/1215 | – | 114/739/843 | 0 (0) | 0 (1) | 221 · 0,7 · 0,2 · 0,16 | 0 | 3,8 | – | 1215/1 | 0 |
| S6 | 0 | 0 | 0 | 0 | 0 | 0/3028 | 4045/4057/4057* | 2894/2937/4362 | 0 (0) | 0 (1) | 221 · 1,9 · 1,0 · 0,33 | 0 | 12,3 | – | 3028/1 | 0 |
| S7 | 0 | 0 | 0 | 0 | 0 | 0/1232 | – | 119/138/140 | 0 (0) | 0 (1) | 221 · 5,7 · 1,7 · 1,24 | 0 | 2,0 | – | 1232/1 | 0 |
| S8 | 0 | 0 | 0 | 0 | 0 | 0/3028 | – | 132/2926/2937 | 0 (0) | 0 (975) | 221 · 1,9 · 1,0 · 0,33 | 0 | 6,3 | – | 3028/1 | 0 |
| S9 | 0 | 0 | 0 | 0 | 0 | 0/3467 | – | 108/137/140 | 19,8 (0) | 0 (2) | 310 · 3,2 · 0,9 · 0,51 | 0 | 11,7 | 97 ms / 1** / 0 | 3941/3 | 0,09 |
| S10 | 0 | 0 | 0 | 0 | 0 | 0/1206 | – | 114/138/140 | 0 (0) | 0 (1) | 221 · 0,7 · 0,2 · 0,16 | 0 | 2,9 | – | 1206/1 | 0 |
| S11 | 0 | 0 | 0 | 0 | 0 | 0/2559 | 53561/63884/63885* | 111/138/30109 | 0 (0) | 0 (2) | 221 · 1,6 · 0,8 · 0,29 | 0 | 2,9 | – | 2559/1 | 0 |
| S12 | 0 | 0 | 0 | 0 | 0 | 0/9674 | – | 0/0/0 | 0 (0) | 0 (566) | 221 · 5,9 · 1,5 · 1,02 | 0 | 6,8 | – | 9674/1 | 0 |
| S12s | 0 | 0 | 0 | 0 | 0 | 0/9674 | – | 0/0/0 | 0 (0) | 0 (65) | 221 · 5,9 · 1,5 · 1,02 | 0 | 6,7 | – | 9674/1 | 0 |
| S13 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 110/137/139 | 0 (0) | 0 (1) | 221 · 0,7 · 0,2 · 0,16 | 0 | 2,3 | – | 1202/1 | 0 |
| S14 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 112/2879/3397 | 0 (0) | 0 (32) | 234 · 0,8 · 0,2 · 0,17 | 0 | 2,6 | – | 1202/1 | 1,45 |
| S15 | 0 | 0 | 0 | 0 | 0 | 0/2559 | – | 0/0/0 | 0 (0) | 0 (0) | 221 · 1,6 · 0,8 · 0,29 | 0 | 2,4 | – | 2559/1 | 0 |
| S15k | 0 | 0 | 0 | 0 | 0 | 0/2568 | – | 800/3200/3200 | 0 (0) | 0 (0) | 221 · 1,6 · 0,8 · 0,26 | 0,01 | 23,1 | – | 2568/1 | 0 |
| S16 | 0 | 0 | 0 | 0 | 0 | 0/2767 | 54141/63893/63904* | 5074/5233/5247 | 0 (0) | 0 (1) | 233 · 1,8 · 0,5 · 0,34 | 0 | 3,6 | – | 2771/3 | 0 |
| S17 | 0 | 0 | 0 | 0 | 0 | 0/8025 | – | 114/139/140 | 0 (0) | 0 (20) | 225 · 251,3 · 63,5 · 43,2 | 0 | 12,5 | – | 8025/1 | 0 |
| S18 | 0 | 0 | 0 | 0 | 0 | 0/3097 | – | 110/137/139 | 0 (0) | 0 (2) | 235 · 2,0 · 0,9 · 0,36 | 0 | 2,4 | – | 3097/1 | 0 |
| S19 | 0 | 0 | 0 | 0 | 0 | 0/2559 | 57963/63895/63909* | 112/139/63880 | 0 (0) | 0 (2) | 221 · 1,6 · 0,8 · 0,29 | 0 | 3,0 | – | 2559/1 | 0 |

Hatalı tohum: hiçbir senaryoda yok (en kötü tohum: yok). Canlı hata: 0.

\* **M6g okuması:** B canlı bar iddiası göstermez; `Provisional` yalnız "otorite kesin, yansı
barı REST'ten kesinleşmiş değerle tutuyor ama grafiği henüz kapatmadı" durumundadır. Düzenek
M6g'yi iddianın gerçek canlı barda **ilk** tuttuğu andan ölçer (bar içinde, kapanıştan
onlarca saniye önce olabilir); bu yüzden 36–58 s görünür. Aynı iddiaların kesinleşmeden
sonraki gecikmesi ≈ 3,8 s'dir (doldurma bekleme 3 s + REST 0,8 s).

\*\* S9'daki 1 çözülmeyen M11: bir düzeltme gerçek **canlı** barda bir geçici iddiayı
doğurdu ve bar kapanmadan kayboldu; B canlı iddia göstermediği için "ekran gerçeğe uydu"
anı hiç gelmedi. Hata değil, canlı iddia göstermeme politikasının ölçüdeki izi.

### 4.1 N ile karşılaştırma (aynı tohumlar)

| Senaryo | N: M1 / M2 / M3k / M3g | N: M5 | N: M6k p50/p99 | N: M9 B/iddia · B/s | B: M1–M3 | B: M5 | B: M6k p50/p99 | B: M9 |
|---|---|---|---|---|---|---|---|---|
| S0 | 1057 / 1306 / 0 / 2 | 0 | 85/105 | 942 · 3,1 | 0 | 0 | 111/139 | 221 · 0,7 |
| S2c | 1600 / 1551 / 0 / 2 | 0 | 85/105 | 942 · 3,1 | 0 | 0 | 61670/62974 | 221 · 0,7 |
| S3 | 0 / 241 / 0 / 89 | 0 | 3561/4029 | 942 · 3,1 | 0 | 0 | 3522/4025 | 221 · 0,7 |
| S4 | 64 / 319 / 0 / 15 | 0 | 310/30128 | 942 · 3,1 | 0 | 2 | 301/30255 | 221 · 0,7 |
| S6 | 2691 / 2770 / 0 / 0 | 0 | 84/107 | 516 · 4,3 | 0 | 0 | 2894/4362 | 221 · 1,9 |
| S7 | 1175 / 16988 / 0 / 7560 | 0 | 87/107 | 87453 · 2244,6 | 0 | 0 | 119/140 | 221 · 5,7 |
| S8 | 2881 / 3206 / 330 / 1906 | 1045 | 85/106 | 2488 · 20,9 | 0 | 0 | 132/2937 | 221 · 1,9 |
| S9 | 2925 / 3467 / 0 / 6 | 0 | 85/107 | 542 · 5,6 | 0 | 0 | 108/140 | 310 · 3,2 |
| S11 | 2936 / 8740 / 0 / 43 | 0 | 86/106 | 3934 · 28,0 | 0 | 0 | 111/30109 | 221 · 1,6 |
| S12s | 8074 / 9074 / 0 / 0 | 0 | 0/0 | 1128 · 30,3 | 0 | 0 | 0/0 | 221 · 5,9 |
| S14 | 1050 / 1439 / 39 / 389 | 60 | 88/2028 | 871 · 2,9 | 0 | 0 | 112/3397 | 234 · 0,8 |
| S15k | 0 / 1489 / 0 / 0 | 0 | 0/0 | 2849 · 20,3 | 0 | 0 | 800/3200 | 221 · 1,6 |
| S16 | 2778 / 3423 / 0 / 9 | 0 | 87/106 | 1707 · 13,1 | 0 | 0 | 5074/5247 | 233 · 1,8 |
| S17 | 5879 / 8189 / 0 / 628 | 0 | 89/107 | 3523 · 3926,5 | 0 | 0 | 114/140 | 225 · 251,3 |
| S19 | 3154 / 8770 / 0 / 35 | 0 | 86/106 | 3934 · 28,0 | 0 | 0 | 112/63880 | 221 · 1,6 |

N'nin tam tablosu `HARNESS.md` §8'dedir (aynı sayılar bu oturumda yeniden üretildi).

## 5. Senaryo senaryo

- **S0, S1, S10, S13, S18 — B'nin kolay olduğu yer.** Gecikme yokken (S1) N ile aynı kesin
  gösterim, aynı iddialar; geçici gösterim yok (M4 = 0 — N'de 250). S0'da kesin gösterim
  ≈ 25 ms N'den geç: B yansının kendi kapanışını bekler (yansı beslemesi 80–140 ms), N
  otoritenin sözünü (60–110 ms) gösterir — ama N bunu 1057 kez erken yapar. S10 (bölünme
  ×0,5): yüklem ayakta, değerler aynı bit; B hiç etkilenmez (M8 = 0, M13 = 0). S13: anahtar
  başına `n` ve motorun `seq` politikası yinelemeyi tamamen emer (N'de M7 > Δ 9 bölüm,
  B'de 0). S18: geçersizlik dedektörü aynı genel kuralla, ek kod olmadan.
- **S2a/S2/S2c — yansı geride.** B iddiayı yansının barları gelene kadar `Pending`
  tutar: S2a 513 ms, S2 5,1 s (gecikmenin kendisi). S2c'de (60 s) beslemeyi beklemek
  Δ_miss'i (30 s) aşardı: 3 s sonra yansı barı **kendi REST'inden** çeker ve
  `Provisional` (kesinleşmiş değerle) gösterir → kaçan 0; `Final` beslemenin kapanışıyla
  (61,7 s). Başlangıçta yansının hiç verisi yokken doldurma iddianın `anchor`'undan
  başlar (bu olmadan ilk ölçümde S2c kaçanı 396/1202 idi — geçmiş iddialar).
- **S3 — protokol geride (3–4 s).** B'nin doğal zayıflığı: otoritenin mesajı gelmeden
  hiçbir şey gösterilmez. Yansının verisi iddiayı zaten tutarken geçen süre M8 toplamına
  girer (4 182 s / 100 tohum ≈ iddia başına 3,5 s), ama 5 s eşiğini aşan bölüm yok.
  N de aynı gecikmeyle (ve 241 çelişkili + 89 yanlış gösterimle) gösterir.
- **S4 — protokol titremesi, 30 s kopma.** Sıra bozulması/yineleme `n` ile zararsız.
  Kopma boyunca kesinleşen 2 iddia kopma bitene kadar gösterilemedi → **M5 = 2, M8 = 2**
  (M6k p99 30 s). Bu B'nin yapısal sınırıdır: otoritenin iddiası olmadan gösterim yok
  (yansının kendi verisiyle göstermek C'nin işi).
- **S5 — beslemede sıra bozulması.** `SeqLww` + boşluk takibi; geç gelen tik boşluğu
  doldurunca şüphe kalkar. p99 843 ms: kapanış tikinin sıra bozulmasıyla 0,8 s'ye kadar
  geç gelmesi (yansının yüklemi o ana kadar tutmuyor).
- **S6 — yansı beslemesi kayıplı (kapanış tikinin %50'si).** N 2 691 erken + 2 770
  çelişkili. B'de, kapanış tiki kaybolan barda yansının yüklemi tutmaz (grafikte kesişim
  yok) → `Pending`; boşluk takibi barı şüpheli sayar, 2 s sonra REST'ten onarır, iddia
  gösterilir: M6k p50 2,9 s. Yüklemin yerel verisinde tuttuğu durumda otoritenin `ok`'u
  yeterlidir (gecikme yok). Kaçan 0.
- **S7 — canlı bar titremesi.** N'de 7 560 geçici yanlış ve tek iddiada 196 yanıp sönme.
  B canlı iddia göstermediği ve göndermediği için 0 hata, M12 en çok 1, bayt N'nin
  1/390'ı. Bedel: canlı bar boyunca "geçici" bilgi yok (`b-live` varyantı, §6).
- **S8 — kapanış yarışı, iki uçta %25 kapanış tiki kaybı.** N'de 330 kesin yanlış + 1 045
  kaçan. B'de **0 / 0**: otoritenin kaybı otoritede onarılır (yanlış iddia 3 s içinde
  geri çekilir, kaçırdığı iddia eklenir), yansının kaybı yansıda; ikisi birden aynı tiki
  kaybettiğinde güven kuralı onarımı bekletir. p95 2,9 s.
- **S9 — düzeltmeler.** İddia düşüren düzeltme: yansı kendi beslemesinden düzeltmeyi
  alınca kendi yüklemi düşer, otoritenin geri çekmesini beklemeden gizler (M11 p50 97 ms;
  M7 toplam 19,8 s = 100 tohum × ~2 düzeltme × ~100 ms yansı besleme gecikmesi, hiçbiri
  Δ_stale'i aşmaz). İddia doğuran düzeltme: otorite yeni iddiayı gönderir, yansı kendi
  beslemesinden düzeltme gelince gösterir. 1 000 bar önceki düzeltme: bütün sonraki
  kesin iddiaların değerleri değişir; B-yüklemde ekran değişmez (gereksiz değişim 0),
  ama otorite değişen değerleri yeniden gönderir (S9'da iddia başına 310 B). M13 0,09 s
  (100 tohumda): düzeltmenin iki uca farklı anlarda ulaştığı kısa `Disputed`'ler.
- **S11/S19 — yansı besleme kesintisi (30 s / 2 dk).** Kesinti sırasında kesinleşen
  iddialar `Pending`; 3 s sonra yansı kendi REST'inden doldurur → `Provisional` (kesin
  değerle), sonraki bar kesinleşince `Final`; kesinti dönüşündeki `REFRESH` hepsini
  doğrular. Kaçan 0 (S19 2 dk > Δ_miss). M6k p99 30 s / 64 s: kesintide `Final` için
  yansının sonraki barı da gerekir; kullanıcı arada `Provisional` görür.
- **S12/S12s — farklı hesap başlangıcı.** Mesajdaki `anchor` sayesinde yansı farkı ilk
  iddiada görür, `[anchor, ilk bar)`'ı REST'ten çeker (≈ 0,8 s; bu sırada `Pending`,
  M8 toplamı 566 s / 65 s ama hiçbir bölüm 5 s'yi aşmaz). Sonra iki uç bit-eşit hesaplar.
  N: 7 425 / 8 074 erken. B'nin RESEARCH'teki "hesap başlangıcı" uyarısına cevabı budur;
  bedeli yansının grafiğine pencereden daha fazla geçmiş eklemesi (kullanıcı pencereyi
  "son 300 bar" diye bağladıysa bu bir ürün kararıdır).
- **S14 — otoritenin beslemesi hatalı (bozuk fiyat, kapanış tiki kaybı).** B'nin en çok
  parladığı yer: otoritenin yanlış iddiası yansının temiz verisinde tutmaz → yansı
  yeniden çeker, yine tutmaz → `Disputed` + `dispute`; otorite yeniden çeker, düzeltir,
  geri çeker (M13 toplam 1,45 s / 100 tohum). Otoritenin kaçırdığı iddia: kendi
  doğrulaması (2,5 s) ya da yansının `check`'i ile eklenir. Sonuç 0 hata, **0 kaçan**
  (N: 39 kesin + 389 geçici yanlış, 60 kaçan). p99 3,4 s.
- **S15/S15k — f32 kaynak.** S15'te f32 farkı yüklemi hiç çevirmez; B-yüklem hiç
  etkilenmez (M13 = 0). S15k'de (bıçak sırtı) yansının f32 yüklemi otoriteninkinden
  ayrılır: yansı `[çapa..t]`'yi kendi REST'inden (f64) yeniden çeker; t kesinleşince
  (≈ 2 s) yüklemler aynı olur → gösterilir (p50 0,8 s, p99 3,2 s), uyuşmazlık süresi 0.
  N: 1 489 çelişkili. B-katı burada sürekli uyuşmazlık verir (§6).
- **S16 — iki bağlı iddia, bağ 1 5 s geride.** Kanıt noktaları bağ başına; iddia iki bağın
  barı da kapanınca `Final` (5,1 s); 3 s'de geride kalan bağ REST'ten doldurulur →
  `Provisional`. 0 hata.
- **S17 — yük (50 bağ, ~1000 tik/s, 500 hesap).** 0 hata; 251 B/s ham, 43 B/s akış-deflate
  (N: 3 927 / 92). Yansı işi tik başına 12,5 birim (N: 12,4): B'nin durum makinesi motor
  maliyetinin yanında görünmez.

### 5.1 Ek deneme (düzenek hazır ayarı değil; `--spec` ile)

İki hatanın birlikte olduğu durumlar (güven kuralının zayıf noktası):

| Senaryo | B: M1/M2/M3 | B: M5 | B: M6k p50/p99 | N: M1/M2/M3k/M3g | N: M5 |
|---|---|---|---|---|---|
| X1 = S14 + yansı beslemesi de kayıplı (kapanış tikinin %30'u, %3 rastgele) | 0/0/0 | 0 | 118/3729 | 1051/1472/39/389 | 60 |
| X2 = S8 + otorite beslemesinde %2 bozuk fiyat | 0/0/0 | 0 | 135/2939 | 2562/3453/260/2272 | 1178 |

Yeniden üretim: `harness spec S14 > x.json`, `replica_feed.drop_last_tick_p = 0.3`,
`replica_feed.drop_p = 0.03`, `name = "X1"`; X2 için `harness spec S8`,
`authority_feed.glitch_p = 0.02`, `glitch_size = 0.01`. Sonra
`harness run --approach b --spec x1.json --spec x2.json`.

## 6. Varyantlar

`cargo test --release -- --ignored variants_table --nocapture` (aynı tohumlar). M8 = 5 s'yi
aşan bölüm sayısı.

Toplamlar (24 senaryo):

| Varyant | Ne değişir | M1 | M2 | M3k | M3g | M5 kaçan | M8 > 5 s | Not |
|---|---|---|---|---|---|---|---|---|
| **b** (seçilen) | yüklem, kesinleşmiş-geçici, hijyen, itiraz, doldurma | 0 | 0 | 0 | 0 | **2** (S4) | 2 (S4) | |
| b-off | geçici hiç yok | 0 | 0 | 0 | 0 | 235 (S2c 148, S19 53, S11 32, S4 2) | 2 | beslemesi geride/kesik yansıda `Final` Δ_miss'ten geç |
| b-live | + canlı bar iddiaları `Provisional` (otorite ∧ yansının canlı barı) | 0 | 0 | 0 | **6 783** (S7 5 982, S17 557, S8 108, …) | 0 | 0 | M6g p50 ≈ 110 ms; bayt S7'de 108,8 B/s (b: 5,7) |
| b-strict | kanıt değerleri bit-eşit olmalı (değilse yeniden çek, sonra `SourceDiffers`) | 0 | 0 | 0 | 0 | 2 | 2 | S15: M6k 3,2 s (b: 0), M8 toplam 5 525 s; S15k 2 693 s; S14 157 s |
| b-core | RESEARCH §3.B harfiyen: bekle, yüklem, göster; hijyen/yeniden çekme/doldurma yok | 0 | 0 | **97** (S8) | 0 | **21 798** | 448 | S8 1 473, S6 1 794, S12/S12s 7 500/8 196, S15k 1 486, S2c 1 202 kaçan; M13 S6'da 3 059 s |
| b-core-live | b-core + canlı geçici | 0 | 0 | 97 | 7 313 | 21 671 | 448 | |

Okuma:

- **Yapısal güvenceler her varyantta tutuyor:** M1 = M2 = 0 her yerde — gösterim yansının
  kendi motorundaki iddiaya bağlı olduğu sürece erken ve çelişkili gösterim olamaz.
- **Çekirdek B "yanlış = 0"ı sağlamaz:** S8'de iki uç aynı kapanış tikini kaybettiğinde
  aynı yanlış veriyle anlaşır → 97 kesin yanlış. Kanıt değerleri "iki uç anlaştı" der,
  "gerçek" demez; bunu boşluk takibi + kesinleşmiş kaynaktan onarım kapatır (b: 0).
- **Çekirdek B çok kaçırır:** yansının kaybı (S6), hesap başlangıcı (S12s), f32 (S15k),
  geride besleme (S2c) yansının yüklemini düşürür; yeniden çekme ve `anchor` uzatması
  olmadan iddia ya hiç gösterilmez ya `Disputed` kalır.
- **B-katı, B-yükleme göre yalnız maliyet ekler:** aynı doğruluk, f32'de her iddia için
  bir yeniden çekme (3,2 s) ve uzun bekleme.
- **Canlı geçici gösterim gerçek canlı barla yarışır:** otorite ∧ yansı birlikte tutsa
  bile ikisinin bilgisi de gerçek canlı bardan 60–140 ms geridedir; S7'de (60 tik/s)
  5 982 geçici yanlış (N: 7 560 — yalnız %21 az). Hiçbir sabitle sıfırlanmaz (tolerans 0).
- **Kesinleşmiş-geçici (b) b-off'tan katı biçimde iyidir:** aynı 0 hata, kaçan 235 → 2.

## 7. Maliyetler

- **Bayt (a→r):** iddia başına 221 B ham (çift iddia 4 nokta ile ≈ 290 B; S16 ortalama
  233). Mesaj başına deflate ham baytın ≈ %25–30'una, akış deflate %10–25'ine iner (S0:
  0,7 → 0,2 → 0,16 B/s). Kesin değer güncellemesi (düzeltme) iddia başına bir mesaj daha (S9 310
  B/iddia). Canlı iddia gönderilmediği için tik sıklığından bağımsızdır (S7 = S0 düzeyinde).
- **Bayt (r→a):** `dispute`/`check` seyrek: S3'te 0,24 B/s (protokol geride olduğu için
  yansının temiz kesin iddiası otoriteden önce gelir → `check`), diğerlerinde ≈ 0.
- **Kesinleşmiş kaynak (REST) çağrıları** — M9'a girmez, düzenek saymıyor: otoritede bar
  başına bağ başına 1 doğrulama + şüpheli bar onarımları + itiraz başına 1 (aralık
  `[çapa..t]`, 400–3000 bar); yansıda şüpheli bar onarımları, itiraz doğrulaması (bütün
  pencere), doldurma yoklaması (5 s'de bir, geride kalan iddia başına). Sunucu tarafında
  50 bağda dakikada ~50 küçük çağrı.
- **Yansı işi (M10):** tik başına 2,0–4,5 birim (N 1,8–2,9) çoğu senaryoda; S6 12,3 ve S9
  11,7 (onarım/düzeltme sonrası yeniden hesap), S15k 23,1 (bütün pencerenin f64 ile
  yeniden çekilip yeniden hesaplanması). Ölçüm duvar saati değil, belirleyici sayaç.
- **Gecikme:** iki kaynak — (i) yansı kendi verisini bekler (beslemesi otoritenin
  mesajından geç kalırsa); (ii) uyuşmazlıkta kesinleşme gecikmesi (2 s) + REST (0,8 s).

## 8. Sınırlar ve açık noktalar

- **Protokol yoksa gösterim yok** (S3, S4): B otoritenin iddiasını şart koşar. Kopma
  sırasında yansının kendi verisinde açıkça tutan iddia bile bekler (S4: 2 kaçan).
- **Canlı bar "geçici" bilgisi yok** seçilen varyantta. Canlı gösterim ancak M3g'yi
  kabul ederek ya da gerçeklik toleransıyla (`live_grace_ms`) yapılabilir; bu bir ürün
  kararıdır (RESEARCH açık soru S1).
- **Güven kuralı iki bağımsız hatada kırılabilir:** biri boşluk bırakmayan (bozuk fiyat)
  ve diğeri aynı yanlış yüklemi üretirse. X1/X2'de görülmedi; olasılığı çarpımsaldır.
- **Kanıt noktaları yüklemin tamamı değildir:** geçersizlik koşulu (S18) 20 barlık bakış
  penceresine dayanır; B-yüklem bunu yansının kendi dedektörüyle sınar (doğru), ama
  `pts` yalnız kesişim noktalarını taşır — B-katı ve "verisi olmayan istemci" için eksik
  (RESEARCH'teki uyarı). Çok olgulu desenlerde nokta listesi büyür.
- **"Aynı veriye dayanıyordu" denemez:** değerler (ya da yüklem) eşit, mum gövdeleri
  farklı olabilir (T1, T2 değil). Dondurulmuş kanıt için özet ayrıca gerekir.
- **Yansının grafiği bağdan farklılaşabilir:** hesap başlangıcı uzatıldığında daha çok
  geçmiş, doldurmada beslemenin önünde REST barları gösterilir (gerçek barlar).
- **Sabitler** (3 s, 5 s, 2 s, 2,5 s) senaryolara göre ayarlanmadı; kesinleşme gecikmesi
  (2 s) sağlayıcı özelliğidir ve yansı/otorite bunu bilmeden yeniden deneyerek çalışır.
- **Düzenekte hata bulunmadı.** Not: `Engine::apply_settled` her eklenen/değişen eski bar
  için baştan yeniden hesap yapar (3000 barlık uzatmada M10 2 327/tik ölçüldü); B bu yüzden
  8+ değişen barı motorun anlık görüntü yolundan tek seferde uygular (M10 6,8).

## 9. Belirleyicilik

- Saat, işletim sistemi rastgeleliği, `HashMap` yok; bütün durum `BTreeMap`/`BTreeSet`;
  zaman yalnız `ctx.now()`; zamanlayıcı ve çekme kimlikleri sayaçla.
- `harness determinism --approach b --scenario S7 --scenario S11 --scenario S9 --scenario
  S14 --scenario S6 --seeds 3`: 15/15 koşu bayt düzeyinde aynı görüntü günlüğü (ör. S9
  tohum 0: 81 satır, `39297a65fef9da1b`; S14 tohum 1: 28 satır, `c574387b2f8e1c7b`).
- S8 20 tohum, `--threads 1` ve `--threads 8`: aynı özet.
- Birim testleri (`approaches::b::tests`): anahtar metni gidiş-dönüş, kanıt değerlerinin
  JSON'da bit-eşit taşınması, S1/S2/S6'da 0 hata, S8'de aynı tohum → aynı günlük.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` temiz.

## Öneriler (kapsam dışı; yapılmadı)

- Düzenek kesinleşmiş kaynak çağrılarını da saymalı (M9'un yanında "REST çağrısı/bayt");
  B ve hijyen kullanan her yaklaşım için maliyetin görünmeyen kısmı bu.
- Veri hijyeni (boşluk → şüpheli bar → kesinleşmiş kaynaktan onarım; otorite doğrulaması)
  yaklaşımdan bağımsızdır ve PROTOCOL §18.4'teki "barındırıcının işi"nin somut hali olarak
  hangi yaklaşım seçilirse seçilsin şartnameye girmeli.
- M6g'nin "ilk gerçek canlı tutma"dan ölçülmesi, kesinleşmiş değerle geçici gösterimde
  yanıltıcı; ayrı bir "kesinleşmeden sonra ilk iddia eden gösterim" gecikmesi eklenebilir.
