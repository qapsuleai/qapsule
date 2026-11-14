# RD1 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Yeni bir Claude Code oturumunda. Başlatmadan önce:

1. Alt ajan çağrılarını engelleyen eklentiyi (R1 ve RP1 raporlarında "context-mode")
   bu proje için kapat. Bu round'un ana ölçümü bağımsız alt ajanlara dayanıyor.
2. `scripts/preflight.sh` çalıştır; "preflight: TAMAM" görmeden başlatma.

---

Bu depoda Qapsule projesinin RD1 tasarım round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`,
`docs/ARCHITECTURE.md`, `DECISIONS.md` (özellikle K13, K18, K20, K21), `STATUS.md`,
`rounds/R1/REPORT.md`, `rounds/RP1/REPORT.md` ve `rounds/RD1/` içindeki `GOAL.md`,
`TASKS.md`, `VERIFY.md`.

Bu bir tasarım ve araştırma görevi. Amaç motor, arayüz ve ajanlar arasındaki alan
protokolünü tasarlamak, küçük bir referans uygulamayla kanıtlamak ve tasarımı ölçümle
sınamak. Bu protokol projenin kalıcı çekirdeğidir; sonraki her round ona yaslanacak.

Nasıl çalışacaksın:
- İlk iş: çalışma ağacında bekleyen doküman değişikliklerini tek bir commit olarak kaydet.
- Hemen ardından küçük bir deneme alt ajanı başlat. Başlatılamıyorsa dur ve bunu bana
  bildir; bu round alt ajanlar olmadan anlamlı biçimde tamamlanamaz.
- `TASKS.md` sırasını izle: önce araştır, sonra yirmi görevi yaz, sonra tasarla, sonra
  kur, sonra ölç, sonra tasarımı düzelt.
- Var olan protokolleri güncel belgelerinden incele; bellekten yazma. Sonra onlarla
  sınırlı kalma: ihtiyaç karşılanmıyorsa yenisini tasarla.
- K21'deki "sınanacak" kararları savunmaya çalışma; alternatifleri en iyi halleriyle kur
  ve ölç. Kanıt çalışma kararının aksini gösteriyorsa kararı kendin değiştirme; bulguyu
  ve önerini raporla.
- Ajan yazabilirlik ölçümünde alt ajanlara yalnızca şartnameyi ve şemayı ver. Sonuçları
  olduğu gibi raporla; düşük çıkarsa bu, tasarımı düzeltmek için en değerli veridir.
- Ölç, varsayma. Kırık testle birleştirme yapma. Altın görselleri güncelleme.
- `qp-compute` ve `qp-render` davranışını değiştirme.
- Bitince bağımsız bir denetçi alt ajan başlat.
- `../Docs` ve `../Services` salt okunur. `.env` dosyalarını okuma.
- Kapsam dışı: görsel gramerin tasarımı, etkileşimin uygulanması, GPU, anlam işaretleri,
  kapsül davranışı, canlı veri, ağ sunucusu.

Bitiş koşulu: `rounds/RD1/GOAL.md` içindeki on üç ölçüt kanıtlarıyla doldurulmuş;
`RESEARCH.md`, `R2_INPUT.md`, `REPORT.md`, `AUDIT.md` yazılmış; `STATUS.md` ve
`ROADMAP.md` güncellenmiş. Başka round başlatma.
