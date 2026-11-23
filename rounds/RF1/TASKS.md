# RF1 — Alt Görevler

| # | Görev | Yazdığı yer | Bağımlı |
|---|-------|-------------|---------|
| T0 | Ana akış belge değişikliklerini kaydet; alt ajan başlatılabiliyor mu, rapora yaz | kök | — |
| B | **Başlangıç tutanağı** (commit): RG1 kalemleri + `rd2-*` altınları, `design-check`, `design-contract --check`, galeri dökümü; performans ve WASM boyutu | `rounds/RF1/experiments/baseline/` | T0 |
| Ö | Ön işler: P8/P9 ölçümü (ayrı fikstür); eski Playwright klasörleri | `scripts/`, `fixtures/design/` | B |
| A1 | `scripts/refactor/` (kalıcı araçlar, ek denetimler) | `scripts/` | B |
| A2 | `arch-check`: sıralı katman, yapılandırma farkı adımı, fonksiyon boyu tavanı; öz-sınama | `scripts/archcheck` | B |
| A3 | Yeni lint'ler *(kesilebilir)*; sözdizimi ağacı *(kesilebilir)* | `scripts/`, kök | A2 |
| V | `qp-voice` (saf taşıma) | `crates/qp-capsule`, `crates/qp-voice` | A1 |
| P | **Bölme planı** `PLAN.md` (ölçülmüş grafik, hedef crate'ler, arayüzler, sıra, risk, ek noktası taslağı) + iki bağımsız eleştiri + cevaplar | `rounds/RF1/` | B |
| K | **Karar noktası:** dur, Arkin'e sor; `DECISION.md` | `rounds/RF1/` | P |
| G1 | Gramer: karakterizasyon sınamaları (katman türü başına, görünüş başına) | `crates/qp-grammar/tests` | K |
| G2 | Gramer: kayıt enjeksiyonu; tasarım tarafı ayrı crate(ler)e | `crates/qp-grammar`, yeni crate | G1 |
| G3 | `prepare` adımlara bölünür | `crates/qp-grammar` | G1 |
| S1 | Oturum: karakterizasyon sınamaları (işlem türü başına `expand_op`; alt sistem başına) | `crates/qp-session/tests` | K |
| S2 | Oturum: yapraklar ve tepeler ayrı crate'lere | ilgili crate'ler | S1 |
| S3 | Oturum: otorite arayüzü ve kayıt; alt sistemler tek tek (dalga dalga; aynı anda tek ajan) | ilgili crate'ler | S2 |
| S4 | `expand_op` adımlara bölünür | ilgili crate | S1 |
| E | Ek noktası: `docs/ARCHITECTURE.md` bölümü + örnek alt sistem sınaması | `docs/`, test | S3 |
| Q | Küçükler: `qp-source` dosya okuması *(kesilebilir)*; geçici yeniden dışa aktarmalar *(kesilebilir)*; eskimiş yorumlar | ilgili crate'ler | S3, G2 |
| R | README'ler, ARCHITECTURE, ENGINEERING §2 katman tablosu, CONCEPTS (gerekiyorsa) | `docs/`, crate'ler | S3, G2 |
| C | Son karşılaştırma, performans, WASM, CI | `rounds/RF1/experiments/` | hepsi |
| D | Bağımsız denetim | `rounds/RF1/AUDIT.md` | C |
| X | `REPORT.md`, `STATUS.md`, `ROADMAP.md`, önerilen kararlar (`docs/decisions/rounds/RF1.md`), kapanış temizliği | kök | D |
