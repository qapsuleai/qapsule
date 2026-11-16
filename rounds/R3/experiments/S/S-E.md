# R3/S-E — Lisanslı akışın girişimsizliği (denetim koşulu K1)

Dal `r3/fix-k1` (`r3/int` `ee75ba3` üstünde). Yazılan yerler: `crates/qp-session/`
(`src/public.rs` yeni; `capsule.rs`, `session.rs`, `lib.rs`; testler
`tests/non_interference.rs` yeni, `tests/capsules.rs`, `tests/capsule_perf_equality.rs`),
`fixtures/protocol/conformance/39-capsule-entitlement.json` (yeniden üretildi),
`docs/PROTOCOL.md` §20.2 ve §20.5, `docs/CAPSULE.md` §9.7 (bir cümle), `docs/CONCEPTS.md`
("Açık dünya", koddan önce ayrı commit), `DECISIONS.md` Ö37 notu, bu dosya.

## 1. Sorun (denetim Y1, O1)

`kinddir` teması lisanslı `flow.institutional` olgusunu **açık** bir hikâyeye teyit olarak
ekliyor. Otorite `Finding.confidence = story.strength` (gizli olgu dahil en büyük güç)
yazıyordu; kapsülün `importance`/`tier`'ı, `capsules` sırası ve karne sayıları
(sıralayıcının `RankCtx`'indeki `stories_all`/`ledger_all`, `ledger` sorgusu) lisanslı
hikâyeleri sayıyordu. İzdüşüm (`project_doc`, `project_story`) yalnız hikâyenin içini
düzeltiyordu. Sonuç: akış yetkisi olmayan Bora ve hiç yetkisi olmayan Can, `document`,
`log`, `list`, `changes` yollarıyla `(|net|/avg5 − 2)/2`'yi tam `f64` alıyordu (mühürün
≤ log2 7 bit sınırını atlayarak); güvenin 1,0 ↔ başka değer farkı olgunun **varlığını**
da söylüyordu. Alan yamalamak (bulgu güvenini yeniden hesaplamak vb.) yetmez: önem, sıra,
karne, olayların varlığı ve zamanı, hikâyelerin birleşme biçimi de lisanslı veriye bağlı.

## 2. Tasarım: açık dünya (girişimsizlik yapı gereği)

**İkinci motor.** Motor lisanslı akış okuyorsa (`init.capsules.flow`) otorite bir
**açık-yalnız motor** daha koşar: aynı `EngineConfig::r3()`, aynı kesin barlar, aynı
dedektör çıktısından yalnız açık lisans sınıfındaki olgular (`qp-session/src/capsule.rs`
`run_capsules`). Akış yoksa ikinci motor ve açık dünya hiç kurulmaz (davranış ve maliyet
eskisi gibi).

**Açık dünya** (`qp-session/src/public.rs`, `World`): ikinci bir doküman, günlük, okuma
dokümanı ve bunların önbellekleri (kimlik dizinleri, cümle belleği, okuyucu izdüşümleri ve
görünümleri, başlıklar). Otoritenin günlüğüyle girdi girdiye eşleşir (aynı `seq`, `at`,
yazar, `cause`); her girdinin açık sürümü `Session::committed` içinde yazılır:

