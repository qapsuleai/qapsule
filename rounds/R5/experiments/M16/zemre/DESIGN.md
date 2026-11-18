# M16 — `zemre`: zaman şeridi (alışılmışın dışı yön, K18)

Açılış: `?look=zemre&voice=marla&face=zemre` (Ayarlar → Görünüş `zemre`, Yüz `zemre`).
Kod: `crates/qp-grammar/src/look.rs` (`ZEMRE`), `crates/qp-grammar/src/faces/zemre.rs`,
yazı tipi `crates/qp-render/assets/fonts/AtkinsonHyperlegibleNext-{Regular,Bold}.ttf` +
`OFL-AtkinsonHyperlegibleNext.txt`; üretim betiği `build_font.py` (bu klasör).

Bu yön bugünkü tasarımı görmeden kuruldu: `r4` görünüşü, var olan yüzler, altınlar, `docs/FACE.md`
ve önceki deney görselleri açılmadı. Girdi: `NORTH_STAR.md`, `docs/CONCEPTS.md`, `docs/CAPSULE.md`,
`docs/LANGUAGE.md`, `rounds/R5/RESEARCH.md`, görev tarifi.

## İlke

"İnsan grafiğe bakmaz; grafik insana haber verir." Kapsül bir **hikâyedir** ve hikâyenin
doğal bir ömrü vardır (`horizon_bars`, CAPSULE §3). Bu yönde kapsülün yüzü iki soruyu
ayrı ayrı ve aynı yerde cevaplar:

1. **Ne oldu, hangi yöne, hâlâ geçerli mi?** — büyük işaret (56 px, sol üst) + kalın başlık
   cümlesi (sesin başlığı, metin sabit) + karne cümlesi (sesin karnesi, soluk).
2. **Zaman neresinde?** — altta **ömür şeridi**: penceresi saat ya da sabit bar sayısı değil,
   **iddianın kendi ömrü** olan küçük bir zaman şeridi. Sağ kenar *şimdi*dir; dikey mürekkep
   çizgi iddianın doğduğu andır.

Kart çerçevesi yok (yüzey düz, gölge yok); yüzler kâğıt üstüne mürekkeple basılmış bir
günlüğün satırları gibi alt alta durur. Akışın zaman duygusu ayrı bir akış düzeninden değil
yüzün kendisinden gelir: her yüzün sağ kenarı aynı "şimdi"dir ve her yüzün sağ oluğunda o anki
fiyat etiketi durur, böylece akışta yukarıdan aşağı inen bir **"şimdi sütunu"** oluşur; doğum
çizgileri bu sütuna yakın olanlar tazedir, sola kaymış olanlar ömrünü doldurmuştur.

## Yeni görsel kanallar (her biri: ne taşır, nasıl ölçülür)

| Kanal | Ne taşır | Nasıl ölçülür |
|---|---|---|
| **Yaş: doğum çizgisinin şeritteki yeri** | İddianın doğal ömrünün ne kadarının geçtiği. Pencere = ömür + ömrün dörtte biri kadar öncesi; ömrünü aşmış iddiada yaş + aynı öncesi. Yer = `(doğum − pencere başı) / (pencere − 1)`: şimdi doğan 1,0 (sağ kenar), ömrünün yarısında ≈ 0,6, ömrünün sonunda 0,2, aşmış olan 0,2'nin solunda | Hesap: `faces::zemre::birth_fraction` (saf işlev) ve sahnede `born` katmanının x'i; birim testleri yaşla **kesin azalan** olduğunu ve uç değerleri sınar (`window_holds_the_horizon_and_follows_the_age`, `birth_moves_left_as_the_claim_ages`). Okuyucu sorusu önerisi: iki yüzden "hangisi daha yeni?" (doğru cevap = doğum çizgisi sağda olan) |
| **Şimdi sütunu** | Akıştaki bütün yüzlerin ortak "şimdi"si ve fiyatın şimdiki değeri | Her yüz sahnesinde şerit panelinin sağ kenarı aynı x (390 − 64 px); son değer etiketi orada. Hesap: sahnelerde `life` panelinin `rect.right` değeri sabit |
| **Kapanmış işaret solmaz** (`closed_alpha` 255) | Tazelik kanalı kapanmış işarette kapatıldı; zaman şeritte taşınır, işaret her zaman tam karşıtlıkta | Kayıt sınaması `zemre_meets_the_hard_thresholds` (kapanmış ≥ 3:1) |
| **Açık işaretin solma tabanı** (`fresh_alpha_min` koyu 172, açık 180; M16 öncesi güncelleme) | Açık işaret tazelikle solmayı sürdürür ama 3:1'in altına inmez; güven plakası tabanı yok (`plate_alpha_min` 0, plaka işaret değil) | `look::tests::zemre_meets_the_hard_thresholds` ve `glyph::outcome::tg5::tests::zemre_signs_keep_3_to_1_at_freshness_zero` (iki alfabe × 6 tür × 3 yön × 4 evre × iki tema, tazelik 0) |

