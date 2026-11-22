# Denetim: rd2/sg-merge (92ffeebb) — bağımsız denetçi

Karar: **KOŞULLU KABUL**

## 1. verify.sh — PASS
`CARGO_TARGET_DIR=target/wt/audit-sg scripts/verify.sh`: 20/20 PASS. Atlanan adım yok, browser
dahil. `design` adımı 13 s sürdü, toplam süre 1009 s, `verify: OK`.

## 2. Var olan davranış — PASS
- `git diff --stat 9aee506c HEAD -- fixtures/{golden,compute,protocol,views}` boş.
- main (90658063) geçici worktree'de ayrı derlendi. `qp look css` r4/oltan/ornek/zemre × dark/light:
  ilk 48 satır main ile birebir aynı ve aynı sırada; 36 satır yalnızca sona eklenmiş; tekrarlanan
  değişken adı yok.
- `qp look show` r4/oltan: değişen ya da kaybolan anahtar yok; yalnızca `chart`, `origin`, `profile`
  eklenmiş.
- `qp render --transcript fixtures/design/desk.json --view desk` (r4, oltan; --scale yok) main ile
  bayt bayt aynı. `--scale 2` 2400×1200 üretiyor. `--theme purple` düzgün bir hatayla reddediliyor.

## 3. arch-check — PASS
0 hata. qp-grammar 19,579 (≤ 21,000), qp-session 31,999 (main ile aynı, crate'e dokunulmamış),
qp-cli 1,378 test dışı satır. Borç sayıları main ile aynı (fn.lines 177, file.soft 37, ...).
`git diff main -- scripts/arch-*.json` boş.

## 4. design-check — PASS (notlarla)
- `scripts/design-check` ve `--look oltan` çıkış kodu 0. oltan 15 FAIL, r4 21 FAIL gösteriyor; ikisi de
  legacy olduğu için düşürmüyor.
- Eşikler tek yerde: `tests/design_check/thresholds.rs`.
- `design-contract --check` gerçekten karşılaştırıyor: geçici kopyada `chrome.bg` ve kart yarıçapı
  değiştirildi, ikisi de rc=1 ve diff verdi.
- P1, P3, P10–P13, H1, H2 VISUAL §5 tanımlarına sadık. P4 ve P6 sahneden ölçülüyor.
- Gözlemler:
  a. Kayıtlı legacy olmayan görünüş yok, yani bugün geçit boş geçiyor. Uçtan uca "legacy olmayan
     görünüş düşürür" testi de yok; yalnızca P10/P12 için TRIAL birim testi var.
  b. P2, PNG kümelemesi yerine SceneDesc ilkel renklerinden ölçülüyor ve bütün nötrler tek ton
     sayılıyor. Bu, VISUAL'den daha gevşek bir yorum; CONTRACT'ta açıklanmalı. `scene.rs` p2'de
     `c[3]==0` denetimi ölü kod (alfa önceden 255'e çekiliyor).
  c. P5 "iki düzey" ve P7 "imleç + haplar" sahneden değil kayıttaki bayraktan okunuyor (derleyici
     ChartLook'u C2'den sonra okuyacak). P8 "drawn" fikstürde glyph katmanı olmadığı için hep 0.
  d. H3, normal görmede ΔE<5 olan çiftleri "bilerek paylaşılmış" diye atlıyor. Bu yüzden yanlışlıkla
     eşit olan up/down da geçer. Atlama adlı izin listesiyle (neutral=invalid=closed, valid≈text)
     sınırlanmalı.

## 5. Maketler — PASS
Renk literali grep sonucu 0 (`white` geçen yerler `white-space`). Tek betik `frame.js`: parametre,
değişken bağlama, img srcset, hazır işareti; ürün mantığı yok. Ayırt edici sınıf adları
`../refs/src` içinde arandı: `.agent-thread` ad olarak OpenChart'ta da var ama içerik farklı
(`@apply` / Tailwind'e karşı `--qp-space`); kopya bulunmadı.

## 6. Galeri — FAIL (küçük)
`--looks r4 --anchors c1,c3 --no-build` ve `c2,c4` çalıştırıldı.
- `out/` git-ignored (`!!`).
- Etiketlerde boyut ve DPR var ("r4, koyu, 1440×900 CSS, DPR 2"). Çekimler 2880×1800 ve 1170×2532.
- Başvuru görselleri `file:///…/refs/src/…` üzerinden yerinde okunuyor; kopyalanmıyor.
- **Hata:** r4 c3'te öneri kartının grafik yuvası 0 yükseklikte ve grafik yok. Nedeni:
  `c3.css:52 height: calc(var(--m-control-h) * 3)`, r4'te `--qp-control-h: auto` (control_h 0 =
  içerik), yani geçersiz bir calc. `design-gallery` 0 boyutlu yuvayı sessizce atlıyor
  (`if (s.w < 1 || s.h < 1) continue;`). oltan doğru çiziliyor. c1, c2 ve c4 r4'te sağlam.

## 7. Kod incelemesi — PASS
Değişiklikler küçük ve okunur. `write_png` ölçek 1'de başvuru yolunu koruyor. CLI testleri
eklenmiş. GRAMMAR.md, look/README ve verify.sh başlığı güncellenmiş. Fonksiyon ve dosya
bütçeleri arch-check içinde. Küçük not: `--theme`, bars girdisiyle (R0 yolu) sessizce
yok sayılıyor.

## Koşullar (birleştirmeden önce ya da G/S2 içinde)
1. c3 yuvası `--qp-control-h: auto` durumunda çökmesin (sabit ya da `max()` geri dönüşü).
   design-gallery 0 boyutlu grafik yuvasında uyarı versin ya da düşsün.
2. H3'ün paylaşılmış rol atlaması adlı çiftlerle sınırlansın.
3. P2'nin (SceneDesc, nötr tek ton), P5, P7 ve P8'in kayıttan okunan kısımları CONTRACT.md
   ve REPORT içinde açıkça yazılsın; P2'deki ölü alfa denetimi düzeltilsin.
4. İlk legacy olmayan görünüş gelmeden, legacy olmayan bir görünüşün testi düşürdüğünü gösteren
   bir test eklensin.

## Temizlik
`target/wt/audit-sg`, geçici main worktree'si, main derleme klasörü ve galeri `out/` silindi.
