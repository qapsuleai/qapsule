# RP3 — Telefonda Çizim ve Açılış

## Neden

Arkin'in ölçümü (K47): güncel üst sınıf bir iPhone'da gösterim akıcı değil.

| Ölçü (p50) | Masaüstü | iPhone (iOS 27, DPR 3) |
|---|---:|---:|
| Fiyat değişimi (tik) | 2,8 ms | 74 ms (p95 118) |
| Kapsül yüzü boyama | 0,9 ms | 32 ms |
| Sahne boyama | 2,6 ms | 71 ms |
| Kare, kaydırırken p95 | 33 ms | 50 ms (p99 179) |
| İlk kapsül | — | 3,9 s (WASM hazır 1,8 s) |

Geleceğin yatırımcısı telefonda yaşıyor (`NORTH_STAR.md`). Üst sınıf telefonda akıcı
olmayan bir şey orta sınıf telefonda kullanılamaz. Ham kayıtlar:
`rounds/R4/experiments/D/phone-iphone-ios27-default.json`,
`desktop-mac-chrome156-default.json`.

## Üçüncü ve en önemli ders (K48)

Cihaz bir hesap yeri değil, bir görüntüleyicidir. Bugünkü gösterimde cihazdaki yansı 30
enstrümanın tamamı için kapsül motorunu her tikte çalıştırıyor; binlerce sembolde bu
hiçbir boyama iyileştirmesiyle kurtarılamaz. Doğru iş bölümü:

- Tespit, bulgu, sıralama, karne: **sunucuda**. Kapsüller cihaza hazır gelir.
- Cihaz yalnızca kullanıcının o an baktığı şey için veri alır, hesaplar ve çizer: açık
  enstrümanın grafiği, göstergeleri, adayı, işaretin canlı hareketi.
- Cihazın işi evrendeki sembol sayısından **bağımsızdır**.
- Akıştaki kapsüller otoritenin kesinleşmiş iddiaları olarak gösterilir; cihaz bir
  kapsülü kendi verisinde ancak açıldığında sınar.

Bu round'un birinci işi budur; boyama yolu ikinci gelir.

## İki ayrı ders

