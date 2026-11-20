# RP4 — Gün İçi Kalibrasyon, İşlem Akışı ve Sağlamlaştırma

## Neden

R6 Qapsule'ü canlı BIST'e bağladı. Gerçek veri üç zayıflığı gösterdi:

1. **Gün içi kapsüller fazla kolay kapanıyor.** Arkin'in 5 dakikalık grafikte gördüğü: bir
   düşüş iddiası ertesi barda "tutmadı" diye kapandı, fiyat sonra iddia yönünde eşiği aştı.
   Kural yazıldığı gibi çalışıyor; sorun ölçekte: çıkış eşiği 5 dakikada yaklaşık 4 fiyat
   adımı (fiyatın ≈ %0,3'ü), tek barlık gürültü hikâyeyi bitiriyor. Çıkış kuralı ve
   dedektör sabitleri R3'te **sentetik** veriyle ayarlandı; gerçek gün içi veride hiç
   ölçülmedi. Karnenin güvenilirliği buna bağlı.
2. **Gün içi barlar yaklaşık.** Fiyat kartı bir işlem akışı değil, birleştirilmiş anlık
   görüntüdür; 1 ve 5 dakikalık barların %5–9'unda fiyat REST'ten farklı (en çok %1,3).
   Kapsüller yaklaşık barlar üzerinde doğuyor. Kesin kaynak **işlem akışıdır** (K66).
3. **Koruma ve ölçek eksik.** Piyasa ucunda "çoklu bağlantı" bildirimi alınamıyor (K64,
   A52); 580 sembolde bellek 0,9 GB, işlemci %69, tikten ekrana p95 835 ms.

Tasarım turu (RD2) bu düzeltmelerden sonra gelir: yanlış kapanan kapsüller üzerinde
tasarım değerlendirilmez (K65).

## Hafta sonu çalışma biçimi (K65)

Borsa kapalıyken round durmaz. Canlı akış yerine **REST geçmişinden hızlandırılmış
benzetim** kullanılır: geçmiş işlemler ve barlar çekilir, canlı yolun kendisinden
(`LiveFeed`, otorite, dedektörler, karne) hızlandırılmış olarak geçirilir. Canlı
bağlantı gerektiren doğrulamalar (işlem ucuna ilk bağlantı, anlık kip, durma kuralının
gerçek davranışı) **ilk iş gününe** bırakılır: uydurulmaz, açık bırakılır ve Arkin'e
hangi komutu ne zaman çalıştıracağı yazılır.

## Veri (K66; `docs/DATA_ACCESS.md` "İşlem (trade) verisi")

- Geçmiş işlem: `GET /dumrul/v1/tick/trade.gz` (sembol; en çok 15 günlük aralık; CSV:
  zaman, gün başından nanosaniye, yön, işlem no, fiyat, adet).
- Geçmiş bar: R6'daki REST istemcisi (1 dk, 5 dk, 1 sa, 1 gün; düzeltilmemiş).
- Canlı işlem: ayrı uç nokta yolu `/trade`, konu `mx/trade/<SEMBOL>`; ilk durum yok.
  Lisansın izin verdiği konu ve düzey Topach servisinden okunur.
- **Düzey 1 alanları kullanılır** (fiyat, adet, yön, zaman, işlem no). **Alıcı ve satıcı
  kurum bilgisi (düzey 2, AKDE) bu round'da alınmaz**: `@lvl2` konusuna abone olunmaz,
  `trade_bs` çağrılmaz; lisans izin verse bile.
- Kimlik, belirteç ve adresler `.env.local`'dan (K61); dosya açılmaz. Arkin kipi
  `realtime` yaptı (K67): iş gününde canlı denemeler anlık uçta yapılır; işlem akışı
  yalnız anlık yayınlanır.
- Çekilen her şey `fixtures/local/` altında kalır; depoya, rapora, günlüğe gerçek veri girmez.

## Değişmezler

- R6'nın koruma kuralları aynen geçerli: uç nokta başına tek bağlantı (işlem ucu ayrı bir
  uç noktadır ve kendi kilidini alır), canlı bağlantıyı yalnız orkestratör açar ve sırayla,
  alt ajanlar kayıt ve benzetimle çalışır, `quantex` salt okunur, NATS'a bağlanılmaz.
- REST istekleri ölçülüdür: sembol başına aralıklı, yeniden deneme sınırlı, toplam istek
  sayısı raporda; aynı veri iki kez çekilmez (yerel önbellek).
- **Geleceğe bakmak yok.** Kalibrasyon bir tarih aralığında yapılır, başka (sonraki) bir
  aralıkta sınanır. Sınama aralığına bakılarak seçilen sabit geçersizdir.
- Hesap deterministik kalır; dil modeli ürün yolunda yok (K3). Hesap özeti (`qp-compute`)
  değişmez. Kapsül sabitleri değişirse bu **gerekçeli bir sürüm değişikliğidir**: eski
  sabitler adıyla seçilebilir kalır, değişen dökümler ve görseller listelenir.
