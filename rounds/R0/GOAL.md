# R0 — Temel

## Hedef

Üzerine her şeyin kurulacağı zemini hazırlamak ve en riskli teknik varsayımı
(aynı Rust çizim kodunun hem ekransız hem tarayıcıda deterministik çalışması) kanıtlamak.

## Kapsam

1. Git deposu, Rust workspace iskeleti, biçim/lint kuralları, CI.
2. Şema v0: Seri, Olgu, Bulgu, Kapsül, Görünüm, Doküman tipleri ve JSON Schema üretimi.
3. Çizim denemesi: sabit bar verisinden PNG; aynı kodun WASM ile tarayıcıda çizmesi.
4. `qp` komut satırı: `qp schema export`, `qp render`.
5. Doğrulama düzeneği: `scripts/verify.sh` ve altın görsel testi.
6. Eski prototipten envanter: taşınacak kod ve Matriks protokol bilgisi.

## Kapsam dışı

- Hesap çekirdeği (R1). Görsel gramer (R2). Kapsül davranışı (R3).
- Etkileşim, canlı veri, mobil.
- Şemanın "doğru" olması beklenmiyor; "var ve evrilebilir" olması bekleniyor.

## Kabul ölçütleri

| # | Ölçüt |
|---|-------|
| 1 | `scripts/verify.sh` temiz bir kopyada tek komutla yeşil. |
| 2 | Şema tipleri JSON'a gidip geri dönüyor; `fixtures/` içinde en az bir örnek Kapsül ve bir örnek Doküman var ve üretilen JSON Schema'ya göre geçerli. |
| 3 | `qp render fixtures/bars/sample.json -o out.png` iki ayrı çalıştırmada aynı görseli üretiyor ve altın görselle eşleşiyor. |
| 4 | Aynı çizim kodu `wasm32` hedefine derleniyor ve bir test sayfasında aynı veriyi çiziyor; tarayıcı çıktısı ile ekransız çıktı arasındaki fark ölçülüp raporlanmış. |
| 5 | `DECISIONS.md` içindeki A1 (çizim yığını) kanıtıyla kapatılmış: denenen seçenekler, ölçümler, seçim. |
| 6 | `docs/LEGACY.md` yazılmış: taşınacak dosyalar, taşınmayacaklar ve nedenleri. `docs/MATRIKS_PROTOCOL.md` yazılmış; içinde gizli bilgi yok. |
| 7 | Depoda `.env` veya token yok; `.gitignore` bunları dışlıyor. |
| 8 | `rounds/R0/REPORT.md` yazılmış, `STATUS.md` güncellenmiş. |
