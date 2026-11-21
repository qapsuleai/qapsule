# RP4 ölçüm düzeneği ve puanlama kuralı (M25, M26) — alternatiflerden önce

Yazıldı: 2026-11-21, `SPLIT.md` (cc476de) commit'inden sonra, hiçbir alternatif dalı açılmadan
ve hiçbir hikâye sonucu hesaplanmadan önce. Bu dosya alternatiflerden önce commit edilir;
sonradan değişirse değişiklik ayrı commit'te gerekçesiyle yazılır ve hangi ölçümden önce
olduğu gösterilir.

## 1. Veri ve aralıklar

- Barlar: REST kesin barlar, **düzeltilmiş** (bölünme sıçraması yok), `fixtures/local/live/rest/`
  (`<SYM>.<period>.<start>.<end>.adj.json`, satırlar `time` epoch ms, `open/high/low/close/volume`).
  Evren `fixtures/local/live/u100.txt` (BIST100; çekilemeyen sembol raporda sayılır).
- Periyotlar: **5 dk** (REST), **15 dk** (5 dk barlardan XIST takvimine hizalı toplama, K14,
  `qp_calendar`), **1 sa** (REST), **1 gün** (REST).
- Aralıklar `SPLIT.md`'den. Düzenek iki kipte çalışır: `--split calib` (varsayılan) yalnız
  `t < 2026-10-19 00:00 İstanbul` barlarını **okur** (dosyadan okurken keser; günlükte
  `t < 2025-01-02`), `--split test` sınama aralığını açar ve yalnız doğumu sınama aralığında
  olan hikâyeleri sayar (ısınma için öncesini okur). Alternatif ajanlarına yalnız
  `prepare` adımının yazdığı **kalibrasyon kopyası** (`fixtures/local/rp4/calib/`, sınama
  günleri fiziksel olarak yok) verilir.
- Fiyat adımı (`tick(p)`): BIST pay tablosu (≤20: 0,01; ≤50: 0,02; ≤100: 0,05; ≤250: 0,10;
  ≤500: 0,25; ≤1000: 0,50; ≤2500: 1,00; üstü 2,50 — `docs/matriks/MESSAGES.md` §(c)); canlı
  `priceStep` ile uyumu yerel ham kayıttan ölçülür ve raporlanır. Adım, doğum kapanışının
  fiyatına göre alınır.

## 2. Akış

Her periyotta her sembol için: dedektörler (`qp-facts`, ürün sabitleri ya da M26 alternatifi)
→ olgular → motor (`qp_capsule::pipeline::simulate`, tema `kinddir`, önem `confirm`, sonuç
kuralı = aday) → hikâye olayları ve karne. Ürün yolunun aynısıdır (ayrı yeniden yazım yok).
Sonuç kuralı `OutcomeRule` arayüzünden takılır; alternatif bir crate ya da `qp-capsule`
içinde adlandırılmış bir kuraldır. Gün içinde hikâye ufku bar sayısıyla sayılır (gece
boşluğu bar değildir; ürün davranışı).

## 3. Ölçüler (M25)

Her periyot için, sayılan hikâyeler üzerinde (yönlü = `up`/`down`; yönsüz ayrı satır):

| Ölçü | Tanım |
|---|---|
| `n` | sayılan hikâye; gün, sembol sayısı |
| `p_real`, `p_inv`, `p_exp` | gerçekleşti / tutmadı / süresi doldu payları |
| **`ECR_own`** (erken kapanma) | `invalidated` olanlar içinde, doğum barından itibaren ufuk penceresi `(born, born+h]` içinde **tutmadı kararından sonraki** bir barın kapanışı, iddia yönünde **kuralın kendi hedef eşiğine** ulaşan pay |
| `ECR_ref` | aynı, ama eşik başvuru kuralının (`r3`: `0,3·√h·ATR14`) hedef eşiği (eşik genişletmeyle oyunu önlemek için ikinci ölçü) |
| `REV` (gerçekleşti sonra döndü) | `realized` olanlar içinde, karar barından sonraki `⌈h/2⌉` bar içinde bir kapanışın doğum kapanışını (`P0`) iddianın tersine geçtiği pay |
| Ömür | doğumdan kapanışa bar sayısının ortancası ve p90'ı (kapanan hikâyeler) |
| Brier | sayılan her hikâyede `ledger_at_birth` oranı `p̂` (karnesi `n ≥ 20` olanlar; yoksa sayılmaz) ile sonuç `y` (gerçekleşti=1, tutmadı/süresi doldu=0): `mean((p̂−y)²)`; yanında taban (`p̂` yerine kalibrasyon aralığının genel oranı) ve 0,25 |
| Kalibrasyon farkı | anahtar başına (`tür/yön/dedektör`) kalibrasyon sonundaki karne oranı ile sayılan aralıktaki gerçekleşme oranı; `n` ağırlıklı mutlak fark |
| **`SENS`** (tek bar duyarlılığı) | karar verilmiş (gerçekleşti/tutmadı) her hikâyede, kararı veren barın kapanışı `±1` ve `±2` fiyat adımı oynatılır (yüksek/düşük kapanışı kapsayacak kadar genişletilir); kural yalnız o hikâye için yeniden çalıştırılır (doğum sabit); dört oynatmadan en az birinde aşaması değişen hikâye payı. Süresi dolanlarda: ufkun son barı ve en yakın barın kapanışı aynı biçimde oynatılır |
| Gerçek − yalancı eş | R3/M5 tanımı (yön çevrilmiş eş hikâyenin isabeti); günlükte R3 başvurusuyla karşılaştırma |

