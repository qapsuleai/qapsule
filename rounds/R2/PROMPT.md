> **BEKLEMEDE.** RD1 sonrası yeniden yazılacak. Bu metni Claude Code'a verme.

# R2 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Başlatmadan önce:
1. `scripts/preflight.sh` çalıştır; "preflight: TAMAM" görmeden başlatma.
2. R1'de alt ajan çağrılarını engelleyen eklentiyi (raporda "context-mode") kapat ya da düzelt.

---

Bu depoda Qapsule projesinin R2 round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`,
`docs/ARCHITECTURE.md`, `DECISIONS.md` (özellikle K12–K18), `STATUS.md`,
`rounds/R1/REPORT.md`, `rounds/R1/AUDIT.md`, `docs/LEGACY.md` ve `rounds/R2/` içindeki
`GOAL.md`, `TASKS.md`, `VERIFY.md`.

Hedef: `rounds/R2/GOAL.md` içindeki on bir kabul ölçütünü karşılamak.

Nasıl çalışacaksın:
- İlk iş: çalışma ağacında bekleyen doküman değişikliklerini tek bir commit olarak kaydet.
- Sen orkestratörsün; `TASKS.md` içindeki sırayı ve yazma alanlarını uygula. Alt ajan
  başlatılamıyorsa bunu hemen `REPORT.md`'nin başına yaz.
- T1'e orantısız zaman ayır. Gramer bu projenin kalıcı çekirdeğidir; aceleyle
  tasarlanmış bir gramer sonraki her round'u pahalılaştırır. Altı görünümün JSON'unu
  önce yazıp grameri onlarla sına.
- `NORTH_STAR.md` içindeki 6. ve 8. ilkeler bu round'un ölçüsüdür: her görsel öğe bir
  olguya bağlı, ve sabit grafik tipleri yerine gramer.
- Ölç, varsayma. Hedefe ulaşamazsan sayıyı olduğu gibi raporla; testi gevşetme.
- Var olan altın görselleri güncelleme. Kırık testle birleştirme yapma.
- Bitince T8 için bağımsız bir denetçi alt ajan başlat; denetçi görselleri gözle incelesin.
- `../Docs` ve `../Services` salt okunur. `.env` dosyalarını okuma.
- Kapsam dışı: etkileşim, kapsül, anlam işareti tasarımı, çizim araçları, canlı veri,
  hesap performansı (RP1).

Bitiş koşulu: `rounds/R2/REPORT.md` ve `AUDIT.md` yazılmış; `STATUS.md` ve `ROADMAP.md`
güncellenmiş; kabul ölçütleri tablosu kanıtlarıyla doldurulmuş. Sonraki round'u başlatma.
