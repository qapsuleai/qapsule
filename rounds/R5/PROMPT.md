# R5 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Yeni bir Claude Code oturumunda. Başlatmadan önce
`scripts/preflight.sh` çalıştır; "preflight: TAMAM" görmeden başlatma.

---

Bu depoda Qapsule projesinin R5 round'unu yürüteceksin: dil ve tasarım.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`, `docs/CAPSULE.md`,
`docs/FACE.md`, `docs/GRAMMAR.md`, `DECISIONS.md` (özellikle K7, K18, K33, K34, K39,
K42, K43, K46, K48–K56 ve açık A21, A30–A33), `STATUS.md`, `rounds/R4/REPORT.md`,
`rounds/R4/experiments/M8` ile `M10` sonuçları (düzeneğin zayıflıkları için),
`rounds/RP3/REPORT.md` ve `rounds/R5/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

Gösterim çalışıyor ve telefonda akıcı. Şimdi iyi olması gerekiyor: dil somut olmalı
("önemli bir eşik" değil; hangi eşik, kaç), görsel hiyerarşi tek bakışta okunmalı,
grafik enstrümanın bütün kapsüllerinin hikâyesini göstermeli, akış güvenilir olanı öne
koymalı. Anlaşılabilirlik bu ürünün asıl farkı. 2026–2030 için tasarlıyoruz; bugünün
uygulamalarını kopyalamak yetmez.

Nasıl çalışacaksın:
- İlk iş: ana ağaçta bekleyen doküman değişikliklerini `CLAUDE.md`'deki "Ana akış
  değişiklikleri" kuralına göre kaydet. Sonra T0: `scripts/clean.sh`, ortak
  `CARGO_TARGET_DIR`, eski worktree'lerin temizliği. Ardından küçük bir deneme alt ajanı
  başlat; başlatılamıyorsa dur ve bana bildir.
- Önce araştır: güncel kaynaklardan; bellekten varsayma. Kanıtı olan fikirleri topla.
- Ölçüm düzeneğini ve puanlama kuralını alternatiflerden önce yaz ve commit et. R4'ün
  hatasını tekrarlama: okuyucu ajan görüntüyü büyütüp uzun uzun okuyamamalı. Düzeneği
  bilinçli bozulmuş bir tasarımla sına; onu sona koyamıyorsa düzeneği düzelt.
- Alternatifleri ayrı alt ajanlara, ayrı dallarda ver; birbirlerini görmesinler; her
  biri en iyi haliyle kurulsun. Görsel yönlerden biri bilinçli olarak alışılmışın dışında
  olsun ve bugünkü tasarımı hiç görmesin. Bugünkü tasarım her ölçümde başvuru olarak yarışsın.
- Dili görsel dilden önce seç; görsel yönler aynı metinle yarışsın.
- Okuyucu ve eleştirmen ajanlar kör olsun. Fark güven aralığının içindeyse "ayırt
  edilemedi" yaz; kazanan uydurma. Her konuda ilk iki adayı gösterimde seçilebilir bırak.
- Bilinen yöntemlerle sınırlı değilsin. Değişmezler: tasarım gramerde, kabuk ince, dil
  deterministik ve gözlem (tavsiye değil), dürüstlük ve lisans kuralı, K52 performans
  bütçesi. Bütçeyi aşan tasarımı seçme.
- Profesyonel çalışma alanı bu round'da yok (R7). Yalnızca seçilen dilin yoğun görünümde
  okunup okunmadığını ölç.
- Ajan insan değildir; bunu raporda açıkça söyle. Seçimleri kesinleştirme:
  `DECISIONS.md`'ye "Önerilen (R5)" olarak yaz. Arkin telefonda galeriden karşılaştıracak.
- Ölç, varsayma. Kırık testle birleştirme. `v1` altınları silme; `v2` ekle.
- Disk: ortak derleme klasörü; elenen alternatifin worktree'sini ölçüm bitince kaldır;
  round sonunda `scripts/clean.sh` ve kalan boyut.
- Bitince bağımsız bir denetçi alt ajan başlat.
- `../Docs` ve `../Services` salt okunur. `.env.local`'ı açma. Gerçek veriden görsel ya
  da türev commit etme.

Bitiş koşulu: `rounds/R5/GOAL.md` içindeki on beş ölçüt kanıtlarıyla doldurulmuş;
`REPORT.md`, `RESEARCH.md`, `AUDIT.md`, `docs/DESIGN.md`, `docs/LANGUAGE.md` yazılmış;
`STATUS.md` ve `ROADMAP.md` güncellenmiş. Raporun sonuna Arkin için "galeriyi telefonumda
nasıl açarım, neye bakarım" bölümü ekle. Başka round başlatma.
