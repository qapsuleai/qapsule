# M20 alternatifi `tesir` — "iddia → dayanak → doğrulayan, yukarıdan aşağı"

Dal `r5/m20-tesir`. Açılış: `?open=tesir` (Ayarlar → Açılmış kapsül (M20)); başvuru (bugünkü
kart) `?open=r4` ya da parametresiz. Görünüş ve dil bağımsız: `?open=tesir&look=ornek`,
`&lang=en`, `&theme=light`, yetkisiz okur `&principal=visitor`.

Görüntüler (sentetik veri, 390 px, DPR 1, `canvas` yolu): `tesir_tr_dark.png`,
`tesir_ornek_light_en.png` (`?look=ornek&theme=light&lang=en`), `tesir_visitor.png`
(yetkisiz okur, metin sütunu), başvuru `r4_reference_tr_dark.png` (aynı kapsül, bugünkü
kart).

## İlke

Açılmış kapsül **tek, kaydırılabilir bir sütun**dur. Okur yukarıdan aşağı inerken her blok bir
soruyu cevaplar ve bir sonrakini sordurur; geri dönmek, sekme değiştirmek, katman açmak yok:

| # | Blok | Soru | İçerik |
|---|------|------|--------|
| 1 | **İddia** | Ne oldu, hâlâ geçerli mi? | İşaret + kapsülün başlığı (seçili dilde: eşik adıyla ve değeriyle, ne zaman, geçerlilik). Geçerli değilse `invalid` renginde (yüzle aynı kural). Yüzde okunan cümlenin aynısı: açmak okurun bildiği hiçbir şeyi değiştirmez. |
| 2 | **Dayanak** | Bu iddia neye dayanıyor? | Eşiğin **adı** (sesin sözlüğünden, başlıkta ilk geçen terim) ve **değeri** (kanıttan, okurun dilinde biçimli: "Ortalama · 159,86"); kanıt ailesi ve lisansı ("Fiyat verisi · herkese açık, cihaz yeniden hesaplar"); **kanıtın küçük resmi** (kanıtın oturduğu fiyat ya da gösterge çizgisi ve kanıt geometrisi, yalnız kökenle bağlı, K33); sesin ayrıntısı (neden, şimdi ne durumda). |
| 3 | **Kim doğruladı** | Bunu kim doğruladı? | Kendi biçiminde tek satır: "Sunucu doğruladı" ya da cihaz kendi verisinde sınadıysa "Bu cihazda sınandı" (açılmış katmanın güven işaretiyle aynı sözcükler, K48), "Bu cihazda sınanıyor", "Kaynaklar farklı"; renk `accent` / `valid` / `muted` / `warn` (renk tek başına anlam taşımaz: sözcük taşır). Altında bunun burada ne demek olduğu ("Lisanslı veri: bu cihaz kendisi sınayamaz." / "Sunucu da doğruladı; telefon aynı kuralı kendi verisiyle yeniden işletti."). Bloğun **tamamı** dokunma hedefidir: açıklama sözlük gibi açılır (`Kim doğruladı?`, ≤ 160 karakter, rakamsız). |
| 4 | **Karne** | Daha önce ne kadar tuttu? | Sesin karne cümlesi (beklentinin kendisi, sayılmış; "benzer durum" terimi dokununca açılır). |

**Yetkisiz okur** (K33/K34, A32): dayanak satırı şablonlu bir satıra dönüşür — **neyin**
kilitli olduğu ve **hangi yetkiyle** açıldığı: "Kilitli: flow/SYN03 yetkisi" / "Akış verisi
lisanslı; değerler bu yetkiyle açılır." (lisans sınıfı `public` ise "Kilitli: SYN21 fiyat
verisi yetkisi"). Değer ve geometri yoktur (katman kurulmaz; derleyici değeri okurun
sağlayıcısından çözdüğü için yetkili görünüm de rakam taşıyamaz). Veri yetkisi hiç olmayan
okurun (demo `visitor`) sahnesi yoktur: açılmış katman aynı blokları **satır sonlu metin
sütunu** olarak gösterir (`open().text`; dokunma yok, güven satırının anlamı yazılı).

## Neden böyle (karşılaştırma: bugünkü kart)

- Bugünkü kartta iddia (üst satır, HTML), ayrıntı + karne (sahne) ve kanıt (ayrı derinlik,
  ayrı düğme) üç yerdedir; "neye dayanıyor" sorusu grafiğe ya da kanıt derinliğine inmeden
  cevaplanmaz; "kim doğruladı" yalnız üst çubuktaki küçük `tiny` yazıdır, ne anlama geldiği
  hiçbir yerde yoktur.
