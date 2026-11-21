# M25d `olcek` — periyoda göre ölçeklenen eşik

Savunucu notları. Şartname `../../harness/SPEC.md` (değiştirilmedi). Veri yalnız kalibrasyon
kopyası `fixtures/local/rp4/calib/`; `--split test` çalıştırılmadı. Aşağıda yalnız toplu sayılar.

## Kural

`r3` ile aynı üçlü bariyer (P0 = doğum kapanışı, doğumdaki ATR14, ilk değen kapanış karar
verir, türe göre ufuk 20/15/10, yönsüz kural aynı). Tek fark, **yönlü** bariyerin periyot
çarpanı:

```
k = 0,3 · √h · m(P),    m(P) = max(1, 480 dk / P)^β,   β = 0,10
```

- `P` barların kendisinden okunur: `bars[..=doğum]`'un son 64 barı içindeki en küçük pozitif
  zaman adımı (gece/hafta sonu boşlukları daha büyük olduğundan en küçük adım bar periyodudur).
  Hikâye üst verisi (`series`) gerekmez; belirleyici, ileriye bakmaz.
- 480 dk = XIST sürekli seans (10:00–18:00). Çarpan: 5 dk 96^0,1 = **1,58**; 15 dk 32^0,1 =
  **1,41**; 1 sa 8^0,1 = **1,23**; günlük ve üstü **1** (kural orada `r3`'ün aynısı; testle ve
  ölçümle bayt düzeyinde aynı sonuç).
- Yönsüz (sıkışma, yönsüz akış) bariyeri değişmedi (`0,65·√h·ATR100`).
- Ufuk ölçeklenmedi: `OutcomeRule::horizon(kind, time)` periyodu bilmiyor (bar almıyor);
  ufku periyoda bağlamak trait değişikliği ister (aşağıda zayıf yanlar).
- Sabitler: `β` (ayarlanan tek sabit) + `r3`'ün 7 sabiti = 8 (kayıtta `constants: 8`).
  480 dk takvimden, 64 bar mekanik geri bakış; ayarlanmadı.

## Denemeler (kalibrasyon, 5 dk, yönlü satır; başvuru `r3`: ECR_own 0,148, SENS 0,212, p_exp 0,118)

| # | β (5 dk çarpanı) | ECR_own | SENS | p_exp | not |
|---|---|---|---|---|---|
| 1 | 0,10 (1,58) | 0,066 | 0,278 | 0,311 | önbellek anahtarında tür yoktu: `sens_replay_mismatch` 309 → hata, sonuç geçersiz |
| 2 | 0,10 (1,58) | 0,065 | 0,279 | 0,315 | anahtara tür eklendi; mismatch 0 |
| 3 | 0,08 (1,44) | 0,077 | 0,272 | 0,267 | ECR_own > 0,074 (½ başvuru): ölçüt 1 kalır |

