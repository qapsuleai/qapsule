# RP2 / T7 — Tutarlılık yaklaşımlarının karşılaştırması

Dört yaklaşım ayrı dallarda, ayrı alt ajanlarla, birbirini görmeden kuruldu
(`rp2/t7a-approach` … `rp2/t7d-approach`; raporlar `T7a.md` … `T7d.md`). Bu karşılaştırma
dördünü tek dalda (`rp2/t7-compare`) birleştirip **aynı düzenek ve aynı tohumlarla** yeniden
koşar. Düzenek yaklaşımlardan önce ve onlardan bağımsız yazıldı (`HARNESS.md`).

- Komut: `harness run --approach <x> --scenario all` (her senaryo 100 tohum, S17 20).
- Ham çıktılar: `raw/t7/table-{n,a,b,c,d}.txt`, (JSON sonuçları boyut nedeniyle commit edilmedi; aynı komut yeniden üretir).
- Ortam: Apple M4 Max, Darwin 27.0; düzenek sanal zamanlı ve belirleyici (sonuçlar makineden
  bağımsız; yalnız duvar saati süreleri değişir).
- Yeniden koşum alt ajanların raporladığı sayıları birebir verdi.

## Sert hatalar (M1 erken, M2 çelişkili, M3f yanlış kesin)

| | N (taban) | A | B | C | D |
|---|---|---|---|---|---|
| M1+M2+M3f, 24 senaryo toplamı | 24 senaryonun 22'sinde > 0 (ör. S8: 2.881 / 3.206 / 330) | **0** | **0** | **0** | **0** |
| M3p (canlı barda yanlış geçici) | 7.560 (S7) … | 0 (canlı göstermez) | 0 (canlı göstermez) | 84 toplam (C-final türevi: 0) | 2 toplam |

## Kaçırma (M5) ve gecikme (M6f, kesin gösterim p50 / p99, ms)

| Senaryo | A | B | C | D |
|---|---|---|---|---|
| S0 gerçekçi ağ | 0 · 111/139 | 0 · 111/139 | 0 · 111/139 | 0 · **2.885/2.907** |
| S2c yansı beslemesi 60 s geride | **149** kaçırma | 0 | 0 | 0 |
| S3 protokol geride | 0 · 3.600/4.018 | 0 · 3.522/4.025 | 0 · **30/40** | 0 · 6.294/6.822 |
| S4 30 s bağlantı kopması | 2 | 2 | 0 | 0 |
| S6 besleme kaybı | 0 · 2.801 | 0 · 2.894 | 0 · 2.809 | 0 · 3.679 |
| S8 kapanış yarışı | 0 · 2.801/3.667 | 0 · **132**/2.937 | 0 · 120/2.901 | 0 · 2.891/3.703 |
| S11 yeniden bağlanma | **32** | 0 | 0 | 0 |
| S14 otoritenin beslemesi hatalı | 0 · 2.854 | 0 · **112**/3.397 | 0 · 111/139 | 0 · 2.885/4.614 |
| S15k f32 kaynak, eşikte | 0 · 2.801 | 0 · **800**/3.200 | 0 · 2.802 | 0 · 3.601 |
| S19 2 dk besleme kesintisi | **53** | 0 | 0 | 0 |

## Bayt (otorite → yansı, canlı saniye başına) ve yansı işi (iş birimi / olay)

| | N | A | B | C | D |
|---|---|---|---|---|---|
| S0 | 3,1 B/s | 2,5 | **0,7** | 0,8 | 4,2 |
| S7 (60 tik/s titreme) | 2.244,6 | 5,5 | 5,7 | **2,2** | 171,0 |
| S17 (yük: 50 bağ) | 3.926,5 | 190,6 | 251,3 | **105,6** | 985,5 |
| iş/olay S15k | 2,2 | 18,7 | 23,1 | **5,6** | 96,0 |

## Modele uyum (sayıların dışında kalan ölçüt)

