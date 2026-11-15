# M4 — Tema kimliği: sonuçlar ve seçim

Düzenek `harness-sim` (`176f77e`). Alternatifler ayrı dallarda, birbirini görmeden: **M4a
`kinddir`** (`r3/m4a-kinddir` `b38b317`), **M4b `arc`** (`r3/m4b-arc` `dd3163d`).
Değerlendirme dalı `r3/eval-sim`; ham kayıtlar bayt eşit yeniden üretildi. Bütünleşik yeniden
koşum: sonuç kuralı `barrier` (M5 seçimi) → `raw-chosen/`.

## Ham sonuçlar (sonuç `barrier`; başvuru sonucuyla aynı tablo `raw/`'da, aynı sıralama)

| Ölçü | kinddir | arc |
|---|---|---|
| Azalma, etiketli s1–s3 | 0,444 / 0,453 / 0,459 | **0,561 / 0,566 / 0,580** |
| Azalma, rejimli pazar | 0,490–0,500 | **0,565–0,567** |
| Azalma, gerçek günlük / saatlik | 0,456 / 0,476 | **0,564 / 0,597** |
| Yanlış birleştirme (çift oranı; karışık hikâye) | 0,000 (0) | 0,000 (0) |
| Bölünme (çift oranı), etiketli | 0,909–0,920 | **0,817–0,846** |
| Kaybolan bilgi: türü/yönü hikâyeden farklı olgu (etiketli) | **241–264 (~%4)** | 1.509–1.588 (~%22) |
| — rejimli pazar / gerçek | **~%17 / %3–7** | ~%31 / %24–30 |
| Kaybolan bilgi: düşen aile | 10–20 | **0** |
| Yok sayılan olgu | 833–847 | 871–881 |

## Seçim: **M4a — tür + yön (akışı kanıt olarak alan)**

- Yanlış birleştirme ikisinde de 0; `arc` %11–13 daha az kapsül ve daha az bölünme veriyor,
  ama olguların beşte biri ile üçte biri hikâyenin türünden farklı: işaretin şekli (tür, K30)
  ve cümle bu olguları yanlış türle anlatıyor. İlke 6 ("her görsel öğe bir olguya bağlı")
  için kaybolan bilginin %4'te kalması belirleyici sayıldı; fazladan kapsül sayısı önem
  sıralaması ve hassasiyetle yönetilir.
- `kinddir`'in K30 istisnası (aynı ya da sonraki barda yönlü akış olgusu kırılım/dönüş/devam
  hikâyesini teyit eder) olmadan fiyat hikâyesi hiç teyit edilemiyordu; ölçüldü (NOTES).

## Sınırlar

- Bölünme büyük ölçüde sonuç kuralının hikâyeyi erken kapatmasından (arc ajanının tanısı:
  4.453 "yeni hikâye" kararından yalnız 196'sı kuralın seçtiği bölünme).
- Yanlış birleştirme = 0 az şey kanıtlıyor: etiketli bölümler 35 sessiz barla ayrık.
- Etiketli kümede kurumsal akış yok; akışın doğru bağlandığı sınanamadı. Sentetik akış aynı
  barın getirisinden kısmen türediği için bazı "teyitler" bağımsız kanıt değil.
- Kırılım↔sıkışma↔devam gibi türler arası bölümler her zaman bölünüyor (öncülün bedeli).
