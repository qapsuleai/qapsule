# RP2 / T5 denetim düzeltmeleri ve gerileme (O1, O2, D1, D3, D4, güvenlik kalıntısı, ölçüm)

Dal: `rp2/t5fix` (`rp2/int` @ `1920ce6` üstünde: T5 canlı yolu + T4 düzeltmeleri + T8
iddiaları/yansı). Denetim: `rounds/RP2/audits/T5.md`. Yazılan yerler: `crates/qp-session/`
(kaynak, `tests/t5_fixes.rs` yeni, `tests/live.rs` ve `tests/t4_fixes.rs` güncellendi),
`rounds/RP2/experiments/tick-bench/` (yalnız ölçüm düzeni), `docs/PROTOCOL.md` (§8.1,
§18.3, §18.8), `docs/CONCEPTS.md` ("Abonelik"), `fixtures/protocol/conformance/36-…` (yeni
döküm), bu dosya, `T5.md`'ye ek bölüm. `qp-schema`, `qp-source`, `qp-compute`'a dokunulmadı.
Var olan 35 dökümün hiçbiri değişmedi.

## 1. Gerileme (zorunlu): bound N=500 `all` 18 → 317 µs/tik

**Belirti.** `rp2/int` @ `1920ce6`'da `live 10000 600 5 5,500 bound all`: n500.bound.all
oturum p50 ≈ 317 µs/tik (yük/çekirdek 40,96); T5'te (`edfa674`) ≈ 18 µs. Bu dalda yeniden
üretildi: `live profile bound all 500 600` → 330 µs. Inline yolda ve `t6d`/`base`
düzenlerinde gerileme yoktu (bound `base` 9 µs).

**Neden.** T4 düzeltmesinin abone başına veri yetkisi denetimi `live::hears`
(`eval_conditions_live` içinde, etkilenen her `condition` aboneliği için her tikte).
`hears` → `DataPlane::unentitled` her koşul serisinden başlayıp hesap girdileri üzerinden
derinlik öncelikli yürür: her adımda `find_series` (dokümanın 500+ serisinde doğrusal arama),
`BTreeSet<String>` ve `String` ayırmaları, sonra sağlayıcıya `entitled` sorusu. 500 abone ×
~0,6 µs ≈ 300 µs. Yalnız bağlı seride görünür, çünkü `hears` bağlı seri yokken
(`plane.series` boş) hemen döner. T8'in iddia kancaları (`claims_after_bars`,
`claim_events`) bu yolda ölçülebilir bir pay almıyor (örnekleyicide <%1).

**Düzeltme** (`crates/qp-session/src/live.rs`, `session.rs`).

