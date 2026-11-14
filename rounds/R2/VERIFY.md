# R2 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`; `scripts/qgh run list --branch main --limit 1` | CI yeşil |
| 3 | `qp conform` üç taşımada | RD1'in 17 dökümü ve yeni dökümler geçer |
| 4 | Altı görünüm, kendi dökümünden kurulup çizilir; iki kez | Çıktılar birebir aynı ve altınla eşleşir |
| 5 | R0 altın görselleri `git diff --stat` | Değişiklik yok (ya da raporda gerekçeli, güncellenmemiş) |
| 6 | Denetçi altı görseli ve derinlik görsellerini açar; her birinde en az iki noktayı veriden bağımsız hesaplayıp eksendeki konumla karşılaştırır | Tutarlı |
| 7 | Denetçi yalnızca `docs/PROTOCOL.md`, `docs/GRAMMAR.md` ve şemayı okuyarak yeni bir görünüm kurar (ör. hacim çubukları + kapanış alanı), kimlikleri bilmeden | Çizilir; karşılaştığı belirsizlikler AUDIT'te |
| 8 | Kökeni olmayan işaret, olmayan seri, geçersiz kanal, belirsiz anlamsal başvuru | Reddedilir; ileti katmanı, alanı, nedeni ve seçenekleri verir |
| 9 | T6c'deki 18 eşzamanlı yazma durumu kimlikli işlemlerle | Yanlış sonuç 0 |
| 10 | `screen` ve nokta sorgusu: altı görünümde rastgele 20 nokta | Değerler veriyle aynı; boş alanda "öğe yok" |
| 11 | Sunucu belirleyiciliği: aynı bulgu 1.000 kez, üç derinlik | Tek çıktı |
| 12 | Ajan yazabilirlik ham kayıtları | Alt ajanların dosya erişimi kayıtlarda; kimlikler kopyalanamaz; yeniden puanlama aynı sayıları verir |
| 13 | 100k bar kare süresi yeniden ölçülür | Raporla uyumlu (±%20) |
| 14 | `grep -rn -i -E "sma|ema|rsi|macd|bollinger" crates/qp-grammar/src crates/qp-render/src` | Hesap kodu yok |
| 15 | Hesap özeti ve `qp-compute` davranışı | Özet değişmedi (denetim kalanları dışında davranış aynı) |
| 16 | Gizli bilgi taraması ve eski klasörlerin değişmediği | Temiz |
