# R5 — Araştırma: yeni nesil kapsül arayüzü (T1)

> Taslak (ara kayıt). Tarih: 2026-11-17. Bölümler doldukça güncellenir.

## Yazı tipi ölçümleri (yerel deneme, 2026-11-17)

Dosyalar `github.com/google/fonts` deposunun `ofl/` klasöründen indirildi; ölçüm fontTools 4.68 ve
fontdue 0.9.5 (depodaki sürüm) ile scratchpad'de yapıldı. Ayrıntılı tablo son sürümde.

Önemli bulgular:
- fontdue 0.9.5, ttf-parser'ı `variable-fonts` özelliği olmadan kullanır; değişken yazı tipini
  yalnız **varsayılan örneğiyle** çizer (Inter değişken dosyası = Inter 400 statik örneği, mürekkep
  toplamı bayt bayt aynı: 173639). Figtree, Public Sans, Source Sans 3, Space Grotesk, Recursive'in
  varsayılan ağırlığı 100–300'dür; fontdue'da ince çizilir.
- fontdue şekillendirme yapmaz (yalnız cmap): `tnum` özelliği uygulanamaz. Varsayılan rakamları
  orantılı olan aileler (Inter, Geist, Atkinson Next, Figtree, Public Sans, Space Grotesk)
  cmap'te rakamlar `tnum` glifine yeniden eşlenmiş statik örnekle kullanılmalı (denendi: 10/10).
- IBM Plex ("Plex") ve Source Sans 3 ("Source") **Ayrılmış Yazı Tipi Adı** taşır; alt küme/örnekleme
  değiştirilmiş sürüm sayılır, adı değiştirilmeli.