- **Yapı kuşağı ve canlı dizin.** `Session::struct_gen` yalnız bar yazısı olmayan her
  günlük girdisinde artar. `LiveIndex` bu kuşak için bir kez kurulur: seri kimliği → konum
  (`HashMap`), bağlı serilerin farklı beslemeleri, etkin niyetler ve canlı kapsüller. Her
  koşul için `CondInfo`: dayandığı bütün seriler (hesap girdileri üzerinden; "koşul → bar
  serisi dizini"), aralarındaki bağlı seriler (hazırlık, §18.3) ve beslemelerinin konumları
  (yetki). `condition` aboneliği kendi `CondInfo`'sunu kuşakla birlikte tutar.
- **Yetki değişim başına bir kez.** `Ents`: bir canlı değişimde kişi (oturumun kişileri,
  konumla) × besleme başına sağlayıcıya bir soru; bağlantının kişisi bağlantı başına bir kez
  çözülür. Sağlayıcının yetkisi bir değişim içinde sabit sayılır (barındırıcı sağlayıcısı
  dinamik olsa bile bir tik içinde); değişimler arasında önbellek yok, yani davranış
  eskisiyle aynı.
- `cond_blocked` dizinden: `all_ready` değişim başına bir kez, sonra bağlı seri listesi.
- `rests_on` (doğrusal `find_series` ile özyinelemeli) kaldırıldı; yerine `CondInfo`'daki
  dayanılan seri listesi.
- Ölçümde kalan iki sıcak nokta da giderildi (macOS `sample`): `eval_on` içinde ana çıktının
  `BTreeMap` dizge araması (`Computed::main_values`: ana çıktı anahtar sırasında ilkse — bar
  sütunlarında `c`, tek çıktılı hesaplar — karşılaştırmasız; değilse adla) ve niyet kenar
  durumunun her tikte iki dizge anahtarlı arama ile okunup yazılması (aşağıda O2).

**Çıktı aynılığı.** Değerlendirilen küme ve sıra eskisiyle aynıdır: `CondInfo.closure` tam
olarak `rests_on`'un ulaştığı seriler, `bound` tam olarak `gate`'in baktığı bağlı seriler,
`feeds` tam olarak `unentitled`'ın sorduğu beslemeler (her `Bind` serisinin bağı vardır).
Kapsül için dizinden "tutuyor" çıkanlar `invalidate_these`'e verilir; o, aynı denetimleri
yavaş yoldan yeniden yapar ve motor girdilerini aynı sırayla yazar. Kanıt: özellik testi
(artımlı ↔ baştan, bayt bayt), 36 döküm × 3 taşıma, bütün `qp-session` testleri.

**Ölçüm** (yerel, `live 10000 600 7 500 inline,bound all`, aynı makinede art arda):

| | T5 (`edfa674`) | `1920ce6` | bu dal (`24be50a`) |
|---|---:|---:|---:|
| bound N=500 `all` | 18,5 µs | ~317–330 µs | 19,3 µs (tam tablo §6) |
| inline N=500 `all` | 19,5 µs | ~21 µs | 20,7 µs |

Aynı oturumda araya sokulan karşılaştırmada (iki tur) bu dal yerelde T5 ile aynı düzeyde
(inline 20,7–21,6 / T5 20,7–20,9; bound 18,9–19,2 / T5 23,6–25,4 — T5 bound bu turda
gürültülü), WASM opt-s'de inline `all` N=500'de T5'ten ~%10–15 yavaş (35–38 / 31–32 µs),
bound'da ~%5 (34,7–34,8 / 33,1–33,3). Fark abone başına ~10 ns: koşulun yapı bilgisinin kuşak
denetimi ve yetki satırına bakış (T5'te yetki denetimi hiç yoktu). Hedefin içinde (en kötü
2,66).

## 2. O1 — kapsülü yeniden canlandıran geri alma: bayt eşitliği

**Neden.** Bir girdi kapsülü `live`'a döndürdüğünde (motorun geçersizleştirme girdisinin
geri alınması, silinen kapsülün geri gelmesi) iki yolda da değerlendirme yoktu
(`invalidations` yalnız `PutSeries`/bar/`PutFinding` görüyordu). Sonraki ilgisiz tikte
baştan yol bütün canlı kapsülleri değerlendirip motor girdisi yazıyor, artımlı yol
yazmıyordu.

**Düzeltme (seçenek a).** `Session::invalidations` girdinin `live` yaptığı kapsülleri
(`SetCapsuleState { state: Live }`, `PutCapsule` canlı) aday sayısına ekler: kapsül geri
alma anında iki yolda da değerlendirilir; koşul hâlâ doğruysa motor onu hemen yeniden
`invalidated` yapar. §8.1 buna göre yazıldı (bar yazısı ve veri olayında yalnız değişen
seriye dayananlar; kapsülü `live` yapan girdiden sonra hemen; geçersizleştirmeyi geri almak
kapsülü kalıcı canlandırmaz, yolu `update_finding`); §18.8'e eşitliğin dayanağı eklendi.
Bulgusu değişen kapsül zaten `PutFinding` ile, dayandığı seriyi yazan girdi `PutSeries` ile
değerlendiriliyordu.

**Test.**

- `tests/t5_fixes.rs::o1_capsule_made_live_again_is_evaluated_at_once`: denetimin
  yeniden üretimi (px, qx; `qx gt 40`; `undo seq 3` → girdi 4 `live`, girdi 5 motor
  `invalidated`; ilgisiz px tiki → iki yolda da yalnız `ack` + `change`), sonra geri
  almanın geri alınması, ikinci bir bulgu ve onu tutturan qx tiki; her adımda artımlı ↔
  baştan çıktılar, doküman ve günlük bayt bayt; yeniden oynatma eşit.
- `tests/live.rs` özellik testi: adımların %3'ü açık `seq`'li geri alma (üçte ikisi motorun
  geçersizleştirme girdisini hedef alır: kapsül yeniden canlanır; kalanı son 20 girdiden
  biri: geri alma, bulgu, bar yazısı), %1'i yeni kapsüllü bulgu (geçersizliği hemen
  tutabilir). Düzeltme kapatılınca iki özellik testi de düşüyor (denendi).
- Döküm 36 son adımı (üç taşımada).

## 3. O2 — niyet ve canlı kapsüllerin tik maliyeti

**Neden.** Niyetler ve kapsüller her tikte `reads_changed` → `rests_on` (doğrusal arama),
`eval_condition` (dizgeyle önbellek araması), `cond_blocked`, `f.invalidation.clone()` ile
değerlendiriliyordu; niyet kenar durumu her tikte `intent_state`'te iki dizge araması.

