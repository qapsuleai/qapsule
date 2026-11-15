# RP2/T7a — Yaklaşım A: veri konumu damgası (ölçüm)

Tarih: 2026-11-15. Dal: `rp2/t7a-approach`. Kod: `rounds/RP2/experiments/harness/src/approaches/a.rs`
(kayıt: `approaches/mod.rs` içinde `approaches!(n, a);`). Düzenek değiştirilmedi.
Başvurular: `rounds/RP2/RESEARCH.md` §1.3, §3.A, §3.E, §4; `rounds/RP2/experiments/HARNESS.md`;
`docs/PROTOCOL.md` §18 (`data_digest`, `DataPosition`, `EvidenceRef`).

Bu belge A'yı **savunucusunun yazacağı en iyi haliyle** kurar ve ölçer. B, C, D dallarına
bakılmadı.

## 0. Özet

- **Bütün senaryolarda (S0–S19, S17'de 20, diğerlerinde 100 tohum) M1 = M2 = M3 = 0;
  hatalı tohum 0.** Aynı koşuda N: 24 senaryonun 22'sinde hatalı tohum var (S1 ve S15 hariç).
- Kesin gösterim gecikmesi (M6k p50) temiz beslemede **~110 ms** (N: ~85 ms); yansının
  verisi otoriteninkinden farklıysa (kayıp tik, f32, bozuk otorite beslemesi) **~2,8 s**
  (kesinleşmiş bar çekme yolu). M8'de 5 s'yi aşan bölüm yalnız S4'te 2 tane (protokol 30 s
  kopuk).
