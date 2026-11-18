# Puanlama (önceden yazılı; hiçbir katılımcıdan önce)

Bu kurallar oturumlardan önce commit edilir. Sonuç görüldükten sonra bir kural değişirse
değişiklik, nedeni ve eski kuralla sonucun ne olacağı `results.md`'de açıkça yazılır.
Kararın kendisi `DECISION.md`'dedir; burası ölçülerin nasıl hesaplandığını söyler.

## 1. Genel

- **Cevapsız, `?` ya da ipuçlu** cevap yanlış (0). "Atlandı" eksiktir: o madde hesaba girmez,
  sayısı raporlanır.
- **Geçersiz set** (iki set aynı görünüyor, yanlış kod, katılımcı büyüttü / örtüyü kaldırdı ve
  yürütücü geçersiz dedi): o katılımcının **o karşılaştırması** atılır (eşli fark kurulamaz); sayısı
  raporlanır.
- **Puan birimi:** katılımcı × aday için o görevdeki maddelerin doğruluk ortalaması (0–1).
  **Eşli fark** = aday 1 − aday 2, katılımcı başına.
- **Ağırlık:** genç 2, alışkın 1 (NORTH_STAR birincil kullanıcısı; R4 v2 ve ajan personaları F 2,
  H 1 ile aynı). Ağırlıklı ve ağırlıksız ortalama ikisi de raporlanır; karar ağırlıklıyla.
- Bütün oranlar **sayıyla birlikte** yazılır ("7/9"), yalnız yüzde değil.

## 2. Görev başına doğruluk (birincil ölçü kalın)

