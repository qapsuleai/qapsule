# RP1 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `scripts/verify.sh` | Çıkış kodu 0; `compute` ve `digest` adımları değişmeden geçer |
| 2 | Hesap özeti `fixtures/compute/digest.txt` | R1'deki değerle aynı |
| 3 | Dört yük profilinin ölçümü yeniden çalıştırılır | `RESEARCH.md` tablolarıyla uyumlu (±%20) |
| 4 | CI koşucusundaki ölçüm (`scripts/qgh` ile son çalışma) | K17 dizüstü hedefleriyle karşılaştırma tablosu |
| 5 | `RESEARCH.md` içinde en az altı yaklaşım, her biri hipotez + ölçüm + karar | Var |
| 6 | Diğer crate'lerde değişiklik `git diff --stat` | Yalnızca `crates/qp-compute/`, `rounds/RP1/` ve durum dosyaları |
