# Karar kuralı (önceden yazılı; hiçbir katılımcıdan önce)

Bu dosya, insan testi v3'ün hangi açık kararı hangi görevle ve hangi kuralla kapatacağını
söyler. Ölçülerin hesabı `SCORING.md`'dedir. Sonuç görüldükten sonra bir kural değişirse
değişiklik ve nedeni `results.md`'de açıkça yazılır ve o karar "kural sonradan değişti" diye
işaretlenir.

**Sınır cümlesi: ajan insan değildir.** R5'in M16–M21 ölçümleri ön elemedir. İnsan sonucu ajan
sonucundan farklıysa **insan sonucu geçerlidir**. İnsan sonucu "ayırt edilemedi" ise ajan seçimi
yerinde kalır ama "insanca onaylandı" denmez: kayıt **"insan testinde ayırt edilmedi; ajan seçimi
geçici olarak kaldı"** olur.

Kararlar `DECISIONS.md`'ye bu paket tarafından değil, orkestratör / ana akış eliyle yazılır
(kanıt: `results.md` yolu, n, ölçü, işaret sayısı, kuralın hangi adımı). Varsayılanın değişmesi
(ör. gösterimin varsayılan dili) ayrı bir ürün işidir.

## 1. Ne kapanır, hangi görevle

| Karar | Görev | Aday 1 (eşitlikte kalan) | Aday 2 | Kural | Kapanış |
|---|---|---|---|---|---|
| **A21** alfabe | G3 (yazılar örtülü, işaret başına tam doğru) | `siluet` | `rozet` | §2 eşli kural | her durumda kapanır (eşitlikte `siluet`) |
| **A32** yetkisiz yüz | G4 (b) | — | — | §3 mutlak | kapanır ya da bulguyla açık kalır |
| **A33** ayrıntı cümlesi | G2 `l_same` (+ G2 doğruluğu) | — | — | §4 mutlak | dil başına kapanır ya da açık kalır |
| **K53** gizleme eşiği | G7 (+ T `k53_want`) | `rate55` | `none` | §5 | Arkin'e kanıt; K53'ü bu test kendiliğinden değiştirmez |
| **K42 muafiyeti** (K53 içinde) | G6 `f_neg` | muaf (gösterimin tek biçimi) | — | §5.3 mutlak | yalnız muaf biçim için kanıt |
| **Akış sırası** (M19) | G6 | `grouped` | `live_new` | §2 | onaylanır ya da değişir |
| **Boş akış** (GOAL kabul 7) | G7 `x_empty` | — | — | §6 mutlak | anlaşılıyor / yeniden tasarım |
| **M16** görünüş | G1 | `<GÖRÜNÜŞ-1>` | `<GÖRÜNÜŞ-2>` | §2 + §7 | onaylanır ya da değişir |
| **M17** dil | G2 | `marla` | `selvi` | §2 + tavsiye kapısı | onaylanır ya da değişir |
| **M18** hikâye çizelgesi | G5 | `pavka` | `korda` | §2 (süre adımı `c_live`) | onaylanır ya da değişir |
| **M20** açılmış kapsül | G4 (a) | `<OPEN-1>` | `<OPEN-2>` | §2 + §8 | onaylanır ya da değişir |
| **M21** hareket | G8 | `<MOTION-1>` | `<MOTION-2>` | §2 + §9 | onaylanır ya da değişir |

## 2. Eşli kural (iki aday; her karşılaştırma için ayrı)

0. **Yeterlilik.** İki seti de geçerli en az **5** katılımcı ve her personada en az **2** kişi
   yoksa karar yok: ajan seçimi geçici kalır, kayıt "insan testinde yetersiz örnek".
1. **Tavsiye kapısı** (G1, G2; "gözlem, tavsiye değil"). Bir adayda "al" ya da "sat" seçen
   katılımcı sayısı ötekinden **en az 2 fazlaysa** o aday bu karşılaştırmayı kazanamaz (kural 2–4
   ne derse desin) ve bu, adaydan bağımsız bir dil bulgusu olarak yazılır.
2. **Doğruluk.** Birincil ölçüde (`SCORING.md` §2) ağırlıklı eşli fark mutlak değerce **≥ 0,10**,
   aynı yönde daha iyi olan katılımcı sayısı öteki yöndekinden **en az 2 fazla** ve iki persona
   ters yönde ayrışmıyor (iki personanın ortalama farkı ikisi de ≥ 0,10 ve ters işaretli değil)
   → kazanan. Personalar ayrışırsa bu adım uygulanmaz, rapor bunu yazar ve 3. adıma geçilir.
3. **Tercih.** 2. adım karar vermezse: tercih belirtenlerin (fark yok hariç) **en az ⅔'ü** ve en az
   **3** kişi aynı adayı seçtiyse o — **zayıf** etiketiyle.
