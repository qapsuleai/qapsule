# RI1 — Claude Code Goal Metni

Çalışma dizini `qapsule/`. Yeni bir Claude Code oturumunda; oturuma `quantex` deposuna
**okuma** erişimi ver:

```sh
cd /Users/arkns/Works/Mchart/qapsule
scripts/preflight.sh
claude --add-dir /Users/arkns/LocalGit/quantex
```

(Açık oturumda `/add-dir /Users/arkns/LocalGit/quantex` de olur.) "preflight: TAMAM"
görmeden başlatma.

---

Bu depoda Qapsule projesinin RI1 round'unu yürüteceksin: bir keşif. Kod yazmayacaksın;
başka bir depodaki çalışan kodu okuyup belgeleyeceksin.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`, `docs/ARCHITECTURE.md`,
`docs/CAPSULE.md`, `docs/DATA_ACCESS.md`, `DECISIONS.md` (özellikle K3, K5, K13, K14, K23,
K29, K30, K33, K34, K38, K42, K44, K48, K59), `STATUS.md` ve `rounds/RI1/` içindeki
`GOAL.md`, `TASKS.md`, `VERIFY.md`.

İncelenecek depo: `/Users/arkns/LocalGit/quantex/services/` (klasör listesi GOAL'de).
Orada Matriks'e nasıl bağlanıldığı (belirteç, MQTT, protolar), verinin içeride NATS ile
nasıl yayınlandığı ve gerçek zamanlı gösterge ile anomali servislerinin ne yaptığı yazılı.
Qapsule'ü gerçek veriye bağlayacak R6 round'u senin belgelerinle tasarlanacak.

Kesin sınırlar (her alt ajana aynen ilet):
- `quantex` deposu salt okunur. Oraya yazma; orada derleme, test, betik çalıştırma. Yalnız
  oku ve ara. Başlamadan önce ve bitince o depoda `git status --porcelain` çıktısını kaydet;
  ikisi aynı olmalı.
- Canlı bağlantı kurma: Matriks'e, MQTT'ye, NATS'a, iç servislere ağ isteği yok; belirteç
  isteme. Aynı kimlikle ikinci bağlantı çalışan sistemin bağlantısını düşürebilir.
- `.env`, anahtar, sertifika, belirteç, parola dosyalarını açma. Kodda gömülü gizli değer
  görürsen değerini hiçbir yere yazma; yalnız yerini bildir.
- Qapsule deposuna `quantex`'ten dosya, proto ya da uzun kod kopyalama. Kendi cümlelerinle
  anlat; kanıt olarak dosya yolu ve satır numarası ver. Tam iç adresleri (IP, sunucu adı)
  yazma. Gerçek piyasa verisi taşıma.
- Qapsule koduna, testlerine ve altınlarına dokunma.

Nasıl çalışacaksın:
- İlk iş: ana ağaçta bekleyen doküman değişikliklerini `CLAUDE.md`'deki "Ana akış
  değişiklikleri" kuralına göre kaydet. Küçük bir deneme alt ajanı başlat; başlatılamıyorsa
  dur ve bana bildir. `quantex` klasörünü okuyabildiğini doğrula; okuyamıyorsan dur.
- Önce deponun haritasını çıkar; sonra konuları alt ajanlara ver (bağlantı, mesajlar, NATS,
  üç gösterge servisi, anomali). Her biri yalnız kendi belgesine yazsın.
- Her iddiayı kaynağına bağla. Kodda görmediğin şeyi yazma; "bilinmiyor" de ve kime
  sorulacağını yaz. Arkin'in GOAL'de yazılı beş bilgisini kodla karşılaştır.
- Göstergelerin sayısal tanımlarını Qapsule'ün `qp-compute` tanımlarıyla formül düzeyinde
  karşılaştır. Anomali türlerini Qapsule'ün altı anlam türüne ve dedektörlerine eşle.
- Sonda bütünleştir: Qapsule kavramlarına eşleme, R6 için seçenekler (artı, eksi, risk,
  çalışan sistemi etkilemeden deneme yolu) ve R6 kapsam taslağı. Karar verme; gerekçeli öner.
- Bitince bağımsız bir denetçi alt ajan başlat: en az 40 iddiayı kaynağında doğrulasın,
  sınırları (gizli değer, iç adres, kopya, veri) tarasın.
- `../Docs` ve `../Services` salt okunur. `.env.local`'ı açma.

Bitiş koşulu: `rounds/RI1/GOAL.md` içindeki dokuz ölçüt kanıtlarıyla doldurulmuş;
`docs/matriks/` altında altı belge, `rounds/RI1/REPORT.md` ve `AUDIT.md` yazılmış;
`STATUS.md` ve `ROADMAP.md` güncellenmiş. Raporun sonuna "Arkin'e ve Matriks ekiplerine
sorular" bölümü ekle. Başka round başlatma.
