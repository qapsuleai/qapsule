# R3 — Ölçüm düzenekleri şartnamesi (H)

Yazan: orkestratör, 2026-11-15, **hiçbir alternatif yazılmadan önce** (TASKS "Düzenek önce").
Uygulama: düzenek alt ajanları, Evre 1 bütünleştikten sonra, alternatiflerden önce ve ayrı
commit'lerde. Alternatif ajanlar düzeneği **değiştiremez**; yalnız arayüzü uygular.

Ortak ilkeler:

- Her düzenek **belirleyicidir** (tohumlu; saat yok). Aynı alternatif + aynı tohum → bayt
  düzeyinde aynı ham kayıt. Yeniden puanlama (`score`) ham kayıttan aynı sonucu verir.
- Alternatif yalnız bir trait'i uygular (`qp_capsule::{Realizer, Ranker, ThemeRule,
  OutcomeRule, TrustPath}`, `qp_grammar::glyph::Alphabet`) ve düzeneğin kayıt listesine bir
  satır ekler. Ölçü yalnız alternatifin **çıktısından** hesaplanır; beyanına bakılmaz.
- Veri: (1) sentetik pazar (`qp-source` rejimli üreteç): 30 enstrüman × 500 işlem günü, üç
  tohum; sonuçlar rejime göre de kırılır. (2) `fixtures/local/` varsa gerçek günlük ve 60
  dakikalık barlar: ayrı tabloda, yalnız toplu istatistik; ham kayıtları `fixtures/local/`
  altına (commit edilmez) ya da yalnız toplu satırlarla `raw/` altına.
- Ortak başvuru yapılandırması (M3–M5 birbirine bağlı): ölçülmeyen iki strateji için
  düzeneğin **başvuru** stratejileri kullanılır (`harness::reference`), aday değildir, en
  sade biçimdedir ve raporda yazılır. Son bütünleştirmede (S) seçilenlerle yeniden koşulur.
- Ham kayıtlar `rounds/R3/experiments/<M>/raw/`; özet `rounds/R3/experiments/<M>/RESULTS.md`.

## M1 — İşaret alfabesi

Girdi: `Box<dyn Alphabet>`. Çizim: `qp-render`'ın CPU çizicisi (altın görsellerle aynı yol).

**Glif kümesi.** Çekirdek küme `K`: tür × yön = devam↑↓, dönüş↑↓, kırılım↑↓, sıkışma·,
aşırılık↑↓, akış↑↓· (12; ilk yazımda yanlışlıkla 13 yazılmıştı, düzenek 12 ile koşar), aşama `final`, güç 0,6, güven 0,6, teyit 1, tazelik 1. Kanal
kümeleri: aynı glifin yalnız bir kanalı değişen çiftleri: yön, aşama (5), güç (0,2 / 0,9),
teyit (1/2/3), güven (yok / 0,3 / 0,8), tazelik (1 / 0,3), sunucu işareti.

**(a) Hesapla ayırt edilebilirlik.** 16, 24, 32 px; açık ve koyu tema; görme benzetimi:
normal, döteranopi, protanopi, tritanopi (Machado 2009, şiddet 1,0), gri. Her glif RGBA →
CIELAB (D65); her piksel çiftinde ΔE76; σ = 0,5 px Gauss bulanıklığı (göz/ekran); ±1 px
kaydırmaların en küçüğü. Çift uzaklığı = ortalama ΔE. Rapor: `K` içinde en zayıf çift ve
ortalama; her kanal için en zayıf çift; koşul başına (boyut × tema × görme) tablo. Ana ölçü:
**en kötü koşulda `K`'nin en zayıf çifti**.

**(b) Okuma testi.** Görsel okuyabilen taze alt ajanlar (her ajan yalnız **bir**
alfabe görür). Sayfa: 4×4 ızgarada 16 glif, her biri 32 px çizilip 4× en yakın komşuyla
büyütülmüş (sınır: model küçük pikseli insan gibi görmez; raporda yazılır), A1…D4 etiketli;
glifler tohumlu seçilir (K'nin 12'si + kanal çeşitlemeleri 4). Ajan her etiket için JSON'la
cevaplar: `kind` (altı türden biri), `direction` (up/down/neutral), `strength`
(low/high), `phase` (candidate/final/realized/failed/retracted). İki koşul: **açıklamasız**
(yalnız "bu işaretler bir piyasa anlam alfabesidir; altı tür: …" listesi; işaretlerin
kuralları verilmez) ve **kısa açıklamalı** (alfabenin kendi en çok 120 kelimelik
açıklaması; alfabe ajanı yazar, düzenek uzunluğu denetler). Koşul başına 3 ajan.

**(c) Öğrenme.** Ajan önce 5 etiketli örnek görür (tohumlu; türlerin en az 4'ü), sonra 12
yeni glifi (örneklerde olmayan tür×yön×aşama birleşimleri dahil) cevaplar. 3 ajan.

Puan: alan başına doğruluk (tür, yön, güç, aşama) ve tam doğru oranı; şans düzeyi yazılır.
Ajan kimliği, tohum, istem metni ve ham cevap `raw/`'da. Bu bir **ön elemedir** (K39).

## M2 — Cümle üretimi

Girdi: `Box<dyn Realizer>` (dil modeli alternatifi: düzenek, alternatifin önceden ürettiği
cümle dosyasını aynı arayüzle okuyan bir sarmalayıcı kullanır; üretimin maliyeti ve
süresi alternatif tarafından ölçülüp kayda yazılır).

**Durum kümesi** (düzenek üretir, tohumlu, 72 durum): üç zaman türü × altı tür × aşama
(doğdu, teyit edildi, zayıfladı, gerçekleşti, geçersiz, süresi doldu) × `still_holds`
(evet/hayır/uygulanamaz) × `redacted` (lisanslı akış) × "kaç bar önce" (0, 1, 5, 30) ×
karne (n küçük / büyük). Ayrıca **K26 durumu**: RSI 3 bar önce 70'i yukarı kesti; şimdi
55 (`still_holds = false`, `current = 55`). Her durumun doğru cevabı (ne oldu, ne zaman,
hâlâ geçerli mi) düzenekte tanımlı.

Ölçüler, iki dilde (TR, EN):
1. **Zaman doğruluğu (K26), otomatik:** dil başına belirleyici sınıflayıcı: olay → geçmiş
   zaman eki/biçimi + zaman başvurusu; bitmiş durum → geçmiş + "artık … değil"/"no longer";
   süren durum → şimdiki zaman; beklenti → koşul (`-sa/-se`, "if"). Sözlükler
   `harness/src/m2/lexicon.rs`'de; ihlaller listelenir.
2. **Belirleyicilik:** aynı girdi iki kez → bayt eşit mi (oran).
3. **Sızıntı:** `redacted` durumlarda rakam (`[0-9]`), sayı sözcüğü (sözlük) ve lisanslı kanıt
   değerinin herhangi bir yuvarlanmış biçimi geçiyor mu. Sayısal olmayan durumlarda
   cümledeki her sayı motorun yuvalarından birine eşit mi (uydurma sayı).
4. **Anlama:** taze alt ajanlar (her biri yalnız bir alternatifin bir dildeki cümlelerini
   görür; 24 cümlelik tohumlu alt küme) her cümle için "ne oldu (tür+yön), ne zaman (kaç bar
   önce / şimdi / koşullu), hâlâ geçerli mi (evet/hayır/bilinmiyor)" cevaplar. Alternatif ×
   dil başına 2 ajan.
5. **Maliyet:** cümle başına süre (ns) ve dil modeli için çağrı başına süre ve belirteç.

## M3 — Önem sıralaması

Girdi: `Box<dyn Ranker>`. Başvuru tema ve sonuç kuralıyla `qp_capsule::simulate`.
Ölçüler (seviye = `important` kademe 1, `balanced` 1–2, `all`):
1. Seviye başına **günlük kapsül sayısı dağılımı** (yeni doğan + kademe süzgecinden geçen):
   ortalama, medyan, p90, en çok, sıfır gün oranı; **kararlılık**: günlük sayının değişim
   katsayısı, rejime göre ortalamaların oranı (en yüksek/en düşük), tohumlar arası fark.
2. **Aynı enstrümanda art arda:** seviye başına, aynı enstrümanda 5 işlem günü içinde
   ikinci kapsül oranı; en uzun ardışık gün dizisi.
3. **Sonradan gerçekleşenlerin payı:** kademe başına gerçekleşen / kapanan (geri çekilenler
   hariç); kademe 1 − kademe 3 farkı (sıralamanın bilgi taşıyıp taşımadığı); taban oran.
4. İleriye bakma: `RankCtx` yalnız geçmişten (düzenek bunu sınar: gelecek veri karıştırılınca
   geçmiş kademeler değişmez).

## M4 — Tema kimliği

Girdi: `Box<dyn ThemeRule>`. İki veri:
- **Etiketli bölüm kümesi:** düzenek sentetik şablonlardan bölümler dikerek seri kurar ve
  her olguya gerçek **bölüm kimliği** verir (ör. "kırılım + hacim teyidi + ertesi gün devam",
  "sıkışma → kırılım", aynı enstrümanda iki ayrı kırılım arası dönüş; 200 bölüm, tohumlu).
- Rejimli sentetik pazar (etiketsiz) ve varsa gerçek veri.
Ölçüler:
1. **Kapsül sayısındaki azalma:** her olgunun kendi hikâyesi olduğu tabana göre.
2. **Yanlış birleştirme:** aynı hikâyede farklı bölümlerden olgu çifti oranı (çift düzeyi
   kesinlik) ve bu tür hikâye sayısı.
3. **Bölünme:** aynı bölümün olgularının ayrı hikâyelere düşme oranı (çift düzeyi duyarlılık).
4. **Kaybolan bilgi:** `Ignore` edilen olgular; hikâyeye katılıp türü/yönü hikâyeninkinden
   farklı olduğu için cümle ve işarette görünmeyen olgular; düşen kanıt ailesi.

## M5 — Sonuç tanımı

Girdi: `Box<dyn OutcomeRule>`. Başvuru tema ve sıralayıcı.
1. **Kararlılık:** karne anahtarı başına isabet oranının örneklem büyüdükçe izi (n = 25, 50,
   100, 200, son); |p(n) − p(son)|; zamanın ilk yarısı ile ikinci yarısının oran farkı.
2. **Duyarlılık:** alternatif `perturbations()` ile kendi ana parametrelerinin ±%20'lik
   (ufuk) ve ±%10'luk (eşik) çeşitlerini verir; anahtar başına oran değişimi ve kapsül başına
   hüküm uyuşması.
3. **İleriye bakma sızıntısı:** (i) `judge`'a yalnız `bars[..=now]` verilir (düzenek
   dilimi korur); (ii) T zamanından sonraki barlar tohumlu karıştırılır → T öncesi kapanan
   hükümler ve T'deki karne bayt eşit olmalı; (iii) hüküm barı ≤ T için aynı.
4. **Anlamlılık:** aynı hikâyelerin yönü tohumlu ters çevrilmiş "yalancı" eşiyle oran farkı
   (tanım her şeye "gerçekleşti" diyorsa fark 0 çıkar); süresi dolan payı.

## M6 — Sunucu olgusu güven yolu

Girdi: `Box<dyn TrustPath>`. Veri: sentetik lisanslı akış (`qp-source`) + barlar.
1. **Kâhin eşdeğerlik saldırısı (en güçlü saldırgan, Kerckhoffs):** saldırgan algoritmayı
   ve **hedef bar dışındaki bütün ham değerleri** bilir. Hedef bar `b`'nin `net` değeri bir
   ızgarada (gerçek değerin ±%300'ü, 2.001 nokta) değiştirilir, güven yolu baştan koşulur;
   kullanıcıya giden ileti dizisi (kodlar, metinler, ekler, gösterim barı; `b` ve sonrası)
   gözlenenle **bayt eşit** kalan değerler kümesi = saldırganın aralığı. Ölçü: aralık
   genişliği / doğal yayılım (|net|'in p5–p95'i), sızan bit = log2(yayılım / genişlik);
   bütün bar ve enstrümanlarda dağılım (medyan, p10, en kötü). Aralık birden çok parçalıysa
   kapsayan genişlik ve parçaların toplamı ayrı yazılır.
2. **Gerçekçi saldırgan:** yalnız iletileri ve herkese açık barları görür; üretecin ön
   dağılımından 2.000 tohumlu aday dizi çeker, aynı iletileri üretenleri tutar; hedef
   değerin sonsal aralığı (p5–p95).
3. **Gösterim gecikmesi:** `shown_bar − bar` dağılımı.
4. **Dürüstlük:** her iletide `trust = server`; metin "sunucu doğruladı / server-verified"
   anlamını taşıyor mu (sözlük); hiçbir iletide rakam yok (otomatik).
5. **Fayda:** lisanslı olguların kaçında kullanıcıya en az bir ileti gitti (hiç iletmemek
   saldırıyı sıfırlar ama yorumu yok eder; K38'in amacı yorumun gitmesi).

## M7 — İtiraz ve geri çekme

Düzenek: RP2 tutarlılık düzeneği (`rounds/RP2/experiments/harness/`), değiştirilmeden
kopyalanır; yeni yaklaşım `bs2` = gerçek `qp-session` otoritesi + R3 itiraz/geri çekme yolu.
Senaryolar: S0–S19 + ekler + denetçinin AUD1–AUD3 spec'leri (`rounds/RP2/AUDIT.md`).
Ölçüler RP2 M1–M13; hedef: M1/M2/M3 (erken, çelişkili, yanlış) = 0 ve M5 (kaçan kesin)
`bs`'nin AUD1/2/3 = 7/49/7 değerinden belirgin az; tohumlar 0–99.
