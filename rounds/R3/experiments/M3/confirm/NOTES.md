# M3 adayı `confirm` — teyit merkezli birleşim

Savunucu notu. Kod: `src/lib.rs` (kural ve parametreler), `src/tests.rs` (11 birim testi).
Ölçüm: `harness-sim m3 run --alt confirm --data synthetic|real`; özetler
`rounds/R3/experiments/M3/raw/confirm/summary-*.md`. Öteki adaylar görülmeden yazıldı.

## Kural

Dayanak K30: bir bulgunun gücü, onu kaç **bağımsız kanıt ailesinin** teyit ettiğiyle artar;
aynı aileden beş gösterge tek teyittir. Bu yüzden sıralama önce **net teyit sayısına**,
sonra ikincil kanıta bakar (sözlük sırası).

1. **Net teyit** `c = destek − karşıt − ½·bitti`
   - destek: hikâyeyi teyit eden ayrı aileler (`story.families`),
   - karşıt: hikâyeye katılmış, ters yönlü olguların ayrı aileleri,
   - bitti: hikâyeyi başlatan durumun kendi `exit` olgusu geldi.
2. **İkincil puan** `s ∈ [0, 1)` = ağırlıklı ortalama:
   - **karne** (ağırlık 0,5): anahtarın gerçekleşme oranı, bütün anahtarların oranına doğru
     30 sözde gözlemle büzülür; fark `0,5 + 2·(p_anahtar − p_hepsi)`, `[0, 1]`'e kırpılır.
     Karne boşken tam 0,5 (tarafsız).
   - **nadirlik** (0,3): türün bütün hikâyeler içindeki payı (`−ln q / −ln 0,02`) ile bu
     enstrümandaki aynı tür+yön sayısının ortalama enstrümana oranının (`1,5 − 0,5·ρ`)
     yarı yarıya ortalaması.
   - **güç** (0,2): `story.strength`.
3. **Önem** = `c + s`. `s < 1` olduğundan bir aile fazlası her zaman bütün ikincil
   kanıttan ağır basar.
4. **Kademe**

   | Net teyit | Kademe |
   |-----------|--------|
   | `c ≥ 2` (iki ya da daha çok bağımsız aile) | 1 |
   | `1 ≤ c < 2` (tek aile) | `s ≥ 0,69` → 1, `s ≥ 0,58` → 2, değilse 3 |
   | `½ ≤ c < 1` (tek aile, durumu bitti) | `s ≥ 0,69` → 2, değilse 3 |
   | `c < ½` (çelişkili) | 3 |

Kural saf bir fonksiyondur: yalnız hikâyeyi ve `RankCtx`'i okur; durum, saat, rastgelelik yok.
Doğumda ve her geçişte yeniden çağrıldığı için ikinci bir aile gelince hikâye kademe 1'e
**yükselir**, ters aile ya da durum sonu gelince **düşer**.

## Ağırlıklar ve neden

- **Teyit önce, sözlük sırasıyla.** Öncül budur; ağırlıklı toplamda bir aileyi "0,4 puan"
  yapmak, güçlü tek aileli bir olgunun iki aileli bir hikâyeyi geçmesine izin verir ve
  K30'u sulandırır. Test: `one_more_family_always_outranks_secondary_evidence`.
