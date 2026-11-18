# M21 — hareket: ölçüm sonucu

Ölçüm ajanı `r5/m21-measure`, 2026-11-18. Düzenek: `rounds/R5/experiments/harness/glance/`, SPEC §G +
**Ek G6** (M21; okumalardan önce commit `fca8f33`) + **Ek G6-8** (bilgi kaybı uygulama düzeltmesi ve
tanım boşluğu; okuyucu/eleştirmen sonuçlarından önce, kayıp sonuçları görüldükten sonra, `69d27cc`).
Ham özet (ekran görüntüsü yok): [`run/`](run/); bütün tablolar `run/tables.md` (`tables.py` ile üretildi).

> Ajan okuyucu insan değildir; bu ölçüm bir ön elemedir (SPEC §G0). Son söz Arkin'in telefondaki
> karşılaştırması ve insan testindedir.
>
> **Sınır:** görünüş henüz seçilmedi. Bütün adaylar `look=r4`, `voice=marla` ile ölçüldü; `surek` ve
> `nabiz` süre, renk ve boyu görünüşten okur, M16 seçimiyle sonuçlar değişebilir.

## Özet

| | `surek` | `nabiz` | `r4` (bugünkü) | `still` (hareketsiz) | `kestim` (bozuk) |
|---|---|---|---|---|---|
| Süreklilik: ara karede "aynı nesne taşınıyor" | 1,00 | 0,85 | 1,00 | **0,98** (ara kare = son kare) | 0,89 |
| Aynı soru **son kareyle** | 0,98 | 1,00 | 0,98 | 0,98 | 0,93 |
| Geçerlilik (§G6.4, Ek G6-1) | geçersiz | geçersiz | geçersiz | "geçerli" (yapay; aşağıda) | geçersiz |
| Fark edilme (`notice`, sınıf dengeli, F2:H1) | 0,297 [0,259, 0,336] | **0,515 [0,471, 0,559]** | 0,265 [0,226, 0,304] | 0,265 [0,228, 0,302] | 0,310 [0,271, 0,350] |
| Fark edilme, hareketi azalt hâli | 0,575 [0,527, 0,615] (`surek-az`) | 0,440 [0,397, 0,484] (`nabiz-az`) | (= `still`) | — | — |
| Eleştirmen BT (%95) | 0,57 [0,28, 0,89] | **1,72 [1,41, 2,12]** | 0,34 [0,03, 0,65] | 1,14 [0,85, 1,47] | −3,76 [−4,42, −3,14] |
| K52 JIT açık (bağlayıcı) | geçti | geçti | geçti | geçti | — |
| K52 JIT kapalı ("kullanılabilir", ≤ %10 uzun kare) | geçti (yüz değişiminde %1,7) | geçti | geçti | geçti | — |
| Hareketi azalt bilgi kaybı (ana: herhangi bir durağan ilke) | **0** | **0** | 0 | 0 (tanım gereği) | — |
| Aynı, G6-5 lafzı (yalnız metin) | 0 | **4** (`c_born`: nabız → canlı noktası metin değil) | 0 | 0 | — |

**Önceden yazılı kural mekanik olarak `still`'i "kazanan" çıkarıyor, ama bu bir yapaydır ve kullanılamaz**
(aşağıda "Seçim kuralı adım adım"):
1. **Süreklilik sorusu geçersiz** (§G6.4): okuyucular her tasarımda, son kareyle de, %93–100
   "aynı nesne taşınıyor" diyor. Soru geçişi değil, açılan kapsülde aynı metnin görünmesini ölçüyor.
   `m_where` hiçbir tasarımda puanlanamadı (p = 0,25 çeyreği hep son çeyrekle aynı ya da çizgiye yakın).
   `still`'in tek "geçerli" sorusu, anahtarı "kesme" olduğu ve okuyucular yine "taşınıyor" dediği için
   doğruluğu 0 olan sorudur; birincili 0,000 olan tek tasarım olarak kural onu öne koyuyor.
