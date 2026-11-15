# RP2 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Yeni bir Claude Code oturumunda. Başlatmadan önce
`scripts/preflight.sh` çalıştır; "preflight: TAMAM" görmeden başlatma.

---

Bu depoda Qapsule projesinin RP2 round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`,
`docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`, `docs/GRAMMAR.md`,
`docs/MATRIKS_PROTOCOL.md`, `DECISIONS.md` (özellikle K13, K18, K20–K26), `STATUS.md`,
`rounds/RD1/experiments/T6d-handover.md`, `rounds/RP1/REPORT.md`, `rounds/R2/REPORT.md`
ve `rounds/RP2/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

Bu round'un iki yüzü var. Mühendislik yüzü: oturumun artımlı canlı yolu ve veri bağı.
Araştırma yüzü: veri protokolün dışından aktığında otorite ile yansının aynı veriye
baktığından nasıl emin olunur.

Nasıl çalışacaksın:
- İlk iş: çalışma ağacında bekleyen doküman değişikliklerini tek bir commit olarak kaydet.
- Hemen ardından küçük bir deneme alt ajanı başlat. Başlatılamıyorsa dur ve bana bildir.
- Önce ölç (T1), sonra tasarla. Yeni kavramları koddan önce sözlüğe yaz.
- Tutarlılık için senaryo düzeneğini yaklaşımlardan önce ve onlardan bağımsız yaz; üç
  yaklaşımı ayrı alt ajanlara, ayrı dallarda ver; birbirlerini görmesinler.
- Değişmezler: artımlı sonuç baştan hesapla bayt düzeyinde aynı; hesap özeti değişmez;
  günlükten yeniden oynatma aynı dokümanı üretir; var olan altın görseller güncellenmez.
- Bilinen yöntemlerle sınırlı değilsin; ihtiyaç karşılanmıyorsa yenisini tasarla.
- Davranışı K23 ya da K25 gereği değişen dökümleri güncelleyebilirsin; her birini raporda
  gerekçesiyle listele. Başka döküm değiştirme.
- Ölç, varsayma. Hedefe ulaşamazsan sayıyı olduğu gibi raporla. Kırık testle birleştirme.
- Mimari kararları kendin kesinleştirme; `DECISIONS.md`'ye "Önerilen (RP2)" bölümü yaz.
- Bitince bağımsız bir denetçi alt ajan başlat.
- `../Docs` ve `../Services` salt okunur. `.env` dosyalarını okuma.
- Kapsam dışı: gerçek Matriks/NATS bağlayıcısı, kapsül ve anlam işaretleri, etkileşim, GPU.

Bitiş koşulu: `rounds/RP2/GOAL.md` içindeki on iki ölçüt kanıtlarıyla doldurulmuş;
`RESEARCH.md`, `REPORT.md`, `AUDIT.md` yazılmış; `STATUS.md` ve `ROADMAP.md`
güncellenmiş. Başka round başlatma.
