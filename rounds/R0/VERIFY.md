# R0 — Doğrulama

Denetçi alt ajan bu adımları deponun temiz bir kopyasında çalıştırır ve her birinin
çıktısını `REPORT.md` içine özetler.

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `scripts/verify.sh` | Çıkış kodu 0 |
| 2 | `cargo fmt --all -- --check` | Fark yok |
| 3 | `cargo clippy --workspace --all-targets -- -D warnings` | Uyarı yok |
| 4 | `cargo test --workspace` | Tüm testler geçer |
| 5 | `cargo run -p qp-cli -- schema export --out /tmp/schema` | JSON Schema dosyaları üretilir |
| 6 | `fixtures/schema/` içindeki örnekler üretilen şemaya göre doğrulanır | Geçerli |
| 7 | `qp render fixtures/bars/sample.json -o /tmp/a.png` iki kez | İki dosya birebir aynı |
| 8 | 7'nin çıktısı `fixtures/golden/` ile karşılaştırılır | Eşleşir |
| 9 | WASM derlemesi | Başarılı; dosya boyutu raporlanır |
| 10 | Test sayfası başsız tarayıcıda açılır, ekran görüntüsü alınır | Çizim var; ekransız çıktıyla fark ölçülür |
| 11 | `git ls-files \| grep -i -E '\.env$\|token\|secret'` | Sonuç yok |
| 12 | `git status --porcelain ../Docs ../Services` (varsa) ve dosya değişiklik zamanları | Eski klasörlerde değişiklik yok |
| 13 | `DECISIONS.md` A1 satırı | Kanıtıyla kapatılmış |

Geçemeyen adım varsa round kapanmaz; neden geçemediği ve önerilen çözüm rapora yazılır.
