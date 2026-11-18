# M21 alternatifi `nabiz` — az ve anlamlı hareket

Dal `r5/m21-nabiz`. Açılış: `?motion=nabiz` (Ayarlar → Hareket). Örnek:
`http://127.0.0.1:<port>/?motion=nabiz&look=r4` ve `…&look=ornek&theme=dark`.
Hareketi azalt: tarayıcının `prefers-reduced-motion: reduce` ayarı ya da `?anim=0`.

## Yön

Hareket bütçesi az; yalnız anlam taşıyan anlara harcanır. Geçişler kısa ve sade, hiçbir şey
ekranın bir yerinden ötekine uçmaz. Okurun gözü üç şeyi hareketten öğrenir:

| An | Hareket (`anim`) | Hareketi azalt (aynı bilgi, durağan) |
|---|---|---|
| Açılma (akış → kapsül → grafik → kanıt, geri) | **Derinleşen çapraz geçiş** (220 ms × `motion.duration_scale`): eski sahne işaretinin merkezi çevresinde %5 büyürken ilk %70'te söner; yeni sahne kendi yerinde %3 küçükten (kapanışta büyükten) yerine oturarak belirir | Anında geçiş (çekirdek `transition_progress` = 1) |
| Kapsül canlı (geçerli, hikâye açık) | **Nabız**: işaretin çevresinde soluk bir halka yavaşça şişip söner (2,6 s; tepe saydamlığı 0,38, büyüme %14). Canlı olmayan durur | **Canlı noktası** işaretin üstündeki satırda, işaret renginde (iki kipte de çizilir) |
| Durum değişimi (cihazın gördüğü) | **Tek vuruş** (0,9 s), türe göre farklı biçim: doğdu = dışa tek halka; güçlendi/teyit = dışa iki hızlı halka; zayıfladı = içe soluk halka; geçersiz/artık tutmuyor = içe güçlü, kalın halka; kapandı (gerçekleşti, süresi doldu, geri çekildi) = işaretin üstüne oturup sönen disk | **Değişim etiketi**: aşama sözcüğü ("doğdu", "güçlendi", "geçersiz", "artık tutmuyor", "süresi doldu"…) işaretin üstündeki satırda, okurun bir sonraki bakışına kadar (iki kipte de çizilir) |
| Değişen öğe | **İz**: etiketin altında vurgu, vuruştan sonra 4 s içinde söner | Etiketin kendisi kalıcı |

Değişim körlüğü (RESEARCH §5.3): bütün kart yeniden çizilmez; nabız, vuruş ve iz yüzün üstüne
konan küçük kutulardır (yerel geçici sinyal). Yüz yalnız etiket ya da nokta değişince yeniden
boyanır (veri değişimiyle zaten boyandığı anda).

## Bilgi eşitliği (hareketi azalt'ta kayıp 0)

Durağan işaretler (canlı noktası, değişim etiketi) yüz sahnesinin parçasıdır ve **hareket
kipinden bağımsızdır**: aynı yüz `anim=1` ve `anim=0`'da bayt eşit sahne verir (sınama
`reduce_motion_loses_no_fact_and_paints_no_overlay`: her yüz iki kipte eşit; hareket katmanı
azalt'ta bir kez boşalır, sonra hiç gelmez; derinlik geçişi anında biter). Hareket yalnız
vurgu ekler; hiçbir olgu yalnız hareketle verilmez (Apple HIG Motion, WCAG 2.3.3). Düzeneğin
`reduced_motion_loss` ölçüsü (§G7.7) bu yüzden yapısal olarak 0 olmalıdır; düzenek koşmadı (ben
koşmadım, görev gereği).

Etiketin ömrü: cihazın bu oturumda **gördüğü** değişimler, okurun bir sonraki "son bakış"ına
kadar; son bakış varsa ayrıca bakıştan sonra bilinebilen geçişler (akışın kendi `fresh`
kuralı; en yeni bardaki "şimdi" bakış anında görülmüş sayılır). İlk akış taban çizgisidir
(sayfa açılırken var olanlar için vuruş ve etiket yok); sonradan ilk kez görünen bir kapsül
yalnız geçişi şimdiye kadar görülenlerin en yenisinden yeniyse "doğdu" sayılır (süzgeç
değişince eski bir kapsülün akışa girmesi doğum değildir).

## Nerede yaşıyor

- `crates/qp-grammar/src/transitions/nabiz.rs`: geçişin `shared`/`frame`'i (TRANSITIONS'ta
  `nabiz`), değişim türü (`Change`), canlılık, etiket sözcükleri, işaretlerin yeri (`places`),
  durağan işaretlerin çizimi (`draw_face`), bir karenin kutuları (`overlay`, saf işlev; trigonometri
  yok, nabız eğrisi `(4u(1−u))²`).
- `crates/qp-grammar/src/transitions.rs`: hareket tasarımı kaydı `MOTIONS` (`r4` = bugünkü,
  canlı işaret yok; `nabiz`), `MotionDesign { name, transition, marks }`.
- `crates/qp-wasm/src/shell/motion.rs`: cihaz tarafı — iki akış arasındaki durum değişimleri,
  vuruş saatleri, etiket ömrü, yüzlere durağan işaret ekleme (`face()` içinde), karenin hareket
  katmanı (`motion_marks(t)`: yüz başına kutu, CSS değerleri hazır), hareketi azalt bayrağı.
