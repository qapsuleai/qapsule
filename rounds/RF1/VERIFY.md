# RF1 — Doğrulama

| # | Komut / bakış | Beklenen |
|---|---------------|----------|
| 1 | `git log --oneline -- rounds/RF1/experiments/baseline \| tail -1` | Tutanak commit'i, ilk kod değişikliğinden eski |
| 2 | `scripts/refactor/compare.sh` | Bütün kalemler bayt eşit; fark 0 |
| 3 | `scripts/verify.sh` | Bütün adımlar PASS (`arch`, `design` ve yeni yapılandırma farkı adımı dahil) |
| 4 | `git diff --stat <tutanak> HEAD -- fixtures/golden fixtures/compute fixtures/protocol fixtures/views fixtures/schema` | Boş (yalnız ön işin eklediği ayrı tasarım fikstürü, adıyla) |
| 5 | `scripts/arch-check`, `--table`, `--self-test` | 0 hata; 12 bin üstü crate 0 ya da tek adlı istisna; yan bağımlılıklar bildirilmiş sırada, İ2 kalmadı; öz-sınama PASS |
| 6 | `arch-check --table`: `qp-grammar`, `qp-capsule`, `qp-session` ve yeni crate'ler | `qp-grammar` ≤ 12.000; `qp-capsule` ≤ 12.000; R7 işaretlerinin crate'inde ≥ 3.000 pay |
| 7 | `find crates -name '*.rs' -path '*/src/*' \| xargs wc -l \| sort -rn \| sed -n 2,4p` | 800 üstü yalnız üretilmiş tablo |
| 8 | `git diff <tutanak> HEAD -- scripts/arch-baseline.json scripts/arch-config.json` | Yalnız sıkılaşma; kapanan istisnalar silinmiş; yeni istisna yok ya da `DECISION.md`'de |
| 9 | Ek noktası sınaması; `git diff --stat` o sınamanın eklediği dosyalar | Geçiyor; otorite crate'inin `src/` altında değişiklik yok |
| 10 | Kayıt enjeksiyonu: `cargo tree -p qp-grammar` | Tasarım crate'lerine bağımlılık yok |
| 11 | Performans (K17 A/B aynı saatte; K52 `calibrate.mjs`), WASM boyutları | Gürültü içinde; ±%2 |
| 12 | `rounds/RF1/PLAN.md`, `DECISION.md`; `git log` sırası | Onay, gramer ve oturum kod değişikliklerinden eski |
| 13 | Rapor: yeniden yazılan öğe listesi, dışa açık yol değişiklikleri, kapanan ve kalan istisnalar | Var |
| 14 | `git worktree list`; `du -sh target`; `AUDIT.md`, `REPORT.md`, `STATUS.md`, `ROADMAP.md` | Yalnız ana ağaç; ≤ 6 GB; belgeler güncel |