Güven aralıkları: **sembol kümeli bootstrap** (sembolleri yerine koyarak yeniden örnekle,
1.000 tekrar, tohum 20261121, belirleyici); alternatif − başvuru farkı her tekrarda aynı
sembol örneğiyle (eşli) hesaplanır; %95 yüzdelik aralık.

## 4. Puanlama kuralı (M25) — seçim yalnız kalibrasyon aralığında

Bir alternatif **aday** sayılır, ancak kalibrasyon aralığında:

1. **5 dk `ECR_own` ≤ 0,5 × başvuru** (nokta tahmini) **ve** farkın %95 aralığının üst ucu < 0;
2. **`ECR_ref` başvurudan büyük değil** (farkın aralığı 0'ın üstünde değil);
3. **Kalibrasyon bozulmuyor:** 5 dk ve 1 sa Brier farkının aralığının alt ucu ≤ 0 değilse
   (yani anlamlı kötüleşme varsa) elenir; kalibrasyon farkı başvurudan +0,03'ten fazla kötü değil;
4. **Süresi dolan payı** 5 dk'da ≤ 0,40 (hikâye karar vermeden ömrünü doldurmamalı);
5. **`SENS`** 5 dk'da başvurudan büyük değil (farkın aralığının alt ucu ≤ 0... büyükse elenir:
   aralığın alt ucu > 0 ise elenir).

Adaylar arasında sıra: (a) 5 dk `SENS` küçük olan, (b) 5 dk `ECR_own` küçük olan,
(c) 15 dk ve 1 sa'te aynı ölçüler, (d) basitlik (daha az sabit). İlk iki ölçüde iki adayın
farkının aralığı 0'ı kapsıyorsa "ayırt edilemedi" denir ve daha basit olan seçilir.
**Hiç aday yoksa bugünkü kural (`r3`) kalır** ve "ayırt edilemedi / hedef tutmadı" yazılır.

Periyoda göre farklı kural seçilebilir (ör. gün içi `rp4`, günlük `r3`); seçim her periyotta
yukarıdaki kuralla ayrı verilir; günlük periyotta yalnız §6 denetlenir.

## 5. M26 — dedektör eşikleri gün içinde

Ölçüler (her periyot, her dedektör): sembol·gün başına doğum (akış yükü); kalibrasyon
aralığında isabet `p_real` ve K53 yazı tura eşiğine (0,55) göre konumu (aralığıyla);
**açılış** (günün ilk iki barı) ve **kapanış** (günün son barı, kapanış seansı) doğumları:
sayısı, payı, isabeti ve geri kalanla farkı ("yanlış tetikleme" = bu barlarda doğup
isabeti geri kalandan anlamlı düşük olanlar); günlük periyotla tutarlılık (aynı dedektörün
günlük isabetiyle fark).

Puanlama: bir alternatif, kalibrasyonda (1) dedektör başına isabeti başvurudan düşük değilse
(aralık), (2) akış yükünü sembol·gün başına başvurunun en çok 1,5 katına çıkarmıyorsa,
(3) açılış/kapanış yanlış tetiklemesini azaltıyorsa (aralığın üst ucu < 0) aday olur.
Sıra: yanlış tetikleme azalması, sonra isabet. Aday yoksa bugünkü sabitler kalır. Bir
dedektörün gün içi isabetinin aralığı tamamen 0,5'in altında ya da 0,55 eşiğinin altında ve
aralığı 0,5'i kapsıyorsa "gün içinde anlamsız" diye raporlanır ve periyot sınırı önerilir
(kapatılmaz; öneri).

## 6. Günlük periyot kötüleşmedi (ölçüt 7)

Seçilen kural(lar) ile `r3` günlük sınama aralığında: `p_real`, `p_exp`, `ECR_own`, gerçek −
yalancı eş, Brier. Kötüleşme = farkın aralığı tamamen kötü yönde. R3 başvurusu (`R3/M5`
RESULTS: gerçek − yalancı eş günlük 0,088, saatlik 0,043; yönlü isabet 0,50–0,54) yan sütun.

## 7. Son ölçüm (S)

Seçilen kurallar + seçilen eşikler, sınama aralığında, iki bar kaynağıyla: REST barları ve
işlemden barlar (T2) — hızlandırılmış benzetim (`LiveFeed`) üzerinden. Tablo §3 ölçüleri,
başvuru `r3` ile yan yana; hedef (5 dk `ECR_own` ≤ başvurunun yarısı, kalibrasyon bozulmadan)
tuttu/tutmadı olduğu gibi yazılır. Sınama sonucu seçimi geri değiştirmez.

## 8. Yeniden üretim

`harness prepare` (kalibrasyon kopyası), `harness m25 --rule <ad> --period 5m [--split test]`,
`harness m26 …`, `harness report` → `rounds/RP4/experiments/M25/` ve `M26/` altında CSV/MD
(yalnız toplu sayılar; sembol adı, fiyat yok). Aynı girdi iki koşuda bayt eşit çıktı (test).
