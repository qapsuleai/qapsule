# M21 alternatifi `surek` — süreklilik: aynı nesne yer değiştirir

Açılış adresi: `?motion=surek` (Ayarlar → Hareket (M21)). Hareketi azalt: sistemin
`prefers-reduced-motion` ayarı, ya da `?reduce=1` / Ayarlar → Hareketi azalt.
Başvurular: `?motion=r4` (bugünkü: sunulan geçiş `zumlak`, yüz değişimi işaretsiz) ve
hareketsiz (`?anim=0`).

## İlke

Göz bir nesneyi izler; ekran "yeni bir şey" göstermez, **aynı nesnenin yer, boy ya da biçim
değiştirdiğini** gösterir (Heer & Robertson 2007, F14). Değişim yerel kalır: değişmeyen piksel
kıpırdamaz (Rensink 1997, F13). Hareketin taşıdığı her bilgi hareketi azalt'ta hareketsiz bir
kanalla, kalıcı bir etiketle verilir (HIG Motion, WCAG 2.3.3).

## Davranış

### 1. Açılış (akış → kapsül → grafik ve geri) — `transitions/surek.rs`

- **Akıştan kapsüle:** işaret, başlık ve karne satırı yüzdeki yerlerinden kalkar, açılmış
  kapsüldeki yerlerine gider (konum ve boy birlikte, yumuşak adım, `p ∈ [0; 0,8]`); işaret en
  üstte boyanır. Kartın geri kalanı **yerinde çözülür** (`p ∈ [0; 0,3]`): parçalar o karttan
  "çıkarılır", kartın hayaleti geride kalmaz. Kapsülün sahnesi parçalar yola çıktıktan sonra
  gelir (`p ∈ [0,3; 0,85]`).
- **Kapsülden grafiğe:** işaret grafikteki kendi barının üstündeki yerine uçar; olgunun
  geometrisi (`fact:*`) ve işaretli bar sütunu (`bar:<i>`) onunla gider. Fiyat paneli taşınmaz
  (iki farklı panel; birini ötekine germek "aynı nesne" değildir), kanıt mürekkebi (`evidence:*`)
  grafiğin kendisidir.
- **Geri:** aynı hareket ters yönde (tasarım simetrik).
- Süre 300 ms (tasarımın kendi süresi, F14 ≤ 300 ms) × görünüşün `motion.duration_scale`,
  eğri görünüşün `motion.ease`'i (`transitions::progress_in`). Her ara kare yalnız paylaşılan
  kutuların ve `p`'nin saf fonksiyonudur: `window.qp.freeze(p)` herhangi bir kareyi dondurur
  (düzeneğin p = 0,25 sorusu için).

### 2. Canlı güncelleme — `motion.rs`, `ChangeKind::Value`

Kapsülün kendi değerleri aynı aşamada değiştiğinde (aynı aileden yeni bir olgu: `last_bar`
değişti; ya da kanıt ailesi sayısı) yalnız değişen metinler değişir, **sırayla**: eski metin
yüzün kendi zemininden bir plakanın üstüne söner (yüzün sol üst köşesindeki zemin pikseli
kutuya gerilir), yeni metin gelir, bir kez hafifçe şişer ve yerine oturur. İkisi aynı anda
görünmez (çift görüntü yok; birim sınaması `texts_change_in_turn_without_a_double_image`).
Kartın geri kalanı kıpırdamaz. Değişen metin kutuları önceki ve sonraki yüz sahnesinin metin
ilkellerinin farkından bulunur (`text_spots`, en çok 4).

Yaş saati ("12 işlem günü önce") ve yüzün fiyat çizgisi her barla değişir; bunlar kapsülün
değişimi sayılmaz (sayılsaydı her barda her yüz işaretlenirdi; ilk denemede böyle oldu:
gösterimin ilk adımında akıştaki bütün kapsüller "güncellendi" çıktı).

### 3. Durum değişimi — işaretin kendisi biçim değiştirir

| Değişim | Hareket | Hareketi azalt etiketi |
|---|---|---|
| doğdu | yüz belirir, işaret kendi yerinde bir noktadan büyür | az önce doğdu / just appeared |
| güçlendi (teyit dahil) | işaret bir kez şişerek yeni biçimine geçer | az önce güçlendi / just strengthened |
| zayıfladı | eski işaret küçülerek yenisine geçer | az önce zayıfladı / just weakened |
| artık geçerli değil | eski işaret çöker (aşağı kayar, küçülür) | az önce geçerliliğini yitirdi / just stopped holding |
| tutmadı (`invalidated`) | eski işaret **ikiye kırılır**: iki yarısı zıt yönlere ayrılır ve söner, yeni (tutmadı) biçim aralarında belirir | az önce tutmadı / just failed |
| gerçekleşti | yeni biçim eskisinin üstüne damga gibi iner | az önce gerçekleşti / just came true |
| süresi doldu, geri çekildi | eski işaret yassılarak katlanır | az önce süresi doldu / az önce geri çekildi |
| değer | yalnız değişen metin sırayla değişir (yukarıda) | az önce güncellendi / just updated |

Değişimler akış yenilendiğinde önceki satırla karşılaştırılarak sınıflanır (`classify`: aşama
değişimi → türü; geçerli → artık geçerli değil; aynı aşamada `last_bar`/kanıt ailesi → değer;
akışa yeni giren ve önceki akışın hepsinden yeni `born` → doğdu; ilk akış hiçbir şey
işaretlemez). Süre görünüşün `motion.change_ms`'i (r4: 360 ms) × `duration_scale`, eğri
`motion.ease`.

