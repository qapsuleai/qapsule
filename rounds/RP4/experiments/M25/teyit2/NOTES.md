# M25 alternatifi `teyit2` — iki kapanışla teyit

Dal `rp4/m25b-teyit2`. Düzenek `rounds/RP4/experiments/harness` (SPEC.md, değiştirilmedi).
Veri yalnız kalibrasyon kopyası `fixtures/local/rp4/calib/`; `--split test` hiç koşulmadı.
Bu dosyada yalnız toplu sayılar var. Tam tablo: `M25.md` / `M25.csv` (bu klasör, `report`
çıktısı, yalnız `teyit2`).

## Kural

`r3`'ün bariyeri (P0 = doğum kapanışı, ATR14 doğumda `bars[..=born]`'dan, türe göre ufuk
`h` aynı) ile, fark:

| | `r3` | `teyit2` |
|---|---|---|
| Eşik (yönlü) | `0,3·√h·ATR14` | `1,25 × 0,3·√h·ATR14 = 0,375·√h·ATR14` |
| Tutmadı (durdurma) | ilk kapanış `d ≤ −eşik` | **art arda 2 kapanış** `d ≤ −eşik` |
| Gerçekleşti (hedef) | ilk kapanış `d ≥ eşik` | **art arda 2 kapanış** `d ≥ eşik` |
| Yönsüz hikâyeler | `0,65·√h·ATR100`, ilk kapanış | aynısı (değişmedi) |

- Bir dizi, eşiğin içine (ya da öbür yanına) dönen bir kapanışla sıfırlanır. Doğum barı
  sayılmaz; en erken karar `doğum + 2`. Arkin'in gördüğü durum (5 dk, düşüş iddiası, ertesi
  barda "tutmadı") bu kuralla **olamaz**: ertesi bar tek başına karar veremez.
- **Ufkun son barı.** Yalnız `(doğum, doğum + h]` barları sayılır. Son bar bir dizinin ilk
  kapanışıysa (teyit `doğum + h + 1`'i isterdi) karar yoktur, motor hikâyeyi **süresi
  doldu** diye kapatır: iddia kendi ömründe ne doğrulandı ne yanlışlandı. Ek "teyit barı"
  verilmedi: ufuk, dolayısıyla karne ve `ECR` pencereleri `r3`'tekiyle aynı kalır. (Son
  barda tek kapanışla "tutmadı" demek de mümkündü; seçilmedi, çünkü hem gürültüyü ufkun
  sonuna geri getirir hem de ECR penceresi o hikâyede boş olduğundan ECR'ı mekanik olarak
  düşürürdü.)
- Yönsüzde değişiklik yok: "tutmadı" zaten ufukta verilir (genişlemedi), tek barlık gürültü
  sorunu orada yok.
- Saf: `judge` her çağrıda `(doğum, now]`'u yeniden tarar; tek durum doğum ATR'ının önbelleği
  (anahtarı doğum barının değerlerini içerir, `r3` gibi). Saat, ağ, rastgelelik, dosya yok.
  `sens_replay_mismatch` dört periyotta 0.

**Hedef için de iki kapanış (simetri).** Durdurmayı iki kapanışa çıkarıp hedefi tek
kapanışta bırakmak (deneme 1) karar verilenlerde gerçekleşme payını yapay olarak yükseltir
(sürüklenmesiz piyasada artık %50 değil) ve 5 dk Brier'i anlamlı kötüleştirdi (§4 koşul 3
kaldı). Simetrik kural karar verilenlerde tabanı yaklaşık korur: 5 dk karar verilenler
içinde gerçekleşen `r3` 0,415/(0,415+0,467) = 0,470, `teyit2` 0,315/(0,315+0,344) = 0,478.
**Ama** karnenin tabanı (bütün kapananlar içinde gerçekleşen) değişir: süresi dolan payı
0,118 → 0,341 olduğu için 5 dk genel gerçekleşme 0,415 → 0,315. Karne sayısı (ör. "%32")
`r3`'ün sayısıyla (ör. "%41") aynı ölçekte okunmaz; yazı tura eşiği (K53, %55) de bu tabana
göre yeniden düşünülmeli (öneri).

## Denemeler (yalnız kalibrasyon, 5 dk, yönlü satır)

