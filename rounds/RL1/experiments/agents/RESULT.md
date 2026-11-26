# RL1 — Ajan yazabilirlik teyidi: sonuç

Üreten `score.py` (koşu kökü `/private/tmp/claude-501/-Users-arkns-Works-Mchart-qapsule/78d3c2c6-5a39-47e7-b848-bebeba6aea4c/scratchpad/rl1-agents`; veri kümeleri seen, heldout). Elle düzenlenmez; yorum REPORT.md'ye.

## Yan yana (RX4 kol A, aynı 20 görev)

| Ölçüt | Qapsule haiku | RX4 A-haiku | Qapsule opus | RX4 A-opus |
|---|---:|---:|---:|---:|
| İlk denemede doğru | 20/20 | 20/20 | 20/20 | 20/20 |
| Sonunda doğru (≤ 3 teslim) | 20/20 | 20/20 | 20/20 | 20/20 |
| Düzelme (ilki yanlış → sonu doğru) | 0/0 | — | 0/0 | — |
| Hata iletisinden sonra doğru | 2/2 | 0/0 | 1/1 | 0/0 |
| Ret gereken yerde ret (son) | 4/4 | 4/4 | 4/4 | 4/4 |
| Ret gereken yerde ret (ilk) | 4/4 | 4/4 | 4/4 | 4/4 |
| Yanlış ret | 0 | 0 | 0 | 0 |
| Uydurma işlev adı (görev) | 0 (0 cevap) | 0 (dolaylı) | 0 (0 cevap) | 0 (dolaylı) |
| Sessiz yanlış (teslim) | 0 | 0 | 0 | 0 |
| Teslim öncesi hata alan oturum | 2 | 0 | 1 | 0 |
| Belirteç, görev başı medyan | 338810 | 270696 | 338285 | 207990 |
| Araç çağrısı, ortalama | 5.05 | 7.45 | 4.95 | 5.15 |

## Hüküm (önceden yazılı eşikler, PLAN.md §6)

- **haiku:** RX4 ile aynı bantta
- **opus:** RX4 ile aynı bantta · not: belirteç medyanı 338285 > 1,5 × RX4 207990 (doğruluk değil, maliyet)

## Ek kayıtlar

- **haiku:** teslim 20 (metin 20, IR 0); teslimsiz 0; hata 0; M5 0; check'siz teslim 0; öneri 11/11 (E_TRY_ONLY 0, add hatası 0); program aracı dışı çağrı 0; karar denemesi 0; uydurma adlar {}
- **opus:** teslim 20 (metin 20, IR 0); teslimsiz 0; hata 0; M5 0; check'siz teslim 0; öneri 11/11 (E_TRY_ONLY 0, add hatası 0); program aracı dışı çağrı 0; karar denemesi 0; uydurma adlar {}

## Görev tablosu

| Görev | Küme | haiku: teslimler | haiku: hata ileti | haiku: belirteç | opus: teslimler | opus: hata ileti | opus: belirteç |
|---|---|---|---:|---:|---|---:|---:|
| G1-01 | G1 | correct[text] | 0 | 402176 | correct[text] | 0 | 335646 |
| G1-03 | G1 | correct[text] | 0 | 338697 | correct[text] | 0 | 336532 |
| G2-01 | G2 | correct[text] | 0 | 338924 | correct[text] | 0 | 337361 |
| G2-03 | G2 | correct[text] | 1 | 473869 | correct[text] | 0 | 338179 |
| G3-01 | G3 | correct[text] | 0 | 412009 | correct[text] | 0 | 339693 |
| G3-02 | G3 | correct[text] | 0 | 338583 | correct[text] | 0 | 336160 |
| G3-03 | G3 | correct[text] | 0 | 341038 | correct[text] | 0 | 338391 |
| G4-01 | G4 | correct[text] | 0 | 343458 | correct[text] | 0 | 338006 |
| G4-02 | G4 | correct[text] | 0 | 340773 | correct[text] | 0 | 338800 |
| G4-04 | G4 | correct(E_LOOKAHEAD)[text] | 0 | 278862 | correct(E_LOOKAHEAD)[text] | 0 | 334253 |
| G5-01 | G5 | correct(E_LOOKAHEAD)[text] | 0 | 277354 | correct(E_LOOKAHEAD)[text] | 0 | 315459 |
| G5-02 | G5 | correct(E_UNSUPPORTED)[text] | 0 | 279187 | correct(E_UNSUPPORTED)[text] | 0 | 259413 |
| G5-03 | G5 | correct(E_UNKNOWN_OP)[text] | 0 | 278771 | correct(E_UNKNOWN_OP)[text] | 0 | 272755 |
| G7-01 | G7 | correct[text] | 0 | 276881 | correct[text] | 0 | 344076 |
| G7-03 | G7 | correct[text] | 0 | 277460 | correct[text] | 0 | 344178 |
| G7-05 | G7 | correct[text] | 0 | 279676 | correct[text] | 0 | 351864 |
| G8-01 | G8 | correct[text] | 0 | 344700 | correct[text] | 0 | 339857 |
| G8-03 | G8 | correct[text] | 0 | 290077 | correct[text] | 0 | 338893 |
| G9-02 | G9 | correct[text] | 1 | 564471 | correct[text] | 1 | 409110 |
| G9-03 | G9 | correct[text] | 0 | 355328 | correct[text] | 0 | 340948 |

## Yanlış teslimlerin ayrıntısı

