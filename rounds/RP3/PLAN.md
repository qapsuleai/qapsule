# RP3 — Yürütme planı (ana oturum)

Başlangıç: `6877bda` (R4 kodu + ana akış dokümanları). Bu commit "iyileştirme öncesi" başvurudur
(VERIFY 3: kalibrasyon bu kodda yapılır).

## Dalgalar

| Dalga | Görev | Dal | Alt ajan | Yazdığı yer | Port aralığı |
|---|---|---|---|---|---|
| 1 | T1 kalibrasyon | `rp3/t1-calib` | ayrı | `rounds/RP3/CALIBRATION.md`, `rounds/RP3/experiments/T1/`, `shells/web/tests/calibrate.mjs` | 8410–8419 |
| 1 | T2 ölçüm ekranı + boyama arayüzü + otomatik karşılaştırma | `rp3/t2-harness` | ayrı | `shells/web/`, `crates/qp-wasm/` | 8420–8429 |
| 1 | T3 araştırma | (dal yok; yalnız doküman) | ayrı | `rounds/RP3/experiments/T3-research.md` (RESEARCH.md'ye ana oturum taşır) | — |
| 2 | M15 görüntüleyici iş bölümü (K48) | `rp3/m15-viewer` | ayrı | `crates/qp-session/`, `crates/qp-schema/`, `crates/qp-wasm/`, `crates/qp-cli/` (demo), `shells/web/`, `docs/PROTOCOL.md` | 8430–8439 |
| 3 | M12a akıllı yazılım çizici | `rp3/m12a-cpu` | ayrı, kör | `crates/qp-render/`, `crates/qp-wasm/`, `shells/web/paint/` | 8440–8449 |
| 3 | M12b tarayıcı tuvali (Canvas2D) | `rp3/m12b-canvas` | ayrı, kör | `shells/web/paint/`, `crates/qp-wasm/` | 8450–8459 |
| 3 | M12c GPU (WebGPU, yedekli) | `rp3/m12c-gpu` | ayrı, kör | `crates/qp-render*/`, `crates/qp-wasm/`, `shells/web/paint/` | 8460–8469 |
| 3 | M13 açılış | `rp3/m13-open` | ayrı | `crates/qp-cli/` (demo), `scripts/demo.sh`, `shells/web/` (yükleme) | 8470–8479 |
| 3 | T4 bellek | `rp3/t4-mem` | ayrı | `rounds/RP3/experiments/T4/`, ilgili crate | 8480–8489 |
| 4 | M14 + S bütünleştirme, toleranslı ekran testleri | `rp3/s-integrate` | ana oturum / ayrı | — | 8490–8499 |
| 5 | A denetim | `rp3/audit` | bağımsız | `rounds/RP3/AUDIT.md` | 8500–8509 |

Kurallar: her alt ajan kendi worktree'sinde, kendi dalında; yalnız kendi yerlerine yazar; birleştirmeden
önce kendi dalında `scripts/verify.sh` (en az fmt, clippy, test) yeşil. Doğrulamayı işi yapan değil
ayrı denetçi alt ajan yapar. M12 ajanları birbirinin dalını görmez (yönerge metinlerinde diğer
yolların ayrıntısı yok).

## Ölçüm notu

Makine paylaşımlı (başlangıçta yük ortalaması ≈ 6–10, başka oturumlar). Her ölçüm satırı yük
ortalamasını yazar; karar verdiren ölçümler aynı koşulda arka arkaya, sıralı olarak (paralel
değil) alınır.