- Günlük periyottaki davranış kötüleşmez (R3'ün gerçek veri ölçümleri başvuru).
- Tasarım değişmez (RD2). Görünen tek değişiklik: olay ve kapsül satırlarında saat.

## Kapsam

1. **Geçmiş işlem istemcisi ve yerel önbellek** (`trade.gz`; 15 günlük pencerelerle).
2. **İşlemden kesin bar:** borsa takvimine hizalı (K14), bütün periyotlar; açılış ve
   kapanış seansı işlemleri; özel işlemlerin (`tradeex`) bara girip girmediği ölçülerek
   belirlenir (REST barıyla hangisi eşleşiyor).
3. **Canlı işlem bağlayıcısı:** `/trade` ucu, kendi kilidi; işlemden canlı bar; fiyat
   kartı günlük alanlar ve seans evresi için kalır. Kod ve sahte sunucu testleri şimdi;
   gerçek bağlantı ilk iş gününde.
4. **Hızlandırılmış benzetim:** geçmiş işlem ve barlardan, canlı yolun aynısından geçen
   belirleyici yeniden oynatma; hız katsayısı; çok günlük; `scripts/demo.sh --sim` ile
   gösterimde de izlenebilir (gerçek veri: yalnız bu makinede).
5. **Gün içi kalibrasyon** (aşağıda M25–M27).
6. **Bağlayıcı sağlamlaştırma:** A52 durma kuralı; `.halted` için insan onaylı sıfırlama
   komutu; geç katılan tam kopya istemcinin eksik seri alması (R6 gözlemi) doğrulanır ve
   varsa düzeltilir; anlık kip ve belirteç ömrü için ilk iş günü sınama listesi.
7. **Ölçek:** görüntüleyici dilimi ve yüzlerin artımlı hesabı; 580 sembol hedefi.
8. **Küçük işler:** olay ve kapsül satırlarında saat (gün içinde saat:dakika, eski günlerde
   tarih); kapanış emir toplamada genel satır "kapanış seansında" (A48).

## Kapsam dışı

Tasarım (RD2). Ajan yüzeyi (R5B). Derinlik, kurumsal akış (AKDE), takas, haber. Yeni
dedektör türü. Kullanıcı başına Matriks yetkisi.

## Ölçümle karar verilecek konular (K39)

Düzenek ve puanlama kuralı alternatiflerden **önce** yazılır ve commit edilir.
Alternatifler ayrı alt ajanlarca kurulur. Bugünkü kural her ölçümde başvuru olarak yarışır.

| # | Konu | Alternatif (en az) | Ölçüm |
|---|------|--------------------|-------|
| M25 | **Çıkış kuralı** (hikâye ne zaman "tutmadı / gerçekleşti / süresi doldu" olur) | Bugünkü (`0,3·√h·ATR14`, ilk değen kapanış); eşiğe en az N fiyat adımı tabanı; iki kapanışla teyit; kapanış yerine yüksek/düşükle karar; periyoda göre ölçeklenen eşik; savunucusunun önerdiği başka bir kural | 5 dk, 15 dk, 1 sa, 1 gün ayrı ayrı: gerçekleşti / tutmadı / süresi doldu oranları; **"tutmadı dedikten sonra süre içinde iddia yönünde eşiği aştı" oranı** (erken kapanma); "gerçekleşti dedikten hemen sonra geri döndü" oranı; hikâye ömrünün dağılımı; karnenin kalibrasyonu (iddia edilen oran ↔ sınama aralığındaki oran, Brier); tek barlık gürültüye duyarlılık (bir barın kapanışı ± 1–2 fiyat adımı oynatılınca sonucu değişen hikâye payı) |
| M26 | **Dedektör eşikleri gün içinde** (yedi dedektörden gün içinde anlamlı olanlar) | Bugünkü sabitler; periyoda göre ölçeklenen; gün içi mevsimselliğe (açılış, öğle, kapanış) göre düzeltilen (hacim için) | Sembol ve gün başına kapsül sayısı (akış yükü); sınama aralığında tutma oranı ve yazı tura eşiğine (K53, %55) göre konumu; açılış ve kapanış seansında yanlış tetikleme; günlük periyotla tutarlılık |
| M27 | **Gün içi bar kaynağı** | Fiyat kartından (R6); işlemden; karma (işlem varsa işlem, yoksa kart) | REST barıyla tam eşit bar oranı ve en büyük sapma (R6 M22 düzeneği); aynı günde iki kaynaktan doğan kapsüllerin farkı (kaç kapsül yalnız bir kaynakta doğdu) |

**Dürüstlük kuralları:** veri azsa söylenir (kaç gün, kaç sembol, kaç hikâye; güven
aralığı). Fark aralığın içindeyse "ayırt edilemedi" yazılır ve bugünkü kural kalır.
Seçim önerilir; `DECISIONS.md`'de "Önerilen (RP4)". Kalibrasyon ve sınama aralıkları
önceden yazılır ve değiştirilmez.

## Hedefler

| Ölçü | Hedef |
|---|---|
| 5 dk'da erken kapanma oranı (M25) | Bugünkü kuralın en çok yarısı; kalibrasyon bozulmadan |
| Gün içi bar ↔ REST (M27, işlemden) | Fiyat ve hacimde ≥ %99 tam eşit; kalan farklar açıklanmış |
| 580 sembol, tek süreç (hızlandırılmış benzetimde 1× hızda) | İşlemci ≤ %30, bellek ≤ 450 MB, tikten çerçeveye p95 ≤ 150 ms |
| Benzetim hızı | Bir işlem günü, 100 sembol, 5 dk: ≤ 2 dakikada |

Hedef tutmuyorsa sayı olduğu gibi raporlanır; hedef gevşetilmez.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | Geçmiş işlem istemcisi: 15 günlük pencereler, yerel önbellek, ölçülü istek; çekilen aralık, sembol sayısı ve toplam istek raporda; veri yalnız `fixtures/local/` altında. Düzey 2 alanları hiçbir yolda istenmiyor (test ve kod taraması). |
| 2 | İşlemden bar: M27 ölçülmüş (en az 30 sembol, en az 10 işlem günü, 1 dk / 5 dk / 1 sa / 1 gün); açılış ve kapanış seansı ile özel işlemlerin ele alınışı ölçümle belirlenmiş ve belgeli; zaman alanının birimi doğrulanmış. |
| 3 | Canlı işlem bağlayıcısı: kod, kendi kilidi, sahte sunucu testleri (ilk durum yok; kopmada bayat ve REST'ten işlem geçmişiyle onarım); fiyat kartıyla birlikte çalışma. Gerçek bağlantı denemesi ya yapılmış (iş günündeyse) ya da "ilk iş günü" listesinde komutuyla. |
| 4 | Hızlandırılmış benzetim: canlı yolun aynısı (ayrı bir kod yolu değil; test); aynı girdi iki kez bayt eşit çıktı; hız hedefi ölçülmüş; `scripts/demo.sh --sim` çalışıyor ve gerçek veriyi yerel ağa açmıyor. |
| 5 | M25: düzenek ve kural önce commit; en az beş alternatif; kalibrasyon ve sınama aralıkları önceden yazılı; dört periyotta tablo; erken kapanma oranı önce/sonra; duyarlılık sınaması; öneri gerekçeli. Arkin'in gördüğü durum (5 dk, düşüş iddiası, ertesi barda "tutmadı") benzetimde yeniden üretilmiş ve seçilen kuralla nasıl sonuçlandığı gösterilmiş. |
| 6 | M26: ölçülmüş; gün içinde anlamsız çıkan dedektör varsa dürüstçe söylenmiş (kapatma ya da periyot sınırı önerisi); açılış ve kapanış seansı yanlış tetiklemeleri sayılmış. |
| 7 | Günlük periyot kötüleşmedi: R3'ün gerçek veri ölçümleriyle karşılaştırma tablosu. |
| 8 | Sabit değişiklikleri sürümlü: eski sabitler adıyla seçilebilir; değişen dökümler ve görseller gerekçeli listeli; hesap özeti aynı. |
| 9 | A52 durma kuralı: 10 dakikada 2 açıklamasız sunucu kapatmasında dur; insan onaylı sıfırlama komutu; sahte sunucu testleri. Geç katılan istemci sorunu doğrulanmış ve (varsa) düzeltilmiş. |
| 10 | Ölçek: artımlı hesap; 580 sembol hedef tablosu (önce/sonra); 100 sembolde gerileme yok. |
| 11 | Saat gösterimi ve kapanış seansı satırı; var olan altınlar bayat kalmıyorsa gerekçeli yeni sürüm, eski sürüm bayt eşit. |
| 12 | "İlk iş günü" listesi: canlı işlem ucu, anlık kip (A49), belirteç ömrü (A50), durma kuralının gerçek davranışı, tam seans kaydı (R6 ölçüt 4). Her biri için komut, beklenen sonuç ve süre. Round iş gününe denk geldiyse yapılanlar `experiments/LIVE-LOG.md`'de. |
| 13 | `scripts/verify.sh` yeşil ve ağsız; CI yeşil; gizli bilgi ve gerçek veri taraması temiz; `quantex` durumu aynı. |
| 14 | **Kapanış temizliği (K68):** geçici dosyalar silindi; `target/` ≤ 6 GB; `fixtures/local/` saklama kuralına göre budandı; raporda silinenler, bırakılanlar ve kalan boyutlar; izlenmeyen artık dosya yok. |
| 15 | `REPORT.md`, `AUDIT.md` (bağımsız; kalibrasyonda geleceğe bakma olmadığını commit sırası ve koddan doğrular; ölçümleri önbellekten yeniden üretir); `docs/CAPSULE.md` ve `docs/matriks/LIVE.md` güncel; `STATUS.md`, `ROADMAP.md`; önerilen kararlar; disk hijyeni (K56, K68). |
