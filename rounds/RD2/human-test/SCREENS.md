# Görseller — hangi ekran, nasıl üretilir

Hiçbir görsel commit edilmez. Aday görselleri karşılaştırma düzeneğinden
(`scripts/design-gallery`, `rounds/RD2/experiments/gallery/README.md`), başvurular
`../refs`'ten gelir (`docs/design/ADOPTED.md` "Galeri başvuru ekranları").

## 1. Üret (sentetik; teste giren tek kaynak)

```sh
scripts/design-gallery --looks tavek        # oltan her zaman eklenir; iki tema
```

- Veri: `fixtures/design/desk.json` görünüm `desk` (sentetik; aynı veri bütün görünüşlerde).
- Boyut: Ç1 1440×900, Ç2 960×600 (DPR 2); Ç3, Ç4 390×844 (DPR 3). Telefon çapaları `tavek-tel`.
- Çıktı (git-ignored): `rounds/RD2/experiments/gallery/out/` → `shots/<çapa>-<görünüş>-<tema>.png`,
  `compare.html`, `index.html`, `manifest.json` (`"real": null` olmalı).
- **`compare.html`** (yürütücünün kendi bakışı için; katılımcıya gösterilmez, adları yazar):
  `open rounds/RD2/experiments/gallery/out/compare.html` — satır = çapa × tema, sütun =
  `tavek`, `oltan`, başvuru. Tarayıcı %100, masaüstü ekranda.

## 2. Nötr kodlu klasör (katılımcı bunu görür)

```sh
rounds/RD2/human-test/screens.sh            # → ~/qp-ht-v4/{blind.html, screens/K*.png, B*.png}
```

Tarayıcının adres çubuğuna `file:///Users/<kullanıcı>/qp-ht-v4/blind.html?o=1` yazın (sıra 1;
telefon bloğu `&blok=tel`, açık tema `&tema=a`). Boşluk: "Hazır mısınız?" → ekran (5 sn, sonra
kendiliğinden kapanır ve soruları gösterir) → sonraki. ← önceki ekranı yeniden açar.

`screens.sh` hedefi depo içine izin vermez. Telefon görsellerini (`K5`–`K8`, `B3`, `B4`) AirDrop
ile telefonun Dosyalar uygulamasına atın; ad sırasıyla durur, yürütücü sıradaki kodu açar.
Test bitince `rm -rf ~/qp-ht-v4` ve telefondaki kopyalar silinir.

## 3. Gerçek veri (yalnız yürütücünün provası; teste girmez)

```sh
scripts/design-gallery --looks tavek --real GARAN --out "$HOME/qp-ht-real"
```

Bar `fixtures/local/bars`'tan okunur. `--out` **depo dışında** ve ayrı bir klasör olmalı
(düzenek çıktı klasörünü önce siler). Bu görseller commit edilmez, paylaşılmaz, katılımcıya
gösterilmez; prova bitince `rm -rf ~/qp-ht-real`.

## 4. Önce / sonra çiftleri (`oltan` → `tavek`, koyu; açık tema `-light`)

| Çapa | Önce (R5) | Sonra (RD2) | Kod (önce → sonra) | Başvuru |
|---|---|---|---|---|
| Ç1 masaüstü çalışma alanı | `c1-oltan-dark.png` | `c1-tavek-dark.png` | K1 → K3 | B1 OpenChart panosu |
| Ç2 grafik hücresi | `c2-oltan-dark.png` | `c2-tavek-dark.png` | K4 → K2 | B2 OpenChart tek grafik |
| Ç3 telefon kontrol kulesi | `c3-oltan-dark.png` | `c3-tavek-tel-dark.png` | K7 → K6 | B3 OpenTerminalUI (kısmi) |
| Ç4 telefon grafik + açılmış kapsül | `c4-oltan-dark.png` | `c4-tavek-tel-dark.png` | K5 → K8 | B4 OpenTerminalUI (kısmi) |
