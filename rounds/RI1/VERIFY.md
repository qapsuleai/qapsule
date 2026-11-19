# RI1 — Doğrulama

| # | Kontrol | Beklenen |
|---|---------|----------|
| 1 | `quantex` deposunda `git status --porcelain`, round öncesi ve sonrası | Aynı; yeni ya da değişmiş dosya yok (derleme çıktısı dahil) |
| 2 | Oturum boyunca ağ: Matriks, MQTT, NATS, iç servis adreslerine bağlantı | Yok |
| 3 | `docs/matriks/` ve `rounds/RI1/` içinde gizli değer taraması (belirteç, parola, anahtar kalıpları) | 0 bulgu |
| 4 | Aynı klasörlerde tam iç adres (IP, sunucu adı, port) taraması | 0 bulgu; genelleştirilmiş anlatım |
| 5 | Kopyalanmış proto tanımı ya da 10 satırdan uzun kod alıntısı | Yok |
| 6 | Gerçek piyasa verisi (fiyat, hacim satırları) | Yok |
| 7 | Rastgele 40 iddia, kaynağında | Doğru; yanlışlar düzeltilmiş ve `AUDIT.md`'de listeli |
| 8 | GOAL A–F soru listesi ile belgelerin karşılaştırması | Her soru cevaplı ya da "bilinmiyor" + kime sorulacağı |
| 9 | Arkin'in beş bilgisi | Her biri için hüküm ve kaynak |
| 10 | Qapsule deposunda `git diff --stat` | Yalnız `docs/matriks/`, `rounds/RI1/`, `STATUS.md`, `ROADMAP.md`, `DECISIONS.md` (öneri bölümü) |
| 11 | `scripts/preflight.sh`; gönderim | Yeşil; yalnız `origin` |
