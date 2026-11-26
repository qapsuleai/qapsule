# RL1 — RX4 uygunluk tablosu

PROGRAMS §15.1; üreten `conform.py` (elle düzenlenen yalnız **Açıklama** sütunu; yeniden
koşuda karar değişmemişse açıklama korunur). Her fark ya hata ya da §15'te yazılı sapmadır.

- Koşu şablonu: `/Users/arkns/Works/Mchart/qapsule/target/wt/u1/debug/qp program run --json --full --data {data} {program}`
- Denetim şablonu: `/Users/arkns/Works/Mchart/qapsule/target/wt/u1/debug/qp program check --json {program}`
- Veri kümeleri: seen, heldout; UTC hizası, kapalı pencere, aralık `1h`
- Karar: eşit 52, ret görevi: eşit 4
- Seri değerlerinde bit eşitlik: 30684/42000

| Görev | Küme | Veri | Karar | Bit eşit (seri) | Ayrıntı | Açıklama |
|---|---|---|---|---|---|---|
| G1-01 | G1 | seen | eşit | — | — | — |
| G1-01 | G1 | heldout | eşit | — | — | — |
| G1-02 | G1 | seen | eşit | — | — | — |
| G1-02 | G1 | heldout | eşit | — | — | — |
| G1-03 | G1 | seen | eşit | 3000/3000 | — | — |
| G1-03 | G1 | heldout | eşit | 3000/3000 | — | — |
| G2-01 | G2 | seen | eşit | 129/3000 | — | Bilinçli sapma (PR §15): Qapsule `sma` kayan toplamla (`(sum + x) − evicted`, var olan çekirdek; `add_study` bit eşitliği bunu zorunlu kılar), RX4 başvurusu her barda pencereyi baştan topluyor; z-skorun paydası `stdev` bit eşit, fark son bitlerde, tolerans içinde. |
| G2-01 | G2 | heldout | eşit | 211/3000 | — | Bilinçli sapma (PR §15): Qapsule `sma` kayan toplamla (`(sum + x) − evicted`, var olan çekirdek; `add_study` bit eşitliği bunu zorunlu kılar), RX4 başvurusu her barda pencereyi baştan topluyor; z-skorun paydası `stdev` bit eşit, fark son bitlerde, tolerans içinde. |
| G2-02 | G2 | seen | eşit | 130/3000 | — | Bilinçli sapma (PR §15): Qapsule `sma` kayan toplamla (`(sum + x) − evicted`, var olan çekirdek; `add_study` bit eşitliği bunu zorunlu kılar), RX4 başvurusu her barda pencereyi baştan topluyor; z-skorun paydası `stdev` bit eşit, fark son bitlerde, tolerans içinde. |
| G2-02 | G2 | heldout | eşit | 214/3000 | — | Bilinçli sapma (PR §15): Qapsule `sma` kayan toplamla (`(sum + x) − evicted`, var olan çekirdek; `add_study` bit eşitliği bunu zorunlu kılar), RX4 başvurusu her barda pencereyi baştan topluyor; z-skorun paydası `stdev` bit eşit, fark son bitlerde, tolerans içinde. |
| G2-03 | G2 | seen | eşit | 3000/3000 | — | — |
| G2-03 | G2 | heldout | eşit | 3000/3000 | — | — |
| G2-04 | G2 | seen | eşit | — | — | — |
| G2-04 | G2 | heldout | eşit | — | — | — |
| G3-01 | G3 | seen | eşit | 3000/3000 | — | — |
| G3-01 | G3 | heldout | eşit | 3000/3000 | — | — |
| G3-02 | G3 | seen | eşit | — | — | — |
| G3-02 | G3 | heldout | eşit | — | — | — |
| G3-03 | G3 | seen | eşit | — | — | — |
| G3-03 | G3 | heldout | eşit | — | — | — |
| G3-04 | G3 | seen | eşit | 3000/3000 | — | — |
| G3-04 | G3 | heldout | eşit | 3000/3000 | — | — |
| G4-01 | G4 | seen | eşit | — | — | — |
| G4-01 | G4 | heldout | eşit | — | — | — |
| G4-02 | G4 | seen | eşit | 3000/3000 | — | — |
| G4-02 | G4 | heldout | eşit | 3000/3000 | — | — |
| G4-03 | G4 | seen | eşit | — | — | — |
| G4-03 | G4 | heldout | eşit | — | — | — |
| G4-04 | G4 | — | ret görevi: eşit | — | kabul ['E_LOOKAHEAD']; ret tanındı: E_LOOKAHEAD; yasak program reddedildi: E_LOOKAHEAD | — |
| G5-01 | G5 | — | ret görevi: eşit | — | kabul ['E_LOOKAHEAD']; ret tanındı: E_LOOKAHEAD; yasak program reddedildi: E_LOOKAHEAD | — |
| G5-02 | G5 | — | ret görevi: eşit | — | kabul ['E_UNSUPPORTED', 'E_COST_LIMIT', 'E_WINDOW_LIMIT']; ret tanındı: E_UNSUPPORTED; yasak program reddedildi: E_WINDOW_LIMIT | — |
| G5-03 | G5 | — | ret görevi: eşit | — | kabul ['E_UNKNOWN_OP', 'E_UNSUPPORTED']; ret tanındı: E_UNSUPPORTED; yasak program reddedildi: E_UNKNOWN_OP | — |
| G7-01 | G7 | seen | eşit | — | — | — |
| G7-01 | G7 | heldout | eşit | — | — | — |
| G7-02 | G7 | seen | eşit | — | — | — |
| G7-02 | G7 | heldout | eşit | — | — | — |
| G7-03 | G7 | seen | eşit | — | — | — |
| G7-03 | G7 | heldout | eşit | — | — | — |
| G7-04 | G7 | seen | eşit | — | — | — |
| G7-04 | G7 | heldout | eşit | — | — | — |
| G7-05 | G7 | seen | eşit | — | — | — |
| G7-05 | G7 | heldout | eşit | — | — | — |
| G8-01 | G8 | seen | eşit | — | — | — |
| G8-01 | G8 | heldout | eşit | — | — | — |
| G8-02 | G8 | seen | eşit | — | — | — |
| G8-02 | G8 | heldout | eşit | — | — | — |
| G8-03 | G8 | seen | eşit | — | — | — |
| G8-03 | G8 | heldout | eşit | — | — | — |
| G9-01 | G9 | seen | eşit | — | — | — |
| G9-01 | G9 | heldout | eşit | — | — | — |
| G9-02 | G9 | seen | eşit | — | — | — |
| G9-02 | G9 | heldout | eşit | — | — | — |
| G9-03 | G9 | seen | eşit | — | — | — |
| G9-03 | G9 | heldout | eşit | — | — | — |
| G9-04 | G9 | seen | eşit | — | — | — |
| G9-04 | G9 | heldout | eşit | — | — | — |