**Düzeltme.** `LiveIndex` niyetleri (`IntentWatch`) ve kapsülleri (`CapsuleWatch`: koşul,
`CondInfo`, sorumlu kişiler) tutar; değerlendirme koşul aboneliğindeki gibi önbellek
yuvası notuyla (`eval_memo`: kuşağı geçerliyse yuvalardan, değilse kimlikle ve not al).
Niyetin kenar durumu dizinde de tutulur (`intent_epoch`: baştan yol `intent_state`'i her
yazışında artar; o zaman dizindeki kopya geçersiz, haritadan okunur); harita yalnız değer
değişince yazılır, yani her zaman doğrudur.

**Ölçüm** (yeni düzen `ic` = `base` + 100 etkin niyet + 100 canlı kapsül, hepsi px'e
dayanır, hiç tetiklenmez; `run-live.sh` `WATCHES=t6d,base,all,ic`, iki koşu ortalaması):

| Yol | N | `base` | `ic` | 200 izleyicinin payı | Denetim (`zz_intents`, inline N=500) |
|---|---:|---:|---:|---:|---:|
| inline yerel | 500 | 10,13 | 13,21 | +3,1 µs (~15 ns/izleyici) | +173 µs (8,6 → 181,7) |
| bound yerel | 500 | 8,55 | 11,89 | +3,3 µs | — |
| inline yerel | 50 | 2,64 | 5,12 | +2,5 µs | — |
| inline WASM opt-s | 500 | 19,24 | 24,03 | +4,8 µs | — |

`ic` tablosunun tamamı §6'da. N=500'de `ic` hedefi her yerde tutuyor (en kötü 1,45);
N=5/50'de tutmuyor (çekirdek tik başına 0,4–1,7 µs, 200 koşul değerlendirmesi 2,5–5 µs):
paydada olmayan gerçek iş, `all`'ın T1 tipi paydadaki durumu gibi.

## 4. D1, D3, D4

