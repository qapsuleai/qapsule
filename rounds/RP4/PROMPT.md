# RP4 — Claude Code Goal Metni

```sh
cd /Users/arkns/Works/Mchart/qapsule
scripts/preflight.sh
claude --add-dir /Users/arkns/LocalGit/quantex
```

"preflight: TAMAM" görmeden başlatma. Hafta sonu başlatılabilir: borsa kapalıyken
geçmiş veriyle çalışır.

---

Bu depoda Qapsule projesinin RP4 round'unu yürüteceksin: gün içi kalibrasyon, işlem
akışı ve sağlamlaştırma.

Önce şunları oku: `CLAUDE.md`, `NORTH_STAR.md`, `docs/CONCEPTS.md`, `docs/CAPSULE.md`,
`docs/DATA_ACCESS.md` (özellikle "İşlem (trade) verisi" ve "Canlı bağlantı değişkenleri"),
`docs/matriks/LIVE.md`, `CONNECTIVITY.md`, `MESSAGES.md`, `DECISIONS.md` (özellikle K3,
K14, K33, K39, K42, K53, K56, K60–K66), `STATUS.md`, `rounds/R6/REPORT.md` (özellikle §4,
§6, §10, §12), `rounds/R6/AUDIT.md`, `rounds/R3/REPORT.md` (çıkış kuralı ve karne ölçümleri)
ve `rounds/RP4/` içindeki `GOAL.md`, `TASKS.md`, `VERIFY.md`.

R6 Qapsule'ü canlı BIST'e bağladı ve üç zayıflık gösterdi: gün içi kapsüller tek barlık
gürültüyle kapanıyor (çıkış kuralı ve eşikler sentetik veriyle ayarlanmıştı), gün içi
barlar fiyat kartından yaklaşık kuruluyor, ve bağlayıcının koruması ile 580 sembol
ölçeği eksik. Bu round bunları gerçek veriyle düzeltir.

Borsa kapalıyken durma. Canlı akış yerine REST geçmişinden (işlem ve bar) hızlandırılmış
benzetim kur: geçmişi canlı yolun aynısından geçir. Canlı bağlantı isteyen doğrulamaları
uydurma; "ilk iş günü" listesine komutuyla yaz.

Kesin kurallar:
- R6'nın koruma kuralları geçerli: uç nokta başına tek bağlantı, çoklu bağlantıda dur,
  kimlik Qapsule'e özgü, `quantex` salt okunur, NATS yok. REST çekimini ve canlı bağlantıyı
  yalnız sen (orkestratör) yap, sırayla; alt ajanlar önbellek ve benzetimle çalışsın.
- REST isteklerini ölçülü yap (aralıklı, sınırlı yeniden deneme, yerel önbellek); aynı
  veriyi iki kez çekme; toplam istek sayısını raporla.
- İşlem verisinde yalnız fiyat, adet, yön, zaman, işlem no. Alıcı ve satıcı kurum bilgisini
  (düzey 2, AKDE) isteme, saklama: `@lvl2` konusu ve `trade_bs` yok.
- `.env.local`'ı açma; değerleri yazdırma. Gerçek veri, adres, belirteç depoya, günlüğe,
  rapora girmesin; rapora yalnız toplu sayı.
- Geleceğe bakma: kalibrasyon ve sınama tarih aralıklarını ilk iş olarak yaz ve commit et;
  sonra değiştirme. Alternatif ajanlarına sınama aralığını verme. Sınama aralığını yalnız
  son ölçümde aç.

Nasıl çalışacaksın:
- İlk iş: ana ağaçta bekleyen doküman değişikliklerini `CLAUDE.md`'deki "Ana akış
  değişiklikleri" kuralına göre kaydet. Küçük bir deneme alt ajanı başlat; başlatılamıyorsa
  dur ve bana bildir.
- Sıra: veri çekimi → işlemden bar ve benzetim → ölçüm düzeneği ve puanlama kuralı
  (commit) → alternatifler (ayrı ajanlar, ayrı dallar) → seçim → bütünleştirme → sınama
  aralığında son ölçüm.
- Arkin'in gördüğü durumu (5 dk, düşüş iddiası ertesi barda "tutmadı", sonra fiyat iddia
  yönünde gitti; `rounds/R6/REPORT.md` §10) benzetimde yeniden üret ve seçtiğin kuralla
  nasıl sonuçlandığını göster.
- Veri azsa söyle: kaç gün, kaç sembol, kaç hikâye, güven aralığı. Fark aralığın içindeyse
  "ayırt edilemedi" yaz ve bugünkü kuralı bırak. Seçimi kesinleştirme: `DECISIONS.md`'ye
  "Önerilen (RP4)".
- Kapsül sabitlerini değiştirirsen sürümle: eski sabitler adıyla seçilebilir kalsın, eski
  dökümler ve altınlar onlarla bayt eşit üretilsin. Hesap özeti değişmesin. Günlük periyot
  kötüleşmesin.
- Tasarımı değiştirme (yalnız satırlarda saat ve kapanış seansı ifadesi).
- Ölç, varsayma. Hedef tutmuyorsa sayıyı olduğu gibi yaz. Kırık testle birleştirme.
- Disk: `target/wt/<görev>`; elenen dalın worktree'sini kaldır. Round sonunda kapanış
  temizliğini atlama (`CLAUDE.md` "Disk hijyeni", K68): geçici her şeyi sil, `target/`
  bütçesine in, raporda silinenleri, bırakılanları ve kalan boyutları listele. Bu bir kabul
  ölçütü.
- Bitince bağımsız bir denetçi alt ajan başlat (ağa çıkmaz).
- `../Docs` ve `../Services` salt okunur.

Bitiş koşulu: `rounds/RP4/GOAL.md` içindeki on beş ölçüt kanıtlarıyla doldurulmuş;
`REPORT.md`, `AUDIT.md` yazılmış; `STATUS.md` ve `ROADMAP.md` güncellenmiş. Raporun
sonuna Arkin için iki bölüm ekle: "hızlandırılmış benzetimi nasıl izlerim" ve "ilk iş
günü yapılacaklar" (komut, beklenen sonuç, süre). Başka round başlatma.
