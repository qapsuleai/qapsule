# RI1 — Alt Görevler

| # | Görev | Okuduğu yer | Yazdığı yer |
|---|-------|-------------|-------------|
| T0 | Hazırlık: ana akış değişikliklerini kaydet; `quantex` deposuna okuma erişimini doğrula; o depoda `git status` çıktısını kaydet (sonra karşılaştırmak için); deponun genel haritası (diller, derleme sistemi, servisler arası bağımlılık) | depo kökü | `rounds/RI1/experiments/MAP.md` |
| T1 | Bağlantı ve kimlik (A) | `quantex-stream/matriks-adapter` | `docs/matriks/CONNECTIVITY.md` |
| T2 | Mesajlar ve protolar (B) | bağlayıcı, proto klasörleri | `docs/matriks/MESSAGES.md` |
| T3 | İç NATS yayını (C) | bağlayıcı ve dört NATS bağlayıcısı | `docs/matriks/NATS.md` |
| T4a–c | Gösterge servisleri (D): fiyat, akış, derinlik; ayrı alt ajanlar | üç C++ servisi ve bağlayıcıları | `docs/matriks/INDICATORS.md` (bölüm başına) |
| T5 | Anomali motoru (E) | `quantex-anomaly-engine`, `anomaly-adapter` | `docs/matriks/ANOMALY.md` |
| T6 | Qapsule için anlamı (F): kavram eşlemesi, seçenekler, riskler, R6 taslağı | T1–T5 çıktıları; Qapsule `docs/` | `docs/matriks/INTEGRATION.md` |
| A | Denetim: bağımsız alt ajan; iddia örneklemesi; sınır taraması | hepsi | `rounds/RI1/AUDIT.md` |

## Notlar

- **Alt ajanlara verilecek ortak kural:** `GOAL.md` "Kesin sınırlar" bölümü aynen. Her alt
  ajan yalnız kendi belgesine yazar; `quantex` içinde yalnız okuma ve arama araçlarını
  kullanır; orada komut çalıştırmaz (`git status` ve `git log` dışında).
- **Gizli dosyalar:** adında `.env`, `secret`, `credential`, `token`, `key`, `.pem`, `.p12`
  geçen dosyalar ve dağıtım gizli tanımları açılmaz; arama sonuçlarında belirirse içerik
  alıntılanmaz.
- **Okuma sırası:** önce yapılandırma şemaları ve giriş noktaları, sonra bağlantı kodu,
  sonra mesaj işleme. C++ servislerinde önce başlık dosyaları ve varsa belgeler.
- **Gösterge karşılaştırması** için Qapsule tarafında `crates/qp-compute/` ve
  `rounds/R1/REPORT.md` okunur; fark formül düzeyinde yazılır (ör. "RSI: Wilder
  yumuşatması, başlangıç ilk N'nin basit ortalaması" gibi), koddan satırla.
- **Anomali eşlemesi** için `docs/CAPSULE.md`, `docs/CONCEPTS.md` ve
  `crates/qp-facts/`, `crates/qp-capsule/` okunur.
- **Lisans:** `docs/CAPSULE.md` §7 ve K33. Kodda yetki ya da paket denetimi görülen her yer
  not edilir.
- Depoda var olan belgeler (README, tasarım notları) değerlidir ama kodla çelişirse kod
  esas alınır ve çelişki yazılır.
- Bu round'da Qapsule'e kod yazılmaz. Küçük bir çözümleme denemesi (ör. bir proto'yu Rust
  tipine çevirmek) gerekli görülürse yapılmaz; R6 için öneri olarak yazılır.
- **Disk (K56):** worktree gerekmez (yalnız belge). Geçici dosyalar round sonunda silinir.
- **GitHub ve ana akış:** `CLAUDE.md` kuralları geçerli; yalnız Qapsule deposuna gönderim.
