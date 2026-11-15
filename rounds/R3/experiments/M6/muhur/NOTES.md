# M6 aday güven yolu — **mühür** (`m6_muhur`, alt adı `muhur`)

R3/M6b, yeni yöntem (K18). Bu aday ötekinden bağımsız yazıldı; `ALTERNATIVES.md` ve diğer
aday klasörü açılmadı.

## 1. Fikir tek cümlede

Sızıntıyı gizlilikle değil **yapıyla** sınırla: ham `net` değeri geldiği anda ikili
büyüklük basamağına (işaret + `floor(log2 |net|)`) çevrilir ve ham değer bir daha hiçbir
kullanıcıya giden hesaba girmez; basamaklar da hissenin kendi yakın düzeyine göre
kırpılır. Böylece hedef barın değeri, saldırgan **her şeyi** bilse bile, iletilerde en çok
7 farklı biçimde iz bırakabilir (≤ log2 7 ≈ 2,81 bit). İletiye eklenen **makbuz**
sunucu anahtarıyla imzalı ve yalnız açıklamanın kendisinin fonksiyonudur (ek sızıntı
sıfır); yetkili denetçi aynı kuralı lisanslı veride baştan koşturup her açıklamayı (ve
her susuşu) yeniden türetir, ham değerlere bağlayıcı taahhütleri denetim günlüğünden
kontrol eder.

## 2. Tasarım

Parametreler herkese açıktır (Kerckhoffs). Varsayılan: `window = 5`, `min_obs = 5`,
`low = 2`, `loud = 2`, `epoch = 1` (`src/lib.rs`, `Params`).

1. **Ağ (kafes) ile alım.** Barın tek okunan değeri `net` (`buy`, `sell`,
   `brokers_net_top5` hiç okunmaz; okunsaydı `buy − sell = net` değeri geri verirdi).
   Hücre `Cell { level, up }`: `level = floor(log2 |net|)` IEEE üs bitlerinden **tam**
   okunur (kayan nokta yuvarlaması yok), `net = 0` en alt düzey, `up = net > 0`.
   `speak(cell, R, p)` imzası gereği ham değeri göremez; durum (pencere, başvuru) yalnız
   hücrelerden oluşur. Ham değerin tek başka kullanımı denetim taahhüdüdür (§4); o
   kullanıcı iletisine hiç girmez.
2. **Başvuru düzeyi `R`.** Son `window` gözlemin **saklanan** düzeylerinin alt medyanı;
   `epoch` barlık dönemlerde dondurulur (bar sırası herkese açık). `min_obs` gözlemden
   önce susulur (ısınma).
3. **Kırpılmış saklama.** Bar `t`'nin düzeyi `clamp(level, R − low, R + loud)` olarak
   saklanır. Sonuç: `|net| < 2^(R−low)` olan bütün değerler (iki işaret, sıfır dahil)
   tek bir şeydir; `|net| ≥ 2^(R+loud)` olan bütün değerler (işaret başına) tek bir
   şeydir.
4. **Açıklama.** `level ≥ R + loud` ise (yani `|net|`, hissenin yakın olağan
   medyan büyüklüğünün kabaca 2–4 katı) tek ileti: yön + "olağan düzeyinin belirgin biçimde
   üzerinde". Başka hiçbir nitelik (şiddet, süreklilik, sıra) söylenmez. Gösterim barı =
   bar (gecikme yok).
5. **Dedektör olgusuna bakılmaz.** `flow.institutional`'ın eşiği (`2·avg5`) kafes
   üzerinde değildir; olguya göre konuşmak hem hedef barda hem sonraki 5 barda keyfî
   (dar) kesim noktaları açar — control `echo`'nun 8–10 bitlik en kötü durumu tam bu
   kanaldan gelir. Mühür kendi kuralını (sunucuda, lisanslı veride) uygular; ileti bu
   kuralın sonucunu söyler, dedektörün değil. Bedeli fayda (§6).

## 3. Yapısal sınır ve kanıt taslağı

