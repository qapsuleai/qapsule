# M10 alternatifi `tutamir` — cümle çapası, işaret gösterir

Kod: `crates/qp-grammar/src/transitions/tutamir.rs` (kayıt: `transitions.rs` `TRANSITIONS`'ta
tek satır, `duration_ms: 420`). Bu not okuyucuya gitmez.

## Amaç

Kapsülden grafiğe inerken okuyucu üç şeyi kaybetmemeli (SPEC §4): **neye** baktığını (G1:
grafikte işaretli yer kapsülün söylediği şeyin neresi), **aynı şeye** baktığını (G2) ve
iddianın **hâlâ geçerli** olup olmadığını (G3). Tasarım tek bir ipliğe dayanır: okuduğu cümle
yerinde kalır, işaret o cümlenin anlattığı yere gider.

## Ne gösterir / ne göstermez

- **Cümle ekranda kalır.** Kabuk açık her derinlikte cümleyi sahnenin üstünde gösterir
  (`#opentext`). Geçiş bu satırın üzerine hiçbir şey çizmez: kapsül ekranında okunan cümle,
  aynı yerde, grafiğin başlık satırı olur. Kartın içindeki cümle kopyası kartla birlikte
  söner (iki aynı cümle üst üste binmez). Akıştan kapsüle geçişte `headline` korunur ve yüzden
  karta tam opaklıkla taşınır (cümle o adımda da gözden kaybolmaz).
- **İşaret olgunun yerine uçar.** Kapsülün işareti (glif), kartın sol üstünden grafikte
  olgunun barı (sütun ortası) × olgunun seviyesi (`evidence:` kutusu, yoksa `fact:` kutusu,
  yoksa fiyat paneli ortası) noktasının hemen üstüne uçar, grafiğin kendi işaret boyuna
  (32 px) küçülür, `p` 0,6'dan sonra orada durur ve `p` 0,8–1'de grafiğe devreder. Nokta
  panelin üst kenarına çok yakınsa işaret noktanın altına konur.
- **Önce kanıt, sonra bağlam.** Grafiğin kanıt öğeleri (seviye/bant çizgileri, işaretli
  barın sütunu) yeni sahneden, grafiğin geri kalanından **önce** belirir (`p` 0,1–0,4; ikisinde
  de varsa karttaki kutusundan grafiğe büyüyerek). Grafiğin tamamı `p` 0,3–0,95'te gelir.
  Böylece `p = 1/3` karesinde ekranda neredeyse yalnız cümle, kanıt ve uçan işaret vardır.
- **Gerisi sakin söner.** Kartın geri kalanı (`backdrop`) `p` 0–0,3'te söner; kartın fiyat
  paneli grafiğin fiyat paneline kayar (0–0,6) ve bu sırada söner (0,1–0,55). Başka ortak
  öğeler (ör. karne satırı) kayar ve geç söner (0,75–1).
- Göstermediği: yeni metin, yeni renk, grafiğe kalıcı işaret. Grafik bütün geçişlerde aynıdır
  (`depth::chart_view`); son ekrandaki işaretleme grafiğin kendi vurgusudur.

## Yerleşim (öğeler, `Opening.shared`)

