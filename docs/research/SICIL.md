# Tasarım taslağı — Sicil: geriye dönük test, tutma ihtimali, zaman aralığı, ortak analiz, topluluk

Durum: **model ve S-1, S-2, S-4, S-5 kabul edildi (X25–X28, 2026-11-21)**; S-3 ve S-6 açık. Kaynak kararlar: X8, X12, X14, X18,
X19, X20, X21 (`docs/rounds/RX2/DECISIONS.md`). Qapsule'e uygulanmadı (X1).

## Temel fikir: tek sicil

Beş konu (geriye dönük test, tutma yüzdesi/ihtimali, zaman aralığına göre okuma, ortak
analiz, topluluk) ayrı sistemler değil; **tek bir kaydın** farklı yüzleri:

> **İddia + sonuç.** Sistemde "şu olacak / şu oldu" diyen her şey, doğduğu anda kendi sonuç
> kuralıyla birlikte kaydedilir; sonucu sonradan, ileriye bakmadan, deterministik kapatılır.

Qapsule'de bu bugün yalnız kapsül için var (karne: `realized` / `invalidated`, mühür,
"karne lisanstan bağımsızdır"). Öneri: aynı kaydı bütün iddia kaynaklarına genişletmek.

| İddia kaynağı | Örnek | Sonuç kuralı |
|---|---|---|
| Kapsül (motor dedektörü) | "yukarı kırılım" | var (kapsül çıkış kuralı) |
| Kullanıcının çizim izlemesi | "kanal yukarı kırıldı" | X14: tutundu / geri döndü |
| Strateji ya da gösterge sinyali | "X stratejisi AL" | karşı sinyal / hedef / durdurma / süre |
| Tutma ihtimali beyanı | "bu durumda %72" | beyanın kendisi kalibrasyon için kaydedilir |
| Hazırlık dosyası / analiz | "bilanço beklentisi" | çoğu sonuçsuz (puanlanmaz); varsa açık kural |

Kural: **sonuç kuralı doğumda yazılı değilse iddia puanlanmaz.**

## Dört katman

1. **Sicil (kayıt).** İddia, kökeni, enstrüman, zaman, koşulun deterministik tanımı, o anki
   durum parmak izi, sonuç kuralı, kanıt; sonra sonuç. Mühürlü, sıralı, "o tarihte ne
   biliniyordu" sorusuna cevap verir (as-of).
2. **Yeniden oynatma (geriye dönük test).** Ayrı bir motor değil: geçmiş veri canlı yolun
   aynısından geçirilir, sicile `kaynak: yeniden oynatma` etiketiyle yazılır. Canlı kayıt ile
   yeniden oynatma kaydı **hiçbir zaman karıştırılmaz** (geçmişe uydurulmuş strateji ile
   canlıda sınanmış strateji aynı şey değildir).
3. **Okuma yüzeyleri.**
   - *Tutma yüzdesi:* sembol × iddia türü × periyot için sonuçların oranı; örnek sayısı ve
     aralığıyla; yazı-turaya yakınsa gizlenir (mevcut karar).
   - *Tutma ihtimali (X18):* çok göstergeli, çok periyotlu durum parmak izi → o durumun
     geçmiş tutma sıklığı; izleme sürerken güncellenir; kalibrasyonu ayrıca ölçülür.
   - *Zaman aralığı okuması (X12):* seçilen aralıkta ne oldu: deterministik dalga özeti, o
     aralıktaki iddialar ve sonuçları, haber/bildirim katmanı, ajanın kaynaklı açıklaması.
     Geriye dönük test değildir: strateji denemez, olmuş olanı okur.
4. **Paylaşım.**
   - *Ortak analiz (X20):* bir kez üretilen, sürümlenen, üye olunan nesne; üye sayacı; kişiye
     özel katman ayrı.
   - *Topluluk (X8):* paylaşılan birim tanım + sicil (strateji/koşul ve canlı tutma kaydı);
     güven birimi canlı, mühürlü sicil.
   - *Sinyal aboneliği (X21):* bir stratejinin sinyallerine üye olmak; eylem basamağı (eşik
     aşılırsa pozisyon aç) onay basamaklarının en üstünde, aracı kurum bağlantısına bağlı.

## Zaman aralığı okuması için altyapı (Arkin'in eki)

Geriye dönük testle aynı temele oturuyor; ayrıca gereken üç parça:

1. **As-of sicil:** gerçekleşmemiş iddialar dahil, zamana göre dizinli; "o gün ekranda ne
   vardı" yeniden kurulabilir.
2. **Olay dizini:** haber, KAP bildirimi, bilanço, kurum raporu zaman × sembol ile
   sorgulanabilir (kaynak: Matriks MCP; küresel açılımda sağlayıcı takılabilir).
3. **Deterministik aralık özeti:** seçilen aralıktan dalgalar, uç noktalar, hacim olayları
   (model değil kod); model yalnız açıklama ve kaynak eşleştirmede.

Bu üçü yoksa aralık okuması "modelin anlattığı hikâye"ye döner; varsa her cümle sicile ya
da kaynağa bağlanır.

## Kararlar (S-1, S-2, S-4, S-5 kabul; S-3 RX4/RX5'te ölçülecek; S-6 araştırılacak)

| # | Soru | Öneri |
|---|------|-------|
| S-1 | "Tuttu" tanımı kimde? | Her iddia sonuç kuralını doğumda taşır; kural yoksa puanlanmaz. Kullanıcı stratejisinde kuralı kullanıcı seçer, sistem varsayılan önerir. |
| S-2 | Yeniden oynatma ile canlı kayıt nasıl gösterilir? | Hep ayrı, etiketli; topluluk sıralaması yalnız canlı kayıtla. |
| S-3 | "Benzer koşul" nasıl tanımlanır? | Kaba, önceden tanımlı durum parmak izi (trend, oynaklık, hacim, konum; birkaç periyot); örnek sayısı az ise gösterilmez; tanım RX4 diliyle genişler. |
| S-4 | Ortak analizi kim yayımlar? | Önce yalnız sistem (bilanço hazırlığı, aralık açıklaması, kapsül yorumu); sonra kullanıcı stratejisi/analizi canlı sicille. |
| S-5 | Kullanıcı verisi toplu istatistiğe girer mi? | Yalnız açık rızayla ve toplu, kimliksiz ("bu seviyeyi 240 kişi izliyor"); pozisyon hiçbir zaman paylaşılmaz. |
| S-6 | Düzenleme | Sinyal paylaşımı ve kopya işlem birçok ülkede düzenlemeye tabi; küresel kapsam (X11) için ülke bazında incelenmeli (araştırılmadı). |

## Bağımlılıklar

- RX4 (ajan yazımı hesap dili): kullanıcı/ajan tanımlı strateji ve durum tanımı için şart.
  Sicilin genişlemesi, yeniden oynatma ve aralık okuması RX4'ü beklemeden mevcut
  dedektörler ve çizim izlemeleriyle başlayabilir.
- Qapsule'de hazır olanlar: kapsül karnesi, mühür, yeniden oynatma/hızlandırılmış benzetim,
  kanıt bağı, lisanstan bağımsız karne gösterimi.
