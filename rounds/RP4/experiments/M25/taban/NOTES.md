# M25 alternatifi `taban` — eşiğe en az N fiyat adımı tabanı

Savunucu notları (RP4, dal `rp4/m25a-taban`). Yalnız kalibrasyon kopyası
(`fixtures/local/rp4/calib/`, `--split calib`) kullanıldı; `--split test` koşulmadı.
Burada yalnız toplu sayılar var. Tam tablo: `M25-calib.md` / `M25-calib.csv` (`harness report`,
yalnız bu kuralın dört sonucuyla). Not: tablodaki "taban" sütunu düzeneğin **Brier tabanıdır**,
kuralın adı değil.

## Kural

`r3` (`qp_capsule::outcome::Barrier`) ile aynı: tür başına ufuk (20/15/10), kapanışla karar,
ilk değen kapanış karar verir, simetrik hedef/durdurma. Tek fark eşik:

- yönlü: hedef = durdurma = `max(0,3·√h·ATR14, N·tick(P0)) − 1e-6·tick(P0)`
- yönsüz: genişleme eşiği = `max(0,65·√h·ATR100, N·tick(P0)) − 1e-6·tick(P0)`

`P0` doğum kapanışı, `tick` SPEC §1 tablosu. Seçilen: **`N = 8`**, bütün periyotlarda aynı.
Sabit sayısı 8 (`r3`'ün 7'si + `N`; `1e-6` ayar değil, yuvarlama payı).

**Yuvarlama payı neden var:** kapanışlar fiyat adımı ızgarasında; taban bağladığında eşik tam
`N` adımdır ve `c − P0 ≥ N·tick` f64'te bıçak sırtıdır (10,12 − 10,00 = 0,1199…). 10–30 TL
arasındaki her başlangıç fiyatında N = 8 ve 12 için, ±yönde tam N adımlık hareketlerin
%34'ü (4.004'te 1.363) paysız sayılmıyordu. Pay bir adımın milyonda biri; gerçek bir fiyatla
verilen kararı değiştirmez. Testte (`exact_n_steps_decides`) sabitlendi.

Saflık: `judge` yalnız `(hikâye, bars[..=now])`'un fonksiyonu; saat, ağ, rastgelelik, dosya
yok. Doğum eşiği örnek başına bellekte tutulur; anahtar araç, seri, **tür ve yön**, doğum
barının indisi, zamanı ve değerleri (ilk taslakta tür/yön anahtarda yoktu, aynı barda doğan
farklı türden iki hikâye birbirinin eşiğini alıyordu; `zero_floor_is_r3` testi yakaladı,
düzeltildi, hiçbir ölçüm o taslakla yapılmadı). Düzeneğin `sens_replay_mismatch` sayacı
dört periyotta 0.

## Denemeler (yalnız 5 dk, kalibrasyon, yönlü satır)

Başvuru `r3`: ECR_own 0,148, p_exp 0,118, SENS 0,212. Hedef ECR_own ≤ 0,074; §4(4) p_exp ≤ 0,40.
R6 §10'a göre 5 dk'da `r3` eşiği ≈ 4 adım, bu yüzden N = 4'ten başlandı.

| # | N | pay | Neden | ECR_own | ECR_ref | SENS | p_exp | Sonuç |
|---|---|---|---|---|---|---|---|---|
| 1 | 4 | yok | R6'daki tipik eşik; taban yalnız en dar hikâyelerde bağlar | 0,120 | 0,129 | 0,239 (fark +0,028, aralık > 0) | 0,212 | ECR yetmez, SENS kötüleşti |
| 2 | 8 | yok | iki katı | 0,067 | 0,085 | 0,185 | 0,435 | ECR tuttu, p_exp > 0,40 |
| 3 | 16 | yok | üst sınırı görmek | 0,026 | 0,044 | 0,079 | 0,740 | hikâyelerin ¾'ü karar vermeden ölüyor |
| — | | | N = 4'teki SENS artışını incelerken yuvarlama bıçak sırtı bulundu (yukarı); pay eklendi, kalan denemeler payla | | | | | |
| 4 | 6 | var | ECR ile p_exp arasını daraltmak | 0,093 | 0,107 | 0,224 | 0,312 | ECR yetmez |
| 5 | 7 | var | | 0,080 | 0,096 | 0,211 | 0,370 | ECR 0,0795 > 0,074 (az farkla) |
| 6 | 8 | var | | 0,069 | 0,087 | 0,193 | 0,426 | ECR tuttu, p_exp 0,426 > 0,40 |