Seçilen: **β = 0,10** (ölçüt 1'i geçen en küçük denenen değer). Başka değer denenmedi.
`SENS` her β > 0'da başvurudan büyük (aşağıda neden).

## Kalibrasyon ölçüleri (β = 0,10; yönlü; aralık sembol kümeli bootstrap %95; fark = olcek − r3, eşli)

| Periyot | n | p_real | p_inv | p_exp | ECR_own | ECR_ref | REV | Brier (taban) | kal. farkı | SENS | gerçek − yalancı |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 5 dk | 341.131 | 0,331 | 0,354 | 0,315 | 0,065 [0,061, 0,068] | 0,090 | 0,169 | 0,222 (0,222) | 0,025 | 0,279 [0,261, 0,299] | −0,022 |
| 15 dk | 126.471 | 0,368 | 0,374 | 0,258 | 0,070 [0,067, 0,073] | 0,092 | 0,168 | 0,233 (0,233) | 0,031 | 0,186 | −0,005 |
| 1 sa | 32.993 | 0,432 | 0,403 | 0,165 | 0,083 [0,078, 0,088] | 0,098 | 0,177 | 0,248 (0,245) | 0,059 | 0,104 | 0,029 |
| 1 gün | 28.122 | 0,507 | 0,420 | 0,073 | 0,159 | 0,159 | 0,244 | 0,246 (0,250) | 0,065 | 0,091 | 0,087 |

Fark (olcek − r3):

| Periyot | ECR_own | ECR_ref | REV | Brier | kal. farkı | SENS | p_exp | gerçek − yalancı |
|---|---|---|---|---|---|---|---|---|
| 5 dk | −0,083 [−0,085, −0,081] | −0,058 [−0,060, −0,056] | −0,081 | −0,021 [−0,022, −0,020] | +0,004 | **+0,068 [+0,061, +0,075]** | +0,196 | +0,030 |
| 15 dk | −0,065 [−0,068, −0,063] | −0,044 | −0,067 | −0,012 | +0,004 | +0,030 [+0,024, +0,037] | +0,139 | +0,013 |
| 1 sa | −0,045 [−0,049, −0,041] | −0,030 | −0,040 | −0,003 [−0,004, −0,003] | +0,002 | +0,013 [+0,007, +0,018] | +0,074 | +0,001 [−0,004, +0,006] |
| 1 gün | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

Yönsüz satır dört periyotta `r3` ile aynı (fark 0). `r3` başvurusu aynı koşudan: 15 dk ECR_own
0,135, SENS 0,156; 1 sa ECR_own 0,128, SENS 0,091; günlük ECR_own 0,159.

## SPEC §4 (mekanik, `harness report`)

| Ölçüt | Durum |
|---|---|
| 1 — 5 dk ECR_own ≤ ½ başvuru (0,074), üst uç < 0 | **geçti** (0,065; fark üst ucu −0,081) |
| 2 — ECR_ref büyümüyor | geçti (−0,058) |
| 3 — Brier 5 dk / 1 sa, kalibrasyon farkı ≤ +0,03 | geçti (Brier düştü; kal. farkı +0,004) |
| 4 — 5 dk p_exp ≤ 0,40 | geçti (0,315) |
| 5 — 5 dk SENS büyümüyor | **kaldı** (+0,068, alt uç +0,061 > 0) |
| **Aday** | **hayır** |

Günlük periyotta kural `r3`'ün aynısı (§6 kötüleşme yok; ölçüm farkı tam 0).

## Zayıf yanlar (dürüstçe)

1. **SENS kaçınılmaz olarak artıyor.** Bariyeri genişletmek süresi dolan payını büyütüyor
   (5 dk 0,118 → 0,315) ve süresi dolanlar tek bar oynatmasına çok duyarlı (ufkun son barı ve
   eşiğe en yakın bar ±2 adım oynatılınca karara dönebiliyor). İki kuralın toplamından kaba
   ayrıştırma: karar verilmiş hikâyelerde SENS ≈ 0,17, süresi dolanlarda ≈ 0,51 (her iki
   kuralda yaklaşık aynı kabul edilerek). ECR'yi yarıya indirecek her genişlik bu yüzden
   SENS'i başvurunun üstüne çıkarıyor (β = 0,08 de: +0,060). Bu kural ailesi §4'ü geçemiyor.
2. **Ufuk ölçeklenmiyor.** Difüzyon ölçeğinde eşik `m` kat büyüyünce ufkun `m²` kat uzaması
   gerekir ki süresi dolan payı korunsun; `OutcomeRule::horizon(kind, time)` bar/periyot
   almadığı için bu kuralda yapılamaz (kural içinde durum tutmak saflığı bozar). Ufuk da
   ölçeklenseydi ECR penceresi de uzayacağından ECR kazancının ne kadarı kalırdı bilinmiyor.
3. **"Gürültü/sinyal" gerekçesi verilerle zayıf.** `r3`'ün ECR_own'u periyoda göre pek
   değişmiyor (5 dk 0,148; 15 dk 0,135; 1 sa 0,128; günlük 0,159). Erken kapanma gün içine özgü
   değil; simetrik dar bariyerin rastgele yürüyüşteki doğal sonucu. Kazanç periyot ölçeğinden
   değil, bariyerin genişlemesinden geliyor; aynı ECR'yi sabit `k_scale = 1,58` de verirdi
   (günlükte de bozarak).
4. **Karne okuması değişiyor.** p_real 0,415 → 0,331 (süresi dolan ıska sayılır); karar
   verilenler içinde isabet 0,471 → 0,483. Brier'in düşmesi beceri değil: taban da aynı
   miktarda düşüyor (0,222 = 0,222), oran 0,5'ten uzaklaştığı için.
5. **Ömür uzuyor** (5 dk ortanca 4 → 8 bar, p90 11 → 19): akışta hikâye daha uzun açık kalır.
6. Periyot barlardan okunuyor: verisi seyrek (çok boşluklu) bir sembolde son 64 bar içinde
   hiç ardışık bar yoksa periyot büyük okunur ve çarpan küçülür (güvenli yön: `r3`'e yaklaşır).
7. Sabit sayısı `r3`'ten bir fazla (8).

## Yeniden üretim

```sh
cd rounds/RP4/experiments/harness && export CARGO_TARGET_DIR=<ana ağaç>/target/wt/m25d
cargo build --release
H=$CARGO_TARGET_DIR/release/harness   # depo kökünden:
for P in 5m 15m 1h 1d; do $H m25 --data fixtures/local/rp4/calib --rule olcek --period $P --split calib; done
$H report --out <dir>
```

Sonuçlar `fixtures/local/rp4/results/M25/olcek.<periyot>.calib.json`; iki koşu bayt eşit
(deneme 2 ile son koşu `cmp` eşit). 5 dk koşu ~44 s (iki kural, 16 çekirdek).