- **D1 (ısınma).** `live` ölçümden önce ilk N ile bütün yolları ve düzenleri bir kez, 3
  tekrarla, çıktısız koşturur (`warmup = …` satırı). Oran ayrıca en iyi/en iyi ile de
  verilir (`best/best overhead/core_parse`). Sonuç: yerel inline N=5 çözmeli çekirdek iki
  koşuda 0,65 / 0,65 µs (T5'te 1,29 / 0,67); WASM release inline N=5 `t6d` 2,54 µs, `base`
  2,53 (T5: 4,36 / 2,48): dipnotlu hücre artık yönteme duyarlı değil (medyan 1,95, en iyi
  2,00).
- **D3.** `docs/CONCEPTS.md` "Abonelik" satırına `view` (görünümün okuduğu serinin barları
  değişti, `view_changed`) eklendi.
- **D4.** `apply_in_place` / `roll_back` tersin uymamasını artık yutmuyor
  (`live::undo_in_place`): hata ayıklama derlemesinde `debug_assert!`, sürüm derlemesinde
  doküman başlangıç dokümanı + kayıtlı günlükten yeniden kurulur (`Restore { initial, log }`;
  başarısız komut günlükte değildir). Bütün çağıranlar (bar komutu, öneri saklama, motor
  girdisi, yansının aynalaması) `Restore` verir. Test edilemez yol (tersler her zaman uyar);
  hata ayıklama derlemeli bütün testler `debug_assert`'e takılmadan geçiyor.

## 5. Güvenlik kalıntısı (T4 düzeltmesinin incelemesi, §18.3)

**Neden.** Bulgunun geçersizliği yalnız köken yazarının yetkisiyle değerlendiriliyordu.
(1) Yetkisiz `editor` yetkili birinin bulgusunun geçersizliğini `update_finding` ile belge
içi bir seriye çevirip (serbest) seriyi bağlatırsa koşul yazarın adına değerlendirilirdi.
(2) Bir koşulun okuduğu seriyi (ya da bir hesabın girdisini) yetkisiz biri yeniden
bağlayabilirdi.

**Seçilen kural (en basit sağlam olan).**

- *Değerlendirme:* bir bulgu geçersizliği, oturumun kişilerinden **sorumluların hepsi**
  koşulun okuduğu her bağlı beslemeye yetkiliyse değerlendirilir. Sorumlular: köken yazarı;
  geçersizliği en son yazan (`invalidation_writer`: geçersizliği değiştiren son
  `PutFinding`'in girdisinin yazarı — tersindeki eski sürümle karşılaştırılır, metin
  düzenlemesi sayılmaz); koşulun dayandığı her seriyi en son yazan (`series_writer`: son
  `PutSeries`'in girdisinin yazarı; motor girdileri, yani çapa damgası sayılmaz; başlangıç
  dokümanının serileri için kimse). Geri alma girdisinin yazarı geri alandır. Haritalar
  `committed`'da güncellenir (günlükten türetilir; oturum hep boş günlükle başlar). İki yol
  aynı kuralı kullanır (`invalidate_these`; artımlı yolun ön elemesi dizindeki `who`).
- *Yazı:* bir bulgu geçersizliğinin ya da niyet tetikleyicisinin (hesap girdileri üzerinden)
  dayandığı seriyi `put_series` ile yeniden yazan, yeni sürümün dayandığı her bağlı beslemeye
  yetkili olmalı; yoksa `no_entitlement` `/ops/i/series/source` (hesapta
  `…/source/inputs/k`). Koşulsuz seriyi rolü izin veren herkes bağlayabilir; değerlendirme
  kuralı onu sonra korur.

§18.3'e "Koşulu yeniden yönlendirmek de veri okumaktır" ve "Değerlendirme tarafı" olarak
yazıldı; açık sorudaki (b) seçeneği bununla uygulanmış oldu, (a)/(c) ve yetkili yazarın
isteyerek sızdırması açık soru olarak kaldı.

**Testler** (`tests/t5_fixes.rs`, artımlı ve baştan yolda):

| Test | Ne gösterir |
|------|-------------|
| `rewritten_invalidation_is_evaluated_under_its_writer` | Yol (1): editor owner'ın bulgusunun geçersizliğini `own gt kapanış±50` yapar, owner `own`'u THYAO'ya bağlar, sonra veri olayı: editor'ün iki denemesi (eşik altında ve üstünde) **aynı** sonucu verir (kapsül `live`): bir şey öğrenilmez. Kontrol: aynısını owner yapınca eşik altında `invalidated`. Kural kapatılınca test düşüyor (denendi). |
| `unentitled_writer_cannot_repoint_a_condition` | Yol (2): editor'ün `put_series own` bağı → `no_entitlement` `/ops/0/series/source`; hesabın girdisini px yapması → `/ops/0/series/source/inputs/0`; hesabın taban serisini bağlaması → `no_entitlement`; koşulsuz seriyi bağlaması serbest; owner bağlayabilir. |
| `binding_written_by_unentitled_principal_is_not_evaluated` | Editor'ün (koşulsuzken) bağladığı seriye owner koşul yazar: değerlendirilmez (`live`); owner bağı yeniden yazınca `invalidated`. Kural kapatılınca düşüyor. |
| `t4_fixes::y1_rebinding_does_not_open_an_oracle` (güncellendi) | Editor'ün kendi serisini bağlaması artık yazıda reddediliyor; owner bağladığında editor'ün koşulu yine değerlendirilmiyor. |

Döküm `36-entitlement-responsible-relive`: yazıda red, editor'ün yazdığı geçersizlik owner
bağladıktan sonra değerlendirilmez, owner yeniden yazınca değerlendirilir, O1 geri alması.
36/36 üç taşımada.

## 6. Son ölçüm (bu dal, `24be50a`)

`OUT=../raw/t5-fixes BEFORE=0 WATCHES=t6d,base,all,ic ./run-live.sh`; ham çıktılar
`rounds/RP2/experiments/raw/t5-fixes/` (`env.txt`: Apple M4 Max, Darwin 27.0.0, rustc 1.92.0,
Node v24.3.0). Yöntem T5 §3.1 gibi (600 tik, uzun döngü; çekirdek 15, oturum 7 tekrar
medyanı; iki koşu ortalaması) + ısınma geçişi (D1). µs/tik. "Yük/çekirdek" = (oturum −
çekirdek+çözme) / çekirdek+çözme, ✓ ≤ 3. "Önce" sütunları T5.md §3.2'de (değişmedi).

**inline**

| N | Hedef | Düzen | Oturum (döngü medyanı) | p50 | ort. | p99 | Çekirdek+çözme (tam sütun) | Çekirdek+çözme (T1 tipi) | Yük/çekirdek (tam, medyan) | Yük/çekirdek (en iyi/en iyi) | Yük/çekirdek (T1 tipi) |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 5 | yerel | `t6d` | 2,04 | 2,12 | 2,25 | 4,08 | 0,65 | 0,62 | 2,16 ✓ | 2,16 ✓ | 2,30 ✓ |
| 5 | yerel | `base` | 2,06 | 2,08 | 2,12 | 2,92 | 0,65 | 0,62 | 2,18 ✓ | 2,19 ✓ | 2,32 ✓ |
| 5 | yerel | `all` | 2,16 | 2,10 | 2,19 | 3,02 | 0,65 | 0,62 | 2,34 ✓ | 2,26 ✓ | 2,49 ✓ |
| 5 | WASM opt-s | `t6d` | 2,91 | 2,88 | 3,00 | 4,38 | 0,93 | 0,91 | 2,13 ✓ | 2,13 ✓ | 2,19 ✓ |
| 5 | WASM opt-s | `base` | 2,95 | 3,00 | 3,07 | 4,40 | 0,93 | 0,91 | 2,18 ✓ | 2,15 ✓ | 2,23 ✓ |
| 5 | WASM opt-s | `all` | 3,08 | 3,15 | 3,23 | 4,79 | 0,93 | 0,91 | 2,32 ✓ | 2,27 ✓ | 2,38 ✓ |
| 5 | WASM release | `t6d` | 2,54 | 2,48 | 2,63 | 4,08 | 0,86 | 0,85 | 1,95 ✓ | 2,00 ✓ | 2,00 ✓ |
| 5 | WASM release | `base` | 2,53 | 2,62 | 2,81 | 5,61 | 0,86 | 0,85 | 1,94 ✓ | 1,94 ✓ | 1,99 ✓ |
| 5 | WASM release | `all` | 2,71 | 2,65 | 2,74 | 4,21 | 0,86 | 0,85 | 2,14 ✓ | 2,21 ✓ | 2,20 ✓ |
| 50 | yerel | `t6d` | 2,71 | 2,67 | 2,90 | 4,28 | 1,46 | 0,86 | 0,85 ✓ | 0,88 ✓ | 2,14 ✓ |
| 50 | yerel | `base` | 2,64 | 2,71 | 2,88 | 5,06 | 1,46 | 0,86 | 0,81 ✓ | 0,78 ✓ | 2,07 ✓ |
| 50 | yerel | `all` | 3,58 | 3,42 | 3,67 | 5,71 | 1,46 | 0,86 | 1,45 ✓ | 1,43 ✓ | 3,15 ✗ |
| 50 | WASM opt-s | `t6d` | 4,30 | 4,19 | 4,81 | 17,63 | 1,69 | 1,51 | 1,54 ✓ | 1,55 ✓ | 1,84 ✓ |
| 50 | WASM opt-s | `base` | 4,35 | 4,39 | 4,66 | 8,75 | 1,69 | 1,51 | 1,57 ✓ | 1,58 ✓ | 1,88 ✓ |
| 50 | WASM opt-s | `all` | 5,63 | 5,52 | 5,72 | 7,58 | 1,69 | 1,51 | 2,33 ✓ | 2,35 ✓ | 2,73 ✓ |
| 50 | WASM release | `t6d` | 3,40 | 3,40 | 4,08 | 17,48 | 1,49 | 1,32 | 1,28 ✓ | 1,27 ✓ | 1,57 ✓ |
| 50 | WASM release | `base` | 3,41 | 3,54 | 3,71 | 5,50 | 1,49 | 1,32 | 1,29 ✓ | 1,26 ✓ | 1,58 ✓ |
| 50 | WASM release | `all` | 4,36 | 4,42 | 4,58 | 6,52 | 1,49 | 1,32 | 1,93 ✓ | 1,95 ✓ | 2,30 ✓ |
| 500 | yerel | `t6d` | 10,18 | 9,27 | 10,72 | 14,72 | 8,06 | 3,42 | 0,26 ✓ | 0,29 ✓ | 1,98 ✓ |
| 500 | yerel | `base` | 10,13 | 9,27 | 10,14 | 13,83 | 8,06 | 3,42 | 0,26 ✓ | 0,30 ✓ | 1,96 ✓ |
| 500 | yerel | `all` | 20,71 | 19,48 | 20,62 | 25,62 | 8,06 | 3,42 | 1,57 ✓ | 1,65 ✓ | 5,06 ✗ |
| 500 | WASM opt-s | `t6d` | 18,57 | 18,52 | 20,21 | 29,81 | 9,81 | 7,59 | 0,89 ✓ | 0,87 ✓ | 1,45 ✓ |
| 500 | WASM opt-s | `base` | 19,24 | 18,31 | 19,42 | 32,89 | 9,81 | 7,59 | 0,96 ✓ | 0,91 ✓ | 1,53 ✓ |
| 500 | WASM opt-s | `all` | 35,07 | 34,50 | 36,09 | 44,09 | 9,81 | 7,59 | 2,57 ✓ | 2,59 ✓ | 3,62 ✗ |
| 500 | WASM release | `t6d` | 14,16 | 13,32 | 15,04 | 21,23 | 8,70 | 6,43 | 0,63 ✓ | 0,62 ✓ | 1,20 ✓ |
| 500 | WASM release | `base` | 14,20 | 13,39 | 14,48 | 21,21 | 8,70 | 6,43 | 0,63 ✓ | 0,63 ✓ | 1,21 ✓ |
| 500 | WASM release | `all` | 27,33 | 26,12 | 27,86 | 38,84 | 8,70 | 6,43 | 2,14 ✓ | 2,11 ✓ | 3,25 ✗ |

**bound**

| N | Hedef | Düzen | Oturum (döngü medyanı) | p50 | ort. | p99 | Çekirdek+çözme (tam sütun) | Çekirdek+çözme (T1 tipi) | Yük/çekirdek (tam, medyan) | Yük/çekirdek (en iyi/en iyi) | Yük/çekirdek (T1 tipi) |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 5 | yerel | `t6d` | 0,76 | 0,71 | 0,82 | 1,10 | 0,40 | 0,37 | 0,92 ✓ | 0,98 ✓ | 1,07 ✓ |
| 5 | yerel | `base` | 0,81 | 0,77 | 0,84 | 1,54 | 0,40 | 0,37 | 1,05 ✓ | 1,06 ✓ | 1,22 ✓ |
| 5 | yerel | `all` | 0,92 | 0,83 | 0,91 | 1,62 | 0,40 | 0,37 | 1,31 ✓ | 1,37 ✓ | 1,49 ✓ |
| 5 | WASM opt-s | `t6d` | 1,33 | 1,33 | 1,40 | 1,71 | 0,67 | 0,65 | 1,01 ✓ | 0,98 ✓ | 1,06 ✓ |
| 5 | WASM opt-s | `base` | 1,41 | 1,38 | 1,50 | 3,42 | 0,67 | 0,65 | 1,11 ✓ | 1,11 ✓ | 1,16 ✓ |
| 5 | WASM opt-s | `all` | 1,60 | 1,56 | 1,69 | 3,35 | 0,67 | 0,65 | 1,40 ✓ | 1,41 ✓ | 1,46 ✓ |
| 5 | WASM release | `t6d` | 1,04 | 1,04 | 1,15 | 1,27 | 0,57 | 0,55 | 0,82 ✓ | 0,82 ✓ | 0,87 ✓ |
| 5 | WASM release | `base` | 1,09 | 1,08 | 1,17 | 2,60 | 0,57 | 0,55 | 0,91 ✓ | 0,91 ✓ | 0,97 ✓ |
| 5 | WASM release | `all` | 1,23 | 1,21 | 1,27 | 2,50 | 0,57 | 0,55 | 1,16 ✓ | 1,17 ✓ | 1,22 ✓ |
| 50 | yerel | `t6d` | 1,31 | 1,23 | 1,38 | 1,60 | 1,22 | 0,62 | 0,08 ✓ | 0,08 ✓ | 1,13 ✓ |
| 50 | yerel | `base` | 1,32 | 1,29 | 1,42 | 2,52 | 1,22 | 0,62 | 0,08 ✓ | 0,07 ✓ | 1,14 ✓ |
| 50 | yerel | `all` | 2,06 | 2,02 | 2,13 | 3,17 | 1,22 | 0,62 | 0,70 ✓ | 0,71 ✓ | 2,35 ✓ |
| 50 | WASM opt-s | `t6d` | 2,60 | 2,54 | 2,67 | 3,23 | 1,38 | 1,22 | 0,88 ✓ | 0,89 ✓ | 1,14 ✓ |
| 50 | WASM opt-s | `base` | 2,65 | 2,62 | 2,75 | 4,25 | 1,38 | 1,22 | 0,92 ✓ | 0,93 ✓ | 1,17 ✓ |
| 50 | WASM opt-s | `all` | 3,96 | 3,94 | 4,07 | 5,81 | 1,38 | 1,22 | 1,87 ✓ | 1,87 ✓ | 2,25 ✓ |
| 50 | WASM release | `t6d` | 1,99 | 1,94 | 2,08 | 2,48 | 1,19 | 1,00 | 0,68 ✓ | 0,67 ✓ | 0,99 ✓ |
| 50 | WASM release | `base` | 2,01 | 1,96 | 2,14 | 4,29 | 1,19 | 1,00 | 0,69 ✓ | 0,69 ✓ | 1,01 ✓ |
| 50 | WASM release | `all` | 3,00 | 2,92 | 3,07 | 4,92 | 1,19 | 1,00 | 1,52 ✓ | 1,55 ✓ | 1,99 ✓ |
| 500 | yerel | `t6d` | 8,71 | 7,85 | 8,76 | 11,86 | 7,94 | 3,19 | 0,10 ✓ | 0,12 ✓ | 1,73 ✓ |
| 500 | yerel | `base` | 8,55 | 7,81 | 8,46 | 10,68 | 7,94 | 3,19 | 0,08 ✓ | 0,11 ✓ | 1,68 ✓ |
| 500 | yerel | `all` | 19,27 | 18,35 | 19,06 | 23,12 | 7,94 | 3,19 | 1,43 ✓ | 1,48 ✓ | 5,03 ✗ |
| 500 | WASM opt-s | `t6d` | 16,82 | 16,57 | 17,17 | 21,84 | 9,49 | 7,23 | 0,77 ✓ | 0,78 ✓ | 1,33 ✓ |
| 500 | WASM opt-s | `base` | 17,12 | 16,66 | 17,50 | 23,31 | 9,49 | 7,23 | 0,80 ✓ | 0,79 ✓ | 1,37 ✓ |
| 500 | WASM opt-s | `all` | 34,72 | 33,62 | 34,90 | 42,00 | 9,49 | 7,23 | 2,66 ✓ | 2,64 ✓ | 3,80 ✗ |
| 500 | WASM release | `t6d` | 12,53 | 11,64 | 12,85 | 17,73 | 8,36 | 6,16 | 0,50 ✓ | 0,51 ✓ | 1,04 ✓ |
| 500 | WASM release | `base` | 12,67 | 11,93 | 13,34 | 18,25 | 8,36 | 6,16 | 0,52 ✓ | 0,52 ✓ | 1,06 ✓ |
| 500 | WASM release | `all` | 25,67 | 24,44 | 25,65 | 30,39 | 8,36 | 6,16 | 2,07 ✓ | 2,13 ✓ | 3,17 ✗ |

**Hedef: 54 durumun 54'ü** (tam sütun paydası, medyan; en iyi/en iyi ile de 54/54). En
kötü: bound N=500 WASM opt-s `all` 2,66; inline N=500 WASM opt-s `all` 2,57. T1 tipi
paydayla `all` N=500'de tutmuyor (yerel 5,06 / 5,03, WASM opt-s 3,62 / 3,80, WASM release
3,25 / 3,17) ve yerel inline N=50 `all` 3,15; diğerleri tutuyor (T5'te olduğu gibi: 500
koşul paydada olmayan iş). Her koşuda `session_eq_core true`; belge özetleri T5 ile aynı
(inline `3d554391f7003300`, `69226187c0ddd11f`, `db50769bc4152be4`). İki koşu arasında
oturum medyanlarında en büyük fark %11,0 (WASM release inline N=5 `all`).