2. **Bozuk tasarım sınaması geçmedi** (Ek G3, §G8): `kestim` BT'de kesin son, ama birincili yok ve fark
   edilmede son değil (0,310; `r4` ve `still` 0,265). Ek G3'e göre bu koşunun ajan sıralaması seçim için
   kullanılmaz; düzenek düzeltilmeli.

**Bu yüzden ölçüm M21'de kazanan ilan etmiyor ("ayırt edilemedi / geçersiz soru").** Hesapla ölçülen
kapılar (K52, bilgi kaybı) geçerlidir ve hiçbir adayı elemiyor (lafzî metin kuralında `nabiz` elenir).
Orkestratörün kararı için iki açık, sonradan (post hoc) okuma aşağıda "Bağ çözme ve varsayılan"da:
geçersiz adım atlanırsa fark edilme ve BT ikisi de **`nabiz`**'i ayrık biçimde öne koyuyor; lafzî metin
kuralında `nabiz` elenir, `surek` ve `still` ayırt edilemez (varsayılan `surek`).

Maliyet: **≈ 13,4 $** (okuyucu 6 015 puanlanan çağrı 5,36 $, eleştirmen 479 çağrı 8,03 $); bütçe 15 $.
Kullanım sınırına takılan 2 654 okuyucu çağrısı cevapsız döndü; yanlış sayılmadı, çıkarılıp yeniden
koşuldu (`run/limited.txt`).

## Düzenek ve protokol (okumalardan önce sabitlendi: Ek G6)

**Tasarımlar ve derlemeler.** Dallar birleştirilmedi; her aday kendi dalının `git archive` kopyasından
(scratchpad) ayrı derlendi (`qp` release + `scripts/demo.sh` WASM paketi;
`CARGO_TARGET_DIR=target/wt/m21m-<ad>`):

