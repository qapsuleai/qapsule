# RP2/T7d — Yaklaşım D: kanıt zarfı + görüntü durumları (E destekli)

Tarih: 2026-11-15. Dal: `rp2/t7d-approach`. Kod: `rounds/RP2/experiments/harness/src/approaches/d.rs`
(yapılandırma, kayıt, ablasyon ve tanı testleri), `approaches/d/wire.rs` (mesajlar),
`approaches/d/authority.rs`, `approaches/d/replica.rs`. Düzenek (`HARNESS.md`) değiştirilmedi;
yalnız `approaches/mod.rs` kayıt satırı `approaches!(n, d);` oldu. Diğer T7 dallarına bakılmadı.

Bu belge D'yi savunucusunun kuracağı en iyi haliyle anlatır, ölçer ve zayıf yerlerini
söyler. Bütün sayılar bu dalda, düzeneğin varsayılan tohumlarıyla (100; S17: 20) ölçüldü.

## 0. Özet

- **M1 = M2 = M3k = 0 bütün senaryolarda** (S0–S19, 2320 koşu). Yansı beslemesi geride
  kaldığında (S2a/S2/S2c, S11, S19, S16) yanlış ya da erken bulgu yok (GOAL ölçüt 8).
- **M3g (geçici etiket, o anki gerçek canlı barda yanlış): 2320 koşuda 2** (S9 tohum 85,
  S11 tohum 27). İkisi de gerçeğin, iki tarafın da elindeki son bilgiden *sonra* tek bir
  tikle dönmesi (6 ms ve 56 ms önce). Geçici iddia gösteren hiçbir yaklaşım bunu kesin
  sıfırlayamaz (§5.3); oran, geçici koruma payıyla (`prov_margin`) seçilir.
- **M5 (kaçan) = 0 her senaryoda**, S2c (yansı 60 s geride) ve S19 (2 dk kesinti) dahil.
- Bedel **gecikmede**: kesin etiket sağlayıcının kesinleşmiş barına karşı onaylandığı için
  M6k p50 ≈ 2,9 s (kesinleşme gecikmesi 2 s + REST 0,8 s). Geçici koruma payı 0,75 RSI
  puanı olduğundan M6g p50 ≈ 6 s (payı olmayan `d_m0`: 116 ms, ama M3g 2320 koşuda 5 972).
- Bayt: a→r ham 4,2 B/s (S0; N 3,1), S7'de 171 B/s (N 2 245), S17'de 986 B/s (N 3 927).
- Uyumsuz bir karışım (X1, §5.4) altında da M1 = M2 = M3k = 0, M3g 7/100.

## 1. Tasarım

### 1.1 Kanıt zarfı

Her olgu bağ başına bir **zarf** taşır. Zarf `qp_schema::data::EvidenceRef`'in bütün
alanlarını içerir (`series`, `from_t`, `to_t`, `bars`, `alg`, `digest`) ve canlı tutarlılık
için beş alan ekler:

| Alan | Kaynak | Anlam |
|---|---|---|
| `series` | EvidenceRef | bar serisi (bağ) |
| `anchor_t` | **yeni** | hesap başlangıcı: otoritenin bar 0'ının zamanı (PROTOCOL §18.3) |
| `from_t`, `to_t` | EvidenceRef (anlamı farklı, §9) | **kanıt penceresi**: iddia barı ve önceki `period` bar (geçersizlikte `lookback + period`) |
| `bars` | EvidenceRef | penceredeki bar sayısı (otoritede) |
| `alg`, `digest` | EvidenceRef | pencerenin veri özeti (FNV-1a 64); **yalnız kesinde** |
| `prefix` | **yeni, isteğe bağlı** | `anchor_t..to_t` özeti (yalnız "tam taahhüt" kipi, `d_prefix`) |
| `rev` | **yeni** | otoritenin o serideki son kaynak `seq`'i (veri konumu / canlı sürüm) |
| `epoch` | **yeni** | otoritenin veri dönemi: kesinleşmiş bir bar değişince +1 (düzeltme, bölünme) |
| `engine` | **yeni** | motor kimliği (`qp-compute/0.0.0`) |

Olgunun kendisi: `id` (otorite atar, anahtar başına kalıcı), `key` (anlam: dedektör,
parametre, bağ, bar zamanı, yön), `m` (olgunluk), `v` (kanıt değerleri: `prev`, `value`,
`level`; çiftte `a`, `b`, `level`).

Gerçek bir `fact_final` (S1 tohum 0, düzenekten basılan mesajdan; 405 B):

```json
{"op":"fact_final","id":2,
 "key":{"detector":"rsi_cross","params":"period=14,level=70","binding":"px0",
        "bar_t":1699981680000,"direction":"up"},
 "v":{"level":70.0,"prev":67.80989755942178,"value":71.55916021146291},
 "ev":[{"series":"px0","anchor_t":1699976040000,"from_t":1699980840000,"to_t":1699981680000,
        "bars":15,"alg":"fnv1a64","digest":"3315fc4a24a0cbe5",
        "rev":0,"epoch":0,"engine":"qp-compute/0.0.0"}]}
```

Geçici olgu aynı biçimdedir, `"op":"fact","m":"provisional"` ve `digest`'siz.

### 1.2 Olaylar

Otorite → yansı mesajı bir **toplu**dur: `{"s": <otorite sıra no>, "ops": [...]}`; bir
toplu bir iddiaya en çok bir işlem içerir. Yansı bir iddiaya bir işlemi yalnız toplunun `s`'i
o iddiaya son uyguladığından büyükse uygular → sıra bozulması ve yineleme zararsız (S4, S13).

