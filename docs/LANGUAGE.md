# Kapsül dili (R5, M17; K49, A30, A32, A33)

> Seçim okuyucu ve eleştirmen **ajanlarıyla yapılmış bir ön elemedir**. Ajan insan değildir.
> İlk iki dil gösterimde ayardan ve adres parametresiyle seçilebilir; son söz Arkin'in
> telefonundaki karşılaştırma ve insan testi v3'tür.

Kanıt: `rounds/R5/experiments/M17/RESULTS.md` (ölçüm, seçim kuralının adım adım uygulanışı,
§11 bütünleştirme), şartname `rounds/R5/experiments/harness/lang/SPEC.md`, tasarım notları
`rounds/R5/experiments/M17/<ad>/DESIGN.md`. Kavramlar `docs/CONCEPTS.md` ("Başlık cümlesi",
"Ek bilgi", "Süre ifadesi", "Geçerlilik", "Başlık yapı taşları", "Karne").

## Seçilen: `marla` — "beş sorunun sırası ve bağlam"

Kod `crates/qp-capsule/src/voice/marla.rs` (giriş `speak` / `speak_both`), sözlük
`voice/marla_glossary.rs`; kayıt `voice/mod.rs` (`VOICES`, `speak`). Gösterimin varsayılan
dili (`PRESENTED_VOICE`); **protokolün varsayılanı `r4`** (bugünkü metin: yüz tasarımının kendi
başlık şablonu ve kapsülün R3 cümlesi), yani çıplak bağlantılar, dökümler ve var olan altınlar
değişmez.

