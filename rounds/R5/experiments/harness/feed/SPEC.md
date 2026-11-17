# M19 — Akış önceliği ve gizleme eşiği: ölçüm düzeneği (şartname)

Yazan: R5/T2 `t2-lang` alt ajanı, 2026-11-17/18, **hiçbir M19 alternatifi yazılmadan önce** (K39;
kabul ölçütü 2 ve 7). Bu dosyanın ve `tool/` kodunun commit'i M19 alternatif dallarından öncedir.
**Seçim kuralı (§4) sonuç görülmeden yazılmıştır** (sabitleri `tool/src/main.rs`'de: `MAX_EMPTY`,
`MIN_KEPT`, `ORDER_TIE`, `BOOT_SEED`); değişirse §7'ye tarihli ek yazılır.
Girdi: DECISIONS K42 (A22), K53; TASKS "Gizleme eşiği"; GOAL M19 satırı; RESEARCH §13 ve F15, F16
(yanlış alarm ve güven: güvenilirlik ≈ 0,70 altında otomasyon zararlı; aynı hisse tek satır).

## 0. İlkeler

1. **Ürün kodu değişmez.** Benzetim ürünün motorunu (`EngineConfig::r3()`, yedi dedektör,
   `qp_capsule::pipeline::simulate_engine`), kitle süzgecini (`qp_capsule::admits`, K42/A22 dahil),
   geçerliliği (`sentence::condition_now_memo`, `validity_now`) ve karne sayım kuralını
   (`LedgerRecord::counts`) dışarıdan çağırır. Alternatifler (M19 görevi `crates/qp-capsule/`,
   `crates/qp-session/` içinde eşik ve sıralamayı üründe kurar) burada ölçülen **kural adıyla**
   karşılaştırılır: ürün kuralı bu düzenekteki `Rule`/`Order` değerlerinden biriyle aynı kararları
   vermelidir (alternatif kendi birim testinde `feed_on` ile karşılaştırır).
2. **Geleceğe bakmama.** Gün `d`'nin kararı yalnız: `d` ya da öncesindeki geçişlerle hikâye anlık
   görüntüsü, `cutoff[d]`'den (o günün son bar zamanı + 1) **kesin önce** kapanmış karne satırları,
   `d`'ye kadarki barlarla durum koşulu. `d`'den sonrası yalnız **değerlendirmede** (kalanların ve
   gizlenenlerin sonradan tutma oranı) okunur.
3. **Gizlenen karneye işler** (K53 (2)): karne satırları motordan gelir; gizleme karara girer, karneye
   girmez. Karar karnesi ürünün `SimResult::ledger_stat`'ıyla aynıdır (test).
4. **Az örnekli ≠ yazı tura** (K53 (3)): her kural yalnız `n ≥ N`'yi yargılar; `n < N` gösterilir ve
   "az örnekli" diye işaretlenir (`Verdict::Thin`).
5. Belirleyici; tohumlar sabit. Gerçek veri (`fixtures/local/bars`) varsa kullanılır; **yalnız toplu
   istatistik** dosyaya yazılır (gün ve kapsül sayıları, oranlar); ham çıktı yok.

## 1. Benzetim

- **Veri:** sentetik `synthetic_market(30, 500, s)` + sentetik lisanslı akış, tohum s = 1, 2, 3; gerçek:
  yerel günlük barlar (31 enstrüman; akış yok). Sayılan günler: 50. bardan (ısınma) sonrası.
- **Kitle** (`qp-capsule/examples/sensitivity.rs` ile aynı): ≥ 30 sembolde ilk 10 izleme listesi,
  pozisyon 1., 4. (açığa) ve 12. (liste dışı), niyet 7.; az sembolde hepsi izlenir, ilk üçü pozisyon,
  dördüncüsü niyet. Seviye `balanced`. "Elindeki" = pozisyon ∪ niyet (K42).
- **Akış öğesi (gün `d`):** evrendeki, `d` sonunda açık ya da `d`'de kapanmış, `admits` süzgecinden
  geçen her kapsül. Her öğenin karnesi `(n, tutan)` = anahtarının (`<tür>/<yön>/<dedektör>@<sürüm>`)
  `cutoff[d]`'deki karnesi.
- **Geleceğe bakmama testi** (`no_look_ahead`, VERIFY 12): geçmiş 80'den 320'ye 6 günde bir kesilir; her
  kesimde sınır günlerinin (`D−1`, `D`) kararları (kapsül, karar, sıra, karne) tam geçmişle **aynı**.
  Düzeneğin kendisi sınandı: karar karnesini bir gün ileriden okuyan değişiklik testi düşürüyor
  (`day 79 (cut 80) differs`).
