# R4/M8 — Kapsülün kapalı yüzü: sonuç

> Persona ajanı insan değildir; model küçük pikseli ve beş saniyelik bakışı insan gibi
> deneyimlemez. Bu ölçüm **ön elemedir**; son söz insan testi v2'dedir (SPEC §0.5).

## Düzen ve sıra (VERIFY 14)

| Adım | Commit | Dal |
|---|---|---|
| Şartname (SPEC §2, seçim kuralı) | `bad3d67` (12:04) | `r4/int` |
| Düzenek (`m8`, S8, `run_readers.sh`, `score`) + başvuru `r3` ölçüsü | `6b4b581` (14:51) | `r4/h-m8910` → `r4/int` |
| SPEC ekleri E1–E5 | `5b327e5` (15:00) | `r4/int` |
| Başlık şablonu arayüzü (H-FP, `HeadlineParts`) | `7d7cae1` (16:01) | `r4/h-fp` → `r4/int` |
| Alternatif B: `tavrik` ("işaret önde", 156 px) | `93f43b8`, not `a14eaa1` (16:30) | `r4/m8-b` |
| Alternatif A: `vadrel` ("cümle önde", 188 px) | `9d281df` (16:35) | `r4/m8-a` |
| SPEC ekleri E6, E7 (ilk teslimden sonra, hiçbir okuma/puan yokken) | `fa07648` (16:39) | `r4/int` |
| Alternatif C: `fotolay` ("olayın resmi", 228 px) | `637c99e` (16:46) | `r4/m8-c` |
| Değerlendirme birleştirmesi (`fa07648` + A + B + C) | `f4e38f5`, `0bd5925`, `ac2b263` | `r4/eval-m8` |
| Düzenek düzeltmeleri (aşağıda) | `cc0d60b` | `r4/eval-m8` |
| Okuyucu/eleştirmen koşuları, puanlama, bu not | bu commit | `r4/eval-m8` |

Düzenek commit'i `6b4b581` üç alternatifin de atasıdır (`git merge-base --is-ancestor`
doğrulandı). Alternatifler yalnız `crates/qp-grammar/src/faces.rs` (kayıt satırı + `mod`),
kendi `faces/<ad>.rs` dosyası ve kendi `DESIGN.md`'sine dokundu.

## Düzeneğe okumadan önce yapılan düzeltmeler (`cc0d60b`)

Hepsi okuyucular koşulmadan, sonuç görülmeden yapıldı; seçim kuralının sırası değişmedi.

1. **E7** (`m8.rs`): başlık uzunluğu sert eşiği ve başlık en çok/ortalaması #9 (yetkisiz
   okuyucu, kabuğun düz metni) dışındaki 10 durumla hesaplanır; #9 `computed.json`
   `headline_chars_unentitled`'da ayrıca yazılır: **TR 104, EN 95, dört tasarımda aynı**.
2. **E6** (`score.rs`, `m8_waived`): kural 1 eşik başına. Bir eşik, hiçbir alternatif
   geçemiyorsa **ve** `r3` dahil bütün tasarımların en kötü değeri aynıysa uygulanmaz; değerler
   farklıysa uygulanmaya devam eder. Kayıt: `scores.json` `rule1_waived`, seçim adımları,
   tasarım başına `thresholds_failed_after_e6`. Birim testi `e6_waives_a_threshold_no_design_can_pass`.
   (`computed.json`'ın `pass` alanı E6'sız ham sonuçtur; dördünde `false`.)