| İşlem | Ne zaman | İçerik |
|---|---|---|
| `hello` | bağ ilk çözülünce | motor, `(seri, anchor_t)` |
| `fact` | iddia otoritede **yeni** tutmaya başladı (geçici) | anahtar, `m:"provisional"`, değerler, zarf (özetsiz) |
| `fact_final` | iddianın dayandığı **bütün** barlar sağlayıcının kesinleşmiş barına karşı onaylandı; ya da kesin zarfı değişti (düzeltme/bölünme: yeni özet, yeni dönem) | anahtar, değerler, özetli zarf |
| `fact_retracted` | iddia otoritede artık tutmuyor | `id`, neden |
| `data_mark` (E) | bar onaylanınca; düzeltmede; ilk çözümde | `(bar t, o barın özeti)` listesi ve tamamlanmış 32 barlık parçaların `(parça no, özet)` listesi |

Yansı → otorite: `data_dispute {id, what}` (yansı kendi kaynağından yeniden çektikten sonra
da kesin bir iddiayı doğrulayamazsa; yalnız bilgi, dokümanı değiştirmez).

Kanalda **piyasa verisi yoktur** (K23): yalnız iddia, değer, aralık ve özet. Yansının bütün
verisi kendi beslemesinden ve kendi `fetch_settled`'ından gelir.

### 1.3 Otorite

1. Paylaşılan motoru kendi beslemesinde çalıştırır.
2. **Kesinleşme onayı (T3).** Yerelde bir bar kapanınca (sonraki bar gelince) sağlayıcının
   kesinleşme gecikmesi kadar (2 s + 1 ms) sonra o barı sağlayıcıdan (`fetch_settled`) ister,
   `apply_settled` ile uygular ve `verified_through`'u ilerletir. Yalnız bütün barları
   onaylanmış iddia `fact_final` olur. Cevap geldiğinde istekten sonra beslemenin yazdığı bar
   (daha yeni bir düzeltme) korunur.
3. **Geçici kapısı.** Yeni bir geçici iddia, dayandığı kapanmış barlar (kendi barı hariç
   hepsi) onaylanmadan söylenmez — "geçici" yalnız canlı barın onaysız olduğu anlamına gelir.
4. Geçici iddianın canlı değerleri akıtılmaz (yansı kendi değerlerini gösterir); yalnız
   tutmaya başlaması/bırakması söylenir.
5. Kesinleşmiş bir bar beslemeden değişirse (sağlayıcı düzeltmesi) dönem +1, o bar için
   `data_mark`, verisi o bara ulaşan kesin iddialar için yeni `fact_final`.
6. Bölünme anlık görüntüsünde dönem +1, bütün parçalar yeniden işaretlenir, bütün kesin
   zarflar yeniden hesaplanır.

### 1.4 Yansının durum makinesi

```
 otorite tutuyor ─► kanıt penceresi bende yok ─────────────────────► Pending (iddia etmez)
                └─► pencere bende ─► yüklem bende yanlış ─┬─ geçici ─► Pending
                                                          └─ kesin ─► kendi kaynağımdan
                                                             yeniden çek (≤ 2 deneme) ─► Disputed
                                   └─► yüklem bende doğru ─┬─ geçici (ya da benim barım açık)
                                                           │     ─► [koruma] ─► Provisional
                                                           └─ kesin ─► pencere özeti + çapa
                                                              + değerler eşit ─► Final
                                                              değilse yeniden çek ─► Final
                                                              | SourceDiffers (kendi değerlerimle)
 otorite geri çekti ─► gösterilmez
```

Kurallar:

- **Erken = 0:** "pencere bende" kuralı hakemin M1 tanımıyla aynıdır (iddia barı ve öncesinde
  en az `period` bar; kesin için bar kapanmış).
