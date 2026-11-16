# R4 — M8/M9/M10 alternatif tasarımı (her alternatif ajan için ortak)

Sen tek bir alternatif tasarımcısısın. Başka alternatiflerin var olup olmadığını, adlarını ya
da yönlerini bilmiyorsun ve aramayacaksın: başka `r4/m8*`, `r4/m9*`, `r4/m10*` dallarına,
`.claude/wt/` altındaki başka worktree'lere, `rounds/R4/experiments/M8|M9|M10/<başka ad>/`
klasörlerine bakma. Başvuru `r3`'ü (bugünkü tasarım) okuyabilirsin.

Önce: `rounds/R4/tasks/COMMON.md`; `NORTH_STAR.md`; `docs/CONCEPTS.md` (Gösterim (R4) bölümü);
`rounds/R4/GOAL.md` ("Kapsülün yüzü için tasarım sınırları"); `rounds/R4/experiments/harness/SPEC.md`
(senin deneyinin bölümü, §0, §6, §8) — seni ölçecek düzenek budur, ölçütleri önceden bil;
`rounds/R4/experiments/harness/m8910/README.md` (arayüz: senin deneyinin bölümü — tek dosya);
`shells/web/ARCH.md`.

Kurallar:
- Yalnız README'nin senin deneyin için gösterdiği dosyaya (ve gerekiyorsa onun alt modülüne)
  yazarsın; kayda tek satır. Başka bir dosyaya dokunmak zorunda kalırsan dur ve yaz.
  Kabukta (`shells/web`) özel kod yok. Düzeneği ve SPEC'i değiştirmezsin.
- Belirleyici; iki dil (TR/EN); renkler `Theme`'den; iç terim ("bar", indikatör adı) yok.
- Ad: `[a-z][a-z0-9_-]*`, sıradan bir sözcük olmayan uydurma bir ad (okuyucu istemlerinde
  yasak sözcük olur).
- Var olan altınlar ve dökümler bayt eşit kalır (varsayılan `r3` değişmez).
- Düzeneği kendi tasarımınla çalıştır (README "Komutlar": ekranlar + `DRY_RUN=1`), ekran
  görüntülerine kendin bak, sert eşikleri ve hesap ölçülerini gör, düzelt. **Gerçek okuyucu
  ajanı koşma** (`run_readers.sh` yalnız `DRY_RUN=1`). Düzeneğin `raw/` çıktılarını commit etme
  (yalnız `rounds/R4/experiments/<M>/<ad>/DESIGN.md`).
- `DESIGN.md`: amaç, ne gösterir/ne göstermez, yerleşim, metin şablonları (TR/EN örnekleri),
  bilinen zayıflıklar. Okuyucuya gitmez.
- `QP_VERIFY_BROWSER=1 scripts/verify.sh` yeşil (`feed-visitor.page.png` demo dalgalanması
  bilinen sorundur; yeniden koşunca geçiyorsa not et).
- Son mesajında: dal, commit, ad, tek paragraf özet.