Her deneme `harness m25 --data fixtures/local/rp4/calib --rule <deneme> --period 5m
--out fixtures/local/rp4/trials/teyit2` ile koştu (denemeler aday değil; kayıttan
silindi, kodda `Confirm::new(ad, durdurma, hedef, çarpan)`). Başvuru `r3`: ECR_own 0,148,
SENS 0,212, p_exp 0,118, Brier 0,2428.

| # | Deneme | durdurma / hedef kapanış | çarpan | ECR_own | ECR_ref | SENS | p_exp | Brier (taban) | §4 |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `s2t1` | 2 / 1 | 1,00 | 0,094 | 0,094 | 0,191 | 0,174 | 0,247 (0,247), fark +0,005 [+0,004, +0,005] | 1 kaldı, 3 kaldı |
| 2 | `s2t2` | 2 / 2 | 1,00 | 0,093 | 0,093 | 0,137 | 0,244 | 0,230 (0,230) | 1 kaldı (−%37) |
| 3 | `s2t2k125` | 2 / 2 | 1,25 | **0,061** | 0,071 | 0,132 | 0,341 | 0,216 (0,216) | hepsi geçti → **seçilen** |
| 4 | `s2t2k150` | 2 / 2 | 1,50 | 0,042 | 0,057 | 0,121 | 0,430 | 0,200 (0,200) | 4 kaldı (p_exp > 0,40) |
| 5 | `s1t1k125` (denetim: teyitsiz, yalnız geniş eşik) | 1 / 1 | 1,25 | 0,100 | 0,117 | **0,250** | 0,200 | 0,236 (0,236) | 1 kaldı, 5 kaldı (SENS kötüleşti) |

Seçim: çarpan, §4'ün bütün koşullarını sağlayan en küçük denenen değer (1,25); 1,50 süresi
dolan sınırını aşıyor. Deneme 5 etkinin kaynağını ayırır: yalnız teyit ECR_own'u %37
düşürür ve SENS'i %35 iyileştirir; yalnız eşik genişletme ECR_own'u %32 düşürür ama SENS'i
kötüleştirir; ikisi birlikte %59 düşüş. **Teyit tek başına hedefe (yarı) yetmiyor.**

Sabitler: `STOP_CLOSES = 2`, `TARGET_CLOSES = 2`, `K_SCALE = 1,25` (kayıtta 9 sabit:
`r3`'ün 7'si, `C_DIR·K_SCALE` tek çarpan sayılarak, + iki kapanış sayısı).

## Sonuç (kalibrasyon, yönlü satır; köşeli = fark `teyit2 − r3`, %95 sembol kümeli bootstrap)

| Periyot | n | ECR_own (r3) | ECR_ref | SENS (r3) | p_real / p_inv / p_exp (r3) | Brier / taban (r3) | kal. farkı (r3) | REV (r3) | gerçek − yalancı (r3) |
|---|---|---|---|---|---|---|---|---|---|
| 5 dk | 317.036 | 0,061 (0,148) [−0,087; −0,089, −0,085] | 0,071 | 0,132 (0,212) [−0,080; −0,093, −0,069] | 0,315 / 0,344 / 0,341 (0,415 / 0,467 / 0,118) | 0,2160 / 0,2158 (0,2428 / 0,2428) | 0,025 (0,021) | 0,147 (0,250) | −0,029 (−0,052) |
| 15 dk | 114.368 | 0,051 (0,136) | 0,060 | 0,098 (0,156) | 0,331 / 0,336 / 0,333 (0,432 / 0,450 / 0,118) | 0,2216 / 0,2214 (0,2454 / 0,2453) | 0,030 (0,027) | 0,129 (0,235) | −0,005 (−0,018) |
| 1 sa | 28.705 | 0,046 (0,128) | 0,057 | 0,061 (0,091) | 0,366 / 0,334 / 0,300 (0,468 / 0,441 / 0,091) | 0,2338 / 0,2319 (0,2509 / 0,2490) | 0,050 (0,057) | 0,138 (0,216) | 0,032 (0,028) |
| 1 gün | 22.457 | 0,053 (0,159) | 0,066 | 0,053 (0,091) | 0,403 / 0,324 / 0,274 (0,507 / 0,420 / 0,073) | 0,2355 / 0,2407 (0,2462 / 0,2500) | 0,082 (0,065) [+0,017; +0,010, +0,024] | 0,141 (0,244) | 0,079 (0,087) |

Yönsüz satırlar `r3` ile birebir aynı (fark 0; kural değişmedi).

