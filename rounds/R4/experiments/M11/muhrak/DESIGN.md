# M11 sonuç işareti — `muhrak` ("mühür / kırık mühür")

Kod: `crates/qp-grammar/src/glyph/outcome/muhrak.rs` (`Muhrak`), `glyph/outcome.rs`'te
`mod` + `STYLES` satırı. Başka ürün dosyası değişmedi; varsayılan `r3`, altınlar bayt eşit.

## Fikir

Sonuç bir köşe rozeti değil, **glifin bütününün dönüşümü**. Üç düzey, renksiz okunur:

| durum | çizim | benzetme |
|---|---|---|
| hâlâ açık | alfabenin kendi çizimi (renkli, çerçevesiz) | — |
| gerçekleşti | kutunun tamamı tek, dolu, koyu bir levha (siluet: yuvarlak köşeli kare; rozet: disk); açık glif biraz küçülüp levhaya **arka plan renginde oyulur** | basılmış mühür, madalya, dolu onay kutusu, kapanmış sağlık halkası |
| tutmadı | açık glif gri, **boş** bir çerçevenin içinde (siluet: ince yuvarlak kare; rozet: kendi halkası); yukarıdan aşağı arka plan renginde **kırık bir çatlak** bölüyor; çatlağın sağındaki yarı soluklaşmış (kopmuş) | kırık mühür, çatlamış cam, yarım kalmış bilet |

Okuma sırası: çerçeve yok → açık; boş, çatlak çerçeve → tutmadı; dolu levha → gerçekleşti.
Gerçekleşti ↔ tutmadı çifti kutunun büyük bölümünde farklıdır (koyu levha ↔ arka plan),
bu yüzden bulanıklık, 16 px ve renk körlüğü simülasyonlarında ayrım küçülmez.

## Kurallar

- Açık aşamalar (`Candidate`, `Final`) ve `Retracted`: `alphabet.draw` aynen (test).
- Gerçekleşti: levha `theme.text`, opaklık = alfabenin tazelik formülü (siluet `0,3+0,7f`,
  rozet `0,22+0,78f`). Açık görünüm (`open_look`) kutu merkezine göre küçültülür (siluet
  0,8; rozet 0,74, dış halkası levha içinde koyu bir kenar bırakacak kadar) ve arka plan
  rengine boyanır; alfa korunur (güven tabakası levhada açık bir bant, teyit blokları /
  halka parçaları beyaz kalır: kanallar kaybolmaz). Arka plan renkli halo ilkelleri olduğu
  gibi kalır.
- Tutmadı: açık görünüm `theme.muted`'a boyanır (arka plan renkleri hariç); siluet 0,8
  küçülür ve `theme.text` renginde ince çerçeve (`max(1, 0,06·s)`) alır; rozet tam boy kalır
  (halkası çerçevedir). Çatlak: sabit kırık çizgi (8 nokta), genişlik `2·max(0,9; 0,065·s)`,
  arka plan renginde; uçları kutunun içinde yarım genişlik içeride, ilk/son bacak dikey
  (çerçeveyi temiz keser). Çatlağın sağı arka plan rengi %72 opak bir dolguyla soluklaşır.
- Yalnız IEEE temel işlemler (çeyrek çember örnekleri sabit tablo), renkler yalnız temadan,
  kutu dışı ilkel 0 (düzenek ve birim test).
- `legend` (TR, tek cümle): siluet 25, rozet 25 sözcük; yasak sözcük denetimi temiz
  (`sheets --cond legend`, `check-prompt`).

## Kendi ölçüm (`m11 measure --style muhrak`, `M11/raw/muhrak/distinct.{json,md}`)

| alfabe | en kötü koşul ΔE | çift | koşul | başvuru `r3` | kat |
|---|---:|---|---|---:|---:|
| siluet | **40,49** | `flow.down` | 24 / koyu / gri | 2,98 | 13,6× |
| rozet | **22,93** | `continuation.neutral` | 16 / açık / gri | 11,33 | 2,0× |

İki alfabe üzerinden ana ölçü **22,93**. Kutu dışı ilkel 0. Bilgi (kurala girmez):
gerçekleşti ↔ açık en kötü siluet 38,47; tutmadı ↔ açık en kötü siluet ~11,2, rozet ~12,6
(r3: 2,23 / 2,02).

Ara denemeler: ilk sürüm (soluklaştırma %60, rozet küçültme 0,8) siluet 38,98 / rozet 18,01;
soluklaştırma %72 → 40,49 / 18,87; rozet gerçekleşti küçültmesi 0,74 → 40,49 / 22,93 (son).

Sayfalara (`sheets --seed 1`, iki alfabe) Read ile bakıldı: üç durum 32 px'te ilk bakışta
ayrılıyor. Okuyucu ajan koşulmadı (orkestratörün işi); körleme çıktıları (`screens`, `keys`,
`key.json`, `prompts`) commit edilmedi.

## Bilinen zayıflıklar

- **Kutup okuması:** koyu dolu levha açıklamasız okuyucuya "güçlü / seçili / etkin" gibi de
  görünebilir; "gerçekleşti" anlamı tamamlanmış/mühürlenmiş benzetmesine dayanıyor, onay
  işareti yok. Okuyucu ölçümü bunu gösterecek.
- **Tutmadı görsel olarak kalabalık:** gri glif + güven tabakası + çatlak + soluk yarı;
  zayıf (ince çizgili) gliflerde siluet 0,8 küçülmeyle iyice incelir, tür okunurluğu düşer.
  Çatlak 16 px'te ~2 px'lik beyaz bir zikzaktır; "kırık" yerine "bozuk görüntü" diye de
  okunabilir.
- **Rozet gerçekleşti** küçülen halka levhanın içinde beyaz bir iç halka olur ("hedef"
  görünümü); 32 px'te desen küçük. Rozet ΔE'si siluetinkinden çok düşük (rozet zaten açıkken
  kalın halkalı bir disk olduğu için).
- Gerçekleşti levhası `theme.text` ile çizilir: açık temada siyah, koyu temada açık gri bir
  blok; kalabalık bir akışta en baskın öğe olur (dikkat çekiciliği yüksek; kapalı kapsül
  açık olandan daha çok göze batabilir). Gösterim düzeyinde değerlendirilmeli.
- Süresi doldu, tutmadı ile aynı çizilir (şema gereği; README).

## Öneriler (kapsam dışı)

- Okuyucu sonucu "levha = gerçekleşti" eşlemesini zayıf bulursa, levhanın içine oyulmuş
  glifin yerine/yanına büyük bir oyma onay biçimi eklemek ayrı bir stil olarak denenebilir.
