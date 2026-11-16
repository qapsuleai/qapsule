# M4 adayı `arc` — yön yayı

Dal: `r3/m4b-arc`. Kod: `src/lib.rs` (`theme()`, ad `arc`), testler `tests/arc.rs` (13).
Düzenekte kayıt: `harness-sim/Cargo.toml` + `src/registry.rs` birer satır. Ham ve özet:
`../raw/arc/` (sentetik ham + özet; gerçek veride yalnız toplu özet).

Bu aday öteki M4 adayı görülmeden yazıldı; `ALTERNATIVES.md` ve başka `M4/*` klasörü
açılmadı (yalnız `M4/raw/{control,reference}` özetleri).

## Öncül

Bir enstrümanda (ve seride) aynı yönde art arda gelen anlamlar tek bir hikâyenin
bölümleridir: kırılım, onu teyit eden hacim sıçraması, arkasından gelen ortalama dizilimi,
trendin sonraki kırılımları, kurumsal alış, hatta RSI'ın 70'i aşması. Okurun izlediği şey
"THYAO yukarı kırdı, hacim teyit etti, hareket sürüyor" yayıdır; hangi dedektörün
tetiklediği değil. Yay türler arası uzanır; yönü tektir.

## Kural

Kural **durumsuzdur**: her şeyi kendisine verilen açık hikâyelerden (olguları, `last_bar`,
yön) okur. Motor aynı kuralı `candidates()` ile karalama kopyalar üzerinde de koşturduğu için
iç durum tutmak (`RefCell`) adayları bozardı; bu yüzden "kırıldı", "mühürlendi" gibi
kavramlar açık hikâyelerin içeriğinden türetilir.

Tanımlar:

- **Etkin yön:** hikâyenin yönü; nötr bir hikâyede (sıkışma *ön bölümü*) ona katılan ilk
  yönlü, `exit` olmayan olgunun yönü.
- **Zayıf / güçlü bölüm:** akış ve aşırılık *zayıftır* (hareketi renklendirir, tek başına
  hareket başlatmaz ya da bitirmez); kırılım, devam, dönüş, sıkışma *güçlüdür*.
- **Mühürlü yay:** motor açık tutsa da bölüm listesi kapanmış yay: etkin yönüne **ters bir
  dönüş** olgusu ya da bir **devam durumunun çıkışı** (bindiği trend bitti) içerir. Mühürlü
  yaya, tuttuğu durumun çıkışı dışında hiçbir şey katılmaz.
- **Güncel güçlü yay:** mühürsüz, etkin yönü belli, en az bir güçlü bölümü olan açık
  hikâyelerden `last_bar`'ı en yeni olan (eşitlikte sonra doğan); `olgu.bar − last_bar ≤ 5`
  ise. **Zayıf yay:** aynısı, yalnız zayıf bölümlü hikâyeler arasında.

Karar (varsayılan parametreler):

1. **`exit` olgusu:** aynı dedektörün `enter`'ını tutan en yeni açık hikâyeye katılır
   (motor: `weakened / state_ended`); yoksa yok sayılır (tek başına bir şey söylemez; motor da
   zaten düşürürdü).
2. **Nötr olgu** (sıkışma girişi): güncel yay varsa ona katılır (hareket içinde bir
   duraklama); yoksa 5 bar içindeki mühürsüz ön bölüme; yoksa yeni ön bölüm `arc.neutral`.
3. **Yönlü olgu, güncel güçlü yay varken:**
   - aynı yönde → katılır (türü ne olursa olsun);
   - ters **dönüş** → yaya katılır (`weakened / opposite_fact`) ve onu mühürler: dönüş yayın
     son bölümüdür;
   - ters **zayıf** olgu → kendi *karşı* hikâyesine (`side.<tür>.<yön>`; aynı tür ve yönde 5
     bar içindeki karşı hikâyeye katılır); yay güncel kalır (tek bir ters bar hareketi
     bitirmez, olgu da kendi işaretini korur);
   - ters **güçlü** olgu (kırılım, devam girişi) → yayı **kırar**: yeni yay `arc.<yön>`; en
     yeni o olduğu için güncel odur, eski yay bir daha devam ettirilmez.
