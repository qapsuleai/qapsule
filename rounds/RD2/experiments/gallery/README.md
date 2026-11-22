# RD2 karşılaştırma düzeneği (G)

Dört çapa ekranı (GOAL §2) her görünüş ve iki tema için aynı veriyle, aynı boyutta ve
doğru DPR'de çeker; yanına başvuru ekranını koyar (P14).

## Çalıştırmak

```sh
scripts/design-gallery --looks a,b,c          # oltan her zaman eklenir
scripts/design-gallery --looks a,b,c --real SYM   # yerel gerçek bar (fixtures/local/bars), commit edilmez
scripts/demo.sh --gallery                     # düzeneği kurar, yolları yazar, sonra R5 galerisiyle gösterim
QP_DEMO_GALLERY_ONLY=1 QP_DESIGN_LOOKS=a,b,c,oltan scripts/demo.sh --gallery
```

Çıktı `out/` altında (git-ignored): `compare.html` (tek sayfa: satır = çapa × tema,
sütun = görünüşler + başvuru), `index.html` (görünüş başına galeri; her görsel canlı maketi
açar), `layout-<görünüş>.json` (P9 girdisi: `regions`, `chart_panes`, `legend_numbers`),
`manifest.json`, `shots/`, `charts/`, `vars/`.

Seçenekler: `--anchors c1,c2,c3,c4`, `--fixture DOSYA --view ID`, `--out DİZİN`,
`--no-build`. Playwright `shells/web/node_modules` içinden (worktree'de yoksa ana ağaçtan)
yüklenir.

## Kurallar

- **Aynı veri:** bütün görünüşler tek fikstürden. Sıra: `--fixture`; yoksa
  `fixtures/design/*.json` içinde `desk` görünümü olan ilk dosya; yoksa
  `fixtures/views/v1-three-panels.json` görünüm `three` (fiyat, RSI, MACD; hacim yok).
- **Aynı boyut, doğru DPR:** Ç1 1440×900 ve Ç2 960×600 DPR 2; Ç3, Ç4 390×844 DPR 3.
  Grafik çekirdekten (`qp render --look --theme --scale <dpr>`), boyutunu maketteki
  yuvadan ölçerek; yeniden örnekleme yok (`srcset <dpr>x`, `object-fit: none`).
- **Başvuru yanda:** `docs/design/ADOPTED.md` "Galeri başvuru ekranları"; `../refs`
  içinden yerinde okunur, depoya kopyalanmaz. Telefon başvuruları kısmidir (DPR 1,
  küçültülmüş masaüstü): biçim için değil içerik yoğunluğu için.
- **Etiket görselin üstünde:** görünüş, tema, CSS boyutu, DPR.
- **Maket:** `docs/design/mockups/<görünüş>/cN.html` varsa o (yönün kopyası), yoksa
  `_base/cN.html`. Telefon çapaları `<görünüş>-tel` kayıtlıysa onu kullanır.
  Değişkenler `qp look css` çıktısından üretilir; elle yazılmaz.
- Gerçek veriyle (`--real`) üretilen hiçbir görsel commit edilmez.

## `legend_numbers` tanımı

Ç1 grafiğinin sahnesinde bölmelerin içinde kalan (fiyat ekseni ve zaman ekseni dışı)
metinlerdeki sayı belirteçlerinin sayısı (ör. `C 124.69` → 1).