- **Geçici (canlı bar) iddia hiç iddia edilmez**; yalnız iddia etmeyen `Pending` ("sunucu bir
  şey gördü, bar kapanınca doğrulanacak"). Canlı barda `Provisional` gösteren deneme
  (A-canlı, §5.4) M1 = M2 = M3k = 0'ı korudu ama M3g'yi sıfırda tutamadı (S7'de 5998).
- Bayt: a→r ham 1,5–5,5 B/s (tek bağ), S17'de 190 B/s (50 bağ; N 3927 B/s). Kanal bar
  taşımaz (K23): yalnız konum, özet, iddia anahtarı.
- Kaçanlar (M5) yalnız yansının **görünen verisi** iddiayı göstermeye yetmediği yerlerde:
  S2c (yansı beslemesi 60 s geride) 149/1202, S11 32/2559, S19 53/2559 (yansı beslemesi
  kesik), S4 2/1202 (protokol kopuk). Bunlar M1 = 0 kuralının bedelidir (§6).

## 1. Tasarım

### 1.1 Fikir

Otorite, her bağ için "şu bara kadar olan veri budur" diyen bir **veri konumu** yayar:
son onaylanmış bar `W`, hesap başlangıcından (`a`) `W`'ye kadar bütün barların özeti `h`,
veri dönemi `e` ve `W`'nin sağlayıcının kesinleşmiş barına karşı denetlenip denetlenmediği
(`s`). Konumla birlikte o konumun **kesinleştirdiği iddiaları** listeler (§3.E'nin
"facts through" işareti). Yansı bir iddiayı ancak **kendi verisi o konuma ulaşıp kendi
önek özeti otoriteninkine eşit olduğunda** `Final` gösterir. Aynı veri + aynı çapa + aynı
motor ⇒ aynı iddia kümesi; yani gösterilen iddia yansının görünen verisinde kesin olarak
tutar (M1 = M2 = 0) ve otoritenin kaynağa eşit olduğunu onayladığı veriye dayanır (M3k = 0).

### 1.2 Mesajlar (JSON, kısa alan adları)

Otorite → yansı (`AMsg`; boş alanlar yazılmaz):

| Alan | Biçim | Anlam |
|---|---|---|
| `s` | sayı | Mesaj sıra numarası (otorite başına, 1'den) |
| `p` | `[[b, W, h, e, a, s], …]` | Veri konumları: bağ dizini, son onaylı bar zamanı, `a..=W` barlarının `data_digest`'i (16 onaltılık), veri dönemi, hesap başlangıcı (ilk bar zamanı), `1` = `W`'ye kadar her bar kesinleşmiş kaynağa karşı denetlendi / `0` = kuyruk yalnız besleme sıra numarasıyla onaylı |
| `f` | `[[tür, b, t], …]` | Bu konumların kesinleştirdiği iddialar (tür: `u` 70 yukarı, `d` 30 aşağı, `i` geçersizlik, `p` iki bağlı — `b`=0) |
| `x` | `[[tür, b, t], …]` | Geri çekilen iddialar (anında) |
| `l` | `[[b, [[tür, t], …]], …]` | Bekleyen ipucu: bağın henüz onaylanmamış iddiaları (canlı bar ve az önce kapanmış bar); bağın önceki ipucunun yerine geçer; 500 ms birleştirilir |

Yansı → otorite (`RMsg`): `{"d":[[b, W], …]}` — uzlaşamayan veri (yalnız kayıt).

Gerçek koşudan örnekler (S9 tohum 0 ve S16 tohum 0):

```json
{"s":5,"p":[[0,1700000040000,"6bb3fce01f790435",0,1699928040000,0]]}
{"s":6,"p":[[0,1700000040000,"6bb3fce01f790435",0,1699928040000,1]]}
{"s":36,"p":[[0,1700000820000,"5c01b80cb0fed076",2,1699928040000,1]]}
{"s":157,"p":[[0,1700002020000,"31087e0d5ac18987",0,1699976040000,0]],"f":[["p",0,1700002020000]]}
{"s":38,"l":[[0,[]]]}
```

İlk iki satır aynı bar için iki aşamadır: bar kapanır kapanmaz besleme onaylı konum (`s=0`,
~30 ms sonra), kesinleşmiş bar geldiğinde aynı özetle kaynak onaylı konum (`s=1`, kapanıştan
~2,8 s sonra). Üçüncü satırda dönem 2: o koşuda iki düzeltme önceden onaylanmış barları
değiştirmiş. Konum mesajı ~70 B; tek bağda bar başına iki konum mesajı.

Özet: `qp_schema::data::data_digest` (FNV-1a 64, PROTOCOL §18.5) — her bar için `t` (i64 LE)
ve `o,h,l,c,v` bitleri (LE). FNV akışkan olduğu için önek özeti bir **zincir** gibi artımlı
tutulur: bar *i*'den sonraki durum = `data_digest(bars[0..=i])` (birim testi
`chain_equals_data_digest_of_prefix`). Böylece konum özeti doğrudan §18'deki `DataPosition`
/ `EvidenceRef` özetiyle aynı nesnedir; ayrı bir özet işlevi gerekmez.

### 1.3 Otoritenin algoritması

1. Kendi beslemesini paylaşılan `Engine`'e uygular (`SeqLww`).
2. **Kirlilik izi:** bir veri olayı `seq` boşluğuyla gelirse son olayın barından bu olayın
   barına kadar bütün barlar "kirli" işaretlenir (kaybolan olay hangisine aitti bilinmez).
   Anlık görüntü kirliliği siler (kaynağın durumudur).
3. **Onaylı bar `W`:** hesap başlangıcından `settled_upto`'ya kadar kesinleşmiş kaynakla
   denetlenmiş barlar + onu izleyen, kapanmış ve **kirli olmayan** barlar. Kirli olmayan
   kapanmış bar, kaynağın olay akışının tamamıyla kurulmuştur, yani kaynağın barıdır.
4. Her kapanmış bar `settled_at = kapanış + 2 s` anında `ctx.fetch_settled` ile çekilir;
   cevap `apply_settled` ile uygulanır, kirliliği siler, `settled_upto` ilerler. Cevap kısaysa
   250 ms sonra yeniden. Çekme sırasında beslemeden kapanmış bir bar değiştiyse (düzeltme)
   cevap atılır ve yeniden istenir (eski cevap düzeltmeyi geri almasın diye).
5. Onaylı konum değişince `p` + yeni kesinleşen iddialar (`f`): bütün bağları `W_b ≥ t` olan
   `Final` iddialar (iki bağlı iddiada konum vektörü).
6. Kapanmış bir bar beslemeden değişirse (düzeltme, bölünme/`REFRESH` anlık görüntüsü):
   o bardan itibaren `settled_upto` geri alınır, dönem artar, yeniden çekilir. Düzeltme
   olayı boşluksuzsa bar yeniden "temiz" olduğu için yeni konum (yeni özet, yeni dönem)
   **hemen** yayılır; kaynak onayı ~0,8 s sonra gelir.
7. İddia kümesinden çıkan, daha önce bildirilmiş iddia anında `x` ile geri çekilir.
8. Onaylanmamış iddialar `l` ile ipucu olarak (500 ms birleştirme).

### 1.4 Yansının durum makinesi (bağ başına)

```
                yeni konum / veri değişti
   ┌──────────┐ ────────────────────────► ┌──────────┐
   │ Waiting  │   W yansıda var, çapa eşit,│ Verified │
   │ (bekler) │   özet eşit               │ (doğru)  │
   └────┬─────┘ ◄──────────────────────── └────┬─────┘
        │       W'ye kadar veri değişti        │
        │ özet farklı                          │ yeni konum, özet farklı
        ▼                                      ▼
   s=0 konum: kendi kesinleşmiş barını kapanış+2 s'de hazır çek, Waiting'de kal
   s=1 konum: ┌──────────┐ cevap → yeniden karşılaştır ┌──────────┐
              │ Fetching │ ──── 2 denemede olmazsa ───► │ Disputed │
              └──────────┘  (1: son doğru konumdan W'ye; │ (uyuşmaz)│
                             2: otoritenin çapasından W'ye)└──────────┘
                                         daha yeni konum gelince yeniden dener
```

- **Waiting:** konum yok ya da yansının verisinde `W` yok (beslemesi geride). Veri gelince
  yeniden karşılaştırılır. Yansının hiç verisi yoksa (S2c'nin başı) bir kez kendi kesinleşmiş
  kaynağından `[a, W]` çeker ("yetişme").
- **Çapa:** `a` yansının ilk barından eskiyse yansı `[a, W]`'yi kendi kaynağından çekip
  geçmişini otoritenin çapasına uzatır (S12/S12s); çok sayıda bar tek bir kendi-veri anlık
  görüntüsü olarak birleştirilir (motor bir kez yeniden hesaplar). `a` yansının ilk barından
  yeniyse (yansı daha uzun geçmiş tutuyor) uzlaşma yok → `Disputed`.
- **Düzeltme önce yansıya ulaştıysa:** yansının kapanmış barı onaylı konumun gerisinde
  değişti → 1,5 s otoritenin yeni konumunu bekler (gereksiz çekmeyi önler), sonra çeker.
- **Kanal sırası güvenilmez:** bağ başına en yeni `s`'li konum geçerli, iddia başına en yeni
  ekle/çek geçerli; yinelemeler eşgüçlü. Kopma sonrası tutulan mesajlar zaten gelir.

Etiket kuralı (her geri çağrının sonunda değişen anahtarlar için):

| Durum | Etiket |
|---|---|
| Otorite kesinleştirdi + yansının motoru iddiayı `Final` tutuyor + (bütün bağları `Verified` ve `W_b ≥ t` **ya da** zaten `Final` gösteriliyor) | `Final` (yansının kendi değerleri ve kanıtıyla) |
| Otorite kesinleştirdi, yukarıdaki sağlanmıyor, bir bağ `Disputed` | `Disputed` |
| Otorite kesinleştirdi, henüz doğrulanmadı | `Pending` |
| Otoritenin ipucunda (canlı/az önce kapanmış bar) | `Pending` |
| Hiçbiri | gösterilmez |

"Zaten gösteriliyor" yapışkanlığı: doğrulanmış ve gösterilmiş bir iddia, yansının verisi
sonradan değişse bile (ör. bölünme anlık görüntüsü) **yansının motoru onu hâlâ tutuyorsa**
ve otorite geri çekmediyse kalır. Motor tutmayı bıraktığı anda (aynı geri çağrıda) gizlenir;
M2 bu yüzden yapısal olarak 0'dır. Yeni gösterim ise her zaman doğrulama ister.

### 1.5 Durumlara göre davranış

| Durum | Ne olur |
|---|---|
| Temiz besleme, yansı otoriteden yavaş | Konum ~85 ms'de gelir, yansı `W+1`'i gördüğünde (`W` görünür ve kapanmış) özet eşit → `Final`. M6k ≈ yansı beslemesi gecikmesi |
| Yansı tik kaybetti (S6), barı kapatan tik kayboldu (S8) | `s=0` konumda özet farklı → yansı kendi kesinleşmiş barını kapanış + 2 s'de çeker → eşit → ~2,8 s |
| Otorite tik kaybetti / bozuk fiyat (S14) | Otoritenin barı kirli → `s=0` konum o bardan önce durur; kesinleşmiş bar gelince `s=1` konum → ~2,85 s. Bozuk fiyat boşluk bırakmaz: o zaman otoritenin `s=0` özeti yansınınkinden farklı çıkar, yansı göstermez, `s=1` beklenir |
| Canlı bar | Hiç iddia edilmez; `Pending` ipucu |
| Düzeltme (S9) | Çekilen iddia anında `x`; doğan iddia düzeltme olayı boşluksuzsa yeni dönemle hemen kesinleşir (M11 p50 98 ms) |
| Uzak geçmiş düzeltmesi (S9c, 1000 bar önce) | Özet zinciri o bardan itibaren yeniden hesaplanır (bir kez 1000 bar); iki taraf da düzeltmeyi beslemesinden aldığı için yeni özetler eşit; gösterilenler yapışkanlıkla kalır |
| Bölünme (S10) | İki tarafta anlık görüntü, bütün özetler değişir, dönem artar; RSI bitleri aynı kaldığı için iddialar aynı; gösterilenler kalır, yenileri yeni konumla hemen |
| Yansı beslemesi kesik (S11, S19) | Konumlar `Waiting`'de bekler; `REFRESH` gelince özet eşit → toplu doğrulama |
| Protokol kopuk (S4, S11) | Gösterilen kalır (motor tutuyorsa); yeni konum gelmez → yeni iddia yok |
| f32 kaynak (S15, S15k) | Yansının geçmişi ve her kapanmış barı f32 → her konumda özet farklı → kendi kesinleşmiş (f64) barını çeker → eşit. Grafik f64 kesinleşmiş barları gösterir |
| Farklı çapa (S12, S12s) | Yansı geçmişini otoritenin çapasına kendi kaynağından uzatır |
| İki bağ (S16) | Konum vektörü: iki bağın da `W_b ≥ t` olması gerekir; geride kalan bağ beklenir |

## 2. Neden bu A'nın en iyi hali

RESEARCH §3.A'daki zayıf noktaların her biri, A'nın ilkesinden (konum + özet eşitliği olmadan
gösterme yok) vazgeçmeden kapatıldı:

1. **"Canlı barda neredeyse hiç eşleşmez → kapanışa kadar bekler."** Kapanmış barda
   eşleşmeyi hızlandırmak için iki aşamalı konum: besleme sıra numarası boşluksuzsa bar
   kaynağın barıdır, kapanış anında onaylanır (`s=0`); kesinleşmiş kaynak sonradan teyit eder
   (`s=1`). M6k p50 2,86 s'den (ilk sürüm, yalnız `s=1`) **111 ms**'ye indi (S0).
2. **"Önek özeti çok katı."** Düzeltme ve bölünmede dönem artar, yeni konum hemen yayılır;
   doğrulanıp gösterilmiş iddia, yansının motoru tuttuğu sürece kalır (yapışkanlık). Ölçülen
   M8 S9'da 1,9 s, S10'da 0,7 s (100 tohum toplamı).
3. **"Uyuşmazlıkta hangi barın farklı olduğunu söylemez."** İki aşamalı yeniden çekme: önce
   son doğrulanmış konumdan bu konuma (çoğu zaman tek bar), olmazsa çapadan (S12'de geçmişi
   uzatır). Merkle bölmesi (E) yerine bunu seçtim: düzenekte uyuşmazlık hep son barlardadır,
   tam çekme yalnız çapa farkında gerekti; bölme gidiş-dönüş başına kanal gecikmesi
   (~100 ms) ekler, son-bar çekmesi eklemez. Bu bir **ölçülmemiş** tercih; ürün için 10 000
   barlık geçmişte eski bir düzeltme kaçarsa E'nin bölmesi daha ucuzdur (Öneriler).
4. **Doğruluk (M3k).** Otorite kendi beslemesine güvenmez: her kapanmış bar kesinleşmiş kaynağa
   karşı denetlenir; boşluklu bar `s=0` ile onaylanmaz. S8'de N 330 kesin yanlış gösterdi,
   A 0; S14'te (otoritenin beslemesi bozuk) N 39 + 389, A 0.
5. **Yansının verisi eksik/farklı.** Yansı kendi kesinleşmiş kaynağından (kendi yetkisiyle;
   RESEARCH açık soru S2) çeker; hiçbir bar protokolden gelmez.
6. **Tekdüzelik ve kanal bozuklukları.** Baş tıkanması yaratan genel sıra tamponu yerine
   bağ/iddia başına "en yeni kazanır" (S4, S13'te hata 0, gecikme kuyruğu yok).
7. **Bayt.** Konum `[b,W,h,e,a,s]` dizisi, iddia `[tür,b,t]`; değerler gönderilmez (yansı
   kendi motorunun aynı bitlerini gösterir). S0'da 2,5 B/s (N 3,1), S7'de 5,5 B/s (N 2245).

## 3. RESEARCH metninden sapmalar

| RESEARCH §3.A / §3.E | Burada | Neden |
|---|---|---|
| Konum: `sealed_t`, `sealed_digest`, `live_t`, `live_rev`, `live_digest`, `data_epoch` | `[b, W, h, e, a, s]`; canlı bar konumu yok | Canlı bar iddia edilmediği için `live_*` gereksiz; çapa `a` farklı çapayı teşhis etmek için eklendi; `s` iki aşamalı onay için |
| Özet 128 bit (kriptografik kısaltma önerisi) | `data_digest` (FNV-1a 64) | §18'deki mevcut özetle aynı nesne; düzenekte kasıtlı değiştirme yok. Ürün kararı açık (RESEARCH S5) |
| Kesinleşme: otorite "tercihen" kesinleşmiş bara karşı | Her zaman; ek olarak boşluksuz besleme ile erken onay | M3k = 0 ile M6k'yi birlikte tutmak için |
| Yansı mesajları `seq` sırasıyla bekletir (tekdüze okuma) | Bağ/iddia başına en yeni kazanır | Aynı güvenlik, baş tıkanması yok |
| Uyuşmazlıkta "aralığı yeniden çek" | Son doğrulanmıştan, sonra çapadan; `s=0`'da kapanış+2 s'de hazır çekme; düzeltme önce geldiyse 1,5 s bekleme | Gereksiz çekmeyi ve gecikmeyi azaltmak |
| E: `data_mark` ayrı konu, `facts_through`, MMR kökü, `data_proof` bölmesi | Konum + `f` listesi aynı mesajda (işaret = konum); MMR/bölme yok | §2 madde 3 |
| Canlı bar: "bar kapanana kadar bekliyor" | Aynı, `Pending` etiketiyle; A-canlı deneyi ayrıca ölçüldü | §5.4 |
| (yok) | Hiç verisi olmayan yansı kendi kaynağından yetişir; çapa uzatma | S2c'de M5: ilk sürümde 1202/1202, son sürümde 149/1202 (100 tohum) |

## 4. Ölçüm

Komutlar (`rounds/RP2/experiments/harness`, `--release`, Apple M4 Max; varsayılan tohumlar:
100, S17'de 20):

```bash
./target/release/harness run --approach a --scenario all    # 85 s duvar
./target/release/harness run --approach n --scenario all    # 66 s duvar
```

Kısaltmalar: M3k/M3g = kesin/geçici yanlış; M6 ms; M7 s (parantezde Δ_stale'i aşan bölüm);
M8 = 5 s'yi aşan bölüm sayısı / toplam engelleme süresi (s, tohumlar toplamı); "–" = örnek
yok. A hiçbir tohumda hata yapmadığı için en kötü tohum sütununda M5 ve M8'in en kötü tohumu
var (yeniden oynatmak için: `harness log --approach a --scenario S# --seed N`).

### 4.1 A — doğruluk, gecikme, engelleme

| Senaryo | Tohum | Hatalı tohum | M1 | M2 | M3k | M3g | M4 | M5 kaçan/uygun | M6g p50/p99 ms | M6k p50/p95/p99 ms | M7 s (>Δ) | M8 adet / s | M11 p50 ms/çözülmeyen/gereksiz | M12 top/en çok | M13 s | En kötü tohum (M5 · M8) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| S0 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 111/137/139 | 0.0 (0) | 0 / 0.7 | –/0/0 | 1202/1 | 0.0 | – · 21 (0.1 s) |
| S1 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 0/0/0 | 0.0 (0) | 0 / 0.0 | –/0/0 | 1202/1 | 0.0 | – · – |
| S2a | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 513/524/525 | 0.0 (0) | 0 / 0.0 | –/0/0 | 1202/1 | 0.0 | – · – |
| S2 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 5130/5238/5248 | 0.0 (0) | 0 / 0.0 | –/0/0 | 1202/1 | 0.0 | – · – |
| S2c | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 149/1202 | – | 62801/63693/63774 | 0.0 (0) | 0 / 191.4 | –/0/0 | 1202/1 | 0.0 | 23 (5) · 19 (9.1 s) |
| S3 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 3600/3987/4018 | 0.0 (0) | 0 / 4187.7 | –/0/0 | 1202/1 | 0.0 | – · 26 (69.1 s) |
| S4 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 2/1202 | – | 322/1616/30098 | 0.0 (0) | 2 / 443.4 | –/0/0 | 1202/1 | 0.0 | 38 (1) · 85 (33.7 s) |
| S5 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1215 | – | 120/739/843 | 0.0 (0) | 0 / 3.3 | –/0/0 | 1215/1 | 0.0 | – · 86 (0.6 s) |
| S6 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3028 | – | 2801/2801/2801 | 0.0 (0) | 0 / 114.0 | –/0/0 | 3028/1 | 0.0 | – · 71 (8.0 s) |
| S7 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1232 | – | 119/138/140 | 0.0 (0) | 0 / 0.6 | –/0/0 | 1232/1 | 0.0 | – · 21 (0.1 s) |
| S8 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3028 | – | 2801/3654/3667 | 0.0 (0) | 0 / 1783.5 | –/0/0 | 3028/1 | 0.0 | – · 59 (35.8 s) |
| S9 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3467 | – | 108/137/140 | 20.0 (0) | 0 / 1.9 | 98/1/0 | 3941/3 | 0.0 | – · 21 (0.3 s) |
| S10 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1206 | – | 114/138/140 | 0.0 (0) | 0 / 0.7 | –/0/0 | 1206/1 | 0.0 | – · 21 (0.1 s) |
| S11 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 32/2559 | – | 111/138/30109 | 0.0 (0) | 0 / 2.0 | –/0/0 | 2559/1 | 0.0 | 1 (1) · 94 (0.1 s) |
| S12 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/9674 | – | 0/0/0 | 0.0 (0) | 0 / 565.6 | –/0/0 | 9674/1 | 0.0 | – · 2 (14.4 s) |
| S12s | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/9674 | – | 0/0/0 | 0.0 (0) | 0 / 64.8 | –/0/0 | 9674/1 | 0.0 | – · 2 (4.0 s) |
| S13 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 110/138/140 | 0.0 (0) | 0 / 3.4 | –/0/0 | 1202/1 | 0.0 | – · 23 (2.7 s) |
| S14 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/1202 | – | 2854/2869/2871 | 0.0 (0) | 0 / 356.5 | –/0/0 | 1202/1 | 0.0 | – · 23 (13.7 s) |
| S15 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/2559 | – | 2801/2801/2801 | 0.0 (0) | 0 / 5058.7 | –/0/0 | 2559/1 | 0.0 | – · 87 (65.6 s) |
| S15k | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/2568 | – | 2801/2801/2801 | 0.0 (0) | 0 / 2977.6 | –/0/0 | 2568/1 | 0.0 | – · 31 (66.8 s) |
| S16 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/2767 | – | 5074/5233/5247 | 0.0 (0) | 0 / 1.0 | –/0/0 | 2767/1 | 0.0 | – · 45 (0.1 s) |
| S17 | 20 | 0 | 0 | 0 | 0 | 0 | 0 | 0/8025 | – | 114/139/140 | 0.0 (0) | 0 / 19.9 | –/0/0 | 8025/1 | 0.0 | – · 5 (1.7 s) |
| S18 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 0/3097 | – | 110/137/139 | 0.0 (0) | 0 / 2.2 | –/0/0 | 3097/1 | 0.0 | – · 7 (0.2 s) |
| S19 | 100 | 0 | 0 | 0 | 0 | 0 | 0 | 53/2559 | – | 112/139/120087 | 0.0 (0) | 0 / 1.8 | –/0/0 | 2559/1 | 0.0 | 1 (1) · 21 (0.1 s) |

M6g bütün satırlarda "–": A canlı barda iddia etmez. M4 bu yüzden 0. M12 = gösterilen iddia
sayısı (her iddia bir kez belirir; S9'da düzeltmelerin gizleyip yeniden gösterdikleri dahil,
en çok 3).

### 4.2 A — maliyet

| Senaryo | M9 a→r B/iddia | mesaj/s | ham B/s | mesaj başına deflate B/s | akış deflate B/s | r→a B/s | M10 iş/olay | M10 duvar µs/olay (belirleyici değil) |
|---|---|---|---|---|---|---|---|---|
| S0 | 755 | 0.036 | 2.5 | 2.3 | 0.42 | 0.000 | 3.8 | 1.44 |
| S1 | 755 | 0.036 | 2.5 | 2.3 | 0.42 | 0.000 | 3.7 | 1.70 |
| S2a | 755 | 0.036 | 2.5 | 2.3 | 0.42 | 0.000 | 3.8 | 1.92 |
| S2 | 755 | 0.036 | 2.5 | 2.3 | 0.42 | 0.000 | 5.3 | 3.15 |
| S2c | 755 | 0.036 | 2.5 | 2.3 | 0.42 | 0.000 | 15.4 | 17.88 |
| S3 | 755 | 0.036 | 2.5 | 2.3 | 0.42 | 0.000 | 3.7 | 1.83 |
| S4 | 755 | 0.036 | 2.5 | 2.3 | 0.42 | 0.000 | 3.7 | 1.73 |
| S5 | 757 | 0.037 | 2.6 | 2.4 | 0.43 | 0.000 | 3.3 | 0.93 |
| S6 | 309 | 0.034 | 2.6 | 2.3 | 0.43 | 0.000 | 14.8 | 6.12 |
| S7 | 215 | 0.109 | 5.5 | 4.8 | 0.96 | 0.000 | 2.2 | 0.91 |
| S8 | 267 | 0.032 | 2.2 | 2.0 | 0.43 | 0.000 | 8.1 | 2.89 |
| S9 | 290 | 0.040 | 3.0 | 2.6 | 0.50 | 0.000 | 13.6 | 5.80 |
| S10 | 766 | 0.037 | 2.6 | 2.4 | 0.43 | 0.000 | 4.4 | 2.22 |
| S11 | 551 | 0.077 | 3.9 | 3.6 | 0.62 | 0.000 | 4.7 | 4.24 |
| S12 | 152 | 0.069 | 4.1 | 3.4 | 0.66 | 0.000 | 14.9 | 29.56 |
| S12s | 152 | 0.069 | 4.1 | 3.4 | 0.66 | 0.000 | 14.6 | 23.51 |
| S13 | 755 | 0.036 | 2.5 | 2.3 | 0.42 | 0.000 | 3.5 | 1.37 |
| S14 | 454 | 0.023 | 1.5 | 1.4 | 0.35 | 0.000 | 3.5 | 1.16 |
| S15 | 486 | 0.063 | 3.5 | 3.2 | 0.56 | 0.000 | 18.2 | 4.04 |
| S15k | 396 | 0.043 | 2.8 | 2.6 | 0.49 | 0.000 | 18.7 | 3.90 |
| S16 | 672 | 0.073 | 5.2 | 4.7 | 0.84 | 0.000 | 5.0 | 3.12 |
| S17 | 171 | 2.431 | 190.6 | 157.8 | 31.26 | 0.000 | 12.6 | 5.74 |
| S18 | 415 | 0.065 | 3.6 | 3.2 | 0.58 | 0.000 | 4.9 | 4.12 |
| S19 | 551 | 0.077 | 3.9 | 3.6 | 0.62 | 0.000 | 4.8 | 4.02 |

r→a 0: hiçbir koşuda `Disputed` olmadı, uyuşmazlık bildirimi gönderilmedi.

Kesinleşmiş kaynak (REST) çağrıları, tohum 0 (kodun geçici bir sayaçla sayılması; kodda
kalmadı): otorite her senaryoda bar başına bağ başına bir (tek bağ, 60 canlı bar: 62;
S9'da düzeltmeler yüzünden 70; S17'de 50 bağ: 400). Yansı yalnız uyuşmazlıkta: S0, S11,
S12, S14: 1; S9: 7; S8: 19; S6: 40; S15: 62 (her bar); S17: 37.

### 4.3 A ile N, aynı koşu

M3 = M3k + M3g; M8 = 5 s'yi aşan bölüm / toplam s; B/s = a→r ham.

| Senaryo | A: hatalı tohum | N: hatalı tohum | A: M1/M2/M3 | N: M1/M2/M3 | A: M5 | N: M5 | A: M6k p50/p99 | N: M6k p50/p99 | A: M8 adet/s | N: M8 adet/s | A: B/s | N: B/s |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| S0 | 0 | 97 | 0/0/0 | 1057/1306/2 | 0 | 0 | 111/139 | 85/105 | 0/1 | 0/1 | 2.5 | 3.1 |
| S1 | 0 | 0 | 0/0/0 | 0/0/0 | 0 | 0 | 0/0 | 0/0 | 0/0 | 0/0 | 2.5 | 3.1 |
| S2a | 0 | 100 | 0/0/0 | 1212/1453/2 | 0 | 0 | 513/525 | 85/105 | 0/0 | 0/0 | 2.5 | 3.1 |
| S2 | 0 | 100 | 0/0/0 | 1239/1466/2 | 0 | 0 | 5130/5248 | 85/105 | 0/0 | 0/0 | 2.5 | 3.1 |
| S2c | 0 | 100 | 0/0/0 | 1600/1551/2 | 149 | 0 | 62801/63774 | 85/105 | 0/191 | 0/0 | 2.5 | 3.1 |
| S3 | 0 | 67 | 0/0/0 | 0/241/89 | 0 | 0 | 3600/4018 | 3561/4029 | 0/4188 | 0/3699 | 2.5 | 3.1 |
| S4 | 0 | 72 | 0/0/0 | 64/319/15 | 2 | 0 | 322/30098 | 310/30128 | 2/443 | 0/336 | 2.5 | 3.1 |
| S5 | 0 | 99 | 0/0/0 | 1093/1569/13 | 0 | 0 | 120/843 | 86/104 | 0/3 | 0/1 | 2.6 | 6.2 |
| S6 | 0 | 100 | 0/0/0 | 2691/2770/0 | 0 | 0 | 2801/2801 | 84/107 | 0/114 | 0/1 | 2.6 | 4.3 |
| S7 | 0 | 100 | 0/0/0 | 1175/16988/7560 | 0 | 0 | 119/140 | 87/107 | 0/1 | 0/1 | 5.5 | 2244.6 |
| S8 | 0 | 100 | 0/0/0 | 2881/3206/2236 | 0 | 1045 | 2801/3667 | 85/106 | 0/1784 | 386/690978 | 2.2 | 20.9 |
| S9 | 0 | 100 | 0/0/0 | 2925/3467/6 | 0 | 0 | 108/140 | 85/107 | 0/2 | 0/2 | 3.0 | 5.6 |
| S10 | 0 | 96 | 0/0/0 | 1058/1360/4 | 0 | 0 | 114/140 | 87/106 | 0/1 | 0/1 | 2.6 | 3.3 |
| S11 | 0 | 100 | 0/0/0 | 2936/8740/43 | 32 | 0 | 111/30109 | 86/106 | 0/2 | 0/1 | 3.9 | 28.0 |
| S12 | 0 | 100 | 0/0/0 | 7425/7500/0 | 0 | 0 | 0/0 | 0/0 | 0/566 | 0/0 | 4.1 | 30.3 |
| S12s | 0 | 100 | 0/0/0 | 8074/9074/0 | 0 | 0 | 0/0 | 0/0 | 0/65 | 0/0 | 4.1 | 30.3 |
| S13 | 0 | 97 | 0/0/0 | 1060/1310/18 | 0 | 0 | 110/140 | 86/103 | 0/3 | 0/1 | 2.5 | 3.1 |
| S14 | 0 | 98 | 0/0/0 | 1050/1439/428 | 0 | 60 | 2854/2871 | 88/2028 | 0/357 | 60/91014 | 1.5 | 2.9 |
| S15 | 0 | 0 | 0/0/0 | 0/0/0 | 0 | 0 | 2801/2801 | 0/0 | 0/5059 | 0/0 | 3.5 | 20.4 |
| S15k | 0 | 100 | 0/0/0 | 0/1489/0 | 0 | 0 | 2801/2801 | 0/0 | 0/2978 | 0/0 | 2.8 | 20.3 |
| S16 | 0 | 100 | 0/0/0 | 2778/3423/9 | 0 | 0 | 5074/5247 | 87/106 | 0/1 | 0/1 | 5.2 | 13.1 |
| S17 | 0 | 20 | 0/0/0 | 5879/8189/628 | 0 | 0 | 114/140 | 89/107 | 0/20 | 0/20 | 190.6 | 3926.5 |
| S18 | 0 | 100 | 0/0/0 | 3098/6923/18 | 0 | 0 | 110/139 | 85/107 | 0/2 | 0/1 | 3.6 | 27.3 |
| S19 | 0 | 100 | 0/0/0 | 3154/8770/35 | 53 | 0 | 112/120087 | 86/106 | 0/2 | 0/1 | 3.9 | 28.0 |

N'nin rakamları `HARNESS.md` §8 ile aynı (aynı tohumlar; M3 burada k+g toplamı).

### 4.4 A-canlı deneyi (ölçülen yaklaşım değil)

Soru: A canlı barda da `Provisional` gösterse ne olur? `a.rs` içindeki `LIVE_PROVISIONAL`
anahtarı `true` yapılıp (ipuçları birleştirmesiz gönderilir) bütün senaryolar koşuldu, sonra
anahtar `false`'a geri alındı (kayıtlı yaklaşım budur). Kural: otoritenin ipucunda olan ve
yansının motorunun canlı barında tuttuğu iddia, bağın öneki doğrulanmışsa `Provisional`;
motor tutmayı bırakınca anında gizlenir.

| Senaryo | Hatalı tohum | M1/M2/M3k | M3g | M4 | M6g p50/p99 ms | B/s |
|---|---|---|---|---|---|---|
| S0 | 4 | 0/0/0 | 4 | 249 | 112/139 | 2.5 |
| S5 | 10 | 0/0/0 | 12 | 493 | 116/1752 | 2.6 |
| S7 | 99 | 0/0/0 | 5998 | 15711 | 131/9868 | 24.3 |
| S8 | 65 | 0/0/0 | 108 | 108 | 113/139 | 2.4 |
| S11 | 31 | 0/0/0 | 39 | 6437 | 109/18978 | 4.0 |
| S17 | 20 | 0/0/0 | 561 | 2314 | 117/5096 | 206.2 |
| S18 | 18 | 0/0/0 | 20 | 4267 | 109/139 | 3.6 |
| S19 | 32 | 0/0/0 | 37 | 6272 | 110/139 | 4.0 |

(Diğer senaryolarda M3g 0–7.) M1 ve M2 canlı barda da 0 kaldı (gösterim yansının kendi
verisine bağlı), ama **M3g sıfırlanamaz**: yansının verisi gerçeğin ~80–140 ms gerisinde;
gösterim anında gerçek canlı bar o arada geri dönmüş olabilir. Bunu hiçbir konum damgası
çözemez, çünkü damga geçmişe aittir. Bu yüzden ölçülen A canlı barda iddia etmez. Geçici
iddia ürün için gerekirse (RESEARCH açık soru S1) M3g'nin tolerans tanımı (`live_grace_ms`)
bir ürün kararıdır.

## 5. Senaryo analizi

**A'nın parladığı yerler.**

- **S6, S8, S14 (kalıcı bar farkı, kesinleşme yarışı, bozuk otorite):** N burada kesin yanlış
  (S8: 330), kaçan (S8: 1045; S14: 60) ve çok uzun çelişki üretiyor. A'da otoritenin kesin
  dediği her bar kaynağın barıdır (boşluksuz besleme ya da kesinleşmiş kaynak) ve yansı
  ancak aynı bitleri tutuyorsa gösterir. Bedeli gecikme: yansının barı farklıysa 2,8 s
  (S6, S8 p50), otoritenin barı kirliyse 2,85 s (S14).
- **S7 (60 tik/s, bar içi salınım):** canlı barda hiçbir şey iddia edilmediği için N'nin 7560
  geçici yanlışı ve 196 yanıp sönmesi tamamen yok; kesin iddialar 119 ms'de. Bayt N'nin
  1/400'ü.
- **S9, S10 (düzeltme, bölünme):** RESEARCH'ün A için öngördüğü "önek kırılganlığı" ve
  "bölünmede yanlış engelleme" bu düzenekte görülmedi (M8 toplam 1,9 s ve 0,7 s, 5 s'yi aşan
  bölüm 0). Sebep: iki taraf da düzeltmeyi ve bölünmeyi kendi beslemesinden alır, dönem artar,
  yeni konum hemen yayılır ve gösterilen iddialar yapışkanlıkla kalır. M11 p50 98 ms
  (N: 85 ms). Önek katılığının gerçek bedeli yansının bir düzeltmeyi **kaçırdığı** durumda
  çıkar (düzenekte düzeltmeler kaybolmaz); o zaman yansı iki aşamalı çekmeyle uzlaşır
  (en kötü durumda bütün geçmiş).
- **S12, S12s (farklı çapa):** N'nin 7425/8074 erken gösterimi yok; yansı geçmişini
  otoritenin çapasına uzatır (bir tam çekme, `M10` iş/olay 14,9; duvar 30 µs/olay). Bedel
  başlangıçta ~0,8 s'lik toplu engelleme (M8 toplam 566 s / 100 tohum, bölüm başına < 5 s).
- **S15, S15k (f32):** bit eşitliğinin hiç olmadığı kaynakta A, "sürekli uyuşmazlık"
  (RESEARCH tahmini) yerine kesinleşmiş f64 barları kendi kaynağından çekerek **uzlaşır**:
  hata 0, uyuşmazlık süresi 0, gecikme 2,8 s, her bar bir REST çağrısı. N S15k'da 1489 çelişki.
- **S4, S13 (kanal sırası, yineleme):** "en yeni kazanır" ile hata 0; S4'te gecikme kuyruğu
  yalnız 30 s'lik kopmadan (p99 30 098 ms).
- **S16 (iki bağ):** konum vektörü: iki bağlı iddia geride kalan bağın konumu gelince
  (5,07 s) gösterilir; N'de 2778 erken gösterim.
- **S17 (50 bağ, ~1000 tik/s):** 190 B/s (N 3927), iş/olay 12,6 (N 12,4), M6k p50 114 ms.

**A'nın zorlandığı yerler.**

- **Canlı bar:** hiç iddia yok (M6g yok). 1 dakikalık barda kesin iddia kapanıştan ~0,1 s
  sonra geldiği için M5 etkilenmez; ama ürün "RSI şu an 70'in üstünde" diyemez — yalnız
  `Pending`. Günlük barda bu gün sonu demektir.
- **S2c (yansı 60 s geride):** 149/1202 kaçan. Canlı iddiaların barı yansıda 60 s sonra
  görünür; M1 = 0 kuralı (görünen veride kanıt yoksa gösterme) bunu zorunlu kılar. Başlangıç
  geçmişini kendi kaynağından çekerek (yetişme) geçmiş iddiaları kurtardı; canlı bar `W`'nin
  iddiası `W+1` görünmeden kesin gösterilemez ve `W+1` kesinleşmiş kaynakta da bir bar sonra
  vardır, yani REST ile yetişme Δ_miss = 30 s içinde yardım etmez.
- **S11, S19 (yansı beslemesi 30 s / 2 dk kesik):** 32 ve 53 kaçan; en kötü örnek (S11 tohum
  1) gerçekte 241,0 s'de kesinleşen bir iddia, yansının `REFRESH`'i 270,0 s'de gelince
  271,1 s'de gösterildi (30,1 s > Δ_miss). Yine görünen veri yokken gösterememe.
- **S3 (kanal 3–4 s):** M6k 3,6 s; engelleme toplamı yüksek (4188 s / 100 tohum) ama 5 s'yi
  aşan bölüm yok. A otoritenin sözü olmadan gösteremez.
- **S4 (protokol 30 s kopuk):** 2 kaçan, 2 uzun engelleme (31 s, 34 s). Yerel yedek yok.
- **Besleme sessizliği:** S6 tohum 41'de bir iddia 11,6 s'de gösterildi: yansının beslemesi
  `W+1`'in ilk tiklerini kaybetti, yansı `W`'nin kapandığını 11 s öğrenemedi (M1 kuralı:
  `Final` için bar görünen veride kapanmış olmalı).
- **S9'daki tek "çözülmeyen" M11:** canlı bar 6'daki düzeltme canlı barda geçici bir iddia
  doğurdu (N onu `Provisional` gösterdi); A canlı barda iddia etmediği ve iddia bar kapanmadan
  kayboldu diye ekran hiçbir zaman "gösteriliyor" durumuna geçmedi. Düzeneğin M11 tanımına
  göre çözülmemiş sayılır; hata değil.
- **M7 (S9: 20 s / 100 tohum):** düzeltme gerçeği çevirdikten sonra geri çekmenin yansıya
  ulaşmasına kadar (~100 ms × düzeltme sayısı). Hiçbir bölüm Δ_stale'i aşmadı.

## 6. Maliyetler

- **Kanal:** konum mesajı ~70 B (tek bağ), bar başına iki (`s=0`, `s=1`); iddia başına 22 B;
  ipucu mesajı ~20 B. Tek bağda 1,5–5,5 B/s ham, 0,35–0,96 B/s akış-deflate. 50 bağda
  190,6 B/s ham, 31 B/s akış-deflate. B/iddia sütunu (150–770) küçük iddia sayısına bölünmüş
  sabit konum trafiğidir.
- **Yansı işi:** `M10` 2,2–18,7 iş/olay (N 1,8–12,4). Fazlası: önek zinciri (kapanan bar başına
  bir bar özeti), anahtar değerlendirme, uyuşmazlıkta kesinleşmiş barların uygulanması (f32:
  her bar). Duvar saati 0,9–30 µs/olay (belirleyici değil; S12'nin 30 µs'si başlangıçtaki
  2700 barlık uzatmadan).
- **Kesinleşmiş kaynak:** otorite bağ başına bar başına bir REST çağrısı (1 dk barda 1000
  sembol için ~17 istek/s — ürün maliyeti, ölçülmedi); yansı yalnız uyuşmazlıkta.
- **Otorite durumu:** bağ başına özet önbelleği (bar başına 8 B), kirlilik kümesi.

## 7. Sınırlar ve varsayımlar

1. **Besleme sıra numarası varsayımı.** Erken onay (`s=0`) beslemenin `seq` taşımasına ve
   kaybın boşluk bırakmasına dayanır. Matriks akışında bu belgelenmemiş (RESEARCH S4). Yoksa
   her kesin iddia kesinleşmiş kaynağı bekler: ölçülen karşılığı S14/S6/S15 satırları, M6k
   ≈ 2,8 s. Düzenekte bozuk fiyat (S14) boşluk bırakmaz; güvenlik o zaman yansının bağımsız
   beslemesinin aynı bozukluğu taşımamasından gelir (iki uçta aynı bozukluk ihtimali sıfır
   varsayıldı).
2. **Kesinleşmiş kaynağa bağımlılık.** Doğruluk (M3k = 0) sağlayıcının kesinleşmiş barına
   dayanır; o kaynak yanlışsa A da yanlıştır. Kesinleşme gecikmesi (`SETTLE_GUESS_MS` = 2 s)
   bir tahmindir; yanlışsa yalnız gecikme değişir (kısa cevap 250 ms sonra yeniden istenir).
3. **Bar takvimi.** Yansı ve otorite bir barın kapanış anını bar zamanından hesaplar
   (`LIVE0`, `BASE_T`; üründe duvar saati). Bu yalnız çekme zamanlamasını etkiler.
4. **Çapa uzatma bağın penceresini aşar.** S12'de yansı, bağı "son 300 bar" dese de 3000 bar
   tutar. A'nın önek özeti çapa eşitliği ister; bağ çapayı sabitlemezse (RESEARCH §1.2/5)
   ya pencere aşılır ya iddia gösterilmez. Yansı otoriteden **uzun** geçmiş tutuyorsa
   uzlaşma yok (`Disputed`; düzenekte bu durum yok).
5. **Verisi olmayan istemci** (ses, ajan): A'da gösterilecek bir şey yok; değer taşınmaz.
6. **Özet 64 bit FNV:** kaza ile çakışmaya karşı yeterli, kasıtlı değiştirmeye karşı değil.
7. **Merkle bölmesi yok:** uzun geçmişte eski bir barı kaçıran yansı bütün geçmişi çeker
   (düzenekte yalnız S12'nin çapa uzatmasında oldu).
8. **Görüntü örneklemesi:** düzenek ekranı sanal anların sonunda okur; A'nın gizleme ve
   gösterme kararları aynı geri çağrı içinde verildiği için bu ona avantaj ya da dezavantaj
   sağlamaz.

## 8. Belirleyicilik

- `harness determinism --approach a --scenario S7 --scenario S9 --scenario S11 --scenario S15 --seeds 3`:
  12/12 aynı (ör. S9 tohum 0: 86 satır, `6ec2fd340062084d`; S11 tohum 1: 180 satır,
  `0da2c3bf65b97672`; S15 tohum 0: 120 satır, `9b170b7245890e77`).
- S8 + S9, 100'er tohum, iki kez (`--threads 4` ve `--threads 8`): 200 koşunun bütün ölçüleri
  (duvar saati hariç) ve günlük özetleri aynı; birleşik özet `400590f42b4a642e`.
- Yaklaşım saat, rastgelelik, ortam okumaz; yalnız `BTreeMap`/`BTreeSet`.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` temiz
  (`a.rs` içinde 3 birim testi: özet zinciri = `data_digest`, anahtar kodlayıcısı bütün
  dedektörlerde gidiş-dönüş, S1/S6/S9/S15'te 3'er tohum hata 0).

## 9. Düzenek hakkında notlar (değiştirilmedi)

Engelleyen bir hata bulunmadı. Gözlemler:

- `Engine::apply_settled` her bar için ayrı `upsert` yaptığı için eski barlar eklenince her
  biri baştan yeniden hesap tetikler (S12'de 2700 bar → ilk sürümde `M10` 2255 iş/olay).
  A bunu kendi verisini tek anlık görüntü olarak motora vererek aştı; motor değiştirilmedi.
- M11 "çözülmeyen", geçici iddiayı hiç göstermeyen bir yaklaşımda canlı bardaki ve kapanmadan
  kaybolan iddiaları da sayar (§5).
- `harness run --out` verilmezse sonuç dosyası crate klasörüne yazılıyor
  (`harness-results-*.json`, `.gitignore`'da).

## Öneriler (kapsam dışı; yapılmadı)

- Ürün: kesin iddiada erken onay için Matriks akışında sıra numarası olup olmadığı R6'da
  netleşmeli; yoksa A'nın kesin gecikmesi ≈ kesinleşme gecikmesi + REST.
- Uzun geçmişli bağlarda E'nin Merkle bölmesi, kaçırılmış eski düzeltmeyi O(log n) bulur;
  A'ya eklenmeli ve bir "yansı düzeltmeyi kaçırdı" senaryosuyla ölçülmeli (düzenekte
  düzeltmeler kaybolmuyor; böyle bir senaryo yok).
- Düzenek: "yansı beslemesi bir düzeltmeyi kaybeder" ve "otorite ile yansı farklı uzunlukta
  geçmiş tutar (otorite kısa)" senaryoları A'nın gerçek önek katılığını gösterir.
- Bağ (K23) hesap başlangıcını sabitlemeli; aksi halde A ya pencereyi aşar ya göstermez.
