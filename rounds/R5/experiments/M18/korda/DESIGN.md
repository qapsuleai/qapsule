# M18 — `korda` (ömür şeritleri)

Dal `r5/m18-korda`. Kod: `crates/qp-grammar/src/stories/{mod.rs,korda.rs}`. Seçim:
`?story=korda`, Ayarlar → Hikâye çizelgesi (`style` hareketinin `story` alanı,
`ViewState.story`). Varsayılan `r4`: çizelge yok (bugünkü tek kapsüllü grafik, altınlar ve
dökümler aynı).

## İlke

Fiyatın altında, zamana hizalı bir **hikâye paneli**. Her kapsülün ömrü bir yatay şerit:
doğum barından kapanış barına (açık hikâye son bara kadar). Doğumda yön işareti, sonda
sonuç işareti. Şeritler çakışmaz: hikâyeler doğum sırasıyla ilk boş şeride yerleşir.
Üç grup, fiyattan aşağı doğru:

1. **Seçili** hikâye tek başına ilk şeritte: odak rengi, kalın çizgi, şeridin üstünde
   adı ve durumu ("aşağı kırılım · tutmadı"), doğum barından fiyata çıkan ince bir kılavuz.
   Kanıt geometrisi (seviye, bant, noktalar) ve fiyat üstündeki anlam işareti **yalnız**
   onundur (sunucunun grafik görünümü, `depth::chart_view_in`).
2. **Canlı** (açık) hikâyeler: "geçerli" renginde çizgi, yön renginde işaretler.
3. **Kapanmış** hikâyeler: kapanmış renginde, ince (K54: soluk, sonucuyla). Solukluk
   saydamlıkla değil çizgi kalınlığıyla verilir; renk tam opak kalır, karşıtlık korunur.

Başlık satırı: "canlı 2 · kapanmış 31" ve sağda "gizle / göster" düğmesi; bütün satır
kapanmışları gizleme/gösterme hedefidir (K54).

### Biçim (renk tek başına anlam taşımaz)

| Anlam | Biçim |
|---|---|
| Yön | doğum işareti ▲ yukarı, ▼ aşağı, ◆ yönsüz (renk de yön rengi, kapanmışta gri) |
| Sonuç | ● gerçekleşti, × tutmadı, ○ süresi doldu, □ geri çekildi, ▸ hâlâ açık |
| "Artık geçerli değil" | çift ray (iki ince çizgi) |
| Yazı tura (K53) | noktalı şerit, etikette "yazı tura" |
| Seçili | ilk şerit, kalın, odak rengi, üstte etiket, fiyata kılavuz |

### Kalabalık (yoğunlaşma / özet)

Panel en çok çizim alanının %42'si. Sığma sırası:

- **Tam**: şerit aralığı 13 px, işaret yarıçapı 4,5 px, satır içi etiket (bitiş
  işaretinin sağında ya da ömrün üstünde, zemin yamasıyla).
- **Sıkı**: aralık 6–12 px, işaret 2,5 px, etiket yok (seçili şerit tam kalır).
- **Özet**: kapanmışlar (sığmıyorsa canlılar da) bar kümelerinde doğum histogramı;
  sütun en az 12 px, sonuca göre üst üste dolgu deseni (dolu: gerçekleşti, çerçeve:
  tutmadı, taralı: süresi doldu / geri çekildi, açık renk: canlı). Bir sütuna dokunmak
  grafiği o barlara yakınlaştırır (en az 20 bar). Seçili ve canlı şeritler özetin üstünde
  ayrı kalır.

Uzaklaştıkça (kıstırma, tekerlek) şeritler sıkılaşır, sonra kümelenir; yaklaştıkça açılır.

### Etkileşim (çekirdekte; kabuk yalnız `tap_at` gönderir)

Otoritenin `tap_at`'ı grafik derinliğinde nokta sorgusunun öğesini
`qp_grammar::stories::story_hit` ile okur (`crates/qp-session/src/interact.rs` `story_tap`):

