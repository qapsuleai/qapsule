# RD2 — Doğrulama

| # | Komut / bakış | Beklenen |
|---|---------------|----------|
| 1 | `scripts/verify.sh` | Bütün adımlar PASS; yeni `design` adımı PASS |
| 2 | `scripts/arch-check` ve `--table` | 0 hata; `qp-session` test dışı satır RG1 sonundan büyük değil; `qp-grammar` ≤ 21.000; taban çizgisi büyümedi |
| 3 | `git diff --stat <round başı> HEAD -- fixtures/golden fixtures/compute fixtures/protocol fixtures/views` | Yalnız eklenen dosyalar; değişen var olan dosya yok (A62'nin kaldırdığı döküm varsa raporda adıyla) |
| 4 | `qp mcp --help`; `cargo test -p qp-mcp human_decision` | Eski `--init --principal` yolu yok; test geçiyor |
| 5 | Kabuk WASM paketi gzip önce/sonra | RG1 tutanağındaki değerin (2.506.216 B) üstünde değil ya da fark raporda gerekçeli |
| 6 | `git branch --list 'recovered/*' \| wc -l`; `ls rounds/R3/experiments/harness/target` | 0; yok |
| 7 | `scripts/design-check` (seçilen görünüş, iki profil, iki tema) | P1–P13 geçiyor ya da sözleşmede Arkin onaylı gerekçe |
| 8a | `git log --oneline -- docs/design/ADOPTED.md \| tail -1`; kartlarda kaynak, yer, fark | Yön dallarından eski; dış kaynaktan kopya kod/CSS taraması 0 |
| 8b | `.claude/skills/qapsule-design/SKILL.md`; taze alt ajan sınaması | Var; sınama maketi `design-check`'ten geçiyor |
| 8 | `docs/design/CONTRACT.md` içindeki her değer ↔ görünüş kaydı (üretilmiş tablo ya da test) | Fark 0 |
| 9 | Galeri: dört çapa × üç yön + `oltan` + başvuru; DPR ve boyut etiketi görselin üstünde | Aynı veri, aynı boyut |
| 10 | `rounds/RD2/DECISION.md` | Ara karar ve son kabul Arkin'in sözleriyle, tarihli |
| 11 | `docs/design/SURFACES.md` | Y01–Y37; her yüzeyde durum listesi, köken, veri durumu, masaüstü/telefon |
| 12 | `docs/design/mockups/` | Kademe 1 tam, kademe 2 ana durum; renk ve ölçü değerleri `css_vars`'tan (elle yazılmış renk kodu taraması 0) |
| 13 | Karşıtlık, renk körlüğü; K52 (`calibrate.mjs` PHONE), K17 | Eşikler tutuyor; sayılar raporda |
| 14 | `git status --porcelain --ignored` özeti; `git ls-files fixtures/local` | Dış projelerin görselleri ve gerçek veriden üretilmiş görsel izlenmiyor |
| 15 | `git worktree list`; `du -sh target`; `AUDIT.md`, `REPORT.md`, `STATUS.md`, `ROADMAP.md` | Yalnız ana ağaç; ≤ 6 GB; belgeler güncel |