Toplam altı koşu (5 dk), üçü paysız üçü paylı. Ara (kesirli) N denenmedi: kapanışlar tam adım
oynadığından taban bağladığında 7 < N ≤ 8 ile N = 8 aynı kararı verir; fark yalnız ATR
kısmının 7–8 adım arasında kaldığı hikâyelerde olur ve ancak ECR ile p_exp'i birlikte aynı
yöne kaydırır.

**Sonuç:** bu kural ailesinde (yalnız taban, ufuk `r3`'ünki) §4(1) ile §4(4)'ü aynı anda
tutan bir N yok: ECR_own'u yarıya indiren taban, ufku değiştirmeden hikâyelerin %42'sini
"süresi doldu"ya itiyor. **N = 8** seçildi; çünkü round hedefi (ECR yarıya) yalnız onunla
tutuyor ve SENS de yalnız onunla anlamlı iyileşiyor; §4(4)'te kalıyor (aday değil).
N = 7 §4(4)'ü geçer ama §4(1)'de kalır; o da aday değil.

**Periyoda göre fark:** gerekmedi, denenmedi. ATR periyotla büyüdüğü için N = 8 tabanı 15 dk'da
seyrek, 1 sa'te çok seyrek bağlıyor; günlükte bağlaması beklenmezdi ama bağlıyor (aşağıda
zayıf yan 4). Öneri: kural seçilirse gün içinde `taban`, günlükte `r3` (SPEC §4 periyoda göre
ayrı seçime izin veriyor).

## Dört periyot, kalibrasyon (N = 8, yönlü satır; köşeli = %95 aralık, fark = taban − r3)

| Ölçü | 5 dk | 15 dk | 1 sa | 1 gün |
|---|---|---|---|---|
| n (r3) | 331.377 (383.867) | 130.466 (138.347) | 34.437 (35.040) | 27.006 (28.122) |
| ECR_own | **0,069** [0,063, 0,074] (r3 0,148) | 0,107 (0,136) | 0,123 (0,128) | 0,151 (0,159) |
| ECR_own farkı | −0,079 [−0,085, −0,073] | −0,029 [−0,034, −0,024] | −0,005 [−0,008, −0,002] | −0,008 [−0,012, −0,004] |
| ECR_ref | 0,087 (fark −0,061 [−0,066, −0,056]) | 0,115 (−0,021) | 0,125 (−0,003) | 0,153 (−0,006) |
| SENS | 0,193 (r3 0,212; fark −0,018 [−0,040, −0,000]) | 0,150 (0,156; −0,006 [−0,017, 0,002]) | 0,091 (0,091; 0,000) | 0,064 (0,091; −0,027) |
| p_real / p_inv / p_exp | 0,284 / 0,290 / **0,426** | 0,382 / 0,388 / 0,230 | 0,450 / 0,424 / 0,126 | 0,470 / 0,388 / 0,143 |
| r3 p_real / p_inv / p_exp | 0,415 / 0,467 / 0,118 | 0,432 / 0,450 / 0,118 | 0,469 / 0,441 / 0,091 | 0,507 / 0,420 / 0,073 |
| Brier (taban) | 0,203 (0,203) | 0,236 (0,236) | 0,249 (0,247) | 0,245 (0,249) |
| Brier farkı | −0,040 [−0,047, −0,033] | −0,009 | −0,001 [−0,003, −0,001] | −0,001 [−0,004, 0,001] |
| Kalibrasyon farkı (fark) | 0,023 (+0,001 [−0,003, 0,018]) | 0,030 (+0,004) | 0,056 (−0,001) | 0,092 (**+0,028** [0,024, 0,033]) |
| REV | 0,173 (r3 0,250) | 0,211 (0,235) | 0,212 (0,216) | 0,237 (0,244) |
| Ömür ortanca/p90 (r3) | 10/20 (4/11) | 5/13 (4/10) | 5/10 (4/10) | 4/10 (3/10) |
| Gerçek − yalancı (r3) | −0,006 (−0,052) | −0,006 (−0,018) | 0,026 (0,028) | 0,082 (0,087) |

Yönsüz satır (5 dk): p_real 0,310 (r3 0,395), SENS 0,077 (r3 0,187), Brier farkı −0,025.

## SPEC §4 (kalibrasyon, `harness report` mekanik puanı)

