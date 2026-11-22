# Ajan grubu yüzey maketleri (RD2 M2/M3)

Yön `tavek`. Şartname: `docs/design/surfaces/agent.md` (Y04, Y05, Y06, Y14, Y24),
`watch.md` (Y12, Y13), `system.md` (Y25). Ortak kurallar `docs/design/SURFACES.md`.

| Sayfa | Yüzey | Kademe | Gösterilen | Profil |
|---|---|---|---|---|
| `Y04.html` | Çizim niyet kartı | 1 | 12/12 durum + 3 veri durumu; akış karesi (grafik + panel) | masaüstü + telefon |
| `Y05.html` | Netleştirme sorusu | 1 | 8/8 durum + bayat; panel içinde yerinde kare | masaüstü + telefon |
| `Y06.html` | Öneri onay / ret | 1 | 10/10 durum + bayat, simüle; grafik önizlemesi; OpenTerminalUI ile yan yana | masaüstü + telefon |
| `Y12.html` | Bildirim | 1 | 10/10 durum; sağ üst yığın yerinde | masaüstü + telefon (kilit ekranı, üst şerit) |
| `Y25.html` | Veri ve piyasa durumu şeridi | 1 | 10/10 durum + iyileşme bildirimi; Ç1 alt şerit yerinde | masaüstü + telefon |
| `Y13.html` | Gelen kutusu | 2 | ana durum (zilden açılan liste) + 6/6 küçük kare + özet | masaüstü |
| `Y14.html` | Ajan etkinlik günlüğü | 2 | ana durum (Etkinlik sekmesi) + 7/7 küçük kare + özet | masaüstü |
| `Y24.html` | Ajan paneli | 2 | ana durum (320 × 860, bir tur) + 9/9 küçük kare + özet; OpenChart ile yan yana | masaüstü |

`agent.css` grubun ortak dosyasıdır; `_base/base.css` ve `tavek/tavek.css` üzerine kurulur.
Değerler yalnız look değişkenlerinden; renk kodu yok. Px yalnız çapa geometrisinde (yorumlu).

## Açma

Değişken dosyaları ve grafik görselleri git-ignored bir klasöre üretilir (commit edilmez):

```sh
QP=target/debug/qp
OUT=rounds/RD2/experiments/gallery/out/surfaces/agent
mkdir -p $OUT/vars
for l in tavek tavek-tel; do for t in dark light; do
  { echo ":root {"; $QP look css $l --theme $t; echo "}"; } > $OUT/vars/$l-$t.css
done; done
# fonts-<look>.css: scripts/design-gallery'nin writeFonts'u ile aynı biçim
# (qp look show <look> | fonts → @font-face, crates/qp-render/assets/fonts/).
for t in dark light; do
  mkdir -p $OUT/charts/$t
  $QP render --look tavek --theme $t --scale 2 --transcript fixtures/design/desk.json \
    --view desk --width 720 --height 340 -o $OUT/charts/$t/desk.png
  $QP render --look tavek-tel --theme $t --scale 3 --transcript fixtures/design/desk.json \
    --view desk --width 358 --height 220 -o $OUT/charts/$t/phone.png
done
```

Sayfa parametreleri (`_base/frame.js`):

- Masaüstü: `Y06.html?look=tavek&theme=dark&vars=<OUT>/vars/&charts=<OUT>/charts/dark/&dpr=2`
- Telefon: `Y06.html?look=tavek-tel&theme=dark&vars=<OUT>/vars/&charts=<OUT>/charts/dark/&dpr=3`
  (390 px genişlikte açın; kademe 2 sayfalarında telefon bölümü yalnız not taşır).

Aynı sayfa iki profili taşır; `?look=` hangisinin görüneceğini seçer.

## Başvuru kırpımları

Y24 ve Y06, başvuru ekranını `../../../../../../refs/src/...` yolundan (ana ağaçtan açıldığında
`Mchart/refs`) `<img>` ile gösterir; görsel depoya kopyalanmaz (K83). Worktree içinden açarken
bu yol `.claude/worktrees/refs`'e düşer; görsel görünmezse kırpım boş kalır, maketin geri kalanı
etkilenmez.

## Bilinen açıklar

- Grafik çekirdekten `desk` görünümüdür (SAMPLE, son 124,69); metindeki 112,40 / 117,50 seviyeleri
  şartnamenin örnek sayılarıdır, bindirme konumları kesirdir (veriye bağlı değil). 117,50 eksen
  etiketi çekirdeğin 120,00 fiyat etiketine değiyor (DIRECTION "Bilinen zayıflıklar": eksen
  etiketinin altındaki fiyat etiketini düşürmek gramer işi).
- Yoğun profil (gövde 13) başvurulardaki ajan panelinden küçük ve sıkı okunur; panel ≤ %30 kuralı.
