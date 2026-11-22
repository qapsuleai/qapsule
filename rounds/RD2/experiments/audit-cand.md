# RD2 denetimi — `rd2/candidate` (a966a581)

Karar: **KOŞULLU KABUL**

Bağımsız denetçi. Kod değiştirilmedi. Derleme klasörü `target/wt/audit-cand` (12 GB) silindi.

## Kontroller

1. **verify.sh — PASS.** 20/20 adım (design 15 sn dahil), toplam 1015 sn, `verify: OK`. wasm gzip 769.830 B.
2. **Satır 2 (arch) — PASS.** 0 hata, 11 adlı istisna. qp-session 31.999 (tavan 31.999), qp-grammar 20.933 (tavan 21.000, 67 satır pay kaldı). Taban çizgisi farkı yalnız sıkılaştırıyor: file.soft 37→36, `scene.rs::compile` 654→564. arch-config'e yalnız K89 `max` mandalları eklendi.
3. **Satır 3 (fixtures) — PASS.** `git diff --stat 9aee506c HEAD -- fixtures/{golden,compute,protocol,views}` boş. Yalnız `fixtures/design/{desk.json,docs/syn20.json}` eklendi; ikisi de sentetik.
4. **Satır 7 (design-check) — PASS.** tavek ve tavek-tel: P1–P13 ve H1–H3 iki temada PASS (P3 açık temada n.a., P9 düzen dökümü verilmeden n.a.). P2 ve P6 desk ile desk-levels sahnelerinden ölçülüyor (P2 5/6 ton, eşik ≤6). P4, P5 ve P7 sahneden ölçülüyor. Zayıflık: P7 ve P8 yalnız desk sahnesine bakıyor; P8'in "çizilen" değeri 0, çünkü fikstürde Glyph işareti yok. P8 bu yüzden fiilen yalnız kayıt değerini (20/24 ≤ 28) ölçüyor; seviye hapları P8'e girmiyor.
5. **Satır 8 (CONTRACT) — PASS.** `design-contract: ... difference 0`; `contract_lists_the_fixed_measures` testi ok. Üretilmiş blokların dışında yalnız "Sabit alfabe ölçüleri" listesi ile eşik ve ölçüm anlatımı var. 7,5/1,5 fiyat merdiveni CONTRACT'ta gerekçeli (C2a koşul 3).
6. **Satır 8a (ADOPTED, temiz oda) — PASS.** ADOPTED ilk commit'i 9e7c5bd4 (20:52); y-a/b/c dallarının kendi commit'leri 21:50 ve sonrası. `../refs/src` ile ≥45 karakterlik 426 satır karşılaştırıldı: 3 tam eşleşme, hepsi genel kalıp (iki `grid-template-columns` satırı, Hinnant `doy`). Ayırt edici adlar (intent-pill, ax-tag, NumFmt, PLACE_GAP …) için eşleşme 0. LABEL_GAP ve t-bracket eşleşmeleri farklı ad ve bağlamda.
7. **Satır 11 (SURFACES) — PASS.** Y01–Y37 ayrı başlıklar (37). Her başlıkta Durumlar tablosu, Köken, Veri durumu ve Masaüstü/telefon var.
8. **Satır 12 (maketler) — PASS, bir not.** Kademe 1'in 15 maketinin hepsi var. Durum etiketleri şartnamedeki durum sayısının yaklaşık 2 katı (iki profil). Y26 ve Y27 telefon yüzeyi; masaüstü karşılığı not olarak verilmiş. Kademe 2'nin 15 maketi var; Y22 kademe 3, B-sınaması için yapıldı. Ürün mantığı yok: frame.js yalnız değişken, yazı tipi ve görsel bağlıyor. **Renk taraması 1 sonuç veriyor:** `docs/design/mockups/tavek/DIRECTION.md:62` düzyazıda `#7c7f86` ve `#73767d` geçiyor. CSS ve HTML'de 0.
9. **Satır 8b (beceri) — PASS.** `.claude/skills/qapsule-design/SKILL.md` var. `rounds/RD2/DECISION.md` dışındaki bütün yollar geçerli. DECISION.md yalnız main 9aae35f7'de; birleştirme çakışmasız (`merge-tree` temiz) ve dosyayı getirir. Beceride değer yok. `.gitignore` yalnız `.claude/skills/` klasörünü izlenir yapıyor. Y22 renk taramasından geçiyor.
10. **Satır 14 (hijyen) — PASS.** Bu dal hiç görsel eklemiyor. `fixtures/local` izlenmiyor. `screens.sh` refs görsellerini depo dışına kopyalıyor; hedef depo içiyse betik reddediyor.
11. **Protokol ve şema — PASS.** `crates/qp-schema` ve `docs/PROTOCOL.md` farkı boş. `PRESENTED_LOOK = "oltan"`.
12. **Kod incelemesi — PASS.** alphabet ve look'ta unwrap, expect ve panic yok. Dizinlemeler sınırlı ve korumalı (boş bar erken dönüyor). HashMap, saat ve rastgelelik yok. En büyük dosya time.rs, 215 satır. Kapılama `familiar() = profile != Legacy`; eski görünüşler `ChartLook::legacy` kullanıyor. Golden, views, demo ve digest bayt eşit.

## Koşullar (birleştirmeye engel değil)
1. DIRECTION.md:62'deki iki onaltılık renk kodu ya rol adına çevrilsin ya da tarama kapsamı CSS ve HTML ile sınırlandığı REPORT'ta yazılsın.
2. P8 seviyeli sahneyi de ölçsün ve fikstürde en az bir Glyph işareti olsun; yoksa REPORT, P8'in bugün yalnız kayıt değerini ölçtüğünü söylesin.
3. `shells/web/style.css`'teki `.trust` arka planı (`--qp-scene-bg`) oltan'da da etkin. Golden ve browser yeşil, ama REPORT'ta anılsın.
4. qp-grammar'da yalnız 67 satır pay kaldı; sonraki gramer işi önce bölme ya da karar ister.