Ölçülemeyecek süs eklenmedi (hareketli nabız, doku, gölge yok).

## Kararlar ve gerekçeleri

- **Kâğıt ve mürekkep, varsayılan açık tema.** Gündüz okunurluğu ve "günlük" duygusu; koyu tema
  aynı rollerle. Zemin `#f6f3ec` (sıcak kâğıt), metin `#1c1a17`.
- **Anlam renkleri mavi / koyu zinober / mürekkep / gri** (yönsüz işaret metin rengindedir: siluet alfabesi her işaretin kenarını metin renginde çizer, düzenek işaretin baskın rengini ölçer; yönsüzü mürekkep yapmak onu yukarıdan renkle değil şekille ayırır) (yeşil-kırmızı değil): yeşil-kırmızı çifti
  protan/deutan okurda çöker (RESEARCH §2.2, §8.3). Gri tonlamada da ayrılmaları için dört renk
  açıklık ekseninde aralıklı seçildi (açıkta mürekkep L*≈10, zinober ≈22, mavi ≈41, gri ≈51; koyuda gri ≈47, zinober ≈59, mavi ≈73, mürekkep ≈92).
  Yön zaten işaretin şekliyle de okunur (renk tek başına anlam taşımaz).
- **İki aile:** başlık, gövde, etiket **Atkinson Hyperlegible Next** (Braille Institute; açık iç
  boşluklu, karışan harfleri ayrıştırılmış, OFL, RFN yok, Türkçe tam); eksen ve fiyat etiketleri
  (rol `figure`) **IBM Plex Mono** (zaten gömülü, tablo rakamı; alışkın kullanıcının terminal
  alışkanlığı). Başlık kalın (700), gövde 400 (Live Activities: orta ve üstü ağırlık, RESEARCH §1.3).
- **Tablo rakamı:** Atkinson Next'in varsayılan rakamları orantılı; fontdue `tnum` uygulamadığı
  için cmap'te 0–9 `tnum` gliflerine yeniden eşlendi (ölçüm: on rakam da 632 birim).
- **Aralama ve bağ yok:** alt kümede GSUB/GPOS atıldı; fontdue ikisini de uygulamaz, tuval
  `fontKerning = 'none'` ile çizer → `ref` ve `canvas` aynı yerleşim.
- **İşaret büyük ve tek:** yüzde 56 px, kartta 48 px; ekranda başka hiçbir öğe bu boyda ve
  doygunlukta değil (F9 tek vurgu). İkincil metin (`muted`) koyu tutuldu: ilk sürümde sunulan
  sonuç stili (`tg5`) tutmayan iddianın işaretini `muted` × 0,55 ile çiziyordu. M16 öncesi
  güncellemeden beri (main `6808268`) bu iz `closed` renginde ve `closed_alpha` tabanında
  çiziliyor; `muted` gerekçesi tarihsel kaldı ama görünüm değiştirilmedi (yalnız uyum). Bedeli: soluk metin ile ana metin arasındaki ayrım boy/ağırlıkla
  kuruluyor, renkle değil.