4. **Yönlü olgu, güncel güçlü yay yokken:** güçlü bir olgu son etkinliği 10 bar içindeki
   mühürsüz bir sıkışma ön bölümünü **çözer** (ona katılır; yayın yönü artık budur); yoksa
   aynı yöndeki zayıf yay onu alır; yoksa yeni yay `arc.<yön>`.

Tema anahtarları: `arc.up`, `arc.down`, `arc.neutral` (ön bölüm), `side.<tür>.<yön>`.

### Tür neden ve nasıl temsil ediliyor

Motor `story.kind`'ı (ve yönü) doğumda sabitler. Bu yüzden yayın türü **açılış bölümünün**
türüdür. Motor bir bardaki olguları `(dedektör, kod)` sırasıyla işlediği için
(`breakout.range` < `continuation.*` < `extreme.*` < `flow.*` < `reversal.*` < `squeeze.*`)
kırılım barında yay hemen hep kırılımla açılır; ki yayın "anlamı" (bir sınır aşıldı ve
hareket sürdü) en iyi bunu taşır. Sonraki bölümler kaybolmaz ama **işaretin şeklinde ve
cümlenin ana türünde görünmez**: olgular hikâyenin `facts` listesinde, her biri bir geçişle
`history`'de ve yeni ailesiyle `families`'te (teyit işareti sayısı) durur. Hacim ve kurumsal
akış bu yüzden yayın teyit işaretlerini artırır (`confirmed / new_family`); aynı aileden
gelen devam ve aşırılık bölümleri `strengthened` olur. Kayıp (ölçüdeki "tür/yön farklı")
öncülün bedelidir ve aşağıda açıkça sayılmıştır. Daha iyisi kuralın değil şemanın işidir
(bkz. Öneriler).

Yön için kural kayıpsızdır: yay yalnız kendi yönündeki olguları alır; ters olgular ya karşı
hikâyeye gider ya yayı kırar. Tek istisnalar mühürleyen dönüş (bilerek: yayın zayıfladığını
göstermenin tek yolu bu) ve sıkışma ön bölümüdür (aşağıda).

### Nötr anlamlar (sıkışma)

Arayüz bir olgunun geçmişteki bir hikâyeye değil yalnız açık bir hikâyeye katılmasına izin
verir; "sıkışmayı sonraki harekete bağlamanın" tek yolu sıkışmanın bir *ön bölüm* hikâyesi
açması ve onu çözen hareketin ona katılmasıdır. Yalnız güçlü bir olgu ön bölümü çözer (tek
bir hacim sıçraması sıkışmanın yönünü belirlemez). Bedeli: motor ön bölümün yönünü nötr
tutar; yay işareti nötr görünür, sonuç kuralı onu nötr hikâye gibi yargılar (karnede
"sıkışmadan sonra bir hareket geldi mi?" sorusu, ki sıkışmanın anlamı tam da budur) ve motor
ters olgularda zayıflatmaz (kural onun yerine yayı kırar). Hareket içindeki bir sıkışma ise
yayın duraklamasıdır ve ona katılır.

## Seçimler ve ablasyon

`harness-sim` kütüphanesiyle ayrı bir ölçüm betiği (depoya girmedi) parametreleri
karşılaştırdı. Etiketli küme: 3 tohum toplamı (çift oranları havuzlanmış, azalma ortalama);
rejimli pazar: yalnız `synthetic-s1`. "Tür/yön farklı" = hikâyede türü ya da yönü
hikâyeninkinden farklı olgu (parantez: türü farklı olan).

