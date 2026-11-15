# M3 adayı "nadirlik" (`rarity`)

Savunduğum öncül: bir hikâyenin önemi, bu türden bir olayın **bu enstrümanda (ve piyasada)
ne kadar seyrek** görüldüğüdür. Önem = şaşkınlık `S = −ln p` (nat); `p`, bu enstrümanın bir
barında böyle bir hikâye görme olasılığıdır ve yalnız geçmişten hesaplanır.

Crate: `rounds/R3/experiments/M3/rarity/` (`m3_rarity::ranker()`, ad `rarity`). Düzeneğe bir
path bağımlılığı ve bir kayıt satırıyla girer. Ölçüm: `harness-sim m3 run --alt rarity`.

## Kural

`S` üç parçanın toplamıdır; hepsi `RankCtx` ve hikâyenin kendisinden okunur (durum yok, saat
yok, ileriye bakma yok).

1. **Sıklık şaşkınlığı** `−ln(p_enst × pay)`.
   - Piyasa oranı (enstrüman-bar başına, bu tür ve yön):
     `p_piy = (same_kind_all × y + 200 × 0,02) / (instruments × bars_seen + 200)`;
     `y` = 0,5 yönlü, 1 nötr hikâye. 200 sahte bar × 0,02 soğuk başlangıç önselidir.
   - Enstrüman oranı, piyasaya doğru büzülmüş (ampirik Bayes, 120 sahte bar):
     `p_enst = (same_kind_on_instrument + 120 × p_piy) / (bars_seen + 120)`.
     Piyasada sık, bu enstrümanda seyrek olan olay bu enstrüman için şaşırtıcıdır; az veride
     piyasa oranı ağır basar.
   - Dedektör payı (türün içinde): karnede kapanmış hikâyelerden
     `pay = ((ledger.n + 0,5) / (ledger_all.n + 1) × stories_all) / same_kind_all`,
     [0,05, 1] aralığında. Aynı türde seyrek dedektör (ör. `flow.volume_surge`, kurumsal
     akışın yanında) daha şaşırtıcıdır.