**`ic` düzeni** (`base` + 100 niyet + 100 canlı kapsül):

**inline**

| N | Hedef | Düzen | Oturum (döngü medyanı) | p50 | ort. | p99 | Çekirdek+çözme (tam sütun) | Çekirdek+çözme (T1 tipi) | Yük/çekirdek (tam, medyan) | Yük/çekirdek (en iyi/en iyi) | Yük/çekirdek (T1 tipi) |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 5 | yerel | `ic` | 4,42 | 4,23 | 4,44 | 6,65 | 0,65 | 0,62 | 5,83 ✗ | 5,72 ✗ | 6,14 ✗ |
| 5 | WASM opt-s | `ic` | 6,94 | 6,83 | 7,03 | 8,68 | 0,93 | 0,91 | 6,47 ✗ | 6,44 ✗ | 6,61 ✗ |
| 5 | WASM release | `ic` | 5,38 | 5,23 | 5,56 | 7,29 | 0,86 | 0,85 | 5,24 ✗ | 5,51 ✗ | 5,36 ✗ |
| 50 | yerel | `ic` | 5,12 | 5,04 | 5,27 | 7,12 | 1,46 | 0,86 | 2,50 ✓ | 2,55 ✓ | 4,95 ✗ |
| 50 | WASM opt-s | `ic` | 8,71 | 8,48 | 8,77 | 10,75 | 1,69 | 1,51 | 4,16 ✗ | 4,20 ✗ | 4,76 ✗ |
| 50 | WASM release | `ic` | 6,71 | 6,31 | 6,57 | 8,64 | 1,49 | 1,32 | 3,51 ✗ | 3,39 ✗ | 4,09 ✗ |
| 500 | yerel | `ic` | 13,21 | 12,82 | 14,02 | 23,50 | 8,06 | 3,42 | 0,64 ✓ | 0,71 ✓ | 2,86 ✓ |
| 500 | WASM opt-s | `ic` | 24,03 | 23,98 | 25,50 | 31,71 | 9,81 | 7,59 | 1,45 ✓ | 1,42 ✓ | 2,17 ✓ |
| 500 | WASM release | `ic` | 18,37 | 17,14 | 18,60 | 25,65 | 8,70 | 6,43 | 1,11 ✓ | 1,05 ✓ | 1,86 ✓ |

