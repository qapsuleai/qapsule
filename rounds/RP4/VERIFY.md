# RP4 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh`, ağsız | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`; CI | Yeşil |
| 3 | `git log`: `experiments/SPLIT.md`, düzenek ve puanlama kuralı alternatif dallarından önce | Önce |
| 4 | Alternatif ajanlarına verilen dosyalar | Sınama aralığı yok |
| 5 | Kod ve istek taraması: `@lvl2`, `trade_bs`, alıcı/satıcı alanları | İstenmiyor, saklanmıyor |
| 6 | İşlemden bar ↔ REST bar, önbellekten yeniden | Raporla aynı tablo |
| 7 | Aynı benzetim iki kez | Bayt eşit mesaj akışı |
| 8 | Benzetim yolu = canlı yol | Aynı `LiveFeed`; ayrı kod yolu yok (test) |
| 9 | M25 tablosu, önbellekten yeniden, sınama aralığında | Raporla aynı (±%2) |
| 10 | Arkin'in gördüğü durumun yeniden üretimi | Eski kuralla "tutmadı", seçilen kuralla sonucu raporla aynı |
| 11 | Duyarlılık: kapanışı ± 1–2 fiyat adımı oynatma | Sonucu değişen hikâye payı raporla aynı |
| 12 | Günlük periyot, R3 başvurusuyla | Kötüleşme yok |
| 13 | Eski sabitler adıyla | Eski dökümler ve altınlar bayt eşit |
| 14 | Sahte sunucu: 10 dk içinde 2 açıklamasız kapatma | Durur; yeniden denemez; sıfırlama yalnız insan onaylı komutla |
| 15 | Geç katılan tam kopya istemci | Seriyi eksiksiz alır |
| 16 | 580 sembol benzetim, 1× hız | Hedef tablosuyla karşılaştırma; raporla uyumlu (±%20) |
| 17 | `scripts/demo.sh --sim --lan` | Reddeder (gerçek veri yerel ağa açılmaz) |
| 18 | Depoda gerçek veri, adres, belirteç; `fixtures/local` izlenmiyor | 0 bulgu |
| 19 | `experiments/LIVE-LOG.md` ve REST istek sayımı | Eşzamanlı canlı deneme yok; istek sayısı raporda |
| 20 | `quantex` `git status --porcelain` önce ve sonra | Aynı |
| 21 | Hesap özeti; WASM paket boyutu | Değişmedi; bağlayıcı pakette yok |
| 22 | Kapanış temizliği: `target/` boyutu; `git worktree list`; `git status --porcelain --ignored` özeti; oturum geçici klasörleri | `target/` ≤ 6 GB; yalnız ana ağaç; beklenmeyen artık yok; raporda üç liste |