| Öğe | önce | sonra | kaynak | ne olur |
|---|---|---|---|---|
| `backdrop` (kendi adı) | kartın tamamı | — | eski | yerinde söner |
| `price` | kart fiyat paneli | grafik fiyat paneli | eski | kayar, söner |
| `headline` (yalnız akış → kapsül) | yüz | kart | eski | tam opak kayar |
| `evidence:*`, `fact:*`, `bar:*` | kartta varsa kutusu | grafik kutusu | **yeni** | erken belirir |
| `pin` (kendi adı; kapsül → grafikte `glyph`'in yerine) | kart işareti | olgu noktasının üstü | eski | uçar, durur, devreder |

Tek kutulu öğeler (önce ya da sonra yok) ortak sayılmaz; düzeneğin ölçüleri yalnız iki kutulu
öğeleri sayar. `pin` gerçekten taşınan işarettir: iki kutusu vardır ve sayılır; `glyph` adı
grafiğin kendi işaret çapasını anlattığı için kullanılmadı.
Kare kuralı: öğe listesi (sayı, sıra, kaynak, `src`) her `p` için aynı; `p` ile yalnız kutu
ve opaklık değişir (birim testi `frame_rule_and_ends`). Hesapta yalnız IEEE temel işlemleri
(yumuşatma `3u² − 2u³`); `detmath::px` ile yuvarlama.

## Metin şablonları

Yeni metin yok. Ekrandaki cümle ürünün kendi cümlesidir (iki dil, kabuğun dili):

- TR: "SYN06 2 bar önce 114,27'deki direnci yukarı kırdı (kapanış 116,27): yukarı kırılım. …
  Hâlâ geçerli. …"
- EN: aynı cümlenin İngilizce gerçekleştirimi (`realize`), tasarım değiştirmez.

## Düzeneğin hesabı (`m8910 m10 --transition tutamir`, kapsül → grafik)

| durum | ortak | öğeler | alan payı önce / sonra | kayma ort. / en çok (px) | dokunma |
|---|---:|---|---|---|---:|
| 6 | 3 | evidence, price, pin | 0,2813 / 0,6530 | 90,57 / 143,01 | 2 |
| 101 | 4 | bar, evidence, price, pin | 0,2861 / 0,6816 | 135,26 / 262,99 | 2 |
| 112 | 3 | bar, price, pin | 0,2676 / 0,6706 | 141,73 / 276,86 | 2 |
| 3 | 4 | bar, evidence, price, pin | 0,2861 / 0,6707 | 151,27 / 324,58 | 2 |
| **ort.** | **3,5** | | **0,2803 / 0,6690** | **129,71** / 251,86 | **2** |

Başvuru `r3` ile alan payı ve ortak sayısı aynı (işaret grafiğin kendi işaret boyuna iner);
ortalama kayma daha yüksek (r3 124,86): işaret köşeye değil olgu noktasına gider — tasarımın
amacı budur, bedeli de budur.

## Bilinen zayıflıklar

- `p = 2/3` karesinde iki işaret görünür: uçan işaret (olgu noktasında) ve grafiğin kendi
  işareti (sütunun üstünde, belirmekte). Okuyucu ikisini ayrı şey sanabilir.
- Kartın fiyat paneli grafiğin paneline gerilerek kayar; kartın çizgisi grafiğin barlarıyla
  hizalı değildir (kısa süre, sönerek). `p = 1/3` karesinde hâlâ yarı görünür.
- Cümlenin sürekliliği kabuğun yerleşimine dayanır (cümle satırı sahnenin üstünde); tasarım
  koordinat varsaymaz ama bu satırı vurgulayamaz (kabukta özel kod yok).
- İşaret noktayı **üstünden** gösterir; okuyucu etiketi işaretin altında arar. Olgu noktası
  panelin üst kenarındaysa işaret altına geçer (tutarsız yön).
- Olgunun kanıt öğesi yoksa (yalnız `fact:` noktaları) seviye noktaların kutusunun ortasıdır;
  eğik bir çizgide bu tam olgu barının seviyesi olmayabilir (düzenekle aynı kural).
- Geri dönüşte (grafik → kapsül) `pin` yok; işaret `r3` gibi kendi kutusuna kayar.

## Öneriler (kapsam dışı)

- Düzenek sabit port kullanır (`m10::PORT` 8472, `demo_shots.mjs`); aynı makinede eşzamanlı
  koşan düzenekler aynı portu ister ve biri ötekinin sunucusuna bağlanabilir. Bu tasarımın
  ekranları planın kopyasıyla özel bir portta (8573) alındı; düzeneğe `--port` ya da boş port
  seçimi eklenmeli. Ayrıca `shells/web/pkg/sizes.json` yoksa sayfa 404 hatası verir ve
  `demo_shots.mjs` durur (yalnız `scripts/demo.sh` yazıyor).
