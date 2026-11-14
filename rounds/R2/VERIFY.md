# R2 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`; `scripts/qgh run list --branch main --limit 1` | CI yeşil; bağlantısı raporda |
| 3 | `qp schema export`; `fixtures/views/*.json` üretilen şemaya göre doğrulanır | Hepsi geçerli |
| 4 | `qp render fixtures/views/<her biri>.json` iki kez | Çıktılar birebir aynı ve altınla eşleşir |
| 5 | R0 altın görselleri `git diff --stat` | Değişiklik yok (ya da raporda gerekçeli, güncellenmemiş) |
| 6 | Denetçi altı görseli açar; her birinde en az iki noktayı (ör. son RSI değeri, bant üst sınırı) veriden bağımsız hesaplayıp eksendeki konumla karşılaştırır | Tutarlı; bulgular AUDIT'te |
| 7 | Bozuk görünüm örnekleri (olmayan seri, yanlış işaret adı, eksik kodlama) | Çıkış kodu sıfırdan farklı; ileti katmanı ve alanı adlandırır |
| 8 | Denetçi motor koduna dokunmadan yeni bir görünüm yazar (ör. hacim çubukları + kapanış alanı) ve çizer | Çizilir; görsel anlamlı |
| 9 | 100k bar kare süresi ölçümü yeniden çalıştırılır | Raporla uyumlu (±%20); hedefle karşılaştırma |
| 10 | `qp explain` altı görünümde rastgele 20 nokta | Dönen değerler veriyle aynı; boş alanda "öğe yok" |
| 11 | GPU karşılaştırma testi | Geçer ya da açık SKIP; yerelde en az bir kez çalıştırılmış ve sonucu raporda |
| 12 | Web gösterimi başsız tarayıcıda: CPU karesi, sonra GPU ya da CPU'da kalma | Çalışır |
| 13 | `grep -rn -i -E "sma|ema|rsi|macd|bollinger" crates/qp-grammar/src crates/qp-render*/src` | Hesap kodu yok |
| 14 | Gizli bilgi taraması ve eski klasörlerin değişmediği | Temiz |