| Koşul | Durum |
|---|---|
| 1 5 dk ECR_own ≤ ½ başvuru, üst uç < 0 | **geçti** (0,069 ≤ 0,074; fark üst ucu −0,073) |
| 2 ECR_ref büyük değil | geçti (−0,061 [−0,066, −0,056]) |
| 3 Brier (5 dk, 1 sa) kötüleşmiyor, kalibrasyon farkı ≤ +0,03 | geçti (Brier aralıkları < 0; 5 dk kal. farkı farkı +0,001) |
| 4 5 dk süresi dolan ≤ 0,40 | **kaldı** (0,426 [0,388, 0,464]) |
| 5 5 dk SENS büyük değil | geçti (fark −0,018 [−0,040, −0,000]) |
| **Aday** | **hayır** (yalnız 4. koşul) |

## Zayıf yanlar (dürüstçe)

1. **Karar vermeden ölme.** 5 dk'da hikâyelerin %43'ü süresi doldu ile kapanıyor (`r3` %12).
   ECR'yi yarıya indiren şeyin büyük kısmı hikâyenin daha geç ve daha seyrek karar vermesi:
   p_real 0,415 → 0,284, p_inv 0,467 → 0,290. Kullanıcı "tutmadı" yerine çoğunlukla "süre
   doldu" görecek; bu da bir tür sessiz kapanış. Ufuk `r3`'ünki kaldığı için taban, bariyeri
   genişletip zamanı genişletmiyor (`r3`'ün kendi tasarımı `k ∝ √h`; tutarlı bir taban,
   bağladığında ufku `(eşik/kA)²` oranında uzatmalı — trait ufku hikâye başına değil tür başına
   verdiği için bu kural yapamıyor; bkz. öneri).
2. **Brier iyileşmesi mekanik.** 5 dk Brier 0,243 → 0,203, ama taban (genel oran) da aynı
   ölçüde iniyor (0,203): karne bilgi eklemiyor, yalnız süresi dolanlar `y = 0` sayıldığı için
   oran düşüyor. "Kalibrasyonu iyileştirdi" denemez; "bozmadı" denebilir.
3. **ECR_own'un kendi eşiğiyle ölçülmesi.** Hedef daha geniş olduğu için ECR_own kendiliğinden
   düşer; bu yüzden ECR_ref'e bakılmalı: 0,148 → 0,087, yani `r3` eşiğine göre de erken
   kapanma %41 azaldı, ama yarıya değil.
4. **Günlükte taban da bağlıyor ve kalibrasyon farkı kötüleşiyor.** 1 gün p_exp 0,073 → 0,143,
   kalibrasyon farkı +0,028 [0,024, 0,033] (sınıra yakın). Olası neden: günlük seri
   **düzeltilmiş** fiyatlarla (bölünmelerle 2018 fiyatları çok küçülüyor), adım tablosu ise o
   küçük düzeltilmiş fiyata uygulanıyor; o günkü gerçek adım bu değildi. Taban günlükte
   anlamsız; kullanılırsa yalnız gün içinde.
5. **Adım sınırı.** Tablo `≤` diyor, borsa sınırda üst bandı kullanıyor (harness README); P0
   tam 20/50/100… olduğunda taban yarı ya da çift genişlikte. Etkisi küçük, sayılmadı.
6. **Sabit sayısı.** `r3`'e bir sabit ekliyor (8 > 7); `N` yalnız 5 dk'ya göre seçildi.
7. **Yönsüz hikâyeler de değişiyor.** Taban yönsüz genişleme eşiğine de uygulandı (tek kural,
   tek sabit); 5 dk yönsüz p_real 0,395 → 0,310. Yönsüzü `r3`'te bırakmak ayrı bir seçenek,
   denenmedi.

## Öneriler (kapsam dışı, yapılmadı)

- Taban + bağladığında ufku diffüzif ölçekle uzatmak (`h' = h·(eşik/kA)²`, üst sınırlı).
  `OutcomeRule::horizon` tür başına olduğu için motor/trait değişikliği ister (hikâye başına
  ufuk); §4(4)'ü taban kaybetmeden geçebilecek tek doğrudan yol bu görünüyor.
- Taban yalnız durdurmaya (hedef `r3`): ECR'yi doğrudan hedefler, p_real'i korur ama simetriyi
  ve "drift yoksa %50" okumasını bozar; ölçülmedi.
