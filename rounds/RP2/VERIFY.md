# RP2 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`; `scripts/qgh run list --branch main --limit 1` | CI yeşil |
| 3 | `qp conform` üç taşımada | Hepsi geçer; değişen dökümlerin listesi raporla aynı |
| 4 | Hesap özeti; var olan altın görseller | Değişmedi |
| 5 | Artımlı = baştan hesap özellik testi, sabit tohum | Geçer |
| 6 | Tik maliyeti ölçümü yeniden çalıştırılır (5/50/500 hesap) | Raporla uyumlu (±%20); hedefle karşılaştırma |
| 7 | 10.000 barlı bağlı dokümanın anlık görüntüsü | Bar içermez; boyut raporla aynı |
| 8 | Yetkisiz veri bağı | Tanımlı hata; belgenin geri kalanı çalışır |
| 9 | Kanıt doğrulama: veri değişmeden ve bir bar düzeltildikten sonra | İlkinde doğrulanır, ikincisinde fark bildirilir |
| 10 | Tutarlılık senaryoları, seçilen yaklaşımla | Yanlış ya da erken gösterilen bulgu 0 |
| 11 | Denetçi kendi yazdığı bir senaryoyla (gecikme + sıra bozulması + düzeltme) dener | Sonuç AUDIT'te |
| 12 | Otorite (yerel) ve yansı (WASM) aynı tik akışıyla | Doküman ve görünüm özetleri aynı |
| 13 | Etkilenmeyen görünüme tikte mesaj gitmediği | Test geçer |
| 14 | Yelpaze görünümü ekranlı bağlantıda `screen` ve nokta sorgusu | Gelecek yuvalar görünür |
| 15 | Bulgu belirtilmeden sunum, iki canlı bulgu | `ambiguous`, seçenekli |
| 16 | Gizli bilgi taraması ve eski klasörlerin değişmediği | Temiz |