| Çeşit | Azalma (etiketli) | Yanlış birleştirme | Arka planla karışık | Bölünme | Tür/yön farklı (etiketli) | Düşen aile | Azalma (rejimli s1) | Tür/yön farklı (rejimli s1) | Düşen aile (rejimli s1) |
|---|---|---|---|---|---|---|---|---|---|
| **varsayılan** (`gap` 5, karşı hikâye, ön bölüm, dönüş mühürler, aşırılık katılır) | **0,570** | **0** | 210 | **0,829** | 4.425 (4.420) | **0** | **0,569** | 2.554 (2.551) | **0** |
| `gap` 2 | 0,547 | 0 | 115 | 0,831 | 4.086 | 0 | 0,530 | 2.350 | 0 |
| `gap` 3 | 0,560 | 0 | 141 | 0,830 | 4.260 | 0 | 0,549 | 2.449 | 0 |
| `gap` 8 | 0,575 | 0 | 247 | 0,829 | 4.496 | 0 | 0,579 | 2.581 | 0 |
| `gap` 12 / 20 | 0,576 | 0 | 252 | 0,829 | 4.507 | 0 | 0,580 | 2.586 | 0 |
| ön bölüm yok | 0,567 | 0 | 209 | 0,830 | 4.313 | 0 | 0,568 | 2.543 | 0 |
| ters zayıf olgu yaya katılır (`weakened`) | 0,573 | 0 | 210 | 0,827 | 4.475 | 44 | 0,601 | 2.899 (2.854) | 198 |
| dönüş mühürlemez (kırar) | 0,558 | 0 | 204 | 0,830 | 4.159 | 0 | 0,559 | 2.475 | 0 |
| aşırılık yaya katılmaz (yan hikâye) | 0,516 | 0 | 189 | 0,880 | 2.926 | 0 | 0,530 | 2.032 | 0 |
| katı (ön bölüm yok, dönüş kırar) | 0,554 | 0 | 203 | 0,831 | 4.056 | 0 | 0,558 | 2.466 | 0 |

Neden bu seçimler:

- **`gap` = 5 bar.** 5'in üstü neredeyse hiç kazandırmıyor (+0,005 azalma) ama arka planla
  karışan hikâyeyi %20 artırıyor; 2–3 azalmayı belirgin düşürüyor. Günlükte bir işlem
  haftası. Öncül "art arda" der: bir haftalık sessizlikten sonra gelen olgu yeni bir yaydır.
- **Ters zayıf olgu → karşı hikâye** (katılmak yerine). Katılmak rejimli pazarda +0,03
  azalma getiriyor, ama karşılığında olgunun yönü işarette kayboluyor (+345 tür/yön farkı; yalnız yönü farklı
  olan 3 → 45) ve ailesi düşüyor (198): `weakened` geçişi aile eklemez. Karşı hikâyede olgu kendi yönüyle
  görünür, yay da bozulmaz. Bu, "yay tek yönlüdür" öncülünün dürüst hâli.
- **Ters dönüş mühürler.** Dönüş olgusu "bu hareket dönüyor olabilir" der; yayın son bölümü
  olarak `weakened` geçişiyle yayda görünür ve yay kapanır. Kırmaya göre +0,012 azalma, ve
  okura yayın zayıfladığını gösteren tek kanal budur.
- **Aşırılık katılır.** Yukarı yayda RSI'ın 70'i aşması aynı hareketin uzamasıdır. Katılmak
  bölünmeyi 5 puan, azalmayı 5 puan iyileştiriyor; bedeli ~1.500 tür farkı (etiketli, 3
  tohum). Öncülün özü bu olduğu için tutuldu; karar ölçüye bağlı değil anlama bağlı, bedeli
  açık.
- **Ön bölüm** az tetikleniyor (başvuru sonuç kuralında sıkışma hikâyesi çoğunlukla
  kırılımdan önce süresi dolarak kapanıyor), etkisi küçük (+0,003). Öncülün "nötr anlam
  sonraki harekete bağlanır" sorusuna arayüzün izin verdiği tek cevap olduğu için tutuldu.

