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
  doygunlukta değil (F9 tek vurgu). İkincil metin (`muted`) koyu tutuldu: sunulan sonuç stili
  (`tg5`) tutmayan iddianın işaretini `muted` rengiyle 0,55 saydamlıkta çizer; bu renk ≥ 3:1
  kalsın diye (kayıt sınaması). Bedeli: soluk metin ile ana metin arasındaki ayrım boy/ağırlıkla
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
| tutmayan işaret (`muted` × 0,55, `tg5`) | ≥ 3 (kayıt sınaması) | ≥ 3 |

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

Düzeneğin kendisiyle (`glance.py shoot` + `compute`, yalnız bu tasarım, 44 çekim, tr/en ×
açık/koyu; okuyucu/eleştirmen koşulmadı):

| ölçü | sonuç |
|---|---|
| metin karşıtlığı (en kötü / eşik) | 6,14 / 4,5 ✓ |
| en küçük yazı | 12 px ✓ (kabuğun güven işareti; sahne rolleri ≥ 13) |
| dokunma < 44 px | 0 ✓ |
| işaret karşıtlığı (en kötü) | **1,40 ✗** — tutmayan eski bir iddia: `tg5` işareti `muted` × 0,55 × tazelik (0,3–1) saydamlıkta çizer; görünüş kaydı tazeliği denetleyemez (aşağıda "Bilinen sınırlar") |
| renk çiftleri | **2 çift ✗** (`closed/failed`–`open/valid:down` 9,16; `open/no_longer:down`–`open/valid:down` 8,17): ikisi de düzeneğin **farklı temalardan** aldığı örnekleri karşılaştırıyor (sınıf başına ilk çekim; `valid:down` açık temadan, ötekiler koyudan). Aynı tema içinde kayıt renkleri yukarıdaki tabloda ≥ 10. Düzeneğe öneri: sınıfları tema başına ayır |
| yüz karakteri ort./en çok | 170,2 / 182 |

## K52 ve paket boyu

`glance.py k52` (kalibre `PHONE` ve JIT açık tahmini, `canvas`):

| yapılandırma | tik p95 | yüz p95 | sahne p95 | ilk kapsül | kaydırma p99 kare |
|---|---|---|---|---|---|
| `phone` | 1,6 ms | 2 ms | 5,55 ms | 252 ms | 30 ms |
| `phone-jit-est` | 1 ms | 1 ms | 1 ms | 200 ms | 19 ms |

Bütçe (≤ 8 ms) içinde (`pass: true`). "Önce" için `r4` bu yön tarafından açılmadı (yönerge: bugünkü
tasarım görülmez); karşılaştırmayı orkestratör aynı düzenekle yapar.

Paket: iki yazı tipi dosyası WASM'a aynen girer: 24 060 + 24 032 = **48 092 bayt ham**
(gzip -9: 13 072 + 13 559 = 26 631; brotli 11: 12 103 + 12 625 = 24 728). Bu dalın paketi
`wasm_raw` 5 373 215, `wasm_gzip` 1 601 664, `wasm_br` 1 052 667 (`pkg/sizes.json`). Plex Mono
yerinde kalır (rakam rolü).

## Bilinen sınırlar (yönün dışında, ölçümü etkileyebilir)

- **Açık işaretin tazelik solması** (`siluet`: saydamlık 0,3 + 0,7·tazelik) ve sunulan sonuç
  stilinin (`tg5`) gerçekleşmiş/tutmamış işaretleri tazeliğe göre soldurması görünüş kaydından
  denetlenemiyor (`closed_alpha` yalnız alfabenin kapanmış çizimine uygulanıyor, `tg5` onu okumuyor).
  Eski bir işaret her görünüşte 3:1'in altına inebilir. Öneri: `tg5` ve açık işaret için de bir
  "en düşük saydamlık" görünüş alanı.
- **Grafik derinliği** hâlâ `r4` görünümüyle kuruluyor (`depth::chart_view`, LOOK_RECIPE): renk
  ve yazı bu görünüşten, boylar `r4`'ten.
- Şeridin içindeki küçük açıklama ("C 76,16") derleyicinin çizgi açıklamasıdır; kapatılamıyor. Kanıt geometrisini (eşik çizgisi) şeride koymak denendi ve bırakıldı: seviye çizgisi bu açıklamanın altından geçince metin karşıtlığı 1,16'ya düştü (düzenek ölçümü); kanıt açılmış kapsülde ve grafikte.
- Yetkisiz okurun yüzü sahne değil, çekirdeğin metin yüzü (A32); bu görünüşün kabuk renkleri ve
  yazısıyla basılır, şerit yoktur.
- Akış düzeni eklenmedi: zaman duygusu yüzden (şimdi sütunu) geliyor; sıralama M19 seçimi.