- **Başka bir şerit** (`story:<kapsül>`): odak o kapsüle taşınır (grafik derinliğinde
  kalır), görünen barlar aynı kalır; uçan ortak öğe yok.
- **Seçilinin kendi şeridi / işareti / seviyesi**: bugünkü açılma kuralı → kanıt.
- **Küme** (`story-cluster:<a>-<b>`): görünen aralık o barlara yakınlaşır.
- **Başlık satırı** (`story-toggle`): `ViewState.closed` false ↔ yok (kapanmışlar gizli /
  görünür). `style` hareketinin `closed` alanı ajan için aynı ayarı yazar.

### Tek doğruluk kaynağı

Sahne `korda::scene` (görünüm + liste → derleme + hikâye paneli). Otorite (`capsule_scene`:
dokunuş, sürükleme, kıstırma, geçiş) okuyucunun dünyasındaki satırlarla
(`Session::story_rows_here`, `stories` sorgusunun aralıksız bütün sayfaları), cihaz
(`viewer::open_scene_stories`) sunucudan aldığı listeyle (§23.5) aynı fonksiyonu çağırır;
`ref` ve `canvas` aynı sahneyi boyar. Her şerit katmanı bir bulguya (`DataSource::Finding`)
bağlı, etiketi (`label`) `screen`'de okunur; nokta sorgusu onu adlandırır. Testler her
dokunuşu cihazın kendi `hit_scene` okumasıyla nişanlar ve otoritenin aynı öğeye göre
davrandığını doğrular.

Liste henüz gelmediyse (ya da aralıkta hikâye yoksa) panel çizilmez: düz grafik.

## Ölçümler

### Testler

- `crates/qp-grammar/tests/stories_korda.rs` (sentetik 300 bar, 5 / 50 / 200 hikâye): her
  görünür hikâye bir şeritte ya da bir küme sayısında; aynı satırdaki şeritler çakışmıyor;
  seçili ilk şerit; kapanmışlar gizlenebiliyor; her öğe nokta sorgusuyla adlandırılıyor ve
  anlamı (`StoryHit`) doğru; aralık sınırı; İngilizce; liste boşken düz derleme ile bayt
  eşit; belirlenimcilik (iki koşu ve ters satır sırası aynı); karşıtlık.
- `crates/qp-wasm/tests/stories_korda.rs` (gösterim verisi, kabuk çekirdeği + otorite):
  varsayılan `r4`'te panel yok; görünür her hikâyenin şeridi var; üç kez odak taşıma
  (cihazın `hit`'iyle nişan, otorite aynı kapsüle taşıyor, aralık değişmiyor, yeni seçili
  ilk şeritte); kendi şeridi kanıtı açıyor; gizle/göster; küme dokunuşu yakınlaştırıyor;
  İngilizce; bilinmeyen ad reddediliyor.

### Karşıtlık (r4 görünüşü, WCAG)

Kapanmış renk koyu 5,2:1, açık 5,8:1 (işaret ≥ 3:1 ve küçük metin ≥ 4,5:1 ikisini de
geçer; tam opak). Yön, odak, geçerli/geçersiz renkleri ≥ 3:1; şerit etiketleri ve başlık
≥ 4,5:1 (test: `closed_marks_and_lane_text_keep_their_contrast`; kapanmış renk her kayıtlı
görünüşte ≥ 3:1).

### K52 bütçesi

`rounds/R5/experiments/M18/korda/measure.mjs` (`canvas` yolu; `qp demo --synthetic`,
adım 700, ilk açık kapsül grafikte, tekerlekle bütün geçmiş görünür; n > 0 için
`qp.benchStories(n)` listeyi n sentetik hikâyeyle değiştirir; 10 sn oynatma; 3 koşunun
ortancası; WebKit 26.5). Zamanlayıcı 1 ms adımlı; `scene` (sahne boyama) yalnız sahne
değişince ölçülür ve bu senaryoda koşu başına 2–3 örnek verir, bu yüzden her tikte koşan
`scene_build` (sahne kurma) ve `data_pass` (veri karesinden sonraki boyama geçişi: kurma +
değişim denetimi + boyama) de verildi.