| Otoritenin girdisi | Açık dünyanın girdisi |
|---|---|
| Motor bakışı (`capsules <seri> bar i`) | Açık-yalnız motorun kendi değişimleri: geçişe yol açan olgular, bulgular (`confidence` = açık hikâyenin gücü, iddia = açık cümle), kapsüller, karne satırları (`capsule_changes`, açık dünyada çalışır: var olan kapsülün onay/ret durumu ve görünümü açık dünyadakinden). |
| Düzeltilmiş veri cevabı (§20.7) | Her iki motorda aynı düzeltilmiş olgulara göre ayrı ayrı geri çekme. Açık dünya varken girdi, hiçbir şey geri çekilmese de yazılır (özet `settled <seri>`): girdinin varlığı lisanslı hikâyelerin bozulmasına bağlı olmasın. Açık görünüm okuyucusunun itirazı açık-yalnız motorda karara bağlanır. |
| Yetkili kişinin doğrudan komutu | Aynı işlemler açık dünyada genişletilir (`expand_ops`, açık doküman ve günlükle): işlemlerin ve açık dünyanın fonksiyonu, otoritenin değişimlerinin çevirisi değil. Uymazsa (ör. lisanslı olguya dayanan kendi bulgusu) çeviri (aşağıda): kişinin kendi öğeleri iki dünyada da olur. |
| Açık görünüm okuyucusunun komutu / önerisi | Yalnız açık dünyada doğrulanır ve genişletilir: hata, `base` çakışması, atanan kimlik, `undoes` ve girdinin değişimleri oradan. Otoritenin dünyası aynı işlemleri ancak orada da uyuyor ve iki dünyanın paylaşmadığı bir motor öğesine (yalnız otoritede olan ya da kimliği açık dünyada başka bir öğenin olan) dokunmuyorsa alır (`diverged_touch`); yoksa açık dünyanın değişimlerinin ters çevirisini (`reverse_changes`: kişinin kendi öğeleri korunur, yalnız açık dünyada olan motor öğeleri düşer). Okuyucunun gözleyebileceği hiçbir şey otoritenin dünyasına bağlı değil. |
| Diğerleri (barlar, seriler, çapa damgaları, öneri ve kabulü, diğer motor girdileri) | Çeviri (`public::translate`): açık dünyada **kimliği** olmayan motor öğesine dokunan değişim düşer; kimliği olan bulgu/kapsül açık dünyanın içeriğini korur ve yalnız değişeni alır (iddia, güven, geçersizlik; görünüm, durum; geri çekme); geri alma açık sürümün tersini uygular. Yalnız bar yazan girdiler olduğu gibi. |
| Hepsi | İstek, özet ve neden otoritenin yalnız kendinde olan motor öğesini anıyorsa izdüşümdeki gibi temizlenir. |

**Açık görünüm okuyucusu.** Lisanslı akışların **hepsine** yetkili olmayan okuyucu
(`Session::public_view`). Kısmi yetki de açık görünüm alır: sıralayıcının bağlamı bütün
enstrümanları kapsadığından, yetkisi olmayan bir akıştan arınmış hikâye ancak açık dünyada
var. Bu okuyucu için bütün okuma yolları açık dünya alanlara takılarak çalışır
(`Session::in_world`, O(1) takas): `as_reader` (bütün sorgular, `subscribe`, `hello`
özeti, girdi, dikkat, seçici çözümü, hata önerileri, komutların kuru koşusu), `change` ve
`finding_state` olayları (`entry_events`), `capsules` konusu (açık-yalnız motorun
geçişleri), `ledger` (açık-yalnız motorun karnesi), itiraz. Üstüne olağan izdüşüm (bar
yetkisi: rakamsız cümle, değersiz kanıt; başkasının kitlesi) uygulanır. Her bağlantının
satırları tek dünyadan, o dünyanın sırasıyla gelir.

**Yetkili okuyucu** (bütün akışlar) değişmedi: otoritenin dünyası, eski izdüşüm (artık
yalnız "akışa yetkili ama bar beslemesine yetkisiz" gibi durumlarda iş görür).

**Otoritenin dokümanı ve günlüğü değişmedi**: açık dünya saklanmaz, günlüğe girmez,
gönderilmez; otoritenin girdilerinin (barlar, komutlar) fonksiyonudur. `replay` aynen
kesin; aynı girdiler aynı iki dünyayı verir.

**Kalan kanal.** Açık görünüm okuyucusuna lisanslı veriden etkilenen tek kanal mühür
iletileridir (`trust: server`; `capsules` sorgusunun `comments`'i, `server_comment`
olayı). K33 gereği bilerek bırakılan başka kanal **yok**: karne ve sıralayıcı bağlamı da
açık-yalnız motorun. "Karne lisanstan bağımsızdır" (K33) bu okuyucu için artık "açık-yalnız
motorun karnesi" demek (Ö37 notu). Kişilerin komutları (yetkili kişinin neyi onayladığı,
ne yazdığı) açık dünyaya geçer: girişimsizlik lisanslı **veri** içindir, kişinin seçimi
için değil (yetkili kişi zaten her şeyi yazabilir); lisanslı öğe adını anan istek/özet
eskisi gibi temizlenir.

