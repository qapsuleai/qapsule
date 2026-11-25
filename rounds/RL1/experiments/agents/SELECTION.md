# RL1 ajan teyidi — görev seçimi (20 / 30)

GOAL §7 "Ajan yazabilirlik teyidi": RX4 görev kümesinden 20 görev. Bu dosya ölçümden **önce**
yazıldı; ölçüm sırasında değişmez (değişirse nedeni REPORT.md'ye).

**Kaynak (veri olarak, kopyalanmadan):** görev cümleleri `../refs/experiments/rx4/harness/tasks/<id>.json`
(`text` alanı; ana ölçüm), beklenen çıktılar `harness/expected/<id>/{seen,heldout}.json`, veri
`harness/data/{seen,heldout}.json`. Sürücü bunları koşu anında salt okur (K91: RX4 kodu
içe aktarılmaz; yalnız görev metni ve veri kullanılır). Seçim listesi kodda tek yerde:
`score.py` `SELECTION`.

## Kural

1. **Ret görevlerinin hepsi** (4): G4-04 (tuzak: kapanmamış 4h bar), G5-01, G5-02, G5-03.
2. Kalan 16 yer 26 program görevine **küme payıyla** dağıtılır: pay = küme program sayısı ×
   16/26; önce tam kısımlar (G1 1,85→1, G2 2,46→2, G3 2,46→2, G4 1,85→1, G7 3,08→3,
   G8 1,85→1, G9 2,46→2 = 12), kalan 4 yer en büyük kesirlere: G1 (0,85), G4 (0,85), G8 (0,85),
   sonra eşit üç 0,46'dan G3 — durum düğümleri (`streak`, `cooldown`, `rearm`) uydurma ad ve
   anlam hatasının en olası yeri olduğu için.
3. Küme içinde, kümenin farklı yapılarını kapsayanlar seçilir; birbirinin aynası olan görevden
   biri bırakılır. Yeni çekirdekler (`stdev`, `percentrank`), üst periyot (`align`, `atr(p, tf)`),
   `line` + `defined` ve RX4'te bir oturumun hata aldığı görev (B-haiku G9-02, `E_UNKNOWN_REF`)
   bilerek içeride.

| Küme | RX4 | Seçilen | Pay |
|---|---:|---:|---:|
| G1 bugün ifade edilebilen | 3 | 2 | 1,85 |
| G2 bileşim | 4 | 2 | 2,46 |
| G3 durum | 4 | 3 | 2,46 |
| G4 çoklu periyot (program) | 3 | 2 | 1,85 |
| G4/G5 ret | 4 | 4 | hepsi |
| G7 sinyal + sonuç kuralı | 5 | 3 | 3,08 |
| G8 durum tanımı | 3 | 2 | 1,85 |
| G9 izleme koşulu | 4 | 2 | 2,46 |
| **Toplam** | **30** | **20** | |

## Seçilen 20 görev

| Görev | Tür | Neden |
|---|---|---|
| G1-01 | olay | RSI + `cross_above`; en yalın koşul (taban) |
| G1-03 | seri | İç içe çekirdek `ema(rsi(...))`: boş girdi atlama kuralı |
| G2-01 | seri + olay | **`stdev` (yeni çekirdek)**, nüfus std, z-skor ve kesişim |
| G2-03 | seri + olay | "bu bar hariç" = `highest(high, 20)[1]`: gecikme yeri, kesin `>` |
| G3-01 | seri + olay | `streak` ve "tam 5" eşitliği |
| G3-02 | olay | `cooldown` ve `i − i₀ ≤ 10` sınırı |
| G3-03 | olay | `rearm`: başta kurulu, yeniden kurulduğu barda tetiklemez (en ince anlam) |
| G4-01 | olay | 4h EMA `align` + saatlik kesişim: saat karışımı (`E_CLOCK_MISMATCH` riski) |
| G4-02 | seri + olay | Günlük `atr(14, 1d)` yazımı ve `align` (uydurma imza riski) |
| G4-04 | ret (tuzak) | Kapanmamış 4h bar → `E_LOOKAHEAD`; programla yazmaya en açık tuzak |
| G5-01 | ret | Yarının kapanışı → `E_LOOKAHEAD` |
| G5-02 | ret | Sınırsız döngü → `E_UNSUPPORTED`/`E_COST_LIMIT`/`E_WINDOW_LIMIT` |
| G5-03 | ret | Parabolic SAR dilde yok → `E_UNKNOWN_OP`/`E_UNSUPPORTED` (uydurma `sar(...)` yazma riski) |
| G7-01 | sinyal | `al`, hedef + durdurma, `max_bars`, kaynak |
| G7-03 | sinyal | Durdurmasız kural (isteğe bağlı alanın yokluğu) |
| G7-05 | iki sinyal | `al` ve `sat` aynı programda, iki çıktı |
| G8-01 | durum tanımı | Üç boyut, `bucket`, eğim ifadesi |
| G8-03 | durum tanımı | **`percentrank` (yeni)**, `atr` üstüne; 4h boyutlar `align` ile |
| G9-02 | üç olay | İki aşamalı kırılım, 4h, kurulum barı; RX4'te tek hatalı oturumun görevi |
| G9-03 | iki olay | `line` + `defined` (Qapsule'e özgü tanımlılık düğümü), kurulum barı |

## Bırakılan 10 görev

| Görev | Neden bırakıldı |
|---|---|
| G1-02 | SMA kesişimi: G1-01 / G7-03 ile aynı yapı |
| G2-02 | ATR bandı: `atr` G4-02 ve G8-03'te, kesişim G2-01'de var |
| G2-04 | Hacim + önceki ortalama: G2-03'ün gecikme yapısının eşi |
| G3-04 | `bars_since`: sayaç yapısı G3-01'de; seçilen üç durum düğümü daha zor |
| G4-03 | 4h yüksek kesişimi: `align` + kesişim G4-01'de |
| G7-02 | G7-01'in aynası (`sat`, `cross_below`); `sat` G7-05'te |
| G7-04 | `lowest[1]` + `sat`: gecikme G2-03'te, `sat` G7-05'te |
| G8-02 | İki periyotlu durum: G8-03 aynı yapıyı yeni çekirdekle taşıyor |
| G9-01 | 4h kırılım: G9-02 onu bir aşama olarak içeriyor |
| G9-04 | Kanal (iki `line`): G9-03'ün eşi |

## Qapsule'e uyarlama (görev cümlesi değişmez)

- İstemdeki çıktı listesinde RX4 `fingerprint` → Qapsule `state` (PR §5.4; cümledeki "durum
  parmak izi" sözü aynen kalır). Puanlama RX4 adıyla karşılaştırır (`conform.adapt_one`).
- Veri RX4 `seen` serisi, oturumda kimliği `rx4`, takvimsiz → UTC hizası (PR §3.2, §15.1);
  saat son barın kapanışından sonra (kapalı pencere). Doğruluk RX4 gibi iki kümede: `seen` ve
  ajanın görmediği `heldout`.