## Son ölçüler (`harness-sim m4 run --alt arc`)

Başvuru sonuç (`atr_barrier`) ve sıralayıcı (`families`) ile. Başvuru ve control değerleri
`../raw/{reference,control}/summary-*.md`'den.

**Etiketli bölüm kümesi** (s1 / s2 / s3)

| Ölçü | control `singleton` | başvuru `kind_direction` | **`arc`** |
|---|---|---|---|
| Azalma | 0 / 0 / 0 | 0,410 / 0,417 / 0,420 | **0,562 / 0,568 / 0,581** |
| Yanlış birleştirme oranı | — | 0,000 | **0,000** (karışık hikâye 0) |
| Arka planla karışık hikâye | — | 84 / 70 / 68 | 70 / 66 / 74 |
| Bölünme oranı | 1,000 | 0,941 / 0,944 / 0,937 | **0,834 / 0,841 / 0,811** |
| Yok sayılan | 0 | 990 / 959 / 972 | 1.003 / 987 / 986 |
| Motorun düşürdüğü | 0 | 0 | 0 |
| Tür/yön farklı (tür) | 0 | 0 | 1.428 (1.427) / 1.488 (1.485) / 1.509 (1.508) |
| Düşen aile | 0 | 0 | 0 |

Şablon başına bölünme (3 tohum toplamı):

| Şablon | başvuru | `arc` |
|---|---|---|
| breakout_volume_continuation | 0,909 | 0,720 |
| squeeze_breakout | 0,922 | 0,860 |
| pair_same | 0,878 | 0,537 |
| pair_opposite | 0,886 | 0,592 |
| extreme_enter_exit | 0,995 | 0,952 |
| divergence_breakdown | 0,978 | 0,958 |