**bound**

| N | Hedef | Düzen | Oturum (döngü medyanı) | p50 | ort. | p99 | Çekirdek+çözme (tam sütun) | Çekirdek+çözme (T1 tipi) | Yük/çekirdek (tam, medyan) | Yük/çekirdek (en iyi/en iyi) | Yük/çekirdek (T1 tipi) |
|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 5 | yerel | `ic` | 3,12 | 2,98 | 3,16 | 3,98 | 0,40 | 0,37 | 6,88 ✗ | 7,16 ✗ | 7,51 ✗ |
| 5 | WASM opt-s | `ic` | 5,46 | 5,27 | 5,54 | 7,12 | 0,67 | 0,65 | 7,20 ✗ | 7,24 ✗ | 7,41 ✗ |
| 5 | WASM release | `ic` | 4,10 | 3,85 | 4,11 | 5,88 | 0,57 | 0,55 | 6,18 ✗ | 6,10 ✗ | 6,40 ✗ |
| 50 | yerel | `ic` | 3,88 | 3,73 | 3,91 | 4,61 | 1,22 | 0,62 | 2,19 ✓ | 2,19 ✓ | 5,30 ✗ |
| 50 | WASM opt-s | `ic` | 7,16 | 6,96 | 7,29 | 8,98 | 1,38 | 1,22 | 4,18 ✗ | 4,18 ✗ | 4,88 ✗ |
| 50 | WASM release | `ic` | 5,19 | 4,90 | 5,18 | 6,64 | 1,19 | 1,00 | 3,36 ✗ | 3,38 ✗ | 4,18 ✗ |
| 500 | yerel | `ic` | 11,89 | 11,04 | 11,93 | 14,33 | 7,94 | 3,19 | 0,50 ✓ | 0,53 ✓ | 2,72 ✓ |
| 500 | WASM opt-s | `ic` | 22,78 | 22,15 | 23,49 | 30,21 | 9,49 | 7,23 | 1,40 ✓ | 1,40 ✓ | 2,15 ✓ |
| 500 | WASM release | `ic` | 16,79 | 15,52 | 16,86 | 21,58 | 8,36 | 6,16 | 1,01 ✓ | 0,99 ✓ | 1,73 ✓ |