**Önerme.** Isınmadan sonra (hedef barda `R_b` tanımlı), diğer bütün girdiler sabitken
hedef barın `net` değeri `v` üzerinde tam ileti dizisi `T(v)` (kodlar, metinler, ekler,
gösterim barı; hedef bar ve sonrası — aslında bütün koşu) yalnız şu sınıfa bağlıdır:

| Sınıf | Koşul | Sayı |
|---|---|---|
| sessiz-alçak | `|v| < 2^(R_b − low)` (işaret yok, `0` dahil) | 1 |
| sessiz-orta | `level(v) = k`, `R_b − low ≤ k < R_b + loud` (işaret yok) | `low + loud` |
| yüksek | `level(v) ≥ R_b + loud`, işaret başına | 2 |

Dolayısıyla `|{T(v) : v ∈ ℝ}| ≤ low + loud + 3 = 7` ve bu iletiler `v` hakkında, hangi
ön dağılım ve hangi yan bilgiyle olursa olsun, **en çok log2 7 ≈ 2,81 bit** taşır
(karşılıklı bilgi ≤ çıktı entropisi ≤ log2 |çıktı kümesi|).

**Kanıt taslağı.** (i) Bar `b`'den önceki durum `v`'den bağımsızdır (nedensellik:
gelecek okunmaz). (ii) `b`'deki adım `v`'yi yalnız `Cell::of(v)` olarak kullanır;
`R_b` önceki duruma bağlıdır. (iii) `b`'nin çıktısı `speak(cell, R_b)`'dir: sınıf
"yüksek" değilse boş, "yüksek" ise yalnız işarete bağlı; makbuz açıklamanın
fonksiyonudur. (iv) `b`'den sonra duruma geçen tek şey saklanan
`s_b = clamp(level, R_b − low, R_b + loud)`'dur; aynı sınıftaki her `v` aynı `s_b`'yi
verir (alçak sınıf `R_b − low`'a, yüksek sınıf `R_b + loud`'a kırpılır; orta sınıfta
`s_b = k`). (v) Sonraki her adım, durumun ve `b` dışındaki girdilerin deterministik
fonksiyonudur; tümevarımla `T(v)` yalnız sınıfa bağlıdır. ∎