**Rejimli sentetik pazar** (s1 / s2 / s3): azalma 0,569 / 0,569 / 0,569 (başvuru 0,378 /
0,379 / 0,386); yok sayılan 799 / 810 / 775 (başvuru 806 / 798 / 760); tür/yön farklı 2.554
/ 2.515 / 2.501 (olguların %30'u; neredeyse hepsi tür); düşen aile 0.

**Gerçek veri** (6 sembol, yalnız toplu): azalma 0,555 (günlük) / 0,588 (saatlik) (başvuru
0,425 / 0,411); yok sayılan 750 / 1.376 (başvuru 740 / 1.345); tür/yön farklı 1.077 / 2.464
(olguların %22 / %28'i; yön farkı 4 / 2); düşen aile 0.

### Bölünmenin kaynağı

Ayrı bir tanı betiği her etiketli olgunun kararında, aynı bölümden olgu tutan açık hikâye
olup olmadığını saydı (3 tohum, varsayılan): `New` kararlarının **4.453**'ünde o bölümün
hiçbir hikâyesi artık açık değildi (başvuru sonuç kuralı kapatmıştı), yalnız **196**'sında
kural açık bir hikâye varken yeni hikâye başlattı (çoğu sıkışma şablonunda, kırılımdan önceki sıkışma
  barlarında ters ya da yeni yönde açılan devam girişleri; ayrıca dönüş şablonunda ters akış
  olgularının karşı hikâyesi).
Yani kalan 0,83'lük bölünmenin ~%96'sı tema kuralının değil sonuç kuralının sonucudur: başvuru
kuralı yukarı kırılımı 1 ATR'lik ilk devam barında `realized` diye kapatıyor, sonraki bölüm
kapanmış hikâyeye katılamıyor (§9.3) ve yeni yay doğuyor. Yay öncülünün asıl kazancı, M5'te
seçilecek sonuç kuralı kırılım hikâyesini bu kadar erken kapatmıyorsa çok daha büyük olur;
son bütünleştirmede yeniden ölçülmeli.

## Zayıflıklar (dürüstçe)

1. **Tür kaybı öncülün kendisidir.** Etiketli kümede olguların ~%22'si, rejimli pazarda
   ~%30'u, gerçek veride %22–28'i türü yayınkinden farklı olarak yayda durur; işaretin şekli
   ve cümlenin ana türü yalnız açılış bölümünü gösterir. Başvuru kuralı bunu sıfırda tutar.
   Azalmadaki her kazanç kabaca bir tür farkıyla ödenir (türler arası birleştirmenin tanımı).
2. **Yanlış birleştirme 0 bir kanıt değil.** Etiketli kümede bölümler arasında 35 sessiz bar
   (çiftlerde 14) var ve başvuru sonuç kuralı hikâyeleri hızla kapatıyor; 5 barlık boşluk
   kuralı bölümleri hiç birleştiremiyor. Gerçek veride ara vermeden süren uzun bir trendde,
   arada ters güçlü olgu yoksa iki ayrı hareket tek yaya düşebilir; bu ölçülemedi.
   Arka planla karışık hikâye sayısı başvurununkiyle aynı düzeyde (~70/tohum).
3. **Ön bölümden doğan yay motorda nötr kalır:** işaret nötr, sonuç kuralı nötr hikâye gibi
   yargılar (herhangi bir 1 ATR hareket "gerçekleşti"), motor ters olguda zayıflatmaz. Sık
   değil ama bu yaylarda yön bilgisi işarette yoktur.
4. **Durumsuzluk bedeli:** "kırıldı" bilgisi açık hikâyelerden türetilir. Yayı kıran ters
   hikâye sonuç kuralıyla kapanırsa ve 5 bar içinde eski yönde bir olgu gelirse eski yay
   yeniden güncel olur (devam eder). Boşluk kuralı bunu 5 barla sınırlar.
5. **Ters zayıf olgular yayı zayıflatmaz.** Karşı hikâye olgunun bilgisini korur ama yayın
   kendi işaretinde karşı baskı görünmez; yaşam döngüsündeki "ters → zayıfladı" yalnız
   mühürleyen dönüşte kullanılır. Katılma çeşidi bunu gösterir ama yön ve aile kaybettirir
   (ablasyon).
6. **Karne anahtarı kayar:** yayın anahtarı açılış olgusunun dedektörüdür
   (`breakout/breakout.range@1`); bu anahtarın karnesi artık "tek kırılım"ı değil "kırılımla
   açılan yay"ı ölçer.
7. **Boşluk bar sayısıdır, zaman değil:** 5 bar günlükte bir hafta, saatlikte beş saat.
   Seri başına ayrı boşluk denenmedi.
8. **Aşırılık "yön" olarak okunuyor:** yukarı yaydaki aşırı alım bir yorgunluk uyarısı da
   olabilir; kural onu hareketin uzaması sayar.
9. **Yok sayılanlar başvuruyla aynı düzeyde** (~990/tohum): hikâyesi kapanmış durumların
   çıkışları. Yaya katılsalar motor düşürürdü; kayıp kuralın değil yaşam döngüsünün.

## Öneriler (kapsam dışı)

- Şema: hikâyeye yayın bölüm türlerini taşıyan bir alan (ör. `chapters: Vec<MeaningKind>`
  ya da tür başına sayı) ve işarette teyit işaretlerinin yanında "bölüm" işaretleri; ya da
  ön bölüm çözüldüğünde hikâyenin yönünü bir kez belirlemeye izin veren bir geçiş
  (`resolved`). İkisi de tür/yön kaybının çoğunu öncülü değiştirmeden kapatır.
- M4 bölünme ölçüsü sonuç kuralına çok bağlı; seçilen M5 kuralıyla yeniden koşulmalı ya da
  sonuç kuralı kapatmayan (yalnız ufuk) bir koşu da raporlanmalı.
- `breakout.range`'in trendde her bar tetiklemesi (T3 önerisi) yay içinde "güçlendi"
  gürültüsü üretir; v2 kenar koşulu yayın geçmişini sadeleştirir.
