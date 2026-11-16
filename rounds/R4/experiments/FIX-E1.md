# R4 — Evre 1 denetim bulgularının düzeltmesi (FIX-E1: B1, B2)

Dal `r4/fix-e1` (taban `r4/int` b9331cd). B3 açık karar olarak bırakıldı (dokunulmadı).

## B1 — XIST takvimi: 2018–2027 tam ve yarım gün kapanışları

### Ne yapıldı

- `qp_calendar::builtin("XIST")` artık `XIST_HOLIDAYS` (107 hafta içi tam kapanış) ve
  `XIST_HALF_DAYS` (22 hafta içi yarım gün, kapanış `XIST_HALF_DAY_CLOSE` = 12:30) taşıyor,
  2018-01-01 → 2027-10-29 (`crates/qp-calendar/src/lib.rs`, yıl yıl yorumlu). Hafta sonuna denk
  gelen tatiller listede yok (hafta sonu zaten kapalı). `fixtures/calendar/xist.json`
  `dump_builtin` ile yeniden üretildi (test, dosyanın yerleşik takvime eşit olmasını istiyor).
- Kaynak: sabit tatiller 2429 sayılı Ulusal Bayram ve Genel Tatiller Kanunu (1 Ocak, 23 Nisan,
  1 Mayıs, 19 Mayıs, 15 Temmuz, 30 Ağustos, 29 Ekim; 28 Ekim ve arifeler yarım gün); dini
  bayramlar Diyanet takvimi (Ramazan 3 gün, Kurban 4 gün); olağanüstü kapanış: 8–14 Şubat 2023
  (6 Şubat depremleri sonrası piyasa kapandı, 8 Şubat işlemleri iptal edildi). Borsaya özgü
  köprü günü kapanışı bu aralıkta yok: kamu için ilan edilen köprü/idari izin günleri
  (ör. 3 Haziran 2019, 8–9 Nisan 2024) gerçek veride işlem günü. Bu liste gerçek veriyle birebir
  doğrulandı (aşağıda).
- **2026-11-17 sonrası ve 2027:** resmî tatil takvimi + Diyanet 2027 tarihleri (Ramazan Bayramı
  9–11 Mart, arife 8 Mart pazartesi yarım gün; Kurban Bayramı 16–19 Mayıs, arife 15 Mayıs
  cumartesi; 19 Mayıs bayramın 4. günüyle çakışıyor). **Belirsizlik:** Borsa İstanbul 2027
  takvimini genellikle aralıkta ilan ediyor; olası bir köprü günü ya da farklı yarım gün saati
  henüz bilinmiyor. Kodda "provisional" diye işaretli.
- **2024-11 öncesi yarım günler** günlük veriden görülemez; aynı kural (hafta içi arife ve
  28 Ekim) uygulandı. Kapanış saati 12:30 bütün yıllar için varsayıldı (sürekli işlem; kapanış
  seansı ayrı modellenmiyor, K14/R1 kararıyla aynı).

### Doğrulama (yerel gerçek veri; yalnız sayılar)

Betik: `rounds/R4/experiments/fix-e1/check_xist.py` (veri yolu `QAPSULE_LOCAL_BARS` ortamdan,
yalnız sayı ve tarih basar):

```
QAPSULE_LOCAL_BARS=/Users/arkns/Works/Mchart/qapsule/fixtures/local/bars \
  python3 -I rounds/R4/experiments/fix-e1/check_xist.py fixtures/calendar/xist.json
```

| Ölçü | Sonuç |
|---|---|
| Günlük dosya | 31 sembol, 2018-01-02 → 2026-11-16, 2315 hafta içi gün; hafta sonu barı olan gün 0 |
| Tam kapanış (hiçbir sembolde bar yok) | veri **95**, takvim **95**, yalnız veride 0, yalnız takvimde 0 |
| Yarım gün (saatlik, 2024-11-15 → 2026-11-13; o günün son saatlik barı 12:00) | veri **5**, takvim **5**, fark 0 |
| Atlanan gün | 2026-11-16: veri günün ortasında alınmış (25/31 sembol, son 5 dk bar 12:05); yarım gün sayılmadı |

Takvim toplamı: 107 tam kapanış, 22 yarım gün (2018–2027). Not: 5 dakikalık veride yarım günün
son barı 12:30'da açılıyor (normal günde son bar 17:55; 18:00 kapanış seansı barı birleştirilmiş,
yarım günün 12:30 kapanış seansı barı birleştirilmemiş görünüyor). Takvim modeli sürekli işlemi
10:00–12:30 sayıyor; bu bar seans dışı kalır. Kapsam dışı, Öneriler'de.