- `tesir`'de üç M20 sorusu **sıfır ek dokunuşla** (kapsülü açan tek dokunuş) aynı ekranda,
  sorulma sırasıyla cevaplanır; "kim doğruladı"nın anlamı bir dokunuş. Kanıt derinliği ve
  grafik yerinde durur (sütunda terim dışı her dokunuş grafiği açar; "Kanıt" düğmesi aynen).
- Dayanak satırı eşiği **adıyla** verir: adı yeni bir sözlükten değil, sesin kendi sözlük
  teriminden alır (başlık ve sözlükle aynı sözcük; K49) — terim dokununca açılır.

## Ölçüler (düzenek koşulmadı; okuyucu/eleştirmen ölçümünü orkestratör koşar)

- **Dokunma sayısı** (akıştan): iddia, dayanak, kim doğruladı, karne: **1** (kapsülü açmak).
  "Kim doğruladı" anlamı: 2. Bugünkü kart: dayanak için 2–3 (kanıt derinliği), güvenin anlamı
  yok. Düzenek M20 kümesini kanıt derinliğinde çekiyor (`tapTo(...,'evidence')`); `tesir`'in
  cevapları kapsül derinliğindedir — öneri: M20 görüntüsü **kapsül derinliğinde** de çekilsin.
- **Yetkisizde rakam:** sahne ve metin sütununda sembol ve yetki adı dışında rakam yok
  (`tests/open_tesir.rs`, `an_unentitled_reader_gets_the_templated_basis_and_no_number`,
  `the_text_column_says_the_same_without_a_scene`); kanıt değeri sahne JSON'unda yok.
- **K52** (kalibre benzetim, `shells/web/tests/calibrate.mjs` yapılandırmaları; kapsül
  derinliği evresi: ilk açık kapsül açık, oynatma 4 adım/sn, 10 sn, sütun tekerlekle aşağı
  yukarı kaydırılıyor; 3 tekrar ortancası; ölçüm aracı ajan karalamasında, depoda değil):

  | Yapılandırma | Tasarım | Çerçeve | Sahne boyama p95 | Sahne kurma p95 | Tik p95 | Sahne boyaması (10 sn) | Uzun kare, kaydırırken | Uzun kare, dururken |
  |---|---|---|---:|---:|---:|---:|---:|---:|
  | `phone-jit-est` | `tesir` | 390×670 | 2 ms | 1 ms | 1 ms | 8 | 0 / 0 / 0,6 % | 1,3–1,6 % |
  | `phone-jit-est` | `r4` | 390×300 | 2 ms | 1 ms | 1 ms | 39 | — (katman kaymıyor) | 0–0,2 % |
  | `phone` (JIT kapalı) | `tesir` | 390×670 | 10 ms (×1,85 sapma: ~18) | 6 ms | 1 ms | 8 | 0,6 % | 2,1–2,4 % |
  | `phone` (JIT kapalı) | `r4` | 390×300 | 5 ms (~9) | 3 ms | 1 ms | 40 | — | 1,0–1,5 % |

  K52 eşikleri (JIT açık tahmini): sahne boyama p95 ≤ 8 ms ✓, tik ≤ 8 ms ✓, kaydırırken uzun
  kare ≤ %1 ✓ (ortanca 0). JIT kapalı: kaydırırken ≤ %10 ✓. **Risk:** uzun sütunun her
  yeniden boyanması (yeni bar geldiğinde: işaretin tazeliği, başlığın "ne zaman"ı) bir uzun
  kare (~55 ms) doğuruyor; dururken uzun kare payı %1,3–1,6 (bugünkü kart ~%0). İlk sürümde
  küçük resim canlı bara kadar uzanıyordu: her tikte boyama (40/10 sn) ve kaydırırken %1,2–1,9
  uzun kare (eşik aşılıyordu). Düzeltme: küçük resim **kanıtın kendi penceresini** gösterir
  (başlangıcından 5 bar önce → bitişinden 5 bar sonra, en az 30 bar; oluşmakta olan son bar
  hariç) — kanıtın resmi her tikte oynamaz; "şimdi ne durumda" ayrıntının ve grafiğin işi.
  Standart K52 senaryosu (akış + kapsülden grafiğe) `?open=tesir` ile: aşağıda "Standart K52".
