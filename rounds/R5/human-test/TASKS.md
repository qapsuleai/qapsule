# Görev listesi — insan testi v3 (yaklaşık 20 dakika)

Görevler sabit sırayla yapılır (G0 → G8 → T). Her karşılaştırmalı görevde iki set vardır;
**1. set hep önce gelir**, hangi adayın 1. sette olduğu katılımcının satırından çıkar
(§ Karşı dengeleme). Setlerin içeriği (adım, kapsül) farklıdır, aynı içerik iki kez sorulmaz.
Adımlar ve satırlar `key.md`'dedir (GUIDE "Prova ve cevap anahtarı"); burada yer tutucu olarak
geçer.

Yürütücünün aynen söylediği cümleler **"tırnak içinde"**dir. Katılımcı yanıt kartından
seçeneğin numarasını söyler; açık uçlu sorularda kendi sözüyle bir cümle. Yardım isterse yalnız:
**"Ekranda ne görüyorsanız o; emin değilseniz tahmin edin."**

## Akış ve süre

| # | Görev | Karşılaştırma (aday 1 / aday 2) | Süre |
|---|---|---|---:|
| G0 | Giriş, onam, alıştırma (puanlanmaz) | — | 1:30 |
| G1 | Tek bakış: kapsülün yüzü (5 sn) + kısma sorusu | Görünüş `<GÖRÜNÜŞ-1>` / `<GÖRÜNÜŞ-2>` (M16) | 2:30 |
| G2 | Kapsülün dili: açılmış kapsülü okuma, sözlük, yüzle tutarlılık | Dil `marla` / `selvi` (M17; A33) | 3:00 |
| G3 | İşaret alfabesi (yazılar örtülü) | `siluet` / `rozet` (A21) | 1:30 |
| G4 | Açılmış kapsül ve kanıt; yetkisiz okuyucu | `<OPEN-1>` / `<OPEN-2>` (M20; A32) | 2:30 |
| G5 | Hikâye çizelgesi | `pavka` / `korda` (M18) | 2:30 |
| G6 | Akış: son bakıştan beri, elimdekinde olumsuz | Akış sırası `grouped` / `live_new` (M19) | 2:30 |
| G7 | Akış: yazı tura eşiği; boş akış | `rate55` / `none` (K53) | 1:30 |
| G8 | Hareket: canlı güncelleme, açılma; hareketi azalt | `<MOTION-1>` / `<MOTION-2>` (M21) + hareketsiz | 2:00 |
| T | Tercihler ve nedenleri | hepsi | 1:00 |
| | **Toplam** | | **~20:30** |

Süre taşarsa sırayla şunlar atlanır (eşli karşılaştırmayı bozmadan): T'deki "neden" soruları
(tercih kalır) → G1 kısma sorusu → G5 (d) sorusu → G8 3. set. Atlanan formda "atlandı" diye
işaretlenir (eksik sayılır, yanlış değil).

---

## G0 — Giriş, onam, alıştırma (puanlanmaz)

Onam metni `GUIDE.md` "Onam"da. Alıştırma: taban bağlantı (U00), `<ADIM-P>`, akışın en üstteki
kutusu. **"Telefonu beş saniye göstereceğim; en üstteki kutuya bakın. Sonra telefonu alıp kartta
sorular soracağım."** 5 sn → kartta G1 sorularını birlikte gezin. Cevap tartışılmaz.

## G1 — Tek bakış: kapsülün yüzü (Görünüş, M16)

- **Ayar:** dil taban (`marla`), görünüş sete göre (U11 / U12). Her sette **iki kapsül**
  (`<ADIM-G1-1a>`/`<SATIR-G1-1a>`, `<ADIM-G1-1b>`/`<SATIR-G1-1b>`; 2. set `…-2a`, `…-2b`).
  Hedef kapsül en üstte, telefon katılımcıya verilir.
- **Söylenen:** **"Beş saniye; en üstteki kutuya bakın."** 5 sn sonra telefon alınır.
- **Kısma sorusu (yalnız her setin ilk kapsülünde, önce):** **"İlk gözünüze ne çarptı? Tek
  kelime ya da kısa bir cümle."** (açık uçlu)
- **Kart soruları** (süre: telefon alındıktan son cevaba kadar):