3. **Ek bilgi sayımı** (`m8.rs`): derlenmiş sahnenin başlık satırı (görünümün `title`'ı, yoksa
   sembol + periyot; `qp_grammar::compile`'ın `over`'a taban çizgisi 23'te koyduğu satır) metin
   taşıyorsa 1 birim. Önceden sayılmıyordu (`vadrel`'in karne satırı orada; 0 → 1). Değer
   etiketleri ve eksen yazıları zaten sayılan grafik panelinin parçası sayıldı (ayrı satır
   sayılsaydı `fotolay` 4 olurdu; görev "başlık dışındaki görünür her metin satırı (title dahil)"
   diyor — grafiğin kendi yazılarını grafik biriminde bıraktım, bu bir yorumdur).
4. **İstemde etiket köşesi** (`prompts.rs`, görüntülere bakınca bulundu): M8 persona ve
   eleştirmen istemi etiketlerin "sol üst"te olduğunu söylüyordu; sayfada (README de) **sağ
   üst**teler. İki istem düzeltildi (M9 istemine dokunulmadı).
5. **Port:** M8 ekranları canlı gösterimden değil, `file://` durağan sayfalardan alınır
   (`shoot.mjs`); sabit port yok, `M8910_PORT` gerekmedi. Elle wasm-pack de gerekmedi (sayfa
   yalnız `shells/web/style.css` ve çekirdeğin boyadığı PNG'leri kullanır).

**`r3`'ün yeniden üretilen `computed` değerleri:** ek bilgi en çok 2 → **3** (başlık satırı
"SYN03 1h"; `extra_units` eşiği artık geçmiyor); başlık ortalaması (#9'suz) TR 174 → 181, EN
177,36 → 185,6; sözcük ortalaması TR 26,18 → 27,4, EN 31,64 → 33,3; başlık en çoğu (208) ve öteki
eşikler değişmedi. Yeni alan `headline_chars_unentitled` {tr 104, en 95}.

Görüntülere bakış (her tasarımın iki sayfası, açık/koyu/gri): düzenek kaynaklı kesilme ya da
çakışma yok; etiket rozetleri metinle çakışmıyor. Görülen kusurlar tasarımın/çekirdeğin kendisi:
`r3` kartında uzun cümle "Karne" satırına ve "C 123.43" etiketi fiyat çizgisine biniyor;
`fotolay`'da soldaki ay etiketi kesik ("023-03", DESIGN zayıflık 1). Ekranlar belirleyici
(`vadrel` yeniden üretildi, üç PNG bayt eşit).

## Körleme

- Sayfalar tohumlu karma kodla (`raw/screens/<kod>{,.dark,.gray}.png`): `r3` `059ce936`/`8d59d758`,
  `vadrel` `262bbace`/`bcd99ae7`, `tavrik` `be730d1e`/`dc16761a`, `fotolay` `37da4c4c`/`59acd941`.
  Eşleme `raw/key.json` ve `raw/keys/`; okuyucu dizinine kopyalanmaz (`run_readers.sh` yalnız
  `images.json`'daki PNG'leri yeni geçici dizine kopyalar).
- Okuyucu: taze `claude -p --model sonnet --tools Read --allowedTools Read --restricted
  --strict-mcp-config --permission-mode dontAsk --no-session-persistence --output-format json`;
  bütün cevaplarda (persona, yeniden koşu ve eleştirmen) model `claude-sonnet-5-5` (`modelUsage`). Her persona okuyucusu tek bir
  tasarımın tek sayfasını görür.
- Her istem gönderilmeden `m8910 check-prompt` yasak sözcük denetiminden geçti; gönderilen
  istemlerde tasarım adı yok (ayrıca `grep` ile denetlendi).
- Kanıt — persona istemi: `rounds/R4/experiments/M8/raw/readers/F_p1_262bbace_r1/prompt.txt`;
  eleştirmen istemi: `rounds/R4/experiments/M8/raw/critics/c1/prompt.txt` (tasarımlar yalnız
  "Tasarım K/L/…" harfleriyle). Eleştirmen anahtarı (harf → tasarım) ayrı dosyada,
  `raw/critics/c<k>/key.json`; eleştirmenin geçici dizinine kopyalanmaz. Harfler ve sıra her
  eleştirmende farklı (c1 T/P/L/R, c2 K/N/S/Z, c3 Z/N/T/M).

## S8 ve SPEC §8 ekleri

- S8 (`harness/s8.json`, değişmedi): 12 durumun 11'i; sayfa 1 = #1, #5, #7, #9, #12, #10;
  sayfa 2 = #3, #11, #4, #6, #2 (sabit sıra, tohum 1; her tasarımda aynı yer).
- **E1:** S4 "az örnek" kovası `n < 20` (#4, #6, #7, #10, #12 anahtarı "az örnek").
- **E2:** #8 (geri çekildi) yok; sayfa 2 beş yüzle.
- **E3:** #1, #11, #12 saatlik dünyadan.
- **E4:** sayfa tam boy (390 CSS px, DPR 2); yüz yüksekliği hesap ölçüsü.
- **E5:** her alternatif kendi başlık şablonu; `r3` bugünkü cümle.
- **E6:** uygulandı — `glyph_contrast` (kapanmış işaretin soluk gri çizimi, en kötü 2,06 açık /
  2,36 koyu, dört tasarımda aynı) kural 1'de uygulanmadı; öteki eşikler eledi.
- **E7:** uygulandı — #9 düz metin (TR 104 / EN 95) başlık eşiğinin dışında.

## (b) Hesap ölçüleri (`raw/<ad>/computed.json`, sahne tarifinden)

Başlık #9'suz (E7). Metin öğesi, metin alanı, işaret alanı ortalama (sahneli 20 yüz × 2 tema);
karşıtlık ve en küçük yazı en kötü.

| ölçü | `r3` (başvuru) | `vadrel` | `tavrik` | `fotolay` |
|---|---:|---:|---:|---:|
| yüz yüksekliği (CSS px) | 240 | 188 | 156 | 228 |
| başlık TR en çok / ort. (karakter) | 205 / 181 | 75 / 66,1 | 78 / 70,9 | 78 / 72,2 |
| başlık EN en çok / ort. | 208 / 185,6 | 77 / 66 | 76 / 66,6 | 80 / 75,8 |
| başlık sözcük TR / EN ort. | 27,4 / 33,3 | 10,3 / 11,6 | 13,9 / 14,2 | 11,1 / 13,3 |
| metin öğesi (ort.) | 12,6 | 4,75 | 4,45 | 7,6 |
| metin alanı / yüz alanı | 0,212 | 0,269 | 0,160 | 0,137 |
| işaret alanı px² / pay | 1600 / 0,017 | 1024 / 0,014 | 5776 / 0,095 | 784 / 0,009 |
| metin karşıtlığı en düşük açık / koyu | **2,79** / 5,27 | 5,78 / 5,27 | 15,8 / 11,74 | 5,78 / 5,27 |
| işaret karşıtlığı en düşük açık / koyu | 2,06 / 2,36 | 2,06 / 2,36 | 2,06 / 2,36 | 2,06 / 2,36 |
| en küçük yazı (CSS px) | 11 | 16 | 15 | 11 |
| ek bilgi (en çok) | **3** | 1 | 1 | 2 |
| sert eşikler (E6 sonrası) | geçmedi: metin karşıtlığı, başlık, ek bilgi | geçti | geçti | geçti |

İşaret karşıtlığı (≥ 3) dördünde 2,06: E6 ile uygulanmadı. #9 başlığı (düz metin) dördünde
TR 104 / EN 95. Gri tonlamada yön/geçerlilik ayrımı (§6) ΔE ile ölçülmedi (bu düzenekte yok);
eleştirmen ölçütü 6 aşağıda.

## (a) Persona okuma testi (`raw/readers/`, `raw/scores.{json,md}`)

Tasarım başına 2 persona × 2 sayfa × 3 okuyucu = 12 koşu, 11 yüz × 3 = 33 yüz cevabı persona
başına. Toplam 48 koşu + 1 yeniden koşu (aşağıda).

Soru başına doğruluk ve yüz başına tam doğru (S1–S5 hepsi):

| tasarım | persona | S1 | S1b | S2 | S3 | S4 | S5 | tam doğru |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| `r3` | F | 0,91 | 0,91 | 0,27 | 0,55 | 0,55 | 0,79 | 0,061 |
| | H | 0,85 | 0,94 | 0,48 | 0,70 | 0,76 | 0,91 | 0,303 |
| `vadrel` | F | 1,00 | 0,97 | 0,82 | 0,91 | 0,85 | 0,91 | **0,758** |
| | H | 0,97 | 1,00 | 0,82 | 0,91 | 0,91 | 0,91 | **0,788** |
| `tavrik` | F | 0,88 | 0,97 | 0,82 | 0,91 | 0,82 | 0,97 | 0,606 |
| | H | 0,85 | 1,00 | 0,82 | 0,91 | 0,82 | 1,00 | 0,576 |
| `fotolay` | F | 0,97 | 1,00 | 0,82 | 0,91 | 0,82 | 0,91 | 0,697 |
| | H | 0,97 | 1,00 | 0,82 | 0,91 | 0,91 | 0,91 | 0,788 |

| tasarım | ağırlıklı tam doğru (F 2, H 1) |
|---|---:|
| `vadrel` | **0,7677** |
| `fotolay` | 0,7273 |
| `tavrik` | 0,5960 |
| `r3` (başvuru) | 0,1414 |

Şans düzeyi: S1 0,25, S1b 0,143, S2 0,167, S3 0,167, S4 0,20, S5 0,20; tam doğru 0,00004.

**Tavan (düzenekten, tasarımdan bağımsız):** iki durum her tasarımda 0/6 tam doğru:
- #1: anahtar S2 "bugün" (3 saatten sonrası "bugün"), yüzler "4 saat önce" yazıyor; 24 okuyucudan
  23'ü "birkaç saat önce", 1'i "bilmiyorum" dedi (H-FP Açık soru 2).
- #9: yetkisiz okuyucunun düz metni zaman, geçerlilik ve karne vermiyor (S2/S3/S4 "bilmiyorum",
  "hiçbir bilgi göremedim").
Bu yüzden ulaşılabilir en yüksek tam doğru 9/11 = **0,818**; `vadrel` H ve `fotolay` H buna çok
yakın (0,788), üç alternatif arasındaki fark birkaç duruma sıkışıyor.

Durum başına tam doğru (6 okuyucu içinde; #1 ve #9 hepsinde 0):

| # | `r3` | `vadrel` | `tavrik` | `fotolay` |
|---|---:|---:|---:|---:|
| 2 | 4 | 6 | 6 | 6 |
| 3 | 0 | 6 | 1 | 4 |
| 4 | 0 | 6 | 6 | 6 |
| 5 | 4 | 6 | 6 | 6 |
| 6 | 0 | 6 | 6 | 6 |
| 7 | 1 | 5 | 2 | 6 |
| 10 | 0 | 5 | 0 | 3 |
| 11 | 3 | 6 | 6 | 6 |
| 12 | 0 | 5 | 6 | 6 |

**Teknik düşme:** `F_p1_37da4c4c_r1` (`fotolay`, F, sayfa 1) cevabı geçersiz JSON'du (bir değerin
içine `.replace(...)` kodu yazmış; ayrıştırıcı ilk geçerli nesne olarak B kartının alt nesnesini
alıyordu, bütün etiketler boş sayılırdı). Bir kez aynı istemle yeniden koşuldu; ilk cevap
`answer.unparseable.json` olarak duruyor. Başka hata yok (`is_error: false`, boş `stderr`).

## (c) Eleştirmenler (`raw/critics/c1..c3/`)

Ölçüt başına ortalama puan (1–5, 3 eleştirmen) ve ortalama sıra (1 en iyi):

| ölçüt | `r3` | `vadrel` | `tavrik` | `fotolay` |
|---|---:|---:|---:|---:|
| 1 Tek bakış | 1,00 | **4,67** | 4,33 | 3,33 |
| 2 Hiyerarşi | 2,00 | **4,00** | 3,67 | 3,33 |
| 3 Geçerlilik | 3,00 | **4,67** | 4,33 | 3,67 |
| 4 Dürüstlük | 3,00 | 3,67 | **4,33** | 4,00 |
| 5 Gündelik dil | 1,00 | **4,33** | 3,33 | 3,67 |
| 6 Erişilebilirlik | 2,67 | 4,00 | **4,33** | 3,33 |
| 7 Tavsiye değil | 3,00 | **5,00** | **5,00** | 4,67 |
| 8 Estetik/özgünlük | 1,33 | 3,67 | **4,00** | 2,67 |
| **ortalama sıra** | 4,00 | **1,67** | **1,67** | 2,67 |

Sıralar: c1 `vadrel` > `tavrik` > `fotolay` > `r3`; c2 `vadrel` > `tavrik` > `fotolay` > `r3`;
c3 `tavrik` > `fotolay` > `vadrel` > `r3`. c2 gri kopyaları açmadığını yazdı (ölçüt 6'yı renkli
ekrandan verdi); bu yüzden §6'daki "gri kopyada 3/3 ayırt" koşulu bu turda kanıtlanmış değil.

## Seçim kuralının uygulanışı (SPEC §2 + E6, E7; `score --exp m8`)

1. **Kural 1 (sert eşikler, eşik başına — E6):** `glyph_contrast` hiçbir alternatifte geçmiyor
   ve dört tasarımda en kötü değer aynı (2,06) → bu eşik uygulanmadı. Öteki eşikler (metin
   karşıtlığı, en küçük yazı, başlık ≤ 80 [E7: #9 hariç], ek bilgi ≤ 2) üç alternatifte de geçti
   → **elenen alternatif yok**. (`r3` başvurudur, sıralanır, seçilmez; zaten metin karşıtlığı,
   başlık ve ek bilgide geçmiyor.)
2. **Kural 2 (birincil, tam doğru F 2 / H 1):** `vadrel` 0,7677, `fotolay` 0,7273, `tavrik`
   0,5960. İlk ikinin farkı 0,0404 < 0,05 → eşitlik bozucuya.
3. **Kural 3 (eleştirmen ortalama sırası, az olan):** `vadrel` 1,67, `fotolay` 2,67 → `vadrel`
   birinci. (Başlık eşitlik bozucusuna gerek kalmadı.) `fotolay` ile `tavrik` arasındaki
   birincil fark 0,131 ≥ 0,05 → `fotolay` ikinci.
4. **Kural 4:** ilk iki gösterimde ayardan seçilebilir kalır ve insan testi v2'ye gider.

## Seçim

- **Seçilen: `vadrel`** ("cümle önde": 20 px tek başlık cümlesi, yanında küçük işaret, üstte
  tek satır karne; 188 px).
- **İlk iki (`top2`): `vadrel`, `fotolay`.** `tavrik` (eleştirmenlerde `vadrel` ile eşit
  birinci, işaret karşıtlığı/estetikte en iyisi) persona birincil ölçüsünde 0,13 geride kaldı.

## Gözlemler

- **Alternatiflerin bildirdiği zayıflıklar cevaplarda:**
  - `vadrel` 5 ("ek bilgi satırı üstte", karne cümleden önce okunabilir): persona
    cevaplarında zarar görülmedi (S4 F 0,85 / H 0,91, en iyisi); eleştirmen c3 aynı şeyi yazdı
    ("güvenilirlik satırı cümlenin üstünde ve başlığı karıştırıyor"). 4 ("son hareket ≠ doğuş",
    #1 "4 saat önce güçlendi") #1'in S2 kaybında payı var ama bütün tasarımlar aynı kaybı yaşadı.
  - `tavrik`: başlık yön sözcüğü taşımayan ad öbeğiyle başlıyor ("Aşırı alım · …" #3, "Para
    girişi · …" #7) ve yanında fiyat resmi yok; okuyucular yönü "belirsiz" dedi (#3 5/6, #7 4/6)
    — DESIGN'da bu yazmıyor; birincil farkın çoğu buradan. `fotolay` aynı ad öbeklerini kullanıyor
    ama fiyat izi yönü veriyor (#3 2/6, #7 0/6 "belirsiz"); `vadrel` fiille yazıyor ("aşırı
    yükseldi", "büyük alım geldi"; 0/6). "karnesi yok" (#10) 6/6 "hiçbir bilgi göremedim" (anahtar "az örnek");
    eleştirmen c2 "kırılım, sıkışma, karnesi yok" terimlerini jargon saydı. DESIGN 4 (satır kırma
    "15 / Mayıs'ta") cevaplarda iz bırakmadı.
  - `fotolay` 1 (eksen/ay etiketleri, kesik "023-03") ve 2 (nokta izi): persona doğruluğuna
    yansımadı; eleştirmenler grafiği "kalabalık", "finans kalıbına yakın" buldu (estetik 2,67,
    en düşük alternatif). #10 "Henüz kapanmış benzeri yok" 3/6 "hiçbir bilgi göremedim".
  - Üçünde ortak (DESIGN'larda yazılı): kapanmış işaretin karşıtlığı 2,06 — okuyucular
    geçerliliği yine de cümleden okudu (S3 üçünde 0,91; kalan hata #9).
- **Başvuru `r3`:** uzun cümle ve "bar" dili; S2 F 0,27, #6 "geçersiz oldu; artık geçerli değil"
  cümlesi 6/6 "artık geçerli değil" okundu (anahtar "kapandı: tutmadı"). Üç alternatifin hepsi
  bu açığı kapattı.
- **Persona modelinin tavan/taban etkileri:** S1b ve S5 neredeyse tavanda (0,91–1,00, F ile H
  arasında fark yok) — model "hiçbir şey, yalnız bilgi veriyor"u kolay seçiyor; "beş saniyelik
  bakış" F personasında görünmüyor (F ≈ H). Ayırt edici sorular S1 (yön), S4 (karne kovası) ve
  #3/#7/#10 gibi birkaç durum. #1 ve #9 düzenekten gelen ortak tavan (yukarıda). Sonuç: üç
  alternatif arasındaki sıra birkaç yüze dayanıyor; insan testi v2'de `vadrel` ile `fotolay`
  (ve gerekirse `tavrik`) gerçekten ayrılmalı.
- `scores.json` ham dosyalardan yeniden üretildi, bayt eşit.

## Maliyet

Persona okuyucuları 49 koşu (48 + 1 yeniden) ≈ 1,37 $; eleştirmenler 3 koşu ≈ 0,31 $; toplam
≈ **1,68 $** (`total_cost_usd`, `claude-sonnet-5-5`).

## Öneriler (kapsam dışı)

- S8 #1 anahtarı: "bugün" ile "N saat önce" çakışıyor (her tasarım 0); anahtar "birkaç saat
  önce"yi de doğru saymalı ya da ürünün "bugün" eşiği anahtarla aynı olmalı (H-FP Açık soru 2).
- #9 yetkisiz okuyucunun yüzü düz metin; M8 bu durumda tasarımları ayırmıyor (README Açık soru 2).
- Eleştirmen isteminde gri kopyaların açılması zorunlu kılınmalı (§6 kanıtı için).
- `computed.json` `pass` alanı E6'yı bilmiyor; `score` uyguluyor. İstenirse E6 sonrası durum
  `computed`'a da yazılabilir.
