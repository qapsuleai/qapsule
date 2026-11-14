# R2 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Yeni bir Claude Code oturumunda. Başlatmadan önce
`scripts/preflight.sh` çalıştır; "preflight: TAMAM" görmeden başlatma.

---

Bu depoda Qapsule projesinin R2 round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`,
`docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`, `DECISIONS.md` (özellikle K12, K18, K20–K23),
`STATUS.md`, `rounds/RD1/REPORT.md`, `rounds/RD1/R2_INPUT.md`,
`rounds/RD1/ergonomics/RESULTS.md` ve `rounds/R2/` içindeki `GOAL.md`, `TASKS.md`,
`VERIFY.md`.

Hedef: `rounds/R2/GOAL.md` içindeki on dört kabul ölçütünü karşılamak. Görsel gramer bu
projenin kalıcı çekirdeğidir ve RD1'de tasarlanan protokolün kelime dağarcığıdır.

Nasıl çalışacaksın:
- İlk iş: çalışma ağacında bekleyen doküman değişikliklerini tek bir commit olarak kaydet.
- Hemen ardından küçük bir deneme alt ajanı başlat. Başlatılamıyorsa dur ve bana bildir.
- `TASKS.md` sırasını ve yazma alanlarını uygula. T1'e orantısız zaman ayır: önce altı
  görünümü ve yirmi görevi yaz, grameri onlarla sına, sonra tipleri yaz.
- Görünümleri protokol mesaj dizileriyle kur, elle yazılmış görünüm dosyasıyla değil.
- Anlamla başvuru için iki tasarımı ayrı alt ajanlara, ayrı dallarda ver; birbirlerini
  görmesinler. Hangisinin daha iyi olduğuna ajan yazabilirlik ölçümüyle karar ver.
- Ölçümde alt ajanların yalnızca şartname ve şemayı görmesini dosya erişimiyle sağla;
  kimlikler kopyalanamaz olsun. Sonuçları olduğu gibi raporla.
- Bilinen yöntemlerle sınırlı değilsin; ihtiyaç karşılanmıyorsa yenisini tasarla ve
  gerekçesini yaz. Değişmezler: belirleyicilik, her işaretin kökeni, tek şema kaynağı.
- Ölç, varsayma. Kırık testle birleştirme yapma. Var olan altın görselleri güncelleme.
- Mimari kararları kendin kesinleştirme; `DECISIONS.md`'ye "Önerilen (R2)" bölümü olarak yaz.
- Bitince bağımsız bir denetçi alt ajan başlat; denetçi görselleri gözle incelesin.
- `../Docs` ve `../Services` salt okunur. `.env` dosyalarını okuma.
- Kapsam dışı: etkileşimin uygulanması, GPU, anlam işareti ve kapsül tasarımı, oturumun
  artımlı canlı yolu, veri bağı, canlı veri.

Bitiş koşulu: kabul ölçütleri tablosu kanıtlarıyla doldurulmuş; `REPORT.md` ve `AUDIT.md`
yazılmış; `STATUS.md` ve `ROADMAP.md` güncellenmiş. Başka round başlatma.
