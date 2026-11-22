# İzleme grubu maketleri (RD2 M2/M3)

Yön `tavek` üzerine kurulu yüzey maketleri. Şartname: `docs/design/surfaces/watch.md` (Y07, Y08,
Y20, Y21, Y31, Y32), `ledger.md` (Y33, Y34, Y35), `phone.md` (Y26), `system.md` (Y36).

| Sayfa | Yüzey | Kademe | İçerik |
|---|---|---|---|
| `Y07.html` | İzleme görevleri listesi | 1 | bağlam içinde ana durum + 8 / 8 durum, iki profil |
| `Y26.html` | Mobil kontrol kulesi | 1 | 9 / 9 durum, telefon 390 × 844; masaüstü yalnız karşılık notu |
| `Y31.html` | Sessiz zil, izleme sağlığı | 1 | 10 / 10 durum, iki profil |
| `Y32.html` | İki aşamalı kırılım | 1 | 9 / 9 durum, iki profil |
| `Y33.html` | Tutma yüzdesi, sicil | 1 | açık sicil bölümü + 11 / 11 durum, iki profil |
| `Y08.html` | İzleme görevi ayrıntısı | 2 | çalışma alanı kesiti (liste · grafik · ayrıntı) + durum listesi |
| `Y20.html` | İzleme listesi paneli | 2 | canlı panel + düzenleme / boş / yükleniyor + durum listesi |
| `Y21.html` | İzleme kurma formu | 2 | yüzen kart (geçerli) + zaten doğru / geçersiz + durum listesi |
| `Y34.html` | Zaman gezintisi | 2 | 900 × 520 grafik hücresi + durum listesi |
| `Y35.html` | Hazırlık dosyası, ortak analiz | 2 | liste + dosya paneli + durum listesi |
| `Y36.html` | Kurumsal işlemde beklemede | 2 | önce / sonra listeler, taşıma mesajı, soru kartı + durum listesi |

Her sayfa: başlık (kademe, kararlar, durum sayısı), akış şeridi, durumlar şartnamedeki sırayla
numaralı ve kalıp adıyla (`İ-*`, `D-*`, `O-*`) etiketli karelerde.

## Açmak

Değişken ve yazı tipi dosyaları üretilir, elle yazılmaz; çıktı git-ignored bir klasöre:

```sh
O=rounds/RD2/experiments/gallery/out/surfaces/watch/vars; mkdir -p $O
for l in tavek tavek-tel; do for t in dark light; do
  { echo ':root {'; qp look css $l --theme $t; echo '}'; } > $O/$l-$t.css
done; done
# fonts-<görünüş>.css: scripts/design-gallery'nin ürettiğiyle aynı (@font-face, crates/qp-render/assets/fonts)
```

Sonra `Y32.html?look=tavek&theme=dark&vars=<O'nun file:// adresi>/` masaüstü bölümünü,
`look=tavek-tel` telefon bölümünü gösterir (sayfa görünüşe göre bölümünü seçer). Ekran
görüntüsü: masaüstü DPR 2, telefon DPR 3.

## Grafikler

Grafik çekirdekten: her `img[data-chart]` yuvası ölçülür ve o boyutta çizilir, `&charts=<önek>`
ile verilir (`frame.js`, `srcset <dpr>x`):

```sh
qp render --transcript <tx> --view desk --look tavek --theme dark --scale 2 \
  --width <w> --height <h> -o <önek><data-chart>.png
```

`<tx>`, yuvanın `data-tx` özniteliğidir: `desk-181.json`, `desk-182.json`, `desk-186.json` bu
klasörde; `fixtures/design/desk.json`'un görünüm aralığını 120–181/182/186 barlarına koyan
kopyalarıdır (tek fark son adımdaki `set_view_range`), böylece açık bar kırılım hikâyesinin
olduğu yerdedir. `data-tx` yoksa `fixtures/design/desk.json`. Bindirmedeki seviye ve çentik
konumları `qp scene` çıktısından (fiyat ekseni ve bar konumu) hesaplanmış kesirlerdir; yuva
boyutu değişirse yeniden hesaplanmalıdır. Görseller commit edilmez.

## Kurallar

- Değerler yalnız `var(--…)`; renk taraması 0
  (`grep -rnE '#[0-9a-fA-F]{3,8}\b|rgba?\(|hsl\(' docs/design/mockups/surfaces/watch`).
- Px yalnız çerçeve geometrisi (yan sütun 256, panel 360, hücre 560, telefon 390, kesit boyları);
  `watch.css` içinde ve sayfa başı yorumunda belirtildi.
- K75: n < 20'de yüzde gizli, sayı görünür; canlı ve yeniden oynatma ayrı satır, simüle etiketli.
  K72 sessiz zil, K73 iki aşama ve periyot yazısı, K88 beklemede ve katsayılı taşıma, K77
  "Sinyal, tavsiye değil" her sicil yüzünde.

## Bilinen açıklar

- Grafik üstü katman maket (DIRECTION "Bilinen zayıflıklar"): eksen etiketi yer yer çekirdeğin
  son değer etiketine ya da ızgara fiyat etiketine biner; telefonda durum taşıyan geniş eksen
  etiketi ("120,00 │ ◐ 2/3") son barların yakınına düşer; aralık parantezi bir tarih etiketini örter.
- Grafik başlığı çekirdekten "SAMPLE 1d" gelir; kabuk metni SYN20 der (fikstür sembolü).
- Telefon kademe 2 profili yok (yalnız masaüstü ana durum).
