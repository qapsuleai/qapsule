# R1 — Doğrulama

Denetçi alt ajan temiz kopyada çalıştırır.

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `scripts/verify.sh` (tarayıcı adımı zorunlu: `QP_VERIFY_BROWSER=1`) | Çıkış kodu 0 |
| 2 | `scripts/preflight.sh`, ardından `scripts/qgh run list --branch main --limit 1` ile CI'ın son çalışması | Yeşil; bağlantısı raporda. Uzak depo yoksa "yapılamadı" |
| 3 | `cargo test -p qp-compute` | Tüm testler geçer; özellik testleri sabit tohumla tekrarlanabilir |
| 4 | Elle hesaplanmış örnekler: denetçi en az iki indikatörü bağımsız olarak (ayrı bir betikle) yeniden hesaplar | Sonuçlar 1e-9 içinde aynı |
| 5 | `qp compute` ile örnek seriden RSI(14) ve MACD çıktısı | Uzunluk = bar sayısı; ısınma `null`; MACD üç çıktı |
| 6 | Toplu, akışkan ve "son barı düzelt" karşılaştırması | Bit-eşit |
| 7 | Tarama ölçümü yeniden çalıştırılır | `BENCH.md` ile uyumlu (±%20); hedeflerle karşılaştırma tablosu |
| 8 | Takvim testleri: BIST ve NYSE, yaz saati geçişi | Geçer |
| 9 | Mimariler arası hesap özeti (aarch64, x86_64, wasm32) | Aynı |
| 10 | `grep -rn "struct Bar" crates/` | Tek tanım, `qp-schema` içinde |
| 11 | `grep -rn -i "sma" crates/qp-render/src` | Çizicide hesap kodu yok; yalnızca `qp-compute` çağrısı |
| 12 | Altın görseller `git diff --stat fixtures/golden` | Değişiklik yok (ya da raporda gerekçeli) |
| 13 | Gizli bilgi taraması (R0 adım 11) ve eski klasörlerin değişmediği | Temiz |
