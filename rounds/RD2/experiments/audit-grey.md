# Denetim: rd2/fix-grey (3835a034, main 2b2fc732 üstünde)

Denetçi: bağımsız alt ajan, 2026-11-23. Derleme klasörü target/wt/audit-grey (silindi).

## Hüküm: KABUL (bağlayıcı olmayan iki notla)

1. verify.sh: PASS. 20/20 adım PASS, exit 0 (fmt, clippy, test 547s, ..., golden, views, demo, arch, design).
2. Eski görünüşler değişmedi: PASS. `git diff main -- fixtures/golden fixtures/compute fixtures/protocol fixtures/views` boş (fixtures/ ve shells/ altında hiç fark yok).
   `closed_label`: r4 "closed" → `token("closed")` = `theme.closed` (eski değerle aynı); ornek `..R4`, zemre `R4.story`, oltan `..R4.story` olduğu için hepsi "closed" alır.
   golden, views ve demo adımları PASS.
3. design-check: PASS. tavek ve tavek-tel'de 52/52 satır PASS (P1–P13, H1–H3, iki tema).
   H1 5,27/5,26; H2 3,04 (koyu, gri/kanıt bandı+şerit) / 3,06 (açık, up/kendi hacmi+şerit); H3 11,3/10,7.
   grounds.rs, gri rolü (closed, neutral, invalid) yedi zeminin her birinde ölçüyor: bg, surface, card, strip, row band, evidence band, evidence band on strip.
   Kendi plakası bg ve surface üstünde ölçülüyor. up/down kendi hacim çubuğuna karşı bg ve şerit üstünde ölçülüyor.
   Geri alma denemesi: tavek.rs main'deki haline döndürüldü. tavek ve tavek-tel 4 FAIL verdi: H1 2,13/2,36 (closed label), H2 2,13/2,36 (grey/evidence band on strip).
   Bu değerler RESULTS.md'deki iddiayla birebir aynı. Sonra `git checkout -- .` ile ağaç temiz.
4. E1 "Düzeltme sonrası": PASS. Eşik altı her yüzeyde 0. Üç satırı elle yeniden hesapladım:
   - koyu gri #6e7076, kendi plakası #1e2023 üstünde = 3,30
   - tavek-tel koyu down #d67159, şerit üstündeki hacim çubuğu #5a3a36 üstünde = 3,04
   - açık gri kendi plakasında (alpha 29) = 3,31; açık up beyaz üstünde = 5,48
   Hepsi dosyadakiyle aynı.
5. arch-check: PASS. 0 hata, taban çizgisi içinde. qp-grammar test dışı 20.951 ≤ 21.000. qp-session 31.999, değişmedi (diff yok).
6. CONTRACT: PASS. design-contract "difference 0". docs/design/mockups içinde hex kod yok.

## Notlar (koşul değil)
- a) Sonraki kanıtta kabuk (DOM) eşikli metin dışı öğe sayısı 169'dan 136'ya düşmüş (DOM denetimi 338 → 272).
  a966a581..main arasında kabuk kodu değişmedi. Fark büyük olasılıkla scratch'teki analyze kopyasından ya da probe durumundan geliyor.
  En kötü değer ve eşik altı sayısı aynı kaldı. RESULTS'a bir cümlelik açıklama eklenmeli.
- b) CONTRACT tabloları strip_alpha, focus_row_alpha, grey_plate_alpha_max ve closed_label değerlerini taşımıyor; yalnız düzyazıda anılıyorlar.
  Bunları tabloya eklemek "Öneriler"e yazılabilir.
