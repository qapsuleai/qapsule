# M25 alternatifi `yukdus` — kapanış yerine yüksek/düşükle karar

Dal `rp4/m25c-yukdus`. Düzenek `rounds/RP4/experiments/harness` (SPEC.md değiştirilmedi).
Veri yalnız kalibrasyon kopyası `fixtures/local/rp4/calib/`; `--split test` çalıştırılmadı.
Bu dosyada yalnız toplu sayılar var (sembol adı, fiyat yok).

## Kural

Çerçeve `r3` ile aynı: `P0` = doğum kapanışı, ölçü = doğum barındaki Wilder ATR
(`bars[..=doğum]`; yönlü ATR14, yönsüz ATR100), türe göre ufuk (20 / 15 / 10 bar, `r3`'ünki),
difüzyon ölçekli bariyer. İki fark:

1. **Değme.** Yönlü iddiada barın lehte uzanımı `f` (yukarı: `h − P0`, aşağı: `P0 − l`) ve
   aleyhte uzanımı `a` (yukarı: `P0 − l`, aşağı: `h − P0`). `(doğum, min(now, doğum+h)]`
   içindeki ilk barda `f ≥ T` → gerçekleşti (`yukdus.target`), `a ≥ T` → tutmadı
   (`yukdus.stop`). Yönsüzde `max(h − P0, P0 − l) ≥ T_n` → gerçekleşti
   (`yukdus.expansion`); ufukta yoksa tutmadı (`yukdus.no_expansion`).
2. **Genişletilmiş bariyer.** `T = S_DIR · 0,3·√h · ATR14`, `T_n = S_NEU · 0,65·√h · ATR100`.
   Seçilen: **`S_DIR = 2,0`, `S_NEU = 1,3`** (etkin `C_DIR' = 0,6`, `C_NEUTRAL' = 0,845`).

**Bağ çözme** (aynı barda hem hedef hem durdurma değdi): kapanışın tarafı karar verir:
`±(c − P0) > 0` → gerçekleşti (`yukdus.tie_target`), değilse tutmadı (`yukdus.tie_stop`;
tam `P0`'da kapanış iddia yönünde ilerleme sayılmaz). Gerekçe: OHLC bar içindeki sırayı
vermez; kapanış barın son durumu hakkındaki tek bilgidir. Alışılmış "kötümser" seçim (hep
durdurma) tutmadı'yı sistematik şişirir — M25'in azaltmak istediği erken kapanma yönünde;
"hep hedef" ayna yanlılıktır. Kapanış kuralı yukarı/aşağı iddialarda simetrik ve
belirleyicidir. Bağ, `T` `r3`'ünkünün iki katı olduğundan barın aralığının `2T`'yi aşmasını
ister; seyrek olmalı, ama düzenek karar nedenini saymadığı için **bağ sayısı ölçülmedi**.

Sabit sayısı 7 (`r3` ile aynı: iki bariyer sabiti, iki ATR periyodu, üç ufuk; bağ kuralı
sabit değil). `judge` `(hikâye, bars[..=now])`'un saf fonksiyonu; doğum ATR'si önbelleğinin
anahtarı doğum barının değerlerini içerir (`Barrier` gibi). Testler (`cargo test`, 6):
fitilde gerçekleşme (kapanış kuralı aynı barda karar vermez), aşağı/yukarı ayna, bağ
kuralının üç durumu ve aynası, yönsüz genişleme/ufuk, ufuk dışı bar okunmaz ve önbellek
bayatlamaz, `levels` = `judge` eşiği.

## Ayar denemeleri (yalnız kalibrasyon, 5 dk, yönlü satır)

Seçim ölçütü: SPEC §4'ün 5 dk koşullarının hepsini payla geçen en yuvarlak ölçek. Beş koşu;
hepsi `m25 --period 5m --split calib` (başvuru `r3` her koşuda aynı: ECR_own 0,148, p_exp
0,118, SENS 0,212, Brier 0,2428). Denemeler geçici kayıt adlarıyla yapıldı
(`outcome_pct::<D, N>`, crate'te duruyor; kayıt satırları commit'ten önce silindi).

| # | `S_DIR` | `S_NEU` | p_real | p_inv | p_exp | ECR_own | ECR_ref | SENS | Brier (fark aralığı) | Sonuç |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 1,0 | 1,0 | 0,456 | 0,500 | 0,044 | 0,223 | 0,223 | 0,070 | 0,2480 (+0,0045, +0,0059) | (1) ve (3) kaldı: yalnız değme erken kapanmayı **artırıyor** |
| 2 | 1,6 | 1,0 | 0,397 | 0,412 | 0,191 | 0,102 | 0,143 | 0,090 | 0,2393 | (1) kaldı (0,102 > 0,074) |
| 3 | 2,0 | 1,15 | 0,342 | 0,342 | 0,316 | 0,066 | 0,115 | 0,099 | 0,2247 | hepsi geçti |
| 4 | 2,3 | 1,15 | 0,302 | 0,295 | 0,403 | 0,050 | 0,098 | 0,092 | 0,2104 | (4) kaldı (p_exp 0,403 > 0,40) |
| 5 | 2,0 | 1,3 | yönlü satır #3 ile aynı | | | | | | | **seçildi** |

Yönsüz satır (puanlamaya girmez; `r3` p_real 0,395): `S_NEU` 1,0 → 0,508 (Brier farkı
+0,010, aralık 0'ın üstünde), 1,15 → 0,431 (+0,005, üstünde), 1,3 → 0,366 (−0,008, altında).
1,3'te kaldı. Yönlü satır `S_NEU`'dan bağımsız (#3 ve #5 yönlü satırda aynı sayılar).
`S_DIR` için uygun bölge dar: ~1,9 altında (1) tutmuyor (doğrusal ara değerle), 2,3'te (4).

## Kalibrasyon ölçüleri, dört periyot (yönlü satır; fark = `yukdus − r3`, eşli bootstrap %95)

5 dk / 15 dk / 1 sa: 2025-11-03 → 2026-10-16, 100 sembol, 241 gün. 1 gün: 2018-01-02 →
2024-12-31, 98 sembol, 1.751 gün. Tam tablolar `M25.md` / `M25.csv` (bu klasör; `harness
report`).

| Periyot | Kural | n | p_real | p_inv | p_exp | ECR_own | ECR_ref | SENS | REV | Brier | taban | kal. farkı | gerçek − yalancı | ömür ort./p90 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 5 dk | r3 | 383.867 | 0,415 | 0,467 | 0,118 | 0,148 | 0,148 | 0,212 | 0,250 | 0,243 | 0,243 | 0,021 | −0,052 | 4/11 |
| 5 dk | yukdus | 346.776 | 0,342 | 0,342 | 0,316 | **0,066** | 0,115 | 0,099 | 0,218 | 0,225 | 0,225 | 0,023 | −0,000 | 7/19 |
| 15 dk | r3 | 138.347 | 0,432 | 0,450 | 0,118 | 0,136 | 0,136 | 0,156 | 0,235 | 0,245 | 0,245 | 0,027 | −0,018 | 4/10 |
| 15 dk | yukdus | 125.959 | 0,354 | 0,344 | 0,303 | 0,058 | 0,109 | 0,038 | 0,202 | 0,229 | 0,229 | 0,028 | 0,010 | 7/16 |
| 1 sa | r3 | 35.040 | 0,468 | 0,441 | 0,091 | 0,128 | 0,128 | 0,091 | 0,216 | 0,251 | 0,249 | 0,057 | 0,028 | 4/10 |
| 1 sa | yukdus | 31.330 | 0,387 | 0,345 | 0,268 | 0,043 | 0,087 | 0,012 | 0,180 | 0,240 | 0,237 | 0,063 | 0,042 | 8/15 |
| 1 gün | r3 | 28.122 | 0,507 | 0,420 | 0,073 | 0,159 | 0,159 | 0,091 | 0,244 | 0,246 | 0,250 | 0,065 | 0,087 | 3/10 |
| 1 gün | yukdus | 24.591 | 0,439 | 0,319 | 0,242 | 0,044 | 0,093 | 0,034 | 0,195 | 0,235 | 0,246 | 0,099 | 0,120 | 6/15 |

Farklar ve aralıkları:

| Periyot | ECR_own | ECR_ref | SENS | p_exp | Brier | kal. farkı |
|---|---|---|---|---|---|---|
| 5 dk | −0,082 [−0,084, −0,079] | −0,033 [−0,036, −0,031] | −0,113 [−0,120, −0,106] | +0,197 [+0,193, +0,203] | −0,018 [−0,019, −0,017] | +0,002 [−0,001, +0,004] |
| 15 dk | −0,077 [−0,081, −0,074] | −0,026 [−0,029, −0,024] | −0,118 [−0,123, −0,113] | +0,184 [+0,179, +0,190] | −0,017 [−0,018, −0,016] | +0,001 [−0,003, +0,005] |
| 1 sa | −0,085 [−0,091, −0,079] | −0,041 [−0,048, −0,035] | −0,079 [−0,085, −0,073] | +0,177 [+0,170, +0,184] | −0,011 [−0,013, −0,010] | +0,005 [−0,002, +0,013] |
| 1 gün | −0,115 [−0,123, −0,106] | −0,066 [−0,075, −0,057] | −0,057 [−0,065, −0,051] | +0,169 [+0,163, +0,175] | −0,011 [−0,012, −0,010] | **+0,034 [+0,027, +0,041]** |

`sens_replay_mismatch` = 0 her periyotta ve iki satırda (kural saf).

## SPEC §4 (kalibrasyon, yönlü satır; `report` mekanik puanlaması: **aday**)

1. 5 dk ECR_own 0,066 ≤ ½·0,148 = 0,074; fark aralığının üst ucu −0,079 < 0 — **geçti** (pay dar: 0,008).
2. ECR_ref farkı [−0,036, −0,031], 0'ın üstünde değil — **geçti**.
3. Brier farkı 5 dk [−0,019, −0,017], 1 sa [−0,013, −0,010], alt uçlar ≤ 0; 5 dk kalibrasyon
   farkı farkı +0,002 ≤ +0,03 — **geçti**.
4. 5 dk p_exp 0,316 ≤ 0,40 — **geçti**.
5. 5 dk SENS farkı [−0,120, −0,106], alt uç ≤ 0 — **geçti** (ama aşağıdaki okuma notuna bak).

## SPEC ölçüleri bu kuralda nasıl okunur

SPEC'in `ECR` ve `SENS` tanımları **kapanışa** dayanır; bu kural **yüksek/düşükle** karar
verir. SPEC değiştirilmedi; okuma şöyle:

- **ECR_own** = tutmadı denen hikâyelerde sonradan bir **kapanışın** kuralın kendi hedefine
  (`T`) ulaşması. Kuralın kendi gerçekleşme ölçütü ise fitilin `T`'ye değmesidir; kapanışla
  ulaşmak daha zordur. Bu yüzden buradaki ECR_own, "kuralın kendi diliyle sonradan
  gerçekleşirdi" oranının **alt sınırıdır**; fitil tabanlı bir ECR daha yüksek çıkar
  (ölçülmedi; düzenek bunu vermiyor). Kurallar arasında adil karşılaştırma **ECR_ref**'tir
  (aynı kapanış tabanlı `r3` hedefi): 0,148 → 0,115, yani yarıya inmiyor; hedefin yarıya
  inişi büyük ölçüde hedefin kendisinin iki kat uzak olmasından.
- **SENS** kararı veren barın **kapanışını** ±1/±2 adım oynatır; yüksek/düşük yalnız kapanış
  onları aşarsa genişler. Bu kuralın kararı fitile bağlı olduğundan oynatma çoğu zaman
  kararı etkilemez; değiştirdiği durumlar kapanışın barın ucunda olduğu barlar ve bağ
  barlarıdır. Yani düşük SENS kısmen **yapısaldır** (ölçünün bu kurala dokunmaması), "tek
  barlık gürültüye dayanıklı" anlamına tam gelmez. Bu kuralın asıl gürültü kaynağı **fitil
  gürültüsü**dür (tek bir uç işlem, ±1–2 adımlık fitil); düzenek bunu ölçmez. Kanıt:
  deneme #1'de (bariyer `r3` ile aynı) SENS 0,070 iken ECR_own 0,223 — duyarlılık düşük
  görünürken erken kapanma `r3`'ten kötü.
- **REV** ve **gerçek − yalancı** kapanış tabanlı kalır ve doğrudan okunur.

## Zayıf yanlar (dürüst)

1. **Kazancın kaynağı değme değil, genişletme.** Yalnız değme (deneme #1) erken kapanmayı
   0,148 → 0,223 artırdı. İyileşme bariyerin iki katına çıkmasından geliyor; değme, genişletmenin
   doğuracağı süresi dolma artışını kısmen geri alıyor (bariyer ×2 iken p_exp 0,316; kapanış
   tabanlı bir genişletmede daha yüksek olması beklenir — ölçeklenen eşik alternatifiyle
   karşılaştırılmalı). Kapanış tabanlı uygun bir genişletme benzer ECR'yi daha basit verebilir.
2. **Süresi dolma üç kat** (5 dk 0,118 → 0,316), ömür ortancası 4 → 7 bar. Kullanıcı daha az
   "karar" görür; hedef yalnız 0,084 payla 0,40'ın altında. 2,3'te sınırı aşıyor; uygun bölge dar.
3. **Brier iyileşmesi beceri değil, taban oranı kayması:** p_real 0,415 → 0,342 düştüğü için
   `p(1−p)` küçüldü; Brier − taban 5 dk'da ≈ 0 (−0,0003; `r3` +0,0000). Günlükte Brier tabanın
   0,011 altında — tek gerçek beceri işareti orada.
4. **Yön becerisi 5 dk'da yok:** gerçek − yalancı −0,052 → −0,000; `r3`'te gerçek iddia yalancı
   eşten kötüydü, burada eşit. Simetrik bariyerde yalancı eşin isabeti ≈ asılın p_inv'i.
5. **Günlükte kalibrasyon farkı kötüleşiyor:** 0,065 → 0,099, aralık [+0,027, +0,041] tamamen
   kötü yönde (+0,03 sınırını da aşıyor; §4(3) yalnız 5 dk'yı sayar, ama §6 için uyarı).
   1 sa yönsüz satırda da +0,025 [+0,002, +0,051].
6. **Fitil verisi kırılgan:** yüksek/düşük tek bir işlemle (tek lot, özel işlem, hatalı tik)
   oluşur; kapanış daha sağlamdır. REST barı ile işlemden bar (M27) fitilde ayrışırsa karar
   kaynağa göre değişir. Canlıda oluşmakta olan barda fitil değdiği an karar verilirse
   davranış "bar kapanınca karar"dan farklılaşır (motor kapanmış barla çağırıyorsa sorun yok;
   doğrulanmadı).
7. **Bağ kuralının etkisi ölçülmedi** (düzenek karar nedeni saymıyor). Kapanış tarafı kuralı
   SENS'in oynattığı tek değere (kapanış) bağlı olduğundan bağ barları SENS'e girer.
8. Ayar yalnız 5 dk'da yapıldı; 15 dk, 1 sa, 1 gün aynı sabitlerle (ayrı ayar yok).

## Yeniden üretim

```sh
cd rounds/RP4/experiments/harness
export CARGO_TARGET_DIR=<ana ağaç>/target/wt/m25c
cargo build --release
# depo kökünden:
$H m25 --data fixtures/local/rp4/calib --rule yukdus --period 5m --split calib   # 15m, 1h, 1d
$H report --results <sonuç klasörü> --out rounds/RP4/experiments/M25/yukdus
```

Süre (`--release`, 16 çekirdek, iki kural eşli): 5 dk ~95 s (~2,1 GB), 15 dk 8 s, 1 sa 1 s,
1 gün 1 s. Denemeler için `m25_yukdus::outcome_pct::<D, N>` / `levels_pct::<D, N>`
(yüzde ölçek) bir kayıt satırıyla takılır.
