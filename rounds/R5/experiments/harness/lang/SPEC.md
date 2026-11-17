# M17 — Kapsül dili ölçüm düzeneği (şartname)

Yazan: R5/T2 `t2-lang` alt ajanı, 2026-11-17/18, **hiçbir dil alternatifi yazılmadan önce**
(GOAL "Ölçümle karar verilecek konular", K39; kabul ölçütü 2). Bu dosyanın ve `tool/` kodunun
commit'i her M17 alternatif dalının ilk commit'inden öncedir. **Seçim kuralı (§5) sonuç
görülmeden burada yazılıdır**; değişirse değişiklik tarihli bir ek (§8) olarak yazılır, bütün
adaylara aynı anda uygulanır ve nedeni RESULTS'ta açıkça söylenir (R3 sapma 1, R4 E6 dersi).
Girdi: `rounds/R5/GOAL.md` (M17, tek bakış okuması, tasarım sınırları, kabul 3), DECISIONS K42,
K49, K53, A30, A33; `rounds/R5/RESEARCH.md` §10, §14, §15 (F1–F3), §17 (U1–U12); R4 düzeneği
(`rounds/R4/experiments/harness/SPEC.md`) ve zayıflıkları (`rounds/R4/experiments/M8/RESULTS.md`).

## 0. İlkeler