- Kabuk (ince): `set_reduced_motion(!anim || prefers-reduced-motion)` ve her animasyon
  karesinde `motion_marks(ts)` → anahtar başına bir `div.mk`, değerler olduğu gibi
  (`arch-check` 0 ihlal; `ARCH.md`'ye iki yöntem eklendi). Ayarlar'da "Hareket" seçicisi
  (`choices().motion`).
- Seçim bir **cihaz sunum ayarıdır** (`feed_order` gibi; protokol değeri değil): hareketi azalt
  da cihazın tercihidir. `nabiz` seçilince kendi geçişini `style` hareketinin `transition` alanıyla
  ister (otorite paylaşılan öğeleri bu tasarımla kurar).

## Düzenek için

- Ara kareler: `window.qp.freeze(p)` (bugünkü kanca, değişmedi) `nabiz` geçişinin donmuş karesini
  verir; kare `transitions::nabiz::frame(shared, from, to, p)` ile sahne tarifinden belirleyicidir.
  p = 0,25'te eski sahne yerinde (%1,25 büyümüş, saydamlık ≈ 0,71), yeni sahne kendi yerinde
  (≈ %2,5 küçük, saydamlık ≈ 0,04). **Uyarı (dürüstlük):** bu yön bilerek uçuş yapmaz; "taşınan
  parça nerede" (`m_where`) sorusu p = 0,25'te parçayı eski yerinde bulur, son karede yeni yerinde.
  §G6.4'ün şartı (p = 0,25 çeyreği ≠ son çeyrek) bu yüzden hedef kapsülde çoğunlukla sağlanır,
  ama "aynı nesne mi taşınıyor" (`m_same`) için uçan geçişlere göre zayıf kalması beklenir.
- Nabız/vuruş/iz kareleri: `window.qp.marks(t)` (yeni kanca) çekirdeğin `t` anındaki katmanını
  JSON verir; kare `t`'nin saf işlevidir (vuruş saati ilk sorulduğu karede başlar). Değişiklik yoksa boş döner: yalnız nabız varken aynı 50 ms adımında ikinci çağrı boştur, düzenek farklı adımlardan sorar.

## Değerler (kayda alınmalı: `Look.motion`)

Bugün `nabiz.rs`'te sabit; süreler `motion.duration_scale` ile çarpılır, eğri `motion.ease`'ten,
renkler ve boylar görünüşten okunur (`r4` ve `ornek` ile denendi).

| Değer | Bugün | Önerilen alan |
|---|---|---|
| Geçiş süresi | 220 ms (TRANSITIONS satırı) | (kayıt zaten çarpıyor) |
| Derinleşme / oturma | %5 / %3 | `motion.deepen`, `motion.settle` |
| Nabız periyodu, büyüme, tepe saydamlığı | 2600 ms, 0,14, 0,38 | `motion.pulse_ms`, `pulse_grow`, `pulse_alpha` |
| Vuruş süresi, ikinci halka gecikmesi | 900 ms, 260 ms | `motion.beat_ms`, `beat_gap_ms` |
| İz süresi, tepe saydamlığı | 4000 ms, 0,28 | `motion.trail_ms`, `trail_alpha` |
| Halka payı | 4 px | `glyph.ring_pad` |

Görünüşten okunanlar: halka kalınlığı `surface.line` katı; nokta ve halka rengi işaretin yön rengi
(`up`/`down`; yönsüzde `text`, çünkü `r4` açık temada `neutral` 2,8:1); etiket `types.label`
(boy, yazı tipi), rengi `text` (doğdu, güçlendi), `invalid` (zayıfladı, geçersiz), `muted`
(kapandı); iz zemini `accent` ya da `invalid`, köşesi `surface.pill_radius`; etiket satırı
`space.plot` üst payında (sesli yüzde boş), dolu ise alt payda; aralık `space.scale`.
Sert eşikler sınandı (`still_marks_keep_contrast`): etiket ≥ 4,5:1, nokta ≥ 3:1, `r4` ve `ornek`,
iki tema. Yer sınaması (`the_marks_find_room_in_every_look`): etiket satırı işaretle çakışmaz,
yüzün içinde kalır (kayıttaki her görünüş).

## K52

Bkz. `K52.md` (aynı klasör): JIT açık tahminde eşikler tutuyor; nabız ve geçiş kare süresini başvurudan ayırt edilir biçimde değiştirmedi; hareket katmanı JIT kapalıda ≈ 0,14 ms/kare (nabız-yalnız kareler 50 ms adımla).

## Bilinen sınırlar

- Ekranda olmayan bir yüzün vuruşu görünmez (vuruş 0,9 s; yüz sonradan görünürse yalnız iz ve
  etiket kalır). Bilgi etiketle korunur.
- Etiket ömrü sayfa yeniden açılınca (yeni çekirdek) sıfırlanır; son bakış verilmişse bakıştan sonra
  bilinebilen geçişler yine etiketlenir.
- Yüzü olmayan okur (metin yüzü, yetkisiz okuyucunun sahnesiz hali) işaret almaz: metin yüzü zaten
  geçerliliği sözle söyler.

## Öneriler (kapsam dışı)

- Sözlüğe yeni kavramlar: **nabız** (canlı kapsülün işaretindeki yavaş halka), **vuruş** (durum
  değişiminin tek, türe özgü hareketi), **iz** (değişen öğenin sönen vurgusu), **canlı noktası**,
  **değişim etiketi** (aşama sözcüğü, son bakışa kadar).
- `Look.motion`'a yukarıdaki alanlar.
- Düzenek `motion` sorusuna uçuşsuz tasarımlar için bir "aynı kapsül mü" görsel ipucu sorusu
  (ör. işaretin yerinde kalıp kalmadığı) eklenebilir; yoksa bu yön tanım gereği `m_same`'de geride
  kalır.