## 7. Değişmezler ve doğrulama

- `cargo fmt`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace`
  yeşil.
- `QP_VERIFY_BROWSER=1 scripts/verify.sh`: 15/15 PASS (fmt, clippy, test, schema 26,
  fixtures, golden, views 8×2, wasm, hygiene, browser, compute, digest `7b1e99d40889f83c`
  arm64/x86_64/wasm32, protocol 61, conform "36 transcripts x inproc, serve, mcp", replica
  "native authority = native replica = wasm32 replica (lag 0, 25)").
- 35 eski döküm değişmedi; yeni 36 üç taşımada geçiyor.
- Özellik testi (artımlı ↔ baştan, çıktılar/doküman/günlük bayt bayt, değerler bit bit),
  `claims`, `replica` testleri, yeniden oynatma eşitliği yeşil.

## 8. Açık sorular ve öneriler

- **DECISIONS.md** (bu alt görevin yazma alanı dışında): "sorumlu kişiler" kuralı (§18.3) ve
  "kapsülü canlandıran girdi kapsülü hemen değerlendirir" (§8.1, T5-3'ün yerine) karar
  satırı olarak eklenmeli; T5-3 "bilinçli kabul" artık geçerli değil.
- Aynı geçersizliği aynen yeniden yazmak "en son yazanı" değiştirmez (metin karşılaştırması);
  yetkili biri yetkisizin koşulunu sahiplenmek için onu değiştirmelidir. Arkin'e: yeterli mi,
  yoksa her `update_finding` sayılmalı mı?
- Sağlayıcının yetki cevapları bir canlı değişim içinde önbellekte; değişimler arasında yok.
  Barındırıcı sağlayıcısı yetkiyi değiştirirse bir sonraki değişim yenisini görür.
- WASM opt-s'de `all` N=500 T5'ten ~%10 yavaş (abone başına ~10 ns); daha fazlası için koşul
  aboneliklerini ayrı bir düz diziye (yapısı `Sub`'dan ayrık) taşımak denenebilir.