- **Standart K52** (`rounds/R5/experiments/harness/glance/k52.mjs`'in `?open=tesir` ekleyen
  kopyası; akış evresi + kapsülden grafiğe; 3 tekrar): **geçti**. `phone-jit-est`: tik p95
  1 ms, yüz 1, sahne 2, ilk kapsül 211 ms, kaydırma kare p99 19 ms; `phone`: tik 1,6, yüz 1,
  sahne 5,55 (sapmalı), ilk kapsül 219 ms, p99 22 ms.
- **Paket:** wasm ham 5 371 092 → 5 372 243 bayt (+1,2 KB), gzip 1 577 788 → 1 577 882.

## Çekirdekteki yer (kabuk ince)

- `crates/qp-grammar/src/open.rs` (kayıt `OPENS`: `r4`, `tesir`; `DeviceCheck`, `TextInput`),
  `open/tesir.rs` (görünüm, yükseklik, metin alanları `open.*`, güven terimi, metin sütunu).
- Gramerde en küçük bağlantılar (var olan sahneler bayt eşit): panele göre `text`'e `x2` (kırılma
  kenarı); en alttaki panel kartsa ay etiketleri değer ölçekli son panelin altına; bulgu
  satırında `open.*` alanları; `terms::mark`'ta tasarımın katman terimi (bütün panel hedef);
  sağlayıcıda `reader_lang`, `device_check` (varsayılan yok).
- Oturum: `style.open`, `ViewState.open` (varsayılan yazılmaz; bilinmeyen ad `invalid`);
  kapsül derinliği tasarımın yüksekliğinde açılır (yüze dokunarak da); tasarım değişince
  hatırlanan kapsül çerçevesi unutulur; sahnesiz okur için kapsül derinliğinde de
  `presentation.facts` (okura göre ayıklanmış). Görüntüleyici: `open_scene_with` (okurun dili,
  cihazın sınaması).
- Kabuk çekirdeği: `frame_at` (kapsül derinliği çerçevesi), `open().text` (sahne üstünde boş,
  sahnesizde metin sütunu), `open().style.touchAction = pan-y` (katman parmakla kayar),
  `wheel` döner değeri (tekerlek grafiğin değilse sayfa kaydırır).
- Sayfa: `?open=`, Ayarlar seçicisi, `#opentext` satır sonlarını korur (yapısal CSS), katman
  kayması `scroll` kipinde ölçülür, `open_desc` (yalnız sınamalar).

## Kayda alınması gereken değerler (bugün `tesir.rs` içinde sabit, `r4`'ten türeyen değil)

Hepsi satır sayısı ya da bar sayısıdır; boy, renk ve boşluklar zaten `Look`'tan:
`CLAIM_LINES = 4` (başlık), `WHY_LINES = 5` (ayrıntı), `LEDGER_LINES = 3` (karne),
`NOTE_LINES = 2` (blok notu), `THUMB_LINES = 7` (küçük resim yüksekliği, gövde satırı),
`THUMB_LEAD = 5` (küçük resim kenar barları). Kullanılan `Look` alanları: `types.*`
(`headline`, `title`, `body`, `label` boy ve satır), `space.scale[0]` (blok içi),
`scale[1]`, `scale[4]` (blok arası), `space.plot`, `panel_gap`, `glyph.card_px`, `glyph_gap`,
`evidence.color`, `evidence.note_color` (kilitli satır), renk rolleri `muted`, `invalid`,
`accent`, `valid`, `warn`. Öneri: `Look`'a bir `open: OpenLook { claim_lines, …, thumb_lines }`
kaydı.

## Bilinen sınırlar ve açık sorular

- Sütunun yüksekliği görünüşe göre sabittir (r4 670 px, `ornek` 774 px); kısa içerikte blok
  aralarında boşluk kalır, çok uzun içerik "…" ile kesilir (sınama: demo görünüşlerinde hiçbir
  metin kesilmiyor).
- Sesin `r4` olduğu bağlantıda (ses yok) başlık R3 cümlesi, ayrıntı boş, eşiğin adı genel
  ("Eşik"); tasarım gösterimin varsayılan sesi (`marla`) için kuruldu.
- Kanıt derinliği (grafik + HTML kanıt satırları) değişmedi.
- `qp drive` (kabuksuz eşlik sınaması) `open_scene`'i dil vermeden çağırır: varsayılan tasarımda
  fark yok; `?open=tesir` eşlik sınamasına girmez.
- Yeni kavram önerisi (sözlüğe): **Dayanak** (açılmış kapsülde eşik + kanıt ailesi + kanıtın
  küçük resmi; yetkisizde "neyin, hangi yetkiyle" satırı).
