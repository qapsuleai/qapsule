# RL1 — Doğrulama

| # | Komut / bakış | Beklenen |
|---|---------------|----------|
| 1 | `scripts/verify.sh` | Bütün adımlar PASS; yeni `program` adımı PASS |
| 2 | `scripts/refactor/compare.sh` (RF1 tutanağına karşı, ön işlerden sonra) | Var olan kalemler bayt eşit (şema dışa aktarımı ve araç tanımları yalnız eklemeyle farklı; fark dökümü raporda) |
| 3 | `git diff --stat <round başı> HEAD -- fixtures/golden fixtures/compute fixtures/protocol fixtures/views` | Yalnız eklenen dosyalar |
| 4 | `git diff --stat <round başı> HEAD -- crates/qp-authority/src` | Boş ya da yalnız ARCHITECTURE'da adı geçen yumuşak dosya |
| 5 | `scripts/arch-check`, `--config-diff` | 0 hata; yeni istisna yok; taban çizgisi büyümedi |
| 6 | Kabuk WASM paketi ham / gzip; varsayılan modül | İ8 tavanı içinde (ön iş 1'den sonraki değerle) |
| 7 | `git log` sırası: `docs/PROGRAMS.md` ve CONCEPTS ↔ ilk `qp-lang` kodu | Belgeler önce |
| 8 | `cargo test -p qp-lang` : hata kodu başına sınama; hepsi birden dönen örnek | Geçiyor |
| 9 | Belirlenimcilik: iki koşu, toplu = akışkan, özet aarch64 = x86_64 = wasm32 | Eşit |
| 10 | Önek kararlılığı özellik testi; kesin bar testi | Geçiyor |
| 11 | `add_study` eşdeğerliği (5 hesap) | Bit eşit |
| 12 | Üst periyot hizası (XIST günlük, 4 saatlik; 7/24 UTC) | Sınama geçiyor |
| 13 | Metin ↔ ara gösterim gidiş-dönüş; hata konumu | Geçiyor |
| 14 | Bütçe: sınırı aşan program çalışmadan reddediliyor; çalışma sırasında aşan duruyor; kişi başı toplam | Sınamalar geçiyor; kötü niyet raporu |
| 15 | Uygunluk dökümleri (ekle / denetle / dene / seri çizimi / olay / öneri olarak ekleme / yeniden oynatma) üç taşımada | Geçiyor; yeniden oynatma bayt eşit |
| 16 | `qp program check` ve MCP `check_program` aynı girdide | Aynı sonuç |
| 17 | RX4 uygunluk tablosu | Her fark açıklanmış |
| 18 | Ajan teyidi sonuçları | Raporda; RX4 ile yan yana |
| 19 | K17, K52, tik başına program maliyeti | Bütçe içinde |
| 20 | `git worktree list`; `du -sh target`; `git ls-files Prompt.txt`; belgeler | Yalnız ana ağaç; ≤ 6 GB; `Prompt.txt` izlenmiyor; `AUDIT.md`, `REPORT.md`, `STATUS.md`, `ROADMAP.md` güncel |
