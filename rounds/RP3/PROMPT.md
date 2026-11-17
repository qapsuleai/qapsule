# RP3 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Yeni bir Claude Code oturumunda. Başlatmadan önce
`scripts/preflight.sh` çalıştır; "preflight: TAMAM" görmeden başlatma.

---

Bu depoda Qapsule projesinin RP3 round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/ARCHITECTURE.md`,
`docs/GRAMMAR.md`, `DECISIONS.md` (özellikle K12, K17, K18, K22, K24, K29, K39, K45–K49),
`STATUS.md`, `rounds/R4/REPORT.md`, `rounds/R4/DEVICE.md`,
`rounds/R4/experiments/D/phone-iphone-ios27-default.json`,
`rounds/R4/experiments/D/desktop-mac-chrome156-default.json` ve `rounds/RP3/` içindeki
`GOAL.md`, `TASKS.md`, `VERIFY.md`.

Arkin gösterimi kendi telefonunda ölçtü: güncel üst sınıf bir iPhone'da masaüstünün
yaklaşık 30 katı yavaş ve akıcı değil. R4'ün benzetimi bunu yaklaşık 3 kat hafif
göstermişti. Bu round iki şeyi düzeltir: telefonda çizimi ve benzetime olan güveni.

En önemli karar K48: cihaz görüntüleyicidir. Tespit ve kapsül sunucuda; cihaz yalnızca
kullanıcının baktığı şey için veri alır, hesaplar ve çizer; cihazın işi evrendeki sembol
sayısından bağımsızdır. Bu round'un birinci işi gösterimi bu iş bölümüne geçirmek,
ikincisi boyama yolunu seçmektir.

Nasıl çalışacaksın:
- İlk iş: ana ağaçta bekleyen doküman değişikliklerini `CLAUDE.md`'deki "Ana akış
  değişiklikleri" kuralına göre kaydet. Ardından küçük bir deneme alt ajanı başlat;
  başlatılamıyorsa dur ve bana bildir.
- Önce kalibrasyon: benzetimi Arkin'in ölçümünü yeniden üretecek hale getir ve nedenleri
  ayrıştır. Bu bitmeden iyileştirme ölçme.
- Ölçüm sınamasını ve boyama yolu arayüzünü alternatiflerden önce yaz.
- Boyama yollarını ayrı alt ajanlara, ayrı dallarda ver; birbirlerini görmesinler; her
  biri en iyi haliyle kurulsun. Seçimi ölçümle yap. İşe yaramayan denemeleri silme,
  `RESEARCH.md`'ye kaydet.
- Bilinen yöntemlerle sınırlı değilsin; karma ve yeni yaklaşımlar serbest. Değişmezler:
  sahne tarifi tek doğruluk kaynağı, referans çizici ve anlamsal okuma değişmez, kabukta
  alan mantığı yok.
- Tarayıcı ve mobil WebKit davranışı için güncel belgelere bak; bellekten varsayma.
- Gerçek telefonda ölçemezsin. Bu yüzden seçimi kesinleştirme: `DECISIONS.md`'ye
  "Önerilen (RP3)" olarak yaz ve Arkin'in tek dokunuşla çalıştıracağı otomatik
  karşılaştırmayı hazırla.
- Görünen tasarımı değiştirme; yalnızca nasıl boyandığını.
- Ölç, varsayma. Hedef tutmuyorsa sayıyı olduğu gibi raporla. Kırık testle birleştirme.
  Var olan altın görselleri güncelleme.
- Bitince bağımsız bir denetçi alt ajan başlat.
- `../Docs` ve `../Services` salt okunur. `.env.local`'ı açma.

Bitiş koşulu: `rounds/RP3/GOAL.md` içindeki on üç ölçüt (0–12) kanıtlarıyla doldurulmuş;
`REPORT.md`, `RESEARCH.md`, `CALIBRATION.md`, `AUDIT.md` yazılmış; `STATUS.md` ve
`ROADMAP.md` güncellenmiş. Raporun sonuna Arkin için "telefonumda karşılaştırmayı nasıl
çalıştırırım" bölümü ekle. Başka round başlatma.
