# RP1 — Alt Görevler

| # | Görev | Yazdığı yer |
|---|-------|-------------|
| T1 | Ölçüm tabanı ve dört yük profili; başlangıç ölçümleri (referans makine, CI, WASM) | `crates/qp-compute/benches/`, `rounds/RP1/BASELINE.md` |
| T2 | Araştırma: literatür ve var olan motorların taranması; aday yaklaşımların listesi ve beklenen kazanç | `rounds/RP1/RESEARCH.md` |
| T3..Tn | Her yaklaşım ayrı dalda, ayrı alt ajanla küçük deneme: hipotez → uygulama → ölçüm → karar. Denemeler birbirinden bağımsız yürür | `crates/qp-compute/` (dal başına), `rounds/RP1/RESEARCH.md` |
| Tn+1 | Kazananların birleştirilmesi; birlikte ölçüm (kazançlar toplanmayabilir) | `crates/qp-compute/` |
| Tn+2 | Denetim: bağımsız alt ajan ölçümleri yeniden üretir, bit-eşitlik testlerini çalıştırır | `rounds/RP1/AUDIT.md` |

Notlar:
- R2 ile aynı anda koşuluyorsa yalnızca `crates/qp-compute/` ve `rounds/RP1/` altına yaz;
  `qp-schema`, `qp-render`, `qp-grammar`, `qp-wasm` dosyalarına dokunma.
- Ölçümde gürültüyü raporla: medyan ve yayılım; CI koşucusunda en az beş çalıştırma.
- Bir yaklaşımın "işe yaramadığı" da bir sonuçtur; silme, kaydet.