1. **Çizim yolu telefonda yetmiyor.** Olası nedenler (varsayım; ölçülecek): yazılımla
   tarama maliyeti piksel sayısıyla büyüyor (DPR 3'te yaklaşık 9 kat piksel); her tikte
   bütün yüzler yeniden boyanıyor; büyük piksel tamponları tuvale kopyalanıyor.
2. **Benzetim yanılttı.** R4'ün 4×–6× işlemci kısıtı gerçeği yaklaşık 3 kat hafif
   gösterdi. Gerçek cihaza kalibre edilmemiş benzetime dayanarak karar verilmez.

## Değişmezler

- Sahne tarifi tek doğruluk kaynağıdır (K24). Hangi boyama yolu seçilirse seçilsin aynı
  sahne tarifini boyar; anlamsal okuma (`screen`, nokta sorgusu) değişmez.
- Deterministik referans çizici kalır: ekransız çıktı, altın görseller ve testler onu
  kullanır. Ekrandaki boyama yolu referansla toleranslı karşılaştırılır, bayt eşitliğiyle değil.
- Kabuk ince kalır: alan mantığı çekirdekte (R4 mimari testi geçmeye devam eder).
- Var olan altın görseller, dökümler ve hesap özeti değişmez.

## Kapsam

1. **Kalibrasyon:** masaüstü benzetimini Arkin'in telefon ölçümünü yeniden üretecek
   biçimde ayarlamak; nedenleri ayrıştırmak (piksel sayısı, işlemci, kopyalama, yeniden
   boyama sıklığı, WASM derleme kipi).
2. **Çizim stratejileri**, alternatifleriyle (K39).
3. **Açılış:** paket boyutu, sıkıştırma, parçalı yükleme, derleme.
4. **Bellek:** uzun oturumda büyüme.
5. **Cihazda otomatik karşılaştırma** ve Arkin'in doğrulama ölçümü.

## Kapsam dışı

- Tasarım değişikliği (yüz, akış, işaretler). Görünen sonuç aynı kalır; yalnızca nasıl
  boyandığı değişir.
- Yerel iOS/Android kabuğu (R8). Gerçek MQTT (R6).

## Ölçümle karar verilecek konular

| # | Konu | Alternatifler (en az; liste sınır değildir) | Ölçüm |
|---|------|---------------------------------------------|-------|
| M12 | Ekrandaki boyama yolu | (a) Bugünkü yazılım çizici + akıllı yeniden boyama: yalnız değişen yüz ve bölge, önbellek, hareket sırasında düşük ölçek. (b) Sahne tarifini tarayıcının kendi tuvaliyle boyayan yol. (c) GPU yolu (K12: WebGPU). (d) Savunucusunun önerdiği başka bir yol ya da karma | Kalibre benzetimde ve gerçek telefonda: tik, yüz ve sahne boyama süresi; kaydırma ve sürükleme kare süresi (p50, p95, p99, uzun kare payı); bellek; paket boyutuna etkisi; referans çiziciyle görsel fark; kod karmaşıklığı; WebGPU olmayan cihazda davranış |
| M15 | **Cihazdaki iş bölümü (K48)** | (a) Bugünkü tam yansı (başvuru). (b) Görüntüleyici: akış sunucudan hazır gelir; cihaz yalnız görünür ve açık olan için veri alır ve hesaplar. (c) Savunucusunun önerdiği ara biçim | Evren büyüklüğü 30 / 300 / 3.000 enstrüman: cihazda tik başına iş, bellek, aktarılan bayt (evrenden bağımsız olmalı); kapsül açılışında veri alma ve ilk çizim süresi; açıldığında doğrulamanın süresi; sunucunun cihaz başına işi; yanlış ya da erken gösterim (RP2/R3 düzeneği, 0 kalmalı) |
| M13 | Açılış | Sunucu sıkıştırması; paketin bölünmesi (ilk ekran için gereken ile sonrası); akışlı derleme; ilk ekranın sunucuda hazırlanması | WASM hazır ve ilk kapsül süresi, soğuk ve sıcak; aktarılan bayt |
| M14 | Canlı güncelleme sıklığı | Her veri olayında boyama; kare başına en çok bir boyama; görünmeyen yüzleri hiç boyamama | Tik başına iş; görünen sonucun gecikmesi |

## Hedefler (öneri; ölçümle doğrulanır, K17'nin telefon kısmı)

| Sınıf | Tik (p95) | Kaydırma karesi (p95) | İlk kapsül (soğuk) |
|---|---:|---:|---:|
| Üst sınıf telefon (Arkin'in iPhone'u) | ≤ 8 ms | ≤ 16,7 ms | ≤ 1,5 s |
| Orta sınıf telefon (4–6 yaşında Android) | ≤ 16 ms | ≤ 33 ms | ≤ 3 s |

Hedef tutmuyorsa sayı olduğu gibi raporlanır; hedef gevşetilmez.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 0 | **İş bölümü (K48):** gösterim görüntüleyici biçiminde çalışıyor: cihaz akış için kapsül motoru koşturmuyor; yalnız görünür ve açık olan için veri alıyor ve hesaplıyor. Ölçüm: cihazdaki tik başına iş ve bellek 30, 300 ve 3.000 enstrümanlık evrende aynı (±%20). Akıştaki kapsül "sunucu tarafından doğrulandı" güveniyle, açılan kapsül cihazda sınanmış olarak gösteriliyor; ikisi ekranda ayırt ediliyor. RP2/R3 tutarlılık senaryolarında yanlış ya da erken gösterim 0. Protokol eki şartnamede; dökümler eklendi; var olan dökümler değişmedi (değişen varsa gerekçeli). |
| 1 | Kalibrasyon: benzetim ayarı Arkin'in iPhone ölçümündeki tik, yüz ve sahne sürelerini ±%30 içinde yeniden üretiyor. Farkın nedenleri ayrıştırılmış ve her birinin payı ölçülmüş (`rounds/RP3/CALIBRATION.md`). R4 `DEVICE.md`'deki eşleme düzeltilmiş. |
| 2 | M12: en az üç boyama yolu, ayrı dallarda, ayrı alt ajanlarla, birbirini görmeden; düzenek alternatiflerden önce. Her yol aynı sahne tarifini boyuyor ve referansla görsel farkı ölçülmüş. |
| 3 | Görüntüleyici iş bölümü ve seçilen boyama yolu birlikte, kalibre benzetimde üst sınıf telefon hedeflerini tutuyor; tutmuyorsa neden ve neyin gerektiği ölçümle gösterilmiş. |
| 4 | Seçilen yol gösterimde varsayılan; diğerleri adres parametresiyle seçilebilir (cihazda karşılaştırma için). WebGPU ya da gereken özellik yoksa güvenli yedek çalışıyor. |
| 5 | M13: açılış ölçülmüş ve iyileştirilmiş; gösterim sunucusu sıkıştırılmış paket veriyor; aktarılan bayt ve ilk kapsül süresi önce/sonra tablosunda. |
| 6 | M14: görünmeyen yüzler boyanmıyor; kare başına en çok bir boyama; ölçüm. |
| 7 | Bellek: 5.000 adımlık yeniden oynatmada WASM ve tarayıcı belleği ölçülmüş; büyüme varsa kaynağı bulunmuş (kapanmış hikâyelerin birikmesi, günlük, önbellek) ve sınırlanmış ya da öneri olarak yazılmış. (Masaüstü ölçümünde 1.260 adımda 152 MB görüldü; R4 raporu 26 MB diyordu.) |
| 8 | **Cihazda otomatik karşılaştırma:** ölçüm ekranında tek dokunuşla başlayan, elle hareket gerektirmeyen bir sınama: her boyama yolunu sırayla aynı senaryoda (durağan, otomatik kaydırma, otomatik sürükleme, tik akışı) çalıştırır ve tek bir karşılaştırmalı özet üretir. Süre üç dakikayı geçmez. |
| 9 | Ölçüm ekranının kendi hataları giderilmiş: 60 Hz ekranda 16,7 ms eşiği zamanlayıcı yuvarlaması yüzünden normal kareleri "uzun" saymıyor; açılış süreleri sayfa uzun süre açık kalınca bozulmuyor (masaüstü ölçümünde `open_ms` 836.034 ms çıktı); tarayıcı yakınlaştırması (DPR 0,8) özetle birlikte uyarı olarak raporlanıyor. |
| 10 | Anlamsal okuma, mimari testi, dökümler, altın görseller ve hesap özeti değişmedi. Ekran görüntüsü testleri seçilen yol için toleranslı karşılaştırmayla eklendi. |
| 11 | Raporun sonunda Arkin için yönerge: telefonda otomatik karşılaştırmayı nasıl çalıştırır, sonucu nereye yapıştırır. Seçim, Arkin'in ölçümü gelene kadar **öneri** olarak kalır; `DECISIONS.md`'de "Önerilen (RP3)". |
| 12 | `REPORT.md`, `RESEARCH.md` (denenen her yaklaşım, işe yaramayanlar dahil), `CALIBRATION.md`, `AUDIT.md`; `STATUS.md`, `ROADMAP.md`. |
