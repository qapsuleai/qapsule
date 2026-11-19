# R6 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh`, ağ kapalıyken | Çıkış kodu 0; hiçbir adım ağa çıkmıyor |
| 2 | `scripts/preflight.sh`; CI son çalışma | Yeşil |
| 3 | İki Qapsule süreci aynı anda canlı bağlanmayı dener (sahte sunucuyla) | İkincisi kilit nedeniyle başlamaz, nedenini söyler |
| 4 | Sahte sunucu "çoklu bağlantı" bildirimi gönderir | Bağlantı kapanır; yeniden deneme yok; açık hata |
| 5 | Günlük ve hata çıktılarında belirteç, parola, yetki başlığı, uç nokta adresi | Yok (maskeli); testle |
| 6 | Depoda gizli değer, uç nokta adresi, gerçek veri, ham kayıt taraması | 0 bulgu; `fixtures/local` izlenmiyor |
| 7 | Alan varlığı: yalnız bir alanı taşıyan güncelleme | Öteki alanlar değişmiyor; 0 yazılmıyor |
| 8 | Aynı ham kayıt iki kez oynatılır | Bayt eşit mesaj akışı |
| 9 | Kayıt üzerinde canlı bar ↔ REST bar karşılaştırması | Raporla aynı tablo |
| 10 | Kayıtta benzetilmiş kopma | Bayat işareti; onarım; yanlış ya da erken kapsül 0 |
| 11 | Borsa kapalı durumunda gösterim (kayıttan) | "Seans kapalı" söyleniyor; bayat veri canlı gibi gösterilmiyor |
| 12 | İç içe gösterge adresleri | Doğrudan kurulan zincirle bit eşit |
| 13 | WASM paket boyutu | Değişmedi (bağlayıcı bağımlılıkları pakette yok) |
| 14 | Hesap özeti, var olan altınlar ve dökümler | Değişmedi |
| 15 | `experiments/LIVE-LOG.md` | Her canlı deneme kayıtlı; eşzamanlı deneme yok; "çoklu bağlantı" olayları açıklamalı |
| 16 | `quantex` deposunda `git status --porcelain`, önce ve sonra | Aynı |
| 17 | `scripts/demo.sh --live --lan` | Reddeder ya da gerçek verinin yerel ağa açılacağını açıkça uyarıp onay ister |
| 18 | `scripts/clean.sh`; kalan boyut | Raporda; `git worktree list` yalnız ana ağaç |
| 19 | Kopyalanan proto dosyalarının özetleri ↔ köken kaydı ↔ `quantex` kaynağı | Üçü aynı; elle değişiklik yok; dosya sayısı kaynakla aynı |