"Önceki yüz": yüz, veri karesi gelir gelmez yeni barla yeniden boyanır; akışın cevabı (neyin
değiştiği) ondan sonra gelir. Bu yüzden önceki yüz kabuğun tuvalinden değil, çekirdeğin
sakladığı sahneden gelir: çekirdek boyamaya verdiği son yüz sahnesini tutar, her veri karesinde
onu "önceki" olarak dondurur ve değişimde başvuru çizicisiyle piksele çevirir
(`face_before_rgba`). Bu yalnız işaretleyen bir tasarım seçiliyken ve hareket açıkken yapılır.

### 4. Hareketi azalt

Hiçbir şey hareket etmez: açılış anında olur (`transition_progress` = 1), yüz değişimi
canlandırılmaz. Her değişim yüzün sol alt köşesine kalıcı küçük bir etiket olarak yazılır
(`stamp`: görünüşün `label` rolünün boyu ve yazı tipi, temanın `text` rengi, `bg` plakası
üstünde; sahnenin kendi ilkeli, bütün boyama yolları aynı çizer; sağ köşe kabuğun güven ve
yazı tura işaretlerinindir). Etiket okur o kapsülü açınca, "Son bakışımı şimdi işaretle"
deyince ya da daha yeni bir değişim gelince kalkar. Etiketlerde rakam yok (yetkisiz okur da
aynısını görür, K33).

**Bilgi eşitliği** (`crates/qp-wasm/tests/motion_m21.rs`): aynı gösterim iki çekirdekte (hareket
açık / azaltılmış) 60 adım oynatılır; her adımda değişim olguları (`face_changes`: kapsül, tür,
etiket) iki kipte aynıdır; azaltılmış kipte her değişimin etiketi yüz sahnesinin metinleri
arasındadır ve hiçbir yüz canlandırılmaz; hareketli kipte her canlandırılan değişimin kareleri
boş değildir ve değişir. Kayıp tanımı tek yönlüdür: hareketin taşıdığı olgu azaltılmış kipte
yoksa kayıptır; azaltılmış kipte fazladan duran etiket kayıp değildir (düzeneğin
`reduced_motion_loss`'u böyle yorumlanmalı; aksi halde etiket kanalı "fark" sayılır).

## Mimari

- Çekirdek: `qp_grammar::motion` (kayıt `MOTIONS`, `ChangeKind`, `classify`, `face_change`,
  `change_frame`, `stamp`, `label`), `qp_grammar::transitions::surek`, kabuk çekirdeğinde
  `crates/qp-wasm/src/shell/motion.rs` (seçim, bayrak, değişim kaydı, önceki yüz).
- Seçim: `?motion=` bir sunum ayarıdır (akış sırası gibi), protokol alanı eklenmedi; tasarım
  açılış geçişini `style` hareketinin var olan `transition` alanıyla ister (`surek` geçiş
  kaydında). `r4` seçilince sunulan geçiş (`zumlak`) geri istenir.
- Kabuk: yalnız piksel kopyalar (geçişin aynı sözlüğü: kaynak önce/sonra, kutu, saydamlık);
  `prefers-reduced-motion`'ı çekirdeğe iletir; hesap yok (`arch-check.mjs` geçer; yeni
  yöntemler `ARCH.md`'de).
- `r4` (varsayılan) altında hiçbir şey değişmez: değişim kaydı tutulmaz, yüz sahnesi
  saklanmaz, etiket yok (sınama `todays_motion_marks_nothing`); altınlar ve dökümler bayt eşit.

## Kayda alınması gereken değerler

| Değer | Şimdi | Öneri |
|---|---|---|
| Açılış süresi 300 ms | `transitions::surek::DURATION_MS` (tasarımın, görünüş çarpar) | `Look.motion.open_ms` |
| Yüz değişimi süresi | `Look.motion.change_ms` (eklendi; r4 360, `ornek` r4'ten) | — (kayıtta) |
| Aşama oranları (yol 0,8; çözülme 0,3; sahne 0,3–0,85; metin sırası 0,35/0,4/0,75; şişme 1,12; işaret şişmesi 1,3; damga 1,4; kırılma ±0,12 genişlik) | tasarım sabitleri | tasarımın kimliği; kayda alınmaları gerekmez, ama `Look.motion.emphasis` (şişme oranı) düşünülebilir |
| Etiket yeri (sol alt, içeri `offsets.title_x`), rolü (`label`), renkleri (`text` / `bg`) | kayıttan okunuyor | — |
| Plaka pikseli (yüzün sol üst köşesi zemindir varsayımı) | `motion::PLATE` | yüz tasarımı kaydı "zemin noktası" verebilir |

## Ölçümler

(aşağıda; `K52` bölümü)

## Bilinen sınırlar

- Önceki yüzün pikselleri başvuru çizicisinden gelir (fontdue); `canvas` yolunda metin
  `fillText` ile çizildiğinden eski metin parçası ile tuvaldeki yeni metin arasında yazı
  çizimi farkı olabilir (yalnız değişimin ilk yarısında, eski parça sönerken).
- Viewer kipinde (gösterimin varsayılanı) otorite yalnız ekrandaki yüzleri gönderir; ekran
  dışındaki bir yüzün değişimi canlandırılmaz (görülmüyor), etiketi (azaltılmış kip) yüz
  ekrana gelince çizilir.
- Kaydırma sırasında süren bir yüz değişimi (360 ms) sabit katmanda kalır.
- Sentetik gösterimde `value` türü nadirdir; 60 adımlık sınamada doğdu, güçlendi,
  gerçekleşti, zayıfladı, tutmadı görüldü; süresi doldu 252. adımda.
