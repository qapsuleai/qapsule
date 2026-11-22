# Sistem grubu yüzey maketleri

Yön `tavek`. Şartname: `docs/design/surfaces/system.md`. Ortak kurallar `docs/design/SURFACES.md`.
Grubun öteki maketleri başka klasörlerde durur: Y25 `../agent/`, Y36 `../watch/`.

| Sayfa | Yüzey | Kademe | Gösterilen | Profil |
|---|---|---|---|---|
| `Y22.html` | Komut paleti ve sembol arama | 3 (maket sınaması, RD2 B) | ana durum (Ç1 üst kesiti, yazıyor) + 7/7 durum + özet | masaüstü + telefon |

`system.css` grubun ortak dosyasıdır; `_base/base.css` ve `tavek/tavek.css` üzerine kurulur.
Değerler yalnız look değişkenlerinden; renk kodu yok. Px yalnız çerçeve geometrisinde (yorumlu).

## Açma

Değişken, yazı tipi ve grafik dosyaları git-ignored bir klasöre üretilir (`../agent/README.md`
"Açma" ile aynı). Y22'nin tek grafik yuvası `bg`:

```sh
qp render --look tavek --theme <t> --scale 2 --transcript fixtures/design/desk.json \
  --view desk --width 1374 --height 440 -o <OUT>/charts/<t>/bg.png
```

- Masaüstü: `Y22.html?look=tavek&theme=dark&vars=<OUT>/vars/&charts=<OUT>/charts/dark/&dpr=2`
- Telefon: `Y22.html?look=tavek-tel&theme=dark&vars=<OUT>/vars/&dpr=3` (390 px genişlikte)

## Bilinen açıklar

- Kısayol harfleri (⌥ I, ⌥ L, ⌥ H, ⌥ 2) kalıptır; kayıt çekirdekte olacak (OV-23), çakışma
  sınaması AT-D1.
- Grafik başlığı çekirdekten "SAMPLE 1d"; kabuk metni SYN20 der (fikstür sembolü).
- Telefon davranışı (gezinme grubu, Ajan sekmesine geçiş) **Varsayım (S25)**.