- **Çelişkili = 0:** iddia eden bir etiket yalnız iddia yansının **kendi görünen verisinde**
  tutarken verilir; yansının verisindeki her değişiklik dokunduğu iddiaları **aynı geri
  çağrıda** yeniden değerlendirir (otoritenin `fact_retracted`'ını beklemeden).
- **Yanlış kesin = 0:** `Final` yalnız `fact_final` ile; `fact_final` yalnız sağlayıcının
  kesinleşmiş barına karşı onaydan sonra.
- **Tek otorite:** otoritenin söylemediği iddia gösterilmez (C'den farkı).
- **Geçici koruma (histerezis):** canlı barda bir iddia ilk kez (ya da gizlendikten sonra
  yeniden) gösterilecekse yüklem yansının verisinde `prov_margin` = 0,75 RSI puanı payla
  tutmalıdır (yukarı kesişimde `value − level ≥ 0,75`). Gösterildikten sonra pay aranmaz;
  yüklem tuttukça kalır.
- **Kesin, ama benim barım henüz açık** (yansı geride): `Provisional` (iddia gerçekte kesin,
  yansının grafiğinde bar henüz kapanmadı). Bu, S2c/S19'da kaçan iddiayı önler.
- **Ödünç süre:** zaten `Final` gösterilen bir iddiada özet farkı belirirse (öbür tarafın
  güncellemesi yolda) en çok 2 s `Final` kalır; sonra yeniden çekme yolu.

### 1.5 Yansının kendi kaynağından çekme (K23 içinde)

Bütün onarımlar `ctx.fetch_settled` ile, yansının **kendi** sağlayıcısından (RESEARCH S2);
protokolden veri gelmez.

| Mekanizma | Tetik | Ne yapar |
|---|---|---|
| **İşaret karşılaştırması (E)** | `data_mark` geldi / kendi barım kapandı / düzeltme / anlık görüntü (300 ms birleştirme) | kendi kapanmış barımın (ya da 32 barlık parçamın) özeti işaretten farklıysa yalnız o barı/parçayı yeniden çeker (bar başına ≤ 2 deneme) |
| **İddia onarımı** | kesin iddiada pencere özeti / değer farkı ya da yüklem yanlış | 1. deneme pencere (E varsa değer farkında E'yi bekler), 2. deneme bütün geçmiş; o bağda başka çekme sürüyorsa deneme sayılmaz; deneme başına ≤ 3 s |
| **Yetişme (catch-up)** | otorite bir barı onayladı, benim beslemem o barı henüz kapatmadı (ya da hiç çözmedi) | aradaki kapanmış barları kendi kesinleşmiş kaynağından çeker; beslemenin o barlar için geç gelen *canlı* tikleri (besleme zamanında son bar) atlanır, düzeltmeler (daha eski bara yama) uygulanır |
| **Çapa hizalama** | zarfın `anchor_t`'si benim ilk barımdan farklı | eksik eski barları kendi kaynağından çekip başa ekler (tek anlık görüntü olarak motora verilir) ya da fazla eskileri atar → hesap başlangıcı otoriteninkiyle aynı (PROTOCOL §18.3) |

Bir yeniden çekme cevabı, istekten sonra beslemenin getirdiği bir düzeltmeyi ezmez (yansı
tarafında da, otorite gibi).

## 2. RESEARCH §3.D'den sapmalar

Her sapma ölçümden doğdu; ablasyonda ilgili değişken ayrıca ölçüldü (§4).

| # | Sapma | Neden (ölçüm) |
|---|---|---|
| 1 | **Geçici koruma payı 0,75** (RESEARCH: "histerezis ürün kararı, ölçülmeli") | Paysız (`d_m0`) M3g = 5 972 / 2 320 koşu (S7: 5 823). Kalanların hepsi gerçeğin son ~100 ms içinde dönmesi (bilgi gecikmesi penceresi). Bekleme süresi (300 ms) etkisiz çıktı (S7: 28, S19: 26); pay etkili. Eğri: 0 → 5 972, 0,5 → 45, 0,75 → 2, 1,0 → 2, 1,5 → 0. Bedel M6g. |
| 2 | **Geçici kapısı** (otorite, dayandığı kapanmış barlar onaylanmadan yeni geçici söylemez) | S8'de iki besleme de son tiki kaçırınca bir sonraki barın açılışında gerçekte olmayan bir kesişim iki tarafta da "doğru" görünüyordu: kapısız M3g = 78 (S8). Kapıyla 0. Bedel: bar başındaki ~2,9 s içinde doğan geçici iddialar gecikir. |
| 3 | **Kapanmış ama onaysız bar = geçici** | RESEARCH'te geçici = canlı bar. Kesin etiket onayı (~2,9 s) beklerken iddia gizlenmesin diye geçici etiket bar kapandıktan sonra onaya kadar sürer (hakem bunu geçici kuralıyla yargılar: gerçekte tutuyor mu). |
| 4 | **Değer eşitliği de denetlenir** (Final için) | Pencere özeti eşit ama değer farklıysa fark pencerenin dışındadır (RSI ≈ 500 bar geriye bağlı). RESEARCH bunu "bilerek kabul" ediyordu; değeri karşılaştırmak bedava ve farkı yakalıyor → `SourceDiffers` ya da onarım. |
| 5 | **Çapa hizalama** | Zarfın `anchor_t`'si zaten vardı; yansı onu kullanarak kendi geçmişini otoritenin çapasından yeniden çözer. Hizalamasız (`d_noanchor`): S12 M5 = 7 500, S12s 8 196, M13 milyonlarca saniye. |
| 6 | **Yetişme (catch-up)** ve "kesin ama benim barım açık → Provisional" | Yansının ilk anlık görüntüsü bile 60 s geç gelebiliyor (S2c): yetişmesiz M5 = 1 130 / 1 202 (geçmiş iddiaların hepsi). S19 (2 dk kesinti): 32. |
| 7 | **E iki seviyeli** (bar özeti + 32 barlık parça özeti), Merkle ikiye bölme sorgusu yok | Parça farkında yansı 32 barı doğrudan kendi kaynağından çeker; bir tur gidiş-dönüş (40–70 ms × log₂n) bir REST isteğinden pahalı. Kanal mesaj kaybetmediği için düzenekte bölme sorgusunun ölçülecek bir faydası yoktu (§7). |
| 8 | **`fact_final` yeniden yayını** düzeltme/bölünmede | Dondurulmuş özet değişince yansının "aynı veri" denetimi eski özetle yanlış fark bulmasın diye. |
| 9 | Canlı değerler akıtılmaz | Yansı kendi değerlerini gösterir; verisiz istemci için değerler `fact`/`fact_final`'dadır (S7'de bayt N'nin 1/13'ü). |

## 3. Sonuçlar (D, bütün senaryolar)

Komut: `harness run --approach d --scenario all` (100 tohum; S17 20). N aynı makinede yeniden
koşuldu (`HARNESS.md` §8 ile aynı sayılar). Hata sütunu M1/M2/M3k/M3g. M6 ms; M7 tohum başına
s; M8 = 5 s'den uzun bölüm (tohum başına toplam s); M9 a→r: ham B/s, akış-deflate B/s,
ham B/gerçek iddia; M10 yansı iş birimi/geri çağrı. r→a bütün senaryolarda ≈ 0 B/s
(`data_dispute` yalnız X1'de görüldü).

| Sen. | Tohum | N hata | **D hata** | M5 | M6g p50 / p99 | M6k p50 / p99 | M8 (s/tohum) | M7 s | M13 ms | M12 top/max | N B/s | D B/s | D akış-defl. | D B/iddia | M10 | en kötü tohum |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| S0 | 100 | 1057/1306/0/2 | **0/0/0/0** | 0/1202 | 5938 / 52124 | 2885 / 2907 | 0 (0,0) | 0,05 | 0 | 1316/5 | 3,1 | 4,2 | 0,86 | 1265 | 3,4 | – |
| S1 | 100 | 0/0/0/0 | **0/0/0/0** | 0/1202 | 5816 / 51993 | 2801 / 2801 | 0 (0,0) | 0,00 | 0 | 1316/5 | 3,1 | 4,2 | 0,86 | 1265 | 3,1 | – |
| S2a | 100 | 1212/1453/0/2 | **0/0/0/0** | 0/1202 | 6334 / 52514 | 2885 / 2907 | 0 (0,0) | 0,05 | 0 | 1318/6 | 3,1 | 4,2 | 0,86 | 1265 | 3,5 | – |
| S2 | 100 | 1239/1466/0/2 | **0/0/0/0** | 0/1202 | 11294 / 56716 | 5130 / 5248 | 0 (0,0) | 0,03 | 0 | 1300/7 | 3,1 | 4,2 | 0,86 | 1265 | 3,6 | – |
| S2c | 100 | 1600/1551/0/2 | **0/0/0/0** | 0/1202 | 36035 / 62893 | 61670 / 62974 | 0 (0,0) | 0,00 | 0 | 1202/1 | 3,1 | 4,2 | 0,86 | 1265 | 2,5 | – |
| S3 | 100 | 0/241/0/89 | **0/0/0/0** | 0/1202 | 6645 / 53617 | 6294 / 6822 | 0 (37,0) | 0,01 | 0 | 1334/9 | 3,1 | 4,2 | 0,86 | 1265 | 3,1 | – |
| S4 | 100 | 64/319/0/15 | **0/0/0/0** | 0/1202 | 6260 / 52124 | 3087 / 30334 | 0 (3,4) | 0,06 | 0 | 1314/5 | 3,1 | 4,2 | 0,86 | 1265 | 3,1 | – |
| S5 | 100 | 1093/1569/0/13 | **0/0/0/0** | 0/1215 | 5958 / 47333 | 2888 / 2908 | 0 (0,0) | 0,09 | 0 | 1443/7 | 6,2 | 4,6 | 0,89 | 1359 | 3,3 | – |
| S6 | 100 | 2691/2770/0/0 | **0/0/0/0** | 0/3028 | 111 / 139 | 3679 / 3708 | 0 (0,0) | 0,00 | 0 | 3028/1 | 4,3 | 8,0 | 1,20 | 957 | 17,3 | – |
| S7 | 100 | 1175/16988/0/7560 | **0/0/0/0** | 0/1232 | 1814 / 19946 | 2887 / 2907 | 0 (0,0) | 0,41 | 0 | 3716/15 | 2244,6 | 171,0 | 9,05 | 6662 | 5,6 | – |
| S8 | 100 | 2881/3206/330/1906 | **0/0/0/0** | 0/3028 | 113 / 139 | 2891 / 3703 | 0 (9,7) | 0,00 | 0 | 3028/1 | 20,9 | 7,5 | 1,18 | 895 | 11,2 | – |
| S9 | 100 | 2925/3467/0/6 | **0/0/0/1** | 0/3467 | 5492 / 47643 | 112 / 2903 | 0 (0,0) | 0,22 | 0 | 4005/4 | 5,6 | 10,6 | 1,53 | 1030 | 12,7 | 85 (1) |
| S10 | 100 | 1058/1360/0/4 | **0/0/0/0** | 0/1206 | 6071 / 55953 | 2886 / 2906 | 0 (0,0) | 0,05 | 0 | 1330/6 | 3,3 | 6,2 | 1,17 | 1837 | 4,8 | – |
| S11 | 100 | 2936/8740/0/43 | **0/0/0/1** | 0/2559 | 2891 / 42977 | 2886 / 30109 | 0 (0,0) | 3,02 | 0 | 9777/7 | 28,0 | 15,4 | 1,65 | 2169 | 7,2 | 27 (1) |
| S12 | 100 | 7425/7500/0/0 | **0/0/0/0** | 0/9674 | 15279 / 56429 | 2801 / 2801 | 0 (0,0) | 0,00 | 0 | 13716/7 | 30,3 | 22,1 | 2,80 | 822 | 11,7 | – |
| S12s | 100 | 8074/9074/0/0 | **0/0/0/0** | 0/9674 | 15279 / 56429 | 2801 / 2801 | 0 (0,0) | 0,00 | 0 | 13716/7 | 30,3 | 22,1 | 2,80 | 822 | 11,2 | – |
| S13 | 100 | 1060/1310/0/18 | **0/0/0/0** | 0/1202 | 5947 / 52104 | 2887 / 2910 | 0 (0,0) | 0,05 | 0 | 1316/5 | 3,1 | 4,2 | 0,86 | 1265 | 3,2 | – |
| S14 | 100 | 1050/1439/39/389 | **0/0/0/0** | 0/1202 | 6232 / 52124 | 2885 / 4614 | 0 (0,3) | 0,05 | 0 | 1346/5 | 2,9 | 4,6 | 0,91 | 1379 | 3,5 | – |
| S15 | 100 | 0/0/0/0 | **0/0/0/0** | 0/2559 | 19628 / 60000 | 3601 / 3601 | 0 (0,0) | 0,00 | 0 | 4047/7 | 20,4 | 12,1 | 1,45 | 1701 | 90,5 | – |
| S15k | 100 | 0/1489/0/0 | **0/0/0/0** | 0/2568 | 26876 / 57727 | 3601 / 3601 | 0 (0,0) | 0,00 | 0 | 2568/1 | 20,3 | 7,0 | 1,10 | 981 | 96,0 | – |
| S16 | 100 | 2778/3423/0/9 | **0/0/0/0** | 0/2767 | 6131 / 44074 | 5074 / 5247 | 0 (0,0) | 0,47 | 0 | 3983/13 | 13,1 | 10,0 | 1,71 | 1305 | 4,7 | – |
| S17 | 20 | 5879/8189/0/628 | **0/0/0/0** | 0/8025 | 5261 / 54502 | 2898 / 2909 | 0 (1,0) | 0,58 | 0 | 8355/8 | 3926,5 | 985,5 | 176,55 | 884 | 13,0 | – |
| S18 | 100 | 3098/6923/0/18 | **0/0/0/0** | 0/3097 | 2879 / 30734 | 2886 / 2907 | 0 (0,0) | 2,27 | 0 | 8515/11 | 27,3 | 13,5 | 1,59 | 1574 | 9,0 | – |
| S19 | 100 | 3154/8770/0/35 | **0/0/0/0** | 0/2559 | 2893 / 59563 | 2886 / 63685 | 0 (0,0) | 2,96 | 0 | 9623/7 | 28,0 | 15,4 | 1,65 | 2169 | 7,3 | – |

M4 (geri çekilen geçici, hata değil): S0 57, S7 1 242, S11 3 609, S18 2 709, S19 3 532.
M11 (S9): p50 98 ms, çözülmeyen 0, gereksiz 0. Canlı faz hatası: S9 1, S11 1, diğerleri 0.

## 4. Ablasyon

Aynı kod, farklı yapılandırma (`approaches/d.rs` `Cfg`); 100 tohum, S17 hariç bütün
senaryolar. Yalnız D'den farklı çıkan hücreler (önce D → sonra varyant):

| Varyant | Ne eksik | Değişen (senaryo: D → varyant) |
|---|---|---|
| `d_noe` | E (veri işareti) yok | Hata yok. **Yetişme işarete dayandığı için**: S2c M5 0 → 1 130, S19 0 → 32. S8 M6k p50 2 891 → 3 672 ms (fark yalnız `fact_final`'da görülüp onarılıyor). Bayt S0 4,2 → 1,9 B/s (işaretler baytın yarısı). M10 biraz düşük. |
| `d_nodigest` | özet ve işaret yok (B-yüklem + olgunluk) | **Hata yok** (M1–M3 özetten bağımsız). S2c/S19 kaçan (işaret yok → yetişme yok). `Final` artık "aynı veri" demiyor: S15'te f32 geçmişle de `Final` (M6k 3 601 → 2 801 ms, çünkü onarım beklenmiyor). Bayt −55 %. M10 S15'te 90 → 3 (geçmiş onarılmıyor). |
| `d_prefix` | pencere yerine önek özeti (tam taahhüt) | Hata/kaçan/etiket aynı (çapa hizalama + E sayesinde önekler eşleşiyor). **M10 ×4–60**: S12 11,7 → 736, S9 12,7 → 166, S6 17 → 160 (her kesin değerlendirmede 400–3000 bar özetleniyor). Bayt +2 %. |
| `d_nosettle` | sağlayıcıya karşı onay yok (kesin = otoritenin kendi kapanışı) | **S8: M3k 1, M3g 303, M5 1 228, M8 949 bölüm**; S6 M5 860; S14 M5 60, M8 60; S15k M5 748; M13 S8'de 2,2·10⁶ s. Kazanç: M6k p50 2 885 → 111 ms. |
| `d_nogate` | geçici kapısı yok | S8 M3g 0 → 78; M6g p50 hafif düşük (S12: 15,3 → 5,8 s). |
| `d_noanchor` | çapa hizalama yok | S12 M5 7 500, S12s 8 196 (yansının penceresi dışındaki geçmiş iddialar hiç gösterilmez), M13 5,5·10⁶ / 3,1·10⁶ s (`SourceDiffers`: değerler farklı). |
| `d_nocatchup` | yetişme yok | S2c M5 1 130, S19 32. |
| `d_norepair` | kendi kaynağından hiçbir çekme yok | S6 M5 1 794, S8 1 087, S15k 1 486, S12/S12s 7 500/8 196; M13 S6 3,6·10⁶ s, S15 6,7·10⁶ s; S9/S10'da 2 786 / 2 136 gereksiz M11 değişimi. Hata yok. |
| `d_m0` | geçici koruma yok | M3g toplam 5 972 (S7 5 823, S11 42, S19 41, S18 22, S5 12, S16 6, S9 5, S0 4, S4 4, S2a 3, S10 3, S13 3, S14 3, S2 1); M6g p50 ≈ 0,1 s; M12 en çok 192. |
| `d_m05` | pay 0,5 | M3g 45 (S7 30, S19 6, S11 5, S18 2, S2a 1, S16 1); M6g p50 ≈ 3,5 s. |
| `d_m10` | pay 1,0 | M3g 2 (D ile aynı iki vaka); M6g p50 ≈ 9,8 s (S0). |
| `d_m15` | pay 1,5 | M3g 0; M6g p50 ≈ 16 s (S0), 22 s (S11/S19). |
| `d_w300` | pay yok, 300 ms bekleme | M3g 125 (S7 28, S19 26, S11 25, S5 15, S18 10, S16 9 …) — bekleme, pay kadar etkili değil. |

Okuma:

- **Hatayı kapatan parçalar:** sağlayıcıya karşı onay (M3k, M5, M8), geçici kapısı (M3g
  S8), yerel yüklem + aynı geri çağrıda yeniden değerlendirme (M1, M2), geçici payı (M3g
  yarış penceresi).
- **Kaçanı kapatan parçalar:** kendi kaynağından onarım, yetişme (işarete dayanır), çapa
  hizalama.
- **Özet ve E bu düzenekte M1–M3'ü değiştirmiyor**; `Final`'ın "aynı veri" (T2) anlamını,
  onarımın hedefini (yalnız farklı bar) ve yetişmeyi sağlıyor. Önek özeti aynı sonucu ×4–60
  işle veriyor; pencere + değer + E, önekin bütün işini yapıyor.

## 5. Senaryo bazında

### 5.1 Gecikme ve sıra (S0–S5, S13)

Yansı yalnız kendi verisinde tutan iddiayı gösterdiği için geride kalan yansı (S2a/S2/S2c)
erken ya da çelişkili gösteremez. S2'de yetişme kesinleşmiş barı yansının beslemesinden önce
getirir (M6k 5,1 s ≈ besleme gecikmesi; yetişme olmadan aynı, çünkü `Final` yansının barının
kapanmasını bekler). S2c'de geçmiş iddialar da yetişmeyle (yansının ilk anlık görüntüsü 60 s
gecikiyor) `Provisional` → `Final`; M5 = 0. S3'te (protokol 3–4 s) her şey otoritenin
sözünü bekler: M6k 6,3 s; başlangıçta geçmiş iddialar 3,9 s bekler (M8 süresi 37 s/tohum —
tek otorite ilkesinin bedeli, 5 s'yi aşan bölüm yok). S4/S13: toplu `s`'i ile iddia başına
son yazan kazanır; yineleme ve sıra bozulması etkisiz.

### 5.2 Besleme kaybı ve kapanış yarışı (S6, S8, S14)

S6: yansı kapanış tiklerinin yarısını kaybediyor. İşaret, yansının kapanmış barı
sağlayıcınınkinden farklıysa o barı kendi kaynağından çektirir (M10 17/olay, en yüksek
ikinci) → M5 = 0, M13 = 0. S8: iki besleme de son tiki %25 kaybediyor; otorite kesin demeden
önce sağlayıcıya sorduğu için M3k = 0 (onaysız `d_nosettle` M3k 1, M3g 303, M5 1 228);
geçici kapısı sonraki barın sahte kesişimini kesiyor. S14: otoritenin beslemesi bozuk;
geçici iddia ancak yansının (doğru) verisinde de tutarsa gösterildiği, kesin iddia
sağlayıcıya karşı onaylandığı için otoritenin yanlışı gösterilmez (N: 39 + 389).

### 5.3 Canlı bar titremesi (S7, S11, S18, S19) — geçici yanlışın sınırı

M3g tanımı katı (tolerans 0): geçici etiket gösterildiği anda gerçek canlı barda yüklem
tutmalı. Yansının gerçek hakkındaki son bilgisi en iyi ihtimalle
`min(yansı besleme gecikmesi, otorite besleme + kanal)` ≈ 60–140 ms eskidir; bu pencerede
gerçek dönerse hiçbir yaklaşım bunu bilemez. Paysız D'de S7 (60 tik/s, 70 ± 2,5) 5 823 M3g
verdi; tanı testi (`diag_m3p`) hepsinin gerçeğin gösterimden 6–130 ms önce döndüğü vakalar
olduğunu gösterdi. Pay 0,75 eşiği az aşan iddiaları canlı barda göstermez (kapanınca
onaylanıp `Final` olur, M5 = 0); kalan iki vaka tek tikte >1 RSI puanlık sıçrama. Yan etki:
M12 (yanıp sönme) S7'de 31 446 → 3 716, M7 (bayat) düşer; M6g yükselir.

S11/S19 (yansı beslemesi kesik): kesinti boyunca yetişme kesinleşmiş barları getirir ve
otoritenin kesin iddiaları `Provisional` görünür; geri gelişte `Final`. Kesintide canlı bar
yok → canlı geçici iddia gösterilmez.

S18 (bulgu + geçersizlik koşulu): geçersizlik iddiası da aynı kurala tabi; M1–M3 = 0.

### 5.4 Düzeltme, bölünme, çapa, kaynak farkı, çok bağ, yük

- **S9:** M11 p50 98 ms, çözülmeyen 0, gereksiz 0. Pencereye dokunmayan 1 000 bar geriki
  düzeltme yalnız değerleri değiştirir; yansının beslemesi de düzeltmeyi aldığı için değerler
  yine eşit → etiket değişmez.
- **S10 (bölünme ×0,5):** bütün özetler değişir, RSI değişmez; otorite yeni dönemle kesin
  zarfları yeniden yayınlar (B/s 3,3 → 6,2), yansı en çok 2 s ödünç süreyle `Final` kalır:
  M13 = 0, gereksiz değişim 0.
- **S12/S12s:** çapa hizalama yansının geçmişini otoritenin bar 0'ından yeniden çözer
  (~0,8 s); sonra bitler aynı. M5 = 0 (N'de M1 7 425).
- **S15/S15k (f32 yansı):** işaretler bütün geçmiş parçalarının farklı olduğunu söyler;
  yansı geçmişini kendi kesinleşmiş (f64) kaynağından yeniler → değerler bit-eşit, `Final`.
  Bedel M10 ≈ 90–96 iş/olay (motorun `apply_settled`'ı her eski bar için baştan hesaplıyor,
  §6) ve M6k 3,6 s.
- **S16 (iki bağ, biri 5 s geride):** iddia, iki bağın penceresi de yansıda olunca; M6k 5,1 s.
- **S17 (50 bağ, ~1 000 tik/s):** hata 0; a→r 986 B/s (N 3 927), M10 13 (N 12,4).
- **X1 (bu belge için yazılan uyumsuz karışım, `harness run --spec`):** S6 yansı kaybı + %20
  sıra bozma + %5 yineleme, otoritede %25 son tik kaybı, S4 kanalı (30 s kopma dahil),
  S9 düzeltmeleri, 1 200 geçmiş bar, 60 tik/bar. N: 1 145/4 242/417/3 002. **D: 0/0/0/7**,
  M5 0/4 894, M8 11 bölüm (hepsi 30 s'lik kanal kopması: otoritenin sözü gelmeden gösterilmez),
  M13 5,5 s/100 tohum (yansının kendi beslemesi iddiayı düşüren düzeltmeyi otoritenin geri
  çekmesinden önce getirince ~1,5 s `Disputed`). X1 bir onarım yarışını ortaya çıkardı ve
  düzeltildi: istekten önce gönderilmiş bir yeniden çekme cevabı yansının yeni aldığı
  düzeltmeyi eziyordu; ayrıca birleştirilen uzak aralıklar ve ilgisiz cevapların deneme
  bütçesini harcaması (M13 23,9 s → 5,5 s; S15'te geriye gidiş olmadı).

## 6. Maliyetler

- **Bayt (a→r).** Kesin olgu ~400 B (anahtar ~110, zarf ~200, değerler ~80); geçici aynı,
  özetsiz ~370 B; geri çekme ~45 B; bar işareti ~100 B (toplu başlığıyla). Saniyede: sakin
  senaryolarda 4–15 B/s (N 3–28), S7 171 B/s (N 2 245, çünkü canlı değer akıtılmıyor), S17
  986 B/s (N 3 927). Akış deflate S0 0,86 B/s. İlk çözüm 400 barlık seri için ~3,7 KB
  (çoğu ilk ve son eksik parçanın bar işaretleri). E baytın yaklaşık yarısı (`d_noe`).
- **r→a:** yalnız `data_dispute`; varsayılan senaryolarda ≈ 0.
- **Yansı işi (M10):** 2,5–17 iş/olay; S15/S15k 90–96 (geçmiş onarımı); `d_prefix` 7–736.
  Paylaşılan motorun `apply_settled`'ı son bardan önceki her bar için baştan hesaplar
  (düzeneğin özelliği); toplu bir yol bu maliyeti düşürür (Öneriler).
- **REST (kendi kaynağından çekme):** düzenek saymıyor. Nitel: otorite bar kapanışı başına
  bağ başına 1 istek (1 bar); yansı yalnız fark bulunca (S6'da kapanışların ~yarısı, S15'te
  başlangıçta ~13 parça, S12'de bir kerelik ~2 700 bar).

## 7. Sınırlar

- **M3g kesin sıfır değil** (§5.3): geçici gösteren her yaklaşım bilgi gecikmesi penceresine
  açıktır; D'de oran pay ile seçilir. Ürün kararı (S1): katı tanımda pay 1,5 → 0 ama geçici
  iddiaların çoğu canlı barda hiç gösterilmez.
- **Kesin etiket ≈ 2,9 s gecikir** (S3 sorusu). `d_nosettle` 0,1 s ama S8/S6/S14/S15k'de
  yanlış ve kaçan.
- **Tek otorite:** otorite susarsa (kanal kopması) yeni iddia gösterilmez (X1 M8).
- **Kanal mesaj kaybetmiyor** (düzenek); D, toplu sıra numarasında kalıcı boşluk için
  yeniden eşitleme ve Merkle ikiye bölme sorgusu içermiyor — gerçek taşımada bağlantı
  koptuğunda anlık görüntü (`welcome`) gerekecek.
- **Kesinleşme gecikmesi (2 s) sağlayıcının bilinen bir özelliği** olarak koda yazılı;
  cevap eksikse 300 ms sonra yeniden sorulur.
- **Yetişme filtresi** besleme zamanında "son bar"a gelen tiki canlı tik sayar; daha eski
  bara gelen yamayı düzeltme sayar (geç sıralı tik de bu yoldan uygulanır, motorun `seq`
  kuralı çoğunu eler; kalanını işaret onarımı yakalar).
- **Çapa hizalama** yansının grafiğine kullanıcının istediği pencereden fazla geçmiş ekler.
- REST maliyeti ölçülmedi; FNV-1a 64 kriptografik değil (S5).

## 8. Belirleyicilik

`harness determinism --approach d --scenario S7 --scenario S11 --scenario S9 --scenario S15 --scenario S2c --seeds 3`:

```
S7 seed 0: 6073 lines, hash c36ab3ac29c329a9 / c36ab3ac29c329a9 -> identical
S7 seed 1: 3442 lines, hash dd153430c687aaf1 / dd153430c687aaf1 -> identical
S7 seed 2: 2285 lines, hash 56946b3e11990e6b / 56946b3e11990e6b -> identical
S11 seed 0: 377 lines, hash f7aad9f59555aee1 / f7aad9f59555aee1 -> identical
S11 seed 1: 393 lines, hash e45b8fa6aac098f3 / e45b8fa6aac098f3 -> identical
S11 seed 2: 429 lines, hash bea05fe21481daea / bea05fe21481daea -> identical
S9 seed 0: 88 lines, hash fce2b0d85b6d4b0f / fce2b0d85b6d4b0f -> identical
S9 seed 1: 78 lines, hash 4eeb5c5c042059db / 4eeb5c5c042059db -> identical
S9 seed 2: 114 lines, hash d83cb7fc203a1288 / d83cb7fc203a1288 -> identical
S15 seed 0: 248 lines, hash 7a5dd80cb0acd709 / 7a5dd80cb0acd709 -> identical
S15 seed 1: 315 lines, hash e2d40e4dd5ba6a9b / e2d40e4dd5ba6a9b -> identical
S15 seed 2: 360 lines, hash c76089c8dae06ad7 / c76089c8dae06ad7 -> identical
S2c seed 0: 32 lines, hash 8dd05e408bd0a38d / 8dd05e408bd0a38d -> identical
S2c seed 1: 34 lines, hash b2fe01e162abf23b / b2fe01e162abf23b -> identical
S2c seed 2: 30 lines, hash d89d006b3023d8de / d89d006b3023d8de -> identical
```

Bağdaştırıcılar saat, rastgelelik ve ortam okumaz; yalnız `BTreeMap`/`BTreeSet`. Birim testi
`deterministic_and_clean_on_ideal` (S1, S0, S9 × 2 tohum: aynı günlük, hata 0). Ablasyon ve
tanı testleri (`#[ignore]`) ortam değişkenini yalnız *test* olarak okur (hangi senaryo,
kaç tohum), bağdaştırıcılar değil.

## 9. D seçilirse şartnamede gerekenler

1. **`EvidenceRef` genişlemesi** (geriye uyumlu, isteğe bağlı alanlar):
   `anchor_t` (hesap başlangıcı), `rev` (otoritenin o serideki kaynak `seq`'i), `epoch`
   (veri dönemi), `engine` (motor kimliği), `maturity` (`provisional` | `final`).
   **Aralık anlamı kararı:** PROTOCOL §18.5 bugün aralığı bar 0'dan olgunun barına kadar
   alıyor (önek). D'nin ölçümü: canlı tutarlılık için **pencere** (iddia barı + `period`)
   özeti + değer eşitliği + çapa yeterli ve önek kadar sağlam; önek ×4–60 iş. Öneri:
   `from_t..to_t` = kanıt penceresi; tam taahhüt gerekiyorsa ayrı bir `prefix` (ya da
   `verify_evidence`'ın önek kipi) alanı. Bu §18.5'te bir değişikliktir ve karar ister.
2. **`Fact`:** `t` (bar zamanı) zaten kimlik; `evidence` (değerler) var; `evidence_ref`
   **her olguda** (geçici dahil, özetsiz) zorunlu.
3. **Yeni olaylar** (anlam düzlemi, günlüğe `qp` yazarlı girer): `fact_final {id, evidence_ref[]}`,
   `fact_retracted {id, reason}`; `condition_met`/`intent_triggered`/`finding_state` aynı
   `evidence_ref`'i taşır.
4. **Yeni abonelik konusu** (günlüğe girmez): `data_mark {series, epoch, bars:[[t,digest]],
   chunks:[[c,digest]]}`; parça boyu (32) ve hizası (mutlak bar zamanı) şartnamede sabit.
5. **Yansı → otorite:** `data_dispute {id, what}` (bilgi; varlık durumu gibi günlüğe girmez).
6. **Barındırıcı sözleşmesi:** veri sağlayıcının kesinleşmiş bar ucu (`fetch_settled`) ve
   kesinleşme gecikmesi; otorite kesin demeden önce ona sorar (S3). Yansı farkta, geride
   kaldığında ve çapası farklıyken **kendi** yetkisiyle oradan çeker (S2).
7. **Kavram sözlüğü** (önce `docs/CONCEPTS.md`): kanıt penceresi, kesinleşme onayı, veri
   dönemi, veri işareti, geçici koruma payı, görüntü durumları (`Provisional`, `Final`,
   `Pending`, `SourceDiffers`, `Disputed`), yetişme, çapa hizalama.
8. **Ürün kararları:** geçici koruma payı (önerim 0,75 RSI puanı; dedektör başına birim),
   `Provisional`'ın "kapandı, onay bekliyor" durumunu da kapsaması, `SourceDiffers`/`Disputed`
   dili.

## 10. Yeniden üretim

```bash
cd rounds/RP2/experiments/harness
cargo build --release
./target/release/harness run --approach d --scenario all            # §3
./target/release/harness run --approach n --scenario all            # N
D_SEEDS=100 D_SCEN=S0,S1,S2a,S2,S2c,S3,S4,S5,S6,S7,S8,S9,S10,S11,S12,S12s,S13,S14,S15,S15k,S16,S18,S19 \
  cargo test --release --lib ablation_table -- --ignored --nocapture      # §4 (D_VAR=d,d_noe,… süzer)
D_VAR=d_m0 D_DIAG=S7:0-9 cargo test --release --lib diag_m3p -- --ignored --nocapture
./target/release/harness determinism --approach d --scenario S7 --scenario S11 --seeds 3
```

Ablasyon varyantları kayıtta değil: kayıt makrosu modül başına bir yaklaşım kaydediyor;
`d_noe` gibi ayrı kayıt için `approaches/` altında ayrı dosyalar gerekirdi (yazma kapsamım
dışında). Varyantlar `d::variants()` ile aynı `run_scenario` üzerinden koşuluyor.

X1: `harness spec S6` alınıp §5.4'teki alanlar değiştirildi (`replica_feed.reorder_p=0.2,
reorder_ms=800, dup_p=0.05, dup_ms=1000`; `authority_feed.drop_last_tick_p=0.25`; `a2r`,
`r2a`, `disconnects` S4'ten; `corrections` S9'dan; `history_bars=1200`; `ticks_per_bar=60`).

## Öneriler (kapsam dışı; yapılmadı)

- Düzenek: `fetch_settled` istek ve bar sayısını ölçülere eklemek (REST maliyeti D/A için
  belirleyici; şu an görünmüyor).
- Düzenek/motor: `Engine::apply_settled` için toplu yol (bir kez yeniden hesap); S15'te M10'u
  ~90'dan ~3'e indirir.
- Kayıt makrosu: bir modülün birden çok yaklaşım (ablasyon) kaydedebilmesi.
- Hakem: M3g için `live_grace_ms` > 0 ile duyarlılık tablosu (ürün kararı S1'e girdi).
- Kanal modeli: mesaj kaybı + yeniden bağlanmada anlık görüntü, D'nin yeniden eşitleme
  yolunu ölçmek için.