- **Karne testi** (`hidden_still_count_in_ledger`): gizlenmiş ve sonra kapanmış her kapsülün karne satırı
  var; kararların karnesi `SimResult::ledger_stat` ile eşit; satır toplamı motorunkine eşit.

## 2. Parametreler

### 2.1 Gizleme kuralları (`Rule`; N ∈ {5, 10, 20, 30})

| kural | gizle, eğer `n ≥ N` ve … | gerekçe |
|---|---|---|
| `none` | hiç | bugünkü ürün (başvuru) |
| `wilson` | tutma oranının Wilson %95 aralığı 0,5'i içeriyor | "yazı turadan ayırt edilemiyor" |
| `delta` δ ∈ {0,05; 0,10} | `|p − 0,5| < δ` | GOAL örneği |
| `bayes` δ = 0,10 | Beta(1 + r, 1 + n − r) sonsalının ≥ 0,5'i [0,5 − δ, 0,5 + δ] içinde | örnek sayısını kendiliğinden tartar |
| `rate` τ ∈ {0,55; 0,60; 0,70} | `p < τ` | RESEARCH F16: güvenilirlik 0,6–0,7 bandı (Wickens–Dixon) |

### 2.2 Sıralamalar (`Order`)

`LiveNew` (canlı = açık ve geçerli önce, sonra artık geçerli değil, sonra bugün kapanan; içinde son
geçişi yeni olan önce — kabul 7 "canlı ve yeni önce"); `Trust` (karnenin Wilson alt sınırı yüksek olan
önce, az örnekli sonda, sonra yeni); `Grouped` (enstrüman başına tek satır, satırlar en yeni geçişe göre —
RESEARCH F15, Ancker 2017).

### 2.3 K42 sorusu (`exempt`)

`exempt = true`: elindeki hissede **bugün olumsuz geçiş** (zayıfladı, geçersiz, geri çekildi) yaşayan
kapsül gizlenmez (`Verdict::Exempt`). İki biçim de her kural × sıralamada ölçülür; karar Arkin'in.

## 3. Ölçüler (`Metrics`)