- **Alt kenar boşluğu 42 px:** kabuk yüzün sağ altına güven işaretini ("Sunucu doğruladı")
  koyar; şeridin ay etiketleri onunla çakışmasın diye.
- **Kart yok, köşe yuvarlak (14 px) yalnız kabuğun düğmelerinde;** dokunma hedefleri 44–48 px.

## Sert eşikler (kendi hesabım)

Kayıttan (birim testi `look::tests::zemre_meets_the_hard_thresholds`) ve `rounds/R5/experiments/harness/glance/gl/color.py`
ile (düzeneğin kendi renk işlevleri; ΔE00, Viénot protan/deutan, Brettel tritan, gri):

Kayıt renkleri (tema başına; `final.py` hesabı, düzeneğin `color.py`'si):

| rol | açık: karşıtlık | koyu: karşıtlık |
|---|---|---|
| metin | 15,67 | 14,95 |
| soluk metin (`muted`) | 11,82 | 8,45 |
| uyarı (`warn`, kanıt notu) | 6,75 | 10,86 |
| yukarı | 5,75 | 8,69 |
| aşağı | 10,09 | 5,62 |
| kapanmış / artık geçerli değil | 3,83 | 3,65 |
| tutmayan işaret (`tg5` izi `closed`, taban `closed_alpha` 255) | 3,83 | 3,65 |
| açık işaret, tazelik 0 (`fresh_alpha_min` açık 180 / koyu 172): yukarı | 3,19 | 4,57 |
| açık işaret, tazelik 0: aşağı | 4,77 | 3,17 |
| açık işaret, tazelik 0: yönsüz (mürekkep) | 6,22 | 7,26 |

Anlam renkleri çift çift ΔE00, beş koşulun en küçüğü (normal, gri, protan, deutan, tritan):

| çift | açık | koyu |
|---|---|---|
| yukarı–aşağı | 12,3 | 11,0 |
| yukarı–yönsüz | 22,6 | 13,0 |
| yukarı–kapanmış | 10,7 | 22,9 |
| aşağı–yönsüz | 10,4 | 23,8 |
| aşağı–kapanmış | 22,8 | 12,1 |
| yönsüz–kapanmış | 32,6 | 35,1 |

Bilinçli açık: `warn` (yalnız metin, hiçbir zaman işaret değil) gri koşulda bazı anlam renklerine
yakın (açıkta `warn`–`invalid` 1,0; koyuda `warn`–yukarı 4,1). Uyarı her zaman sözcükle gelir.

Düzeneğin kendisiyle, M16 öncesi güncellemeden sonra (main `6808268` + `9c01848` üstüne yeniden
temellendirilmiş dal; `glance.py probe/states/shoot --voice marla` + `compute`, yalnız bu tasarım,
52 çekim, tr/en × açık/koyu; okuyucu/eleştirmen koşulmadı; bozuk derleme kullanılmadı):

| ölçü | sonuç | önce (ilk sürüm) |
|---|---|---|
| metin karşıtlığı (en kötü / eşik) | 6,14 / 4,5 ✓ (kanıt ekranı "Bu kanıt şu yetkiyi gerektirir:") | 6,14 ✓ |
| en küçük yazı | 12 px ✓ (kabuğun güven işareti; sahne rolleri ≥ 13) | 12 ✓ |
| dokunma < 44 px | 0 ✓ | 0 ✓ |
| işaret karşıtlığı (en kötü, gövde = en opak çizgi) | **3,11 ✓** (`continuation.down`, zayıflamış) | 1,40 ✗ |
| renk çiftleri (tema başına, Ek G4-2) | **✗** — açık 3 çift, koyu 1 çift (aşağıda) | 2 çift ✗ (temalar arası) |
| K52 | ✓ | ✓ |
| yüz karakteri ort./en çok | 170,2 / 182 | 170,2 / 182 |

Renk kapısının başarısız çiftleri (ΔE00, beş koşulun en küçüğü; eşik 10). Düzenek artık işaret
rengini en opak çizgiden alıyor (§G7.2): `siluet` işaretin kenarını metin renginde çizdiği için
bu koşudaki yukarı ve yönsüz hedeflerin gövdesi mürekkep çıkıyor (yön rengi dolguda kalıyor); yukarı ve yönsüz aynı renk (ΔE 0, "renkle kodlanmamış", kapıdan muaf, şekille ayrılır):

| tema | çift | en küçük ΔE00 (koşul) |
|---|---|---|
| açık | `open/valid:down` (zinober, α 204) – `open/valid:up` / `open/valid:flat` (mürekkep gövde) | 2,5 (gri) |
| açık | `closed/failed` (`closed` 255) – `open/no_longer:down` (`closed` α 225) | 5,27 (gri) |
| koyu | `closed/failed` – `open/valid:down` | 6,6 (gri) |

Bunlar yönün renk seçiminin (gri koşulda zinober ile mürekkebin açıklıkları yakın; kapanmış ile
"artık geçerli değil" aynı renk rolü) ve düzeneğin yeni gövde kuralının sonucu; bu güncellemede
renkler değiştirilmedi (tarif: yalnız taban uyumu).

## K52 ve paket boyu

`glance.py k52 --reps 3 --voice marla` (yeniden ölçüm, M16 öncesi güncellemeden sonra; kalibre `PHONE` ve JIT açık tahmini, `canvas`):

| yapılandırma | tik p95 | yüz p95 | sahne p95 | ilk kapsül | kaydırma p99 kare |
|---|---|---|---|---|---|
| `phone` | 1,6 ms | 2 ms | 5,55 ms | 252 ms | 29 ms |
| `phone-jit-est` | 1 ms | 1 ms | 1 ms | 207 ms | 19 ms |

Bütçe (≤ 8 ms) içinde (`pass: true`). "Önce" için `r4` bu yön tarafından açılmadı (yönerge: bugünkü
tasarım görülmez); karşılaştırmayı orkestratör aynı düzenekle yapar.

Paket: iki yazı tipi dosyası WASM'a aynen girer: 24 060 + 24 032 = **48 092 bayt ham**
(gzip -9: 13 072 + 13 559 = 26 631; brotli 11: 12 103 + 12 625 = 24 728). Bu dalın paketi
`wasm_raw` 5 453 299, `wasm_gzip` 1 620 547, `wasm_br` 1 064 750 (`pkg/sizes.json`, main üstüne yeniden temellendirildikten sonra; ilk sürümde 5 373 215 / 1 601 664 / 1 052 667). Plex Mono
yerinde kalır (rakam rolü).

## Bilinen sınırlar (yönün dışında, ölçümü etkileyebilir)

- ~~Açık işaretin tazelik solması ve `tg5`'in tazeliğe göre soldurması kayıttan denetlenemiyor~~:
  main'deki tabanlarla (`fresh_alpha_min`, `closed_alpha` `tg5`'te de) kapandı; düzenekte işaret
  karşıtlığı 1,40 → 3,11.
- **Grafik derinliği** hâlâ `r4` görünümüyle kuruluyor (`depth::chart_view`, LOOK_RECIPE): renk
  ve yazı bu görünüşten, boylar `r4`'ten.
- Şeridin içindeki küçük açıklama ("C 76,16") derleyicinin çizgi açıklamasıdır; kapatılamıyor. Kanıt geometrisini (eşik çizgisi) şeride koymak denendi ve bırakıldı: seviye çizgisi bu açıklamanın altından geçince metin karşıtlığı 1,16'ya düştü (düzenek ölçümü); kanıt açılmış kapsülde ve grafikte.
- Yetkisiz okurun yüzü sahne değil, çekirdeğin metin yüzü (A32); bu görünüşün kabuk renkleri ve
  yazısıyla basılır, şerit yoktur.
- Akış düzeni eklenmedi: zaman duygusu yüzden (şimdi sütunu) geliyor; sıralama M19 seçimi.
