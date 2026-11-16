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
