# M17 — `tunca`: çok kısa başlık + taranabilir katmanlar

Dal `r5/m17-tunca`. Ürün kodu `crates/qp-capsule/src/voice/tunca.rs` (+ `tunca_glossary.rs`),
düzenek bağdaştırıcısı `rounds/R5/experiments/harness/lang/tool/src/adapters/tunca.rs`,
tarama ve tablo `rounds/R5/experiments/harness/lang/results/tunca/`.

## İlke

Başlık birkaç sözcükle **ne olduğunu** söyler ve eşiğin adını taşır. Somut bilgi (eşiğin adı ve değeri,
zaman, geçerlilik, karne) kısa, **sabit sıralı**, her biri kendi başına anlamlı parçalar halinde gelir:
`etiket: değer · değer · değer`. Göz parçaların üstünden geçer; hiçbir parça başka bir parçayı okumadan
anlaşılmaz değildir. Ayrıntı, karne ve sözlük aynı düzeni izler.

| Yüzey | Biçim (sıra sabit) |
|---|---|
| Başlık | `{başlık} · {ne zaman} · {geçerlilik}` — başlık + en çok iki küçük ek (K43) |
| Ayrıntı | `Eşik: … . Zaman: … · … . Durum: … · … .` — üç katman, her biri bir cümle |
| Karne | `Karne: {n}'de {r} tuttu · {süre} içinde {beklenen hareket}` (az örnekte etiket `Az örnek`) |
| Sözlük | ekranın üç terimi, sabit sırayla: dedektörün kavramı → karne → geçerlilik sözcüğü |

## Örnekler (sentetik döküm; `results/tunca/TABLE.md` tamamı)

**Kırılım, doğdu, geçerli, karne 20 örnek** (s117)
- TR: `SYN20 son 20 günün zirvesini aştı (227,08) · bugün · hâlâ geçerli`
  — `Eşik: son 20 işlem gününün en yükseği, 227,08. Zaman: bugün başladı. Durum: hâlâ geçerli · son kapanış 229,81 · bu cihazda sınandı.`
  — `Karne: 20'de 11 tuttu · 10 işlem günü içinde yükseliş`
- EN: `SYN20 broke above its 20-day high (227.08) · today · still holds`
  — `Threshold: the high of the last 20 trading days, 227.08. Timeline: began today. Status: still holds · last close 229.81 · checked on this device.`
  — `Record: 11 of 20 played out · rise within 10 trading days`

**RSI aşırılık, zayıfladı, artık geçerli değil, az örnek** (s040)
- TR: `SYN29 hızlı yükseldi: RSI 70'i aştı · cuma günü · artık geçerli değil`
  — `Eşik: 70 çizgisi (14 günlük RSI). Zaman: cuma günü başladı · bugün zayıfladı. Durum: artık geçerli değil · RSI şu an 68,5 · bu cihazda sınandı.`
  — `Az örnek: 2'de 2 tuttu · 10 işlem günü içinde yükselişin sürmesi`
- EN: `SYN29 rose fast: RSI crossed above 70 · on Friday · no longer holds` — `Few cases: 2 of 2 played out · further rise within 10 trading days`

**Ortalama dizilimi, kapandı: gerçekleşti** (s150)
- TR: `SYN06 düşüş eğilimine girdi · 2 işlem günü önce · gerçekleşti`
  — `Eşik: 20 günlük ortalama 96,07 · 50 günlük ortalama 99,78. Zaman: 2 işlem günü önce başladı · bugün kapandı. Durum: gerçekleşti · fiyat 20 işlem günü içinde düştü · bu cihazda sınandı.`
- EN: `SYN06 entered a downtrend · 2 trading days ago · played out`

**Uyumsuzluk, kapandı: tutmadı** (s107)
- TR: `SYN25'te tepeden aşağı dönüş izi · 3 işlem günü önce · tutmadı`
  — `Eşik: önceki tepe 208,11 (RSI 79,8) · yeni tepe 227,07 (RSI 73,1). …`
- EN: `SYN25: a turn-down sign at a new high · 3 trading days ago · did not play out`

**Sıkışma, geçerli** — TR `SYN07 dar bir aralığa sıkıştı · bugün · hâlâ geçerli`
— `Eşik: 20 günlük bant 120 işlem gününün en darı · bant 7,78–8,12. …` — `Karne: henüz örnek yok · 20 işlem günü içinde aralığın kırılması`;
EN `SYN07 settled into a narrow range · today · still holds`.

**Hacim, geri çekildi (kurgu)** (s523) — TR `SYN11 ortalamanın 4,99 katı hacimle düştü · 2 işlem günü önce · geri çekildi`
— `Eşik: 20 günlük ortalama hacmin 2,5 katı · o gün 4,99 katı. Zaman: … · bugün kapandı. Durum: geri çekildi · dayandığı veri düzeltildi · bu cihazda sınandı.`
EN `SYN11 fell on 4.99× its average volume · 2 trading days ago · withdrawn`.

**Kurumsal akış, yetkili** (s007) — TR `SYN25'te kurumsal net alış 2 katı aştı · 3 işlem günü önce · hâlâ geçerli`
— `Eşik: kurumsal net alışta 5 günlük ortalamanın 2 katı · ortalama 7.870.125. Zaman: 3 işlem günü önce başladı · bugün güçlendi. Durum: hâlâ geçerli · sunucu doğruladı.`

**Kurumsal akış, yetkisiz (rakamsız, A32)** (s008)
- TR: `SYN25'te kurumsal net alış · birkaç işlem günü önce · hâlâ geçerli`
  — `Eşik: kurumsal net alışta beş günlük ortalamanın iki katı. Zaman: birkaç işlem günü önce başladı. Durum: hâlâ geçerli · sunucu doğruladı · rakamlar kurumsal akış lisansıyla açılır.`
  — `Karne: henüz örnek yok · on işlem günü içinde yükseliş`
