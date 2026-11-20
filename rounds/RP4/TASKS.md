# RP4 — Alt Görevler

| # | Görev | Yazdığı yer | Ağ | Bağımlılık |
|---|-------|-------------|----|------------|
| T0 | Hazırlık: ana akış değişiklikleri; `scripts/live-env.sh` ile değişken varlığı; borsa açık mı (takvimden) ve buna göre plan; kalibrasyon ve sınama tarih aralıklarını **önceden** yaz (`experiments/SPLIT.md`) | `rounds/RP4/experiments/` | yok | — |
| T1 | Geçmiş işlem istemcisi ve önbellek; zaman alanı çözümü; ölçülü çekim betiği | `crates/qp-live/` (rest), `scripts/` | **REST (yalnız orkestratör çalıştırır)** | — |
| D1 | **Veri çekimi (orkestratör, sırayla):** evren (en az 100 sembol), en az son 15 işlem günü işlem; gün içi barlar REST'in verdiği en uzun aralıkta | `fixtures/local/` | **REST** | T1 |
| T2 | İşlemden bar üretici; M27 | `crates/qp-source/` | yok (önbellek) | D1 |
| T3 | Canlı işlem bağlayıcısı (`/trade`), kilit, sahte sunucu; Topach ile konu ve düzey keşfi | `crates/qp-live/`, `crates/qp-matriks/` | yok (sahte) | — |
| T4 | Hızlandırılmış benzetim: önbellekten canlı yola; `--sim` | `crates/qp-matriks/` (feed), `crates/qp-cli/`, `scripts/demo.sh` | yok | D1, T2 |
| H | Ölçüm düzeneği ve puanlama kuralı (M25, M26): erken kapanma, kalibrasyon, duyarlılık; **alternatiflerden önce commit** | `rounds/RP4/experiments/harness/` | yok | T4 |
| M25a–e | Çıkış kuralı alternatifleri, ayrı alt ajanlar, ayrı dallar | `crates/qp-capsule/` | yok | H |
| M26a–c | Dedektör eşiği alternatifleri | `crates/qp-facts/`, `crates/qp-capsule/` | yok | H |
| T5 | A52 durma kuralı, sıfırlama komutu; geç katılan istemci | `crates/qp-live/`, `crates/qp-cli/`, `crates/qp-session/` | yok (sahte) | — |
| T6 | Ölçek: artımlı dilim ve yüz | `crates/qp-session/`, `crates/qp-cli/` | yok (benzetim) | T4 |
| T7 | Saat gösterimi; kapanış seansı satırı | `crates/qp-capsule/`, `crates/qp-grammar/`, `shells/web/` | yok | — |
| S | Bütünleştirme: seçilen kural + eşikler + işlem barı birlikte; sınama aralığında son ölçüm; sürümlü sabitler | bütünleştirme dalı | yok | M25–M27, T5–T7 |
| L | **İlk iş günü denemeleri (orkestratör ya da Arkin):** canlı işlem ucu, anlık kip, durma kuralı, tam seans | `experiments/LIVE-LOG.md` | **canlı, tek** | S |
| A | Denetim: bağımsız alt ajan; canlı bağlantı ve REST açmaz | `rounds/RP4/AUDIT.md` | yok | hepsi |

## Notlar

- **Ağ disiplini:** REST çekimini ve canlı bağlantıyı yalnız orkestratör yapar, sırayla.
  Alt ajanlar önbellek ve benzetimle çalışır. REST çekimi sürerken canlı bağlantı açılmaz
  (aynı kimlik; R6 denetimi B2: kimlik kilidi).
- **Önce veri, sonra düzenek, sonra alternatif.** Düzenek sınama aralığını yalnız son
  ölçümde açar; alternatif ajanları yalnız kalibrasyon aralığını görür (sınama aralığının
  dosyaları onlara verilmez).
- **Veri miktarı:** işlem geçmişi 15 günlük pencerelerle geriye doğru çekilebilir; kaç
  pencere çekileceğine istek yükü ve disk boyutuna göre karar ver, raporda gerekçesini
  yaz. Gün içi kalibrasyon için hedef en az 30 işlem günü (20 kalibrasyon + 10 sınama);
  REST daha azını veriyorsa eldeki kadarla ve bunu açıkça söyleyerek.
- **Fiyat adımı:** eşik tabanı için sembolün fiyat adımı gerekir; kaynağı
  `docs/matriks/MESSAGES.md` (canlı `priceStep` ya da adım tablosu). Varsayma; doğrula.
- **Arkin'in gördüğü durum** `rounds/R6/REPORT.md` §10'da anlatılı; R6 ham kayıtları
  `fixtures/local/live/raw/` altında (30 gün saklama). Benzetimde yeniden üretilir.
- **Kapsül sabitlerinin sürümü:** sabitler bir kayıt olarak adlandırılır (ör. `r3`,
  `rp4`); protokol varsayılanının değişip değişmeyeceği öneri olarak yazılır, karar Arkin'in.
- **Ölçek:** hedef makine bu dizüstü; ölçüm hızlandırılmış benzetimde 1× hızda ve
  başka ağır iş yokken alınır; yük raporda.
- **İş gününe denk gelme:** round hafta içine sarkarsa L adımı yapılır; sarkmazsa ölçüt 12
  listesiyle kapanır ve canlı ölçütler "ilk iş günü" diye açık bırakılır.
- **Disk (K56):** `target/wt/<görev>`; elenen dalın worktree'si hemen kaldırılır;
  `fixtures/local/` boyutu ve saklama kuralı raporda.
- **GitHub ve ana akış:** `CLAUDE.md` kuralları geçerli.
