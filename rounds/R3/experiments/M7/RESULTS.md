# R3/M7 — İtiraz ve geri çekme (A20 → K36)

Tarih: 2026-11-15. Dal: `r3/m7-dispute` (`r3/int` @ a3a216d'den). Şartname:
`rounds/R3/experiments/harness/SPEC.md` §M7; karar K36 ("A20: ikisi birden; otorite
kesinleştirmeden önce kesin barla doğrular ve yansıdan itiraz yolu açılır"), K37
(`retracted`). Protokol: `docs/PROTOCOL.md` §19.9 (yeni), döküm
`fixtures/protocol/conformance/37-dispute-retract.json`.

## 1. Özet

- **Sorun (RP2 son denetimi O1):** otorite kendi beslemesinden yanlış bir **kesin** iddia
  çıkarırsa yansı onu doğru biçimde `disputed` gösteriyor ama otorite geri çekmiyor; `bs`
  AUD1/AUD2/AUD3'te 7/49/7 kesin iddiayı kaçırıyordu.
- **Tasarım (K36, ikisi birden):** (a) otoritede **kesin bar hijyeni** — her kapanan bar ve
  beslemenin her kapalı-bar düzeltmesi barındırıcıdan kesin kaynakla doğrulanır; farklıysa
  otorite verisini düzeltir, iddiaları yeniden sınar (`data_corrected`) ve kesin verinin
  tuttuğu ama kayıtlı olmayan koşul iddialarını ileri sürer; (b) yansıdan **`dispute`** —
  otorite yansının değerlerine değil kesin veriye bakar: haklı → `withdrawn`
  (`dispute_upheld`) + kapsül `retracted` (K37), haksız → kanıtı itiraz edene
  (`dispute_rejected`). Kesinlik doğrulamayı **beklemez** (gecikme maliyeti yok).
- **Ölçüm (gerçek otorite + gerçek yansı, `bs2`):** M1 = M2 = M3 = 0 bütün senaryolarda
  (S0–S19 + ekler, AUD1–AUD3, M7X); M5 AUD1/AUD3 **0/0** (`bs` 7/7), AUD2 53 (`bs` 49; yansının
  REFRESH'siz kesintisi — O1 sınıfı değil, §5), M7X (otorite beslemesi de bozuk) **1**
  (`bs` 51).
- **Ablasyon:** iyileşmenin taşıyıcısı **hijyen**; gerçek otorite §19.9 olmadan (`bs2n`)
  AUD1/2/3/M7X'te 521/373/488/904 kaçırıyor ve AUD3'te **sert hata** (M3f 7, M3p 4)
  yapıyor; yalnız itiraz (`bs2d`) bunu düzeltmiyor (512/318/481/830, AUD3 sert hata
  aynı); yalnız hijyen (`bs2h`) `bs2` ile hemen aynı.
- **Güvenlik incelemesi (iki ORTA bulgu) düzeltildi**: yetkisiz kişiye ve olmayan iddiaya
  aynı cevap (`unknown`, varlık kâhini yok); `digest` tam 16 onaltılık hane; bağlantı başına
  ömür boyu 256, iddia+`t` başına 4 farklı özet sınırı (§3.4).
- 36 eski döküm üç taşımada değişmeden geçiyor; yeni döküm 37 üç taşımada geçiyor;
  `scripts/verify.sh` (`QP_VERIFY_BROWSER=0`) yeşil (§7).

## 2. Düzenek kökeni (önce düzenek, sonra uygulama)

| Commit | İçerik |
|---|---|
| `0f0a49e` | **Düzenek + taban (uygulamadan önce):** `rounds/RP2/experiments/harness/` birebir kopya (`harness/`; yalnız `Cargo.toml` yol bağımlılıkları bir düzey derin), `specs/aud1-3.json` (AUDIT.md'den; `tools/mkspecs.py` S9 üzerine kurar), `raw/baseline/` |
| `ab36ad0` | qp-schema/qp-session uygulaması, §19.9, döküm 37, testler |
| `75666b7` | güvenlik incelemesi düzeltmeleri |
| `bdc1a3d` | hijyende besleme düzeltmesinin taranması; yansıda itiraz fırtınasının kesilmesi |
| `92bbc51` | düzenekte `bs2` + ablasyonlar + `m7stats` + `specs/m7x.json` |

**Taban yeniden üretimi:** `bs` AUD1/2/3, tohum 0–99: M5 **7/4539, 49/4739, 7/4524**, M13
1.268/4/1.269 s, M10 47,5/867,1/1.857,4 — denetimle bire bir. S0–S19 tablosu
`rounds/RP2/experiments/raw/t8/table-bs.txt` ile **bayt bayt aynı** (`raw/baseline/`).
Düzeneğin ölçü kodu (oracle, metrics, sim, world, feed, channel) değiştirilmedi; eklenenler
yalnız yaklaşım dosyaları (`approaches/bs2*.rs`, `mod.rs` tek satır) ve ayrı bir ikili
(`src/bin/m7stats.rs`, yaklaşımın kendi sayaçlarını okur).

## 3. Tasarım

### 3.1 Protokol (`qp-schema::protocol`, hepsi ekleme, isteğe bağlı)

- `ClientMsg::Dispute {id, claim, t, values?, anchor_t?, digest?}` → `result`
  `{"kind":"dispute","verdict","evidence"?}`; `DisputeVerdict`: `pending`, `upheld`,
  `rejected`, `duplicate`, `unknown`, `limited`.
- `Event::Claim.reason?`: `data_corrected`, `dispute_upheld`, `dispute_rejected`.
- `InitData.settled?` (`InitSettled {feeds:[{feed,bars}], verify_closed}`): test
  sağlayıcısının eşzamanlı kesin kaynağı (dökümler için).
- MCP: `dispute` bir araç değildir (yansı barındırıcısının mesajıdır); taşıma `input` gibi
  doğrudan iletir — `qp-mcp` değişmedi, araç sayısı aynı.

### 3.2 Otorite (`qp-session/src/dispute.rs`)

Barındırıcı API'si: `set_settle_check`, `take_settle_requests` → `SettleRequest {series,
feed, from_t, to_t}`, `settled_bars(series, bars)`, `resync_bindings`. Belirleyici: saat
yok; kesin veri barındırıcıdan gelir.

- **Hijyen (açıkken):** bağlı serinin kapanan her barı (ve beslemenin düzelttiği kapalı
  bardan son kapalı bara kadar) için istek. Cevapta: farklı kapalı barlar düzeltilir
  (`FeedStore::reset`, aynı `seq`) → `Claims::after_bars` (Rebuild) kayıtlı iddiaları yeniden
  sınar → geri çekmeler `data_corrected`; cevabın aralığında koşul aboneliklerinin tuttuğu
  ama kayıtlı olmayan iddialar `asserted` (`final`, `data_corrected`). Bu, §19.8'deki
  "düzeltmenin doğurduğu iddia iddia edilmez" sınırını hijyen açıkken kaldırır (ör. kaybolan
  kapanış tiki, S9'un iddia doğuran düzeltmesi). Beslemenin kendi düzeltmesi taramayı
  hemen yapar (kesin cevap sonra doğrular): S9'da M6f p50 3,4 s → 108 ms.
- **İtiraz:** yalnız kayıtlı iddia; `[çapa, t]` kesin isteği; cevap gelince karar. Açık
  itirazlı iddianın her geri çekilmesi `dispute_upheld`; hâlâ kayıtlıysa itiraz edene
  `asserted` + `dispute_rejected` + otoritenin kanıtı.
- **Kapsül geri çekme (K37):** sebepli düşen kapsül iddiası → motor girdisi `put_capsule`,
  hikâye aşaması `retracted`, geçmişe `{stage: retracted, bar, t, reason}`; karneye satır
  yok. Hikâyesiz (R3 öncesi) kapsülde girdi yok; `CapsuleState`'te `retracted` olmadığı için
  `state` (`invalidated`) değiştirilmiyor — öneri §8.
- Sıradan besleme düzeltmesi (hijyen kapalı, itiraz yok) eskisi gibi: sebepsiz geri
  çekme, doküman değişmez → 35/36 dökümleri aynı.

### 3.3 Yansı (`replica.rs`)

`ReplicaOptions.dispute` (varsayılan **kapalı**, barındırıcı açar): bir iddia `disputed`
olunca (kendi temiz, yeniden çekilmiş verisi tutmuyor) giden kutusuna `dispute` (kendi
değerleri, çapa, `t`'ye kadarki barlarının `data_digest`'i). Tekrarsızlık **(iddia, `t`,
otoritenin kanıtı)** başına: ilk sürümde anahtar yansının özetiydi ve yansının verisi her
değiştiğinde yeni itiraz doğuyordu (AUD1'de 100 tohumda 25.461 itiraz, 24.091'i `limited`);
anahtar değiştirilince haksız itirazın cevabı aynı kanıtı yeniden ileri sürdüğü için döngü
de yok.

### 3.4 Güvenlik incelemesi (koordinatörün bildirdiği iki ORTA bulgu, `75666b7`)

1. **Varlık kâhini / yetki atlatma:** iddia kaydı yetki denetiminden önce bakılıyordu
   (olmayan → `unknown`, var ama yetkisiz → `no_entitlement`). Düzeltme: yetkisiz kişinin
   itirazı ile olmayan iddianınki **aynı** cevabı alır (`unknown`), kesin istek de yapılmaz.
   Test: `an_unentitled_dispute_learns_nothing` (iki cevap bayt eşit, istek kuyruğu boş).
2. **Sınırsız durum büyümesi:** tekrarsızlık kaydı istemcinin seçtiği `digest` ile
   anahtarlanıyordu. Düzeltme: `digest` verilirse tam 16 küçük onaltılık hane, değilse
   `invalid` (`/digest`); bağlantı başına en çok 16 açık, ömür boyu en çok 256 itiraz;
   (bağlantı, iddia, `t`) başına en çok 4 farklı özet; aşılırsa `limited`. Bağlantı kapama
   kavramı oturumda olmadığı için "kapanışta silme" yok; sınır bağlantı başına sabit
   (≤ 256 kayıt). Test: `disputes_are_bounded_per_claim_and_digests_checked`.

## 4. Düzenek yaklaşımı `bs2`

`approaches/bs2.rs`. **Otorite:** gerçek `qp_session::Session`; barındırıcısı (bağdaştırıcı)
düzeneğin motorunu veri katmanı olarak kullanır (boşluğa dayanıklı, alan başına son yazan),
oturuma boşluksuz yerel akış verir; birden çok bar değişince (bölünme anlık görüntüsü,
çoklu düzeltme) `resync_bindings`. Geçmiş tek barla çözülüp bar bar eklenir: koşullar son
barda değerlendirildiği için canlı bir otoritenin geçmişte vereceği iddialar böyle oluşur
(yoksa geçmiş iddialar hiç gelmez ve M5 anlamsızlaşır). Yansının bağlantısı ve abonelikleri
koşu başında vardır (`bs`'de de B'nin otoritesi abonelikleri örtük biliyordu). Hijyen:
kapanıştan 2,5 s sonra kesin çekme (B'nin `VERIFY_DELAY`'i), bar kesinleşmemişse 1 s'de bir
yeniden; itiraz isteği hemen. Kopukken satırlar tutulup yeniden bağlanınca tek çerçeve
gider (akış taşımasının gönderme tamponu; yoksa düzeneğin kanalı tutulan çerçeveleri 1 µs
arayla dağıtıyor ve "ileri sür + 1 µs sonra geri çek" 1 µs'lik geçici gösterim üretiyordu —
AUD2'de 6 M3p, düzeltildi). Otoritenin bağlantı başına 32 abonelik sınırı yüzünden 30'dan
çok koşul aboneliği (S17: 100) ek bağlantılara dağıtılır; ilk koşuda bu yoktu ve S17'de
5.568/8.025 kaçırma çıktı. **Yansı:** `bs` ile aynı + itiraz.

Ablasyonlar: `bs2h` (yalnız hijyen), `bs2d` (yalnız itiraz), `bs2n` (hiçbiri: gerçek otorite
+ gerçek yansı, §19.9'suz). Ek senaryo `M7X` = AUD1 + S14'ün otorite besleme bozuklukları
(%2 ±%1 bozuk tik, %5 kayıp, kapanış tikinin %30 kaybı).

## 5. Sonuçlar (tohum 0–99; S17 0–19)

Komut: `tools/run.sh`. Ham:
`raw/bs2/table-*.txt`, `raw/bs2/*.compact.json`, `raw/bs2/stats-*.txt`; taban
`raw/baseline/`. Belirleyicilik: `bs2n`, `bs2h`, `bs2d` AUD tabloları iki ayrı koşuda (arada yalnız yansının itiraz anahtarı değişti, onları etkilemiyor) aynı sayıları verdi.

### 5.1 Denetçinin senaryoları ve M7X (`bs` → `bs2`, ablasyon `bs2n`)

| Senaryo | M1/M2/M3f/M3p bs → bs2 | M5 bs | M5 bs2 | M5 bs2n | M6f p50/95/99 bs → bs2 (ms) | M9 B/iddia · B/s bs → bs2 | M10 iş/olay bs → bs2 | M13 s bs → bs2 |
|---|---|---|---|---|---|---|---|---|
| AUD1 | 0/0/0/0 → 0/0/0/0 | 7/4539 | **0/4539** | 521/4539 | 3248/4993/7188 → 3278/5105/7412 | 621 · 10.7 → 1542 · 26.7 | 47.5 → 23.5 | 1268 → 30 |
| AUD2 | 0/0/0/0 → 0/0/0/0 | 49/4739 | **53/4739** | 373/4739 | 3430/7518/97317 → 3476/7854/97197 | 480 · 8.0 → 1385 · 23.0 | 867.1 → 471.8 | 4 → 5 |
| AUD3 | 0/0/0/0 → 0/0/0/0 | 7/4524 | **0/4524** | 488/4524 | 3248/4902/7266 → 3275/5172/7384 | 622 · 10.8 → 1541 · 26.6 | 1857.4 → 1084.8 | 1269 → 26 |
| M7X | 0/0/0/0 → 0/0/0/0 | 51/4539 | **1/4539** | 904/4539 | 3505/6219/267995 → 3562/6083/8379 | 654 · 11.3 → 1708 · 29.5 | 47.9 → 15.9 | 4710 → 49 |

Ablasyon (M5 kaçan / uygun; "sert" = M1+M2+M3 > 0 olan tohum):

| | AUD1 | AUD2 | AUD3 | M7X |
|---|---|---|---|---|
| `bs` (B'nin otoritesi) | 7 | 49 | 7 | 51 |
| `bs2n` (gerçek otorite, §19.9 yok) | 521 | 373 | 488, **sert 5 tohum (M3f 7, M3p 4)** | 904 |
| `bs2d` (yalnız itiraz) | 512 | 318 | 481, **sert 5 tohum (M3f 7, M3p 4)** | 830 |
| `bs2h` (yalnız hijyen) | 0 | 53 | 0 | 1 |
| **`bs2`** (ikisi) | **0** | **53** | **0** | **1** |

`bs2` ek sayaçları (`m7stats`, 100 tohum toplamı):

| | AUD1 | AUD2 | AUD3 | M7X | S14 |
|---|---|---|---|---|---|
| İtiraz gönderilen / karar `pending` → sonra `rejected` / `unknown` / `upheld` | 396 / 367 / 29 / 0 | 41 / 37 / 4 / 0 | 398 / 359 / 39 / 0 | 356 / 318 / 36 / **2** | 15 / 0 / 14 / **1** |
| Sebepli geri çekme `data_corrected` (+ `dispute_upheld`) | 73 | 77 | 64 | 546 + 2 | 27 + 1 |
| Sebepli ileri sürme `data_corrected` (kesin veride bulunan iddia) | 71 | 76 | 73 | 542 | 64 |
| Yanlış **kesin** iddianın düzeltme gecikmesi (kesin gönderildi → sebepli geri çekme) | — (0 olay) | — | — | n = 26, p50 **3,3 s**, p95 3,3 s, en çok 300 s | n = 15, p50 3,3 s, en çok 3,3 s |
| Kesin bar çekme (otorite barındırıcısı) | 6.525 | 5.783 | 6.520 | 6.366 | 6.099 |

Okuma:

- **Hedef tuttu:** M1 = M2 = M3 = 0 her senaryoda; M5 AUD1/AUD3 **7 → 0**, M7X **51 → 1**.
  AUD2 **49 → 53**: ölçülebilir iyileşme yok (§6.1).
- **O1'in kendisi gerçek otoritede bu biçimde oluşmuyor.** AUD1 tohum 76'da B'nin otoritesinin
  yanlış "30 aşağı" kesin iddiası gerçek otoritede yok: barındırıcı bölünmeyi anlık görüntü
  olarak `resync_bindings` ile veriyor, sıra dışı eski tik alan başına son yazan kuralıyla
  geri sarmıyor. Gerçek otoritenin başlıca hatası **eksiklik**: koşullar yalnız son barda
  değerlendirildiği için kaybolan kapanış tiki ya da iddia doğuran düzeltme iddiasız kalıyor
  (`bs2n` AUD1 521, S8 1.045, S9 311, S14 60). Hijyenin kesin veri taraması bunu kapatıyor
  (`bs2` hepsinde 0).
- **Yanlış kesin iddia** gerçek otoritede de var: AUD3'te (`bs2n`, `bs2d`) 5 tohumda yansı
  yanlış kesin gösterdi (M3f 7) — muhtemelen iki ucun aynı kapanışı kaçırıp aynı yanlışta
  anlaştığı §19.8 durumu (yansı f32 + birleştirme, kapanış yarışı 0,2); tek tek izlemedim.
  Hijyen bunu sıfırlıyor (`bs2h`, `bs2`). M7X/S14'te otoritenin bozuk tiklerinden çıkan
  26/15 yanlış kesin iddia hijyenle kapanıştan **~3,3 s** sonra (2,5 s bekleme + 0,8 s kesin
  kaynak) `data_corrected` ile geri çekildi; 2 + 1'i itirazla (`dispute_upheld`). En uzun
  300 s'lik tek olay: kesin kaynağın barı geç kesinleştirdiği bir yeniden deneme zinciri.
- **İtirazın payı küçük:** hijyen açıkken itirazların çoğu `rejected` (yansının verisi geride
  ya da kendi kapanışını kaybetmiş; otorite kesin veride haklı) ve yansı `disputed`
  göstermeye devam ediyor (M13 AUD1 1.268 s → 30 s). `bs2d` ile `bs2n` arasındaki fark
  küçük: itiraz yalnız ileri sürülmüş yanlış iddiayı düzeltebilir, eksik iddiayı değil.
- **M6f:** temiz akışta aynı (S0 111/137/139 ms); hijyen kesinliği beklemediği için gecikme
  eklemiyor. S9'un iddia doğuran düzeltmeleri beslemenin düzeltmesinde hemen taranıyor
  (108 ms; ilk sürümde 3,4 s idi).
- **M9 bayt:** iddia başına ~1,5 kB (`bs` ~0,6 kB): gerçek otorite her canlı kenarda
  `provisional` iddia ve `condition_met` gönderiyor, geçmiş bar bar oynatıldığı için geçmiş
  iddialar da iki adımlı (geçici + kesin). S7'de (bar başına 6 salınım) 12,5 kB/iddia,
  320 B/s. r→a yönü (itirazlar) AUD1 0,39 B/s. Akış-deflate a→r AUD1 0,82 → 1,36 B/s.
- **M10 yansı işi:** düşüyor (AUD1 47,5 → 23,5; AUD3 1.857 → 1.085; S12 2.460 → 586):
  gerçek otoritenin kanıtı yansının kendi verisiyle daha sık aynı çapada geliyor ve
  `disputed` → yeniden çekme döngüsü kısalıyor (bu yorumu ayrıca doğrulamadım).
- **M8 (yanlış engelleme):** AUD1 5.618 → 4.788 s; `bs2n`'de 848.860 s (iddiasız kalan doğru
  barlar) — hijyenin asıl kazancı bu sütunda da görünüyor.

### 5.2 Hazır senaryolar S0–S19 (+ S2a, S2c, S12s, S15k)

| Senaryo | M1/M2/M3f/M3p bs → bs2 | M5 bs | M5 bs2 | M5 bs2n | M6f p50/95/99 bs → bs2 (ms) | M9 B/iddia · B/s bs → bs2 | M10 iş/olay bs → bs2 | M13 s bs → bs2 |
|---|---|---|---|---|---|---|---|---|
| S0 | 0/0/0/0 → 0/0/0/0 | 0/1202 | **0/1202** | 0/1202 | 111/137/139 → 111/137/139 | 463 · 1.5 → 1557 · 5.2 | 3.7 → 3.1 | 0 → 0 |
| S1 | 0/0/0/0 → 0/0/0/0 | 0/1202 | **0/1202** | 0/1202 | 0/0/0 → 0/0/0 | 463 · 1.5 → 1557 · 5.2 | 3.7 → 3.1 | 0 → 0 |
| S2a | 0/0/0/0 → 0/0/0/0 | 0/1202 | **0/1202** | 0/1202 | 513/524/525 → 513/524/525 | 463 · 1.5 → 1557 · 5.2 | 3.7 → 3.1 | 0 → 0 |
| S2 | 0/0/0/0 → 0/0/0/0 | 0/1202 | **0/1202** | 0/1202 | 5130/5238/5248 → 5130/5238/5248 | 463 · 1.5 → 1557 · 5.2 | 4.0 → 3.6 | 0 → 0 |
| S2c | 0/0/0/0 → 0/0/0/0 | 0/1202 | **3/1202** | 3/1202 | 61670/62893/62974 → 61670/62893/62974 | 463 · 1.5 → 1557 · 5.2 | 4.1 → 2.6 | 0 → 0 |
| S3 | 0/0/0/0 → 0/0/0/0 | 0/1202 | **0/1202** | 0/1202 | 3522/3987/4025 → 3520/3960/4003 | 463 · 1.5 → 1557 · 5.2 | 3.7 → 3.1 | 0 → 0 |
| S4 | 0/0/0/0 → 0/0/0/0 | 2/1202 | **2/1202** | 2/1202 | 301/1532/30255 → 271/1626/30323 | 463 · 1.5 → 1557 · 5.2 | 3.7 → 3.1 | 0 → 0 |
| S5 | 0/0/0/0 → 0/0/0/0 | 0/1215 | **0/1215** | 0/1215 | 114/739/843 → 114/739/843 | 463 · 1.6 → 1748 · 5.9 | 3.6 → 3.3 | 0 → 0 |
| S6 | 0/0/0/0 → 0/0/0/0 | 0/3028 | **0/3028** | 0/3028 | 3292/3337/3537 → 3227/3252/3537 | 456 · 3.8 → 1325 · 11.1 | 11.9 → 11.7 | 0 → 0 |
| S7 | 0/0/0/0 → 0/0/0/0 | 0/1232 | **0/1232** | 0/1232 | 119/138/140 → 119/138/140 | 462 · 11.9 → 12484 · 320.4 | 3.0 → 3.0 | 0 → 0 |
| S8 | 0/0/0/0 → 0/0/0/0 | 0/3028 | **0/3028** | 1045/3028 | 132/3323/3337 → 132/3394/3403 | 456 · 3.8 → 1404 · 11.8 | 6.9 → 6.7 | 0 → 0 |
| S9 | 0/0/0/0 → 0/0/0/0 | 0/3467 | **0/3467** | 311/3467 | 108/137/140 → 108/137/140 | 640 · 6.6 → 1469 · 15.1 | 10.9 → 9.7 | 0 → 0 |
| S10 | 0/0/0/0 → 0/0/0/0 | 0/1206 | **0/1206** | 0/1206 | 114/138/140 → 114/138/140 | 463 · 1.6 → 1587 · 5.3 | 4.3 → 3.8 | 0 → 0 |
| S11 | 0/0/0/0 → 0/0/0/0 | 0/2559 | **0/2559** | 0/2559 | 111/138/30109 → 111/138/30109 | 457 · 3.2 → 3760 · 26.7 | 4.1 → 0.9 | 0 → 0 |
| S12 | 0/0/0/0 → 0/0/0/0 | 0/9674 | **0/9674** | 0/9674 | 0/0/0 → 0/0/0 | 441 · 11.8 → 1833 · 49.2 | 2459.8 → 585.9 | 0 → 0 |
| S12s | 0/0/0/0 → 0/0/0/0 | 0/9674 | **0/9674** | 0/9674 | 0/0/0 → 0/0/0 | 441 · 11.8 → 1833 · 49.2 | 2483.2 → 591.5 | 0 → 0 |
| S13 | 0/0/0/0 → 0/0/0/0 | 0/1202 | **0/1202** | 0/1202 | 110/137/139 → 110/137/139 | 463 · 1.5 → 1557 · 5.2 | 3.4 → 2.9 | 0 → 0 |
| S14 | 0/0/0/0 → 0/0/0/0 | 0/1202 | **0/1202** | 60/1202 | 112/2879/3397 → 112/3375/3397 | 494 · 1.7 → 1900 · 6.3 | 3.7 → 2.1 | 3 → 2 |
| S15 | 0/0/0/0 → 0/0/0/0 | 0/2559 | **0/2559** | 0/2559 | 0/0/0 → 0/0/0 | 457 · 3.2 → 2790 · 19.8 | 3.7 → 1.1 | 0 → 0 |
| S15k | 0/0/0/0 → 0/0/0/0 | 0/2568 | **0/2568** | 0/2568 | 800/3200/3200 → 800/3200/3200 | 457 · 3.3 → 1328 · 9.5 | 100.4 → 31.3 | 0 → 0 |
| S16 | 0/0/0/0 → 0/0/0/0 | 284/2767 | **284/2767** | 284/2767 | 138/5224/5246 → 138/5224/5246 | 415 · 3.2 → 1337 · 10.3 | 3.9 → 3.5 | 0 → 0 |
| S17 | 0/0/0/0 → 0/0/0/0 | 0/8025 | **0/8025** | 0/8025 | 114/139/140 → 114/139/140 | 478 · 533.2 → 1639 · 1826.4 | 13.5 → 13.5 | 0 → 0 |
| S18 | 0/0/0/0 → 0/0/0/0 | 1448/3097 | **1448/3097** | 1448/3097 | 109/137/139 → 109/137/139 | 245 · 2.1 → 874 · 7.5 | 3.7 → 2.6 | 0 → 0 |
| S19 | 0/0/0/0 → 0/0/0/0 | 0/2559 | **0/2559** | 0/2559 | 112/139/120087 → 112/139/69805 | 457 · 3.2 → 3760 · 26.7 | 4.1 → 0.9 | 0 → 0 |

`bs2` S17'nin ilk koşusu 5.568/8.025 kaçırdı: otoritenin bağlantı başına 32
abonelik sınırı, 100 koşul aboneliği sığmıyordu; aboneliklerin ek bağlantılara dağıtılmasıyla
0/8.025.

## 6. Dürüst sınırlar

1. **AUD2 iyileşmedi (49 → 53).** Kaçırmalar yansının REFRESH'siz 240 s'lik kesintisinden
   (bölünme ve düzeltmeler kesintinin içinde); `bs2n`/`bs2h`/`bs2` aynı tohumlarda birer
   kaçırma. Bu O1 sınıfı (otoritenin yanlışı) değil, yansının veri katmanının sınırı;
   §19.9 onu hedeflemiyor. 4 tohumda `bs` kaçırmadı, `bs2` kaçırdı (23, 52, 95; 51'de 2).
2. **S2c 0 → 3** (`bs2n` de 3): son geçmiş barın iddiası canlı başlangıcında kesinleşiyor,
   60 s geride kalan yansı onu Δ_miss (30 s) içinde gösteremiyor; `bs`'de B'nin otoritesi
   bu iddiayı farklı zamanda kesinleştiriyordu. §19.9 ile ilgisiz; incelemedim.
3. **Düzenekte gerçek otoritenin geçmişi bar bar oynatılıyor**; böyle yapılmasa koşullar son
   barda değerlendirildiği için geçmiş iddialar hiç gelmez. Gerçek bir sunucu geçmiş
   iddiaları nasıl üreteceğini ayrıca seçmeli (öneri §8).
4. **Kapsül geri çekmesi** yalnız hikâyesi olan kapsülde motor girdisi yazıyor; `state`
   (`invalidated`) değişmiyor (`CapsuleState`'te `retracted` yok; şema değişikliği bu
   deneyin dosya kapsamı dışında). Karneden satır silme yok: `qp-session` geçersizleşmede
   karneye zaten yazmıyor; `qp-capsule` motorunun `retract`'ıyla bağ kurulmadı.
5. **Hijyen her kapanan barı kesin kaynaktan çekiyor** (100 tohumda ~6.100 çekme / 60 bar;
   bağ × bar başına bir REST çağrısı). Matriks REST kotası bilinmiyor; şüpheli barla
   sınırlamak mümkün (B gibi) ama ölçülmedi.
6. **Tarama koşul abonelikleriyle sınırlı;** niyet ve kapsül geçersizleşmeleri kesin veride
   yeniden taranmıyor. S16/S18 kaçırmaları (`qp/0` koşulu olmayan dedektörler) aynen kaldı.
7. Tarama, kesin cevabın aralığındaki bütün kapalı barlara bakıyor: itirazda `[çapa, t]` —
   abonelikten önceki barlarda da kesin veride tutan iddiayı ileri sürebilir (doğru ama
   gürültülü). Aboneliğin başlangıcıyla sınırlamak öneri.
8. Hijyen açıkken yeniden kurulumda (Rebuild) canlı barın geçici iddiası da yeniden sınanıp
   `data_corrected` ile geri çekilebiliyor (AUD'lardaki 64–77 "sebepli geri çekmenin" çoğu
   bu: kesin gönderilmemiş geçici iddialar); anlam doğru, sebep etiketi yanıltıcı.
9. Belirleyicilik ve ölçümler tek makinede (aarch64 macOS); `bs2n` AUD3 sert hatalarının
   olay düzeyinde izi çıkarılmadı.
10. `docs/PROTOCOL.md` §19.8'deki "`dispute` v0'da yok" maddesi dosya kapsamı gereği
    değiştirilmedi; §19.9 onun yerini aldığını söylüyor — ana akışta §19.8 düzeltilmeli.

## 7. Doğrulama

- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`: yeşil (qp-session `tests/dispute.rs` 6 test dahil).
- `qp conform fixtures/protocol/conformance` inproc / serve / mcp: **37/37** (36 eski döküm
  değişmeden; 37 yeni).
- `QP_VERIFY_BROWSER=0 scripts/verify.sh`: **verify: OK** — 1 fmt, 2 clippy, 3 test,
  4 schema (28 şema), 5 fixtures, 6 golden, 7 views, 8 wasm (gzip 598.471 B), 9 hygiene,
  11 compute, 12 digest (`7b1e99d40889f83c`), 13 protocol, 14 conform (37 × 3), 15 replica
  PASS; 10 browser SKIP.

## 8. Önerilen karar satırı (DECISIONS'a ana akış yazar)

> **K-M7 (A20 uygulaması, K36):** `qp/1` §19.9 kabul: (a) **kesin bar hijyeni**
> otoritenin barındırıcı API'sidir (`set_settle_check`, `take_settle_requests`,
> `settled_bars`, `resync_bindings`); açıkken kapanan ve düzeltilen her kapalı bar kesin
> kaynakla doğrulanır, farklılık otoritenin verisini düzeltir, iddialar `data_corrected`
> ile geri çekilir ya da yeniden ileri sürülür ve **kesin veride tutan ama kayıtlı olmayan
> koşul iddiaları ileri sürülür**; kesinlik doğrulamayı beklemez. (b) **`dispute`**
> yansıdan otoriteye mesajdır; karar yansının değerlerinde değil kesin veridedir
> (`upheld` → `withdrawn`/`dispute_upheld`, `rejected` → kanıt yalnız itiraz edene);
> yetkisiz ve olmayan iddia aynı cevabı alır; sınırlar 16 açık / 256 ömür / iddia başına 4
> özet. (c) Sebepli düşen kapsül iddiası kapsülü **geri çeker** (hikâye aşaması `retracted`,
> karne yok). Kanıt: M7 — gerçek otorite + gerçek yansı, AUD1/AUD3 M5 7 → 0, M7X 51 → 1,
> bütün senaryolarda M1–M3 = 0; ablasyon: iyileşmeyi taşıyan hijyen (§19.9'suz gerçek
> otorite AUD1'de 521 kaçırma ve AUD3'te 5 tohumda yanlış kesin gösterim), itiraz yanlış
> **ileri sürülmüş** iddiaya karşı ikinci savunma (M7X 2, S14 1 haklı itiraz). Sınır: AUD2
> (yansının REFRESH'siz kesintisi) iyileşmedi; hijyen bağ × bar başına bir kesin çekme
> ister. **Açık:** `CapsuleState`'e `retracted` eklenmesi; hijyenin şüpheli barlarla
> sınırlanması (REST kotası); gerçek sunucuda geçmiş iddiaların üretimi.

## 9. Öneriler (kapsam dışı; yapılmadı)

- `CapsuleState::Retracted` (şema) ve `qp-capsule` `Engine::retract` ile `qp-session`
  geri çekmesinin bağlanması (karnenin zamana bağlı `retracted_at`'ı).
- §19.8'in "`dispute` v0'da yok" maddesinin kaldırılması (ana akış).
- Taramanın aboneliğin başlangıç barıyla sınırlanması; Rebuild'de canlı geçici iddianın
  sebep etiketi.
- Düzeneğe gerçek otoriteyle REFRESH'siz kesinti sonrası yansı onarımı (AUD2) için ayrı
  bir deney.