- EN: `SYN25: institutional net buying · a few trading days ago · still holds`
  — `… Status: still holds · verified by the server · figures need a flow licence.`

## Bilinçli seçimler

1. **Başlıkta eşiğin adı, sığarsa değeri.** "önemli bir eşik" yerine "son 20 günün zirvesi (227,08)",
   "RSI 70", "ortalamanın 4,99 katı hacim", "kurumsal net alış 2 katı". Başlık önce sade fiili söyler
   ("hızlı yükseldi", "düşüş eğilimine girdi", "dar bir aralığa sıkıştı") — teknik analiz bilmeyen okur
   için; terim (RSI) iki noktadan sonra gelir ve sözlükte açılır.
2. **Başlık adayları uzundan kısaya;** sınırı (80) aşan aday bırakılır. Seçim **iki dilde birlikte** yapılır:
   TR'ye sığan ama EN'e sığmayan değer TR'de de düşer, böylece iki dil aynı sayıları taşır (eşdeğerlik).
   Ayrıntıda da aynı: isteğe bağlı parçalar önceliğe göre ve iki dilde aynı sayıda düşer (önce şimdiki değer,
   sonra eşik katmanındaki ham sayılar, sonra kapanış nedeni, en son aşama değişimi). Eşiğin adı ve değeri,
   doğum zamanı, geçerlilik ve güven işareti hiç düşmez.
3. **Zaman takvimle.** Bütün zamanlar R4 süre ifadesinden (`dün`, `cuma günü`, `3 işlem günü önce`,
   `dün 15:00'te`); pencereler ve ufuk da bar sayısı değil takvim birimi (`son 20 işlem günü`, `10 işlem saati`).
4. **Karne beklentinin kendisini yazar:** sayı (`20'de 11 tuttu`) + süre + yön (`10 işlem günü içinde yükseliş`;
   RSI'da `yükselişin sürmesi`, yönsüzde `aralığın kırılması`). Bu, ürünün sonuç kuralının (oynaklık bariyeri)
   dürüst özeti; "tuttu"nun tam anlamı sözlükte. Karnesizde de beklenti yazılır (`henüz örnek yok · …`).
5. **Ayrıntı yüzün yapı taşlarından kurulur** (A33): geçerlilik `Parts::validity_phrase`, zamanlar
   `Parts::when / moved`, aynı değer yüzde ve ayrıntıda. Çelişki taraması 0.
6. **İç terim yok.** "teyit" yerine "{veri} da aynı yönü gösterdi", "aday" yerine "dönem kapanmadı, henüz kesin
   değil", "sinyal" hiç yok. "destek/direnç/kırılım/sıkışma" gibi teknik sözcükler de yüzeyde kullanılmıyor
   (yerine zirve/dip/dar aralık); kalan teknik terimler (RSI, bant, hacim, kurumsal, karne) ekranın sözlüğünde.
7. **Dürüstlük:** her ayrıntıda `sunucu doğruladı` ya da `bu cihazda sınandı`; az örnekte karne etiketi
   `Az örnek`; yetkisiz okurda hangi lisansın rakamları açtığı yazılır.
8. **Yetkisiz okur (K33/K34, A32):** hiçbir yüzeyde rakam yok (sözlük dahil). Dedektörün **herkese açık**
   kuralı (5 dönem, 2 kat, 10 günlük ufuk) yazıyla verilir ("beş günlük ortalamanın iki katı"); lisanslı
   değerler (net, ortalama) hiç verilmez; zaman belirsizleşir, karne niteldir (`çoğunlukla tuttu`).
   Sayıyı yazıyla vermek lisanslı veri sızdırmaz; ama "rakam görmez" kuralının yorumu Arkin'in onayına açıktır.
9. **Hiçbir cümle eylem önermez;** fiiller olayı ve sonucu anlatır.

## Giriş noktası

```rust
qp_capsule::voice::tunca::speak(&VoiceInput { sentence: &SentenceInput, parts: &Parts, period: Period }, Lang) -> Voice
// Voice { headline, detail, ledger, glossary: Vec<Gloss { term, text }> }
```

`Parts`, otoritenin zaten kurduğu başlık yapı taşlarıdır (`qp_capsule::realize::parts::parts`). Veri eki gerekmedi:
eşik değerleri öncü olgunun kanıtında, parametreler dedektör sabitlerinde, ufuk hikâyede.

## Kendi sınamam

13 tarama, 531 durum × 2 dil: hepsi **0** (`results/tunca/SCAN.md`). Uzunluk (karakter) ortalama / en çok:
başlık TR 65,5 / 80, EN 67,7 / 80; ayrıntı TR 163,8 / 199, EN 175,0 / 200; karne TR 54,6 / 65, EN 57,0 / 67.

## Bilinen zayıflıklar

- EN uzun olduğu için iki dilin birlikte sığdırılması TR'de de parça düşürür: kapanmış kurumsal akışta
  başlıktaki "2 katı" ve bazı kapanmış durumlarda kapanış nedeni ("fiyat ters yöne gitti") düşüyor.
- Sınırlara yakın: ayrıntı sınıra 1 karakter kalıyor; uzun enstrüman adlarında (ör. 6+ harf) daha çok parça düşer.
- "hâlâ geçerli" koşulsuz olaylarda (hacim, uyumsuzluk, kurumsal) "iddia henüz kapanmadı" anlamındadır;
  sözlük bunu söylüyor, ama okuyucu ölçüsü gösterecek.
- Okuyucu/eleştirmen ölçümünü koşmadım (tarif gereği orkestratörün işi).