**SPEC §4 (kalibrasyon, `report` mekanik puanlaması): 1 geçti, 2 geçti, 3 geçti, 4 geçti,
5 geçti → aday.** (1) 5 dk ECR_own 0,061 ≤ 0,074 (½ × 0,148), farkın üst ucu −0,085 < 0;
(2) ECR_ref farkı −0,077 [−0,079, −0,075]; (3) Brier farkı 5 dk −0,027 [−0,028, −0,025],
1 sa −0,017 [−0,019, −0,015]; kalibrasyon farkı farkı 5 dk +0,003 ≤ +0,03; (4) 5 dk p_exp
0,341 ≤ 0,40; (5) SENS farkı −0,080 [−0,093, −0,069].

## Zayıf yanlar (dürüstçe)

1. **Süresi dolan payı üç katına çıkıyor** (5 dk 0,118 → 0,341; sınır 0,40'a yakın; 15 dk
   0,333, 1 sa 0,300, günlük 0,073 → 0,274). Kullanıcı daha çok "süresi doldu" görür;
   hikâyelerin üçte biri karar vermeden biter. Payın ne kadarının "ufukta bekleyen tek
   kapanış"tan geldiği ayrıca ölçülmedi.
2. **Brier iyileşmesi beceri değil, taban kayması.** Her periyotta Brier ≈ kendi tabanı
   (5 dk 0,2160 / 0,2158), `r3`'te de öyle (0,2428 / 0,2428). Brier düşüşü, gerçekleşme
   oranının 0,5'ten uzaklaşmasıyla (Bernoulli varyansı küçülür) gelir; karne yine
   bilgi taşımıyor. "Kalibrasyon bozulmadı" doğru, "iyileşti" denmemeli.
3. **ECR düşüşünün bir kısmı mekanik.** Teyit kararı en az bir bar geciktirir; tutmadı
   kararından sonra ufka kalan pencere kısalır, ECR'ın arayacağı bar azalır. Eşik de %25
   geniş (ECR_own kendi geniş hedefiyle ölçülür); ECR_ref (`r3` hedefi) yine yarıdan az
   (0,071 ≤ 0,074) ama sınırda.
4. **Teyit tek başına hedefi tutmuyor** (deneme 2: −%37). Hedefe çarpan 1,25 ile ulaşılıyor;
   bu kısım "periyoda göre ölçeklenen eşik" alternatifine yakın. Teyitin kendine özgü katkısı
   SENS'tedir (deneme 5'te geniş eşik tek başına SENS'i kötüleştiriyor).
5. **Günlükte kalibrasyon farkı anlamlı kötü** (+0,017 [+0,010, +0,024]); gerçek − yalancı
   0,079 vs 0,087 (fark −0,007 [−0,016, +0,001], ayırt edilemedi). Günlük §6 sınama
   aralığında denetlenmeli; gerekirse günlükte `r3` kalır.
6. **SENS hâlâ 0,132** (5 dk): kararı veren ikinci kapanış ±1–2 adımla içeri itilince karar
   kayabiliyor. Teyit gürültüyü azaltır, kaldırmaz.
7. **5 dk gerçek − yalancı eş negatif** (−0,029; `r3` −0,052): iyileşme var ama yönlü iddia
   5 dk'da yalancı eşinden iyi değil; çıkış kuralı bunu çözmez (M26 konusu).
8. **Karar gecikmesi:** ömür ortancası 5 dk'da 9 bar (p90 19); "gerçekleşti" haberi de bir
   bar geç gelir (hedef için de iki kapanış).

## Yeniden üretim

```sh
cd rounds/RP4/experiments/harness && CARGO_TARGET_DIR=<ana>/target/wt/m25b cargo build --release
# depo kökünden:
H=<ana>/target/wt/m25b/release/harness
for p in 5m 15m 1h 1d; do $H m25 --data fixtures/local/rp4/calib --rule teyit2 --period $p; done
# report yalnız teyit2 sonuçlarıyla (sonuç klasöründe başka kurallar da olabilir):
mkdir -p /tmp/t2/M25 && cp fixtures/local/rp4/results/M25/teyit2.* /tmp/t2/M25/
$H report --results /tmp/t2 --out rounds/RP4/experiments/M25/teyit2
```

Süre (16 çekirdek, başka koşularla eşzamanlı): 5 dk 60–100 s, 15 dk 10 s, 1 sa ve 1 gün < 1 s.