| kod | soru (aynen) | seçenekler |
|---|---|---|
| `dir` | "Fiyat hangi yöne gidiyor?" | 1 yukarı · 2 aşağı · 3 belirsiz · 4 anlamadım |
| `kind` | "Ne oldu?" | 1 fiyat önemli bir seviyeyi geçti · 2 yön değişiyor · 3 hareket sürüyor · 4 fiyat sıkıştı, hareketsiz · 5 aşırı gitti · 6 büyük alım ya da satım var · 7 anlamadım |
| `valid` | "Hâlâ geçerli mi?" | 1 evet, sürüyor · 2 hayır, artık geçerli değil · 3 kapandı: gerçekleşti · 4 kapandı: tutmadı · 5 kapandı: süresi doldu ya da geri çekildi · 6 bilmiyorum |
| `when` | "Ne zaman oldu?" | 1 bugün · 2 dün · 3 birkaç gün önce · 4 bir haftadan önce · 5 bilmiyorum |
| `trust` | "Geçmişte bu tür durum ne sıklıkla tuttu?" | 1 çoğunlukla tuttu · 2 yarı yarıya · 3 çoğunlukla tutmadı · 4 az örnek var, bilinmiyor · 5 hiçbir bilgi göremedim |
| `advice` | "Bu ekran size ne yapmanızı söylüyor?" | 1 almamı · 2 satmamı · 3 beklememi · 4 hiçbir şey, yalnız bilgi veriyor · 5 bilmiyorum |

## G2 — Kapsülün dili (Dil, M17; A33)

- **Ayar:** görünüş taban, dil sete göre (U21 / U22). Her sette **bir kapsül**
  (`<ADIM-G2-1>`/`<SATIR-G2-1>`; 2. set `<ADIM-G2-2>`/`<SATIR-G2-2>`), sözlük terimi taşıyan.
- **(a) Yüz ve açılma.** Telefon katılımcıda, hedef en üstte: **"Bu kutuya bakın, sonra dokunup
  açın. Okuyun; hazır olunca telefonu bana verin."** Süre: dokunuştan telefonu verene kadar
  (`okuma`).
- **(b) Sorular** (telefon yürütücüde; açık uçlular kendi sözüyle, yürütücü aynen yazar):

| kod | soru (aynen) | biçim |
|---|---|---|
| `l_what` | "Ne oldu?" | G1 `kind` seçenekleri |
| `l_level` | "Hangi seviye ya da sınır söz konusuydu, değeri kaçtı?" | açık uçlu (doğru: eşiğin adı **ve** değeri, `key.md` toleransıyla) |
| `l_when` | "Ne zaman oldu?" | G1 `when` |
| `l_valid` | "Hâlâ geçerli mi?" | G1 `valid` |
| `l_ledger` | "Geçmişte kaç örnekte kaç kez tuttu?" | açık uçlu (doğru: örnek sayısı ve tutan sayısı; örnek yoksa "henüz yok") |
| `l_advice` | "Bu metin size ne yapmanızı söylüyor?" | G1 `advice` |
| `l_same` | "Kapalı kutudaki kısa cümle ile açılınca gelen metin aynı şeyi mi söylüyordu?" | 1 evet, aynı · 2 hayır, çelişiyordu · 3 biri ötekinde olmayan bir şey söylüyordu · 4 emin değilim |