| Görev | Doğru sayılan | Birincil | Ayrıca raporlanan |
|---|---|---|---|
| G1 yüz | `dir`, `kind`, `valid` anahtarla; `when` takvim günü **ya da** işlem günü kovası (ikisi farklıysa ikisi de doğru; ajan düzeneğiyle aynı, glance SPEC §G4); `trust` doğumdaki karne (n < 20 → az örnek; oran ≥ 0,6 / 0,4–0,6 / < 0,4); `advice` yalnız 4 | **yüz başına altı alanın ortalaması**, sonra iki yüzün ortalaması | tam doğru yüz oranı; alan başına; **tavsiye okuması** (al ya da sat seçimi sayısı, aday başına); kısma sorusu: ilk çarpan **işaret, yön ya da olay** mı (1) yoksa başka bir şey mi (sayı, ad, renk, süs; 0) — iki kişi ayrı kodlar, uyuşmazlık raporlanır |
| G2 dil | `l_what`, `l_when`, `l_valid`, `l_advice` anahtarla; `l_level` eşiğin adı **ve** değeri (tolerans `key.md`); `l_ledger` örnek sayısı **ve** tutan sayısı (örnek yoksa "henüz yok" ya da 0) | **altı sorunun ortalaması** | `l_level` ve `l_ledger` ayrı (K49'un somutluk iddiası bunlarda); okuma süresi; `l_same` dağılımı (A33); sözlük: `g_found`, `g_right`, `g_taps` ortancası |
| G3 alfabe | işaret başına `dir`, `kind`, `valid` | **işaret başına tam doğru** (üçü birden), üç işaretin ortalaması | alan başına; set süresi |
| G4 açılmış kapsül | `e_rests`: ailelerin tamamı 1, eksik ya da fazla ½, hiç örtüşmeyen 0; `e_who`, `e_valid` anahtarla | **üç sorunun ortalaması** | soru başına süre ve dokunma ortancası; `e_who` set 1 (sunucu) ve set 2 (cihaz) ayrı; yetkisiz: `v_trust`, `v_why`, `v_who` (A32; karşılaştırma dışı da raporlanır) |
| G5 hikâye çizelgesi | `c_recent`, `c_held` anahtar kovası; `c_live` yürütücünün işareti; `c_evid` odak doğru işarete taşındı **ve** aile doğru 1, yalnız biri ½ | **dört sorunun ortalaması** | soru başına süre ve dokunma ortancası; `c_live` süresi (GOAL M18 "canlıyı bulma süresi") ayrı |
| G6 akış sırası | `f_new`: hisse düzeyinde F1 (söylenen hisseler ↔ `key.md`'de t0–t1 arası değişen ve akışta görünenler); `f_neg`: hisse doğru **ve** ne doğru 1, yalnız hisse ½ | **`f_new` F1 ile `f_neg` ortalaması** | soru başına süre ve kaydırma ortancası; `f_neg`'i bulamayan sayısı |
| G7 yazı tura eşiği | `r_best` yürütücünün işareti; `r_few` anahtarla | **`r_trust` (1–5)** eşli farkı | `r_best`, `r_few` doğruluğu; boş akış `x_empty` doğru oranı; T'deki `k53_want` dağılımı |
| G8 hareket | `m_seen` = 1 **ve** `m_what` doğru → fark etti (1); `m_same` = 1 | **fark etme** (1. ve 2. set) | `m_same` oranı, `m_lost` ortancası; **hareketi azalt bilgi kaybı**: 3. setteki fark etme ile aynı katılımcının 1.–2. setteki aday-1 fark etmesi arasındaki fark (DECISION M21) |

## 3. Süre ve dokunma

- Katılımcı × aday için görevdeki madde sürelerinin **ortancası**; eşli fark (aday 1 − aday 2);
  raporda farkların ortancası ve en küçük–en büyük değeri.
- Süre **yalnız doğru cevaplanan** maddelerde hesaplanır (yanlış cevap hızlı olabilir); doğru
  maddesi olmayan katılımcı o görevin süre karşılaştırmasına girmez.
- Dokunma ve kaydırma sayısı aynı biçimde (ortanca, eşli fark).

## 4. Tercih

- T'deki "1 ilk / 2 ikinci" katılımcının sıra satırından adaya çevrilir.
- **Tercih oranı** = adayı seçenler / tercih belirtenler ("fark yok" paydadan çıkar). "Fark yok"
  sayısı ayrıca yazılır.
- Nedenler iki kişi tarafından kısa etiketlere (okunurluk, kalabalık, renk, hız, güven, başka)
  ayrılır; etiket sayıları raporlanır, alıntılar kişiyi tanıtmadan.
- **Sıra etkisi:** tercih edilen setin 1. mi 2. mi olduğu ayrıca sayılır (her karşılaştırmada ve
  toplamda); 1. ya da 2. sete belirgin eğilim (ör. 10 kişide 8/10) raporda uyarı olarak yazılır.

## 5. Küçük örnekte belirsizlik — neyin anlamlı OLMADIĞI

5–10 kişiyle bu test **istatistiksel anlamlılık üretmez**. Karar kuralı (`DECISION.md`) bir
**uzlaşım**dır: önceden yazıldığı için keyfi değildir, ama "kanıtlandı" demez. Raporda şu sınırlar
aynen yazılır:

| Durum | Hesap | Sonuç |
|---|---|---|
| İşaret testi, n = 5 | 5/5 aynı yönde → iki yönlü p = 0,0625; 4/5 → 0,375 | n = 5'te **hiçbir sonuç** p < 0,05 olamaz |
| n = 6 | 6/6 → 0,031; 5/6 → 0,219 | yalnız oybirliği |
| n = 8 | 8/8 → 0,008; 7/8 → 0,070; 6/8 → 0,289 | |
| n = 10 | 9/10 → 0,021; 8/10 → 0,109; 7/10 → 0,344 | 7/10 "çoğunluk" bile şanstan ayrılmaz |
| Tercih oranı, Wilson %95 | 4/5 [0,38; 0,96] · 5/6 [0,44; 0,97] · 6/8 [0,41; 0,93] · 8/10 [0,49; 0,94] · 9/10 [0,60; 0,98] | 9/10'un altındaki hiçbir oran 0,5'i aralığın dışına itmez |

Bu yüzden:

1. **Doğruluk farkı:** ağırlıklı eşli fark mutlak değerce **< 0,10** ise ya da aynı yönde daha
   iyi olan katılımcı sayısı öteki yöndekinden **en az 2 fazla değilse** → **"ayırt edilemedi"**.
   (0,10 ≈ bir görevde altı alandan biri; tek katılımcının tek maddesine dayanmasın diye işaret
   şartı.) Bu iki şartı geçen fark "insan testinde yön: aday X" diye yazılır, "anlamlı" denmez;
   işaret testi p değeri yanında verilir.
2. **Tercih:** ⅔'ün ve en az 3 kişinin altı → "ayırt edilemedi". Geçse de Wilson aralığı yanında
   yazılır ve 0,5'i içeriyorsa "zayıf" etiketi alır.
3. **Süre:** farkların ortancası **< 2 sn** ya da aday ortancalarının büyüğünün **%20'sinden
   küçükse** yorumlanmaz. Süre, doğruluk farkı "ayırt edilemedi" iken yalnız ikincil kanıttır.
4. **Persona ayrımı:** persona başına 2–5 kişi yalnız betimseldir; "gençler X'i, alışkınlar Y'yi
   seçti" ancak iki personada da kural 1'in şartları ayrı ayrı sağlanırsa yazılır, yoksa
   "persona farkı gözlenmedi / betimsel olarak şöyle" denir.
5. **Ajanla uyum:** insan sonucu "ayırt edilemedi" ise bu ajan sonucunu **ne doğrular ne
   çürütür**; tabloda "insan: ayırt edilemedi" yazılır.
6. **Mutlak eşikli kurallar** (A32, A33, boş akış, hareketi azalt; `DECISION.md`): kişi sayısıyla
   yazılmıştır ("en çok 1 kişi"), oranla değil; n = 5'te bir kişi %20'dir.
7. **Çoklu karşılaştırma:** sekiz karşılaştırma ve onlarca ikincil ölçü var; ikincil bir ölçüde
   tek başına görülen fark karar için kullanılmaz, yalnız "sonraki ölçüm için gözlem" olarak yazılır.

## 6. Renk görme

- "var" beyan eden kişi sayısı yazılır (yalnız sayı).
- 1 kişi: puanı yazılmaz; yalnız G1/G3'te anlam renklerinden kaynaklandığı **gözlenen** karışıklık
  var mı (ör. "kapanmış ile canlı işaret karıştırıldı") nitel olarak, kişiyi belli etmeden.
- ≥ 2 kişi: bu grubun G1 ve G3 doğruluğu (toplu) öteki katılımcıların en küçük–en büyük aralığında
  mı, altında mı. Altındaysa adaydan bağımsız bir erişilebilirlik bulgusu olarak yazılır ve
  M16 kararına "renk körlüğü benzetiminin insanda doğrulanmadığı" notu düşülür.

## 7. Rapor (`results.md`) iskeleti

1. Katılımcılar: persona başına n, dil, gösterim türü, renk görme sayısı, atılan set sayısı.
2. Her karşılaştırma: birincil (ağırlıklı ve ağırlıksız), işaret sayısı (+/−/=), işaret testi p;
   süre ve dokunma; tercih oranı ve Wilson aralığı; sıra etkisi; kural sonucu.
3. Mutlak kurallar (A32, A33, boş akış, hareketi azalt, tavsiye okuması).
4. Ajan sonuçlarıyla karşılaştırma tablosu (aşağıda).
5. Karar bölümü (`DECISION.md` kuralının uygulanışı) ve "Sınırlar".

## 8. Ajan sonuçlarıyla karşılaştırma tablosu (şablon)

Ajan sütunları ilgili `RESULTS.md`'den kopyalanır; insan sütunları bu testten. "Uyum": **aynı**
(ikisi aynı adayı ayırt etti), **ters** (farklı adaylar), **insan ayırt edemedi**, **ajan ayırt
edemedi**, **ikisi de ayırt edemedi**.

| Konu | Adaylar (1 / 2) | Ajan: ölçü, sonuç [aralık] | Ajan kararı | İnsan: birincil eşli fark (ağırlıklı), +/−/= | İnsan: tercih (1 : 2 : fark yok) | İnsan kararı (kural adımı) | Uyum |
|---|---|---|---|---|---|---|---|
| M16 görünüş | `<GÖRÜNÜŞ-1>` / `<GÖRÜNÜŞ-2>` | | | | | | |
| M17 dil | `marla` / `selvi` | birincil 0,889 [0,856; 0,915] / 0,898 [0,866; 0,924]; BT 1,41 / 1,69 | ayırt edilemedi; kısa olan `marla` (SPEC §5 adım 4) | | | | |
| A21 alfabe | `siluet` / `rozet` | — (R3 v1 / Ö32) | `siluet` ön seçim | | | | |
| M20 açılmış kapsül | `<OPEN-1>` / `<OPEN-2>` | | | | | | |
| M18 hikâye çizelgesi | `pavka` / `korda` | `chart` doğruluğu 0,987 / 0,928; BT 3,82 / 1,37 | `pavka` (son düzeltmeden sonra; RESULTS ve sonraki commit) | | | | |
| M19 akış sırası | `grouped` / `live_new` | persona görev ortalaması 0,881 / 0,727 (muaf) | `grouped` | | | | |
| K53 eşik | `rate55` / `none` | kalan Brier becerisi 0,024 [−0,000; 0,047] / 0,019 [0,003; 0,036] | ayırt edilemedi; eşitlik bozucu ile `rate55` | | | | |
| M21 hareket | `<MOTION-1>` / `<MOTION-2>` | | | | | | |

Ajan sayıları burada tablonun nasıl doldurulacağını göstermek için o günkü `RESULTS.md`'den
alınmıştır; rapor yazılırken kaynaktan yeniden okunur.