| | A | B | C | D |
|---|---|---|---|---|
| Yansı yalnız otoritenin söylediği iddiayı mı gösterir? (tek otorite, K21-2; "her görsel öğe bir olguya bağlı", ilke 6) | evet | evet | **hayır**: yansı kendi bulduğu, dokümanda olmayan iddiaları gösterir; iki kullanıcı farklı şey görebilir | evet |
| Dondurulmuş kanıtla (§18.5 `EvidenceRef`) ilişki | önek özeti = `EvidenceRef` özeti | değerler anlam taşır; özet ayrıca gerekir (`T7b.md` §5) | yok | kanıt penceresi özeti; `EvidenceRef` genişler |
| Protokole ek | konum `[b,W,h,e,a,s]` + "şuraya kadar olgular" işareti | olgu/olayda kanıt değerleri + çapa + olgunluk + temizlik bayrağı | `report` | zarf (5 yeni alan), `fact_final`, `fact_retracted`, `data_mark`, `data_dispute` |
| Karmaşıklık | orta | düşük–orta | orta | yüksek (beş görüntü durumu, dört yeniden çekme yolu) |

## Ortak bulgu (dört savunucu ayrı ayrı vardı)

1. **Sıfıra götüren şey yansının kendi verisinde yeniden sınamasıdır**: yansı bir iddiayı
   yalnız kendi gördüğü veride yüklem tutuyorsa gösterir ve veri her değiştiğinde aynı
   geri çağrıda yeniden sınar. Bu, M1 ve M2'yi yapı gereği sıfırlar.
2. **Kesinlik sağlayıcının kesin barına dayanır** (Matriks REST karşılığı): iki besleme aynı
   kapanış tikini kaybettiğinde (S8) ya da otoritenin beslemesi bozuk olduğunda (S14)
   kesinleşmeyi kurtaran tek şey budur. Barındırıcı sözleşmesine "kesin bar çekme" girer.
3. **Canlı barda geçici iddia sıfır hatayla gösterilemez**: yansının verisi gerçeğin en az
   ~80 ms gerisindedir; 0 ms toleransla M3p > 0 kaçınılmazdır (A, B, C, D ayrı ayrı ölçtü).
   Bu bir ürün kararıdır (açık soru).
4. **Hesap başlangıcı (çapa) aynı olmalı**: S12/S12s'yi dördü de yansının geçmişini çapaya
   uzatarak çözdü (§18.3 çapa kuralı doğrulandı).

## Seçim: B (kanıt değerleri + yansıda yüklem sınaması), dondurulmuş kanıt için §18.5 özeti

Gerekçe:

- **Sert hata 0** (hepsi gibi), **kaçırma en az** (yalnız S4'te bağlantı kopukken 2; A: 236,
  C ve D: 0), **gecikme en düşük grupta** (temiz akışta ~110 ms; D'nin her kesin gösterimde
  ~2,9 s beklemesi yok), **bayt en düşük grupta** (S0 0,7 B/s).
- **Modele uyar:** yansı yalnız otoritenin dokümanındaki olguyu/olayı gösterir (C bu ölçütte
  elenir); iddianın anlamı (kanıt değerleri) zaten `Fact.evidence`'tadır.
- **Protokol eki küçük:** olguda zaten `t` ve kanıt değerleri var; eklenen olgunluk
  (`final`/`provisional`) ve verinin temizliği (otoritenin o bara kadar boşluksuz olması),
  koşul olaylarında da aynı değerler.
- D, kaçırmada B'den iyidir (S4: 0 / 2) ama bunun bedeli her kesin gösterimde ~2,9 s ve
  S17'de 4× bayttır; D'nin ayırt edici parçaları (pencere özeti, E işaretleri) ablasyonda
  M1–M3'ü değiştirmedi (`T7d.md`, ablasyon). A, uzun gecikmeli ve kesintili senaryolarda
  kaçırır (S2c 149, S19 53, S11 32).
- B'nin açık zayıflığı (`T7b.md`): değerlerin bit eşitliği aynı veriyi kanıtlamaz; bu yüzden
  dondurulmuş kanıt (§18.5 `EvidenceRef`, veri özeti) korunur ve `verify_evidence` bu işi
  yapar. Seçilen tasarım = B + §18.5.

Bu bir **öneridir** (`DECISIONS.md` "Önerilen (RP2)"); şartname §19'a ve otoriteye/yansıya
T8'de uygulanır.

## Sınırlar

- Düzenek tek dedektörle (RSI(14) 70/30 kesişimi) ve sentetik veriyle; gerçek Matriks akışı
  yok (R6). Kesin bar kaynağının gecikmesi ve hız sınırı benzetildi, ölçülmedi; REST çağrı
  sayısı M9'a girmiyor.
- Kanal mesaj kaybetmiyor (yeniden bağlanma dışında); kayıplı kanal senaryosu yok.
- Belirleyicilik tek makinede (aarch64) sınandı.