- **(c) Sözlük.** Telefon geri verilir, açılmış kapsül ekranda: **"Ekranda dokununca açıklaması
  gelen bir sözcük var mı? Varsa açın ve ne anlattığını kendi sözünüzle söyleyin."** Kayıt: buldu
  mu (`g_found`), açılana kadar dokunma sayısı (`g_taps`), süre, açıklamayı doğru aktardı mı
  (`g_right`: yürütücü `key.md`'deki sözlük metniyle karşılaştırır; anlam aynıysa doğru).

## G3 — İşaret alfabesi (A21)

- **Ayar:** görünüş ve dil taban, alfabe sete göre (U31 / U32). `<ADIM-G3-1>` (2. set
  `<ADIM-G3-2>`), akışın görünen ilk **üç** kapsülü. Metin örtüsü (GUIDE "Metin örtüsü") yalnız
  işaretleri açıkta bırakır.
- **Söylenen:** **"Şimdi yazılar kapalı; yalnız işaretler açık. Yukarıdan aşağı üç işaret var.
  Her biri için kartta üç soru: yön, ne oluyor, hâlâ geçerli mi. Telefona dokunmayın,
  büyütmeyin."**
- **Sorular** her işaret için: `dir`, `kind`, `valid` (G1 seçenekleri). Süre: örtü yerleşince
  başlar, üçüncü işaretin son cevabında biter.

## G4 — Açılmış kapsül ve kanıt (M20); yetkisiz okuyucu (A32)

- **Ayar:** açılmış kapsül tasarımı sete göre (U41 / U42 yetkili; U43 / U44 aynı tasarım yetkisiz,
  `principal=visitor`). Yetkili kapsül `<ADIM-G4-1>`/`<SATIR-G4-1>` (2. set `…-2`): kanıtı en az
  iki aileden (ör. fiyat ve hacim) ve güven iddiası "sunucu doğruladı" olan bir kapsül; 2. sette
  "bu cihazda sınandı" olan bir kapsül (iki güven durumu iki sete dağılır; hangisinin hangi sette
  olduğu sabittir, tasarımdan bağımsızdır).
- **(a) Yetkili.** Kapsül açık (kapsül derinliği), telefon katılımcıda: sorular sırayla; her
  soruda süre ve dokunma sayısı (ilk dokunuştan cevaba).

| kod | soru (aynen) | biçim |
|---|---|---|
| `e_rests` | "Bu iddia neye dayanıyor? Gerekirse dokunarak bulun ve söyleyin." | 1 fiyatın hareketi · 2 işlem hacmi · 3 alım-satım akışı · 4 haber · 5 bilmiyorum (birden çok seçilebilir; doğru = `key.md`'deki ailelerin tamamı, eksik ya da fazla yarım puan) |
| `e_who` | "Bunu kim doğruladı?" | 1 sunucu (uygulamanın merkezi) · 2 bu telefon · 3 kimse · 4 bilmiyorum |
| `e_valid` | "Hâlâ geçerli mi?" | G1 `valid` |

- **(b) Yetkisiz.** Yürütücü yetkisiz bağlantıyı açar (aynı tasarım, `<ADIM-G4-V1>` / `-V2`,
  hedef en üstte). **"Beş saniye; en üstteki kutuya bakın."** Sonra telefon katılımcıda, kutu açık:

| kod | soru (aynen) | biçim |
|---|---|---|
| `v_trust` | "Geçmişte bu tür durum ne sıklıkla tuttu?" | G1 `trust` + 6 bana gösterilmiyor (doğru: 6) |
| `v_why` | "Neden rakam göremiyorsunuz?" | 1 bu bilgiyi görme yetkim yok · 2 henüz veri yok · 3 bir hata var · 4 rakam görüyorum · 5 bilmiyorum (doğru: 1) |
| `v_who` | "Bunu kim doğruladı?" | `e_who` seçenekleri (doğru: `key.md`) |

## G5 — Hikâye çizelgesi (M18)

- **Ayar:** hikâye çizelgesi sete göre (U51 / U52). Kapsül `<ADIM-G5-1>`/`<SATIR-G5-1>` (2. set
  `…-2`) grafik derinliğinde açık; görünen aralıkta en az 10 hikâye, en az bir canlı, en az üç
  kapanmış (en az biri gerçekleşti, biri tutmadı). Kapanmışlar görünür (K54 varsayılanı).
- Telefon katılımcıda; her soruda süre ve dokunma sayısı:

| kod | soru (aynen) | biçim |
|---|---|---|
| `c_recent` | "Grafikte görünen dönemde bu hissede kaç olay olmuş?" | 1 hiç · 2 1–4 · 3 5–9 · 4 10–29 · 5 30 ve üstü · 6 bilmiyorum |
| `c_held` | "Biten olayların kaçı tuttu?" | 1 çoğu · 2 yarı yarıya · 3 çoğu tutmadı · 4 biten yok · 5 bilmiyorum |
| `c_live` | "Şu an hâlâ süren olay hangisi? Parmağınızla gösterin." | gösterme (doğru: canlı hikâyelerden biri; yürütücü işaretler) |
| `c_evid` | Yürütücü kapanmış bir işareti (`key.md`) parmakla gösterir: **"Şu işaret neye dayanıyor? Ona ulaşın ve söyleyin."** | dokunma sayısı + `e_rests` seçenekleri (doğru: o hikâyenin kanıt aileleri; odak o işarete taşınmış olmalı) |

## G6 — Akış: son bakıştan beri (Akış sırası, M19)

- **Ayar:** eşik taban (`rate55`), sıra sete göre (U61 / U62). Kitle URL'de:
  `?watch=<KİTLE-İZLEME>&long=<KİTLE-ELDE-1>&seen=<ADIM-G6-1-T0>`, önce `<ADIM-G6-1-T0>`'a, sonra
  `<ADIM-G6-1-T1>`'e gidilir (2. set `…-2-…`, elde `<KİTLE-ELDE-2>`). Pencere seçimi: elde tutulan
  bir hissede t0–t1 arasında olumsuz geçiş (zayıfladı, tutmadı, geri çekildi) **ve** en az iki
  izlenen hissede değişiklik; olumsuz geçiş iki sırada da akışta görünür (K42 muafiyeti).
- **Söylenen:** **"Bu sizin uygulamanız. En son dün baktınız. Elinizde {elde: `key.md`} var;
  izleme listenizde birkaç hisse daha. Kaydırabilirsiniz."**

| kod | soru (aynen) | biçim; süre ve kaydırma sayısı her soruda |
|---|---|---|
| `f_new` | "Son baktığınızdan beri ne değişti?" | açık uçlu: "hisse + ne" listesi ya da "yok" |
| `f_neg` | "Elinizdeki hisselerde olumsuz bir şey var mı? Hangisinde, ne?" | hisse ya da "yok" + 1 tutmadı, geçersiz oldu · 2 zayıfladı · 3 geri çekildi · 4 başka · 5 bilmiyorum |

## G7 — Akış: yazı tura eşiği (K53); boş akış

- **Ayar:** sıra taban (`grouped`), eşik sete göre (U71 / U72); kitle yok (bütün akış),
  `<ADIM-G7-1>` (2. set `<ADIM-G7-2>`). Anın seçimi: `none` ile akışta en az iki yazı tura karneli
  kapsül (`rate55`'te gizlenen) ve en az bir "az örnekli" kapsül görünür.
- Telefon katılımcıda, 15 sn kaydırma serbest; sonra:

| kod | soru (aynen) | biçim |
|---|---|---|
| `r_trust` | "Bu akıştaki bilgilere ne kadar güvenirsiniz?" | 1 hiç … 5 tamamen |
| `r_best` | "Geçmişi en iyi olan kutuyu gösterin." | gösterme (doğru: örnek sayısı ≥ 20 olanlardan en yüksek oran kovasındakiler) |
| `r_few` | "Ekranda 'az örnek' gibi bir şey yazan kutu var mı? Ne anlama geliyor?" | 1 geçmişte az kez olmuş, bilgi az · 2 güvenilmez, tutmuyor · 3 görmedim · 4 bilmiyorum (doğru: 1; görünmüyorsa 3) |

- **Boş akış** (tek ekran, karşılaştırmasız; U70, `<ADIM-G7-BOŞ>` ve `<KİTLE-BOŞ>`: akışın boş
  olduğu an). **"Bu ekran size ne söylüyor?"** 1 şu an anlatılacak bir şey yok · 2 uygulama bozuk
  ya da yükleniyor · 3 piyasa kapalı · 4 bilmiyorum (doğru: 1; `x_empty`).

## G8 — Hareket (M21); hareketi azalt

- **Ayar:** hareket sete göre (U81 / U82); 3. set aday 1 ile **hareketi azalt açık** (U83: `anim=0`
  ya da işletim sisteminin "hareketi azalt" ayarı; hangisi olduğu provada denetlenip `key.md`'ye
  yazılır). Her sette akış açık, `<ADIM-G8-n>`'de duraklatılmış; değişecek kapsül görünür alanda.
- **(a) Canlı güncelleme.** **"Ekrana bakın; on saniye hiçbir şeye dokunmayın."** Yürütücü
  oynatmayı `<ADIM-G8-n>`'den bir adım ilerletir (**+1**) ve telefonu katılımcıda bırakır; 10 sn
  sonra:

| kod | soru (aynen) | biçim |
|---|---|---|
| `m_seen` | "Ekranda bir şey değişti mi?" | 1 evet · 2 hayır · 3 emin değilim |
| `m_what` | (evetse) "Nerede, ne değişti? Gösterin." | gösterme + kısa söz (doğru: `key.md`'deki kapsül ve değişimin türü: doğdu / güçlendi / geçersiz oldu) |

- **(b) Açılma** (yalnız 1. ve 2. sette): **"Bu kutuya dokunun."** Açılınca:

| kod | soru (aynen) | biçim |
|---|---|---|
| `m_same` | "Az önceki kutuyla aynı şeye mi bakıyorsunuz?" | 1 evet · 2 hayır · 3 emin değilim |
| `m_lost` | "Açılırken nerede olduğunuzu kaybettiniz mi?" | 1 hiç … 5 tamamen |

## T — Tercihler

Her karşılaştırma için, görev adını anarak (G1 "kutular", G2 "metinler", G3 "işaretler", G4
"açılan ekran ve dayanağı", G5 "grafikteki olaylar", G6 "akışın sırası", G7 "akış", G8 "hareketler"):
**"İlk gördüğünüz mü ikinci gördüğünüz mü daha kolaydı, yoksa fark yok mu? Neden?"** Cevap: 1 ilk ·
2 ikinci · 0 fark yok; neden aynen. Sonra G7 için ayrıca: **"Geçmişi yazı tura kadar belirsiz olan
kutuları akışta görmek ister miydiniz?"** 1 evet · 2 hayır · 3 fark etmez (`k53_want`). Son olarak:
**"Eklemek istediğiniz bir şey?"**

## Karşı dengeleme

Hücre: hangi aday 1. sette (1 = aday 1 önce, 2 = aday 2 önce). Satır katılımcının sıra numarasıdır;
**iki persona ayrı sayılır** (genç G01, G02 …; alışkın A01, A02 … aynı satırı kullanır). Ardışık iki
satır (01–02, 03–04, …) birbirinin tersidir: her persona içinde çift sayıda katılımcıda her
karşılaştırma tam dengelenir. Sütunlar birbirinden farklı desendedir (bir katılımcının her şeyi
hep aynı sırayla görmesi yalnız 01'de olur, 02 onun tersidir).

| sıra | G1 görünüş | G2 dil | G3 alfabe | G4 açılmış | G5 hikâye | G6 sıra | G7 eşik | G8 hareket |
|---|---|---|---|---|---|---|---|---|
| 01 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| 02 | 2 | 2 | 2 | 2 | 2 | 2 | 2 | 2 |
| 03 | 1 | 2 | 1 | 2 | 1 | 2 | 1 | 2 |
| 04 | 2 | 1 | 2 | 1 | 2 | 1 | 2 | 1 |
| 05 | 1 | 1 | 2 | 2 | 1 | 1 | 2 | 2 |
| 06 | 2 | 2 | 1 | 1 | 2 | 2 | 1 | 1 |
| 07 | 1 | 2 | 2 | 1 | 1 | 2 | 2 | 1 |
| 08 | 2 | 1 | 1 | 2 | 2 | 1 | 1 | 2 |
| 09 | 1 | 1 | 1 | 1 | 2 | 2 | 2 | 2 |
| 10 | 2 | 2 | 2 | 2 | 1 | 1 | 1 | 1 |

Ör. G03 satırı: G1'de 1. set `<GÖRÜNÜŞ-1>` (U11), 2. set `<GÖRÜNÜŞ-2>` (U12); G2'de 1. set `selvi`
(U22), 2. set `marla` (U21). G8'in 3. seti (hareketi azalt) her zaman en sondadır.

## Bağlantı kartı (yalnız yürütücü; katılımcı görmez)

Taban (U00): `http://<adres>:8390/?lang=tr&theme=dark&voice=marla&look=<GÖRÜNÜŞ-1>&story=pavka&feed_order=grouped&feed_rule=rate55&open=<OPEN-1>&motion=<MOTION-1>&alphabet=siluet`

Her kod tabandan yalnız bir parametreyi değiştirir:

| kod | değişen | kod | değişen |
|---|---|---|---|
| U11 | `look=<GÖRÜNÜŞ-1>` | U12 | `look=<GÖRÜNÜŞ-2>` |
| U21 | `voice=marla` | U22 | `voice=selvi` |
| U31 | `alphabet=siluet` | U32 | `alphabet=rozet` |
| U41 | `open=<OPEN-1>` | U42 | `open=<OPEN-2>` |
| U43 | `open=<OPEN-1>&principal=visitor` | U44 | `open=<OPEN-2>&principal=visitor` |
| U51 | `story=pavka` | U52 | `story=korda` |
| U61 | `feed_order=grouped` + G6 kitlesi | U62 | `feed_order=live_new` + G6 kitlesi |
| U71 | `feed_rule=rate55` | U72 | `feed_rule=none` |
| U70 | taban + `<KİTLE-BOŞ>` (boş akış) | | |
| U81 | `motion=<MOTION-1>` | U82 | `motion=<MOTION-2>` |
| U83 | `motion=<MOTION-1>&anim=0` (ya da işletim sistemi ayarı) | | |

Bağlantılar yer imi olarak telefona eklenir; adres çubuğu katılımcıya gösterilmez (tasarım adı URL'de).

---

## English annex (participant-facing lines and card)

Use only if the participant prefers English; then add `lang=en` to every link and record
`dil=en` on the form. Lines are translated one-to-one; the same rules apply.

- G0: "I'll show you the phone for five seconds; look at the top box. Then I'll take the phone and ask questions from the card."
- G1: "Five seconds; look at the top box." · Throttle: "What caught your eye first? One word or a short sentence."
  Card: `dir` "Which way is the price going?" (up · down · unclear · didn't understand) · `kind` "What happened?" (price crossed an important level · direction is changing · move continues · price is squeezed, flat · went too far · big buying or selling · didn't understand) · `valid` "Is it still valid?" (yes, ongoing · no longer valid · closed: happened · closed: didn't hold · closed: expired or withdrawn · don't know) · `when` "When did it happen?" (today · yesterday · a few days ago · more than a week ago · don't know) · `trust` "How often did this kind of thing hold in the past?" (mostly held · about half · mostly didn't · few cases, unknown · saw no information) · `advice` "What does this screen tell you to do?" (buy · sell · wait · nothing, it only informs · don't know)
- G2: "Look at this box, then tap to open it. Read it; hand me the phone when ready." · `l_level` "Which level or limit was involved, and what was its value?" · `l_ledger` "In how many past cases, and how many times, did it hold?" · `l_same` "Did the short sentence on the closed box and the opened text say the same thing?" (yes, the same · no, they contradicted · one said something the other didn't · not sure) · Glossary: "Is there a word on the screen that shows an explanation when tapped? If so, open it and tell me in your own words what it says."
- G3: "The text is covered now; only the marks are visible. There are three marks from top to bottom. For each, three questions: direction, what is happening, still valid. Don't touch or zoom the phone."
- G4: `e_rests` "What does this claim rest on? Tap to find it if needed." · `e_who` "Who verified this?" (the server · this phone · nobody · don't know) · `v_why` "Why can't you see numbers?" (I'm not entitled to see it · no data yet · something is broken · I do see numbers · don't know)
- G5: "How many events happened in this stock in the period shown?" · "How many of the finished events held?" · "Which event is still ongoing now? Point at it." · "What does that mark rest on? Get to it and tell me."
- G6: "This is your app. You last looked yesterday. You hold {holding}; a few more stocks are on your watchlist. You may scroll." · "What has changed since you last looked?" · "Is there anything negative in the stocks you hold? Which, what?"
- G7: "How much would you trust the information in this feed?" (1 not at all … 5 fully) · "Point at the box with the best past record." · "Is there a box that says something like 'few cases'? What does it mean?" · Empty: "What is this screen telling you?" (nothing to report right now · the app is broken or loading · the market is closed · don't know)
- G8: "Look at the screen; don't touch anything for ten seconds." · "Did anything change on the screen?" · "Where, what changed? Point at it." · "Tap this box." · "Are you looking at the same thing as the box a moment ago?" · "Did you lose track of where you were while it opened?" (1 not at all … 5 completely)
- T: "Was the first or the second one easier, or no difference? Why?" · "Would you want to see boxes whose past record is as uncertain as a coin flip in your feed?" · "Anything you'd like to add?"