- gün başına kalan (görünen) ve gizlenen öğe, satır (Grouped'da enstrüman), **boş akış günlerinin payı**,
  görünenlerde az örnekli payı;
- **kalanların sonradan tutma oranı**: doğduğu gün "göster" (n ≥ N) kararı verilen ve sonra kapanan
  kapsüllerin gerçekleşme oranı; **gizlenenlerinki**; az örneklilerinki (geri çekilenler ve hâlâ açık
  olanlar dışarıda);
- **kalan Brier becerisi** `1 − BS(p̂)/BS(0,5)`: gösterilen karnenin (p̂ = karar anındaki tutma oranı)
  sonucu yazı turadan iyi öngörüp öngörmediği; doğumlar üzerinden önyükleme %95 aralığı (1000, tohum 1);
- **eşiğin N'ye duyarlılığı**: her kural ailesi için N = 5/10/20/30 tablosu;
- **persona görevleri (akış listesinden deterministik; görüntü ve okuyucu yok):**
  - T1 "son baktığımdan beri ne değişti" (son bakış = dün kapanış): bugün geçiş yaşayan öğelerin görünme
    oranı ve **ilk 5 sırada** (bir telefon ekranı) olma oranı (`min(5, değişen)` paydasıyla);
  - T2 "elimdekinde olumsuz bir şey var mı": bugün elindeki hissede olumsuz geçiş yaşayan öğelerin görünme
    ve ilk 5'te olma oranı.
  Gerekçe: iki görev de "doğru cevap akışta, bakılan yerde mi" sorusudur; anahtar motordan bellidir,
  dil modeli gerekmez. Görsel okunurluk M16/M18'in işidir.

## 4. Seçim kuralı (önceden yazılı)

Seçim **gerçek veride** yapılır (varsa); yoksa sentetik üç tohumun birleşiminde. Sentetik pazarın
getirileri rejim süreçlidir ve dedektör karnelerini yazı turaya yakın üretir; sentetikte kural farkı
beklenmez (bilgi için raporlanır).

1. **Kapı:** testler (§1) geçer; boş akış günü payı ≤ **0,25** ve gün başına kalan ≥ **1,0**
   (`LiveNew`, muafiyetsiz). Gerekçe: K53 "boş akış dürüstçe" tasarlanır ama akışın dörtte birinden
   fazlası boşsa ürün sessizleşir; günde birden az kapsül, akışı tek bakışlık olmaktan çıkarır. Hiçbir
   kural geçemezse kapı uygulanmaz ve yazılır. `none` başvurudur, sıralanır, seçilmez.
2. **Birincil:** kalan Brier becerisi (yüksek). İlk ikinin önyükleme aralıkları örtüşüyorsa **ayırt
   edilemedi**.
3. Ayırt edilemeyenler içinde eşitlik bozucular sırasıyla: kalanların tutma oranı (yüksek), az parametre
   (`wilson` 1, öteki 2), büyük N.
4. **K42 önerisi:** muafiyetsiz T2 görünme oranı < 1 ise (gizleme elindeki olumsuz geçişi kaçırıyorsa)
   "muaf" önerilir; değilse "gerek yok".
5. **Sıralama:** seçilen kuralda (ve 4'teki muafiyet biçiminde) T1 ve T2 ilk-5 oranlarının ortalaması;
   en iyi ile `LiveNew` arasındaki fark < **0,02** ise `LiveNew` (kabul 7'nin varsayılanı).
6. İlk iki (kural + sıralama) gösterimde seçilebilir kalır.

## 5. Kuru koşu (`results/dry-run/RESULTS.md`)

Komut: `feedh run --out results/dry-run --real` (≈ 2 dk; sentetik 3 × 30 × 500 gün, gerçek 31 enstrüman
× 2220 gün). Özet:

| veri, kural (`LiveNew`, muafiyetsiz) | kalan/gün | boş gün | kalan tutma | gizlenen tutma | kalan beceri [%95] | T2 görünür |
|---|---:|---:|---:|---:|---:|---:|
| gerçek, `none` (başvuru) | 4,95 | 0,014 | 0,556 | — | 0,019 [0,003; 0,036] | 1,000 |
| gerçek, `rate` τ=0,55 N=20 | 3,18 | 0,138 | 0,583 | 0,509 | 0,024 [−0,000; 0,047] | 0,422 |
| gerçek, `delta` δ=0,05 N=20 | 3,55 | 0,096 | 0,576 | 0,513 | 0,022 [−0,002; 0,042] | 0,666 |
| gerçek, `wilson` N=30 | 3,25 | 0,180 | 0,570 | 0,519 | 0,021 [−0,001; 0,043] | 0,643 |
| gerçek, `rate` τ=0,70 N=20 | 0,20 | 0,884 | 0,591 | 0,556 | −0,085 | 0,018 |
| sentetik, `none` | 8,85 | 0,000 | 0,473 | — | −0,007 [−0,017; 0,004] | 1,000 |
| sentetik, `wilson` N=30 | 2,32 | 0,164 | 0,488 | 0,471 | −0,033 [−0,075; 0,009] | 0,248 |

Kuralın uygulanışı (gerçek veri): kapı 12/28 kural; birincil `rate τ=0,55 N=20` 0,024 ↔ `delta δ=0,05
N=20` 0,022, aralıklar örtüşüyor → **ayırt edilemedi**; eşitlik bozucuyla (kalan tutma) **`rate τ=0,55
N=20`**, ikinci `rate τ=0,55 N=10`. K42: muafiyetsiz T2 görünme 0,42 → **muafiyet önerilir** (muaf
biçimde T2 görünme 1,00). Sıralama: `Grouped` (T1/T2 ilk 5 ortalaması 0,881) > `LiveNew` (0,727) →
`Grouped`.

**Okuma notları (kuru koşunun söyledikleri, karar değil):**
- Gerçek veride hiçbir kuralın becerisi `none`'dan (0,019 [0,003; 0,036]) ayrılmıyor: gizleme kalanların
  tutma oranını 0,556'dan ≈ 0,58'e çıkarıyor, gizlenenlerinki ≈ 0,51; ama karnenin öngörü gücü
  zayıf. RESEARCH F16'nın 0,70 hedefi bu dedektörlerle ulaşılmıyor: τ = 0,70 akışın %88'ini boşaltıyor.
- K53'ün gizlemesi, K42 muafiyeti olmadan elindeki olumsuz geçişlerin yarısından fazlasını saklıyor.
- `Grouped` sıralama T1/T2'de belirgin önde; kabul 7 "canlı ve yeni önce" ile çelişmez (satır içinde
  `LiveNew`), ama bu bir **akış tasarım** kararıdır (M18/M16 ile birlikte) — Arkin'e sorulmalı.
- Seçim kuralının "birincil = beceri" tercihi, beceri farkları bu kadar küçükken pratikte eşitlik
  bozucuya (kalan tutma) düşüyor. Bu bir düzenek bulgusudur; kural değiştirilmedi.

## 6. Riskler

- Kitle sentetik ve küçük (11 enstrüman evren); kalan/gün ve boş gün oranı evren boyuna bağlı.
- Gerçek veri 31 enstrüman, akış yok (kurumsal akış dedektörü gerçekte üretmiyor).
- Persona görevleri "ilk 5" ile ölçülür: kart boyu M16'da değişirse K değişebilir (sabit `TOP_K`).
- Brier becerisi doğumlar üzerinden; tek veri setinde önyükleme aralığı geniş.

## 7. Ekler (tarihli)

- (yok)
