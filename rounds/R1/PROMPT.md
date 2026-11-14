# R1 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Başlatmadan önce terminalde `scripts/preflight.sh` çalıştır;
"preflight: TAMAM" görmeden başlatma. Sonra aşağıdaki metni ver.

---

Bu depoda Qapsule projesinin R1 round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`,
`docs/ARCHITECTURE.md`, `DECISIONS.md` (özellikle K12–K15), `STATUS.md`,
`rounds/R0/REPORT.md`, `docs/LEGACY.md` ve `rounds/R1/` içindeki `GOAL.md`,
`TASKS.md`, `VERIFY.md`.

Uzak depo: `git@github-qapsule:qapsuleai/qapsule.git` (GitHub: https://github.com/qapsuleai/qapsule).
`github-qapsule`, bu makinedeki `~/.ssh/config` içinde tanımlı bir takma addır; adresi değiştirme.

Hedef: `rounds/R1/GOAL.md` içindeki on kabul ölçütünü karşılamak.

Nasıl çalışacaksın:
- Sen orkestratörsün. `TASKS.md` içindeki sırayı ve yazma alanlarını uygula; her alt
  görev kendi worktree'sinde, kendi alt ajanıyla.
- İlk iş: çalışma ağacında bekleyen doküman ve betik değişikliklerini (R1 dosyaları,
  `scripts/qgh`, `scripts/preflight.sh`, `CLAUDE.md` vb.) tek bir commit olarak kaydet.
- `CLAUDE.md` içindeki "GitHub hesabı" kurallarına uy: `gh` yerine `scripts/qgh`, her
  gönderim ve CI işleminden önce `scripts/preflight.sh`. `origin` zaten tanımlı.
- T0'da yalnızca `origin`'e gönder. Depo oluşturma, görünürlük değiştirme ya da başka
  bir hesaba gönderme yapma. `preflight` başarısızsa T0'ı atla, nedenini raporla ve
  round'un geri kalanına devam et.
- Ölçüm gerektiren yerlerde (tarama hızı, takvim kütüphanesi, WASM boyutu) varsayma; ölç.
- Hedef sürelere ulaşamazsan sayıları olduğu gibi raporla; testi gevşetme.
- Altın görselleri kendiliğinden güncelleme.
- Bitince T8 için bağımsız bir denetçi alt ajan başlat.
- `../Docs` ve `../Services` salt okunur. `.env` dosyalarını okuma.
- Kapsam dışına çıkma: görsel gramer, GPU, olgu dedektörleri, canlı veri sonraki round'lar.

Bitiş koşulu: `rounds/R1/REPORT.md` ve `AUDIT.md` yazılmış; `STATUS.md` ve `ROADMAP.md`
güncellenmiş; kabul ölçütleri tablosu kanıtlarıyla doldurulmuş. R2'yi başlatma.
