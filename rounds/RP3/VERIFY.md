# RP3 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`; CI son çalışma | Yeşil |
| 3 | Kalibre benzetim, R4 kodunda (iyileştirme öncesi commit) | Arkin'in iPhone ölçümündeki tik, yüz, sahne süreleri ±%30 içinde |
| 4 | Aynı benzetim, seçilen yolla | Hedef tablosuyla karşılaştırma; raporla uyumlu (±%20) |
| 5 | Her boyama yolu adres parametresiyle açılır | Çalışır; WebGPU kapalıyken yedek devreye girer |
| 6 | Otomatik karşılaştırma başsız tarayıcıda | Üç dakikadan kısa; tek özet; elle hareket gerektirmez |
| 7 | Seçilen yol ile referans çizici arasında görsel fark, altı görünüm ve kapsül yüzleri | Tolerans içinde; fark raporla aynı |
| 8 | `screen` ve nokta sorgusu, boyama yolundan bağımsız | Sonuçlar bayt düzeyinde aynı |
| 9 | Mimari testi ve denetçinin kendi taraması | Kabukta alan mantığı yok |
| 10 | 5.000 adım yeniden oynatma, bellek | Raporla uyumlu; sınırsız büyüme yok ya da kaynağı yazılı |
| 11 | Açılış: aktarılan bayt ve sıkıştırma başlığı | Sıkıştırılmış paket; önce/sonra tablosuyla uyumlu |
| 12 | 60 Hz benzetiminde uzun kare sayacı; uzun süre açık sayfada açılış süreleri | Yanlış pozitif yok |
| 13 | Var olan altın görseller, dökümler, hesap özeti | Değişmedi |
| 14 | Gizli bilgi taraması; `fixtures/local` depoda yok | Temiz |
| 15 | Evren 30 / 300 / 3.000 enstrüman, görüntüleyici biçimi | Cihazda tik başına iş ve bellek ±%20 içinde aynı |
| 16 | Akıştaki ve açılmış kapsülün güven işareti; RP2/R3 tutarlılık senaryoları | Ekranda ayırt ediliyor; yanlış/erken gösterim 0 |
| 17 | Cihazın açtığı veri abonelikleri, akış ekranında ve bir kapsül açıkken | Akışta bar verisi aboneliği yok; açıkken yalnız o enstrüman |
