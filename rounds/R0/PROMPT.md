# R0 — Claude Code Goal Metni

Aşağıdaki metni Claude Code'a, çalışma dizini `qapsule/` olacak şekilde ver.

---

Bu depoda Qapsule projesinin R0 round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`,
`docs/ARCHITECTURE.md`, `DECISIONS.md`, `STATUS.md`, `rounds/README.md` ve
`rounds/R0/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

Hedef: `rounds/R0/GOAL.md` içindeki sekiz kabul ölçütünün tamamını karşılamak.

Nasıl çalışacaksın:
- Sen orkestratörsün. `TASKS.md` içindeki alt görevleri alt ajanlara ver. T1'i önce
  bitir; T2, T3 ve T4'ü paralel yürüt; sonra T5 ve T6.
- Her alt ajana yalnızca kendi yazma alanını, ilgili kabul ölçütlerini ve
  `CLAUDE.md` kurallarını ver. Alt ajanlar birbirinin alanına yazmaz.
- T3 bir denemedir: çizim yığını seçimini varsayımla değil ölçümle yap ve sonucu
  `DECISIONS.md` içinde A1 satırına kanıtıyla yaz.
- İş bitince T7 için ayrı bir denetçi alt ajan başlat. Denetçi, işi yapan ajanların
  bağlamını görmeden `VERIFY.md` adımlarını temiz bir kopyada çalıştırır.
- `../Docs` ve `../Services` salt okunurdur. `.env` dosyalarını okuma.
- Kapsam dışına çıkma. Hesap çekirdeği, görsel gramer ve kapsül davranışı sonraki
  round'ların işidir.

Bitiş koşulu:
- `scripts/verify.sh` yeşil ve `VERIFY.md` içindeki 13 adım geçmiş, ya da geçemeyenler
  nedenleriyle raporlanmış.
- `rounds/R0/REPORT.md` yazılmış: yapılanlar, ölçümler, kabul ölçütleri tablosu,
  sapmalar, açık sorular, R1 için öneriler.
- `STATUS.md` ve `ROADMAP.md` güncellenmiş.

R1'i başlatma. Raporu yazıp dur.