2. **Büyüklük şaşkınlığı** `κ × güç`, κ = 1. Dedektör gücü eşiğin ne kadar ötesine
   geçildiğidir; üstel kuyrukla `P(güç ≥ s | olay) ≈ e^(−κs)`. Güç sık doyuyor (sentetikte
   kurumsal akış ve MA dizilişi olaylarının ~%40'ı tam 1), bu yüzden κ küçük tutuldu:
   güç 1 ≈ "bu türün olaylarının üçte biri". κ = 2 sınandı: önemli seviyeyi iki katına
   çıkarıyor, K1 − K3'ü düşürüyor (aşağıda duyarlılık).
3. **Birliktelik şaşkınlığı**: her ek bağımsız kanıt ailesi (K30) için `−ln q`,
   `q = herhangi bir hikâyenin enstrüman-bar oranı × 5 bar` ([0,01; 0,9]). Bağımsızlık
   altında başka bir ailenin 5 bar içinde tetiklemesinin olasılığıdır. Doğum barında
   hikâyelerin tek ailesi olduğu için doğum kademesini etkilemez; sonraki geçişlerde
   (`confirmed`) önemi artırır.

**Kademe:** `N = e^S` "böyle bir olay bu enstrümanda N barda bir" diye okunur.

| Kademe | Eşik | Günlük barda anlamı |
|---|---|---|
| 1 | `N ≥ 150` bar | yılda ~1,5 kereden seyrek |
| 2 | `N ≥ 50` bar | çeyrekte ~1 kereden seyrek |
| 3 | geri kalan | — |

Önem = `S` (nat); kademe her geçişte yeniden hesaplanır (motor böyle çağırır).

## Dikkat bütçesi ve eşiklerin gerekçesi

`NORTH_STAR.md`: geleceğin yatırımcısı liste görmek istemez, bilgi önüne kapsül olarak gelir;
`docs/CAPSULE.md` §6: onun varsayılanı dar, "az ama önemli"; alışkın kullanıcının listesi her
şeyi gösterir. Hedefim:

- **important** (kademe 1): 30 enstrümanlık evrende **günde ~1 kapsül** (enstrüman başına
  ayda ~0,7). 10 enstrümanlık bir izleme listesine haftada ~2 kapsül demektir: her gün
  bakılacak bir akış değil, gelince dikkat çeken bir haber. Sıfır gün oranı ~%35–40 kabul
  edilebilir: "bugün olağandışı bir şey yok" da bir bilgidir.
- **balanced** (1–2): günde ~4 (doğumların ~%35'i): meraklı ama boğulmayan kullanıcı.
- **all**: hepsi (~10,7/gün).

Eşikler önce yuvarlak "N barda bir" değerleriyle (250/60) denendi; önemli seviye 0,49/gün
(sıfır gün %63) çıktı, fazla dar. 150/50'ye inildi. Ayar yalnız tohum 1'e bakılarak yapıldı;
tohum 2–3 ve gerçek veri sonradan doğrulama olarak okundu (tohumlar arası göreli fark %6).

## Sonuçlar (son kural; başvuru tema `kind_direction` + sonuç `atr_barrier`)

### Sentetik (30 enstrüman × 500 gün × 3 tohum)

| Ölçü | rarity | başvuru `families` | control `round_robin` |
|---|---|---|---|
| Günlük doğum ort. important / balanced / all | **1,02** / 4,02 / 10,69 | 0,64 / 5,58 / 10,69 | 3,56 / 7,13 / 10,69 |
| important medyan · p90 · en çok | 1 · 2 · 5 | 0 · 2 · 4 | — |
| important sıfır gün · DK | 0,381 · **1,03** | 0,520 · 1,22 | 0,002 · — |
| important rejim oranı (en yüksek / en düşük) | **1,42** | 6,05 | — |
| balanced rejim oranı | 2,05 | 1,99 | — |
| important tohum farkı (göreli) | 0,061 | 0,066 | — |
| 5 günde tekrar important / balanced | 0,185 / 0,526 | 0,080 / 0,596 | 0,453 / — |
| En uzun dizi important | 2 | 2 | — |
| K1 / K2 / K3 gerçekleşen | 0,540 / 0,478 / 0,457 | 0,470 / 0,485 / 0,458 | 0,465 / 0,473 / 0,477 |
| Taban · **K1 − K3** | 0,471 · **+0,083** (tohumlar +0,082 / +0,085 / +0,084) | 0,471 · +0,011 | 0,471 · −0,012 |
| İleriye bakma | geçti (9 kesim, 0 fark) | geçti | geçti |

### Gerçek veri (6 sembol, toplu; ham kayıt `fixtures/local/`'da)

| Küme | important / balanced / all günlük ort. | important sıfır gün | 5 günde tekrar important | K1 / K3 pay | Taban | K1 − K3 | İleriye bakma |
|---|---|---|---|---|---|---|---|
| günlük | 0,17 / 0,80 / 1,30 | 0,856 | 0,159 | 0,636 / 0,552 | 0,546 | +0,083 | geçti |
| saatlik | 0,97 / 7,33 / 10,37 | 0,448 | 0,583 | 0,656 / 0,486 | 0,493 | +0,170 | geçti |

Günlük gerçek veride important 6 sembolde 0,17/gün = enstrüman-gün başına 0,028; sentetikteki
0,034'e yakın (eşikler bar cinsinden olduğu için veriden bağımsız taşındı). Başvuru
sıralayıcı gerçek veride hiç kademe 1 vermiyor (doğumda iki aile olmuyor).

### Kademe 1'in bileşimi (doğumda, tür başına; analiz betiği, depoya girmedi)

| Tür | sentetik K1 (pay) | gerçek günlük K1 | gerçek saatlik K1 |
|---|---|---|---|
| continuation | 409 (0,507) | 48 | 41 |
| reversal | 461 (0,431) | 109 | 196 |
| extreme | 238 (0,500) | 30 | 2 |
| squeeze (nötr) | 225 (**0,959**) | 97 (**1,000**) | 173 (**0,988**) |
| flow | 187 (0,435) | 90 | 74 |
| breakout | 89 (0,539) | 6 | 3 |

Kademe 1 tek bir türe kilitlenmiyor: sentetikte altı tür de var (büzülme ve büyüklük
terimi sayesinde). Sık türler (kırılım, kurumsal akış) ancak bu enstrümanda seyrekse ya da
güçlüyse yükseliyor.

### Duyarlılık (sentetik havuz; yalnız biri değişir)

| Değişiklik | important ort. | important rejim oranı | K1 − K3 |
|---|---|---|---|
| son kural (büzülme 120, κ 1) | 1,02 | 1,42 | +0,083 |
| büzülme 40 | 1,17 | 1,51 | +0,066 |
| büzülme 400 | 0,80 | 1,39 | +0,096 |
| κ = 0 (yalnız sıklık) | 0,62 | 1,44 | +0,122 |
| κ = 2 | 2,25 | 2,09 | +0,068 |
| eşik 250 / 60 | 0,49 | 1,49 | +0,135 |
| eşik 180 / 60 | 0,77 | 1,41 | +0,102 |

## Zayıflıklar (dürüstçe)

1. **K1 − K3 üstünlüğü büyük ölçüde nötr sıkışmadan geliyor.** Sıkışma seyrek olduğu için
   hep kademe 1'de; başvuru sonuç kuralı nötr hikâyeyi `|hareket| ≥ 1 ATR` ile
   "gerçekleşti" sayıyor ve hiç geçersiz saymıyor, bu yüzden sıkışmanın payı ~0,96–1,00.
   Yalnız **yönlü** hikâyelerde (analiz betiğiyle) K1 / K3: sentetik 0,472 / 0,456
   (**+0,016**), gerçek günlük 0,505 / 0,552 (**−0,047**), saatlik 0,462 / 0,486
   (**−0,024**). Yani nadirlik yönlü hikâyelerin gerçekleşmesini **öngörmüyor**. Bu, öncülün
   doğası: seyrek olan önemlidir, isabetli olduğu için değil. Düzenek için de bir bulgu:
   M3'ün "gerçekleşen payı" ölçüsü, nötr hikâyeyi yukarı taşıyan her sıralayıcıyı
   ödüllendirir; adaylar karşılaştırılırken yönlü/nötr ayrımı ya da M5'te seçilen sonuç
   kuralıyla yeniden koşu gerekir.
2. **Yakın tekrarı göremez.** `RankCtx` aynı türün bu enstrümandaki son doğumundan beri
   geçen barı vermiyor; yalnız toplam sayıyı veriyor. Bu yüzden aynı enstrümanda arka arkaya
   gelen seyrek olay ikinci kez de seyrek sayılıyor: 5 günde tekrar important 0,185
   (başvuru 0,080); saatlik gerçek veride 0,583. Nadirliğin en doğal girdisi ("son kez ne
   zaman?") eksik.
3. **"Piyasada" boyutu zayıf.** Aynı barda kaç enstrümanda aynı olayın olduğu (piyasa
   geneli hareket mi, tekil mi?) `RankCtx`'te yok; yalnız birikimli sayılar var. Piyasa
   yalnız büzülme önseli olarak giriyor.
4. **Eşikler bar cinsinden.** Saatlik seride gün başına bar ~8 kat olduğu için günlük
   bütçe de büyüyor (saatlik important 0,97/gün, 6 sembolde). Bar süresi `RankCtx`'te yok.
5. **Güç kalibrasyonu varsayım.** Dedektör güçlerinin dağılımı farklı (kırılımda %22,
   kurumsal akışta %40 tam 1); tek κ hepsine aynı davranıyor. Geçmiş güç dağılımı olmadan
   gerçek koşullu kuyruk hesaplanamaz.
6. **Dedektör payı karneden geliyor**, yani kapanmış hikâyelerden: ufku kısa dedektörün payı
   biraz şişer, kapanış gecikmesi kadar geride kalır.
7. **Rejim kararlılığı kısmi.** important rejim oranı 1,42 (başvuru 6,05) iyi; balanced
   2,05, başvurunun 1,99'u kadar, çünkü oranlar birikimli ve rejime uyum yavaş.

## Öneriler (kapsam dışı; yapılmadı)

- `RankCtx`'e `bars_since_same_kind_on_instrument` (son doğumdan beri bar) ve
  `same_kind_this_bar_all` (bu barda aynı türü doğan enstrüman sayısı) eklenirse nadirlik
  yakın tekrarı ve piyasa genelini doğrudan ölçebilir (hazard tabanlı şaşkınlık).
- Bar süresi (`series` dönemi) `RankCtx`'te olursa eşik "N barda bir" yerine "N günde bir"
  yazılabilir; saatlik ve günlük seriye aynı bütçe gider.
- M3 gerçekleşen payı nötr ve yönlü hikâyelerde ayrı raporlanmalı (zayıflık 1).

## Testler

`cargo test` (6): belirlenimcilik (aynı girdi → bayt eşit önem, aynı kademe; `ranker()` ile
`Rarity::default()` aynı), kademe her zaman 1..=3 ve önem sonlu (uç girdiler: sıfır bar, sıfır
enstrüman, `u32::MAX`, NaN/negatif/>1 güç, boş aile listesi), enstrümanda seyrek olan daha
önemli, güçlü ve teyitli olan daha önemli, eşikler bar cinsinden, ad `rarity`.
`cargo clippy --all-targets -D warnings` temiz. Düzeneğin kendi testleri (14) geçti.
