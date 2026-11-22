# Denetim: rd2/c2a-alphabet (0979239a) — bağımsız denetçi

Karar: **KOŞULLU KABUL**

## 1. verify.sh — PASS
`CARGO_TARGET_DIR=target/wt/audit-c2a scripts/verify.sh`: 20/20 PASS, atlanan yok (browser dahil),
1106 s, `verify: OK`. design: `c2a-trial` (dense) PASS; oltan 15 / r4 21 FAIL (legacy, düşürmez).

## 2. Legacy bayt kimliği — PASS
- `git diff --stat 9aee506c HEAD -- fixtures/{golden,compute,protocol,views}` boş.
- Taşıma incelendi (`git diff main -M -- crates/qp-grammar/src/compile`): `price::legacy`,
  `time::legacy`, `marks::tags` (legacy dalı), `legend::close` R0 kodunun birebir taşıması;
  yalnız `figure_px` → `ch.size("figure")` (= `types.figure.px`), `last_rect` yardımcıya alındı.
  `body_width(slot, 0.7)` legacy'de (ChartLook::legacy body_ratio 0.7), `mark_px` legacy'de
  kimlik, `margins` legacy'de `space.plot`, `soften/volume` legacy'de no-op. golden, views (12×2),
  demo sahneleri = altın görseller ile doğrulandı.

## 3. arch-check — PASS
0 hata, 11 adlı istisna (değişmedi). qp-grammar 20,477 (≤ 21,000). qp-session diff boş (31,999).
`arch-baseline.json` yalnız sıkılaşıyor: file.soft 37→36 (scene.rs düştü),
`scene.rs::compile` 654→564. `arch-config.json` değişmedi.

## 4. S1+G koşulları — PASS
- c3 yuvası: `min-height: calc(var(--qp-bar-h) * 3)` eklendi.
- design-gallery 0 boyutlu yuva: stderr + `process.exitCode = 1`.
- H3: `H3_SHARED` adlı 4 çift (valid=text, invalid=closed, invalid=neutral, closed=neutral);
  kalan her çift ölçülüyor.
- P2 alfa: `c[3]==0` artık 255'e çekmeden önce bakılıyor (canlı).
- CONTRACT "Ölçüm" bölümü P2, P4–P8, H3'ün kayıt/sahne kaynağını yazıyor.
- `a_bad_non_legacy_look_fails`: P4, P8, H3 düşüyor; aynı kayıt legacy iken düşmüyor.
- `--theme`/`--cursor` bars girdisiyle clap `conflicts_with = "input"`.

## 5. Alfabe kalitesi — PASS (kusurlarla)
TRIAL görünüşü, desk fikstürü, 960×600 koyu, imleç 120. bar, ölçek 2 çizildi; OpenChart
01-single-chart ile karşılaştırıldı.
Var: sağ eksen ≈52 CSS px aralık; iki düzey zaman ekseni (gün muted, ay/yıl text, TR ay adları);
O H L C + değişim + "-%1,32" (yön rengi); son değer etiketi + noktalı değer çizgisi; kesik
imleç tüm bölmelerde, fiyat hapı ve "19 Haz 2025" tarih hapı; haplar etiketlerle çakışmıyor,
çerçeve dışına taşan yok; yumuşak hacim.
Kusurlar:
a. Fiyat adımı 7,5 seçiliyor (127,50 / 120,00 / 112,50 / 97,50). STEPS içindeki 1,5 / 3 / 4 /
   7,5 profesyonel eksenlerde alışılmış değil (OpenChart 200'lük adım). P4 ±%22 toleransı
   uğruna okunurluk kaybı; 1-2-2,5-5 tercih edilmeli, P4 aralığı 40–60 zaten geniş.
b. Alt bölmelerde ölçek okunmuyor: hacim bölmesinde yalnız "1,4 Mn" etiketi, RSI bölmesinde
   yalnız "55,10"; tek ızgara etiketi son değer etiketi altında kalıp atlanıyor. Kısa bölmede
   en az bir etiket korunmalı (ya da etiket etiketten uzağa konmalı).
c. Gün içi saat etiketleri ve tarih hapı UTC (`format::hm`, `civil`): `display_tz`/borsa
   takvimi (CONCEPTS "Gösterim saat dilimi", K14) yok sayılıyor; BIST 09:30 barı "06:30"
   yazılır. Bu fikstür günlük olduğu için görünmüyor, gün içi fikstür testi yok.
d. Küçük: soldaki "2025" bağlam etiketi x=0'a sıkışıp çizim alanının solundan başlıyor.

## 6. Determinizm ve güvenlik — PASS (not)
HashMap/HashSet yok; sıralamalar tamsayı anahtarlı `sort_by_key` (kararlı); `min_by(total_cmp)`
eşitlikte ilkini döndürür. Kütüphane kodunda unwrap/expect/panic yok; `fr.bars[i]` boş bar
kontrolünden (scene.rs:103) sonra ve sınırlanmış. Metinler çekirdekte (TR/EN sayı, ay, OHLC).
Not: tasarım değerleri look kaydı dışında sabit: `VOLUME_ALPHA 0.35`, `DASH (4,3)`, `DOT (2,2)`,
`GAPS (3,8)`, `PILL_PAD_X/Y`, `LABEL_GAP 8`, `PLACE_GAP 6`, `mid` 0.36. ChartLook'a taşınmalı ya da
CONTRACT'ta "sabit alfabe" olarak yazılmalı (yönler bunları değiştiremez).

## 7. Temiz oda — PASS
`../refs/src` içinde ayırt edici adlar arandı (PILL_PAD, LABEL_GAP, PLACE_GAP, VOLUME_ALPHA,
title_figures, date_pill, " Mn", TR ay dizisi): kopya yok. time.rs'deki "a date beats the 12:00
beside it" Vela yorumundan alıntı, ama ADOPTED kartında (openchart-vela.md:130) kaynak olarak
anılıyor; algoritma (önce büyük birim, aralık çakışma testi) genel bir kalıp, kod farklı.
Hinnant `civil_from_days` kamuya açık algoritma.

## Koşullar (C2b içinde ya da ilk ürün görünüşünden önce)
1. Saat/tarih etiketleri `display_tz` (yoksa borsa takvimi) ile yazılsın; gün içi bir sahne testi.
2. Kısa bölmelerde (hacim, RSI) en az bir eksen etiketi görünsün.
3. Fiyat adımları alışılmış katlara indirilsin (1, 2, 2,5, 5) ya da 7,5/1,5 seçimi CONTRACT'ta
   gerekçelendirilsin.
4. Alfabe sabitleri ChartLook'a ya da CONTRACT'a.

## Temizlik
`target/wt/audit-c2a`, scratchpad'deki geçici render projesi, sahne JSON ve PNG silindi.