p50 / p95, ms (ham: `k52-*.json`):

| Ayar | Tasarım | Hikâye | tik | sahne boyama (n) | sahne kurma | veri geçişi (tik sonrası boyama) |
|---|---|---|---|---|---|---|
| `PHONE` (JIT kapalı, DPR 3) | `r4` (başvuru) | 38 (gerçek, çizilmez) | 0 / 1 | 5 / 7 (2) | 2 / 3 | 5 / 7 |
| `PHONE` | `korda` | 38 (gerçek) | 0 / 1 | 7 / 8 (2) | 3 / 4 | 6 / 9 |
| `PHONE` | `korda` | 5 | 0 / 1 | 6 / 7 (3) | 3 / 3 | 5 / 8 |
| `PHONE` | `korda` | 50 | 0 / 1 | 8 / 9 (3) | 3 / 4 | 6 / 9 |
| `PHONE` | `korda` | 200 | 0 / 1 | 7 / 8 (2) | 3 / 4 | 7 / 9 |
| `PHONE_JIT_EST` (JIT açık) | `r4` | 38 | 0 / 1 | 0 / 1 | 0 / 1 | 1 / 1 |
| `PHONE_JIT_EST` | `korda` | 5 / 50 / 200 | 0 / 1 | ≤ 1 / 1 | 0 / 1 | ≤ 1 / 1 |

Okuma: K52'nin tanımladığı koşulda (HTTPS, JIT açık) bütün n için ≤ 1 ms, bütçenin çok
altında. Kalibre en kötü durum `PHONE`'da (JIT kapalı) başvuru `r4` zaten 7 ms; `korda`
sahne kurmaya ~1 ms ekliyor, veri geçişi p95 8–9 ms (bütçe 8 ms; 1 ms adımlı zamanlayıcı,
sahne boyamada koşu başına 2–3 örnek). 200 hikâyede maliyet 50'den fazla değil: özet kipi
şerit sayısını sınırlıyor. Yerel (Apple silicon, release) ölçüm: 300 barlık sahnede derleme
~50 µs, `korda` 5 / 50 / 200 hikâyeyle 57 / 73 / 76 µs (`timing_probe`, `--ignored`).

## Kayda alınması gereken değerler (`look.rs`'e)

`korda.rs`'teki `const`'lar, `r4`'ün boşluk ve yazı ölçeğinden türetildi:
`HEADER_PX` 16, `SELECTED_PITCH` 22, `FULL_PITCH` 13, `COMPACT_MAX` 12, `MIN_PITCH` 6,
`SUMMARY_LIVE_PITCH` 8, `GROUP_GAP` 3, `MIN_HIST_PX` 24, `MAX_SHARE` 0,42, `BUCKET_PX` 12,
`LANE_GAP_PX` 6, `MARK_R` [4,5; 2,5], `LANE_W` [3; 2; 1,5], `GUIDE_OPACITY` 0,55. Renkler
var olan rollerden (`focus`, `valid`, `invalid`, `closed`, `up`, `down`, `neutral`, `text`,
`muted`, `bg`); etiketler `label` yazı rolü.

## Bilinen sınırlar

- Telefon sahne çerçevesi 390×300: panel en çok ~98 px; tam kipte seçili + 3–4 şerit,
  sıkı kipte ~8 şerit sığar. Bu yüzden ~30'dan fazla hikâyeli tam geçmiş çoğu kez özettir.
- Dokunma hedefleri: başlık satırı 16 px yüksek (tam genişlik), şerit 6–13 px. 44 pt
  altında; yakınlaştırınca şeritler büyür. M16/M20'de ölçülmeli.
- Kümede `hit` en yeni hikâyeyi (seçili oradaysa onu) ve sayıları adlandırır; tek tek
  hikâyeler yakınlaşınca okunur.
- Odak taşınınca cihazın açık dilimi (§22.3) yeni kapsül için yeniden kurulur; arada bir
  karede sahne yoktur.