| ad | kaynak | adres |
|---|---|---|
| `surek` | `r5/m21-surek` 055c302 | `anim=1&motion=surek&reduce=0` |
| `nabiz` | `r5/m21-nabiz` c1bbb9f | `anim=1&motion=nabiz` |
| `r4` (bugünkü hareket, `zumlak` açılışı) | `main` d8f83c5 + `broken/look.patch` | `anim=1` |
| `still` (hareketsiz) | aynı | `anim=0` |
| `kestim` (bozuk, gizli ad) | aynı | `anim=1&look=kestim` |
| `surek-az`, `nabiz-az` (bilgi) | aday dalı | aday adresi + Playwright `reducedMotion: reduce` (sistemin `prefers-reduced-motion`'ı) |

Hepsi `look=r4`, `voice=marla`, `canvas=run`, 402×874 @1×, duyarlılık `all`, 2 dil × 2 tema. İki aday dalı
`main`'den yalnız hareket kodu ve belgelerle ayrılıyor (`git diff --stat`); akış ve yüzler aynı.

**Açılış (süreklilik).** `glance.py probe/states` (main derlemesi): `live_event` = `cap.SYN06.breakout.up.118`
(adım 1), `closed_failed` = `cap.SYN26.breakout.down.47` (adım 1). Önce + p = 0,25 karesi (`window.qp.freeze`;
`still`'de dondurma yok, kare = ekran). Sorular `m_same`, `m_where` (§G4); `motion_final` aynı iki soruyu son
kareyle sorar (Ek G6-1).

**Durum değişimi (fark edilme).** Okurun bütün akışında adım 1–500 ardışık tarandı (`run/m21probe.json.gz`;
9 adımda bir bar, değişimler: 25 doğum, 11 güçlenme, 12 gerçekleşme, 9 tutmama, 2 süresi doldu, 2 artık
geçerli değil). Sabit küme (Ek G6-2): `c_born` 216→217, `c_strengthened` 225→226, `c_failed` 333→334,
`c_realized` 207→208, denetim `n_still1` 211→212, `n_still2` 300→301. Düzenek sayfanın görüntü karesi saatini
(rAF zaman damgası) eline alıp kareleri belirleyici çeker: `before`, `mid` (tasarımın değişim süresinin yarısı:
`surek` 180 ms, `nabiz` 450 ms, ötekiler 180 ms), `after` (1000 ms), `end` (5000 ms). Değişen yüz ekranda
ilk (`c_strengthened`, `c_realized`) ya da ikinci (`c_born`, `c_failed`) sıradadır; 7 tasarımın 112 değişim
çekiminin hepsinde adım uygulandı ve hedef ekrandaydı.

**Okuyucu:** haiku, personalar F ve H, `motion`/`motion_final`/`notice` 6 tekrar, `notice_pair` 3; sonnet
alt küme (`motion`, tr/açık, 1 tekrar) boylar arası uyum için. **Eleştirmen:** sonnet, üç kare
("0" başlangıç | A | B), `CRITIC_M21`, tr/açık, 8 durum × 10 çift × 2 sıra × 3 tekrar. Okuyucu ve eleştirmen
kör: rastgele adlı tek PNG, yasak sözcük denetimi (tasarım adları dahil), kayıtlar `run/*.jsonl.gz`. Örnek
istemler `run/prompts.md`. Tek bakış dışı (birden çok Read) çağrı: 5, çıkarıldı.

## Düzenek sınaması: son kareyle şans düzeyi (§G6.4, Ek G6-1)

| tasarım | `m_same` son kareyle [Wilson %95] | şans | sonuç | `m_where` |
|---|---|---|---|---|
| `surek` | 0,979 [0,927, 0,994] | 0,5 | son kareyle cevaplanıyor → **geçersiz** | puanlanamadı (anahtar yok) |
| `nabiz` | 1,000 [0,962, 1,000] | 0,5 | **geçersiz** | puanlanamadı |
| `r4` | 0,979 [0,927, 0,994] | 0,5 | **geçersiz** | puanlanamadı |
| `kestim` | 0,926 [0,856, 0,964] | 0,5 | **geçersiz** | puanlanamadı |
| `still` | 0,000 [0,000, 0,038] (anahtar "kesme") | 0,5 | tek yönlü "geçerli"; lafzî (aralık şansı içerir) **hayır** | puanlanamadı |

Okuyucunun gerekçeleri ("aynı SYN06 yazısı sağda büyüyerek ayrıntıya geçmiş") sorunun ne ölçtüğünü
gösteriyor: açılan kapsül aynı cümleyi taşıdığı için her ekran çifti "aynı nesne" okunuyor; ara karenin hiçbir
katkısı görünmüyor (hareketsizde ara kare zaten son kare ve pay yine 0,98). `m_where`: p = 0,25 anındaki
taşınan parçaların (`#fx`) ağırlık merkezi hiçbir tasarımda son çeyrekten farklı ve çizgilere %10'dan uzak
değil (bugünkü geçişte kuru koşuda da böyleydi, DRYRUN 3). **Sonuç: §G6.4'e göre M21 süreklilik sorusu
"geçersiz soru"dur; değiştirilmesi gerekir** (Öneriler 1).

## Süreklilik doğruluğu

| tasarım | `motion` hepsi (F2:H1) | ara karede "taşınıyor" payı | son karede "taşınıyor" payı | birincil (geçerli sorular) |
|---|---|---|---|---|
| `surek` | 1,000 [1,000, 1,000] | 1,00 | 0,98 | yok |
| `nabiz` | 0,860 [0,787, 0,927] | 0,85 | 1,00 | yok |
| `r4` | 1,000 [1,000, 1,000] | 1,00 | 0,98 | yok |
| `still` | 0,000 | 0,98 | 0,98 | 0,000 |
| `kestim` | 0,907 [0,847, 0,960] | 0,89 | 0,93 | yok |

Bilgi olarak: `nabiz`'in çapraz geçişinde ara karede okuyucuların %15'i "kayboldu, yerine başka bir şey
geldi" dedi (uçuş yok; tasarımın kendi DESIGN.md'si bunu bekliyordu). Bu, geçersiz bir sorunun içindeki tek
tasarım farkıdır; seçimde kullanılmadı.

## Fark edilme (durum değişimi; Ek G6-2)

Sınıf dengeli `n_where` ("az önce değişen kapsül hangi şeritte / yok"), `mid` + `after` tek kareler:

| tasarım | hepsi | yalnız `mid` | yalnız `after` | değişimde "değişen yok" deme | denetimde yanlış alarm | çift (önce \| mid) | `n_what` |
|---|---|---|---|---|---|---|---|
| `nabiz` | **0,515 [0,471, 0,559]** | 0,565 | 0,465 | 0,08 | 0,77 | 0,917 | 0,91 |
| `surek` | 0,297 [0,259, 0,336] | 0,339 | 0,256 | 0,26 | 0,79 | 0,888 | 0,88 |
| `r4` | 0,265 [0,226, 0,304] | 0,262 | 0,269 | 0,29 | 0,79 | 0,738 | 0,71 |
| `still` | 0,265 [0,228, 0,302] | 0,271 | 0,259 | 0,26 | 0,77 | 0,729 | 0,73 |
| `kestim` | 0,310 [0,271, 0,350] | 0,331 | 0,289 | 0,37 | 0,64 | 0,824 | 0,81 |
| `surek-az` (bilgi) | 0,575 [0,527, 0,615] | 0,580 | 0,569 | | | 0,881 | 0,85 |
| `nabiz-az` (bilgi) | 0,440 [0,397, 0,484] | 0,442 | 0,438 | | | 0,893 | 0,89 |

Durum başına (değişim sınıfında doğru şerit, denetimde "yok"; `mid`/`after`):

| durum | `surek` | `nabiz` | `r4` | `still` | `surek-az` | `nabiz-az` |
|---|---|---|---|---|---|---|
| `c_born` | 0,15 / 0,11 | **0,85 / 0,75** | 0,12 / 0,12 | 0,12 / 0,08 | 0,15 / 0,10 | 0,60 / 0,67 |
| `c_strengthened` | 0,15 / 0,08 | **0,92 / 0,56** | 0,19 / 0,10 | 0,10 / 0,15 | 0,81 / 0,83 | 0,62 / 0,60 |
| `c_failed` | **0,90** / 0,48 | **0,92** / 0,58 | 0,48 / 0,50 | 0,48 / 0,50 | 0,98 / 0,94 | 0,67 / 0,58 |
| `c_realized` | 0,38 / 0,48 | 0,29 / 0,33 | 0,44 / 0,44 | 0,44 / 0,35 | **1,00 / 0,92** | 0,25 / 0,23 |
| `n_still1` (denetim) | 0,04 / 0,08 | 0,06 / 0,04 | 0,06 / 0,00 | 0,06 / 0,08 | 0,08 / 0,06 | 0,04 / 0,08 |
| `n_still2` (denetim) | 0,44 / 0,29 | 0,35 / 0,46 | 0,29 / 0,48 | 0,40 / 0,38 | 0,40 / 0,56 | 0,42 / 0,40 |

Okuma:
- Okuyucular denetim karelerinde de %77–79 "bir şey değişti" diyor (güçlü cevap yanlılığı; sınıf dengeli puan
  bunu yarıya indirir). Bu yüzden ölçü, hareketin değişen kapsülü **gösterip göstermediğini** ayırıyor, sessiz
  ekranı tanımayı değil.
- `surek` doğumu ve güçlenmeyi işaretlemedi (`c_born`'da `changed()` boş: tasarımın "önceki akışın hepsinden yeni
  doğum" kuralı bu doğumu saymadı); `tutmadı`da ara kare güçlü (0,90), hareket bitince iz kalmıyor (`after`
  0,48 ≈ `r4`). Hareketi azalt hâlinin kalıcı etiketi ("az önce tutmadı/gerçekleşti/güçlendi") hareketin
  kendisinden **daha çok** fark ediliyor (0,575 > 0,297).
- `nabiz`'in yüksek puanının bir kısmı değişimden değil **canlılıktan** geliyor: `c_born` ve `c_strengthened`'da
  hedef canlı bir kapsül ve sürekli nabız onu işaretliyor; kapanan kapsülde (`c_realized`) nabız yok ve puan
  `r4`'ün altında (0,29). TR'de "gerçekleşti" etiketi cümlenin son satırıyla aynı sözcük ve etiket konumu
  okuyucuya ayırt edici gelmiyor olabilir (bilgi; ölçülmedi).
- Bugünkü hareket (`r4`) ile hareketsiz (`still`) fark edilmede aynı (0,265): bugün yüz değişimi
  canlandırılmıyor.

## Hareketi azalt bilgi kaybı (sert; Ek G6-5, G6-8)

| tasarım → azaltılmış | (a) son ekran durağan içerik kaybı | hareketli yüz (mid) | (b) ana: herhangi bir durağan ilke | (b) G6-5 lafzı: metin öğesi | (b) ilk uygulama (çıplak dizgi) |
|---|---|---|---|---|---|
| `r4` → `still` | 0 | 0 | 0 | 0 | 0 |
| `surek` → `surek-az` | 0 | 12 | **0** (etiket: "az önce tutmadı / gerçekleşti / güçlendi", EN "just failed / came true / strengthened") | 0 | 0 |
| `nabiz` → `nabiz-az` | 0 | 16 | **0** (etiket "geçersiz / güçlendi / gerçekleşti", EN "invalidated / strengthened / played out"; doğumda canlı noktası) | **4** (`c_born` × 4) | 6 |

`nabiz`'in `c_born` yüzündeki hareket doğumun vuruşu değil sürekli nabızdır (vuruş yok); taşıdığı olgu
"canlı"dır ve azaltılmış hâlde **canlı noktası** (dolu daire, `r4` yüzünde yok) olarak durur. Orkestratörün
tek yönlü tanımıyla kayıp yok; G6-5'in "metin (etiket)" lafzıyla 4 kayıp. İlk uygulamanın 6'sının 2'si
uygulama hatasıydı (TR "gerçekleşti" etiketi, satır kırılmasıyla tek başına duran cümle satırıyla aynı dizgi;
öğe karşılaştırmasıyla etiket (20, 21) konumunda var). Ayrıntı `run/loss.json`.

Ayrıca gözlem: `nabiz` doğumu da işaretlemedi (vuruş ve "doğdu" etiketi yok; tasarımın "görülenlerin en
yenisinden yeni" kuralı); iki aday da bu doğumu doğum saymadı.

## K52 (Ek G6-4)

Kalibre benzetim, 3 koşu ortancası. Koşular okuyucu çağrılarıyla eşzamanlı yapıldı (tutucu koşul).

| tasarım | `k52.mjs` JIT açık: tik / yüz / sahne p95, ilk kapsül, kaydırma p99 | geçişler (açılış, grafik, geri) uzun kare payı JIT açık / kapalı | oynatma 8 s, JIT açık / kapalı | yüz değişimi görünürken (ek), JIT açık / kapalı | kapı |
|---|---|---|---|---|---|
| `r4` | 1 / 1 / 1 ms, 198 ms, 19 ms | 0 / %0,49 (p99 23, en çok 41 ms) | 0 / 0 | 0 / 0 (p99 19) | geçti |
| `still` | 1 / 1 / 1 ms, 199 ms, 19 ms | 0 / %0,49 | 0 / 0 | — | geçti |
| `surek` | 1 / 1 / 2 ms, 193 ms, 19 ms | 0 / %0,59 (p99 23) | 0 / 0 | 0 / **%1,72 (p99 50, en çok 52 ms)** | geçti |
| `nabiz` | 1 / 1 / 2 ms, 212 ms, 19 ms | 0 / %0,39 | 0 / 0 | 0 / 0 (p99 20) | geçti |

Not: 8 s oynatma penceresinde (adım 120'den) ekranda yüz değişimi olmadı (`surek` `changed()` = 0); yüz
değişiminin maliyeti bu yüzden ek ölçümle (adım 225, 333, 207'de değişen kapsül görünürken bir adım, 1000 ms,
3 tekrar × 3 durum) alındı. `surek`'in JIT kapalı telefonda değişim başında 50 ms'lik kareleri var (önceki
yüzün başvuru çizicisiyle DPR 3'te piksele çevrilmesi; kendi DESIGN.md'si 6–8 ms ölçmüştü, JIT açık); eşik
(%10) içinde ama hareket tasarımlarında tek görünür maliyet bu.

## Eleştirmen BT

| tasarım | BT [%95] |
|---|---|
| `nabiz` | 1,72 [1,41, 2,12] |
| `still` | 1,14 [0,85, 1,47] |
| `surek` | 0,57 [0,28, 0,89] |
| `r4` | 0,34 [0,03, 0,65] |
| `kestim` | −3,76 [−4,42, −3,14] |

Çelişki oranları hepsi ≤ %17 (`nabiz`–`surek` 4/24); hiçbir çift "ayırt edilemedi" değil. Eleştirmen ara
karede üst üste binen kopyaları (`r4`'ün `zumlak`'ı, `surek`'in uçan parçaları) okunaklılıkta cezalandırıyor;
hareketsiz kare 2. sırada. Boylar arası uyum (haiku–sonnet, `motion`, tr/açık, n = 20): 0,85.

## Bozuk tasarımın yeri

- BT: **kesin son** (−3,76; aralığı herkesinkinin altında).
- Birincil (süreklilik): birincili yok (soru geçersiz) → "kesin son" koşulu sağlanamıyor.
- Fark edilme: 0,310 — `r4` ve `still`'in (0,265) **üstünde**, son değil. `kestim` bugünkü hareketle çizilen
  bozuk bir *görünüştür*; fark edilme hareket sinyalini ölçer ve sinyalsiz tasarımlar şans civarında
  toplanır, görünüşün bozukluğu onu aşağı itmiyor (yanlış alarmı bile daha az: 0,64).
- Bütün `motion` doğruluğu: 0,907, `nabiz`'in (0,860) üstünde, son değil.

**Ek G3 kabul kuralı: geçmedi.** Bu koşunun okuyucu sıralaması seçim için doğrulanmış sayılmaz. M21 için bozuk
tasarım bir bozuk *hareket* olmalı (Öneriler 2).

## Seçim kuralı adım adım (Ek G6-7)

1. **Sert eşikler.** K52 (JIT açık; geçişler ve oynatma JIT açık ≤ %1, kapalı ≤ %10): dördü de geçer. Bilgi
   kaybı: ana kuralda dördü de 0 → hepsi kalır. (Duyarlılık, G6-5 lafzı: `nabiz` 4 → **elenir**.) Hiçbir eşik
   kaldırılmadı.
2. **Birincil (süreklilik, geçerli sorular).** `surek`, `nabiz`, `r4`: geçerli soru yok → birincil yok.
   `still`: tek "geçerli" soru, birincil 0,000. Kural "birincili olanlar önce" dediği için ilk grup = {`still`}.
   **Bu adım geçersizdir:** `still`'in sorusu okuyucu yanlılığı yüzünden (herkese "taşınıyor") şansın altında
   kaldığı için geçerli görünüyor; kuralın amacı (sürekliliği gösterilmemiş tasarım ilk gruba giremez) tersine
   işliyor. §G6.4: "geçersiz soru".
3. (2b, 3 bu yüzden koşmadı; tek aday kaldı.)
4. Mekanik sonuç: "kazanan" `still`, varsayılan `still`, ilk iki `still`, `nabiz`. **Kullanılamaz**:
   birincil geçersiz (§G6.4) ve bozuk tasarım sınaması geçmedi (Ek G3). **Kazanan ilan edilmiyor.**

## Bağ çözme ve varsayılan (orkestratörün kararı için; açıkça sonradan)

Ölçüm kararı yok. Bütünleştirme için iki okuma, ikisi de geçersiz adım 2 atlanarak, kalan önceden yazılı
adımlarla (2b fark edilme → 3 BT → metin → ad):

| okuma | 1. adımdan kalan | 2b fark edilme ilk grubu | 3 BT | sonuç | ilk iki |
|---|---|---|---|---|---|
| **A (ana bilgi kaybı kuralı)** | `r4`, `still`, `surek`, `nabiz` | {`nabiz`} (0,515; aralığı ötekilerle örtüşmüyor) | `nabiz` de en üstte (1,72, ayrık) | **`nabiz`** | `nabiz`, `surek` |
| B (G6-5 lafzı: yalnız metin) | `r4`, `still`, `surek` | {`surek`, `r4`, `still`} (örtüşüyor) | {`still`, `surek`} (örtüşüyor; `r4` dışarıda) | ayırt edilemedi; varsayılan (fark edilme noktası) **`surek`** | `surek`, `still` |

Okumada çekinceler: A'da `nabiz`'in fark edilme üstünlüğünün önemli bir kısmı canlı kapsüllerdeki sürekli
nabızdan geliyor (kapanma değişiminde `r4`'ün altında); BT'de üstünlüğü ara karede kopya üst üste binmemesinden
(okunaklılık). B'de `surek`'in ana katkısı hareketi azalt etiketinin kendisi.

## Maliyet

| kalem | çağrı | $ |
|---|---|---|
| okuyucu haiku (`motion`, `motion_final`) | 952 | 0,98 |
| okuyucu haiku (`notice`, `notice_pair`; 2 654'ü sınır sonrası yeniden) | 5 040 | ≈ 4,08 |
| okuyucu sonnet (uyum alt kümesi) | 20 | 0,30 |
| eleştirmen sonnet | 479 (+1 tek bakış dışı) | 8,03 |
| **toplam** | | **≈ 13,4** |

## Sorunlar

1. **Süreklilik sorusu geçersiz** (yukarıda). `m_same` bütün ekran çiftlerinde "aynı nesne" okunuyor;
   `m_where` puanlanamıyor. Ayrıca `m_where`'in seçenekleri harfle, bölgeler de A–D harfiyle adlandırılmış:
   okuyucu "A, ... A'da duruyor" yazıp karışık harf eşlemesiyle başka bir koda düşebiliyor (M18 `c_live`'da
   bu yüzden numaralı bölgeye geçilmişti). Puanlanmadığı için sonuca etkisi yok.
2. **Bozuk tasarım sınaması geçmedi** (Ek G3). Görünüş bozukluğu hareket ölçülerini bozmuyor.
3. **Okuyucu cevap yanlılığı fark edilmede güçlü** (denetimde %77–79 yanlış alarm); sınıf dengeli puan bunu
   düzeltir ama ölçünün çözünürlüğü düşük.
4. Bilgi kaybı (b) ölçüsü tanımı sonradan iki noktada düzeltildi (Ek G6-8); karar iki okumada farklı
   (`nabiz` kalır / elenir).
5. K52 koşuları okuyucu çağrılarıyla eşzamanlıydı; sayılar tutucu. 8 s oynatma yüz değişimi içermedi; ek
   ölçümle kapatıldı.
6. SPEC'te "Ek G5" yok (görev Ek G4/G5 dedi); main'deki son ek G4'tü, M21 eki G6 olarak yazıldı.
7. Kullanım sınırı: 2 654 okuyucu çağrısı cevapsız döndü, yeniden koşuldu; yeniden koşuda sınır yok (denetlendi).

## Öneriler (kapsam dışı)

1. **Süreklilik sorusu yeniden tasarlanmalı**, son kareyle cevaplanamayacak biçimde: ör. ara karede hedefin
   *işareti* nerede (numaralı bölge; son karede işaret başka yerdeyken), ya da iki farklı kapsülün açıldığı iki
   ara kareden "hangisi soldaki üst kapsülden geliyor". Okuyucu istemi "aynı nesne"yi metin benzerliğinden
   değil konumdan sormalı.
2. **M21 için bozuk hareket**: düzenek tarafında (ürüne girmeden) yanlış yere uçan ya da değişen olmayan
   kapsülü vuran bir geçiş/vuruş; bozuk görünüş yerine o yarışmalı.
3. Fark edilmede denetim karelerinin payı artırılmalı ve canlılık sinyali değişim sinyalinden ayrılmalı (hedefi
   canlı olmayan doğum/güçlenme durumu, ya da ekranda başka canlı kapsül varken).
4. `surek`: hareketi azalt etiketi hareketten daha çok fark ediliyor; etiketin hareket kipinde de kısa süre
   kalması düşünülebilir. JIT kapalı telefonda değişim başı 50 ms (önceki yüzü yalnız değişen kutularla çizmek).
5. İki aday da `c_born`'u doğum saymadı; "doğum" sınıflaması ortak bir çekirdek kuralına alınabilir.
6. Sözlüğe (adaylardan): nabız, vuruş, iz, canlı noktası, değişim etiketi — seçimden sonra.

## Ek (2026-11-18, orkestratör): bütünleştirme için geçici varsayılan

Önceden yazılı kural geçerli bir kazanan çıkarmadı (süreklilik sorusu geçersiz; bozuk tasarım fark edilmede son
değil). Kazanan ilan edilmez: M21 **ayırt edilemedi / ölçüm geçersiz**. Gösterimin bir varsayılana ihtiyacı olduğu
için yalnız sonuçlardan ÖNCE yazılmış kurallar uygulandı: Ek G6-5'in lafzı (hareketi azalt'ta bilgi, metin olarak
durur) `nabiz`'i doğum değişiminde eler (nabız → metinsiz canlı noktası); `surek` bütün kapıları geçer (K52, bilgi
kaybı 0). **Geçici varsayılan `surek`; ikinci `nabiz`** (fark edilme ve eleştirmende önde; metin eksiği küçük bir
düzeltmeyle giderilebilir); hareketsiz başvuru `?anim=0`. Son söz insan testi v3'tedir (G8). Ek G6-8'in "herhangi bir
durağan ilke" okuması kayıp sonuçları görüldükten sonra yazıldığı için seçimde kullanılmadı, duyarlılık olarak durur.

## Ek düzeltme (2026-11-18, bağımsız denetim koşulu 4)

Önceden yazılı G6-5 lafzıyla ilk iki **`surek` ve hareketsiz (`still`)**'dir; `nabiz` bu lafızla elenir. `nabiz`
gösterimde seçilebilir kalır (Arkin'in karşılaştırması için) ama ölçüme dayalı ikinci aday değildir: sıralaması
geçersiz sayılan ajan okumasından gelir, bütünleştirmede eklenen "canlı" etiketi sonradan bir düzeltmedir ve yeniden
ölçülmedi. Ek G6-8'in "ana karar herhangi bir durağan ilke" cümlesi kayıp sonuçlarından sonra yazıldığı için seçimde
kullanılmaz; yalnız duyarlılıktır.