4. **Süre / dokunma** (yalnız G4, G5, G6; hedefe ulaşma görevleri): 2–3 karar vermezse, doğru
   maddelerin süre farklarının ortancası **≥ 2 sn ve ≥ %20** ise hızlı olan — **zayıf**. G5'te
   ölçü `c_live` süresidir.
5. Hiçbiri karar vermezse **ayırt edilemedi**: aday 1 kalır.

2. adımla verilen karar, ajan ölçümünün tersi olsa da geçerlidir. 3–4. adımla verilen karar "zayıf"
diye işaretlenir; sonraki ölçümde daha çok katılımcıyla yeniden bakılması önerilir. Hiçbir adım
"anlamlı" sözcüğünü kullanmaz (`SCORING.md` §5).

## 3. A32 — yetkisiz okuyucunun yüzü (mutlak)

Ölçü: G4 (b), iki sette (iki aday tasarımında) yetkisiz yüz ve açılmış hâli. Katılımcı başına
"doğru" = iki setin ikisinde de `v_why` = 1.

- **KAPANDI** (şablonlu rakamsız yüz anlaşılıyor): `v_why`'ı en az bir sette yanlış cevaplayan
  **en çok 1** katılımcı **ve** "rakam görüyorum" (4) diyen **0** katılımcı **ve** `v_trust`'ta
  "bana gösterilmiyor" (6) diyen en az **n − 1** katılımcı (iki setin en az birinde).
- **Açık kalır, bulguyla:** "bir hata var" (3) ya da "henüz veri yok" (2) diyen **≥ 2** katılımcı
  → "yetkisiz yüz yetki eksikliği olarak değil, hata / eksik veri olarak okunuyor"; alıntılar
  rapora.
- "Rakam görüyorum" diyen herhangi bir katılımcı → önce girişimsizlik testlerinin (K33) o ekranda
  geçtiği yeniden denetlenir; geçiyorsa ekrandaki neyin rakam sanıldığı bulgu olarak yazılır.
- İki aday arasında yetkisiz sorulardaki fark M20 kararında yalnız ikincil kanıttır.

## 4. A33 — ayrıntı cümlesi yüzle çelişmiyor (mutlak, dil başına)

Ölçü: G2 `l_same`, her dil için (o dili gören katılımcılar).

- **KAPANDI** (o dil için): `l_same` = 2 ("çelişiyordu") diyen **en çok 1** katılımcı **ve** o
  dilde G2 birincil doğruluğu (ağırlıklı) ≥ 0,60.
- **Açık kalır:** "çelişiyordu" diyen **≥ 2** katılımcı → hangi kapsülde, neyin çeliştiği
  (sesli düşünme notlarından) rapora; ilgili şablon düzeltmesi önerilir.
- `l_same` = 3 ("biri ötekinde olmayan bir şey söylüyordu") çelişki sayılmaz (ayrıntının ek bilgi
  vermesi K49 gereğidir); ama yarıdan fazlaysa "yüz eksik okunuyor" gözlemi yazılır.
- A33 seçilen dil (M17 kararı) için kapanırsa kapanmış sayılır; öteki dilin sonucu bilgi içindir.

## 5. K53 — yazı tura eşiği ve K42 muafiyeti

K53 Arkin'in kararıdır; bu test eşiği kendiliğinden kaldırmaz ya da değiştirmez. Arkin'e şu
biçimde kanıt verir:

1. **Güven algısı** (G7 `r_trust`, 1–5): ağırlıklı eşli fark **≥ 0,5 puan** ve işaret farkı
   **≥ 2** → "insan testinde yön: X". Yoksa ayırt edilemedi.
2. **Sonuç:**
   - `rate55` yönünde ya da ayırt edilemedi **ve** `k53_want`'ta "hayır" diyenler ≥ ⅔ (fark etmez
     hariç) → **`rate55` kalır**; kayıt "insan testi gizlemeyi destekledi" (ilk durum) ya da
     "ayırt edilmedi; tercih gizlemeden yana" (ikinci).
   - `none` yönünde **ve** `k53_want`'ta "evet" diyenler ≥ ⅔ → **Arkin'e K53'ün yeniden
     değerlendirilmesi önerilir** (kanıt bu satırlar; eşik kendiliğinden kalkmaz).
   - Öteki durumlar → `rate55` kalır; kayıt "insan testinde ayırt edilmedi". Not düşülür: ajan
     benzetimi de `rate55`'i `none`'dan ayıramadı (M19 §4.2); eşiğin dayanağı ölçüm değil, K53'ün
     gerekçesidir.
   - Ek: `rate55` `r_best` doğruluğunu ağırlıklı **≥ 0,10** ve işaret farkı ≥ 2 ile düşürüyorsa
     "gizleme en güvenilir kutuyu bulmayı zorlaştırdı" bulgusu yazılır.
