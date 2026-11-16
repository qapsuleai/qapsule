# R4 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Yeni bir Claude Code oturumunda. Başlatmadan önce:

1. `fixtures/local/symbols.txt` dosyasını güncelle (yazım hatasını düzelt, sembol ekle).
   `.env.local` içindeki `MATRIKS_TOKEN` geçerli olsun.
2. `scripts/preflight.sh` çalıştır; "preflight: TAMAM" görmeden başlatma.

---

Bu depoda Qapsule projesinin R4 round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CAPSULE.md`, `docs/CONCEPTS.md`,
`docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`, `docs/GRAMMAR.md`, `docs/DATA_ACCESS.md`,
`DECISIONS.md` (özellikle K17, K18, K39, K41–K45), `STATUS.md`, `rounds/R3/REPORT.md`,
`rounds/R3/AUDIT.md`, `rounds/R3/ergonomics/RESULTS.md` ve `rounds/R4/` içindeki
`GOAL.md`, `TASKS.md`, `VERIFY.md`.

Bu round'un sonunda ilk kez elle kullanılabilen bir şey olacak: telefon boyutunda bir
sayfa, gerçek geçmiş verinin yeniden oynatılan akışı, doğup evrilen kapsüller ve
dokunarak cümleden kanıta inme. R3'ün kapsülü doğruydu ama rapor gibi okunuyordu; bu
round onu tek bakışta anlaşılır hale getirir.

Nasıl çalışacaksın:
- İlk iş: ana ağaçta bekleyen doküman değişikliklerini `CLAUDE.md`'deki "Ana akış
  değişiklikleri" kuralına göre kaydet. Ardından küçük bir deneme alt ajanı başlat;
  başlatılamıyorsa dur ve bana bildir.
- `fixtures/local/symbols.txt` değiştiyse eksik veriyi `scripts/fetch-bars.sh` ile çek.
  `.env.local`'ı açma, belirteci yazdırma. Veriyi, türevlerini ve ondan alınan ekran
  görüntülerini asla commit etme.
- `TASKS.md`'deki iki evreyi izle. Yeni kavramları koddan önce sözlüğe yaz.
- Kabuk ince kalacak: alan mantığı çekirdekte, tasarım gramerde. Bir tasarım yalnızca
  kabukta özel kodla mümkünse grameri genişlet ya da tasarımı ele.
- M8–M11 için: önce düzenek ve puanlama ölçütü; sonra alternatifler ayrı alt ajanlara,
  ayrı dallarda, birbirini görmeden; persona ve eleştirmen ajanları tasarımın adını ve
  yapanını bilmeden, yalnızca ekran görüntüsünden puanlasın.
- Emin olmadığın başka bir tasarım noktası çıkarsa aynı yolu izle: alternatif kur, ölç.
- Bilinen kalıplarla sınırlı değilsin; "tek bakışta anlam" hedefinden tasarla.
- Sınırı dürüst yaz: persona ajanı insan değildir; son söz insan testindedir.
- Ölç, varsayma. Kırık testle birleştirme. Var olan altın görselleri güncelleme; K42
  gereği değişen görsel ve dökümleri gerekçesiyle listele.
- Mimari kararları kendin kesinleştirme; `DECISIONS.md`'ye "Önerilen (R4)" bölümü yaz.
- Bitince bağımsız bir denetçi alt ajan başlat; denetçi gösterimi kendisi kullansın.
- `../Docs` ve `../Services` salt okunur.
- Kapsam dışı: gerçek MQTT bağlantısı, yerel mobil kabuk, GPU çizimi, profesyonel
  çalışma alanı, dil modeliyle sohbet, sunucuya dağıtım.

Bitiş koşulu: `rounds/R4/GOAL.md` içindeki on beş ölçüt kanıtlarıyla doldurulmuş;
`scripts/demo.sh` çalışıyor; `REPORT.md`, `AUDIT.md`, `docs/FACE.md`, `DEVICE.md` ve
insan testi v2 paketi yazılmış; `STATUS.md` ve `ROADMAP.md` güncellenmiş. Raporun
sonuna Arkin için "gösterimi nasıl açarım ve telefonumda nasıl ölçerim" bölümü ekle.
Başka round başlatma.