**Kimlik çakışması.** Kapsül kimliği enstrüman, tür, yön ve doğum barıdır: açık dünyada
bir hacim sıçraması hikâyesi (`flow` türü, açık) otoritedeki lisanslı doğumlu hikâyenin
kimliğini taşıyabilir (`cap.THYAO.flow.up.57`). İkisi ayrı öğedir; okuyucuya gideni açık
olanıdır (yetki taraması bunu hesaba katıyor, §3).

**Sıra numarası.** İki dünya `seq`'i paylaşır (ayrı bir açık `seq` yok: `seq` her mesajda,
`base`'te, `undo`'da, `after_seq`'te; ikisini eşlemek protokolün her yerine dokunurdu).
Bu yüzden otoritenin her girdisinin **varlığı** açık veriye ve kişilerin komutlarına bağlı
olacak biçimde kuruldu: motor bakışı her bar kapanışında, düzeltme girdisi her düzeltilmiş
cevapta, çapa damgası bağlamadan, komut kişiden; §8.1 geçersizleşmesi motor kapsülüne
(izlenen serideki hikâye) hiç uygulanmaz (motor kapsülü yalnız sonuç kuralıyla kapanır — modülün zaten
söylediği; artık `update_finding` ile yazılmış veri koşulu da değerlendirilmez); kişinin
kendi kapsülleri iki dünyada da bulunur (çeviriler onları korur), koşulları aynı açık
barlarda aynı anda tutar, girdinin kanıtı açık veridir. Açık dünyada karşılığı olmayan bir
motor girdisi (kuruluş gereği yazılmaz) orada kanıtsız ve özeti `…` olurdu.

## 2b. Güvenlik incelemesi (dört ORTA bulgu) ve düzeltmeleri

İlk sürümün (`58e7e63`) güvenlik incelemesi dört girişimsizlik kırığı buldu; dördü de
düzeltildi ve girişimsizlik testine eklendi (§3, test 3):

1. **Geçersizleşme girdileri** (`invalidations()`): otoritenin günlüğündeki
   "invalidate {cap}: invalidation of {fnd} holds" girdileri (kanıtıyla) açık günlüğe
   özeti `…` yapılarak kopyalanıyordu; varlık, zaman ve kanıt lisanslı dünyaya bağlı
   olabiliyordu (ör. sahibin motor bulgusuna yazdığı veri koşulu, iki dünyada farklı
   durumdaki kapsülde). Düzeltme: `invalidate_these` çalışan motorun kapsülünü (izlenen serideki hikâye) hiç
   değerlendirmez; kişinin kendi kapsülleri iki dünyada da var (yetkili kişinin açık
   dünyada uymayan komutu çevrilir, açık görünüm okuyucusununki ters çevrilir), böylece
   geçersizleşme girdisinin varlığı ve kanıtı açık veriden; kalan her durumda karşılıksız
   motor girdisi açık dünyada kanıtsız ve `…`. Sıra numarası kararı yukarıda.
2. **Öneri yolu**: kuru koşudan sonra `expand_ops(&self.doc, …)` otoritenin dokümanında
   koşuyordu; hata ve öneri (`did_you_mean`) açık görünüm okuyucusuna gidiyordu (eski kodda
   test 3 tam bunu yakaladı: `not_found` önerileri tohuma göre farklı). Düzeltme: açık
   görünüm okuyucusunun önerisi yalnız açık dünyada doğrulanır (`base` çakışması dahil).
3. **Komut yolu**: açık görünüm okuyucusunun işlemi, otoritede başka bir öğeye (lisanslı
   doğumlu, aynı kimlik) uygulanabiliyordu; `created`, `undoes` ve `base` denetimi
   otoritenin genişletmesinden geliyordu. Düzeltme: yukarıdaki tablo (yalnız açık
   dünyadan cevap; otoritenin dünyası paylaşılmayan motor öğesine dokunmaz).
4. **İtiraz yönlendirmesi** (`dispute.rs`): `engine_capsule` otoritenin dünyasında
   bakılıyordu. Düzeltme: itirazcının dünyasında (`in_world(public, …)`); açık görünüm
   okuyucusu için yalnız otoritede olan kapsül, olmayan kapsülle aynı yoldan gider. (Bu
   kurguda iki yolun cevabı zaten aynıydı — `unknown`, aynı `digest` hatası —; test 3 eski
   yönlendirmeyle de geçiyor; düzeltme bunu kuruluş gereği yapıyor.)

## 3. Testler