### Etki analizi ve seçilen yol

Takvim üç yerde kullanılıyordu: (1) sentetik pazarın tarihleri (`qp_source::regimes`,
2023-01-02'den itibaren işlem günleri), (2) yeniden oynatmanın OHLC yolundaki seans süresi ve
gün gruplaması (`qp_source::replay`), (3) `qp-capsule` süre ifadesi testleri.

**Ölçüm:** yalnız takvim verisi değiştirildiğinde `cargo test --workspace` 6 testte kırıldı:
resmî dökümler (süreç içi, `qp serve`, MCP), `capsule_perf_equality` (iki özet). Neden:
sentetik pazar 2023'ten başladığı için gerçek 2023–2026 tatilleri sentetik işlem günlerini
kaydırıyor; bütün sentetik barların `t` değeri, dolayısıyla dökümler, görünüm altınları ve
özetler değişirdi.

**Seçilen yol: sentetik pazar kendi sabit takvimini kullanır.** `qp_source::synthetic_calendar()`
= XIST seansları, hafta sonu ve saat dilimi + R1'deki beş örnek tatil ve bir yarım gün
(`SYNTHETIC_HOLIDAYS`, `SYNTHETIC_HALF_DAYS`; değerler eski yerleşik takvimle bayt bayt aynı).
Gerekçe: sentetik pazar BIST değil, yalnız tarihlerinin sabit kalması gerekir; takvim sürümü
eklemek (ör. `XIST@r1`) şemaya ve takvim kimliğine yeni bir kavram sokardı, oysa tek kullanıcı
sentetik üreteç. Yeniden oynatmada takvim seçimi `ReplayOptions::calendar`
(`ReplayCalendar::Xist` varsayılan / `ReplayCalendar::Synthetic`); `ReplayHost` kaynak türüne göre
seçiyor (yerel gerçek veri → `Xist`, sentetik pazar → `Synthetic`); `synthetic_replay_feeds`
gün gruplamasında sentetik takvimi kullanıyor (yalnız saat dilimi; sonuç aynı).

**Sonuç: B1 hiçbir dökümü, altını, görünüm altınını, hesap özetini ya da sentetik veriyi
değiştirmiyor** (B1 commit'inde `cargo test --workspace` yeşil, döküm dosyasına dokunulmadı;
`--bin validity` sayımları denetçininkiyle aynı: closed 2642 / no_longer 42 / valid 132).
Değişen tek veri dosyası `fixtures/calendar/xist.json` (yerleşik takvimin dökümü; gerekçe:
takvim verisinin kendisi).

Davranış değişikliği (yalnız gerçek veride): yerel veriyle yeniden oynatmada ince bar olmayan
günlerin OHLC yolu yarım günlerde 8 saat yerine 2,5 saatlik seansa yayılıyor (doğru olan bu).
Tatillerde bar olmadığı için başka fark yok.

### Testler

- `qp-capsule` `realize::elapsed::tests::real_xist_holidays`: 14 gerçek kapanış örneği (2026 Kurban
  + arife, 15 Temmuz 2026, 2025'in 1 Mayıs / Ramazan / Kurban / 28–29 Ekim / 15 Temmuz'u, 2019
  Kurban, 2023 deprem kapanışı, 2027 Ramazan arifesi saatlik) ve yarım günün 12:30 kapanışı.
- `qp-calendar` `xist_holiday_data_is_well_formed`: listeler artan, tekil, yalnız hafta içi,
  2018–2027; tatil ile yarım gün çakışmıyor; 107/22; her türden örnek; köprü günü işlem günü.
- `qp-source` `synthetic_calendar_is_the_frozen_r1_sample`: sentetik takvim R1 örneği, gerçek
  takvimden ayrı (2023-04-21 sentetikte işlem günü, gerçekte tatil).
- Denetçi sondası `--bin elapsed`: **23/24 PASS** (önce 13/24). Kalan tek fark takvim değil:
  `60m 2026-11-16 14:00 → 15:00` için sonda TR `1 saat önce` bekliyor, işlev `bir saat önce`
  diyor (EN `an hour ago`; sondanın EN beklentisi de `an hour ago`). `bir saat önce` T3'ün bilinçli
  seçimi (`elapsed.rs` testinde açıkça var) ve EN karşılığıyla tutarlı; denetçinin tablosunda da
  yanlış olarak sayılmamıştı (13 doğru + 10 takvim yanlışı = 23). Ürün metnini sondaya uydurmak
  için değiştirmedim; sondanın beklentisi `bir saat önce` olmalı. Orkestratörün kararı.

## B2 — Kapanmış kapsülün cümlesi "hâlâ geçerli" demiyor

### Ne yapıldı

`qp_capsule::realize`: `Plan::closed()` (aşama kapanış aşaması: gerçekleşti, geçersiz kaldı,
süresi doldu, geri çekildi). TR/EN `tail` (geçerlilik yan cümlesi) yalnız **açık** hikâyede
yazılıyor; kapanmış hikâyede geçerlilik yan cümlesi aşama notunun kendisi ve kapanışın türünü
söylüyor ("Beklenen sonuç … gerçekleşti" / "came true", "geçersiz oldu" / "was invalidated",
"ömrü … sonuçsuz doldu" / "life ran out", "geri çekildi" / "was withdrawn"). Önceden yalnız
`invalidated/expired/retracted` için böyleydi; `realized` hikâyede durum koşuluna göre
"Hâlâ geçerli", "Artık geçerli değil" ya da "… bilinmiyor" ekleniyordu. Yeni kavram yok
(`docs/CONCEPTS.md` "Geçerlilik" satırı zaten `closed` = "gerçekleşti, tutmadı, süresi doldu,
geri çekildi" diyor).

### Değişen dosyalar ve satırlar (hepsi K42: işaret `closed` ↔ cümle kapanışı söyler)

Her yaprak farkı (`git diff` + yaprak karşılaştırma): metinlerde **yalnız silme**, eklenen metin
yok; tek sayısal fark başlık kutusunun yüksekliği (cümle bir satır kısaldı).

| Dosya | Değişen yapraklar | Neden |
|---|---|---|
| `fixtures/protocol/conformance/38-capsule-lifecycle.json` | adım 9 (`event.capsule.sentence` tr/en/text), 10 (`capsules[0].sentence`), 11 (`detail.item.sentence`): `Hâlâ geçerli. ` / `It still holds. ` silindi (9 yaprak) | `cap.THYAO.breakout.down.124` realized/closed |
| `41-capsule-sensitivity.json` | adım 11 (bildirim), 12 (liste): aynı silme (6 yaprak) | aynı kapsül |
| `42-capsule-candidate.json` | adım 6 `capsules[0].sentence`: `Hâlâ geçerli olup olmadığı bilinmiyor. ` / `Whether it still holds is unknown. ` silindi (3) | realized akış kapsülü |
| `44-capsule-selectors-bound.json` | adım 16 günlükte saklanan `headline` (`changes[1]`, `inverse[0]`): `Hâlâ geçerli. ` silindi (2) | aynı kırılım kapsülü |
| `48-capsule-selector.json` | adım 36 `capsules[0].sentence`: `Artık geçerli değil. ` / `It no longer holds. ` silindi (3) | realized kapsül; "artık geçerli değil" `no_longer` işaretinin sözü, `closed` ile çelişikti |
| `51-input-tap-hit.json` | adım 3 `presentation.text` silme; `shared[4]` (`headline`) `after.bottom` 130 → 114 (2) | sunum metni = cümle; başlık kutusu bir satır kısa |
| `52-input-pan-pinch.json` | adım 4 `presentation.text` silme (1) | aynı |
| `53-input-open-close.json` | adım 2–5 `presentation.text` silme (4); adım 2 `after.bottom`, adım 5 `before.bottom` 130 → 114 (2) | aynı |
| `54-input-evidence.json` | adım 2, 4, 5 `presentation.text` silme (3) | aynı |
| `55-input-presence.json` | adım 5, 7, 10, 12 `presentation.text` silme (4); `headline` kutusu `after/before.bottom` 130 → 114 (4) | aynı |
| `crates/qp-capsule/tests/fixtures/m2_sentences.json` | `product_changes`: s03, s58 eklendi; s66, s67 güncellendi (yalnız yan cümle silindi); `note` | R3 ölçülen şablonla eşitlik testi belgelenmiş farkları burada tutuyor (20 → 22) |
| `crates/qp-session/tests/capsule_perf_equality.rs` | `bound` (satır özeti, `a`, `i` bağlantıları) ve `inline` özetleri yeniden kaydedildi; eski T3 değerleri yorumda; satır sayıları aynı (2996, 3739), baytlar azaldı | kapanmış cümlelerin kısalması |

Değişmeyenler: dökümler 01–37 ve 39, 40, 43, 45–47, 49, 50; `fixtures/golden/**` (görünüm
altınları dahil; `views` adımı yeşil), `fixtures/views/*` ve `explain` dosyaları,
`fixtures/compute/digest.txt`, sentetik veri.

### Test ve sonda

- `crates/qp-session/tests/capsule_k42.rs` `closed_sentences_say_how_the_story_ended`: 100 bar
  boyunca her bar, her kapanmış öğe, TR ve EN: cümle "hâlâ geçerli / hâlâ açık / bugünkü durumu
  bilinmiyor / still holds / still open" içermez ve aşamaya göre kapanış sözünü içerir;
  realized ve invalidated her iki dilde görülmüş olmalı. Eski kodla denendi: düşüyor
  (`cap.THYAO.reversal.down.51 realized … Hâlâ geçerli olup olmadığı bilinmiyor`).
- `--bin validity`: `closed(realized)+sentence says still valid` satırı **yok** (önce 594);
  uyuşmazlık 0; sayımlar denetçininkiyle aynı (closed 2642, no_longer 42, valid 132,
  `valid+sentence says unknown` 34 = B3, açık).
- `--bin interact` E satırları: realized/closed kapsülün cümlesi ve sunum metni
  "… Beklenen sonuç bu barda gerçekleşti. Karnesi henüz az örnekli: 6 seferin 3'ünde tuttu."

## Doğrulama

`QP_VERIFY_BROWSER=1 scripts/verify.sh` (8036d98 üzerinde): **16/16 PASS, çıkış 0** — fmt, clippy,
test, schema, fixtures, golden, views, wasm, hygiene, browser, compute, digest (arm64, x86_64,
wasm32: `7b1e99d40889f83c`, değişmedi), protocol, conform (55 döküm × inproc, serve, MCP),
replica, replay. `git ls-files | grep fixtures/local`: boş.

## Önerilen karar

- **FIX-E1-1:** Sentetik pazar kendi sabit takvimini kullanır (`qp_source::synthetic_calendar`,
  R1'in örnek tatilleri); gerçek borsa takvimi (`builtin("XIST")`) sentetik veriyi etkilemez.
  Kanıt: yalnız takvim verisi değiştiğinde 6 test kırıldı, ayırınca hiçbir döküm/altın/özet
  değişmedi.
- **FIX-E1-2:** Kapanmış hikâyenin cümlesinde ayrı geçerlilik yan cümlesi yok; kapanış aşama
  notu geçerliliği söyler (K42). `parts::validity_phrase` (M8 başlık yapı taşı) zaten bu
  biçimde.

## Açık sorular

- 2027 takvimi geçici; Borsa İstanbul ilanından sonra (aralık) kontrol edilmeli.
- 2024-11 öncesi yarım gün saatleri gerçek veriyle doğrulanamadı (günlük veri); kural varsayıldı.
- Sonda `--bin elapsed` 24. örneğinin beklentisi (`1 saat önce` / `bir saat önce`).

## Öneriler (kapsam dışı)

- Kapanmış `invalidated`/`expired`/`retracted` notları hâlâ "; artık geçerli değil / it is no
  longer valid" ekliyor. Çelişik değil (kapanmış iddia geçerli değil), ama `no_longer` işaretinin
  sözünü kullanıyor; M8 başlığında kapanmış kapsül için yalnız kapanış türü söylenebilir.
- Yarım günün 12:30 kapanış seansı barı (5 dk veride) takvim modelinde seans dışı; kapanış
  seansını modellemek ya da veri çekiminde birleştirmek (`closing_auction_bars_merged`) düşünülmeli.
- `XNYS` hâlâ R1 örnek tatilleri taşıyor; NYSE verisi gelirse aynı yöntemle tamamlanmalı.
- `rounds/R3/experiments/S/crosscheck` (m2 fikstürünü yeniden üreten araç) koşulmadı.
  `product_changes`'ı doğrudan ürün realizer'ından ürettiği için aynı dört kaydı verir; yalnız
  `note` metni araçta sabit (R3 klasörü değiştirilmedi), yeniden üretilirse FIX-E1 cümlesi düşer.
