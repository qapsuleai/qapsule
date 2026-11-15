# R3 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Yeni bir Claude Code oturumunda. Başlatmadan önce
`scripts/preflight.sh` çalıştır; "preflight: TAMAM" görmeden başlatma.

Gerçek veriyle ölçüm için (önerilir):
1. Depo kökünde `.env.local` dosyası oluştur ve içine `MATRIKS_TOKEN=<belirteç>` yaz.
2. `fixtures/local/symbols.txt` dosyasına satır başına bir sembol yaz (ör. BIST 30).
Bu iki dosya depoya girmez. Yapmazsan round yalnızca sentetik veriyle koşar ve bunu
raporda belirtir.

---

Bu depoda Qapsule projesinin R3 round'unu yürüteceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CAPSULE.md`, `docs/CONCEPTS.md`,
`docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`, `docs/GRAMMAR.md`, `docs/DATA_ACCESS.md`,
`docs/MODELS.md`, `DECISIONS.md` (özellikle
K18, K22, K23, K26–K39), `STATUS.md`, `rounds/RP2/REPORT.md`, `rounds/RP2/AUDIT.md`,
`rounds/R2/REPORT.md` ve `rounds/R3/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

Bu round'da kapsül doğuyor: deterministik motorun tespit ettiği, kişiye haber veren,
zamanla evrilen, sonuçlanan ve karnesini taşıyan en küçük birim. `docs/CAPSULE.md`
ana akışta kararlaştırılan çerçevedir; onu değiştirme, ayrıntılandır.

Nasıl çalışacaksın:
- İlk iş: ana ağaçta bekleyen doküman değişikliklerini `CLAUDE.md`'deki "Ana akış
  değişiklikleri" kuralına göre kaydet. Ardından küçük bir deneme alt ajanı başlat;
  başlatılamıyorsa dur ve bana bildir.
- Gerçek veri: `docs/DATA_ACCESS.md`'yi izle. `scripts/fetch-bars.sh`'ı yaz; `MATRIKS_TOKEN`
  ortamda ve `fixtures/local/symbols.txt` varsa günlük barları (2018'den bugüne) ve 60
  dakikalık barları (son iki yıl) çek, ölçümleri onlarla da koş. `.env.local`'ı açma,
  belirteci yazdırma. Veri yoksa ya da çekim başarısızsa sentetikle devam et ve raporla.
  Veriyi, türevlerini ve ondan üretilen görselleri asla commit etme.
- `TASKS.md`'deki iki evreyi izle. Yeni kavramları koddan önce sözlüğe yaz.
- M1–M7'nin her biri için: önce düzeneği, alternatifleri görmeden yaz; sonra
  alternatifleri ayrı alt ajanlara, ayrı dallarda ver; birbirlerini görmesinler; her
  alternatif en iyi haliyle kurulsun. Seçimi ölçümle yap.
- Emin olmadığın başka bir tasarım noktası çıkarsa aynı yolu izle: alternatif kur, ölç.
  Ölçemeyeceğin bir şeyi varsayımla kapatma; açık karar olarak yaz.
- Ürünün içinde dil modeli yok: kapsül, cümlesi ve işareti deterministik motordan çıkar.
- Bilinen yöntemlerle sınırlı değilsin; özellikle işaret alfabesinde ve sunucu olgusu
  güven yolunda yeni bir şey tasarlaman bekleniyor.
- Sınırları dürüst yaz: görsel okuyan alt ajan insan gözü değildir; sentetik veride karne
  başarı kanıtı değildir.
- Ölç, varsayma. Kırık testle birleştirme. Var olan altın görselleri güncelleme.
- Mimari kararları kendin kesinleştirme; `DECISIONS.md`'ye "Önerilen (R3)" bölümü yaz.
- Bitince bağımsız bir denetçi alt ajan başlat.
- `../Docs` ve `../Services` salt okunur. `.env` dosyalarını okuma.
- Kapsam dışı: gerçek Matriks/NATS bağlayıcısı, etkileşim, GPU, mobil kabuk, yorumlayan
  ajanların çalışma ortamı, tam formasyon kütüphanesi.

Bitiş koşulu: `rounds/R3/GOAL.md` içindeki on sekiz ölçüt kanıtlarıyla doldurulmuş;
`REPORT.md`, `AUDIT.md`, deney kayıtları ve insan testi paketi yazılmış; `STATUS.md` ve
`ROADMAP.md` güncellenmiş. Başka round başlatma.