- **Girişimsizlik** (`crates/qp-session/tests/non_interference.rs`, yeni):
  1. Denetçinin `session_probe`'unun koşusu: döküm 39 kurgusu, 200 bar
     (`synthetic_market(1, 200, 1)`), Bora ve Can kitle yazar ve `capsules`,
     `capsules {all}`, `capsules {instrument}`, `changes`, `claims` konularına abone olur;
     sonra 4 bağlantıda 16'şar sorgu (`capsules` beş biçim, `ledger` iki, `describe`,
     `log`, `document`, `list` facts/capsules/findings, `present` iki, `verify_evidence`).
     Akış tohumu 1 ↔ 2, 7, 42 ve **akışsız** (`flow: []`): Bora ve Can'a giden bütün
     satırlar, mühür iletileri (`trust: server`, `comments`, `server_comment`) çıkarıldıktan
     sonra **bayt eşit**. Pozitif kontroller: Ana'nın (tam yetkili) satırları tohumla
     değişiyor; mühür iletileri değişiyor.
  2. Döküm 43 kurgusu (kötü baskı, Bora'nın itirazı, düzeltilmiş veriyle geri çekme) +
     Can ve Arkın (sahip, yetkili): Arkın itiraz edilen bulguyu onaylar, gösterir, kitlesini
     yazar; Bora ve Can'a sorgular. Tohum 1 ↔ 2, 7, 42, akışsız: Bora ve Can'ın satırları
     mühür dışında bayt eşit (karar `upheld`, `dispute_upheld`, görünüm `v152` onlara da
     gidiyor); Ana'nınkiler değişiyor.
  3. Komut, öneri, itiraz ve geçersizleşme kâhini (güvenlik incelemesi): Bora'ya yazma
     yetkisi verilir (rolü `owner`; veri yetkisi aynı) ve **herhangi bir tohumun
     otoritesinde olup açık dünyada olmayan bütün motor kimlikleri** ile açık dünyanın
     bütün kimlikleri üzerinde doğrudan `approve_finding`, `mode: propose` ile
     `reject_finding`, iki `dispute` (biri geçersiz `digest`'le) ve `capsule` sorgusu
     gönderir. Arkın (yetkili) kendi olgu + bulgu + kapsülünü hemen tutan bir veri
     koşuluyla ekler (geçersizleşme girdisi, kanıtıyla) ve her dünyada olan bir motor
     bulgusuna veri koşulu yazar (hiç değerlendirilmez). Tohum 1 ↔ 2, 7, 42, akışsız: Bora
     ve Can'ın satırları mühür dışında bayt eşit.
  - Eski kodda (`ee75ba3`) 1. test kırılıyor (`seed 1 vs 2: b differs: 263 vs 265 lines`);
    ilk K1 sürümünde (`58e7e63`) 3. test kırılıyor (öneri yolunun `not_found` önerileri
    otoritenin dünyasından; motor bulgusunun geçersizleşmesi).
- **Yeniden oynatma, yetki taraması, gizlilik** (`tests/capsules.rs`): yeşil. Yetki
  taramasında tek değişiklik: yasak kimlik listesinden, akışsız aynı koşunun dokümanında da
  olan (yani açık dünyada açık bir hikâye olan) kimlikler çıkarıldı (§2 kimlik çakışması);
  lisanslı olgu kimlikleri ve yalnız lisanslı doğumlu kimlikler yasak kaldı.
- **`capsule_perf_equality.rs`**: özet artık bağlantı başına da yazılıyor. Bağlı senaryo
  (6 enstrüman, 3'ünde akış): tam yetkili bağlantıların satırları (`a`, `i` Arkın; `f`
  besleme), günlük (1.177.744 B, `e38025b8f0d9a037`) ve doküman (661.323 B,
  `01db88a5bab1d57f`) **önceki kayıtla bayt eşit**. Değişen satırlar yalnız tam yetkili
  olmayan okuyucularınki: Bora (`b`, `g`), Can (`c`, `e`), Ana (`d`, `h`; üç akıştan yalnız
  birine yetkili → açık görünüm). Toplam satır özeti `lines 3023 … 923f2ecc80daa91b` →
  `lines 3017 21188765 5d85e68b720ce155`. Belge içi senaryo (akış yok) bütünüyle aynı.
- **Dökümler**: 44 × inproc/serve/mcp yeşil (verify 14). 01–38 ve 40–44 bayt eşit;
  yalnız **39** yeniden üretildi (`make_capsule_transcripts -- --write`). Değişen satırların
  hepsi Bora'ya (`b`) ya da Can'a (`c`):
  - Bar 91 olayında `capsules {all}` konusu: `cap.THYAO.reversal.down.76` `importance`
    1,490984 → 1,485308 (sıralayıcının bağlamında lisanslı hikâyeler yok), Bora ve Can;
  - `e2` (Bora, `capsule cap.THYAO.flow.up.90`): yine `not_found`, ama öneri artık
    `cap.THYAO.flow.up.57` (açık dünyadaki açık bir hacim sıçraması hikâyesi);
  - `e3` (Bora, `capsules since_bar 85`): aynı önem farkı;
  - `e4` (Can, `capsule cap.THYAO.continuation.down.84`): `importance` 1,454755 → 1,452844
    (öğe ve işaret tarifi);
  - `e5` (Bora, `ledger by_kind`): `flow` satırı `n` 7 → 2, `realized` 5 → 1; toplam
    17/8 → 12/4 (lisanslı hikâyelerin sonuçları artık sayılmıyor).
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (459 geçti, 0 kaldı, 2 yok sayıldı) ve
  `QP_VERIFY_BROWSER=0 scripts/verify.sh`: §5.

## 4. Performans (ikinci motorun maliyeti)

`capsule_tick -- 30 1000 600 3 caps,flow bound` (S-D §1.1 yöntemi; 30 enstrüman, 1.000
bar geçmiş, 600 piyasa tiki, 10 kapanış, 3 koşu, medyan). Aynı gün, aynı makine (Apple M4
Max), yük ortalaması 6,0–7,2: düzeltmeden önceki kod (`ee75ba3`, `git archive` ile ayrı
derlendi) ve K1'in son hâli, sırayla iki kez. Ham: `raw/perf/k1-base-*-run{1,2}.txt`,
`raw/perf/k1-after-*-run{1,2}.txt`.

| Ölçü (bağlı yol) | S-D "Düzeltmeden sonra" | Önce (`ee75ba3`), koşu 1 / 2 | K1, koşu 1 / 2 | Fark |
|---|---:|---:|---:|---:|
| `caps` kapanış tiki (30 enstrüman) | 48,2 ms | 48,7 / 47,9 ms | 50,3 / 50,5 ms | +%4 (akış yok: açık dünya kurulmaz; aynı kod yolu, gürültü) |
| `caps` düzeltme tiki, olay başına | 2,27 µs | 2,28 / 2,28 µs | 2,32 / 2,28 µs | ≈ 0 |
| `flow` `Session::new` (1.000 bar yetişme) | 1,05 s | 1,044 / 1,049 s | 1,373 / 1,377 s | **+%31** (ikinci motor + açık dünyanın yazımı) |
| `flow` bağlanma (4 `hello` + 6 abone) | 247 ms | 256 / 251 ms | 173 / 177 ms | **−%31** |
| `flow` düzeltme tiki, olay başına | 2,33 µs | 2,34 / 2,33 µs | 2,39 / 2,39 µs | +%2,5 (veri değişimi açık dünyanın okuma dokümanına da) |
| `flow` kapanış tiki (30 enstrüman) | 89,6 ms | 89,4 / 87,7 ms | 77,8 / 77,2 ms | **−%12** |
| `flow` doküman özeti | `c0f2042a1be8204a` | aynı | aynı | otoritenin dokümanı değişmedi |

Okuma: ikinci motorun kendisi ucuz (S-D §2: 30 enstrümanda `on_bar` ~0,2 ms); maliyet
açık dünyanın yazımında (kapsül değişimleri, cümleler, uygulama). Buna karşılık yetkisiz
okuyucular artık otoritenin dünyasında hikâye başına izdüşüm ve lisanslı katılımlı
hikâyelerin yeniden yazımı istemiyor; kapanış ve bağlanma bu yüzden **ucuzladı**.
Açık dünyanın başlıkları (TR) her motor girdisinden sonra değil, bir açık görünüm okuyucusu
okuduğunda yenilenir (`as_reader`; aynı artımlı `refresh_headlines`, çıktı aynı — eşitlik
testleri ve dökümler bunu sınar). Ara sürümde (başlıklar her girdiden sonra) `flow`
kapanışı 129,5 ms, bağlanma 160 ms, `Session::new` 1,40 s ölçülmüştü (o ham dosya
tutulmadı). `flow` düzeninde Bora/Can'a giden satır sayısı 516 → 520 (açık-yalnız motorun
geçişleri).

## 5. Doğrulama

`CARGO_INCREMENTAL=0 QP_VERIFY_BROWSER=0 scripts/verify.sh` → **verify: OK** (201 s):
fmt, clippy, test (459 geçti, 0 kaldı, 2 yok sayıldı), schema (28), fixtures, golden,
views (12 × 2), wasm (gzip -9 616.021 B, değişmedi), hygiene, compute, digest (arm64,
x86_64 `7b1e99d40889f83c`), protocol (61 örnek), conform (44 döküm × inproc, serve, mcp),
replica PASS; browser SKIP (istendiği gibi). `git diff ee75ba3 -- fixtures/`: yalnız
döküm 39. Ara bir koşuda döküm 37 kırıldı (belgede motor olmadan hikâyeli bir kapsül var,
§8.1 ile geçersizleşir); bu yüzden geçersizleşme dışı bırakma yalnız **çalışan motorun**
kapsülüne (izlenen serideki hikâye) uygulanıyor. Performans ölçümü bu son küçük
değişiklikten önceydi (yalnız geçersizleşme adayında bir dizin bakışı ekliyor).

## 6. Sınırlar ve öneriler

- **Kısmi akış yetkisi** açık görünüm alır: birden çok akış okunurken yalnız bazılarına
  yetkili okuyucu, yetkili olduğu akışın lisanslı hikâyelerini görmez (güvenli taraf; R3'te
  tek akış kaynağı sentetik test akışı). Öneri: okuyucuların gerçekte bulunan yetki alt
  kümeleri başına motor (başlangıçta tembel kurulum için bakış başına olgu kaydı gerekir).
- Kişilerin komutları açık dünyaya geçer (yetkili kişinin hangi kapsülü onayladığı); lisanslı
  öğeyi anan öneri/görünüm/anlatı düşer, istek ve özet temizlenir. Bu bir insan kanalıdır,
  veri kanalı değil.
- Açık dünya bellekte ikinci bir doküman, günlük ve okuma dokümanı tutar (akış varken).

## 7. K1b — ikinci güvenlik incelemesi ve yeniden denetim (kanal a, b)

Dal `r3/fix-k1b` (`r3/int` `ea254ab` üstünde; denetçinin `k1_probe`'u için `r3/int` `4ffebc3`
birleştirildi). Bulgular:

| # | Kaynak | Bulgu | Düzeltme |
|---|---|---|---|
| 1 | Güvenlik incelemesi (YÜKSEK); yeniden denetim kanal (a), deneme 4 | Açık görünümlü yazarın komutu otoritede uygulanamayınca (`diverged_touch`/`base`) ters çeviri + `apply_each` otoritenin dokümanına `put_*` yazıyordu: yalnız otoritede olan bir motor öğesini (ya da başka koleksiyondaki aynı kimliği) ezebiliyordu (`k1_probe`: akışlı her yapılandırmada lisanslı doğumlu 1 kapsül değişiyordu) | Açık görünümlü yazarın komutu otoriteye **yalnız** açık değişimlerin ters çevirisiyle ulaşır (`reverse_changes`): her değişim iki dünyanın aynı tuttuğu bir öğeyi adlandırmalı — iki belgede aynı koleksiyon (ya da ikisinde de yok), hiçbir belgede motor öğesi değil (`meaning`'li olgu, motor bulgusu, hikâyeli kapsül) —, çeviri hiçbir değişimi düşürmemeli ya da bulguyu olgusuz bırakmamalı; değişimler **atomik** uygulanır. Biri tutmazsa otorite hiçbir şey almaz; yazar açık dünyanın cevabını aldığı için bunu göremez (hata otoriteden gelmez) |
| 2 | Güvenlik incelemesi (ORTA) | `accept_proposal` otoritede genişletiliyor, otoritenin hatasını/kimliklerini döndürüyordu | Kabul de her dünyada kendi başına: açık görünümlü karar verenin kabulü açık dünyada genişletilir ve `base`'i orada denetlenir (cevap oradan); otorite yalnız ters çeviriyi (1) alır. Yetkili karar verenin kabulü otoritede (cevabı oradan) ve, aynı öneriyi açık dünya da tutuyorsa (`prop_map`), açık dünyada kendi başına. Kuru koşu `accept_proposal`'ı hâlâ atlıyor: açık görünümlü için doğrulama `accept` içinde, açık dünyada, her şeyden önce yapılıyor |
| 3 | Güvenlik incelemesi (ORTA) | `apply_each` değişimleri tek tek uygulayıp tutmayanı sessizce atlıyordu (kısmi girdi) | `apply_atomic`: hep ya da hiç (`apply_in_place` geri alır); açık dünyaya yazılan her girdi de atomik (tutmazsa o dünyada girdi yok) |
| 4 | Yeniden denetim kanal (b), deneme 3 | **Yetkili** bir kişinin (Arkın) her yapılandırmada aynı komut listesi, yalnız lisanslı bir bulguya dokunan her komut için açık günlüğe boş bir girdi (`changes: []`, `summary: "…"`, `cause`) yazıyor, sonraki `seq`'leri kaydırıyordu (47–4.266 farklı satır) | **Açık dünyanın kendi sıra numaraları** (aşağıda) |

### Karar: açık dünyanın kendi sıra numaraları

İki seçenek vardı: (i) her kişi komutu her iki dünyada her zaman tam bir girdi yazar, ya da
(ii) açık dünyanın kendi `seq`'i olur ve açık karşılığı olmayan otorite girdisi orada hiç
yer tutmaz. (i) yetmez: otoritede **başarısız** olan bir komut (hedefi o yapılandırmada
otoritede yok) otoritede girdi yazmaz; `seq` ortakken açık dünya da yazamaz. Hedef açık
dünyada varsa (açık bir hikâye) bu, girdinin varlığını yine lisanslı veriye bağlar. Ayrıca
otoritedeki her otomatik girdi (yalnız otoritede olan kişi kapsülünün geçersizleşmesi) yer
tutardı. **(ii) seçildi**:

- Açık dünyanın günlüğü kendi numarasıyla büyür (`commit_public`); eşlemeler
  `full_to_pub` (otorite girdisi → yaptığı açık girdi) ve `pub_at` (otorite günlüğü
  uzunluğu → o sırada açık günlüğün uzunluğu).
- Açık dünya **her kişinin komutunu kendi başına** genişletir (açık `seq` ve kimliklerle;
  `pub_expand`): orada uyuyorsa girdi, uymuyorsa hiçbir şey — otoritede ne olduğundan
  bağımsız (yetkili kişinin otoritede başarısız olan ama açık dünyada uyan komutu açık
  dünyada uygulanır; kişi otoritenin hatasını alır). Yetkili kişinin `undo`'su ve
  `accept_proposal`'ı otoritenin girdilerini adlandırır: açık karşılığına çevrilir
  (`ops_to_pub`, `prop_map`), yoksa açık dünya hiçbir şey almaz. Öneriler de böyle; açık
  görünümlü önerenin önerisi otoriteye, kimlik iki dünyada boşsa aynı kimlikle geçer.
- Motor bakışları (bar kapanışı başına) ve düzeltme girdileri her zaman; çapa damgaları ve
  bar yazıları aynen; otoritenin diğer motor girdileri (geçersizleşme vb.) **hiç**.
- Açık dünyanın kendi kapsüllerinin (kişilerin) §8.1 geçersizleşmesi açık dünyada
  değerlendirilir ve yazılır (`invalidate_in_public`; iddia kaydı yok).
- Açık görünüm okuyucusuna giden her `seq` açık dünyanın: `welcome`, sorgu ve girdi
  `result`'ları, `ack`, `change`/`finding_state`/`proposal` (açık girdinin numarası),
  `capsule`/`server_comment` (bakışın açık girdisi), `condition_met`, `intent_triggered`,
  `view_changed`, `claim`, itiraz cevabı (`seq_to`). `claims` konusunda açık görünümlü
  okuyucu yalnız kendi koşul iddialarını, `intents` konusunda açık dünyada aynı kimlik ve
  tetikle olan niyetleri alır (belge öğesi iddiaları ve niyetler otoritenin dünyasında
  değerlendiriliyor).
- Otoritenin dokümanı ve günlüğü değişmedi; yeniden oynatma kesin. Açık dünya yokken (akış
  yok) bütün yollar eskisi gibi (tek dünya kodu ayrı dalda, değişmeden).

### Testler (K1b)

- `non_interference.rs` 3. test genişletildi: Bora (rolü `owner`, veri yetkisi aynı) her
  öneriyi kabul eder (açık görünümlü karar veren); bütün tohumların otoritelerinde olup
  açık dünyada olmayan kimliklerle (önce tohum 1'inkiler) **kendi** bulgu ve kapsülünü
  yazar (kimlik çakışması); **Arkın (yetkili) her yapılandırmada aynı listeyle** her
  bulgu kimliğini onaylar/reddeder ve birini gösterir (denetim deneme 3); Bora'nın
  eylemlerinden önce ve sonra otoritenin bütün motor öğeleri (hikâyeli kapsüller, motor
  bulguları) bayt eşit (deneme 4); Bora'nın kendi kapsülü açık dünyada geçersizleşir;
  çakışan kimlikteki otorite bulgusu motorun kalır. Tohum 1 ↔ 2/7/42: Bora ve Can'ın
  satırları mühür dışında bayt eşit. (Akışsız koşu bu testte karşılaştırılmıyor: akış
  yokken açık dünya yok, Bora otoriteyi okur ve belge kapsüllerinin iddialarını da alır.)
  Eski kodda (`4ffebc3`) deneme 4 kırılıyor ("a public-view writer changed an authority
  engine item").
- `capsule_perf_equality.rs`: tam yetkili bağlantılar (a, f, i), günlük ve doküman yine
  bayt eşit; tam yetkili olmayanların satırları açık numaralarla (3017 → 3014 satır,
  `d43ff5b9b8b70b19`).
- Dökümler: 44 × 3 taşıma; `make_capsule_transcripts`: 38–44 hepsi "same" (bu dökümlerde
  açık dünya ile otoritenin numaraları çakışıyor); 01–37 değişmedi.
- **`k1_probe`** (denetçinin; `gen_bars.py k1_bars.json 13 2 200`, 200 bar, iki enstrüman,
  beş yapılandırma): Arkın'ın komutlarıyla ve `K1_NO_ARKIN=1` ile, dört karşılaştırmanın
  hepsinde (akışsız dahil) Bora, Can, Dora, Eda, Fatih **0 farklı satır** (Arkın'lı:
  4.465 / 4.464 / 4.463 / 4.829 / 4.128 satır; Arkın'sız: 4.416 / 4.415 / 4.414 / 4.829 /
  4.128); Ana'nınki 4.271–4.456 satır farklı (pozitif kontrol); açık görünümlü yazarların
  otoritede değiştirdiği lisanslı doğumlu kapsül her yapılandırmada **0**.

### Kalan

- Açık görünümlü okuyucu belge öğesi iddialarını (`claims`: kapsül ve niyet) ve açık
  dünyada aynı olmayan niyetlerin tetiklerini almaz (açık dünyada iddia kaydı ve niyet
  değerlendirmesi yok): işlev eksiği, kanal değil.
- Yetkili kişinin açık dünyadaki öğelerinin kimlikleri otoritedekinden farklı olabilir
  (her dünya kendi `seq`'iyle); yetkili kişi otoritenin kimliğini adlandırdığında açık
  dünya o kimliği kendi içinde arar (aynı dize, belki başka öğe ya da hiç): komutun açık
  sonucu yine yalnız komutun ve açık dünyanın fonksiyonu.
- Mühür dışında bilerek bırakılan kanal yok.

### Performans (K1b)

`capsule_tick -- 30 1000 600 3 flow bound`, aynı makine (yük ~6): `flow` düzeltme tiki olay
başına 2,44 µs (K1: 2,39; önce 2,33), kapanış tiki 81 ms (K1: 77–78; önce 88–89), bağlanma
177 ms, `Session::new` 1,40 s; doküman özeti `c0f2042a1be8204a` (otorite değişmedi). İlk
denemede düzeltme tiki 7,7 µs idi (her veri olayında bağlantı başına açık `seq` eşlemesi ve
akış beslemesi listesinin yeniden kurulması); besleme anahtarları `Runner`'da tutuluyor ve
eşleme yalnız koşul aboneliği varken hesaplanıyor. Ham dosya tutulmadı (S-D yöntemi, tek koşu
grubu).
