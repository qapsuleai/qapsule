# R6 — Claude Code Goal Metni

Önce `.env.local`'a canlı bağlantı değişkenlerini gir (`docs/DATA_ACCESS.md` "Canlı
bağlantı değişkenleri"); `MATRIKS_HARDWARE_ID` için Qapsule'e özgü bir değer ver ve
`MATRIKS_MQTT_MODE=delayed` ile başla. Sonra:

```sh
cd /Users/arkns/Works/Mchart/qapsule
scripts/preflight.sh
claude --add-dir /Users/arkns/LocalGit/quantex
```

"preflight: TAMAM" görmeden başlatma. Canlı ölçümler için borsa açıkken başlatmak iyidir.

---

Bu depoda Qapsule projesinin R6 round'unu yürüteceksin: canlı veri.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`, `docs/ARCHITECTURE.md`,
`docs/DATA_ACCESS.md`, `docs/matriks/` altındaki altı belge (özellikle `CONNECTIVITY.md`,
`MESSAGES.md`, `INTEGRATION.md`), `docs/PROTOCOL.md` §18 ve §21–§23, `DECISIONS.md`
(özellikle K3, K13, K14, K23, K33, K44, K48, K52, K56, K57, K60–K63), `STATUS.md`,
`rounds/RI1/REPORT.md` ve `rounds/R6/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

Qapsule kendi kimliğiyle doğrudan Matriks MQTT'ye bağlanacak. Örnek uygulama
`/Users/arkns/LocalGit/quantex/services/quantex-stream/matriks-adapter` klasöründe (salt
okunur): belirteç alma, uç nokta başına bağlantı, konu kuralları, hangi istek için hangi
proto. Bağlantı mantığını Rust'ta yeniden yaz; kusurlarını devralma (eksik alanı 0 sayma,
belirteç paylaşımı). Proto dosyalarını ise yeniden yazma: Matriks protoları sabittir;
o klasördeki proto dosyalarının tamamını değiştirmeden bu depoya kopyala, köken kaydını
(kaynak yol, commit, tarih, dosya özetleri) yaz ve Rust tiplerini derleme sırasında
onlardan üret. Proto dosyalarına elle dokunma. Bağlantı bilgileri `.env.local`'da; dosyayı açma, değerleri yazdırma; kod
ortamdan okusun. Eksik değişken varsa adını söyle ve bekle.

En önemli kural: çalışan sistemleri düşürme. Aynı kimlikle aynı uç noktaya ikinci bağlantı
birincisini düşürür.
- Uç nokta başına tek bağlantı; süreç içinde ve makinede kilitle zorla.
- Canlı bağlantıyı yalnız sen (orkestratör) aç, sırayla. Alt ajanlar canlı bağlantı
  açmaz; kayıtlı iletiler ve sahte sunucuyla çalışır.
- "Çoklu bağlantı" bildirimi gelirse dur, yeniden deneme, bana bildir.
- Kimlik Qapsule'e özgü; `quantex` deposundan kimlik, belirteç ya da ortam dosyası
  kullanma, okuma. Orada hiçbir şey çalıştırma. NATS'a ve Quantex servislerine bağlanma.
- Önce gecikmeli kip ve 3–5 sembol; her şey çalışınca evreni adım adım büyüt. Anlık kip
  için benden kipi değiştirmemi iste; `.env.local`'ı sen değiştirme.
- Her canlı denemeyi `rounds/R6/experiments/LIVE-LOG.md`'ye yaz.

Nasıl çalışacaksın:
- İlk iş: ana ağaçta bekleyen doküman değişikliklerini `CLAUDE.md`'deki "Ana akış
  değişiklikleri" kuralına göre kaydet. Küçük bir deneme alt ajanı başlat; başlatılamıyorsa
  dur ve bana bildir.
- Önce ağsız kısım: çözücü (gelmeyen alan "değişmedi" demektir, 0 değil), bağlantı
  çekirdeği, sahte sunucuyla testler, günlük maskeleme. Sonra ilk canlı deneme ve ham kayıt.
  Geri kalan geliştirme kayıt üzerinde.
- Canlı barı borsa takvimine göre kendin üret; REST barlarıyla karşılaştır; farkları ölç
  ve açıkla. Hacim birimini ve düzeltmeyi varsayma.
- Kopmada veriyi bayat işaretle, yeniden bağlanınca eksik aralığı yeniden çöz; yanlış ya
  da erken kapsül 0 olmalı.
- Bağlayıcı bağımlılıkları WASM paketine girmesin; hesap çekirdeği ağ ve saat bilmesin;
  hesap özeti, altınlar ve dökümler değişmesin.
- Gerçek veri, ham kayıt, belirteç ve uç nokta adresi depoya, günlüğe, belgeye girmesin.
  Rapora yalnız toplu sayı.
- Ölç, varsayma. Seans kapalıysa canlı ölçütleri uydurma; açık bırak ve bana hangi komutu
  ne zaman çalıştıracağımı yaz. Kırık testle birleştirme.
- Disk: `target/wt/<görev>`; elenen dalın worktree'sini kaldır; sonda `scripts/clean.sh`.
- Bitince bağımsız bir denetçi alt ajan başlat (canlı bağlantı açmaz).
- `../Docs` ve `../Services` salt okunur.

Bitiş koşulu: `rounds/R6/GOAL.md` içindeki on dört ölçüt kanıtlarıyla doldurulmuş;
`REPORT.md`, `AUDIT.md`, `docs/matriks/LIVE.md` yazılmış; `STATUS.md` ve `ROADMAP.md`
güncellenmiş. Raporun sonuna Arkin için "canlı gösterimi nasıl açarım" bölümü ekle.
Başka round başlatma.