**İlke.** `NORTH_STAR.md`'deki beş soru dilin iskeletidir. Her yüzey bir (en çok iki) soruya
cevap verir, hep aynı sırada, birbirini tekrar etmeden. Her sayı bir başvuruyla söylenir:
eşiğe uzaklık (değer ve yüzde), neyin uç noktası olduğu ("son 20 günün en yüksek fiyatı"),
neyin katı olduğu ("20 günlük ortalamanın 2,5 katı"), sonucun ne zaman belli olacağı ("en geç
7 işlem günü içinde"). Süre her yerde takvimden gelen süre ifadesidir; bar sayısı yoktur.
Karne yüzde değil doğal sıklık söyler ("20 durumun 11'inde").

### Dört yüzey

| Yüzey | Soru | Nerede görünür | Sınır |
|---|---|---|---|
| **Başlık** | Ne değişti? (+ ne zaman, hâlâ geçerli mi) — eşik adıyla ve değeriyle | yüzün başlık cümlesi; açılmış kapsülün üst satırı | 80 karakter |
| **Ayrıntı** | Neden? (+ şimdi ne durumda, sonuç ne zaman belli) | açılmış kapsülün kartı (yüzün yapı taşlarından, A33) | 200 karakter, 3 cümle |
| **Karne** | Ne kadar emin? Daha önce oldu mu? — beklentinin kendisi, sayılmış | yüzün ek bilgisi; açılmış kapsülün kartı | 70 karakter |
| **Sözlük** | Bu terim ne demek? | ekrandaki terim işaretli (altı çizili); dokununca açıklama | açıklama 160 karakter, rakamsız |

Sınırlar iki dilde birlikte tutulur: en zengin biçimden başlayıp sığana kadar kısaltılır
(başlıkta 6 katman, ayrıntıda 8 bileşim, karnede 3); iki dil her zaman aynı sayıları taşır.
"Zaman" düşürmek son çaredir (531 durumda hiç gerekmedi).

Örnek (kırılım yukarı, doğdu, geçerli; sentetik veri):

- Başlık: "SYN20 son 20 günün en yüksek fiyatını (227,08) aştı · bugün, hâlâ geçerli" /
  "SYN20 rose above its 20-day high (227.08) · today, still holds"
- Ayrıntı: "O gün kapanış 229,81 oldu, eşiğin %1,2 üstünde. Sonuç en geç 10 işlem günü içinde
  belli olacak." / "That day it closed at 229.81, 1.2% above the high. The outcome is due
  within 10 trading days."
- Karne: "Benzer 20 durumun 11'inde fiyat 10 işlem günü içinde yükseldi." / "In 11 of 20
  similar cases the price rose within 10 trading days."
- Sözlük: **en yüksek fiyat** — "Sayılan dönemde görülen en yüksek fiyat; içinde bulunulan gün
  hariç. …"; **benzer durum** — "Aynı tür ve yönde, bundan önce kapanmış iddialar. Fiyat
  süresinde olağan oynaklığı kadar o yöne gittiyse tuttu sayılır; rastgele bir gün de yarı
  yarıya tutar." (karnenin taban oranı burada).

### Yüzeylere bağlantı (R5/`m17-int`)

Seçim çekirdekte, `?face=` / `?look=` ile aynı yoldan:

- **Protokol:** `style` hareketinin `voice` alanı (`r4`, `marla`, `selvi`; bilinmeyen ad
  `invalid` ve bilinen adlar). Bağlantının görünüm durumu (`ViewState.voice`) hatırlar;
  varsayılan (`r4`) yazılmaz.
- **Kapsül öğesi:** bir ses seçiliyse `capsules` (ve `capsule`, `capsules` konusu) her
  kapsüle `voice` ekler (`CapsuleVoice`: `headline`, `detail`, `ledger`, `glossary`, `face`),
  okurun dilinde ve aynı izleyici kuralıyla (yetkisize rakamsız). Otorite yazar
  (`Session::reader_voice`, `qp-session` `headparts::story_voice`): cümlenin girdisi (aynı
  hikâye, olgular, "şimdi", geçerlilik, gizleme) ve yüzün yapı taşları (`realize::parts`)
  → `qp_capsule::voice::speak`.
- **Yüz:** sahne sağlayıcısı (`DataProvider::voice`) sesi verdiğinde her yüz tasarımı başlığı
  bu kaynaktan alır (`headline` metin alanı). `vadrel` karneyi de: tam bir cümle olduğu için
  tek satırlık başlık satırına sığmaz, başlığın altında, `title` rolünde üç satırlık bir
  banda geçer; yüz o bant kadar uzar (`faces::height_for`, `vadrel::voiced_height`: `r4`
  görünüşte 188 → 259 px). `fotolay` ve `tavrik` başlığı sesten alır; karne bandı yok (açık
  sorun, aşağıda).
- **Açılmış kapsül:** kart sesin ayrıntısını (`detail` metin alanı) ve karne cümlesini (iki
  satırlık bant) yazar, kart paneli yüksekliğin beşte dördünü alır; açılmış katmanın üst
  satırı sesin başlığıdır. Ayrıntı yüzün yapı taşlarından kurulur (geçerlilik, ne zaman, son
  hareket ve sonuç yalnız `Parts`'tan), yüzle çelişemez (A33; `crates/qp-session/tests/voice.rs`).
- **Gösterim:** kabuk çekirdeği `marla` ile açar (`ShellCore::presented_look`); `?voice=r4`
  bugünkü metni aynen verir (v1 altınları `screens.mjs` bu parametreyle üretir, bayt eşit);
  Ayarlar → Dil seçimi (`choices().voice`).

### Sözlük: dokununca açılır (K49)

- Terimler sesin sözlüğündendir (deterministik, TR/EN, rakamsız, ≤ 160 karakter). Ekrandaki
  yerini çekirdek bulur (`qp_grammar::terms`): metin ve terim harf harf küçültülerek
  karşılaştırılır; terimin sözcükleri, sonra son sözcüğü bir-iki harf kısaltılmış hali
  (Türkçe ek yumuşaması: "düşük" → "düşüğünün"), sonra ilk sözcüğü (en az dört harfse) sözcük
  başında aranır; işaret sözcüğün sonuna kadar uzar. Her terimin sahnede ilk geçtiği yer
  işaretlenir.
- Sahne tarifi terimi taşır: altı çizgi katmanın kendi ilkelidir (her boyama yolu çizer) ve
  `SceneDesc.terms` her terimin kutusunu, metnini ve açıklamasını verir. Nokta sorgusu
  (`hit`) parmağın altındaki terimi `HitInfo.term` olarak bildirir.
- Dokunuş: kabuk `tap_at` gönderir; otorite terimin üstündeyse kapsülü açmaz, cevabın
  `hit.term`'ünü döner. Çekirdek açıklamayı tutar (`gloss()`), sayfa iki metni olduğu gibi
  gösterir, dokununca kapanır (`close_gloss()`). Kabuk konum hesaplamaz.
- Ses yokken (`r4`) sahnede `terms` alanı yoktur (bayt eşitlik).

### Yetkisiz okurun yüzü (A32)

Veri yetkisi olmayan okurun yüz sahnesi yoktur; kabuk çekirdeğin **metin yüzünü** gösterir
(`CapsuleVoice.face`: başlık, karne, ayrıntı; satır sonlarıyla). `marla` yetkisiz okura aynı
iskeleti tek rakam olmadan verir: dedektörün kamuya açık kuralı sözcükle ("beş günlük
ortalamanın iki katı"), zaman belirsiz süre ifadesiyle ("birkaç işlem günü önce"), geçerlilik
sözcükle, karne nitel ("az örnek", "çoğunlukla tuttu"); ayrıntı **neyin hangi yetkiyle
açıldığını** ve sunucunun hesaplayıp doğruladığını söyler:

> SYN03: kurumsal net satım ortalamanın iki katını aştı · bugün, hâlâ geçerli
> Henüz kapanmış benzer durum yok, ne sıklıkla tuttuğu bilinmiyor.
> Tutarlar lisanslı veridir: sunucu hesapladı ve doğruladı, kurumsal akış veri yetkisiyle
> görünür. Sonuç en geç on işlem günü içinde belli olacak.

Girişimsizlik: yetkisiz okura giden her satır, ses seçiliyken de lisanslı akıştan bağımsızdır
ve rakamsızdır (`crates/qp-session/tests/non_interference.rs`
`voiced_readers_are_not_interfered_with_either`; `tests/voice.rs`). Sözlükteki terimler metin
yüzünde dokununca açılmaz (sahne yok); açık sorun.

## İkinci: `selvi` — "konuşur gibi tek cümle"

Kod `crates/qp-capsule/src/voice/selvi.rs`, sözlük `voice/selvi_glossary.rs`. Gösterimde
`?voice=selvi` ya da Ayarlar → Dil ile seçilir. Başlık konuşma diliyle tek cümle ("SYN04 az önce
düşüşte: 20 saatlik ortalama (44,32) altında. Hâlâ geçerli."), zaman cümlenin içinde; ayrıntı
sayıların hikâyesi; karne ve sözlük `marla` ile aynı görevde. Eleştirmen BT'de nokta tahmini
birinci (1,685), anlama doğruluğunda `marla`'dan ayırt edilemedi; seçim kuralının 4. adımında
(kısa olan) `marla` önde (başlık+ayrıntı ort. 222,9 ↔ 235,0 karakter).

## Elenen: `tunca`

"Cevap önce, eşik hemen ardından" yapısı. Taramalarda 0, anlamada üç aday ayırt edilemedi
(nokta tahmini birinci 0,914), ama BT'de üçüncü ve seçim kuralında ilk ikiye girmedi. Ürün
ağacından çıkarıldı; kodu `r5/m17-tunca` dalında, tasarım notu `rounds/R5/experiments/M17/tunca/`.

## Ölçüm özeti (RESULTS'tan)

| | `marla` | `selvi` | `tunca` | `r4` (başvuru) | `bozuk` (düzenek sınaması) |
|---|---:|---:|---:|---:|---:|
| 13 tarama toplamı (531 × 2) | **0** | **0** | **0** | 9266 | 34006 |
| birincil anlama (haiku, Wilson %95) | 0,889 [0,856; 0,915] | 0,898 [0,866; 0,924] | 0,914 [0,883; 0,937] | 0,869 [0,834; 0,898] | 0,500 |
| eleştirmen BT log güç (haiku) | 1,409 [1,141; 1,723] | 1,685 [1,411; 2,012] | 1,345 [1,110; 1,602] | −0,750 | −3,687 |
| başlık+ayrıntı ort. karakter | **222,4** (S1 düzeltmesiyle 222,9) | 234,9 (235,0) | 236,0 | 258,5 | 443,8 |
| tavsiye okuması | 0 | 0 | 0 | 0 | 0 |

- Üç aday sert eşikleri geçti; anlamada ve eleştirmende birbirinden ayırt edilemedi; `bozuk`
  ayrık biçimde son → düzenek geçerli. Seçim kuralı 4. adıma (kısa olan) düştü: **`marla`**;
  ilk iki **`marla`, `selvi`**.
- Adaylar `r4`'e göre eşik adı/değeri sorusunda (S2) kazanır, "ne oldu" sorusunda (S1)
  kaybeder. S1'in ana nedeni "ortalama dizilimi aşağı"nın "sınırın altına indi" diye
  okunmasıydı; bütünleştirmede iki dilde en küçük düzeltme yapıldı (olay fiili önce ve düz:
  "düşüşünü sürdürdü" / "düşüşte" / "kept falling"; RESULTS §11.2), 13 tarama yine 0.
  Okuyucu ölçümü yeniden koşulmadı.
- Maliyet 3,47 $ (haiku tam koşu + sonnet alt küme). Tek model ailesi; insan testi v3 asıl
  ölçüdür.

## Yedi dedektör × yaşam döngüsü aşaması × iki dil (`marla`)

**Tam tablo:** `rounds/R5/experiments/M17/marla/TABLE.md` (`langh dump --lang marla`; 531
durum: dedektör × aşama × geçerlilik × karne durumu × yetki × periyot, her durumda başlık,
ayrıntı, karne ve sözlük, TR ve EN; 354 KB). `selvi`'nin tablosu
`rounds/R5/experiments/M17/selvi/TABLE.md`. Aşağıdaki özet her (dedektör, aşama) hücresinden
bir durumun başlığını verir (günlük, yetkili okur öncelikli); "durum" o hücredeki durum
sayısıdır. Hücresi olmayan beş bileşim (kurumsal akış ve hacim `weakened`, RSI aşırılığı ve
sıkışma `confirmed`, sıkışma `expired`) düzeneğin durum kümesinde yoktur.

| dedektör | aşama | durum | TR başlık | EN başlık |
|---|---|---:|---|---|
| `flow.institutional` | born | 24 | SYN03: kurumsal net satım ortalamanın 2 katını aştı · bugün, hâlâ geçerli | SYN03: institutional net selling topped 2 times its average · today, still holds |
| `flow.institutional` | strengthened | 22 | SYN13: kurumsal satım ortalamayı 2 kat aştı · 2 işlem günü önce, hâlâ geçerli | SYN13: institutions sold over 2x average · 2 trading days ago, still holds |
| `flow.institutional` | confirmed | 14 | SYN04: kurumsal net alım ortalamanın 2 katını aştı · bugün, hâlâ geçerli | SYN04: institutional net buying topped 2 times its average · today, still holds |
| `flow.institutional` | realized | 22 | SYN02: kurumsal net satım ortalamanın 2 katını aştı · bugün gerçekleşti | SYN02: institutional net selling topped 2 times its average · played out today |
| `flow.institutional` | invalidated | 22 | SYN09: kurumsal satım ortalamayı 2 kat aştı · bugün tutmadı | SYN09: institutions sold over 2x average · did not play out today |
| `flow.institutional` | expired | 24 | SYN03: kurumsal net satım ortalamanın 2 katını aştı · bugün süresi doldu | SYN03: institutions sold over 2x average · ran out of time today |
| `flow.institutional` | retracted | 2 | SYN02: kurumsal net satım ortalamanın 2 katını aştı · bugün geri çekildi | SYN02: institutional net selling topped 2 times its average · withdrawn today |
| `extreme.rsi` | born | 12 | SYN07 kısa sürede çok düştü: RSI 30'un altına indi · bugün, hâlâ geçerli | SYN07 fell a lot in a short time: RSI went below 30 · today, still holds |
| `extreme.rsi` | strengthened | 8 | SYN24 hızla düştü: RSI 30'un altına indi · 5 işlem günü önce, artık sürmüyor | SYN24 fell fast: RSI went below 30 · 5 trading days ago, no longer holds |
| `extreme.rsi` | weakened | 8 | SYN29 kısa sürede çok yükseldi: RSI 70'i aştı · cuma günü, artık geçerli değil | SYN29 rose a lot in a short time: RSI went above 70 · on Friday, no longer holds |
| `extreme.rsi` | realized | 11 | SYN25 kısa sürede çok yükseldi: RSI 70'i aştı · bugün gerçekleşti | SYN25 rose a lot in a short time: RSI went above 70 · played out today |
| `extreme.rsi` | invalidated | 9 | SYN07 kısa sürede çok düştü: RSI 30'un altına indi · bugün tutmadı | SYN07 fell a lot in a short time: RSI went below 30 · did not play out today |
| `extreme.rsi` | expired | 8 | SYN05 kısa sürede çok yükseldi: RSI 70'i aştı · bugün süresi doldu | SYN05 rose a lot in a short time: RSI went above 70 · ran out of time today |
| `extreme.rsi` | retracted | 2 | SYN25 kısa sürede çok yükseldi: RSI 70'i aştı · bugün geri çekildi | SYN25 rose a lot in a short time: RSI went above 70 · withdrawn today |
| `breakout.range` | born | 19 | SYN28 son 20 günün en yüksek fiyatını (82,99) aştı · bugün, hâlâ geçerli | SYN28 rose above its 20-day high (82.99) · today, still holds |
| `breakout.range` | strengthened | 20 | SYN27 son 20 günün en yüksek fiyatını (77,09) aştı · dün, hâlâ geçerli | SYN27 rose above its 20-day high (77.09) · yesterday, still holds |
| `breakout.range` | confirmed | 21 | SYN02 son 20 günün en yüksek fiyatını (137,22) aştı · bugün, hâlâ geçerli | SYN02 rose above its 20-day high (137.22) · today, still holds |
| `breakout.range` | weakened | 16 | SYN21 son 20 günün en yüksek fiyatını (200,43) aştı · dün, artık geçerli değil | SYN21 rose above its 20-day high (200.43) · yesterday, no longer holds |
| `breakout.range` | realized | 12 | SYN28 son 20 günün en yüksek fiyatını (82,99) aştı · bugün gerçekleşti | SYN28 rose above its 20-day high (82.99) · played out today |
| `breakout.range` | invalidated | 11 | SYN02 son 20 günün en yüksek fiyatını (137,22) aştı · bugün tutmadı | SYN02 rose above its 20-day high (137.22) · did not play out today |
| `breakout.range` | expired | 7 | SYN24 son 20 günün en düşük fiyatının (7,91) altına indi · bugün süresi doldu | SYN24 fell below its 20-day low (7.91) · ran out of time today |
| `breakout.range` | retracted | 2 | SYN02 son 20 günün en yüksek fiyatını (137,22) aştı · bugün geri çekildi | SYN02 rose above its 20-day high (137.22) · withdrawn today |
| `flow.volume_surge` | born | 16 | SYN11: hacim ortalamanın 2,5 katını aştı, fiyat düştü · bugün, hâlâ geçerli | SYN11: volume over 2.5x average, price fell · today, still holds |
| `flow.volume_surge` | strengthened | 4 | SYN04: hacim ortalamayı 2,5 kat aştı, fiyat yükseldi · bir saat önce, sürüyor | SYN04: volume over 2.5x average, price rose · an hour ago, still holds |
| `flow.volume_surge` | confirmed | 5 | SYN21: hacim ortalamayı 2,5 kat aştı, fiyat yükseldi · geçen hafta, hâlâ geçerli | SYN21: volume over 2.5x average, price rose · last week, still holds |
| `flow.volume_surge` | realized | 12 | SYN05: hacim ortalamanın 2,5 katını aştı, fiyat yükseldi · bugün gerçekleşti | SYN05: volume topped 2.5 times its average as the price rose · played out today |
| `flow.volume_surge` | invalidated | 10 | SYN11: hacim ortalamayı 2,5 kat aştı, fiyat düştü · bugün tutmadı | SYN11: volume over 2.5x average, price fell · did not play out today |
| `flow.volume_surge` | expired | 3 | SYN21: hacim ortalamanın 2,5 katını aştı, fiyat yükseldi · bugün süresi doldu | SYN21: volume over 2.5x average, price rose · ran out of time today |
| `flow.volume_surge` | retracted | 3 | SYN11: hacim ortalamanın 2,5 katını aştı, fiyat düştü · bugün geri çekildi | SYN11: volume topped 2.5 times its average as the price fell · withdrawn today |
| `reversal.rsi_divergence` | born | 12 | SYN26 yukarı dönebilir: fiyat yeni dip yaptı, RSI yapmadı · bugün, hâlâ geçerli | SYN26 could turn up: new price low, not matched by RSI · today, still holds |
| `reversal.rsi_divergence` | strengthened | 5 | SYN09 aşağı dönebilir: RSI yeni tepe yapmadı · 8 işlem günü önce, hâlâ geçerli | SYN09 could turn down: RSI made no new high · 8 trading days ago, still holds |
| `reversal.rsi_divergence` | confirmed | 7 | SYN01 aşağı dönebilir: fiyat yeni tepe yaptı, RSI yapmadı · dün, hâlâ geçerli | SYN01 could turn down: RSI made no new high · yesterday, still holds |
| `reversal.rsi_divergence` | weakened | 8 | SYN29 aşağı dönebilir: RSI yeni tepe yapmadı · cuma günü, hâlâ geçerli | SYN29 could turn down: RSI made no new high · on Friday, still holds |
| `reversal.rsi_divergence` | realized | 10 | SYN26 yukarı dönebilir: fiyat yeni dip yaptı, RSI yapmadı · bugün gerçekleşti | SYN26 could turn up: new price low, not matched by RSI · played out today |
| `reversal.rsi_divergence` | invalidated | 10 | SYN25 aşağı dönebilir: fiyat yeni tepe yaptı, RSI yapmadı · bugün tutmadı | SYN25 could turn down: RSI made no new high · did not play out today |
| `reversal.rsi_divergence` | expired | 6 | SYN19 yukarı dönebilir: fiyat yeni dip yaptı, RSI yapmadı · bugün süresi doldu | SYN19 could turn up: new price low, not matched by RSI · ran out of time today |
| `reversal.rsi_divergence` | retracted | 2 | SYN26 yukarı dönebilir: fiyat yeni dip yaptı, RSI yapmadı · bugün geri çekildi | SYN26 could turn up: new price low, not matched by RSI · withdrawn today |
| `continuation.ma_stack` | born | 12 | SYN03 düşüşünü sürdürdü, 20 günlük ortalamanın altında · bugün, hâlâ geçerli | SYN03 kept falling, below its 20-day average · today, still holds |
| `continuation.ma_stack` | strengthened | 14 | SYN21 düşüşte: 20 günlük ortalama altı · 3 işlem günü önce, artık geçerli değil | SYN21 kept falling below 20-day average · 3 trading days ago, no longer holds |
| `continuation.ma_stack` | confirmed | 19 | SYN04 düşüşünü sürdürdü, 20 günlük ortalamanın altında · bugün, hâlâ geçerli | SYN04 kept falling, below its 20-day average · today, still holds |
| `continuation.ma_stack` | weakened | 20 | SYN16 düşüşünü sürdürdü, 20 günlük ortalamanın altında · bugün, hâlâ geçerli | SYN16 kept falling, below its 20-day average · today, still holds |
| `continuation.ma_stack` | realized | 12 | SYN06 düşüşünü sürdürdü, 20 günlük ortalamanın altında · bugün gerçekleşti | SYN06 kept falling, below its 20-day average · played out today |
| `continuation.ma_stack` | invalidated | 12 | SYN15 yükselişini sürdürdü, 20 günlük ortalamanın üstünde · bugün tutmadı | SYN15 kept rising, above its 20-day average · did not play out today |
| `continuation.ma_stack` | expired | 6 | SYN04 düşüşünü sürdürdü, 20 günlük ortalamanın altında · bugün süresi doldu | SYN04 kept falling, below its 20-day average · ran out of time today |
| `continuation.ma_stack` | retracted | 2 | SYN06 düşüşünü sürdürdü, 20 günlük ortalamanın altında · bugün geri çekildi | SYN06 kept falling, below its 20-day average · withdrawn today |
| `squeeze.bandwidth` | born | 6 | SYN07 sıkıştı: 20 günlük bant 120 günün en darı · bugün, hâlâ geçerli | SYN07 went quiet: 20-day band narrowest in 120 days · today, still holds |
| `squeeze.bandwidth` | strengthened | 1 | SYN23: 20 günlük bant 120 günün en darı · 18 işlem günü önce, artık sürmüyor | SYN23: 20-day band at a 120-day low · 18 trading days ago, no longer holds |
| `squeeze.bandwidth` | weakened | 5 | SYN30: 20 günlük bant 120 günün en darı · 7 işlem günü önce, artık geçerli değil | SYN30: 20-day band at a 120-day low · 7 trading days ago, no longer holds |
| `squeeze.bandwidth` | realized | 6 | SYN30 sıkıştı: 20 günlük bant 120 günün en darı · bugün gerçekleşti | SYN30 went quiet: 20-day band narrowest in 120 days · played out today |
| `squeeze.bandwidth` | invalidated | 6 | SYN07 sıkıştı: 20 günlük bant 120 günün en darı · bugün tutmadı | SYN07 went quiet: 20-day band narrowest in 120 days · did not play out today |
| `squeeze.bandwidth` | retracted | 1 | SYN30 sıkıştı: 20 günlük bant 120 günün en darı · bugün geri çekildi | SYN30 went quiet: 20-day band narrowest in 120 days · withdrawn today |

Yetkisiz okur (tek lisanslı dedektör kurumsal akıştır; öteki altısı kamuya açık veriden
hesaplanır, yetkisiz okur onları da yetkili gibi görür):

| dedektör | TR (yetkisiz) | EN (unentitled) |
|---|---|---|
| `flow.institutional` | SYN03: kurumsal net satım ortalamanın iki katını aştı · bugün, hâlâ geçerli / Henüz kapanmış benzer durum yok, ne sıklıkla tuttuğu bilinmiyor. / Tutarlar lisanslı veridir: sunucu hesapladı ve doğruladı, kurumsal akış veri yetkisiyle görünür. Sonuç en geç on işlem günü içinde belli olacak. | SYN03: institutions sold twice the usual · today, still holds / No similar closed cases yet, so no hit rate. / The amounts are licensed, computed and verified by the server, shown with institutional flow access. The outcome is due within ten trading days. |

## Doğrulama

- 13 deterministik tarama (`langh dump --lang marla|selvi`): hepsi 0 (S1 düzeltmesinden sonra
  yeniden koşuldu; RESULTS §11.2).
- `cargo test -p qp-capsule voice` (her olay × aşama × dil kuralları, sözlük rakamsız ve kısa,
  belirleyicilik), `qp-grammar` `terms` (terim bulma), `qp-session` `tests/voice.rs` (seçim,
  dört yüzey, sınırlar, yetkisiz rakamsız, A33 geçerlilik uyumu, terim işareti ve dokunuş),
  `tests/non_interference.rs` (sesli okurlar dahil girişimsizlik).
- `scripts/verify.sh` (tarayıcı adımı dahil): var olan altınlar `?voice=r4` ile bayt eşit.

## Açık sorunlar

1. **`fotolay` ve `tavrik` yüzlerinde karne bandı yok.** Ses seçiliyken bu iki yüz başlığı
   sesten alır ama karne cümlesine yer ayırmaz (eski şablonları karneyi başlığın içine
   gömüyordu). Gösterimin yüzü `vadrel`'dir; M16/M20 seçiminde bu iki yüz ya band alır ya
   da galeriden düşer.
2. **Metin yüzünde sözlük dokunuşu yok** (yetkisiz okur, A32): sahne olmadığı için terim
   işaretlenmez. Çekirdek metni terim parçalarıyla verebilir (kabuk yalnız boyar); öneri.
3. **Terim iki satıra bölünürse** yalnız ilk sözcükleri (en az dört harfse) işaretlenir.
4. **S1 düzeltmesinin etkisi ölçülmedi** (okuyucu ölçümü koşulmadı).
5. **Yeni dille altın yok:** `marla` görünümünün `v2` altınlarını bütünleştirme (S) ekler.