1. **Arayüze bağlı, uygulamaya değil.** Düzenek bir dilin kodunu okumaz. Bir dil, `tool/src/lang.rs`
   içindeki `Language` arayüzünü uygular: bir **durum** (`Situation`: motorun bir kapsülü bir "şimdi"de
   bir okur için gördüğü her şey — ürünün `SentenceInput`'u ve başlık yapı taşlarının girdileri) ve bir
   dil (TR/EN) verilir, dil dört **yüzey** döner: `headline` (başlık), `detail` (ayrıntı cümlesi),
   `ledger` (karne cümlesi), `glossary` (o ekranın sözlük girdileri: terim + açıklama). Taramalar,
   tablo, okuyucu sayfaları ve puanlama yalnız bu metinleri ölçer.
2. **Alternatif nasıl eklenir.** Alternatif dalı `crates/qp-capsule/` şablonlarını (ve gerekirse yüz
   başlık şablonunu) değiştirir; düzeneğe **tek bir adaptör dosyası** (`tool/src/adapters/<ad>.rs`,
   yalnız kendi ürün işlevlerini çağırır) ve `lang::registry()`'ye bir satır ekler. Adaptör dışında
   düzenek kodu değişmez. Adaptör ürün yolunu aynen çağırmalıdır (ürün başka, adaptör başka metin
   üretemez); denetçi bunu adaptörü okuyarak doğrular.
3. **Belirleyicilik.** Durumlar, döküm, taramalar, sayfa sırası, körleme kodları ve önyükleme tohumlu
   ve deterministiktir; aynı komut aynı baytları verir. Dil modeli yalnız okuyucu ve eleştirmendir;
   ürün yolunda yoktur (K3).
4. **Sınır.** Ajan insan değildir; bu ölçümler **ön elemedir**. İlk iki aday gösterimde seçilebilir
   kalır; son söz Arkin'in telefonu ve insan testi v3'tür. Her RESULTS bu cümleyi taşır.
5. Başvuru **`r4`** (bugünkü ürün dili) her ölçümde yarışır, sıralanır, seçilmez. **`bozuk`**
   (§6) her okuyucu koşusuna gizlice eklenir.

## 1. Döküm

### 1.1 Durumlar (`tool/src/situ.rs`)

- Motor: `EngineConfig::r3()`, yedi varsayılan dedektör; ürün kodu değişmez.
- Dünyalar (yalnız sentetik; gerçek veri dökümde yok): `syn-d` = `synthetic_market(30, 500, 1)` +
  sentetik lisanslı akış, günlük; `syn-h` = `synthetic_market_hourly(10, 160, 2)` + akış, 60 dakika.
  Takvim: sentetik pazarın donmuş takvimi (`qp_source::synthetic_calendar`, oturumun yaptığı gibi).
- "Şimdi" adayları: her hikâye geçişinin barı; açık hikâyede ayrıca bir sonraki geçişinden önce durum
  koşulunun ilk tutmadığı bar (geçerlilik `no_longer`; en çok 60 bar ileri).
- Okur: lisanslı olguda hem yetkili hem **yetkisiz** (K33, rakamsız) durum.
- **Hücre** `(dünya, dedektör, yön, aşama, geçerlilik, okur, karne kovası)`; karne kovası
  `empty` (n = 0), `thin` (n < 20, ürünün `LEDGER_MIN_N`'i), `counts`. Her hücreden olay sırasındaki ilk
  durum alınır. Kimlikler `s001…` üretim sırasıyla.
- **Kurgu durum:** sentetik oynatma geri çekme üretmez (PROTOCOL §20.7). Her (dedektör, yön) için ilk
  kapanmış günlük durumun aşaması `retracted` yapılır (`origin = constructed`; tabloda "kurgu").
- `candidate` (aday) aşaması dökümde yok: yüzeye cümleyle girmez (yalnız izleyene, açık barda).
- Bugünkü ürünle: **531 durum** (iki dünya); aşama × dedektör kapsamı `results/r4/TABLE.md` sonunda.

### 1.2 Cevap anahtarı (`tool/src/key.rs`) — yalnız motor verisinden

Olay kategorisi (tür + yön), yön, aşama, geçerlilik ve kapanış biçimi, doğuştan ve son geçişten beri
işlem günü, kabul edilebilir gün/bar uzaklıkları (her geçiş, öncü olgu, son olgu, şimdi), bu olayların
yerel tarihleri, karne `(n, tutan)`, sonuç ufku (bar), okurun rakam görüp göremediği ve **eşik**:

| dedektör | eşik (`id`) | adı sayılan kalıp TR / EN | değer (en az biri) | parametre (en az biri) |
|---|---|---|---|---|
| `breakout.range` | son 20 periyodun en yükseği/düşüğü (`range_extreme`) | `en yükse, en düşü, zirve, dip, direnç, destek` / `high, low, peak, trough, resistance, support` | `level` | `WINDOW` = 20 |
| `extreme.rsi` | RSI 70 / 30 (`rsi_level`) | `rsi, göreceli güç` / `rsi, relative strength` | `level` | — |
| `continuation.ma_stack` | 20 ve 50 periyotluk ortalamalar (`moving_average`) | `ortalama` / `average` | `sma20`, `sma50` | 20, 50 |
| `reversal.rsi_divergence` | önceki dip/tepe ve oradaki RSI (`divergence_swing`) | RSI **ve** dip/tepe sözcüğü | `p1_price`, `p2_price`, `p1_rsi`, `p2_rsi` | — |
| `squeeze.bandwidth` | 20 periyotluk bandın son 120'nin en darı; bandın sınırları (`band`) | `bant` / `band` | `upper`, `lower`, `middle` | 20, 120 |
| `flow.volume_surge` | 20 periyot ortalama hacmin 2,5 katı (`volume_multiple`) | `hacim` / `volume` | 2,5 ya da `ratio` | 20 |
| `flow.institutional` | kurumsal net akışın 5 periyot ortalamasının 2 katı (`institutional_multiple`) | `kurum` / `institution` | 2, `net`, `avg5` (mutlak) | 5 |

Parametre sayıları dedektörün kendi sabitlerinden okunur (`RangeBreakout::WINDOW` vb.); değerler
öncü olgunun kanıtından. Sayı eşleşmesi: `|x − v| ≤ max(0,006·|v|, 0,011)`.

### 1.3 Çıktılar

`langh dump --lang <ad>` → `out/<ad>/dump.json` (her durum: hücre, anahtar, TR ve EN yüzeyleri; izlenmez),
`TABLE.md` (dedektör × aşama × geçerlilik × dil tablosu: günlük yetkili, saatlik yetkili, yetkisiz;
her hücreden ilk durum; sonunda kapsam matrisi — `docs/LANGUAGE.md`'ye girecek tablo budur),
`SCAN.md`, `findings.json`, `scan.json`. Başvurunun tablosu ve taraması `results/r4/` altında.

## 2. Deterministik taramalar (hepsi 0 olmalı) — `tool/src/scan.rs`

Kalıplar küçük harfli düzenli ifadedir; metin küçük harfe çevrilir (TR'de I→ı, İ→i kuralıyla; bu yüzden
TR kalıplarında RSI `rs[iı]` yazılır), Unicode sözcük sınırı (`\b`) ile eşlenir. **Türkçe ek kuralı:**
bir kalıp yalnız listelediği ekleri kabul eder; kısa kök başka bir sözcüğü yakalamaz ("bar" ≠ "barış",
"olgu" ≠ "olgun", "bulgu" ≠ "bulgur", "al" ≠ "alım", "sat" ≠ "satış"). Enstrüman simgesi (ör. `SYN03`)
sayı ve rakam taramalarından önce metinden çıkarılır. Listelerin tam metni `scan.rs`'deki sabitlerdir;
buradaki özet onları adlandırır. Liste genişletmesi §8'e tarihli ek olarak yazılır ve bütün adaylara
uygulanır.

| # | tarama | kural | gerekçe |
|---|---|---|---|
| 2.0 | `empty` | başlık, ayrıntı, karne boş değil | — |
| 2.1 | `vague` | **sert liste** her zaman yasak: TR "önemli bir eşik/seviye", "kritik/kilit seviye, eşik, nokta", "güçlü (bir) sinyal", "dikkat çekici", "kayda değer", "anlamlı/belirgin bir hareket", "teknik görünüm", "muhtemelen", "büyük ihtimalle", "olasılıkla", "%N daha", "ortalamadan N daha iyi"; EN "key level", "important/critical/crucial/significant/major level", "strong signal", "notable", "noteworthy", "meaningful move", "technical picture", "likely", "probably", "possibly", "N% more/better". **Yumuşak liste** (TR olağandışı, büyük, önemli, kritik, güçlü, sert; EN unusual(ly), heavy, big, significant(ly), strong(ly), sharp(ly), key) yalnız **içinde sayı olmayan cümlede** yasak (yetkili okur; yetkisiz okur sayı göremez, ona uygulanmaz) | K49 ("önemli eşik nedir?"); sözel olasılık/önem sözcükleri çok geniş yorumlanır, yerine sayı + pencere (RESEARCH §10.1, F2); göreli ifade etkiyi büyütür (§10.2). Yumuşak sözcük sayıyla birlikte somuttur ("ortalamanın 2,5 katı, olağandışı yüksek") |
| 2.2 | `advice` | TR al, alın(ız), alınmalı, almalı, alabilirsin(iz), alınabilir, sat, satın, satınız, satılmalı, satmalı, satabilirsin, satılabilir, fırsat*, tavsiye*, öneririz/öneriyoruz/önerimiz, hedef fiyat, kaçırma*, pozisyon aç*, zarar kes*, stop loss, kâr al*; EN buy, sell, opportunit(y/ies), should, recommend*, advice, advis*, target price, don't/do not miss, stop-loss, take profit, go long/short | `docs/CAPSULE.md` §5, §8: gözlem, tavsiye değil. "alım/satış/buying/selling" betimlemedir, yakalanmaz |
| 2.3 | `repeat` | aynı ekranda (başlık + ayrıntı + karne + sözlük) normalleştirilmiş (küçük harf, yalnız harf/rakam) aynı cümle iki kez, en az iki sözcük | K49 "aynı cümle ekranda iki kez yer almaz" ("Hâlâ geçerli." başlıkta ve ayrıntıda tekrar sayılır) |
| 2.4a | `internal` | **iç terimler her yüzeyde, sözlük dahil, her zaman yasak:** TR bar (ekli biçimleri), teyit*, aile (ekli), dedektör*, olgu (ekli), bulgu (ekli), kademe*, sinyal*, aday (ekli), kesinleş*, skor*; EN bar(s), confirm(ed/s/ation), famil(y/ies), detector(s), fact(s), finding(s), tier(s), signal(s), candidate(s), score(s) | ürünün iç kavram adları (`docs/CONCEPTS.md`), kullanıcıya bir şey söylemez (K43, A33). "sinyal/signal" al-sat sinyali çağrıştırır (tavsiyeye yakın); "teyit/confirmed" ürün aşamasının adıdır ve "kim teyit etti?" sorusu doğurur. Sözlükte açıklansa bile yasak: iç terimin yerine gündelik söz konur |
| 2.4b | `unglossed` | **teknik terimler** yalnız o ekranın sözlüğünde açıklanıyorsa serbest (bir girdinin `term`'i aynı kalıba uyuyorsa): TR rsi, hareketli ortalama, bollinger, bant, hacim, kırılım, aşırı alım/satım, sıkışma, uyumsuzluk, kurumsal, direnç, destek, oynaklık, momentum, karne, atr; EN rsi, moving average, bollinger, band, volume, breakout, overbought, oversold, squeeze, divergence, institutional, resistance, support, volatility, momentum, track record, atr | K49 "bilinmesi gereken her terim dokununca açıklanır; açıklama deterministik sözlükten". Düz "ortalama/average" gündelik sözcüktür, listede değil |
| 2.5 | `length` | başlık ≤ 80 karakter; ayrıntı ≤ 200 karakter ve ≤ 3 cümle; karne ≤ 70; sözlük açıklaması ≤ 160 | başlık sınırı R4 yüz kuralı (SPEC §2(b)); 390 pt telefonda 16 px'te ≈ 40 karakter/satır → ayrıntı ≈ 5 satır, karne ≈ 2 satır; sözlük bir dokunuşta okunur |
| 2.6 | `threshold` | ekran (başlık + ayrıntı) eşiğin **adını** (§1.2 kalıbı) ve yetkili okurda **değerini** (bir değer sayısı) ve **dedektör parametresini** (parametre listesi doluysa birini) içerir; yetkisiz okurda yalnız ad aranır | K49, kabul 3 "her dedektör kendi eşiğini adıyla ve değeriyle söylüyor" |
| 2.7 | `ledger_expect` | karne ya da ayrıntıda "beklendiği gibi / beklenen yönde / as expected / the expected way …" yasak; yetkili okurda n > 0 iken karne cümlesi **n ve tutan sayısını**, **beklentinin süresini** (sonuç ufku, bar sayısı) ve **yönünü** (yön/sonuç sözcüğü: TR yüksel, art, çık, yukarı, düş, geril, in-, aşağı, sürdü, devam, dışına, kırıl, dön…; EN rise, up, fall, down, continue, kept, outside, break, turn…) içerir | K49 "karnede beklendiği gibi yerine beklentinin kendisi"; doğal sıklık ("10 örnekte 7") ve pencere (RESEARCH §9.2, §10.1, F2, F3) |
| 2.8 | `contradiction` | başlık ve ayrıntının her biri **anahtarla** karşılaştırılır: geçerlilik sınıfı (sözlük: TR hâlâ geçerli/sürüyor/devam ediyor → `valid`; "geçerli olup olmadığı bilinmiyor" → `unknown`; artık geçerli değil → `no_longer`; gerçekleşti, tutmadı/gerçekleşmedi/geçersiz oldu, süresi doldu, geri çekildi → `closed:*`; EN eşdeğerleri) anahtarınkinden farklıysa; başlıkta yön anahtarın tersi olup anahtarınki hiç geçmiyorsa; göreli gün ("bugün", "dün", "N işlem günü önce", "N bar önce") ya da takvim tarihi kapsülün hiçbir olayına uymuyorsa. Ayrıca başlık ve ayrıntı geçerlilik sınıfı söyleyip hiç ortak sınıfları yoksa. Karne cümleleri (geçmişte, benzer, karne, seferin, örnek / historically, past, record, cases) geçerlilik ve yön için okunmaz; uzun eşleşme içindeki kısa eşleşme sayılmaz ("did not play out" ≠ "played out") | A33 (ayrıntı yüzle çelişebiliyor), kabul 3 "yüzün yapı taşlarından kuruluyor ve çelişmiyor". Yapı taşlarından karşılaştırma: anahtar motorun değeridir, metin değil. Yön yalnız başlıkta: ayrıntı RSI ya da hacmin hareketini anlatır, hikâyenin yönü değildir |
| 2.9a | `equivalence` | her durumda TR ve EN aynı yüzeyde aynı sayı çoklu kümesi (TR "1.234,56" = EN "1,234.56"), aynı saatler, aynı tarihler (ay adları eşlenir), aynı geçerlilik sınıfları; sözlük girdisi sayısı eşit | kabul 3 "TR↔EN eşdeğerliği" |
| 2.9b | `redacted_digit` | yetkisiz okurda hiçbir yüzeyde rakam yok (simge çıkarıldıktan sonra) | K33, K34 |
| 2.10 | `determinism` | aynı durum iki kez üretilince aynı yüzeyler | K3 |

**Bilinen sınırlar (yazılı kabul):** sözlükler (geçerlilik, yön, eşik adı) sözcük düzeyindedir; bir
aday yeni bir deyişle geçerlilik söylerse sınıf "söylenmedi" sayılır (çelişki yakalanmaz ama okuyucu
ölçüsü yakalar); sözlük eklemesi §8'e yazılır. Hafta günü ("cuma günü"), saat sayısı ("2 saat önce")
ve takvim günü sayısı çelişki taramasında okunmaz. RESEARCH F1 ("başlık önden yüklü": ilk üç
sözcükte eşik adı/fiyat/olay fiili) düzenekte **ölçülmedi**; öneri olarak kaldı.

**Başvuru satırı (`r4`, 531 durum × 2 dil; `results/r4/SCAN.md`):**

| vague | advice | repeat | internal | unglossed | length | threshold | ledger_expect | contradiction | equivalence | redacted_digit | determinism | empty |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 756 | 0 | 76 | 2344 | 1557 | 1294 | 1002 | 1581 | 636 | 20 | 0 | 0 | 0 |

Bozuk dil (`bozuk`, §6): 7681 / 0 / 0 / 15088 / 2414 / 3184 / 1854 / 3375 / 396 / 14 / 0 / 0 / 0
(`results/bozuk/SCAN.md`). Taramalar bozuk dili başvurudan açıkça kötü buluyor (vague, internal,
length, threshold, ledger_expect); `contradiction` bozukta düşük, çünkü bozuk başlık geçerlilik
söylemiyor (çelişecek bir şey yok) — eksik geçerlilik okuyucu sorusu S4'te ölçülür.

## 3. Anlama soruları (kör okuyucu)

### 3.1 Personalar (istemde birebir; `readers.rs::persona`)

| kod | persona | metin |
|---|---|---|
| `F` | geleceğin yatırımcısı | "Yirmi yaşındasın. Hisse senedi alıp satıyorsun ama teknik analiz bilmiyorsun; grafik, indikatör, RSI, ortalama gibi terimler sana bir şey ifade etmiyor. Telefonuna bir bildirim geldi; her metne yalnızca birkaç saniye bakacaksın. İlk bakışta ne anladığını cevapla; geri dönüp uzun uzun inceleme." |
| `H` | alışkın kullanıcı (40+, terminal) | "Kırk beş yaşındasın. Yıllardır borsada işlem yapıyorsun; terminal ekranlarına, teknik analize ve indikatörlere alışkınsın; ayrıntıya güvenirsin. Telefonunda bir piyasa uygulamasına bakıyorsun. Okuyup cevapla." |

İstem "uygulama tavsiye vermez" **demez** (S6 bunu ölçer).

### 3.2 Okuyucu kümesi S12 (`readers.rs::S12`)

On iki ölçüt, sırayla; her biri eşleşen ve henüz seçilmemiş durumlardan **kümede henüz olmayan
dedektörü**, sonra **söylenen karneyi** (`counts` > `thin` > `empty`; karne sorusu ayırt etsin diye),
sonra üretim sırasını yeğler; eşleşme yoksa aşama/geçerlilik/karne koşulu düşürülür ve `*` ile yazılır:
kırılım yukarı doğdu geçerli; kırılım aşağı açık geçerli; RSI aşırılık artık geçerli değil; sıkışma
sürüyor; ortalama dizilimi kapandı gerçekleşti; uyumsuzluk kapandı tutmadı; hacim kapandı süresi doldu;
geri çekildi (kurgu); kurumsal akış yetkisiz okur; az örnekli karne; saatlik geçerli; teyit edilmiş açık.
Sayfa 1 = ilk altı, sayfa 2 = son altı; sayfa içi sıra tohumlu karışık. Seçilen küme `runs/<koşu>/set.json`.

### 3.3 Sorular ve anahtar (cevap: JSON; açık sorular **en çok bir cümle** — ilk cümle ve 200 karakter
dışı kesilir; puanlama deterministik anahtarla, dil modeliyle değil — U6)

| soru | biçim | doğru | şans |
|---|---|---|---|
| S1 Ne oldu? | 11 sade olay seçeneği + bilmiyorum (`readers.rs::S1`) | anahtarın olay kategorisi | 1/11 |
| S2 Hangi eşik/sınır ve değeri kaç? | bir cümle | yetkili: cevapta eşik değerlerinden biri (§1.2 toleransı); yetkisiz: cevapta rakam yok | ≈ 0 |
| S3 Ne zaman oldu? | bugün / dün / 2–4 işlem günü önce / bir haftaya yakın ya da daha önce / birkaç saat önce / bilmiyorum | doğuşun ya da son geçişin kovası (saatlikte 0 gün = "bugün" ya da "birkaç saat önce") | 1/5 |
| S4 Hâlâ geçerli mi? | geçerli / artık geçerli değil (kapanmadı) / kapandı: gerçekleşti / tutmadı / süresi doldu / geri çekildi / bilmiyorum | anahtarın geçerlilik sınıfı | 1/6 |
| S5 Geçmişte kaç örnekte kaç kez tuttu? | bir cümle | n > 0: cevapta n ve tutan sayısı; n = 0: rakam yalnız 0 ve "henüz / hiç / örnek yok / karne(si) yok" ya da 0; yetkisiz okur: puanlanmaz | ≈ 0 |
| S6 Bu metin sana ne yapmanı söylüyor? | almamı / satmamı / beklememi / hiçbir şey, yalnız bilgi veriyor / bilmiyorum | "hiçbir şey"; ayrıca **tavsiye okuması** = al ya da sat seçilme oranı | 1/4 |

### 3.4 Sunum: nötr düz metin ve kısa maruz kalma

Dil, görsel tasarımdan önce seçilir (TASKS sıra notu); görüntü verilirse yazı ailesi, boyu ve
hiyerarşi dili gölgeler. Bu yüzden metin **düz metin** olarak, bütün adaylarda aynı biçimde (karne satırı,
başlık, ayrıntı, sözlük parantez içinde) istemin içinde verilir; okuyucuda araç yoktur. Kısa maruz
kalma taklidi iki koşulla yapılır: **`yuz`** — yalnız bildirimde ilk görünen kısım (karne satırı +
başlık; ayrıntı ve sözlük yok); **`acik`** — dokununca açılan ekranın tamamı. Aynı sorular iki koşulda
ayrı koşularda sorulur (okuyucu `yuz`'dayken ayrıntıyı görmez). Sınır: model metni "birkaç saniyede"
okumaz; persona metni ister ama zorlayamaz (R4 dersi: F ≈ H). Görsel tek bakış M16'nın işidir.

### 3.5 Koşu planı (`langh plan`)

Her aday × persona × koşul × sayfa × okuyucu (varsayılan 3) bir koşu; tohum her koşuya ayrı ve
`meta.json`'da. Model varsayılan `haiku` (`claude-haiku-5-5`); `MODEL` ile değişir. İkinci model ailesi
yok (`docs/MODELS.md` anahtarı istenmedi): sınır olarak yazılır; aynı ailede farklı boy (ör. `sonnet`)
ikinci okuyucu olarak kullanılabilir ve aileler/boylar arası uyum raporlanır.

### 3.6 Körlük (U4, U12)

İstem dil adını, kimin yazdığını, "eski/başvuru"yu, kaç aday olduğunu içermez. Diller tohumlu altı hane
onaltılık **kodla** adlanır (`stats::blind_code`); `runs/<koşu>/key.json` (kod → ad) okuyucu dizinine
kopyalanmaz. Her istem gönderilmeden önce `langh check-prompt` yasak sözcük listesine (dil adları,
`r3 r4 vadrel fotolay tavrik bozuk eski başvuru referans reference variant alternatif "bugünkü dil"
"yeni dil" harness düzenek`) bakar; bulursa koşu yapılmaz. Okuyucu süreci: boş geçici dizinde taze
`claude -p --model <m> --tools "" --max-turns 1 --strict-mcp-config --permission-mode dontAsk
--no-session-persistence --output-format json`, istem standart girdiden. Kayıt: `prompt.txt` (gönderilen
istem birebir), `answer.json` (ham çıktı; maliyet `total_cost_usd`), `meta.json` (kod, persona, koşul,
sayfa, tohum, model, madde → durum). Sıcaklık CLI varsayılanıdır (ayarlanamıyor; sınır).

### 3.7 Ölçüler

Soru × persona × koşul doğruluğu; **birincil** = bütün puanlanan sorular ve iki koşul üzerinden doğruluk,
persona ağırlığı F 2, H 1; aralık = persona başına Wilson %95 sınırlarının aynı ağırlıklı ortalaması.
İki adayın aralıkları örtüşüyorsa **"ayırt edilemedi"**; kazanan ilan edilmez. Tavsiye okuması ayrıca.

## 4. Kör eleştirmen ikili karşılaştırması

1. Ölçüt (istemde birebir, `readers.rs::CRITERIA`): somut (eşik adı ve değeri); ne zaman ve hâlâ geçerli
   mi ilk okumada; karne dürüst ve beklentiyi söylüyor; iç terim/jargon/belirsiz ifade yok; eylem
   önermiyor; kısa ve hızlı, ilk satır yeterli; kısa metin ile ayrıntı çelişmiyor. Eleştirmen önce kendi
   gözlemini bir cümle yazar, sonra "A" ya da "B" seçer (eşitlik yok; U6); okuyucu cevaplarını görmez.
2. Her aday çifti, S12'nin 12 durumu (koşul `acik`), **iki sırada** (A/B yer değiştirir) ve **3 tekrar**
   (farklı tohumlu madde sırası) (U1, U11). Bir (çift, tekrar, durum) birimi: iki sırada aynı kazanan =
   galibiyet; bölünme = beraberlik (0,5). **Çelişki oranı** çift başına raporlanır; > %30 ise o çift
   "ayırt edilemedi".
3. Birleştirme: Bradley–Terry (MM, her çifte bir beraberlik önsel), birimlerin yerine koyarak önyüklemesi
   (1000 kez, tohum kayıtlı) → %95 aralık. Aralıklar örtüşürse kazanan ilan edilmez (U5).
4. **Uzunluk yanlılığı (U2):** her birimde iki metnin karakter sayısı kaydedilir (`meta.lens`); kararlı
   birimlerde uzun metnin kazanma oranı ve korelasyonu raporlanır. Tercih ≠ performans (U9): eleştirmen
   sonucu anlama doğruluğundan ayrı raporlanır, seçimde ikincildir.

## 5. Seçim kuralı (önceden yazılı)

1. **Sert eşikler:** §2'nin bütün taramaları adayın bütün dökümünde (531 durum × 2 dil) **0**. Bir taramayı
   geçemeyen aday elenir. Hiçbir aday bir taramayı geçemiyorsa ve neden dilden bağımsızsa (ör. düzenek
   sözlüğü eksik), o tarama için tarihli ek yazılır (§8; R4 E6 gibi); dilin kendisindense hiçbir aday
   seçilmez ve bu yazılır. Başvuru `r4` sıralanır, seçilmez.
2. **Birincil:** anlama doğruluğu (§3.7). İlk iki aday aralıkları ayrıksa birinci seçilir.
3. Örtüşüyorsa (**ayırt edilemedi**): eleştirmen BT (§4). İlk iki aralık ayrık ve çelişki ≤ %30 ise BT
   birincisi.
4. O da ayırt edemezse: TR+EN ortalama (başlık + ayrıntı) karakter sayısı **kısa** olan.
5. İlk iki aday gösterimde seçilebilir kalır ve galeriye gider (kabul 11).
6. **Geçerlilik koşulu:** `bozuk` birincilde ve BT'de **son** değilse düzenek geçersizdir; seçim yapılmaz,
   düzenek düzeltilip yeniden koşulur.

## 6. Bozuk dil sınaması

`tool/src/adapters/bozuk.rs`: şablon değiştirmeden, başvurunun yapı taşlarından metin düzeyinde üretilir:
belirsiz ("önemli bir eşikte güçlü sinyal", "kritik seviye", "dikkat çekici"), iç terimli (olgu, dedektör,
bar, teyit, aile, bulgu, kademe, skor), uzun, eşiksiz (ayrıntıdaki bütün sayılar silinir), başlıkta
zaman ve geçerlilik yok, karne "çoğunlukla beklendiği gibi". Olay adını korur (gürültü değil, dildir).
Her okuyucu koşusuna gizlice eklenir (kodu ötekiler gibi; `key.json`'da adı).

**Kuru koşu sonucu** (`runs/dry2/RESULTS.md`; `r4` + `bozuk`, F ve H, iki koşul, 3 okuyucu, eleştirmen 3
tekrar × 2 sıra; `haiku`; 54 koşu, **0,158 $**): birincil `r4` 0,876 [0,841; 0,904], `bozuk` 0,494
[0,447; 0,541] → ayrık; BT `r4` > `bozuk` (36/36 birim, çelişki 0); **bozuk son → düzenek geçerli.**
Ayırt edici sorular S2 (eşik, `acik`'ta `r4` 0,69–0,72, `bozuk` 0,08), S3 (`yuz`'da `bozuk` 0,00),
S4 ve S5; S1 ve S6 tavanda (R4'teki gibi). Bu sınama yalnız "çok kötü dili sona koyabiliyor mu"yu
gösterir; iyi adaylar arasındaki ayırma gücünü göstermez (§7 riskler).

## 7. Riskler ve sınırlar

- **Tavan:** başvuru zaten 0,88; S1 ve S6 tavanda. Adaylar arası farkın çoğu S2 (eşik adı/değeri), S5 (karne)
  ve `yuz` koşulunun S3/S4'ünden gelecek. Okuyucu sayısı 3 ile Wilson aralığı ≈ ±0,03–0,05; küçük farklar
  "ayırt edilemedi" çıkar. Gerekirse okuyucu sayısı artırılır (maliyet doğrusal).
- **Tek model ailesi** (Claude); ikinci aile yok (sınır). Haiku küçük bir modeldir; `sonnet` ile tekrar
  önerilir (maliyet ≈ 10×).
- **Sözlük tabanlı taramalar** yeni deyişleri kaçırabilir (§2 bilinen sınırlar); bu yüzden sert eşikler
  okuyucu ölçüsüyle birlikte okunur.
- **Durum seçimi sentetik veriden:** ilk-durum kuralı karne kovası dağılımını belirler; S12'de ölçütle
  dengelenir. Gerçek veride (yerel) dil dökümü yapılmaz (gerçek veriden türev commit edilmez).
- Okuyucu süreci kullanıcının genel Claude ayarlarını (ör. `~/.claude`) yükleyebilir; proje `CLAUDE.md`'si
  geçici dizinde yoktur. Sıcaklık ayarlanamıyor.

## 8. Ekler (tarihli; alternatiflerden önce)

- (yok)
