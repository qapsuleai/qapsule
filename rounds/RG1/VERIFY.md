# RG1 — Doğrulama

| # | Komut / bakış | Beklenen |
|---|---------------|----------|
| 1 | `git log --oneline -- rounds/RG1/experiments/baseline \| tail -1` | Tutanak commit'i, ilk kod taşımasından eski |
| 2 | `scripts/rg1-compare.sh` | Tutanaktaki bütün kalemler bayt eşit; fark 0 |
| 3 | `scripts/verify.sh` | Bütün adımlar PASS (isteğe bağlı adımlar önceki gibi); yeni `arch` adımı PASS |
| 4 | `git diff --stat <tutanak-commit> HEAD -- fixtures/golden fixtures/compute fixtures/protocol fixtures/views` | Boş |
| 5 | `scripts/arch-check` | Kesin ihlal 0; taban çizgisi içindekiler listeleniyor |
| 6 | `scripts/arch-check --self-test` | Bilinen ihlal örneklerinin hepsi yakalandı |
| 7 | `find crates -name '*.rs' -path '*/src/*' -exec wc -l {} + \| sort -rn \| sed -n 2,4p` | 800 üstü yok (üretilmiş ve işaretli olanlar hariç) |
| 8 | Crate satır sayıları tablosu (rapor) | `qp-cli`, `qp-wasm` < 4 bin ya da istisna isteği; 12 bin üstü crate yok ya da istisna isteği |
| 9 | Taban çizgisine bilerek bir aşım ekleyip `arch-check` | Hata veriyor (mandal); deneme geri alındı |
| 10 | `ls crates/*/README.md \| wc -l` = crate sayısı | Eşit |
| 11 | `docs/PARAMETERS.md` ve üretilen parametre belgesi | Envanter; ≥ 10 sabit kayıt defterinden okunuyor |
| 12 | `DECISIONS.md` bölünme karşılaştırması (betik) | Bölünmeden önceki ve sonraki karar satırları kümesi eşit; kırık başvuru 0 |
| 13 | Performans ve WASM boyutu tablosu | Tutanağa göre sınırlar içinde |
| 14 | `git worktree list`; `du -sh target` | Yalnız ana ağaç; ≤ 6 GB |
| 15 | `rounds/RG1/AUDIT.md`, `STATUS.md`, `ROADMAP.md` | Denetim hükmü; durum güncel |