- **Tek aileli hikâye de kademe 1 olabilir (dar kapı).** Aksi hâlde akış verisi olmayan
  her kullanıcının (gerçek veride olduğu gibi) "yalnız önemliler" listesi sonsuza dek boş
  kalır (başvuru `families` gerçek veride kademe 1'e hiç hikâye koymuyor: sıfır gün 1,000).
- **Karne 0,5, nadirlik 0,3, güç 0,2.** Öncülün sırası "güç, sonra karne/nadirlik"; ölçüm
  bunu desteklemedi ve sırayı ölçüme göre değiştirdim:
  - Güç dedektörler arasında karşılaştırılabilir değil (sentetik medyan: `extreme.rsi`
    0,06, `flow.institutional` 0,88, `continuation.ma_stack` çeyreklerin biri 1,00'da
    doygun). Güç beşte birlerine göre gerçekleşme payı düz (sentetik 0,45–0,51, gerçek
    0,47–0,56; tekdüze değil). Ağır güç ağırlığı kademe 1'i `ma_stack`'e ve kurumsal akışa
    boğuyordu (deneme: güç 0,3–0,5 → sentetik kademe 1 payı %20, K1 − K3 daha düşük).
  - Karne bu üçünden bilgi taşıyan tek terim (tür başına gerçekleşme 0,43–0,96).
  - Nadirlik az görülen türü ve sık tetiklenmeyen enstrümanı öne alır; dikkat bütçesini
    "gürültülü" enstrümandan korur.
- **Eşikler (θ₁ = 0,69, θ₂ = 0,58)** aşağıdaki bütçeye göre **yalnız tohum 1–3**'te
  seçildi; tohum 4–6 ve gerçek veri sınama olarak ayrı tutuldu (tablo aşağıda).

### Hedef dikkat bütçesi

30 enstrümanlık bir izleme listesinde (sentetik pazar) günde ~10,7 hikâye doğuyor.

| Seviye | Hedef | Gerekçe |
|--------|-------|---------|
| important | günde ~1 (doğumların ~%10'u; enstrüman başına ayda ~1) | "Geleceğin yatırımcısı" varsayılanı: az ama önemli (K31). Günde bir itme okunur; sıfır gün oranı Poisson'dan kötü olmamalı. |
| balanced | günde ~4–5 (~%40) | Bir oturuşta taranabilecek liste. |
| all | hepsi | Alışkın kullanıcı listesi. |

Bu bütçe kademe 1'in yarısını gerçek teyitli (≥ 2 aile) hikâyelere, yarısını tek aileli
"istisnai" hikâyelere bırakıyor (sentetik).

## Sonuç tablosu (başvuru tema + sonuç; `harness-sim m3`)

**Sentetik, tohum 1–3 havuzu** (başvuru `families` ve control karşılaştırma için README'den):

| Sıralayıcı | Günlük doğum ort. important / balanced / all | Sıfır gün (imp.) | DK (imp.) | Rejim oranı (imp.) | Tohum farkı göreli (imp.) | 5 günde tekrar (imp.) | En uzun dizi (imp.) | K1 / K2 / K3 gerçekleşen | Taban | K1 − K3 | İleriye bakma |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **confirm** | **1,16 / 4,51 / 10,69** | **0,327** | 0,98 | **2,56** | 0,165 | 0,195 | 4 | **0,543** / 0,471 / 0,458 | 0,471 | **+0,084** | geçti (9 kesim) |
| başvuru `families` | 0,64 / 5,58 / 10,69 | 0,520 | 1,22 | 6,05 | 0,066 | 0,080 | 2 | 0,470 / 0,485 / 0,458 | 0,471 | +0,011 | geçti |
| control `round_robin` | 3,56 / 7,13 / 10,69 | 0,002 | — | — | — | 0,453 | — | 0,465 / 0,473 / 0,477 | 0,471 | −0,012 | geçti |

Tohum başına (important ort. · sıfır gün · K1 − K3): s1 1,13 · 0,340 · +0,100;
s2 1,27 · 0,284 · +0,063; s3 1,08 · 0,356 · +0,092.

**Sınama tohumları 4–6** (eşik seçiminde kullanılmadı; ham kayıt depoda değil, yeniden
üretmek için `--seeds 4,5,6`): important 1,14 / 1,18 / 1,15; balanced 4,58 / 4,90 / 4,84;
sıfır gün 0,333 / 0,336 / 0,351; 5 günde tekrar 0,194 / 0,201 / 0,178; K1 − K3 +0,096 /
+0,082 / +0,086; ileriye bakma geçti. Tohum 1–3 ile aynı düzeyde: eşikler tek tohuma
oturmamış.

**Gerçek veri** (6 sembol, akış yok; yalnız toplu):

| Küme | important / balanced / all günlük ort. | Sıfır gün (imp.) | 5 günde tekrar (imp. / bal.) | K1 / K2 / K3 gerçekleşen | Taban | K1 − K3 | İleriye bakma |
|---|---|---|---|---|---|---|---|
| real-1day | 0,04 / 0,36 / 1,30 | 0,960 | 0,000 / 0,271 | 0,990 / 0,493 / 0,543 | 0,546 | +0,447 | geçti |
| real-1hour | 0,39 / 2,98 / 10,37 | 0,723 | 0,215 / 0,935 | 0,948 / 0,456 / 0,482 | 0,493 | +0,466 | geçti |
| başvuru real-1day | 0,00 / 0,53 / 1,30 | 1,000 | — | — / 0,550 / 0,543 | 0,546 | — | geçti |

### Ayrıştırma (düzenek dışı tanı; doğumdaki kademe, ilk 50 bar hariç)

Düzeneğin tablosu K1 − K3'ün nereden geldiğini göstermiyor; aynı koşuyu hikâye düzeyinde
dökerek ayırdım (geçici tanı programı, depoya girmedi):

| | Sentetik s1–3 | Gerçek |
|---|---|---|
| Kademe payları 1 / 2 / 3 | 0,108 / 0,314 / 0,578 | 0,036 / 0,246 / 0,717 |
| Kademe 1 içinde ≥ 2 aileli (teyitli) | %56 | %0 |
| Kademe 1 içinde `squeeze.bandwidth` | %14 | %94 |
| ≥ 2 aileli hikâyelerin gerçekleşen payı | 0,468 (taban 0,471) | — (hiç yok) |
| K1 − K3, `squeeze` hariç | **+0,016** | **+0,026** |
| Doğumda kademe 2–3 olup sonradan ≥ 2 aileye yükselen | 132 hikâye | 0 |

## Zayıflıklar (dürüstçe)

1. **Teyit, başvuru kurallarıyla gerçekleşmeyi öngörmüyor.** Sentetikte ≥ 2 aileli hikâyeler
   0,468 gerçekleşiyor, taban 0,471. Öncülün merkezindeki terim, bu ölçüde, bilgi taşımıyor;
   K1 − K3'ün çoğu karneden geliyor.
2. **K1 − K3'ün büyük kısmı `squeeze`'den.** Başvuru sonuç kuralı yönsüz hikâyeyi "\|hareket\|
   ≥ 1 ATR" ile gerçekleşmiş sayıyor; sıkışmalar %96–99 gerçekleşiyor. Karne bunu doğru
   okuyup sıkışmayı kademe 1'e taşıyor. `squeeze` hariç fark yalnız +0,016 / +0,026.
   M5'in seçeceği sonuç kuralı yönsüz hikâyeyi farklı tanımlarsa bu kazanç büyük ölçüde
   kaybolur. Gerçek veride "yalnız önemliler" listesi neredeyse yalnız sıkışmadır.
3. **Başvuru teması teyidi yapısal olarak kısıtlıyor.** Tema = tür + yön; hacim olgusu
   (`volume_surge`, tür = akış) bir kırılım hikâyesine hiç katılamıyor. Bu yüzden ≥ 2 aile
   yalnız akış türünde (kurumsal akış + hacim) oluşuyor; gerçek veride akış olmadığı için
   hiç oluşmuyor. Öncülün asıl gücü, türler arası teyidi birleştiren bir M4 temasıyla
   ortaya çıkar; bu koşullarda ölçülemedi.
4. **Bütçe veriye göre kayıyor.** Kademe 1'in yarısı teyitten geldiği için, akış verisi
   olmayan kullanıcıda bütçe kendiliğinden daralıyor (gerçek: doğumların %3,6'sı; günlükte
   günde 0,04, sıfır gün %96). Bu öncülün bilinçli sonucudur ("teyit yoksa dar kapı") ama
   "günde ~1" sözü yalnız teyit kaynağı varken tutuyor.
5. **Güç karşılaştırılamıyor.** Dedektörlerin güç ölçeği farklı; güç terimi bu yüzden
   küçük tutuldu ve sıralamaya pek katkı vermiyor. Doğru yeri olgu katmanı (güçlerin
   dedektörler arası ayarlanması) ya da `RankCtx`'e anahtar başına güç yüzdeliği eklemek.
6. **Tekrar.** important seviyesinde 5 gün içinde tekrar 0,195; bu, günde 1,16 kapsülün 30
   enstrümana rastgele dağılmasının beklenen değeri kadar (~0,21): kümelenme yok ama
   bastırma da yok. `RankCtx` yakın geçmişi (son N gün) taşımadığı için saf bir sıralayıcı
   yakın tekrarı göremiyor.
7. **Rejim oranı 2,56**: oynak rejimde important doğumları yatay rejimin ~2,5 katı (all
   seviyesinde zaten 1,73). Başvurudan (6,05) iyi, ama sabit eşik oynaklıkla gelen olgu
   bolluğunu kısmen geçiriyor.
8. **Soğuk başlangıç.** Karne boşken karne terimi 0,5; ilk aylarda kademe 1'e tek aileli
   hikâye girmesi zorlaşır, yalnız teyitliler ve nadir türler girer.

## Öneriler (kapsam dışı)

- M3 seçimi, seçilen M4 teması ve M5 sonuç kuralıyla yeniden koşulmalı (SPEC zaten
  öngörüyor); özellikle yönsüz hikâyenin "gerçekleşti" tanımı K1 − K3'ü belirliyor.
- `RankCtx`'e "anahtar başına güç yüzdeliği" ve "bu enstrümanda son N barda doğan hikâye
  sayısı" eklenirse güç terimi anlam kazanır ve yakın tekrar bastırılabilir; ikisi de
  geçmişten hesaplanır (ileriye bakmaz).
