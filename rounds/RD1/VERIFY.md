# RD1 — Doğrulama

| # | Komut / kontrol | Beklenen |
|---|-----------------|----------|
| 1 | `QP_VERIFY_BROWSER=1 scripts/verify.sh` | Çıkış kodu 0; önceki adımlar değişmeden geçer; döküm adımı eklenmiş |
| 2 | `scripts/preflight.sh`; `scripts/qgh run list --branch main --limit 1` | CI yeşil |
| 3 | `fixtures/protocol/` örnekleri üretilen şemaya göre | Geçerliler geçer, geçersizler reddedilir |
| 4 | Döküm koşucusu, süreç içi | On iki ya da daha çok senaryo geçer |
| 5 | Aynı dökümler `qp serve` üzerinden | Aynı sonuç |
| 6 | Bir dökümün günlüğü baştan oynatılır | Doküman bayt düzeyinde aynı |
| 7 | Denetçi yalnızca `docs/PROTOCOL.md` ve şemayı okuyarak üç yeni görev için mesaj dizisi yazar ve çalıştırır | Sonuç ve karşılaştığı belirsizlikler AUDIT'te |
| 8 | T6 ölçümlerinden en az ikisi yeniden çalıştırılır | Raporla uyumlu (±%20) |
| 9 | Ajan yazabilirlik ölçümünün ham kayıtları | Var; alt ajanların yalnızca şartname ve şema gördüğü kayıtlardan anlaşılır; önce/sonra tablosu tutarlı |
| 10 | Üç yüzey bildirimiyle aynı bulgu | Üç farklı derinlikte cevap |
| 11 | MCP uyarlayıcısı üzerinden üç döküm | Geçer |
| 12 | `RESEARCH.md`: incelenen her protokol için kaynak bağlantısı | Var; güncel belgelere dayanıyor |
| 13 | `git diff --stat` ile `qp-compute`, `qp-render`, `fixtures/golden` | Davranış değişikliği yok |
| 14 | Gizli bilgi taraması ve eski klasörlerin değişmediği | Temiz |