3. **K42 muafiyeti** (G6 `f_neg`; gösterim yalnız muaf biçimi çalıştırır): elde tutulan hissedeki
   olumsuz geçişi iki pencerenin en az birinde bulamayan **en çok 1** katılımcı → "muaf biçimde
   olumsuz haber bulunuyor": M19 önerisi (muafiyet) insan kanıtıyla desteklenir. **≥ 2** katılımcı
   bulamadıysa → "muafiyet akışa sokuyor ama görünmüyor" bulgusu (sıra ya da biçim sorunu; akış
   sırası kararına girdi). **Muafsız biçim insanla sınanmadı**; muafiyet kararı Arkin'indir.
4. **"Az örnekli" ile "yazı tura" ayrımı** (K53 kural 3; G7 `r_few`): "az örnek" kutusunu görüp
   "güvenilmez, tutmuyor" (2) diyen **≥ 2** katılımcı → "az örnek yazı tura sanılıyor" bulgusu;
   M17 sözlüğüne / yüz metnine öneri.

## 6. Boş akış (mutlak)

G7 `x_empty`: "şu an anlatılacak bir şey yok" (1) diyen en az **n − 1** katılımcı → **boş akış
durumu anlaşılıyor** (GOAL kabul 7 insanca desteklendi). "Bozuk ya da yükleniyor" (2) diyen
**≥ 2** katılımcı → yeniden tasarım önerilir (dürüstlük ölçütü insan gözünde tutmadı).

## 7. M16 görünüş — ek kurallar

- Birincil: G1 (§2). Kısma sorusu (ilk çarpan işaret / yön / olay mı) ikincil kanıttır; tek
  başına karar vermez, ama birincil "ayırt edilemedi" iken iki aday arasında **≥ 3 kişilik**
  fark varsa raporun "gözlem" bölümüne yazılır.
- Renk görme farkı beyan eden katılımcılar `SCORING.md` §6'ya göre ayrıca raporlanır; seçilen
  görünüşte anlam renklerinden kaynaklanan karışıklık gözlenirse M16'ya "insan gözünde renk
  ayrımı sorunu" notu düşülür (seçim değişmez, düzeltme önerilir).

## 8. M20 açılmış kapsül — ek kural (güven ayrımı)

GOAL kabul 8 "sunucu doğruladı ile bu cihazda sınandı ayrımı anlaşılıyor": seçilen adayda
(§2'nin sonucu) `e_who`'yu **set 1'de (sunucu) ve set 2'de (cihaz) ayrı ayrı** doğru cevaplayan
katılımcı oranı ikisinde de ≥ **⅘** → "ayrım anlaşılıyor". Yoksa hangi yönde karıştırıldığı
(cihaz → sunucu mu, tersi mi) rapora; seçim değişmez, düzeltme önerilir.

## 9. M21 hareket — ek kural (hareketi azalt)

GOAL kabul 9 "hareketi azalt ayarında bilgi kaybı yok":
- Katılımcı başına: aday-1 setinde fark edip 3. sette (hareketi azalt) fark etmeyen = kayıp;
  tersi = kazanç. **Kayıp − kazanç ≥ 2** → "hareketi azalt açıkken canlı güncelleme kaçırılıyor"
  → GOAL 9 insan gözünde tutmadı, düzeltme önerilir. Aksi halde "bilgi kaybı gözlenmedi"
  (n ile birlikte; "kanıtlandı" denmez).
- Sınır: 3. set farklı bir andadır (aynı değişim iki kez gösterilemez); içerik farkı ile ayar
  etkisi ayrılamaz, rapor bunu yazar.

## 10. Eksik konu

Bir konunun adayları (ör. `<OPEN-1/2>`, `<MOTION-1/2>`, `<GÖRÜNÜŞ-1/2>`) test gününe kadar
gösterimde seçilebilir değilse: o görev **taban ayarla iki sette de** yapılır (öteki görevlerin
akışı ve süresi bozulmasın), karşılaştırması atlanır, tercih sorusu sorulmaz. Kayıt: "insan
testinde koşulmadı; ajan seçimi geçici". O görevin mutlak kuralları (A32, §8, §9) yine uygulanır.
A21, G3'ün metin örtüsü seçilen görünüşte uygulanamıyorsa (GUIDE Hazırlık 6) kapanmaz; neden
yazılır.

## Neden bu eşikler

- **0,10 ve "en az 2 fazla":** R3 ve R4 insan testi kurallarıyla aynı (karşılaştırılabilirlik);
  0,10 bir görevde aşağı yukarı bir alanlık fark, işaret şartı tek kişiye dayanmayı engeller.
- **⅔ ve 3 kişi:** R4 v2 ile aynı; n = 5'te 3–1 ya da 4–1, n = 10'da 6–3 gibi.
- **"En çok 1 kişi" / "n − 1":** mutlak kurallar oransız yazıldı; n = 5–10'da tek bir dikkatsiz
  cevabı tolere eder, ikinci bir kişi bir örüntüdür.
- **0,5 puan (1–5 ölçeği):** ölçeğin bir basamağının yarısı; daha küçüğü 5–10 kişide okunamaz.
- **Genç ağırlığı 2:** NORTH_STAR birincil kullanıcısı; ajan personaları F 2, H 1.