**Harness ölçüsüyle karşılığı.** Kâhinin tuttuğu küme en az hedefin sınıfıdır:
yüksekse `[2^(R+loud), ∞)` (ızgara üst kenarına değer: doymuş), alçaksa
`(−2^(R−low), 2^(R−low))`, ortaysa `±[2^k, 2^(k+1))` (iki parça, toplam genişlik
`2^(k+1) ≥ 2^(R−1)`, yani hissenin medyan büyüklüğünün en az ~dörtte biri). Hiçbir durumda
tutulan küme bir ikilik basamaktan dar olamaz; pratikte sınıflar çoğunlukla birleşir
(çoğu orta değer `R`'yi değiştirmez).

**Isınma** (ilk `min_obs` gözlem; harness hedefleri ≥ bar 5 ve olgu da en erken 6.
gözlemde olur): düzey kırpılmadan saklanır; o barlar için güvence yalnız "işaret yok +
bir ikilik basamak" (genişlik ≥ `|v|`). Sınırsız sayıda sınıf olabilir; dürüstçe zayıf
nokta.

Testler: `leakage_bound_by_construction` (her hedefin güvence kümesinde 17–34 nokta →
bayt eşit döküm), `at_most_low_plus_loud_plus_three_transcripts` (±10⁻¹⁴…10²⁵ tarama ve sıfır,
sınıf sayısı ≤ 7), `lattice_is_exact`, `reads_only_net`.

## 4. Doğrulama: "sunucu doğruladı" neyi garanti ediyor

Her iletinin ekleri (hepsi rakamsız):

| Ek | Değer | Anlamı |
|---|---|---|
| `entitlement` | `flow/*` | ham kanıt hangi yetkiyi gerektirir (K34) |
| `evidence` | `flow.institutional/net` | neye dayanır |
| `rule`, `rule_version` | `muhur`, `one` | açık kural ve sürümü (herkes kuralın ne dediğini bilir) |
| `receipt` | ör. `hupor-…` (5 proquint sözcüğü, 80 bit) | `HMAC-SHA-256(sunucu anahtarı, "m6.muhur|enstrüman|bar|t|kodlar")` |
| `check` | `receipt by server key; replay by entitled auditor` | nasıl doğrulanır |

- **Makbuz** açıklamanın (zaten dökümde olan alanların) fonksiyonudur → ek sızıntı sıfır
  (bayt eşitlik ölçüsünde de görünmez). Anahtar sahibi (sunucu uç noktası ya da
  doğrulama anahtarı verilmiş denetçi) iletinin değiştirilmediğini, başka bara
  taşınmadığını, yönünün çevrilmediğini, metninin kuralın metni olduğunu
  `verify_receipts` ile denetler (testte: yanlış anahtar, yön çevirme, bar kaydırma,
  metin değişikliği yakalanıyor).
- **Yetkili tekrar (replay).** `flow/*` yetkisi olan denetçi lisanslı veriyi zaten
  görebilir; `replay(gözlemler)` kuralı baştan koşturur, sunulan iletilerle bayt eşitliği
  ve **susuşları** (atlanan ileti) denetler.
- **Ham değere taahhüt.** Sunucu her gözlemli bar için denetim günlüğüne
  `HMAC(denetim anahtarı, enstrüman|bar|t|net bitleri)` yazar (yalnız yetkiliye). Sunucu
  sonradan "başka veriyle hesapladım" diyemez; aynı hücredeki farklı ham değer kullanıcı
  dökümünü değiştirmez ama taahhüdü değiştirir (test). Taahhüt kullanıcı iletisine asla
  girmez — girseydi her değer için farklı olur ve kâhin değeri ızgara çözünürlüğünde
  bulurdu.
- **Neden zincir yok.** İlk sürümde makbuzlar zincirliydi (önceki makbuzu içeren, ekleme
  sıralı günlük). Bütün geçmişi gören kullanıcıya bilgi eklemez, ama her makbuzu bütün
  geçmişe bağlar; atlama zaten yetkili tekrarda yakalanıyor. Basitlik için kaldırıldı.

**Dürüst sınırlar.** HMAC simetriktir: makbuzu son kullanıcı kendi başına doğrulayamaz,
sunucuya ya da anahtarlı denetçiye sorar. Üretimde aynı bayt dizisine **deterministik
Ed25519 imzası** (RFC 8032, rastgelelik yok) konursa herkes açık anahtarla doğrular;
sızıntı sınırı değişmez (imza açıklamanın fonksiyonu). Depoda kripto bağımlılığı
olmadığı için SHA-256/HMAC crate içinde yazıldı ve FIPS 180-4 / RFC 4231 test
vektörleriyle sınandı. Anahtarlar deneyde sabit sahte değerlerdir (`DEMO_*`).

## 5. Saldırgan modeli

- **Kâhin (en güçlü):** algoritmayı, parametreleri, hedef dışındaki bütün ham değerleri
  ve **anahtarları bile** bilir. Sınır (§3) anahtarın gizliliğine dayanmaz.
- **Gerçekçi:** yalnız iletileri ve açık barları görür, ön dağılımdan aday çeker.
- **Sahtecilik saldırganı** (iletiyi değiştiren ara katman): makbuz anahtarını bilmez;
  HMAC-SHA-256'nın sahtecilik direnci varsayılır.
- **Kötü niyetli sunucu:** makbuz tek başına sunucuyu bağlamaz (kendi anahtarı); onu
  bağlayan yetkili tekrar + denetim taahhüdüdür. Yetkisiz kullanıcı sunucuya güvenmek
  zorunda; mühürün kazandırdığı, bu güvenin **sınanabilir** olmasıdır.
- Sızıntı sınırı tek hedef bar içindir. Bir saldırgan aynı hissenin **birçok** barını
  birlikte bilinmeyen sayarsa her bar en çok 2,81 bit verir; toplam bar sayısıyla
  doğrusal büyür (ama her barın kendi değeri hakkında bir basamaktan ince bilgi asla
  çıkmaz).

## 6. Ölçümler (harness-sim, sentetik, tohum 1–3)

Ham: `rounds/R3/experiments/M6/raw/muhur/` (özet `summary-synthetic.md`). Süre: 3 tohum
~52 dk (makinede başka iş vardı; iletiler nadiren ayrıştığı için kâhin koşuları hep
sonuna kadar gider). Karşılaştırma: control `echo` (aynı düzenek, aynı hedefler).

| Ölçü (havuz, 8.406 hedef) | mühür | control `echo` |
|---|---|---|
| Kâhin genişlik/yayılım medyan · p10 · en kötü | **1,557 · 0,428** · 0,00073 | 1,171 · 0,197 · 0,00073 |
| Kâhin bit medyan · p90 · en çok (düzenek) | 0 · **1,22** · 10,41 | 0 · 2,34 · 10,41 |
| Bütün ızgara tutulan hedef (sıfır bilgi, bit yalnız ölçek yapıntısı) | 80 | 99 |
| Bit, bütün-ızgara dışı: p99 · en çok | **2,73 · 5,42** | 4,42 · 8,04 |
| Doymamış hedeflerde bit medyan · p90 · en çok | **0,81 · 2,35 · 5,42** (1.458 hedef) | 1,30 · 3,36 · 8,04 (2.990) |
| Olgulu hedeflerde bit p90 · en çok | **1,13 · 5,42** | 2,27 · 8,04 |
| Olgusuz (sıradan) hedeflerde bit p90 | 3,71 | 3,43 |
| Çok parçalı · doymuş · gerçek tutulmadı | 790 · 6.948 · 0 | 56 · 5.416 · 0 |
| Gerçekçi: sonsal/yayılım medyan · p10 · önsel medyan · kapsama | 0,851 · 0,281 · 0,549 · 0,853 | 1,083 · 0,296 · 0,549 · 0,889 |
| Gecikme | hepsi 0 | hepsi 0 |
| trust=server · metin/kod/ek rakam · TR/EN ifade | 1 · 0/0/0 · 1/1 | 1 · 0/0/0 · 1/1 |
| İleti sayısı · **fayda** | 9.503 · **0,826** (s1 0,820, s2 0,832, s3 0,826) | 8.136 · 1,000 |

Okuma:

- **En kötü bit 10,41 iki yolda da aynı hedefte ve bütün ızgara tutulmuş**: iletiler o
  barın değeri hakkında hiçbir şey söylemiyor; bit, ızgaranın `|net_b|` ile ölçeklenmesinin
  yapıntısı (küçük `|net_b|`, büyük yayılım). Anlamlı en kötü (bütün-ızgara dışı) 5,42'ye
  karşı 8,04; p99 2,73'e karşı 4,42.
- 5,42 bitlik hedef bir **orta hücre**: tutulan küme `±[2^k, 2^(k+1))` iki parçası; hücre
  hissenin rejimler arası yayılımına göre dar kaldığı için düzenek ölçüsü yüksek. Bilgi
  kuramsal sınır (≤ log2 7 = 2,81 bit, §3) sınıf sayısıyla ilgilidir, genişlik/yayılım
  oranıyla değil; iki ölçü farklı şeyleri sayar (§8).
- "Çok parçalı" sayısı yüksek (790): beklenen — orta sınıfta işaret kullanılmadığı için
  tutulan küme `+` ve `−` iki parçadır (saldırgan işareti öğrenemez).
- Olgusuz hedeflerde p90 control'den biraz kötü (3,71 / 3,43) ve gerçekçi saldırganın
  sonsalı control'den dar (0,85 / 1,08): mühür olgusuz barlarda da konuşur (iletilerin
  ~%29'u dedektör olgusu olmayan barlarda), bu da sıradan barlar hakkında "eşiğin altı /
  üstü" bilgisini çoğaltır. Kapsama 0,85 (control 0,89).
- Birim testleri (`cargo test --release`, 11 test): yapısal sınır, ≤ 7 döküm, kafes
  tamlığı, yalnız `net` okunur, rakamsızlık/ifade/K34 ekleri, belirlenimcilik, makbuz
  (doğru/yanlış anahtar, yön, bar, metin), yetkili tekrar ve taahhüt, SHA-256/HMAC test
  vektörleri.

Parametre çalışması (`tests/tune.rs`, ayrı tohumlar, fayda · ileti/bar): `loud = 1` fayda
0,96–0,99 ama barların ~%42–46'sında ileti ("olağanın üstü" anlamını yitirir);
`loud = 2`: pencere 5 → 0,83 · 0,21, pencere 10 → 0,82 · 0,24, pencere 20 → 0,75 · 0,21;
dönem 5 ya da 20 faydayı 0,03–0,07 düşürür (donmuş dönem kararlılık verir, ama ölçülen
yarar yok; varsayılan `epoch = 1`). `low = 1` sınıf sayısını 6'ya indirir (2,58 bit)
ama faydayı 0,76'ya düşürür.

## 7. Örnek iletiler

Sentetik s1, `SYN01` (ham kayıttan, aynen):

- **TR:** "Sunucu doğruladı: bu hissede kurumsal net alış, hissenin yakın dönemdeki olağan
  düzeyinin belirgin biçimde üzerinde. Ham akış rakamları akış veri yetkisi gerektirir."
- **EN:** "Server-verified: institutional net buying in this stock is well above its usual
  recent level. Raw flow figures require the flow data entitlement."
- Kodlar `flow_institutional`, `net_buy`, `well_above_usual`; ekler: `entitlement=flow/*`,
  `evidence=flow.institutional/net`, `rule=muhur`, `rule_version=one`,
  `receipt=fibor-jisul-mosus-lopan-vadih`, `check=receipt by server key; replay by
  entitled auditor`.
- Satış biçimi: "… kurumsal net satış, …" / "… institutional net selling …", makbuz ör.
  `kazup-sapip-mihuh-hipop-kimun`.

Söylenebilecek bütün cümleler bu iki cümledir (yön başına bir); başka hiçbir şey
söylenemez — bu da sınırın parçasıdır.

## 8. Zayıflıklar (dürüstçe)

1. **Fayda 0,83, control'ün 1,00'ı değil.** Dedektörün keyfî eşiği (`2·avg5`) yerine kafes
   eşiği kullanıldığı için olguların ~%17'si iletisiz kalır; iletilerin ~%29'u olgusuz
   barlardadır. İleti "dedektör olgusu" değil "mühür kuralının açıklaması"dır; ürün
   dilinde bu ayrım (aynı kapsül mü, ayrı mı) karar ister. Olguya bağlamak sınırı bozar
   (§2.5).
2. **Sınır bilgi kuramsaldır, genişlik oranı değil.** ≤ 2,81 bit "sınıf sayısı" sınırıdır;
   düzeneğin `log2(yayılım/genişlik)` ölçüsü bir orta hücrede 5,42'ye çıkabilir, çünkü
   bir ikilik basamak hissenin rejimler arası yayılımına göre dar olabilir. Güvence
   "değer bir ikilik basamaktan (ve işaretten) ince asla bilinmez"dir; "yayılımın şu
   kadarından ince bilinmez" değildir.
3. **Çok barlı birleşik saldırı.** Sınır tek hedef içindir; bir saldırgan çok sayıda barı
   birlikte çıkarmaya çalışırsa her bar ≤ 2,81 bit verir, toplam doğrusal büyür (her bar
   yine en fazla kendi basamağına iner).
4. **Isınma** (ilk 5 gözlem): kırpma yok; güvence yalnız "bir basamak, işaretsiz".
5. **Gerçekçi saldırgan için control'den kötü** (sonsal 0,85 / 1,08; olgusuz bar p90 3,71
   / 3,43): daha çok konuşmanın bedeli. `loud = 3` bunu azaltır ama faydayı düşürür
   (ölçülmedi).
6. **Makbuz simetrik (HMAC).** Kullanıcı tek başına doğrulayamaz; Ed25519'a geçiş
   önerisi (§4). Anahtarlar deneyde sahte sabit. Atlama (ileti silme) yalnız yetkili
   tekrarda yakalanır.
7. **Yalnız sentetik akış** (yerelde gerçek lisanslı akış yok); kafes sabitleri (taban 2,
   `low`/`loud`) sentetik dağılıma göre seçildi.
8. **Süre:** iletiler nadiren ayrıştığı için düzenekte en pahalı yol türü (~50 dk / 3 tohum).
